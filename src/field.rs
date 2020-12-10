use itertools::Itertools;
use model::{AttackProperties, Entity, EntityProperties, EntityType};
#[cfg(feature = "enable_debug")]
use model::Color;

#[cfg(feature = "enable_debug")]
use crate::my_strategy::{
    debug,
    Vec2f,
};
use crate::my_strategy::{Config, Group, index_to_position, position_to_index, Positionable, Vec2i, visit_range, visit_square, World};

#[derive(Default, Clone)]
struct PlayerFragment {
    sight_range_power: f32,
    attack_range_power: f32,
    destroy_score: f32,
    sight_score: f32,
}

#[derive(Default, Clone)]
struct Fragment {
    my_static_sight_range_power: f32,
    my_static_attack_range_power: f32,
    my_static_destroy_score: f32,
    my_group_in_attack_range_scores: Vec<(usize, f32)>,
    my_group_in_sight_range_scores: Vec<(usize, f32)>,
    my_entities_in_attack_range_scores: Vec<(i32, f32)>,
    my_entities_in_sight_range_scores: Vec<(i32, f32)>,
    resource: f32,
    opponent_sight_range_power: f32,
    opponent_attack_range_power: f32,
    opponent_destroy_score: f32,
    player_fragments: Vec<PlayerFragment>,
}

pub struct Field {
    size: usize,
    config: Config,
    fragments: Vec<Fragment>,
    players: Vec<i32>,
}

impl Field {
    pub fn new(map_size: i32, config: Config) -> Self {
        Self {
            size: map_size as usize,
            config,
            fragments: std::iter::repeat(Fragment::default()).take((map_size * map_size) as usize).collect(),
            players: Vec::new(),
        }
    }

    pub fn update(&mut self, groups: &Vec<Group>, world: &World) {
        if self.players.len() != world.players().len() {
            self.players = world.players().iter().map(|v| v.id).collect();
        }
        let bounds = world.bounds();
        for fragment in self.fragments.iter_mut() {
            fragment.my_static_sight_range_power = 0.0;
            fragment.my_static_attack_range_power = 0.0;
            fragment.my_static_destroy_score = 0.0;
            fragment.my_group_in_attack_range_scores.clear();
            fragment.my_group_in_sight_range_scores.clear();
            fragment.my_entities_in_attack_range_scores.clear();
            fragment.my_entities_in_sight_range_scores.clear();
            fragment.resource = 0.0;
            for v in fragment.player_fragments.iter_mut() {
                *v = PlayerFragment::default();
            }
            if fragment.player_fragments.len() != self.players.len() {
                fragment.player_fragments = std::iter::repeat(PlayerFragment::default()).take(self.players.len()).collect();
            }
        }
        for entity in world.entities() {
            if let Some(player_id) = entity.player_id {
                let player_index = self.players.iter().find_position(|v| **v == player_id).unwrap().0;
                let properties = world.get_entity_properties(&entity.entity_type);
                if let Some(attack) = properties.attack.as_ref() {
                    visit_range(entity.position(), properties.size, attack.attack_range, &bounds, |position| {
                        let fragment = &mut self.fragments[position_to_index(position, self.size)];
                        if player_id == world.my_id() {
                            let score = get_range_score(position, entity.id, world, |_, v| v.attack_range);
                            fragment.my_entities_in_attack_range_scores.push((entity.id, score));
                        }
                        fragment.player_fragments[player_index].attack_range_power += get_range_score(position, entity.id, world, |_, v| v.attack_range);
                    });
                    visit_range(entity.position(), properties.size, properties.sight_range, &bounds, |position| {
                        let fragment = &mut self.fragments[position_to_index(position, self.size)];
                        if player_id == world.my_id() {
                            let score = get_range_score(position, entity.id, world, |v, _| v.sight_range);
                            fragment.my_entities_in_sight_range_scores.push((entity.id, score));
                        }
                        fragment.player_fragments[player_index].sight_range_power += get_range_score(position, entity.id, world, |v, _| v.sight_range);
                    });
                }
                if player_id == world.my_id() && is_static(&entity.entity_type) {
                    if let Some(attack) = properties.attack.as_ref() {
                        let power = (attack.damage * entity.health) as f32;
                        visit_range(entity.position(), properties.size, properties.sight_range, &bounds, |position| {
                            let fragment = &mut self.fragments[position_to_index(position, self.size)];
                            let distance = position.center().manhattan_distance(entity.center_f(properties.size)) as f32;
                            fragment.my_static_sight_range_power += field_function(distance, power, properties.sight_range as f32);
                        });
                        visit_range(entity.position(), properties.size, attack.attack_range, &bounds, |position| {
                            let fragment = &mut self.fragments[position_to_index(position, self.size)];
                            let distance = position.center().manhattan_distance(entity.center_f(properties.size)) as f32;
                            fragment.my_static_attack_range_power += field_function(distance, power, attack.attack_range as f32);
                        });
                    }
                    visit_square(entity.position(), properties.size, |position| {
                        let fragment = &mut self.fragments[position_to_index(position, self.size)];
                        fragment.my_static_destroy_score += properties.destroy_score as f32;
                    });
                }
                if player_id != world.my_id() {
                    visit_square(entity.position(), properties.size, |position| {
                        let fragment = &mut self.fragments[position_to_index(position, self.size)];
                        fragment.player_fragments[player_index].destroy_score += properties.destroy_score as f32;
                    });
                }
                visit_range(entity.position(), properties.size, properties.sight_range, &bounds, |position| {
                    let fragment = &mut self.fragments[position_to_index(position, self.size)];
                    fragment.player_fragments[player_index].sight_score += properties.destroy_score as f32;
                });
            }
            if matches!(entity.entity_type, EntityType::Resource) {
                let fragment = &mut self.fragments[position_to_index(entity.position(), self.size)];
                fragment.resource += 1.0;
            }
        }
        let my_player_index = self.players.iter().find_position(|v| **v == world.my_id()).unwrap().0;
        for i in 0..self.fragments.len() {
            let position = index_to_position(i, self.size);
            let fragment = &mut self.fragments[i];
            let mut opponent_attack_range_power = 0.0;
            let mut opponent_sight_range_power = 0.0;
            let mut opponent_destroy_score = 0.0;
            for i in 0..fragment.player_fragments.len() {
                if i != my_player_index {
                    opponent_attack_range_power += fragment.opponent_attack_range_power;
                    opponent_sight_range_power += fragment.opponent_sight_range_power;
                    opponent_destroy_score += fragment.opponent_destroy_score;
                }
            }
            fragment.opponent_attack_range_power = opponent_attack_range_power;
            fragment.opponent_sight_range_power = opponent_sight_range_power;
            fragment.opponent_destroy_score = opponent_destroy_score;
            for group_index in 0..groups.len() {
                let group = &groups[group_index];
                let distance = position.distance(group.position());
                let attack_range = group.sight_range() + group.radius();
                if distance <= attack_range {
                    let score = field_function(
                        distance as f32,
                        group.power() as f32,
                        attack_range as f32,
                    ).min(group.power() as f32);
                    fragment.my_group_in_attack_range_scores.push((group_index, score));
                }
                let sight_range = group.sight_range() + group.radius();
                if distance <= sight_range {
                    let score = field_function(
                        distance as f32,
                        group.power() as f32,
                        sight_range as f32,
                    ).min(group.power() as f32);
                    fragment.my_group_in_sight_range_scores.push((group_index, score));
                }
            }
        }
    }

