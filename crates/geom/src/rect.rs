//! Axis-aligned rectangles.

use crate::point::{Pt, Vec2};

/// An axis-aligned rectangle given by its lower-left (`min`) and upper-right (`max`) corners.
///
/// A rectangle is *normalized* when `min.x <= max.x` and `min.y <= max.y`. Every constructor
/// and every method that returns a `Rect` returns a normalized one; the query methods assume a
/// normalized rectangle (call [`Rect::normalized`] on one built from raw fields). Zero width or
/// height is allowed (a point or a line is a valid, degenerate rectangle). Boundaries are
/// inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// Corner with the smallest coordinates.
    pub min: Pt,
    /// Corner with the largest coordinates.
    pub max: Pt,
}

impl Rect {
    /// The rectangle spanned by two opposite corners, in any order.
    ///
    /// Uses IEEE `min`/`max`, so a NaN coordinate is ignored in favour of the other corner's.
    pub fn new(a: Pt, b: Pt) -> Self {
        Self { min: Pt::new(a.x.min(b.x), a.y.min(b.y)), max: Pt::new(a.x.max(b.x), a.y.max(b.y)) }
    }

    /// The degenerate rectangle containing exactly `p`.
    pub fn from_point(p: Pt) -> Self {
        Self { min: p, max: p }
    }

    /// The smallest rectangle containing every finite point of `points`.
    ///
    /// Points with a NaN or infinite coordinate are skipped; `None` when no finite point remains.
    pub fn from_points<I: IntoIterator<Item = Pt>>(points: I) -> Option<Self> {
        points.into_iter().filter(|p| p.is_finite()).fold(None, |acc: Option<Rect>, p| match acc {
            None => Some(Rect::from_point(p)),
            Some(r) => Some(r.include(p)),
        })
    }

    /// The same rectangle with `min` and `max` ordered per axis.
    pub fn normalized(self) -> Self {
        Self::new(self.min, self.max)
    }

    /// Width (`max.x - min.x`).
    pub fn width(self) -> f64 {
        self.max.x - self.min.x
    }

    /// Height (`max.y - min.y`).
    pub fn height(self) -> f64 {
        self.max.y - self.min.y
    }

    /// The vector from `min` to `max`.
    pub fn size(self) -> Vec2 {
        self.max - self.min
    }

    /// The centre point.
    pub fn center(self) -> Pt {
        self.min.midpoint(self.max)
    }

    /// The four corners, counter-clockwise from `min`.
    pub fn corners(self) -> [Pt; 4] {
        [self.min, Pt::new(self.max.x, self.min.y), self.max, Pt::new(self.min.x, self.max.y)]
    }

    /// `true` when `p` lies inside or on the boundary. Always `false` for a NaN coordinate.
    pub fn contains(self, p: Pt) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    /// `true` when `other` lies entirely inside `self` (boundaries may touch).
    pub fn contains_rect(self, other: Rect) -> bool {
        self.contains(other.min) && self.contains(other.max)
    }

    /// `true` when the rectangles share at least one point (touching edges count).
    pub fn intersects(self, other: Rect) -> bool {
        self.min.x <= other.max.x && other.min.x <= self.max.x && self.min.y <= other.max.y && other.min.y <= self.max.y
    }

    /// The overlap of two rectangles, or `None` when they do not share any point.
    pub fn intersection(self, other: Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }
        Some(Rect {
            min: Pt::new(self.min.x.max(other.min.x), self.min.y.max(other.min.y)),
            max: Pt::new(self.max.x.min(other.max.x), self.max.y.min(other.max.y)),
        })
    }

    /// The smallest rectangle containing both.
    pub fn union(self, other: Rect) -> Rect {
        Rect {
            min: Pt::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            max: Pt::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        }
    }

    /// The smallest rectangle containing `self` and `p`.
    pub fn include(self, p: Pt) -> Rect {
        self.union(Rect::from_point(p))
    }

    /// Grows every side outward by `d` (shrinks for negative `d`).
    ///
    /// Shrinking past zero size collapses that axis to the centre line instead of inverting the
    /// rectangle. A non-finite `d` returns the rectangle unchanged.
    pub fn inflate(self, d: f64) -> Rect {
        if !d.is_finite() {
            return self;
        }
        let c = self.center();
        let (x0, x1) = inflate_axis(self.min.x, self.max.x, c.x, d);
        let (y0, y1) = inflate_axis(self.min.y, self.max.y, c.y, d);
        Rect { min: Pt::new(x0, y0), max: Pt::new(x1, y1) }
    }

    /// The rectangle moved by `v`.
    pub fn translate(self, v: Vec2) -> Rect {
        Rect { min: self.min + v, max: self.max + v }
    }
}

