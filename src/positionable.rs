use crate::my_strategy::Vec2i;

pub trait Positionable {
    fn position(&self) -> Vec2i;

    fn distance<T: Positionable>(&self, other: &T) -> i32 {
        self.position().distance(other.position())
    }
}
