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
    pub entity_plan_max_transitions: usize,
    pub entity_plan_max_cost_per_tick: usize,
    pub entity_plan_max_total_cost: usize,
    pub min_player_inactive_ticks: i32,
    pub engage_distance: i32,
    pub battle_plan_min_depth: usize,
    pub battle_plan_max_depth: usize,
    pub battle_plan_max_transitions: usize,
    pub battle_plan_max_cost_per_tick: usize,
    pub battle_plan_max_total_cost: usize,
}

impl Config {
    pub fn new() -> Self {
        Self {
            entity_plan_min_depth: 1,
            entity_plan_max_depth: 4,
            entity_plan_max_transitions: 200,
            entity_plan_max_cost_per_tick: 100000,
            entity_plan_max_total_cost: 10000000,
            min_player_inactive_ticks: 5,
            engage_distance: 1,
            battle_plan_min_depth: 1,
            battle_plan_max_depth: 7,
            battle_plan_max_transitions: 100000,
            battle_plan_max_cost_per_tick: 1000000,
            battle_plan_max_total_cost: 100000000,
        }
    }
}
