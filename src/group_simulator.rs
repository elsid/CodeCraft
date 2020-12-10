use model::EntityType;

use crate::my_strategy::{Group, index_to_position, Tile, Vec2i, World};

#[derive(Clone, Debug)]
pub struct SimulatedGroup {
    pub id: u32,
    pub position: Vec2i,
    pub destroy_score: f32,
    pub damage: f32,
    pub health: f32,
    pub move_direction: Vec2i,
}

#[derive(Default, Clone, Debug)]
struct Segment {
    my_health: f32,
    my_damage: f32,
    my_destroy_score: f32,
    opponent_destroy_score: f32,
    opponent_damage: f32,
    opponent_health: f32,
    resource_health: f32,
}

#[derive(Clone, Debug)]
pub struct GroupSimulator {
    groups: Vec<SimulatedGroup>,
    size: usize,
    segment_size: i32,
    segments: Vec<Segment>,
    max_resource_health: f32,
    my_score_gained: f32,
    opponent_score_gained: f32,
}

impl GroupSimulator {
    pub fn new(groups: &Vec<Group>, segment_size: i32, world: &World) -> Self {
        let size = (world.map_size() / segment_size) as usize;
        let mut segments: Vec<Segment> = std::iter::repeat(Segment::default()).take(size * size).collect();
        let mut group_units = Vec::new();
        for group in groups.iter() {
            for unit in group.units() {
                group_units.push(*unit);
            }
        }
        for i in 0..segments.len() {
            let position = index_to_position(i, size) * segment_size;
            let segment = &mut segments[i];
            world.visit_map_square(position, segment_size, |_, tile, _| {
                if let Tile::Entity(entity_id) = tile {
                    let entity = world.get_entity(entity_id);
                    if matches!(entity.entity_type, EntityType::Resource) {
                        segment.resource_health += entity.health as f32;
                    } else {
                        let is_my = entity.player_id == Some(world.my_id());
                        if is_my && group_units.iter().any(|v| *v == entity.id) {
                            return;
                        }
                        let is_opponent = entity.player_id.is_some() && entity.player_id != Some(world.my_id());
                        let properties = world.get_entity_properties(&entity.entity_type);
                        if let Some(attack) = properties.attack.as_ref() {
                            if is_opponent {
                                segment.opponent_damage += (attack.damage * segment_size) as f32;
                            } else if is_my {
                                segment.my_damage += (attack.damage * segment_size) as f32;
                            }
                        }
                        if is_opponent {
                            segment.opponent_destroy_score += properties.destroy_score as f32;
                            segment.opponent_health += entity.health as f32;
                        } else if is_my {
                            segment.my_destroy_score += properties.destroy_score as f32;
                            segment.my_health += entity.health as f32;
                        }
                    }
                }
            })
        }
        Self {
            groups: groups.iter()
                .map(|v| SimulatedGroup {
                    id: v.id(),
                    position: v.position() / segment_size,
                    destroy_score: v.destroy_score() as f32,
                    damage: (v.damage() * segment_size) as f32,
                    health: v.health() as f32,
                    move_direction: Vec2i::zero(),
                })
                .collect(),
            size,
            segment_size,
            segments,
            max_resource_health: world.get_entity_properties(&EntityType::Resource).max_health as f32,
            my_score_gained: 0.0,
            opponent_score_gained: 0.0,
        }
    }

    pub fn segment_size(&self) -> i32 {
        self.segment_size
    }

    pub fn my_score_gained(&self) -> f32 {
        self.my_score_gained
    }

    pub fn opponent_score_gained(&self) -> f32 {
        self.opponent_score_gained
    }

    pub fn groups(&self) -> &Vec<SimulatedGroup> {
        &self.groups
    }

    pub fn contains_position(&self, position: Vec2i) -> bool {
        0 <= position.x() && position.x() < self.size as i32
            && 0 <= position.y() && position.y() < self.size as i32
    }

    pub fn move_group_to(&mut self, group_id: u32, direction: Vec2i) {
        if let Some(group) = self.groups.iter_mut().find(|v| v.id == group_id) {
            group.move_direction = direction;
        }
    }

    pub fn simulate(&mut self) {
        for segment_index in 0..self.segments.len() {
            if self.segments[segment_index].opponent_health == 0.0 {
                continue;
            }
            let mut my_damage = self.segments[segment_index].my_damage;
            let mut my_destroy_score = self.segments[segment_index].my_destroy_score;
            let mut my_health = self.segments[segment_index].my_health;
            let position = index_to_position(segment_index, self.size);
            for group in self.groups.iter() {
                if group.position == position {
                    my_damage += group.damage;
                    my_destroy_score += group.destroy_score;
                    my_health += group.health;
                }
            }
            if my_health == 0.0 {
                continue;
            }
            let my_new_health = my_health - self.segments[segment_index].opponent_damage;
            let opponent_new_health = self.segments[segment_index].opponent_health - my_damage;
            let my_damage_left = (my_damage - self.segments[segment_index].opponent_health).max(0.0);
            let my_new_destroy_score = (my_destroy_score * my_new_health) / my_health;
            let opponent_new_destroy_score = (self.segments[segment_index].opponent_destroy_score * opponent_new_health) / self.segments[segment_index].opponent_health;
            let my_new_damage = my_damage;
            let opponent_new_damage = (self.segments[segment_index].opponent_damage * opponent_new_health) / self.segments[segment_index].opponent_health;
            self.opponent_score_gained = my_destroy_score - my_new_destroy_score;
            self.my_score_gained = self.segments[segment_index].opponent_destroy_score - opponent_new_destroy_score;
            self.segments[segment_index].resource_health -= my_damage_left;
            self.segments[segment_index].opponent_health = opponent_new_health.max(0.0);
            self.segments[segment_index].opponent_destroy_score = opponent_new_destroy_score.max(0.0);
            self.segments[segment_index].opponent_damage = opponent_new_damage.max(0.0);
            if self.segments[segment_index].my_health > 0.0 {
                let factor = self.segments[segment_index].my_health / my_health;
                self.segments[segment_index].my_health = (my_new_health * factor).max(0.0);
                self.segments[segment_index].my_destroy_score = (my_new_destroy_score * factor).max(0.0);
                self.segments[segment_index].my_damage = (my_new_damage * factor).max(0.0);
            }
            for group in self.groups.iter_mut() {
                if group.position == position {
                    let factor = group.health / my_health;
                    group.health = my_new_health * factor;
                    group.destroy_score = my_new_destroy_score * factor;
                    group.damage = my_new_damage * factor;
                }
            }
        }
        self.groups.retain(|v| v.health > 0.0);
        for group in self.groups.iter_mut() {
            group.position += group.move_direction;
        }
    }
}
