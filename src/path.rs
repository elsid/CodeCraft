use std::collections::{BinaryHeap, VecDeque};

#[cfg(feature = "enable_debug")]
use model::Color;
use model::EntityType;

use crate::my_strategy::{index_to_position, position_to_index, Positionable, Rect, Tile, Vec2i, World};
#[cfg(feature = "enable_debug")]
use crate::my_strategy::debug;

#[derive(Debug)]
pub struct ReachabilityMap {
    map_size: usize,
    reachable: Vec<bool>,
}

impl ReachabilityMap {
    pub fn new(map_size: usize) -> Self {
        Self {
            map_size,
            reachable: std::iter::repeat(false).take(map_size * map_size).collect(),
        }
    }

    pub fn is_reachable(&self, dst: Vec2i) -> bool {
        self.reachable[position_to_index(dst, self.map_size)]
    }

    pub fn update(&mut self, start: Vec2i, is_passable: &Vec<bool>) {
        for value in self.reachable.iter_mut() {
            *value = false;
        }
        let start_index = position_to_index(start, self.map_size);
        self.reachable[start_index] = true;

        let mut discovered: Vec<Vec2i> = Vec::new();
        discovered.push(start);

        const EDGES: &[Vec2i] = &[
            Vec2i::only_x(1),
            Vec2i::only_x(-1),
            Vec2i::only_y(1),
            Vec2i::only_y(-1),
        ];

        let bounds = Rect::new(Vec2i::zero(), Vec2i::both(self.map_size as i32));

        while let Some(node_position) = discovered.pop() {
            let node_index = position_to_index(node_position, self.map_size);
            self.reachable[node_index] = true;
            for &shift in EDGES.iter() {
                let neighbor_position = node_position + shift;
                if !bounds.contains(neighbor_position) {
                    continue;
                }
                let neighbor_index = position_to_index(neighbor_position, self.map_size);
                if self.reachable[neighbor_index] || !is_passable[neighbor_index] {
                    continue;
                }
                self.reachable[neighbor_index] = true;
                discovered.push(neighbor_position);
            }
        }
    }
}

pub fn visit_reversed_shortest_path<F: FnMut(usize)>(src: usize, dst: usize, backtrack: &Vec<usize>, mut visit: F) {
    if src == dst {
        return;
    }
    let mut index = dst;
    loop {
        let prev = backtrack[index];
        visit(index);
        if prev == src {
            break;
        }
        index = prev;
    }
}

pub trait FindPathTarget {
    fn has_reached(&self, position: Vec2i) -> bool;

    fn get_distance(&self, position: Vec2i) -> i32;
}

pub struct PathFinder {
    start: Vec2i,
    map_size: usize,
    costs: Vec<i32>,
    backtrack: Vec<usize>,
    destination: Option<usize>,
    path: Vec<Vec2i>,
}

impl PathFinder {
    pub fn new(start: Vec2i, map_size: usize) -> Self {
        Self {
            start,
            map_size,
            costs: std::iter::repeat(std::i32::MAX)
                .take(map_size * map_size)
                .collect(),
            backtrack: (0..(map_size * map_size)).into_iter().collect(),
            destination: None,
            path: Vec::new(),
        }
    }

    pub fn path(&self) -> &Vec<Vec2i> {
        &self.path
    }

    pub fn cost(&self) -> Option<i32> {
        self.destination.map(|v| self.costs[v])
    }

    pub fn find_with_a_star<T: FindPathTarget>(&mut self, target: &T, find_nearest: bool, damage: i32, world: &World) {
        self.reset();

        let size = self.map_size;
        let mut open: Vec<bool> = std::iter::repeat(true)
            .take(size * size)
            .collect();
        let mut discovered = BinaryHeap::new();
        let src_index = position_to_index(self.start, size);
        let bounds = world.bounds();

        self.costs[src_index] = 0;
        let mut min_distance = target.get_distance(self.start);
        discovered.push((-min_distance, src_index));

        while let Some((_, node_index)) = discovered.pop() {
            let node_position = index_to_position(node_index, size);
            let reached = target.has_reached(node_position);
            let distance = target.get_distance(node_position);
            if reached || min_distance > distance && find_nearest {
                min_distance = distance;
                self.destination = Some(node_index);
                if reached {
                    break;
                }
            }
            open[node_index] = true;
            for &shift in EDGES.iter() {
                let neighbour_position = node_position + shift;
                if !bounds.contains(neighbour_position) {
                    continue;
                }
                if let Some(cost) = self.get_cost(neighbour_position, damage, world) {
                    let new_cost = self.costs[node_index] + cost;
                    let neighbour_index = position_to_index(neighbour_position, size);
                    if self.costs[neighbour_index] <= new_cost {
                        continue;
                    }
                    self.costs[neighbour_index] = new_cost;
                    self.backtrack[neighbour_index] = node_index;
                    if !open[neighbour_index] {
                        continue;
                    }
                    open[neighbour_index] = false;
                    let new_score = new_cost + target.get_distance(neighbour_position);
                    discovered.push((-new_score, neighbour_index));
                }
            }
        }

        self.reconstruct_path();
    }

