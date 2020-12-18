use crate::my_strategy::{FindPathTarget, Rect, Vec2i};

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

    pub fn distance(&self, position: Vec2i) -> i32 {
        self.center.distance(position)
    }

    pub fn contains(&self, position: Vec2i) -> bool {
        self.distance(position) <= self.radius
    }
}

impl FindPathTarget for Range {
    fn has_reached(&self, position: Vec2i) -> bool {
        self.contains(position)
    }

    fn get_distance(&self, position: Vec2i) -> i32 {
        self.distance(position)
    }
}

#[derive(Default, Clone, Debug, PartialOrd, PartialEq, Eq, Hash)]
pub struct SizedRange {
    position: Vec2i,
    size: i32,
    radius: i32,
}

impl SizedRange {
    pub fn new(position: Vec2i, size: i32, radius: i32) -> Self {
        Self { position, size, radius }
    }

    pub fn position(&self) -> Vec2i {
        self.position
    }

    pub fn size(&self) -> i32 {
        self.size
    }

    pub fn radius(&self) -> i32 {
        self.radius
    }

    pub fn distance(&self, position: Vec2i) -> i32 {
        Rect::new(self.position, self.position + Vec2i::both(self.size))
            .distance_to_position(position)
    }

    pub fn contains(&self, position: Vec2i) -> bool {
        self.distance(position) <= self.radius
    }
}

impl FindPathTarget for SizedRange {
    fn has_reached(&self, position: Vec2i) -> bool {
        self.contains(position)
    }

    fn get_distance(&self, position: Vec2i) -> i32 {
        self.distance(position)
    }
}
