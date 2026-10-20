//! Line segments: distance to a point and segment–segment intersection.

use crate::ortho;
use crate::point::{Pt, Vec2};
use crate::rect::Rect;

/// Relative tolerance of the intersection tests (scaled by the segment lengths involved).
const REL_EPS: f64 = 1e-12;

/// The closed line segment from `a` to `b`. `a == b` is a degenerate (point) segment.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Segment {
    /// Start point.
    pub a: Pt,
    /// End point.
    pub b: Pt,
}

/// How two segments meet; see [`Segment::intersect`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Intersection {
    /// No common point.
    Disjoint,
    /// Exactly one common point (crossing, touching, or collinear segments meeting end to end).
    Point(Pt),
    /// Collinear segments sharing a stretch of positive length; the overlap runs in the
    /// direction of the first segment (the receiver of [`Segment::intersect`]).
    Overlap(Segment),
}

impl Segment {
    /// The segment from `a` to `b`.
    pub const fn new(a: Pt, b: Pt) -> Self {
        Self { a, b }
    }

    /// The vector from `a` to `b`.
    pub fn direction(self) -> Vec2 {
        self.b - self.a
    }

    /// Euclidean length.
    pub fn length(self) -> f64 {
        self.direction().length()
    }

    /// `true` when both ends are the same point.
    pub fn is_degenerate(self) -> bool {
        self.a == self.b
    }

    /// The same segment from `b` to `a`.
    pub fn reversed(self) -> Segment {
        Segment::new(self.b, self.a)
    }

    /// The smallest rectangle containing the segment.
    pub fn bounds(self) -> Rect {
        Rect::new(self.a, self.b)
    }

    /// `true` when both ends have the same y, exactly (see [`ortho::is_horizontal`]).
    pub fn is_horizontal(self) -> bool {
        ortho::is_horizontal(self.a, self.b)
    }

    /// `true` when both ends have the same x, exactly (see [`ortho::is_vertical`]).
    pub fn is_vertical(self) -> bool {
        ortho::is_vertical(self.a, self.b)
    }

    /// `true` when the segment is horizontal or vertical.
    pub fn is_orthogonal(self) -> bool {
        self.is_horizontal() || self.is_vertical()
    }

    /// The point of the segment nearest to `p`. NaN in, NaN out; never panics.
    pub fn closest_point(self, p: Pt) -> Pt {
        let r = self.direction();
        let len2 = r.length_squared();
        if len2 == 0.0 {
            return self.a;
        }
        let t = (p - self.a).dot(r) / len2;
        if t <= 0.0 {
            self.a
        } else if t >= 1.0 {
            self.b
        } else if t.is_nan() {
            // `p` or the segment is not finite, or `len2` overflowed: fall back to the endpoints.
            if p.distance(self.a) <= p.distance(self.b) { self.a } else { self.b }
        } else {
            self.a + r * t
        }
    }

    /// Euclidean distance from `p` to the nearest point of the segment (NaN for NaN input).
    pub fn distance_to_pt(self, p: Pt) -> f64 {
        p.distance(self.closest_point(p))
    }

    /// How `self` and `other` meet.
    ///
    /// Tests use a relative tolerance of 1e-12 of the segment lengths, so a point computed in
    /// floating point still counts as touching. When the meeting point is an endpoint of either
    /// segment, that endpoint is returned exactly (important for grid-snapped wires). Any NaN or
    /// infinite coordinate gives [`Intersection::Disjoint`].
    pub fn intersect(self, other: Segment) -> Intersection {
        if ![self.a, self.b, other.a, other.b].iter().all(|p| p.is_finite()) {
            return Intersection::Disjoint;
        }
        match (self.is_degenerate(), other.is_degenerate()) {
            (true, true) if self.a == other.a => return Intersection::Point(self.a),
            (true, true) => return Intersection::Disjoint,
            (true, false) => return other.point_on(self.a),
            (false, true) => return self.point_on(other.a),
            (false, false) => {}
        }
        let (r, s, qp) = (self.direction(), other.direction(), other.a - self.a);
        let denom = r.cross(s);
        if denom.abs() <= REL_EPS * r.length() * s.length() {
            if qp.cross(r).abs() > REL_EPS * r.length() * qp.length() {
                return Intersection::Disjoint; // parallel, apart
            }
            return self.collinear_overlap(other);
        }
        let t = qp.cross(s) / denom;
        let u = qp.cross(r) / denom;
        let inside = |v: f64| (-REL_EPS..=1.0 + REL_EPS).contains(&v);
        if !inside(t) || !inside(u) {
            return Intersection::Disjoint;
        }
        Intersection::Point(exact_end(other, u).or_else(|| exact_end(self, t)).unwrap_or_else(|| self.a + r * t))
    }

