//! Property tests: `Transform` composition and inverse, and grid snapping.

use proptest::prelude::*;
use protasena_geom::{Orient, Pt, Rect, Transform, Vec2, snap};

fn orient() -> impl Strategy<Value = Orient> {
    proptest::sample::select(Orient::ALL.to_vec())
}

/// Integer-valued coordinates keep every operation exact.
fn coord() -> impl Strategy<Value = f64> {
    (-1_000_000i32..=1_000_000).prop_map(f64::from)
}

fn pt() -> impl Strategy<Value = Pt> {
    (coord(), coord()).prop_map(|(x, y)| Pt::new(x, y))
}

fn transform() -> impl Strategy<Value = Transform> {
    (orient(), coord(), coord()).prop_map(|(o, x, y)| Transform::new(o, Vec2::new(x, y)))
}

proptest! {
    #[test]
    fn inverse_round_trips(t in transform(), p in pt()) {
        prop_assert_eq!(t.inverse().apply_pt(t.apply_pt(p)), p);
        prop_assert_eq!(t.apply_pt(t.inverse().apply_pt(p)), p);
    }

    #[test]
    fn then_matches_sequential_application(a in transform(), b in transform(), p in pt()) {
        prop_assert_eq!(a.then(b).apply_pt(p), b.apply_pt(a.apply_pt(p)));
    }

    #[test]
    fn composition_with_inverse_is_identity(t in transform()) {
        prop_assert_eq!(t.then(t.inverse()), Transform::IDENTITY);
        prop_assert_eq!(t.inverse().then(t), Transform::IDENTITY);
    }

    #[test]
    fn composition_is_associative(a in transform(), b in transform(), c in transform()) {
        prop_assert_eq!(a.then(b).then(c), a.then(b.then(c)));
    }

    #[test]
    fn about_keeps_pivot_fixed(o in orient(), pivot in pt()) {
        prop_assert_eq!(Transform::about(pivot, o).apply_pt(pivot), pivot);
    }

    #[test]
    fn apply_rect_contains_transformed_corners(t in transform(), a in pt(), b in pt()) {
        let r = Rect::new(a, b);
        let out = t.apply_rect(r);
        prop_assert!(out.contains(t.apply_pt(a)));
        prop_assert!(out.contains(t.apply_pt(b)));
        prop_assert_eq!(out.width() * out.height(), r.width() * r.height());
    }

    #[test]
    fn snap_lands_on_grid_and_is_idempotent(x in -1.0e6f64..1.0e6, y in -1.0e6f64..1.0e6, k in 1u32..1000) {
        let grid = f64::from(k);
        let s = snap(Pt::new(x, y), grid);
        prop_assert_eq!(s.x % grid, 0.0);
        prop_assert_eq!(s.y % grid, 0.0);
        prop_assert!((s.x - x).abs() <= grid / 2.0);
        prop_assert!((s.y - y).abs() <= grid / 2.0);
        prop_assert_eq!(snap(s, grid), s);
    }

    #[test]
    fn snap_never_panics(x in any::<f64>(), y in any::<f64>(), grid in any::<f64>()) {
        let _ = snap(Pt::new(x, y), grid);
    }
}
