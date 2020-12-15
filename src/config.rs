#[cfg(feature = "read_config")]
use serde::Deserialize;
#[cfg(feature = "print_config")]
use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "read_config", derive(Deserialize))]
#[cfg_attr(feature = "print_config", derive(Serialize))]
pub struct Config {
    pub entity_plan_min_depth: usize,
    pub entity_plan_max_depth: usize,
    pub entity_plan_max_iterations: usize,
    pub min_player_inactive_ticks: i32,
}

impl Config {
    pub fn new() -> Self {
        Self {
            entity_plan_min_depth: 1,
            entity_plan_max_depth: 4,
            entity_plan_max_iterations: 200,
            min_player_inactive_ticks: 5,
        }
    }
}
