use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

use model::{Vec2F32, Vec2I32};

#[derive(Default, Clone, Copy, Debug, PartialOrd)]
pub struct Vec2f {
    x: f32,
    y: f32,
}

impl Vec2f {
    #[inline(always)]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline(always)]
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    #[inline(always)]
    pub const fn i() -> Self {
        Self::only_x(1.0)
    }

    #[inline(always)]
    pub const fn only_x(x: f32) -> Self {
        Self { x, y: 0.0 }
    }

    #[inline(always)]
    pub const fn only_y(y: f32) -> Self {
        Self { x: 0.0, y }
    }

    #[inline(always)]
    pub const fn both(value: f32) -> Self {
        Self { x: value, y: value }
    }

    #[inline(always)]
    pub const fn x(&self) -> f32 {
        self.x
    }

    #[inline(always)]
    pub const fn y(&self) -> f32 {
        self.y
    }

    #[inline(always)]
    pub const fn as_model(&self) -> Vec2F32 {
        Vec2F32 { x: self.x as f32, y: self.y as f32 }
    }

    #[inline(always)]
    pub fn left(&self) -> Self {
        Self { x: -self.x, y: self.y }
    }

    #[inline(always)]
    pub fn abs(&self) -> Self {
        Vec2f::new(self.x.abs(), self.y.abs())
    }

    #[inline(always)]
    pub fn sum(&self) -> f32 {
        self.x + self.y
    }

    #[inline(always)]
    pub fn manhattan_distance(&self, other: Self) -> f32 {
        (other - *self).abs().sum()
    }
}

impl From<Vec2i> for Vec2f {
    fn from(value: Vec2i) -> Self {
        Self::new(value.x() as f32, value.y() as f32)
    }
}

impl From<Vec2F32> for Vec2f {
    fn from(value: Vec2F32) -> Self {
        Self::new(value.x, value.y)
    }
}

impl From<Vec2I32> for Vec2f {
    fn from(value: Vec2I32) -> Self {
        Self::new(value.x as f32, value.y as f32)
    }
}

impl Add for Vec2f {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Vec2f {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<f32> for Vec2f {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Mul for Vec2f {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl Div<f32> for Vec2f {
    type Output = Self;

    #[inline(always)]
    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl Div for Vec2f {
    type Output = Self;

    #[inline(always)]
    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl Neg for Vec2f {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

impl PartialEq for Vec2f {
    #[inline(always)]
    fn eq(&self, rhs: &Self) -> bool {
        (self.x, self.y).eq(&(rhs.x, rhs.y))
    }
}

impl Eq for Vec2f {}

impl AddAssign for Vec2f {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x + other.x,
            y: self.y + other.y,
        };
    }
}

impl SubAssign for Vec2f {
    fn sub_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x - other.x,
            y: self.y - other.y,
        };
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vec2i {
    x: i32,
    y: i32,
}

impl Vec2i {
    #[inline(always)]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    #[inline(always)]
    pub const fn zero() -> Self {
        Self { x: 0, y: 0 }
    }

    #[inline(always)]
    pub const fn x(&self) -> i32 {
        self.x
    }

    #[inline(always)]
    pub const fn y(&self) -> i32 {
        self.y
    }

    #[inline(always)]
    pub const fn only_x(x: i32) -> Self {
        Self { x, y: 0 }
    }

    #[inline(always)]
    pub const fn only_y(y: i32) -> Self {
        Self { x: 0, y }
    }

    #[inline(always)]
    pub const fn with_x(&self, x: i32) -> Self {
        Self::new(x, self.y)
    }

    #[inline(always)]
    pub const fn with_y(&self, y: i32) -> Self {
        Self::new(self.x, y)
    }

    #[inline(always)]
    pub const fn both(value: i32) -> Self {
        Self { x: value, y: value }
    }

    #[inline(always)]
    pub const fn as_model(&self) -> Vec2I32 {
        Vec2I32 { x: self.x, y: self.y }
    }

    #[inline(always)]
    pub fn lowest(&self, other: Self) -> Self {
        Self { x: self.x.min(other.x), y: self.y.min(other.y) }
    }

    #[inline(always)]
    pub fn highest(&self, other: Self) -> Self {
        Self { x: self.x.max(other.x), y: self.y.max(other.y) }
    }

    #[inline(always)]
    pub fn abs(&self) -> Self {
        Self { x: self.x.abs(), y: self.y.abs() }
    }

    #[inline(always)]
    pub fn center(&self) -> Vec2f {
        Vec2f::new(self.x as f32 + 0.5, self.y as f32 + 0.5)
    }

    #[inline(always)]
    pub fn sum(&self) -> i32 {
        self.x + self.y
    }

    #[inline(always)]
    pub fn distance(&self, other: Self) -> i32 {
        (other - *self).abs().sum()
    }
}

impl From<Vec2f> for Vec2i {
    fn from(value: Vec2f) -> Self {
        Self::new(value.x() as i32, value.y() as i32)
    }
}

impl From<Vec2I32> for Vec2i {
    fn from(value: Vec2I32) -> Self {
        Self::new(value.x, value.y)
    }
}

impl Add for Vec2i {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Vec2i::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Vec2i {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Vec2i::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul for Vec2i {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        Vec2i::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl Mul<i32> for Vec2i {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: i32) -> Self::Output {
        Vec2i::new(self.x * rhs, self.y * rhs)
    }
}

impl Div<i32> for Vec2i {
    type Output = Self;

    #[inline(always)]
    fn div(self, rhs: i32) -> Self::Output {
        Vec2i::new(self.x / rhs, self.y / rhs)
    }
}

impl AddAssign for Vec2i {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x + other.x,
            y: self.y + other.y,
        };
    }
}

impl SubAssign for Vec2i {
    fn sub_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x - other.x,
            y: self.y - other.y,
        };
    }
}
