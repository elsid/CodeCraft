#[cfg(feature = "read_config")]
use serde::Deserialize;

#[cfg(feature = "print_config")]
use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "read_config", derive(Deserialize))]
#[cfg_attr(feature = "print_config", derive(Serialize))]
pub struct Config {
    pub segment_size: i32,
    pub group_distance_to_position_weight: f32,
    pub group_my_attack_range_power_weight: f32,
    pub group_opponent_attack_range_power_weight: f32,
    pub group_my_sight_range_power_weight: f32,
    pub group_opponent_sight_range_power_weight: f32,
    pub group_my_destroy_score_weight: f32,
    pub group_my_static_destroy_score_weight: f32,
    pub group_opponent_destroy_score_weight: f32,
    pub group_my_static_attack_range_power_weight: f32,
}

impl Config {
    pub fn new() -> Self {
        Self {
            segment_size: 5,
            group_distance_to_position_weight: -1.0,
            group_my_attack_range_power_weight: 1.0,
            group_opponent_attack_range_power_weight: -1.0,
            group_my_sight_range_power_weight: 1.0,
            group_opponent_sight_range_power_weight: -1.0,
            group_my_static_destroy_score_weight: 1.0,
            group_my_destroy_score_weight: -1.0,
            group_opponent_destroy_score_weight: 1.0,
            group_my_static_attack_range_power_weight: 0.1,
        }
    }
}
