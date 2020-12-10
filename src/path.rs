use std::collections::BinaryHeap;

#[cfg(feature = "enable_debug")]
use model::Color;
use model::EntityType;

use crate::my_strategy::{index_to_position, position_to_index, Vec2i, Rect};
#[cfg(feature = "enable_debug")]
use crate::my_strategy::debug;

pub trait PathMap {
    fn is_passable(&self, position: Vec2i) -> bool;
}

#[derive(Debug)]
pub struct Path {
    map_size: usize,
    start: Option<Vec2i>,
    distances: Vec<i32>,
    backtrack: Vec<usize>,
}

impl Path {
    pub fn new(map_size: usize) -> Self {
        Self {
            map_size,
            start: None,
            distances: std::iter::repeat(std::i32::MAX).take(map_size * map_size).collect(),
            backtrack: (0..map_size * map_size).collect(),
        }
    }

    pub fn from_position<M: PathMap>(map_size: usize, start: Vec2i, map: &M) -> Self {
        let mut result = Self::new(map_size);
        result.update(start, map);
        result
    }

    pub fn start(&self) -> Option<Vec2i> {
        self.start
    }

    pub fn is_reachable(&self, dst: Vec2i) -> bool {
        self.distances[position_to_index(dst, self.map_size)] != std::i32::MAX
    }

    pub fn get_distance_to(&self, dst: Vec2i) -> i32 {
        self.distances[position_to_index(dst, self.map_size)]
    }

    pub fn get_first_position_to(&self, dst: Vec2i) -> Option<Vec2i> {
        let src = self.start.unwrap();
        let src_index = position_to_index(src, self.map_size);
        let mut index = position_to_index(dst, self.map_size);
        loop {
            let prev = self.backtrack[index];
            if prev == index {
                return None;
            }
            if prev == src_index {
                return Some(index_to_position(index, self.map_size));
            }
            index = prev;
        }
    }

    pub fn find_shortest_path(&self, dst: Vec2i) -> Vec<Vec2i> {
        let mut result = self.find_reversed_shortest_path(dst);
        result.reverse();
        result
    }

    pub fn find_reversed_shortest_path(&self, dst: Vec2i) -> Vec<Vec2i> {
        let src = self.start.unwrap();
        let src_index = position_to_index(src, self.map_size);
        let mut result = Vec::new();
        result.reserve(2 * self.map_size);
        let mut index = position_to_index(dst, self.map_size);
        loop {
            let prev = self.backtrack[index];
            if prev == index {
                return Vec::new();
            }
            result.push(index_to_position(index, self.map_size));
            if prev == src_index {
                break;
            }
            index = prev;
        }
        result
    }

    pub fn update<M: PathMap>(&mut self, start: Vec2i, map: &M) {
        self.update_impl(start, map);
        self.start = Some(start);
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, debug: &mut debug::Debug) {
        for i in 0..self.backtrack.len() {
            let prev = self.backtrack[i];
            if prev != i {
                debug.add_world_line(
                    index_to_position(i, self.map_size).center(),
                    index_to_position(prev, self.map_size).center(),
                    Color { a: 1.0, r: 1.0, g: 0.0, b: 0.0 },
                )
            }
        }
    }

