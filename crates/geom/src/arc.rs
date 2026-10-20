//! Circular arcs through three points.

use std::f64::consts::{FRAC_PI_2, TAU};

use crate::point::{Pt, Vec2};
use crate::rect::Rect;

/// A circular arc as centre, radius, start angle and sweep (radians).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcGeom {
    /// Centre.
    pub center: Pt,
    /// Radius.
    pub radius: f64,
    /// Start angle, counter-clockwise from +x.
    pub start: f64,
    /// Sweep, positive counter-clockwise.
    pub sweep: f64,
}

/// The arc from `start` through `mid` to `end`; `None` when the points are (nearly) collinear
/// or not finite.
pub fn arc_through(start: Pt, mid: Pt, end: Pt) -> Option<ArcGeom> {
    let (ax, ay, bx, by, cx, cy) = (start.x, start.y, mid.x, mid.y, end.x, end.y);
    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    let scale = (start.distance(mid) + mid.distance(end)).max(1e-9);
    if !d.is_finite() || d.abs() < 1e-9 * scale * scale {
        return None;
    }
    let (a2, b2, c2) = (ax * ax + ay * ay, bx * bx + by * by, cx * cx + cy * cy);
    let center = Pt::new((a2 * (by - cy) + b2 * (cy - ay) + c2 * (ay - by)) / d, (a2 * (cx - bx) + b2 * (ax - cx) + c2 * (bx - ax)) / d);
    let radius = center.distance(start);
    let angle = |p: Pt| (p.y - center.y).atan2(p.x - center.x);
    let (s, m, e) = (angle(start), angle(mid), angle(end));
    let ccw = |from: f64, to: f64| (to - from).rem_euclid(TAU);
    let sweep = if ccw(s, m) <= ccw(s, e) { ccw(s, e) } else { ccw(s, e) - TAU };
    let arc = ArcGeom { center, radius, start: s, sweep };
    (center.is_finite() && radius.is_finite()).then_some(arc)
}

impl ArcGeom {
    /// The point at `angle` (radians, counter-clockwise from +x) on the arc's circle.
    pub fn point_at(&self, angle: f64) -> Pt {
        self.center + Vec2::new(self.radius * angle.cos(), self.radius * angle.sin())
    }

    /// The smallest box holding the arc: its ends and the circle's extreme points it passes.
    pub fn bounds(&self) -> Rect {
        let (lo, hi) = if self.sweep >= 0.0 { (self.start, self.start + self.sweep) } else { (self.start + self.sweep, self.start) };
        let first = (lo / FRAC_PI_2).ceil();
        // A sweep of at most a full turn passes at most five quarter-turn angles.
        let quarters = (0..5).map(|i| (first + f64::from(i)) * FRAC_PI_2).take_while(|&a| a <= hi);
        quarters.chain([lo, hi]).map(|a| Rect::from_point(self.point_at(a))).fold(Rect::from_point(self.point_at(lo)), Rect::union)
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn arc_through_three_points_keeps_its_direction() {
        let ccw = arc_through(Pt::new(100.0, 0.0), Pt::new(0.0, 100.0), Pt::new(-100.0, 0.0)).unwrap();
        assert!(close(ccw.radius, 100.0) && close(ccw.center.x, 0.0) && close(ccw.center.y, 0.0));
        assert!(close(ccw.sweep, PI), "{ccw:?}");
        let cw = arc_through(Pt::new(100.0, 0.0), Pt::new(0.0, -100.0), Pt::new(-100.0, 0.0)).unwrap();
        assert!(close(cw.sweep, -PI), "{cw:?}");
        assert!(arc_through(Pt::new(0.0, 0.0), Pt::new(1.0, 1.0), Pt::new(2.0, 2.0)).is_none());
        assert!(arc_through(Pt::new(f64::NAN, 0.0), Pt::new(1.0, 1.0), Pt::new(2.0, 0.0)).is_none());
    }

    #[test]
    fn bounds_cover_the_bulge() {
        let up = arc_through(Pt::new(-100.0, 0.0), Pt::new(0.0, 100.0), Pt::new(100.0, 0.0)).unwrap().bounds();
        assert!(close(up.max.y, 100.0) && close(up.min.y, 0.0) && close(up.min.x, -100.0), "{up:?}");
        let down = arc_through(Pt::new(-100.0, 0.0), Pt::new(0.0, -100.0), Pt::new(100.0, 0.0)).unwrap().bounds();
        assert!(close(down.min.y, -100.0) && close(down.max.y, 0.0), "{down:?}");
        let small = arc_through(Pt::new(100.0, 0.0), Pt::new(70.710678, 70.710678), Pt::new(0.0, 100.0)).unwrap().bounds();
        assert!(close(small.min.x, 0.0) && close(small.max.x, 100.0) && close(small.max.y, 100.0), "{small:?}");
    }
}
