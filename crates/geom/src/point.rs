//! Points ([`Pt`]) and displacement vectors ([`Vec2`]).
//!
//! The two types are kept apart so that the arithmetic says what it means: `Pt - Pt = Vec2`,
//! `Pt + Vec2 = Pt`, and only vectors scale.

use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// A position in world coordinates (y up).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pt {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate (up is positive).
    pub y: f64,
}

/// A displacement between two positions.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    /// Horizontal component.
    pub x: f64,
    /// Vertical component (up is positive).
    pub y: f64,
}

impl Pt {
    /// The origin `(0, 0)`.
    pub const ORIGIN: Pt = Pt { x: 0.0, y: 0.0 };

    /// A point at `(x, y)`.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// The vector from the origin to this point.
    pub const fn to_vec(self) -> Vec2 {
        Vec2 { x: self.x, y: self.y }
    }

    /// `true` when both coordinates are finite (neither NaN nor infinite).
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Euclidean distance to `other`.
    pub fn distance(self, other: Pt) -> f64 {
        (other - self).length()
    }

    /// The point a fraction `t` of the way from `self` to `other` (`t = 0` is `self`, `t = 1`
    /// is `other`; other values extrapolate).
    pub fn lerp(self, other: Pt, t: f64) -> Pt {
        self + (other - self) * t
    }

    /// The point halfway between `self` and `other`.
    pub fn midpoint(self, other: Pt) -> Pt {
        Pt::new((self.x + other.x) / 2.0, (self.y + other.y) / 2.0)
    }
}

impl Vec2 {
    /// The zero vector.
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    /// A vector with components `(x, y)`.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// The point reached by moving from the origin by this vector.
    pub const fn to_pt(self) -> Pt {
        Pt { x: self.x, y: self.y }
    }

    /// `true` when both components are finite (neither NaN nor infinite).
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Dot product.
    pub fn dot(self, other: Vec2) -> f64 {
        self.x * other.x + self.y * other.y
    }

    /// The z component of the 3D cross product: positive when `other` lies counter-clockwise
    /// of `self`, zero when they are parallel.
    pub fn cross(self, other: Vec2) -> f64 {
        self.x * other.y - self.y * other.x
    }

    /// Squared Euclidean length (exact for integer-valued components within 2^26).
    pub fn length_squared(self) -> f64 {
        self.dot(self)
    }

    /// Euclidean length, computed without intermediate overflow.
    pub fn length(self) -> f64 {
        self.x.hypot(self.y)
    }

    /// Taxicab length `|x| + |y|`.
    pub fn manhattan_length(self) -> f64 {
        self.x.abs() + self.y.abs()
    }

    /// This vector rotated 90° counter-clockwise: `(-y, x)`.
    pub fn perp(self) -> Vec2 {
        Vec2::new(-self.y, self.x)
    }
}

impl Add<Vec2> for Pt {
    type Output = Pt;
    fn add(self, v: Vec2) -> Pt {
        Pt::new(self.x + v.x, self.y + v.y)
    }
}

impl AddAssign<Vec2> for Pt {
    fn add_assign(&mut self, v: Vec2) {
        *self = *self + v;
    }
}

impl Sub<Vec2> for Pt {
    type Output = Pt;
    fn sub(self, v: Vec2) -> Pt {
        Pt::new(self.x - v.x, self.y - v.y)
    }
}

impl SubAssign<Vec2> for Pt {
    fn sub_assign(&mut self, v: Vec2) {
        *self = *self - v;
    }
}

impl Sub for Pt {
    type Output = Vec2;
    fn sub(self, other: Pt) -> Vec2 {
        Vec2::new(self.x - other.x, self.y - other.y)
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, other: Vec2) {
        *self = *self + other;
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x - other.x, self.y - other.y)
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, other: Vec2) {
        *self = *self - other;
    }
}

impl Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}

impl Mul<f64> for Vec2 {
    type Output = Vec2;
    fn mul(self, k: f64) -> Vec2 {
        Vec2::new(self.x * k, self.y * k)
    }
}

impl Mul<Vec2> for f64 {
    type Output = Vec2;
    fn mul(self, v: Vec2) -> Vec2 {
        v * self
    }
}

impl Div<f64> for Vec2 {
    type Output = Vec2;
    /// Component-wise division; dividing by zero yields IEEE 754 infinities/NaN, never a panic.
    fn div(self, k: f64) -> Vec2 {
        Vec2::new(self.x / k, self.y / k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_vector_arithmetic() {
        let a = Pt::new(1.0, 2.0);
        let b = Pt::new(4.0, 6.0);
        assert_eq!(b - a, Vec2::new(3.0, 4.0));
        assert_eq!(a + (b - a), b);
        assert_eq!(b - (b - a), a);
        assert_eq!(a.distance(b), 5.0);
        assert_eq!(a.midpoint(b), Pt::new(2.5, 4.0));
        assert_eq!(a.lerp(b, 0.5), Pt::new(2.5, 4.0));
        let mut p = a;
        p += Vec2::new(1.0, 1.0);
        p -= Vec2::new(0.0, 2.0);
        assert_eq!(p, Pt::new(2.0, 1.0));
    }

    #[test]
    fn vector_algebra() {
        let u = Vec2::new(3.0, 4.0);
        let v = Vec2::new(-4.0, 3.0);
        assert_eq!(u.length(), 5.0);
        assert_eq!(u.length_squared(), 25.0);
        assert_eq!(u.manhattan_length(), 7.0);
        assert_eq!(u.dot(v), 0.0);
        assert_eq!(u.cross(v), 25.0);
        assert_eq!(u.perp(), v);
        assert_eq!(u * 2.0, 2.0 * u);
        assert_eq!(u / 2.0, Vec2::new(1.5, 2.0));
        assert_eq!(-u + u, Vec2::ZERO);
        let mut w = u;
        w += v;
        w -= v;
        assert_eq!(w, u);
        assert_eq!(u.to_pt().to_vec(), u);
    }

    #[test]
    fn hostile_values_never_panic() {
        let v = Vec2::new(f64::MAX, f64::MAX);
        assert!(v.length().is_infinite());
        assert!((v / 0.0).x.is_infinite());
        assert!(!Pt::new(f64::NAN, 0.0).is_finite());
        assert!(!Vec2::new(0.0, f64::INFINITY).is_finite());
        assert!(Pt::ORIGIN.is_finite());
    }
}
