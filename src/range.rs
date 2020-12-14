use crate::my_strategy::Vec2i;

#[derive(Default, Clone, Debug, PartialOrd, PartialEq, Eq, Hash)]
pub struct Range {
    center: Vec2i,
    radius: i32,
}

impl Range {
    pub fn new(center: Vec2i, radius: i32) -> Self {
        Self { center, radius }
    }

    pub fn center(&self) -> Vec2i {
        self.center
    }

    pub fn radius(&self) -> i32 {
        self.radius
    }

    pub fn contains(&self, position: Vec2i) -> bool {
        self.center.distance(position) <= self.radius
    }
}