    fn update_impl<M: PathMap>(&mut self, start: Vec2i, map: &M) {
        for value in self.distances.iter_mut() {
            *value = std::i32::MAX;
        }
        for i in 0..self.backtrack.len() {
            self.backtrack[i] = i;
        }
        let start_index = position_to_index(start, self.map_size);
        self.distances[start_index] = 0;

        let mut new: BinaryHeap<Vec2i> = BinaryHeap::new();
        new.push(start);

        let mut open: Vec<bool> = std::iter::repeat(false)
            .take(self.distances.len())
            .collect();
        open[start_index] = true;

        const EDGES: &[Vec2i] = &[
            Vec2i::only_x(1),
            Vec2i::only_x(-1),
            Vec2i::only_y(1),
            Vec2i::only_y(-1),
        ];

        let bounds = Rect::new(Vec2i::zero(), Vec2i::both(self.map_size as i32));

        while let Some(node_position) = new.pop() {
            let node_index = position_to_index(node_position, self.map_size);
            open[node_index] = false;
            for &shift in EDGES.iter() {
                let neighbor_position = node_position + shift;
                if !bounds.contains(neighbor_position) || !map.is_passable(neighbor_position) {
                    continue;
                }
                let neighbor_index = position_to_index(neighbor_position, self.map_size);
                let new_distance = self.distances[node_index] + 1;
                if new_distance < self.distances[neighbor_index] {
                    self.distances[neighbor_index] = new_distance;
                    self.backtrack[neighbor_index] = node_index;
                    if !open[neighbor_index] {
                        open[neighbor_index] = true;
                        new.push(neighbor_position);
                    }
                }
            }
        }
    }
}

pub fn is_passable(entity_type: &EntityType) -> bool {
    match entity_type {
        EntityType::House | EntityType::BuilderBase | EntityType::MeleeBase | EntityType::RangedBase | EntityType::Resource | EntityType::Turret => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Map {
        size: usize,
        passable: Vec<bool>,
    }

    impl PathMap for Map {
        fn is_passable(&self, position: Vec2i) -> bool {
            self.passable[position_to_index(position, self.size)]
        }
    }

    #[test]
    fn get_distance() {
        let map = Map {
            size: 3,
            passable: vec![
                false, true, false,
                false, true, true,
                true, true, false,
            ],
        };
        let path = Path::from_position(map.size, Vec2i::zero(), &map);
        assert_eq!(path.get_distance_to(Vec2i::new(0, 0)), 0);
        assert_eq!(path.get_distance_to(Vec2i::new(1, 0)), 1);
        assert_eq!(path.get_distance_to(Vec2i::new(2, 0)), std::i32::MAX);
        assert_eq!(path.get_distance_to(Vec2i::new(0, 1)), std::i32::MAX);
        assert_eq!(path.get_distance_to(Vec2i::new(1, 1)), 2);
        assert_eq!(path.get_distance_to(Vec2i::new(2, 1)), 3);
        assert_eq!(path.get_distance_to(Vec2i::new(0, 2)), 4);
        assert_eq!(path.get_distance_to(Vec2i::new(1, 2)), 3);
        assert_eq!(path.get_distance_to(Vec2i::new(2, 2)), std::i32::MAX);
    }

    #[test]
    fn is_reachable() {
        let map = Map {
            size: 3,
            passable: vec![
                false, true, false,
                false, true, true,
                true, true, false,
            ],
        };
        let path = Path::from_position(map.size, Vec2i::zero(), &map);
        assert!(path.is_reachable(Vec2i::new(0, 0)));
        assert!(path.is_reachable(Vec2i::new(1, 0)));
        assert!(!path.is_reachable(Vec2i::new(2, 0)));
    }

    #[test]
    fn get_first_position_to() {
        let map = Map {
            size: 3,
            passable: vec![
                false, true, false,
                false, true, true,
                true, true, false,
            ],
        };
        let path = Path::from_position(map.size, Vec2i::zero(), &map);
        assert_eq!(path.get_first_position_to(Vec2i::new(2, 1)), Some(Vec2i::new(1, 0)));
    }

    #[test]
    fn find_shortest_path() {
        let map = Map {
            size: 3,
            passable: vec![
                false, true, false,
                false, true, true,
                true, true, false,
            ],
        };
        let path = Path::from_position(map.size, Vec2i::zero(), &map);
        assert_eq!(path.find_shortest_path(Vec2i::new(0, 2)), vec![
            Vec2i::new(1, 0),
            Vec2i::new(1, 1),
            Vec2i::new(1, 2),
            Vec2i::new(0, 2),
        ]);
    }
}