    /// `Point(p)` when `p` lies on the (non-degenerate) segment, else `Disjoint`.
    fn point_on(self, p: Pt) -> Intersection {
        if self.distance_to_pt(p) <= REL_EPS * self.length() { Intersection::Point(p) } else { Intersection::Disjoint }
    }

    /// Overlap of two collinear, non-degenerate segments.
    fn collinear_overlap(self, other: Segment) -> Intersection {
        let r = self.direction();
        let len2 = r.length_squared();
        let t0 = (other.a - self.a).dot(r) / len2;
        let t1 = (other.b - self.a).dot(r) / len2;
        let (o_lo, o_hi) = if t0 <= t1 { ((t0, other.a), (t1, other.b)) } else { ((t1, other.b), (t0, other.a)) };
        // Each end of the overlap is an endpoint of one of the segments: pick it exactly.
        let lo = if o_lo.0 > 0.0 { o_lo } else { (0.0, self.a) };
        let hi = if o_hi.0 < 1.0 { o_hi } else { (1.0, self.b) };
        if lo.0 > hi.0 + REL_EPS {
            Intersection::Disjoint
        } else if hi.0 - lo.0 <= REL_EPS {
            Intersection::Point(lo.1)
        } else {
            Intersection::Overlap(Segment::new(lo.1, hi.1))
        }
    }
}

