#[cfg(feature = "write_stats")]
use serde::Serialize;

#[derive(Debug)]
#[cfg_attr(feature = "write_stats", derive(Serialize))]
pub struct StatsResult {
    pub entity_planner_iterations: usize,
}

#[derive(Default)]
pub struct Stats {
    entity_planner_iterations: usize,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            entity_planner_iterations: 0,
        }
    }

    pub fn get_result(&self) -> StatsResult {
        StatsResult {
            entity_planner_iterations: self.entity_planner_iterations,
        }
    }

    pub fn add_entity_planner_iterations(&mut self, number: usize) {
        self.entity_planner_iterations += number;
    }
}