fn inflate_axis(lo: f64, hi: f64, center: f64, d: f64) -> (f64, f64) {
    let (lo, hi) = (lo - d, hi + d);
    if lo > hi { (center, center) } else { (lo, hi) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
        Rect::new(Pt::new(x0, y0), Pt::new(x1, y1))
    }

    #[test]
    fn new_normalizes_corners() {
        let a = r(4.0, 1.0, 0.0, 3.0);
        assert_eq!(a, Rect { min: Pt::new(0.0, 1.0), max: Pt::new(4.0, 3.0) });
        let raw = Rect { min: Pt::new(4.0, 3.0), max: Pt::new(0.0, 1.0) };
        assert_eq!(raw.normalized(), a);
        assert_eq!((a.width(), a.height()), (4.0, 2.0));
        assert_eq!(a.size(), Vec2::new(4.0, 2.0));
        assert_eq!(a.center(), Pt::new(2.0, 2.0));
        assert_eq!(a.corners(), [Pt::new(0.0, 1.0), Pt::new(4.0, 1.0), Pt::new(4.0, 3.0), Pt::new(0.0, 3.0)]);
    }

    #[test]
    fn contains_is_inclusive_and_rejects_nan() {
        let a = r(0.0, 0.0, 2.0, 2.0);
        assert!(a.contains(Pt::new(0.0, 2.0)));
        assert!(a.contains(Pt::new(1.0, 1.0)));
        assert!(!a.contains(Pt::new(2.1, 1.0)));
        assert!(!a.contains(Pt::new(f64::NAN, 1.0)));
        assert!(a.contains_rect(r(0.0, 0.0, 1.0, 1.0)));
        assert!(!a.contains_rect(r(1.0, 1.0, 3.0, 1.0)));
    }

    #[test]
    fn union_intersection_and_include() {
        let a = r(0.0, 0.0, 2.0, 2.0);
        let b = r(1.0, -1.0, 3.0, 1.0);
        assert_eq!(a.union(b), r(0.0, -1.0, 3.0, 2.0));
        assert_eq!(a.intersection(b), Some(r(1.0, 0.0, 2.0, 1.0)));
        assert_eq!(a.intersection(r(2.0, 2.0, 5.0, 5.0)), Some(Rect::from_point(Pt::new(2.0, 2.0))));
        assert_eq!(a.intersection(r(3.0, 3.0, 5.0, 5.0)), None);
        assert!(!a.intersects(r(2.5, 0.0, 3.0, 1.0)));
        assert_eq!(a.include(Pt::new(-1.0, 5.0)), r(-1.0, 0.0, 2.0, 5.0));
        assert_eq!(a.translate(Vec2::new(1.0, 1.0)), r(1.0, 1.0, 3.0, 3.0));
    }

    #[test]
    fn from_points_skips_non_finite() {
        let pts = [Pt::new(1.0, 5.0), Pt::new(f64::NAN, 0.0), Pt::new(-2.0, 3.0), Pt::new(f64::INFINITY, 9.0)];
        assert_eq!(Rect::from_points(pts), Some(r(-2.0, 3.0, 1.0, 5.0)));
        assert_eq!(Rect::from_points([Pt::new(f64::NAN, f64::NAN)]), None);
        assert_eq!(Rect::from_points(std::iter::empty()), None);
    }

    #[test]
    fn inflate_grows_shrinks_and_collapses() {
        let a = r(0.0, 0.0, 4.0, 2.0);
        assert_eq!(a.inflate(1.0), r(-1.0, -1.0, 5.0, 3.0));
        assert_eq!(a.inflate(-0.5), r(0.5, 0.5, 3.5, 1.5));
        assert_eq!(a.inflate(-1.5), r(1.5, 1.0, 2.5, 1.0));
        assert_eq!(a.inflate(-10.0), Rect::from_point(Pt::new(2.0, 1.0)));
        assert_eq!(a.inflate(f64::NAN), a);
        assert_eq!(a.inflate(f64::INFINITY), a);
    }
}
