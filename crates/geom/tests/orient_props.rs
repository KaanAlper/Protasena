//! Property tests: `Orient` forms the dihedral group of order 8 and acts on vectors as one.

use proptest::prelude::*;
use protasena_geom::{Orient, Pt, Vec2};

fn orient() -> impl Strategy<Value = Orient> {
    proptest::sample::select(Orient::ALL.to_vec())
}

/// Integer-valued coordinates: every orientation maps them exactly (no rounding).
fn coord() -> impl Strategy<Value = f64> {
    (-1_000_000i32..=1_000_000).prop_map(f64::from)
}

fn vec2() -> impl Strategy<Value = Vec2> {
    (coord(), coord()).prop_map(|(x, y)| Vec2::new(x, y))
}

proptest! {
    #[test]
    fn composition_is_associative(a in orient(), b in orient(), c in orient()) {
        prop_assert_eq!(a.then(b).then(c), a.then(b.then(c)));
        prop_assert_eq!((a * b) * c, a * (b * c));
    }

    #[test]
    fn identity_is_neutral(a in orient()) {
        prop_assert_eq!(a.then(Orient::IDENTITY), a);
        prop_assert_eq!(Orient::IDENTITY.then(a), a);
    }

    #[test]
    fn inverse_cancels(a in orient()) {
        prop_assert_eq!(a.then(a.inverse()), Orient::IDENTITY);
        prop_assert_eq!(a.inverse().then(a), Orient::IDENTITY);
        prop_assert_eq!(a.inverse().inverse(), a);
    }

    #[test]
    fn four_rotations_are_identity(a in orient()) {
        prop_assert_eq!(a.rotate_ccw().rotate_ccw().rotate_ccw().rotate_ccw(), a);
        prop_assert_eq!(a.rotate_cw().rotate_cw().rotate_cw().rotate_cw(), a);
        prop_assert_eq!(a.rotate_cw().rotate_ccw(), a);
    }

    #[test]
    fn mirror_twice_is_identity(a in orient()) {
        prop_assert_eq!(a.mirror_x().mirror_x(), a);
        prop_assert_eq!(a.mirror_y().mirror_y(), a);
    }

    #[test]
    fn mirror_x_then_mirror_y_is_half_turn(a in orient()) {
        prop_assert_eq!(a.mirror_x().mirror_y(), a.rotate_ccw().rotate_ccw());
    }

    #[test]
    fn apply_respects_composition(a in orient(), b in orient(), v in vec2()) {
        prop_assert_eq!(a.then(b).apply_vec(v), b.apply_vec(a.apply_vec(v)));
        prop_assert_eq!((a * b).apply_vec(v), a.apply_vec(b.apply_vec(v)));
    }

    #[test]
    fn apply_inverse_round_trips(a in orient(), v in vec2()) {
        prop_assert_eq!(a.inverse().apply_vec(a.apply_vec(v)), v);
        let p = v.to_pt();
        prop_assert_eq!(a.inverse().apply_pt(a.apply_pt(p)), p);
    }

    #[test]
    fn apply_preserves_length(a in orient(), v in vec2()) {
        prop_assert_eq!(a.apply_vec(v).length_squared(), v.length_squared());
    }

    #[test]
    fn mirrored_flag_flips_orientation_sign(a in orient(), u in vec2(), v in vec2()) {
        let before = u.cross(v);
        let after = a.apply_vec(u).cross(a.apply_vec(v));
        if a.is_mirrored() {
            prop_assert_eq!(after, -before);
        } else {
            prop_assert_eq!(after, before);
        }
    }

    #[test]
    fn angle_round_trips(a in orient()) {
        prop_assert_eq!(Orient::from_angle(a.angle_degrees(), a.is_mirrored()), Ok(a));
    }

    #[test]
    fn from_angle_accepts_any_multiple_of_90(k in -1_000_000i32..1_000_000, m in any::<bool>()) {
        let o = Orient::from_angle(k * 90, m);
        prop_assert!(o.is_ok());
        prop_assert_eq!(o.map(|o| o.angle_degrees()), Ok((k * 90).rem_euclid(360)));
    }

    #[test]
    fn from_angle_rejects_other_angles(deg in any::<i32>(), m in any::<bool>()) {
        prop_assume!(deg % 90 != 0);
        prop_assert!(Orient::from_angle(deg, m).is_err());
    }

    #[test]
    fn apply_pt_never_panics(a in orient(), x in any::<f64>(), y in any::<f64>()) {
        let _ = a.apply_pt(Pt::new(x, y));
    }
}
