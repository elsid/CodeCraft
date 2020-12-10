#[cfg(feature = "enable_debug")]
use model::Color;
use model::Entity;

#[cfg(feature = "enable_debug")]
use crate::my_strategy::{
    color_from_heat,
    debug,
    Vec2f,
    Rect,
};
use crate::my_strategy::{Field, position_to_index, Positionable, Tile, Vec2i, visit_range, World};

pub struct EntityField {
    size: i32,
    field_scores: Vec<f32>,
    area_field_scores: Vec<f32>,
}

impl EntityField {
    pub fn new(map_size: i32) -> Self {
        Self {
            size: map_size,
            field_scores: std::iter::repeat(0.0).take((map_size * map_size) as usize).collect(),
            area_field_scores: std::iter::repeat(0.0).take((map_size * map_size) as usize).collect(),
        }
    }

    pub fn get_position_score(&self, position: Vec2i) -> f32 {
        self.area_field_scores[position_to_index(position, self.size as usize)]
    }

    pub fn update(&mut self, entity: &Entity, field: &Field, world: &World) {
        for v in self.field_scores.iter_mut() {
            *v = 0.0;
        }
        for v in self.area_field_scores.iter_mut() {
            *v = 0.0;
        }
        let bounds = world.bounds();
        let properties = world.get_entity_properties(&entity.entity_type);
        visit_range(entity.position(), properties.size, properties.sight_range, &bounds, |position| {
            self.field_scores[position_to_index(position, self.size as usize)] = field.get_entity_score(position, entity, world);
        });
        visit_range(entity.position(), properties.size, properties.sight_range, &bounds, |position| {
            if let Tile::Entity(entity_id) = world.get_tile(position) {
                if entity_id != entity.id {
                    return;
                }
            }
            self.area_field_scores[position_to_index(position, self.size as usize)] = if let Some(attack) = properties.attack.as_ref() {
                let mut sum_score = 0.0;
                let mut visited = 0;
                visit_range(position, properties.size, attack.attack_range, &bounds, |sub_position| {
                    sum_score += self.field_scores[position_to_index(sub_position, self.size as usize)];
                    visited += 1;
                });
                sum_score / visited as f32
            } else {
                self.field_scores[position_to_index(position, self.size as usize)]
            }
        });
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, entity: &Entity, debug: &mut debug::Debug) {
        let mut min_score = std::f32::MAX;
        let mut max_score = -std::f32::MAX;
        for score in self.area_field_scores.iter() {
            min_score = min_score.min(*score);
            max_score = max_score.max(*score);
        };
        let norm = (max_score - min_score).max(1.0);
        let bounds = Rect::new(Vec2i::zero(), Vec2i::both(self.size));
        visit_range(entity.position(), 1, 11, &bounds, |position| {
            let score = self.area_field_scores[position_to_index(position, self.size as usize)];
            debug.add_world_square(
                Vec2f::from(position),
                1.0,
                color_from_heat(0.25, ((score - min_score) / norm) as f32),
            );
            debug.add_world_text(
                format!("{}", score),
                position.center(),
                Vec2f::zero(),
                Color { a: 1.0, r: 0.5, g: 0.0, b: 0.0 },
            );
        });
        debug.add_static_text(format!("Field: [{}, {}] for entity {}", min_score, max_score, entity.id));
    }
}
