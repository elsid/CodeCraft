use std::ops::Mul;

pub trait Square: Mul + Copy {
    fn square(self) -> Self::Output {
        self * self
    }
}

impl Square for f32 {}

pub fn as_score(value: f32) -> i32 {
    (value * 100000.0).round() as i32
}
