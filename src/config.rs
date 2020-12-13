#[cfg(feature = "read_config")]
use serde::Deserialize;
#[cfg(feature = "print_config")]
use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "read_config", derive(Deserialize))]
#[cfg_attr(feature = "print_config", derive(Serialize))]
pub struct Config {
    pub segment_size: i32,
    pub resource_weight: f32,
    pub opponent_dynamic_sight_range_power_weight: f32,
    pub opponent_static_sight_range_power_weight: f32,
    pub opponent_dynamic_attack_range_power_weight: f32,
    pub opponent_static_attack_range_power_weight: f32,
    pub opponent_dynamic_military_destroy_score_weight: f32,
    pub opponent_dynamic_economy_destroy_score_weight: f32,
    pub opponent_static_destroy_score_weight: f32,
    pub opponent_dynamic_sight_score_weight: f32,
    pub opponent_static_sight_score_weight: f32,
    pub my_static_sight_range_power_weight: f32,
    pub my_static_attack_range_power_weight: f32,
    pub my_dynamic_economy_destroy_score_weight: f32,
    pub my_static_destroy_score_weight: f32,
    pub group_min_score_ratio: f32,
    pub entity_min_score_ratio: f32,
    pub group_plan_max_depth: usize,
    pub group_plan_max_iterations: usize,
    pub entity_plan_min_depth: usize,
    pub entity_plan_max_depth: usize,
    pub entity_plan_max_iterations: usize,
}

impl Config {
    pub fn new() -> Self {
        Self {
            segment_size: 5,
            resource_weight: -1.0,
            opponent_dynamic_sight_range_power_weight: -1.0,
            opponent_static_sight_range_power_weight: -1.0,
            opponent_dynamic_attack_range_power_weight: -1.0,
            opponent_static_attack_range_power_weight: -1.0,
            opponent_dynamic_military_destroy_score_weight: 1.0,
            opponent_dynamic_economy_destroy_score_weight: 1.0,
            opponent_static_destroy_score_weight: 1.0,
            opponent_dynamic_sight_score_weight: -1.0,
            opponent_static_sight_score_weight: -1.0,
            my_static_sight_range_power_weight: 1.0,
            my_static_attack_range_power_weight: 1.0,
            my_dynamic_economy_destroy_score_weight: 1.0,
            my_static_destroy_score_weight: 1.0,
            group_min_score_ratio: 0.1,
            entity_min_score_ratio: 0.3,
            group_plan_max_depth: 8,
            group_plan_max_iterations: 1000,
            entity_plan_min_depth: 1,
            entity_plan_max_depth: 4,
            entity_plan_max_iterations: 200,
        }
    }
}
