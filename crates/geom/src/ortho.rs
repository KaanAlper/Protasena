//! Orthogonal helpers for wire routing.
//!
//! The comparisons are exact: coordinates are expected to come from grid snapping, which yields
//! bit-identical values for the same grid position.

use crate::point::Pt;

/// Taxicab distance `|Δx| + |Δy|`: the length of any monotone orthogonal path from `a` to `b`.
pub fn manhattan(a: Pt, b: Pt) -> f64 {
    (b - a).manhattan_length()
}

/// `true` when `a` and `b` have the same y (a zero-length pair counts as horizontal).
pub fn is_horizontal(a: Pt, b: Pt) -> bool {
    a.y == b.y
}

/// `true` when `a` and `b` have the same x (a zero-length pair counts as vertical).
pub fn is_vertical(a: Pt, b: Pt) -> bool {
    a.x == b.x
}

/// The corner of the one-bend orthogonal path from `from` to `to`: `(to.x, from.y)` when the
/// path runs horizontally first, `(from.x, to.y)` when it runs vertically first. When the two
/// points are already aligned the corner coincides with one of them.
pub fn elbow(from: Pt, to: Pt, horizontal_first: bool) -> Pt {
    if horizontal_first { Pt::new(to.x, from.y) } else { Pt::new(from.x, to.y) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manhattan_sums_axis_distances() {
        assert_eq!(manhattan(Pt::new(1.0, 1.0), Pt::new(-2.0, 5.0)), 7.0);
        assert_eq!(manhattan(Pt::ORIGIN, Pt::ORIGIN), 0.0);
        assert!(manhattan(Pt::ORIGIN, Pt::new(f64::NAN, 0.0)).is_nan());
    }

    #[test]
    fn alignment_is_exact() {
        let a = Pt::new(0.0, 5.0);
        assert!(is_horizontal(a, Pt::new(10.0, 5.0)));
        assert!(!is_horizontal(a, Pt::new(10.0, 5.000_001)));
        assert!(is_vertical(a, Pt::new(0.0, -3.0)));
        assert!(is_horizontal(a, a) && is_vertical(a, a));
        assert!(!is_horizontal(Pt::new(0.0, f64::NAN), Pt::new(1.0, f64::NAN)));
    }

    #[test]
    fn elbow_picks_the_corner() {
        let (a, b) = (Pt::new(0.0, 0.0), Pt::new(4.0, 3.0));
        let h = elbow(a, b, true);
        let v = elbow(a, b, false);
        assert_eq!(h, Pt::new(4.0, 0.0));
        assert_eq!(v, Pt::new(0.0, 3.0));
        assert!(is_horizontal(a, h) && is_vertical(h, b));
        assert!(is_vertical(a, v) && is_horizontal(v, b));
        assert_eq!(manhattan(a, h) + manhattan(h, b), manhattan(a, b));
    }
}
