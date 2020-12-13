use std::collections::VecDeque;

#[cfg(feature = "enable_debug")]
use model::Color;

use crate::my_strategy::{Config, Group, GroupField, index_to_position, position_to_index, Range, Rect, Vec2i, visit_reversed_shortest_path};
#[cfg(feature = "enable_debug")]
use crate::my_strategy::{
    debug,
    Vec2f,
};

#[derive(Default, Debug, Clone)]
pub struct GroupPlan {
    pub transitions: Vec<Vec2i>,
    pub cost: f32,
}

pub struct GroupPlanner {
    group_id: u32,
    size: usize,
    shift: Vec2i,
    costs: Vec<f32>,
    backtrack: Vec<usize>,
    plan: GroupPlan,
    config: Config,
}

impl GroupPlanner {
    pub fn new(group_id: u32, config: Config) -> Self {
        Self {
            group_id,
            size: 0,
            shift: Vec2i::zero(),
            costs: Vec::new(),
            backtrack: Vec::new(),
            plan: GroupPlan::default(),
            config,
        }
    }

    pub fn group_id(&self) -> u32 {
        self.group_id
    }

    pub fn plan(&self) -> &GroupPlan {
        &self.plan
    }

    pub fn reset(&mut self) {
        self.plan = GroupPlan::default();
    }

    pub fn update(&mut self, groups: &Vec<Group>, group_field: &GroupField, range: &Range) {
        let group = groups.iter()
            .find(|group| group.id() == self.group_id)
            .unwrap();

        let size = group_field.size() as usize;
        self.size = size;
        if self.costs.len() != size * size {
            self.costs.resize(size * size, 0.0);
        }
        if self.backtrack.len() != size * size {
            self.backtrack.resize(size * size, 0);
        }

        for value in self.costs.iter_mut() {
            *value = std::f32::MAX;
        }
        for i in 0..self.backtrack.len() {
            self.backtrack[i] = i;
        }

        self.shift = group_field.shift();

        self.plan.cost = 0.0;
        self.plan.transitions.clear();

        let start = group.position() / self.config.segment_size;
        let start_index = position_to_index(start + Vec2i::both(1), size);
        self.costs[start_index] = group_field.get_score(start);

        let mut discovered: VecDeque<Vec2i> = VecDeque::new();
        discovered.push_back(start);

        let mut visited: Vec<bool> = std::iter::repeat(false)
            .take(self.costs.len())
            .collect();

        const EDGES: &[Vec2i] = &[
            Vec2i::only_x(1),
            Vec2i::only_x(-1),
            Vec2i::only_y(1),
            Vec2i::only_y(-1),
        ];

        let bounds = Rect::new(Vec2i::zero(), Vec2i::both(group_field.size() - 2));
        let mut min_cost = self.costs[start_index];
        let mut optimal_destination = Some(start_index);

        while let Some(node_position) = discovered.pop_front() {
            let node_index = position_to_index(node_position + Vec2i::both(1), size);
            visited[node_index] = true;
            if min_cost > self.costs[node_index] {
                min_cost = self.costs[node_index];
                optimal_destination = Some(node_index);
            }
            for &shift in EDGES.iter() {
                let neighbor_position = node_position + shift;
                if !range.contains(neighbor_position) || !bounds.contains(neighbor_position) {
                    continue;
                }
                let neighbor_index = position_to_index(neighbor_position + Vec2i::both(1), size);
                if visited[neighbor_index] {
                    continue;
                }
                let new_cost = self.costs[node_index]
                    + self.config.group_distance_to_position_cost
                    - group_field.get_score(neighbor_position);
                if self.costs[neighbor_index] > new_cost {
                    self.costs[neighbor_index] = new_cost;
                    self.backtrack[neighbor_index] = node_index;
                    discovered.push_back(neighbor_position);
                }
            }
        }

        if let Some(dst) = optimal_destination {
            let backtrack = &self.backtrack;
            let transitions = &mut self.plan.transitions;
            let segment_size = self.config.segment_size;
            let shift = self.shift;
            let bounds = Rect::new(Vec2i::zero(), Vec2i::both((self.size as i32 - 2) * segment_size));
            let success = visit_reversed_shortest_path(start_index, dst, backtrack, |index| {
                transitions.push(bounds.clip((index_to_position(index, size) - Vec2i::both(1)) * segment_size + shift));
            });
            if success {
                self.plan.cost = min_cost;
                self.plan.transitions.reverse();
            } else {
                self.plan.transitions.clear();
            }
        }
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, debug: &mut debug::Debug) {
        let mut min_cost = std::f32::MAX;
        let mut max_cost = -std::f32::MAX;
        for cost in self.costs.iter() {
            if *cost == std::f32::MAX {
                continue;
            }
            min_cost = min_cost.min(*cost);
            max_cost = max_cost.max(*cost);
        }
        let norm = (max_cost - min_cost).max(1.0);
        for i in 0..self.backtrack.len() {
            if self.costs[i] == std::f32::MAX {
                continue;
            }
            let position = (index_to_position(i, self.size) - Vec2i::both(1)) * self.config.segment_size + self.shift;
            debug.add_world_square(
                Vec2f::from(position),
                self.config.segment_size as f32,
                debug::color_from_heat(0.25, (self.costs[i] - min_cost) / norm),
            );
            debug.add_world_text(
                format!("{}", self.costs[i]),
                Vec2f::from(position) + Vec2f::both(self.config.segment_size as f32 / 2.0),
                Vec2f::zero(),
                Color { a: 1.0, r: 0.0, g: 0.0, b: 0.0 },
            );
        }
        for i in 1..self.plan.transitions.len() {
            debug.add_world_line(
                self.plan.transitions[i - 1].center(),
                self.plan.transitions[i].center(),
                Color { a: 1.0, r: 0.0, g: 1.0, b: 0.0 },
            );
        }
        debug.add_static_text(format!("Group planner: group_id={} plan={:?}", self.group_id, self.plan));
    }
}
