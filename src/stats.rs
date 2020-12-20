use std::time::Duration;

#[cfg(feature = "write_stats")]
use serde::Serialize;

#[derive(Debug)]
#[cfg_attr(feature = "write_stats", derive(Serialize))]
pub struct StatsResult {
    pub total_entity_plan_cost: usize,
    pub find_hidden_path_calls: usize,
    pub reachability_updates: usize,
    pub last_tick_duration: Duration,
    pub max_tick_duration: Duration,
    pub last_tick_entity_plan_cost: usize,
    pub max_tick_entity_plan_cost: usize,
}

#[derive(Default)]
pub struct Stats {
    total_entity_plan_cost: usize,
    find_hidden_path_calls: usize,
    reachability_updates: usize,
    last_tick_duration: Duration,
    max_tick_duration: Duration,
    last_tick_entity_plan_cost: usize,
    max_tick_entity_plan_cost: usize,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            total_entity_plan_cost: 0,
            find_hidden_path_calls: 0,
            reachability_updates: 0,
            last_tick_duration: Duration::new(0, 0),
            max_tick_duration: Duration::new(0, 0),
            last_tick_entity_plan_cost: 0,
            max_tick_entity_plan_cost: 0,
        }
    }

    pub fn get_result(&self) -> StatsResult {
        StatsResult {
            total_entity_plan_cost: self.total_entity_plan_cost,
            find_hidden_path_calls: self.find_hidden_path_calls,
            reachability_updates: self.reachability_updates,
            last_tick_duration: self.last_tick_duration,
            max_tick_duration: self.max_tick_duration,
            last_tick_entity_plan_cost: self.last_tick_entity_plan_cost,
            max_tick_entity_plan_cost: self.max_tick_entity_plan_cost,
        }
    }

    pub fn total_entity_plan_cost(&self) -> usize {
        self.total_entity_plan_cost
    }

    pub fn add_entity_plan_cost(&mut self, number: usize) {
        self.total_entity_plan_cost += number;
        self.last_tick_entity_plan_cost = number;
    }

    pub fn add_find_hidden_path_calls(&mut self, number: usize) {
        self.find_hidden_path_calls += number;
    }

    pub fn add_path_updates(&mut self, number: usize) {
        self.reachability_updates += number;
    }

    pub fn set_last_tick_duration(&mut self, value: Duration) {
        self.last_tick_duration = value;
        if self.max_tick_duration < value {
            self.max_tick_duration = value;
            self.max_tick_entity_plan_cost = self.last_tick_entity_plan_cost;
        }
    }
}