/// The endpoint of `seg` at parameter `t` when `t` is (within tolerance) 0 or 1.
fn exact_end(seg: Segment, t: f64) -> Option<Pt> {
    if t.abs() <= REL_EPS {
        Some(seg.a)
    } else if (t - 1.0).abs() <= REL_EPS {
        Some(seg.b)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(x0: f64, y0: f64, x1: f64, y1: f64) -> Segment {
        Segment::new(Pt::new(x0, y0), Pt::new(x1, y1))
    }

    fn pt(x: f64, y: f64) -> Intersection {
        Intersection::Point(Pt::new(x, y))
    }

    #[test]
    fn distance_to_interior_and_ends() {
        let s = seg(0.0, 0.0, 10.0, 0.0);
        assert_eq!(s.distance_to_pt(Pt::new(5.0, 3.0)), 3.0);
        assert_eq!(s.distance_to_pt(Pt::new(-3.0, 4.0)), 5.0);
        assert_eq!(s.distance_to_pt(Pt::new(13.0, -4.0)), 5.0);
        assert_eq!(s.closest_point(Pt::new(7.0, 9.0)), Pt::new(7.0, 0.0));
        assert_eq!(seg(1.0, 1.0, 1.0, 1.0).distance_to_pt(Pt::new(4.0, 5.0)), 5.0);
    }

    #[test]
    fn hostile_distance_never_panics() {
        assert!(seg(0.0, 0.0, 1.0, 0.0).distance_to_pt(Pt::new(f64::NAN, 0.0)).is_nan());
        let huge = seg(-f64::MAX, 0.0, f64::MAX, 0.0);
        let _ = huge.closest_point(Pt::new(1.0, 1.0));
        let _ = seg(f64::NAN, 0.0, 1.0, f64::INFINITY).distance_to_pt(Pt::ORIGIN);
    }

    #[test]
    fn crossing_touching_and_corner() {
        assert_eq!(seg(0.0, 0.0, 10.0, 10.0).intersect(seg(0.0, 10.0, 10.0, 0.0)), pt(5.0, 5.0));
        // T junction: exact endpoint of the second segment.
        assert_eq!(seg(0.0, 0.0, 10.0, 0.0).intersect(seg(5.0, 0.0, 5.0, 5.0)), pt(5.0, 0.0));
        // L corner.
        assert_eq!(seg(0.0, 0.0, 10.0, 0.0).intersect(seg(10.0, 0.0, 10.0, 5.0)), pt(10.0, 0.0));
        // Endpoints given by a non-representable decimal still come back exactly.
        let a = seg(0.1, 0.3, 0.7, 0.3);
        assert_eq!(a.intersect(seg(0.7, 0.3, 0.7, 0.9)), pt(0.7, 0.3));
        assert_eq!(seg(0.0, 0.0, 10.0, 0.0).intersect(seg(5.0, 1.0, 5.0, 5.0)), Intersection::Disjoint);
    }

    #[test]
    fn parallel_and_collinear() {
        let a = seg(0.0, 0.0, 10.0, 0.0);
        assert_eq!(a.intersect(seg(0.0, 1.0, 10.0, 1.0)), Intersection::Disjoint);
        assert_eq!(a.intersect(seg(12.0, 0.0, 20.0, 0.0)), Intersection::Disjoint);
        assert_eq!(a.intersect(seg(10.0, 0.0, 20.0, 0.0)), pt(10.0, 0.0));
        assert_eq!(a.intersect(seg(15.0, 0.0, 5.0, 0.0)), Intersection::Overlap(seg(5.0, 0.0, 10.0, 0.0)));
        assert_eq!(a.intersect(seg(2.0, 0.0, 4.0, 0.0)), Intersection::Overlap(seg(2.0, 0.0, 4.0, 0.0)));
        assert_eq!(a.intersect(seg(-5.0, 0.0, 15.0, 0.0)), Intersection::Overlap(a));
        assert_eq!(a.reversed().intersect(seg(-5.0, 0.0, 15.0, 0.0)), Intersection::Overlap(a.reversed()));
    }

    #[test]
    fn degenerate_segments() {
        let a = seg(0.0, 0.0, 10.0, 0.0);
        assert_eq!(a.intersect(seg(3.0, 0.0, 3.0, 0.0)), pt(3.0, 0.0));
        assert_eq!(seg(3.0, 0.0, 3.0, 0.0).intersect(a), pt(3.0, 0.0));
        assert_eq!(a.intersect(seg(3.0, 1.0, 3.0, 1.0)), Intersection::Disjoint);
        assert_eq!(seg(1.0, 1.0, 1.0, 1.0).intersect(seg(1.0, 1.0, 1.0, 1.0)), pt(1.0, 1.0));
        assert_eq!(seg(1.0, 1.0, 1.0, 1.0).intersect(seg(1.0, 2.0, 1.0, 2.0)), Intersection::Disjoint);
    }

    #[test]
    fn hostile_intersection_is_disjoint() {
        let a = seg(0.0, 0.0, 10.0, 0.0);
        assert_eq!(a.intersect(seg(f64::NAN, 0.0, 5.0, 5.0)), Intersection::Disjoint);
        assert_eq!(a.intersect(seg(0.0, f64::INFINITY, 5.0, -5.0)), Intersection::Disjoint);
        let _ = seg(-f64::MAX, -f64::MAX, f64::MAX, f64::MAX).intersect(seg(-f64::MAX, f64::MAX, f64::MAX, -f64::MAX));
    }

    #[test]
    fn orthogonal_queries() {
        assert!(seg(0.0, 2.0, 5.0, 2.0).is_horizontal());
        assert!(seg(1.0, 0.0, 1.0, 5.0).is_vertical());
        assert!(!seg(0.0, 0.0, 1.0, 1.0).is_orthogonal());
        assert!(seg(1.0, 1.0, 1.0, 1.0).is_degenerate());
        assert_eq!(seg(4.0, 0.0, 0.0, 3.0).bounds(), Rect::new(Pt::new(0.0, 0.0), Pt::new(4.0, 3.0)));
        assert_eq!(seg(4.0, 0.0, 0.0, 3.0).length(), 5.0);
    }
}
