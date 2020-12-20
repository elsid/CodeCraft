use crate::my_strategy::{Rect, Vec2f, Vec2i};

pub trait Positionable {
    fn position(&self) -> Vec2i;

    fn center(&self, size: i32) -> Vec2i {
        self.position() + Vec2i::both(size / 2)
    }

    fn center_f(&self, size: i32) -> Vec2f {
        Vec2f::from(self.position()) + Vec2f::both(size as f32 / 2.0)
    }

    fn bounds(&self, size: i32) -> Rect {
        let position = self.position();
        Rect::new(position, position + Vec2i::both(size))
    }
}
