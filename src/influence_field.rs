#[cfg(feature = "enable_debug")]
use crate::my_strategy::{
    debug,
    Vec2f,
};
use crate::my_strategy::{Field, index_to_position, position_to_index, Vec2i, World};

pub struct InfluenceField {
    size: i32,
    influence_score: Vec<f32>,
    control: Vec<Option<i32>>,
}

impl InfluenceField {
    pub fn new(size: i32) -> Self {
        Self {
            size,
            influence_score: std::iter::repeat(0.0).take((size * size) as usize).collect(),
            control: std::iter::repeat(None).take((size * size) as usize).collect(),
        }
    }

    pub fn get_score(&self, position: Vec2i) -> f32 {
        self.influence_score[position_to_index(position, self.size as usize)]
    }

    pub fn get_player(&self, position: Vec2i) -> Option<i32> {
        self.control[position_to_index(position, self.size as usize)]
    }

    pub fn update(&mut self, field: &Field, world: &World) {
        for v in self.influence_score.iter_mut() {
            *v = 0.0;
        }
        for v in self.control.iter_mut() {
            *v = None;
        }
        for player in world.players().iter() {
            let player_index = field.get_player_index(player.id);
            for i in 0..self.influence_score.len() {
                let position = index_to_position(i, self.size as usize);
                let score = field.get_player_influence_score(position, player_index);
                if self.influence_score[i] < score {
                    self.influence_score[i] = score;
                    self.control[i] = Some(player.id);
                }
            }
        }
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, debug: &mut debug::Debug) {
        use std::collections::{BTreeMap, btree_map};

        let mut total_influence_count = BTreeMap::new();
        for i in 0..self.control.len() {
            if let Some(player_id) = self.control[i] {
                debug.add_world_square(
                    Vec2f::from(index_to_position(i, self.size as usize)),
                    1.0 as f32,
                    debug::get_player_color(0.2, player_id),
                );
                match total_influence_count.entry(player_id) {
                    btree_map::Entry::Vacant(v) => {
                        v.insert(self.influence_score[i]);
                    }
                    btree_map::Entry::Occupied(mut v) => {
                        *v.get_mut() += self.influence_score[i];
                    }
                }
            }
        }
        debug.add_static_text(format!("Influence by player: {:?}", total_influence_count));
    }
}