    pub fn get_player_index(&self, player_id: i32) -> usize {
        self.players.iter().find_position(|v| **v == player_id).unwrap().0
    }

    pub fn get_player_influence_score(&self, position: Vec2i, player_index: usize) -> f32 {
        let fragment = &self.fragments[position_to_index(position, self.size)];
        let player_fragment = &fragment.player_fragments[player_index];
        0.0
            + player_fragment.destroy_score
            + player_fragment.attack_range_power
            + player_fragment.sight_range_power
            + player_fragment.sight_score
    }

    pub fn get_group_score(&self, position: Vec2i, group: &Group, groups: &Vec<Group>) -> f32 {
        let group_index = groups.iter().find_position(|v| v.id() == group.id()).unwrap().0;
        let fragment = &self.fragments[position_to_index(position, self.size)];
        let my_attack_range_power = if fragment.opponent_attack_range_power > 0.0 {
            group.power() as f32
                + fragment.my_static_attack_range_power
                + fragment.my_group_in_attack_range_scores.iter()
                .filter(|(index, score)| *index != group_index && *score > 0.0)
                .filter_map(|(index, score)| {
                    let my_group = &groups[*index];
                    if my_group.is_empty() || my_group.power() == 0 {
                        return None;
                    }
                    Some(*score)
                })
                .sum::<f32>()
        } else {
            fragment.my_static_attack_range_power * self.config.group_my_static_attack_range_power_weight
        };
        let my_sight_range_power = if fragment.opponent_sight_range_power > 0.0 {
            group.power() as f32
                + fragment.my_static_sight_range_power
                + fragment.my_group_in_sight_range_scores.iter()
                .filter(|(index, score)| *index != group_index && *score > 0.0)
                .filter_map(|(index, score)| {
                    let my_group = &groups[*index];
                    if my_group.is_empty() || my_group.power() == 0 {
                        return None;
                    }
                    Some(*score)
                })
                .sum::<f32>()
        } else {
            0.0
        };
        0.0
            + group.position().distance(position) as f32 * self.config.group_distance_to_position_weight
            + my_attack_range_power * self.config.group_my_attack_range_power_weight
            + fragment.opponent_attack_range_power * self.config.group_opponent_attack_range_power_weight
            + my_sight_range_power * self.config.group_my_sight_range_power_weight
            + fragment.opponent_sight_range_power * self.config.group_opponent_sight_range_power_weight
            + ((fragment.opponent_sight_range_power > 0.0) as i32 * group.destroy_score()) as f32 * self.config.group_my_destroy_score_weight
            + ((fragment.opponent_sight_range_power > 0.0) as i32) as f32 * fragment.my_static_destroy_score * self.config.group_my_static_destroy_score_weight
            + fragment.opponent_destroy_score * self.config.group_opponent_destroy_score_weight
    }

