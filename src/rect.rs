use crate::my_strategy::Vec2i;

#[derive(Default, Clone, Debug, PartialOrd, PartialEq, Eq, Hash)]
pub struct Rect {
    min: Vec2i,
    max: Vec2i,
}

impl Rect {
    pub fn new(min: Vec2i, max: Vec2i) -> Self {
        Rect { min, max }
    }

    pub fn min(&self) -> Vec2i {
        self.min
    }

    pub fn max(&self) -> Vec2i {
        self.max
    }

    pub fn center(&self) -> Vec2i {
        (self.min + self.max) / 2
    }

    pub fn contains(&self, position: Vec2i) -> bool {
        self.min.x() <= position.x() && position.x() < self.max.x() &&
            self.min.y() <= position.y() && position.y() < self.max.y()
    }
}
