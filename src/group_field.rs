#[cfg(feature = "enable_debug")]
use model::Color;

#[cfg(feature = "enable_debug")]
use crate::my_strategy::{
    color_from_heat,
    debug,
    Vec2f,
};
use crate::my_strategy::{Config, Field, field_function, Group, index_to_position, position_to_index, Rect, Vec2i, visit_range, visit_square};

pub struct GroupField {
    group_id: u32,
    size: i32,
    config: Config,
    area_field_scores: Vec<f32>,
    segment_scores: Vec<f32>,
}

impl GroupField {
    pub fn new(group_id: u32, map_size: i32, config: Config) -> Self {
        let size = map_size / config.segment_size;
        Self {
            group_id,
            size,
            config,
            area_field_scores: std::iter::repeat(0.0).take((map_size * map_size) as usize).collect(),
            segment_scores: std::iter::repeat(0.0).take((size * size) as usize).collect(),
        }
    }

    pub fn group_id(&self) -> u32 {
        self.group_id
    }

    pub fn get_segment_position_score(&self, segment_position: Vec2i) -> f32 {
        self.segment_scores[position_to_index(segment_position, self.size as usize)]
    }

    pub fn update(&mut self, field: &Field, groups: &Vec<Group>) {
        for v in self.area_field_scores.iter_mut() {
            *v = 0.0;
        }
        for v in self.segment_scores.iter_mut() {
            *v = 0.0;
        }
        let group = groups.iter().find(|group| group.id() == self.group_id).unwrap();
        if group.is_empty() || group.power() == 0 {
            return;
        }
        let segment_size = self.config.segment_size;
        let map_size = self.size * segment_size;
        let bounds = Rect::new(Vec2i::zero(), Vec2i::both(map_size));
        for i in 0..self.area_field_scores.len() {
            let position = index_to_position(i, map_size as usize);
            let mut sum_score = 0.0;
            let mut visited = 0;
            visit_range(position, 1, group.sight_range(), &bounds, |sub_position| {
                sum_score += field_function(
                    sub_position.distance(position) as f32,
                    field.get_score(sub_position),
                    group.sight_range() as f32,
                );
                visited += 1;
            });
            self.area_field_scores[i] = sum_score / visited as f32;
        }
        for i in 0..self.segment_scores.len() {
            let segment_position = index_to_position(i, self.size as usize);
            let position = segment_position * segment_size;
            let mut sum_score = 0.0;
            let mut visited = 0;
            visit_square(position, segment_size, |tile_position| {
                sum_score += self.area_field_scores[position_to_index(tile_position, map_size as usize)];
                visited += 1;
            });
            let target_score = sum_score / visited as f32;
            self.segment_scores[i] = target_score;
        }
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, debug: &mut debug::Debug) {
        let mut min_score = std::f32::MAX;
        let mut max_score = -std::f32::MAX;
        for score in self.segment_scores.iter() {
            min_score = min_score.min(*score);
            max_score = max_score.max(*score);
        };
        let norm = (max_score - min_score).max(1.0);
        for i in 0..self.segment_scores.len() {
            let position = index_to_position(i, self.size as usize) * self.config.segment_size;
            debug.add_world_square(
                Vec2f::from(position),
                self.config.segment_size as f32,
                color_from_heat(0.25, ((self.segment_scores[i] - min_score) / norm) as f32),
            );
            debug.add_world_text(
                format!("{}", self.segment_scores[i]),
                Vec2f::from(position) + Vec2f::both(self.config.segment_size as f32 / 2.0),
                Vec2f::zero(),
                Color { a: 1.0, r: 0.5, g: 0.0, b: 0.0 },
            );
        }
        debug.add_static_text(format!("Field: [{}, {}] for group {}", min_score, max_score, self.group_id));
    }
}
