use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    pub fn normalized(self) -> Self {
        let len = self.length();
        if len < 1e-10 {
            Self::ZERO
        } else {
            self / len
        }
    }

    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }

    pub fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }

    pub fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
        }
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self { x: self.x * rhs, y: self.y * rhs }
    }
}

impl Mul<Vec2> for f32 {
    type Output = Vec2;
    fn mul(self, rhs: Vec2) -> Vec2 {
        Vec2 { x: self * rhs.x, y: self * rhs.y }
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self {
        Self { x: self.x / rhs, y: self.y / rhs }
    }
}

impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self {
        Self { x: -self.x, y: -self.y }
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl From<egui::Pos2> for Vec2 {
    fn from(p: egui::Pos2) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl From<Vec2> for egui::Pos2 {
    fn from(v: Vec2) -> Self {
        egui::Pos2::new(v.x, v.y)
    }
}

impl From<egui::Vec2> for Vec2 {
    fn from(v: egui::Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

impl From<Vec2> for egui::Vec2 {
    fn from(v: Vec2) -> Self {
        egui::Vec2::new(v.x, v.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let a = Vec2::new(1.0, 2.0);
        let b = Vec2::new(3.0, 4.0);
        let c = a + b;
        assert_eq!(c, Vec2::new(4.0, 6.0));
    }

    #[test]
    fn test_sub() {
        let a = Vec2::new(5.0, 7.0);
        let b = Vec2::new(2.0, 3.0);
        assert_eq!(a - b, Vec2::new(3.0, 4.0));
    }

    #[test]
    fn test_mul_div() {
        let a = Vec2::new(3.0, 4.0);
        assert_eq!(a * 2.0, Vec2::new(6.0, 8.0));
        assert_eq!(2.0 * a, Vec2::new(6.0, 8.0));
        assert_eq!(a / 2.0, Vec2::new(1.5, 2.0));
    }

    #[test]
    fn test_neg() {
        assert_eq!(-Vec2::new(1.0, -2.0), Vec2::new(-1.0, 2.0));
    }

    #[test]
    fn test_length_and_distance() {
        let a = Vec2::new(3.0, 4.0);
        assert!((a.length() - 5.0).abs() < 1e-6);
        assert!((a.length_sq() - 25.0).abs() < 1e-6);

        let b = Vec2::new(0.0, 0.0);
        assert!((a.distance(b) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalized() {
        let a = Vec2::new(3.0, 4.0);
        let n = a.normalized();
        assert!((n.length() - 1.0).abs() < 1e-6);
        assert!((n.x - 0.6).abs() < 1e-6);
        assert!((n.y - 0.8).abs() < 1e-6);

        assert_eq!(Vec2::ZERO.normalized(), Vec2::ZERO);
    }

    #[test]
    fn test_dot() {
        let a = Vec2::new(1.0, 0.0);
        let b = Vec2::new(0.0, 1.0);
        assert!((a.dot(b)).abs() < 1e-6);
        assert!((a.dot(a) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_lerp() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(10.0, 20.0);
        let mid = a.lerp(b, 0.5);
        assert_eq!(mid, Vec2::new(5.0, 10.0));
        assert_eq!(a.lerp(b, 0.0), a);
        assert_eq!(a.lerp(b, 1.0), b);
    }

    #[test]
    fn test_assign_ops() {
        let mut a = Vec2::new(1.0, 2.0);
        a += Vec2::new(3.0, 4.0);
        assert_eq!(a, Vec2::new(4.0, 6.0));
        a -= Vec2::new(1.0, 1.0);
        assert_eq!(a, Vec2::new(3.0, 5.0));
    }

    #[test]
    fn test_min_max() {
        let a = Vec2::new(1.0, 5.0);
        let b = Vec2::new(3.0, 2.0);
        assert_eq!(a.min(b), Vec2::new(1.0, 2.0));
        assert_eq!(a.max(b), Vec2::new(3.0, 5.0));
    }

    #[test]
    fn test_serde_round_trip() {
        let v = Vec2::new(1.5, -3.7);
        let json = serde_json::to_string(&v).unwrap();
        let v2: Vec2 = serde_json::from_str(&json).unwrap();
        assert_eq!(v, v2);
    }
}
