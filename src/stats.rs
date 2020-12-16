use std::time::Duration;

#[cfg(feature = "write_stats")]
use serde::Serialize;

#[derive(Debug)]
#[cfg_attr(feature = "write_stats", derive(Serialize))]
pub struct StatsResult {
    pub entity_planner_iterations: usize,
    pub find_hidden_path_calls: usize,
    pub path_updates: usize,
    pub last_tick_entity_planner_iterations: usize,
    pub last_tick_duration: Duration,
    pub max_tick_duration: Duration,
    pub max_tick_duration_entity_planner_iterations: usize,
    pub last_entities_to_plan: usize,
    pub max_entities_to_plan: usize,
    pub max_entity_planner_iterations_per_entity: usize,
}

#[derive(Default)]
pub struct Stats {
    entity_planner_iterations: usize,
    find_hidden_path_calls: usize,
    reachability_updates: usize,
    last_tick_entity_planner_iterations: usize,
    last_tick_duration: Duration,
    max_tick_duration: Duration,
    max_tick_duration_entity_planner_iterations: usize,
    last_entities_to_plan: usize,
    max_tick_duration_entities_to_plan: usize,
    max_tick_duration_entity_planner_iterations_per_entity: usize,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            entity_planner_iterations: 0,
            find_hidden_path_calls: 0,
            reachability_updates: 0,
            last_tick_entity_planner_iterations: 0,
            last_tick_duration: Duration::new(0, 0),
            max_tick_duration: Duration::new(0, 0),
            max_tick_duration_entity_planner_iterations: 0,
            last_entities_to_plan: 0,
            max_tick_duration_entities_to_plan: 0,
            max_tick_duration_entity_planner_iterations_per_entity: 0,
        }
    }

    pub fn get_result(&self) -> StatsResult {
        StatsResult {
            entity_planner_iterations: self.entity_planner_iterations,
            find_hidden_path_calls: self.find_hidden_path_calls,
            path_updates: self.reachability_updates,
            last_tick_entity_planner_iterations: self.last_tick_entity_planner_iterations,
            last_tick_duration: self.last_tick_duration,
            max_tick_duration: self.max_tick_duration,
            max_tick_duration_entity_planner_iterations: self.max_tick_duration_entity_planner_iterations,
            last_entities_to_plan: self.last_entities_to_plan,
            max_entities_to_plan: self.max_tick_duration_entities_to_plan,
            max_entity_planner_iterations_per_entity: self.max_tick_duration_entity_planner_iterations_per_entity,
        }
    }

    pub fn entity_planner_iterations(&self) -> usize {
        self.entity_planner_iterations
    }

    pub fn add_entity_planner_iterations(&mut self, number: usize) {
        self.entity_planner_iterations += number;
        self.last_tick_entity_planner_iterations += number;
    }

    pub fn add_find_hidden_path_calls(&mut self, number: usize) {
        self.find_hidden_path_calls += number;
    }

    pub fn add_path_updates(&mut self, number: usize) {
        self.reachability_updates += number;
    }

    pub fn reset_last_tick_entity_planner_iterations(&mut self) {
        self.last_tick_entity_planner_iterations = 0;
    }

    pub fn set_last_tick_duration(&mut self, value: Duration) {
        self.last_tick_duration = value;
        if self.max_tick_duration < value {
            self.max_tick_duration = value;
            self.max_tick_duration_entity_planner_iterations = self.last_tick_entity_planner_iterations;
            if self.last_entities_to_plan > 0 {
                self.max_tick_duration_entity_planner_iterations_per_entity = self.last_tick_entity_planner_iterations / self.last_entities_to_plan;
                self.max_tick_duration_entities_to_plan = self.last_entities_to_plan;
            }
        }
    }

    pub fn add_entities_to_plan(&mut self, value: usize) {
        self.last_entities_to_plan = value;
    }
}
