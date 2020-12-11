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

    pub fn square(&self) -> i32 {
        (self.max - self.min).product()
    }

    pub fn center(&self) -> Vec2i {
        (self.min + self.max) / 2
    }

    pub fn contains(&self, position: Vec2i) -> bool {
        self.min.x() <= position.x() && position.x() < self.max.x() &&
            self.min.y() <= position.y() && position.y() < self.max.y()
    }

    pub fn distance_to_position(&self, position: Vec2i) -> i32 {
        let x = if position.x() < self.min.x() {
            self.min.x() - position.x()
        } else if position.x() >= self.max.x() {
            position.x() + 1 - self.max.x()
        } else {
            0
        };
        let y = if position.y() < self.min.y() {
            self.min.y() - position.y()
        } else if position.y() >= self.max.y() {
            position.y() + 1 - self.max.y()
        } else {
            0
        };
        x + y
    }

    pub fn distance(&self, rect: &Rect) -> i32 {
        let x = if self.min.x() > rect.max.x() {
            self.min.x() - rect.max.x()
        } else if self.max.x() <= rect.min.x() {
            rect.min.x() + 1 - self.max.x()
        } else if rect.min.x() > self.max.x() {
            rect.min.x() - self.max.x()
        } else if rect.max.x() <= self.min.x() {
            self.min.x() + 1 - rect.max.x()
        } else {
            0
        };
        let y = if self.min.y() > rect.max.y() {
            self.min.y() - rect.max.y()
        } else if self.max.y() <= rect.min.y() {
            rect.min.y() + 1 - self.max.y()
        } else if rect.min.y() > self.max.y() {
            rect.min.y() - self.max.y()
        } else if rect.max.y() <= self.min.y() {
            self.min.y() + 1 - rect.max.y()
        } else {
            0
        };
        x + y
    }

    pub fn overlaps(&self, other: &Rect) -> bool {
        self.min.x() < other.max.x()
            && self.max.x() > other.min.x()
            && self.min.y() < other.max.y()
            && self.max.y() > other.min.y()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_to_position_for_1x1_rect() {
        let rect = Rect::new(Vec2i::zero(), Vec2i::both(1));
        assert_eq!(rect.distance_to_position(Vec2i::new(-1, 0)), 1);
        assert_eq!(rect.distance_to_position(Vec2i::new(1, 0)), 1);
        assert_eq!(rect.distance_to_position(Vec2i::new(0, 1)), 1);
        assert_eq!(rect.distance_to_position(Vec2i::new(0, -1)), 1);
        assert_eq!(rect.distance_to_position(Vec2i::new(-1, -1)), 2);
        assert_eq!(rect.distance_to_position(Vec2i::new(-1, 1)), 2);
        assert_eq!(rect.distance_to_position(Vec2i::new(1, -1)), 2);
        assert_eq!(rect.distance_to_position(Vec2i::new(1, 1)), 2);
    }

    #[test]
    fn distance_to_position_for_2x2_rect() {
        let rect = Rect::new(Vec2i::zero(), Vec2i::both(2));
        assert_eq!(rect.distance_to_position(Vec2i::new(-1, 0)), 1);
        assert_eq!(rect.distance_to_position(Vec2i::new(-1, 1)), 1);
        assert_eq!(rect.distance_to_position(Vec2i::new(2, 0)), 1);
        assert_eq!(rect.distance_to_position(Vec2i::new(2, 1)), 1);
        assert_eq!(rect.distance_to_position(Vec2i::new(0, 2)), 1);
        assert_eq!(rect.distance_to_position(Vec2i::new(1, 2)), 1);
        assert_eq!(rect.distance_to_position(Vec2i::new(0, -1)), 1);
        assert_eq!(rect.distance_to_position(Vec2i::new(1, -1)), 1);
        assert_eq!(rect.distance_to_position(Vec2i::new(-1, -1)), 2);
        assert_eq!(rect.distance_to_position(Vec2i::new(-1, 2)), 2);
        assert_eq!(rect.distance_to_position(Vec2i::new(2, -1)), 2);
        assert_eq!(rect.distance_to_position(Vec2i::new(2, 2)), 2);
        assert_eq!(rect.distance_to_position(Vec2i::new(4, 2)), 4);
        assert_eq!(rect.distance_to_position(Vec2i::new(4, 1)), 3);
    }

    #[test]
    fn distance_for_2x2_rects() {
        let rect = Rect::new(Vec2i::zero(), Vec2i::both(2));
        assert_eq!(rect.distance(&Rect::new(Vec2i::new(-2, 0), Vec2i::new(0, 2))), 1);
        assert_eq!(rect.distance(&Rect::new(Vec2i::new(2, 0), Vec2i::new(4, 2))), 1);
        assert_eq!(rect.distance(&Rect::new(Vec2i::new(0, -2), Vec2i::new(2, 0))), 1);
        assert_eq!(rect.distance(&Rect::new(Vec2i::new(0, 2), Vec2i::new(2, 4))), 1);
        assert_eq!(rect.distance(&Rect::new(Vec2i::new(-2, 1), Vec2i::new(0, 3))), 1);
        assert_eq!(rect.distance(&Rect::new(Vec2i::new(2, 1), Vec2i::new(4, 3))), 1);
        assert_eq!(rect.distance(&Rect::new(Vec2i::new(1, -2), Vec2i::new(3, 0))), 1);
        assert_eq!(rect.distance(&Rect::new(Vec2i::new(1, 2), Vec2i::new(3, 4))), 1);
        assert_eq!(rect.distance(&Rect::new(Vec2i::new(2, 2), Vec2i::new(4, 4))), 2);
        assert_eq!(rect.distance(&Rect::new(Vec2i::new(-2, -2), Vec2i::new(0, 0))), 2);
        assert_eq!(rect.distance(&Rect::new(Vec2i::new(2, -2), Vec2i::new(4, 0))), 2);
        assert_eq!(rect.distance(&Rect::new(Vec2i::new(-2, 2), Vec2i::new(0, 4))), 2);
    }

    #[test]
    fn overlaps() {
        let rect = Rect::new(Vec2i::zero(), Vec2i::both(2));
        assert!(rect.overlaps(&rect));
        assert!(!rect.overlaps(&Rect::new(Vec2i::new(-2, -2), Vec2i::new(0, 0))));
        assert!(rect.overlaps(&Rect::new(Vec2i::new(-2, -2), Vec2i::new(1, 1))));
        assert!(!rect.overlaps(&Rect::new(Vec2i::new(2, 2), Vec2i::new(4, 4))));
        assert!(rect.overlaps(&Rect::new(Vec2i::new(1, 1), Vec2i::new(4, 4))));
    }
}
