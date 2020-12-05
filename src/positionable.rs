use crate::my_strategy::Vec2i;

pub trait Positionable {
    fn position(&self) -> Vec2i;

    fn center(&self, size: i32) -> Vec2i {
        self.position() + Vec2i::both(size / 2)
    }
}