    pub fn find_with_bfs<T: FindPathTarget>(&mut self, target: &T, find_nearest: bool, damage: i32, world: &World) {
        self.reset();

        let size = self.map_size;
        let mut discovered = VecDeque::new();
        let src_index = position_to_index(self.start, size);
        let bounds = world.bounds();

        self.costs[src_index] = 0;
        discovered.push_back(src_index);
        let mut min_distance = target.get_distance(self.start);

        while let Some(node_index) = discovered.pop_front() {
            let node_position = index_to_position(node_index, size);
            let reached = target.has_reached(node_position);
            let distance = target.get_distance(node_position);
            if reached || min_distance > distance && find_nearest {
                min_distance = distance;
                self.destination = Some(node_index);
                if reached {
                    break;
                }
            }
            for &shift in EDGES.iter() {
                let neighbour_position = node_position + shift;
                if !bounds.contains(neighbour_position) {
                    continue;
                }
                let neighbour_index = position_to_index(neighbour_position, size);
                if self.costs[neighbour_index] != std::i32::MAX {
                    continue;
                }
                if let Some(cost) = self.get_cost(neighbour_position, damage, world) {
                    self.costs[neighbour_index] = self.costs[node_index] + cost;
                    self.backtrack[neighbour_index] = node_index;
                    discovered.push_back(neighbour_index);
                }
            }
        }

        self.reconstruct_path();
    }

    fn reset(&mut self) {
        for value in self.costs.iter_mut() {
            *value = std::i32::MAX;
        }

        for i in 0..self.backtrack.len() {
            self.backtrack[i] = i;
        }

        self.destination = None;
        self.path.clear();
    }

    fn get_cost(&self, position: Vec2i, damage: i32, world: &World) -> Option<i32> {
        if world.is_tile_locked(position) {
            return None;
        }
        match world.get_tile(position) {
            Tile::Entity(entity_id) => {
                let entity = world.get_entity(entity_id);
                if world.get_entity_properties(&entity.entity_type).can_move {
                    if !world.has_move_from(entity.position()) {
                        return None;
                    }
                } else {
                    if matches!(entity.entity_type, EntityType::Resource) && damage > 0 {
                        return Some(1 + entity.health / damage + (entity.health % damage != 0) as i32);
                    }
                    return None;
                }
            }
            _ => (),
        }
        if world.has_move_to(position) {
            return None;
        }
        Some(1)
    }

    fn reconstruct_path(&mut self) {
        if let Some(dst) = self.destination {
            let size = self.map_size;
            let src = position_to_index(self.start, size);
            let path = &mut self.path;
            visit_reversed_shortest_path(src, dst, &self.backtrack, |index| {
                path.push(index_to_position(index, size));
            });
            self.path.reverse();
        }
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, debug: &mut debug::Debug) {
        if self.path.is_empty() {
            return;
        }
        debug.add_world_line(
            self.start.center(),
            self.path[0].center(),
            Color { a: 1.0, r: 0.0, g: 1.0, b: 0.0 },
        );
        for i in 1..self.path.len() {
            debug.add_world_line(
                self.path[i - 1].center(),
                self.path[i].center(),
                Color { a: 1.0, r: 0.0, g: 1.0, b: 0.0 },
            );
        }
    }
}

pub const EDGES: &[Vec2i] = &[
    Vec2i::only_x(1),
    Vec2i::only_x(-1),
    Vec2i::only_y(1),
    Vec2i::only_y(-1),
];
