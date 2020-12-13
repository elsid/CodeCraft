#[cfg(feature = "enable_debug")]
use model::Color;

#[cfg(feature = "enable_debug")]
use crate::my_strategy::{
    debug,
    Vec2f,
};
use crate::my_strategy::{Config, Field, field_function, Group, index_to_position, position_to_index, Rect, Vec2i, visit_range};

pub struct GroupField {
    group_id: u32,
    size: i32,
    config: Config,
    segment_scores: Vec<f32>,
    shift: Vec2i,
}

impl GroupField {
    pub fn new(group_id: u32, map_size: i32, config: Config) -> Self {
        let size = map_size / config.segment_size + 2;
        Self {
            group_id,
            size,
            config,
            segment_scores: std::iter::repeat(0.0).take((size * size) as usize).collect(),
            shift: Vec2i::zero(),
        }
    }

    pub fn group_id(&self) -> u32 {
        self.group_id
    }

    pub fn size(&self) -> i32 {
        self.size
    }

    pub fn segment_size(&self) -> i32 {
        self.config.segment_size
    }

    pub fn shift(&self) -> Vec2i {
        self.shift
    }

    pub fn get_score(&self, segment_position: Vec2i) -> f32 {
        assert!(Rect::new(Vec2i::zero(), Vec2i::both(self.size as i32)).contains(segment_position + Vec2i::both(1)), "{:?}", segment_position);
        self.segment_scores[position_to_index(segment_position + Vec2i::both(1), self.size as usize)]
    }

    pub fn update(&mut self, field: &Field, groups: &Vec<Group>) {
        for v in self.segment_scores.iter_mut() {
            *v = 0.0;
        }
        let group = groups.iter().find(|group| group.id() == self.group_id).unwrap();
        if group.is_empty() {
            return;
        }
        self.shift = group.position() % self.config.segment_size - Vec2i::both(self.config.segment_size / 2);
        let map_size = (self.size - 2) * self.config.segment_size;
        let bounds = Rect::new(Vec2i::zero(), Vec2i::both(map_size));
        for i in 0..self.segment_scores.len() {
            let segment_position = index_to_position(i, self.size as usize) - Vec2i::both(1);
            let position = segment_position * self.config.segment_size + self.shift;
            let mut sum_score = 0.0;
            let mut visited = 0;
            visit_range(position, self.config.segment_size, group.sight_range() / 2, &bounds, |sub_position| {
                let field_score = field.get_score(sub_position);
                sum_score += field_function(
                    sub_position.distance(position) as f32,
                    field_score,
                    (group.sight_range() / 2) as f32,
                );
                visited += 1;
            });
            if visited != 0 {
                self.segment_scores[i] = sum_score / visited as f32;
            }
        }
        let min_score = self.segment_scores.iter()
            .fold(std::f32::MAX, |r, value| r.min(*value));
        let max_score = self.segment_scores.iter()
            .fold(-std::f32::MAX, |r, value| r.max(*value));
        let norm = max_score - min_score;
        for value in self.segment_scores.iter_mut() {
            let normalized = (*value - min_score) / norm;
            *value = normalized;
        }
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, debug: &mut debug::Debug) {
        let norm: f32 = self.segment_scores.iter().sum();
        for i in 0..self.segment_scores.len() {
            let position = (index_to_position(i, self.size as usize) - Vec2i::both(1)) * self.config.segment_size + self.shift;
            debug.add_world_square(
                Vec2f::from(position),
                self.config.segment_size as f32,
                debug::color_from_heat(0.25, self.segment_scores[i] / norm),
            );
            debug.add_world_text(
                format!("{}", self.segment_scores[i]),
                Vec2f::from(position) + Vec2f::both(self.config.segment_size as f32 / 2.0),
                Vec2f::zero(),
                Color { a: 1.0, r: 0.5, g: 0.0, b: 0.0 },
            );
        }
    }
}
