#[cfg(feature = "write_stats")]
use serde::Serialize;

#[derive(Debug)]
#[cfg_attr(feature = "write_stats", derive(Serialize))]
pub struct StatsResult {
    pub entity_planner_iterations: usize,
    pub find_hidden_path_calls: usize,
    pub path_updates: usize,
}

#[derive(Default)]
pub struct Stats {
    entity_planner_iterations: usize,
    find_hidden_path_calls: usize,
    reachability_updates: usize,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            entity_planner_iterations: 0,
            find_hidden_path_calls: 0,
            reachability_updates: 0,
        }
    }

    pub fn get_result(&self) -> StatsResult {
        StatsResult {
            entity_planner_iterations: self.entity_planner_iterations,
            find_hidden_path_calls: self.find_hidden_path_calls,
            path_updates: self.reachability_updates,
        }
    }

    pub fn add_entity_planner_iterations(&mut self, number: usize) {
        self.entity_planner_iterations += number;
    }

    pub fn add_find_hidden_path_calls(&mut self, number: usize) {
        self.find_hidden_path_calls += number;
    }

    pub fn add_path_updates(&mut self, number: usize) {
        self.reachability_updates += number;
    }
}