    pub fn get_entity_score(&self, position: Vec2i, unit: &Entity, world: &World) -> f32 {
        let properties = world.get_entity_properties(&unit.entity_type);
        let power = if let Some(attack) = properties.attack.as_ref() {
            (unit.health * attack.damage) as f32
        } else {
            0.0
        };
        let fragment = &self.fragments[position_to_index(position, self.size)];
        let my_attack_range_power = if fragment.opponent_attack_range_power > 0.0 {
            power
                + fragment.my_entities_in_attack_range_scores.iter()
                .filter(|(entity_id, score)| *entity_id != unit.id && *score > 0.0)
                .map(|(_, score)| *score)
                .sum::<f32>()
        } else {
            fragment.my_static_attack_range_power * self.config.group_my_static_attack_range_power_weight
        };
        let my_sight_range_power = if fragment.opponent_sight_range_power > 0.0 {
            power
                + fragment.my_entities_in_sight_range_scores.iter()
                .filter(|(entity_id, score)| *entity_id != unit.id && *score > 0.0)
                .map(|(_, score)| *score)
                .sum::<f32>()
        } else {
            0.0
        };
        0.0
            + unit.position().distance(position) as f32 * self.config.entity_distance_to_position_weight
            + my_attack_range_power as f32 * self.config.entity_my_attack_range_power_weight
            + fragment.opponent_attack_range_power * self.config.entity_opponent_attack_range_power_weight
            + my_sight_range_power as f32 * self.config.entity_my_sight_range_power_weight
            + fragment.opponent_sight_range_power * self.config.entity_opponent_sight_range_power_weight
            + fragment.opponent_destroy_score * self.config.entity_opponent_destroy_score_weight
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, debug: &mut debug::Debug) {
        let mut min_score = std::f32::MAX;
        let mut max_score = -std::f32::MAX;
        for i in 0..self.size * self.size {
            let fragment = &self.fragments[i];
            let score = fragment.opponent_sight_range_power;
            min_score = score.min(score);
            max_score = score.max(score);
        }
        let norm = (max_score - min_score).max(1.0) as f32;
        for i in 0..self.size * self.size {
            let position = index_to_position(i, self.size);
            let fragment = &self.fragments[i];
            let score = fragment.opponent_sight_range_power;
            debug.add_world_square(
                Vec2f::from(position),
                1.0,
                color_from_heat(0.25, (score - min_score) as f32 / norm),
            );
            debug.add_world_text(
                format!("{}", score),
                position.center(),
                Vec2f::zero(),
                Color { a: 1.0, r: 0.0, g: 0.0, b: 0.0 },
            );
        }
    }
}

pub fn field_function(distance: f32, factor: f32, max: f32) -> f32 {
    factor - factor * distance / max
}

pub fn is_static(entity_type: &EntityType) -> bool {
    match entity_type {
        EntityType::House | EntityType::BuilderBase | EntityType::MeleeBase | EntityType::RangedBase | EntityType::Resource | EntityType::Turret => true,
        _ => false,
    }
}

fn get_range_score<F: Fn(&EntityProperties, &AttackProperties) -> i32>(position: Vec2i, entity_id: i32, world: &World, get_range: F) -> f32 {
    let entity = world.get_entity(entity_id);
    let entity_properties = world.get_entity_properties(&entity.entity_type);
    if let Some(entity_attack) = entity_properties.attack.as_ref() {
        let power = entity.health * entity_attack.damage;
        field_function(
            entity.center_f(entity_properties.size).manhattan_distance(position.center()) as f32,
            power as f32,
            (entity_properties.size - 1 + get_range(&entity_properties, &entity_attack)) as f32,
        ).min(power as f32)
    } else {
        0.0
    }
}

#[cfg(feature = "enable_debug")]
pub fn color_from_heat(alpha: f32, mut value: f32) -> Color {
    value = value.max(0.0).min(1.0);
    if value < 0.25 {
        Color { a: alpha, r: 0.0, g: 4.0 * value, b: 1.0 }
    } else if value < 0.5 {
        Color { a: alpha, r: 0.0, g: 1.0, b: 1.0 - 4.0 * (value - 0.5) }
    } else if value < 0.75 {
        Color { a: alpha, r: 4.0 * (value - 0.5), g: 1.0, b: 0.0 }
    } else {
        Color { a: alpha, r: 1.0, g: 1.0 - 4.0 * (value - 0.75), b: 0.0 }
    }
}
