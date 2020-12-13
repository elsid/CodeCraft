use itertools::Itertools;
#[cfg(feature = "enable_debug")]
use model::Color;
use model::EntityType;

#[cfg(feature = "enable_debug")]
use crate::my_strategy::{
    debug,
    index_to_position,
    Vec2f,
};
use crate::my_strategy::{Config, position_to_index, Positionable, Vec2i, visit_range, visit_square, World};

#[derive(Default, Clone)]
struct PlayerFragment {
    dynamic_sight_range_power: f32,
    static_sight_range_power: f32,
    dynamic_attack_range_power: f32,
    static_attack_range_power: f32,
    dynamic_military_destroy_score: f32,
    dynamic_economy_destroy_score: f32,
    static_destroy_score: f32,
    dynamic_sight_score: f32,
    static_sight_score: f32,
}

#[derive(Default, Clone)]
struct Fragment {
    resource: f32,
    player_fragments: Vec<PlayerFragment>,
    score: f32,
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

    pub fn update(&mut self, world: &World) {
        if self.players.len() != world.players().len() {
            self.players = world.players().iter().map(|v| v.id).collect();
        }
        let bounds = world.bounds();
        for fragment in self.fragments.iter_mut() {
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
                visit_square(entity.position(), properties.size, |position| {
                    let fragment = &mut self.fragments[position_to_index(position, self.size)];
                    if properties.can_move {
                        if matches!(entity.entity_type, EntityType::BuilderUnit) {
                            fragment.player_fragments[player_index].dynamic_economy_destroy_score += properties.destroy_score as f32;
                        } else {
                            fragment.player_fragments[player_index].dynamic_military_destroy_score += properties.destroy_score as f32;
                        }
                    } else {
                        fragment.player_fragments[player_index].static_destroy_score += properties.destroy_score as f32;
                    }
                });
                visit_range(entity.position(), properties.size, properties.sight_range, &bounds, |position| {
                    let fragment = &mut self.fragments[position_to_index(position, self.size)];
                    fragment.player_fragments[player_index].static_sight_score += 1.0;
                });
                let entity_center = entity.center_f(properties.size);
                if let Some(attack) = properties.attack.as_ref() {
                    let power = (entity.health * attack.damage) as f32;
                    visit_range(entity.position(), properties.size, attack.attack_range, &bounds, |position| {
                        let fragment = &mut self.fragments[position_to_index(position, self.size)];
                        let score = field_function(
                            entity_center.manhattan_distance(position.center()) as f32,
                            power,
                            (properties.size - 1 + attack.attack_range) as f32,
                        );
                        if properties.can_move {
                            fragment.player_fragments[player_index].dynamic_attack_range_power += score;
                        } else {
                            fragment.player_fragments[player_index].static_attack_range_power += score;
                        }
                    });
                    visit_range(entity.position(), properties.size, properties.sight_range, &bounds, |position| {
                        let fragment = &mut self.fragments[position_to_index(position, self.size)];
                        let score = field_function(
                            entity_center.manhattan_distance(position.center()) as f32,
                            power,
                            (properties.size - 1 + properties.sight_range) as f32,
                        );
                        if properties.can_move {
                            fragment.player_fragments[player_index].dynamic_sight_range_power += score;
                        } else {
                            fragment.player_fragments[player_index].static_sight_range_power += score;
                        }
                    });
                }
            }
            if matches!(entity.entity_type, EntityType::Resource) {
                let fragment = &mut self.fragments[position_to_index(entity.position(), self.size)];
                fragment.resource += entity.health as f32;
            }
        }
        for i in 0..self.fragments.len() {
            let mut score = 0.0
                + self.fragments[i].resource * self.config.resource_weight;
            let mut opponent_power_score = 0.0;
            for j in 0..self.players.len() {
                if self.players[j] != world.my_id() {
                    let player_fragment = &self.fragments[i].player_fragments[j];
                    opponent_power_score += 0.0
                        + player_fragment.dynamic_attack_range_power * self.config.opponent_dynamic_attack_range_power_weight
                        + player_fragment.static_attack_range_power * self.config.opponent_static_attack_range_power_weight
                        + player_fragment.dynamic_sight_range_power * self.config.opponent_dynamic_sight_range_power_weight
                        + player_fragment.static_sight_range_power * self.config.opponent_static_sight_range_power_weight;
                    score += opponent_power_score
                        + player_fragment.dynamic_military_destroy_score * self.config.opponent_dynamic_military_destroy_score_weight
                        + player_fragment.dynamic_economy_destroy_score * self.config.opponent_dynamic_economy_destroy_score_weight
                        + player_fragment.static_destroy_score * self.config.opponent_static_destroy_score_weight
                        + player_fragment.dynamic_sight_score * self.config.opponent_dynamic_sight_score_weight
                        + player_fragment.static_sight_score * self.config.opponent_static_sight_score_weight;
                }
            }
            for j in 0..self.players.len() {
                if self.players[j] == world.my_id() {
                    let player_fragment = &self.fragments[i].player_fragments[j];
                    if opponent_power_score > 0.0 {
                        score += 0.0
                            + player_fragment.dynamic_economy_destroy_score * self.config.my_dynamic_economy_destroy_score_weight
                            + player_fragment.static_destroy_score * self.config.my_static_destroy_score_weight;
                    }
                    score += 0.0
                        + player_fragment.static_attack_range_power * self.config.my_static_attack_range_power_weight
                        + player_fragment.static_sight_range_power * self.config.my_static_sight_range_power_weight;
                    break;
                }
            }
            self.fragments[i].score = score;
        }
        let min_score = self.fragments.iter()
            .fold(std::f32::MAX, |r, fragment| r.min(fragment.score));
        let max_score = self.fragments.iter()
            .fold(-std::f32::MAX, |r, fragment| r.max(fragment.score));
        let norm = max_score - min_score;
        for fragment in self.fragments.iter_mut() {
            let normalized = (fragment.score - min_score) / norm;
            fragment.score = normalized;
        }
    }

    pub fn get_player_index(&self, player_id: i32) -> usize {
        self.players.iter().find_position(|v| **v == player_id).unwrap().0
    }

    pub fn get_player_influence_score(&self, position: Vec2i, player_index: usize) -> f32 {
        let fragment = &self.fragments[position_to_index(position, self.size)];
        let player_fragment = &fragment.player_fragments[player_index];
        0.0
            + player_fragment.dynamic_attack_range_power
            + player_fragment.static_attack_range_power
            + player_fragment.dynamic_sight_range_power
            + player_fragment.static_sight_range_power
            + player_fragment.dynamic_military_destroy_score
            + player_fragment.dynamic_economy_destroy_score
            + player_fragment.static_destroy_score
            + player_fragment.dynamic_sight_score
            + player_fragment.static_sight_score
    }

    pub fn get_score(&self, position: Vec2i) -> f32 {
        self.fragments[position_to_index(position, self.size)].score
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, debug: &mut debug::Debug) {
        let norm: f32 = self.fragments.iter().map(|fragment| fragment.score).sum();
        for i in 0..self.size * self.size {
            let position = index_to_position(i, self.size);
            debug.add_world_square(
                Vec2f::from(position),
                1.0,
                debug::color_from_heat(0.25, self.fragments[i].score / norm),
            );
            debug.add_world_text(
                format!("{}", self.fragments[i].score),
                position.center(),
                Vec2f::zero(),
                Color { a: 1.0, r: 0.0, g: 0.0, b: 0.0 },
            );
        }
    }
}

pub fn field_function(distance: f32, factor: f32, max: f32) -> f32 {
    (factor - factor * distance / max).max(0.0).min(factor)
}
