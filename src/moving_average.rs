use std::collections::VecDeque;
use std::ops::{Add, Sub};

pub struct MovingAverageSpeed<T: Add<Output=T> + Sub<Output=T> + Default + Copy + Into<f64>> {
    max_values: usize,
    max_interval: i32,
    values: VecDeque<(T, i32)>,
    duration: i32,
    distance: T,
}

impl<T: Add<Output=T> + Sub<Output=T> + Default + Copy + Into<f64>> MovingAverageSpeed<T> {
    pub fn new(max_values: usize, max_interval: i32) -> Self {
        assert!(max_values >= 2);
        Self {
            max_values,
            max_interval,
            values: VecDeque::new(),
            duration: 0,
            distance: T::default(),
        }
    }

    pub fn add(&mut self, value: T, current_tick: i32) {
        while self.values.len() >= self.max_values
            || (self.values.len() >= 2 && self.duration >= self.max_interval) {
            if let Some((removed_value, removed_tick)) = self.values.pop_front() {
                if let Some((first_value, first_tick)) = self.values.front() {
                    self.distance = self.distance - (*first_value - removed_value);
                    self.duration -= *first_tick - removed_tick;
                }
            }
        }
        if let Some((last_value, last_tick)) = self.values.back() {
            self.distance = self.distance + (value - *last_value);
            self.duration += current_tick - *last_tick;
        }
        self.values.push_back((value, current_tick));
    }

    pub fn get(&self) -> f32 {
        if self.values.len() < 2 {
            0.0
        } else {
            (self.distance.into() / self.duration as f64) as f32
        }
    }
}
