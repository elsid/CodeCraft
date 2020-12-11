use std::collections::BinaryHeap;

#[cfg(feature = "enable_debug")]
use model::Color;

use crate::my_strategy::{GroupSimulator, SimulatedGroup, Vec2i};
#[cfg(feature = "enable_debug")]
use crate::my_strategy::{
    color_from_heat,
    debug,
    Vec2f,
};

#[derive(Default, Debug)]
pub struct GroupPlan {
    pub transitions: Vec<Vec2i>,
    pub score: i64,
}

#[derive(Clone, Debug)]
struct State {
    pub depth: usize,
    pub simulator: GroupSimulator,
    pub transition: Option<usize>,
}

#[derive(Debug)]
struct GroupPlanState {
    depth: usize,
    position: Vec2i,
    destroy_score: f32,
}

pub struct GroupPlanner {
    group_id: u32,
    states: Vec<State>,
    transitions: Vec<(usize, Vec2i)>,
    visited: Vec<GroupPlanState>,
    group_plan: GroupPlan,
    max_depth: usize,
    segment_size: i32,
    optimal_final_state_index: Option<usize>,
}

impl GroupPlanner {
    pub fn new(group_id: u32, max_depth: usize, segment_size: i32) -> Self {
        Self {
            group_id,
            states: Vec::new(),
            transitions: Vec::new(),
            visited: Vec::new(),
            group_plan: GroupPlan::default(),
            max_depth,
            segment_size,
            optimal_final_state_index: None,
        }
    }

    pub fn group_id(&self) -> u32 {
        self.group_id
    }

    pub fn group_plan(&self) -> &GroupPlan {
        &self.group_plan
    }

    pub fn update(&mut self, simulator: GroupSimulator, max_iterations: usize) {
        self.states.clear();
        self.transitions.clear();
        self.visited.clear();
        self.segment_size = simulator.segment_size();
        self.states.push(State {
            depth: 0,
            simulator,
            transition: None,
        });

        let mut frontier: BinaryHeap<(i64, usize)> = BinaryHeap::new();
        frontier.push((0, 0));

        let mut max_score = std::i64::MIN;
        let mut optimal_final_state_index = None;
        let mut iteration = 0;

        while let Some((score, state_index)) = frontier.pop() {
            iteration += 1;
            let depth = self.states[state_index].depth;
            if depth >= self.max_depth {
                if max_score < score {
                    max_score = score;
                    optimal_final_state_index = Some(state_index);
                }
                continue;
            }
            if iteration >= max_iterations {
                break;
            }
            let group = if let Some(group) = self.states[state_index].simulator.groups().iter()
                .find(|v| v.id == self.group_id) {
                group.clone()
            } else {
                continue;
            };
            if self.visited.iter().any(|v| v.depth == depth && v.position == group.position && v.destroy_score == group.destroy_score) {
                continue;
            }
            self.visited.push(GroupPlanState {
                depth,
                position: group.position,
                destroy_score: group.destroy_score,
            });
            self.move_group_to(state_index, &group, Vec2i::only_x(1)).map(|v| frontier.push(v));
            self.move_group_to(state_index, &group, Vec2i::only_y(1)).map(|v| frontier.push(v));
            self.move_group_to(state_index, &group, Vec2i::only_x(-1)).map(|v| frontier.push(v));
            self.move_group_to(state_index, &group, Vec2i::only_y(-1)).map(|v| frontier.push(v));
            self.move_group_to(state_index, &group, Vec2i::zero()).map(|v| frontier.push(v));
        }

        self.optimal_final_state_index = optimal_final_state_index;
        self.group_plan = optimal_final_state_index
            .map(|state_index| GroupPlan {
                score: max_score,
                transitions: self.reconstruct_sequence(state_index),
            })
            .unwrap_or_else(|| GroupPlan::default())
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, debug: &mut debug::Debug) {
        // for (prev_state_index, direction) in self.transitions.iter() {
        //     let state = &self.states[*prev_state_index];
        //     if let Some(group) = state.simulator.groups().iter()
        //         .find(|v| v.id == group_id) {
        //         debug.add_world_line(
        //             group.position.center() * self.segment_size as f32,
        //             (group.position + *direction).center() * self.segment_size as f32,
        //             color_from_heat(1.0, state.depth as f32 / self.max_depth as f32)
        //         );
        //     }
        // }
        if self.states.is_empty() {
            return;
        }
        if let Some(max_destroy_score) = self.visited.iter().map(|v| v.destroy_score).max_by_key(|v| (*v * 1000.0) as i32) {
            for group_state in self.visited.iter() {
                debug.add_world_square(Vec2f::from(group_state.position * self.segment_size), self.segment_size as f32, color_from_heat(0.25, group_state.destroy_score / max_destroy_score))
            }
        }
        let group = self.states[0].simulator.groups().iter()
            .find(|v| v.id == self.group_id)
            .unwrap();
        let mut position = group.position;
        for transition in self.group_plan.transitions.iter() {
            debug.add_world_line(
                position.center() * self.segment_size as f32,
                (position + *transition).center() * self.segment_size as f32,
                Color { a: 1.0, r: 0.0, g: 0.0, b: 0.0 },
            );
            position += *transition;
        }
        debug.add_static_text(format!(
            "Group planner: states={} transitions={} visited={} score={} start={:?}",
            self.states.len(), self.transitions.len(), self.visited.len(), self.group_plan.score, group.position
        ));
    }

    fn move_group_to(&mut self, state_index: usize, group: &SimulatedGroup, direction: Vec2i) -> Option<(i64, usize)> {
        if !self.states[state_index].simulator.contains_position(group.position + direction) {
            return None;
        }
        let transition_index = self.transitions.len();
        self.transitions.push((state_index, direction));
        let new_state_index = self.states.len();
        self.states.push(self.states[state_index].clone());
        let new_state = &mut self.states[new_state_index];
        new_state.transition = Some(transition_index);
        new_state.simulator.move_group_to(group.id, direction);
        new_state.simulator.simulate();
        new_state.depth += 1;
        Some((
            self.states[new_state_index].simulator.my_score_gained() as i64,
            new_state_index
        ))
    }

    fn reconstruct_sequence(&self, mut state_index: usize) -> Vec<Vec2i> {
        let mut result = Vec::new();
        while let Some(transition_index) = self.states[state_index].transition {
            let (prev_state_index, direction) = self.transitions[transition_index];
            result.push(direction);
            state_index = prev_state_index;
        }
        result.reverse();
        result
    }
}
