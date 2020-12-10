#[cfg(feature = "read_config")]
use serde::Deserialize;

#[cfg(feature = "print_config")]
use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "read_config", derive(Deserialize))]
#[cfg_attr(feature = "print_config", derive(Serialize))]
pub struct Config {
}

impl Config {
    pub fn new() -> Self {
        Self {
        }
    }
}
