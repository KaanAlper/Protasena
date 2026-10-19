//! KiCad millimetres to Protasena thou, and the two KiCad y axes.
//!
//! Protasena library geometry is in thou with the y axis pointing **up**
//! ([`protasena_library::units`]). KiCad symbol libraries also have y pointing up, so symbol
//! points keep their sign; KiCad footprints have y pointing **down** (screen convention), so
//! footprint points are mirrored in y. Angles: a KiCad footprint angle turns counter-clockwise
//! as seen on the board, which stays counter-clockwise after the y mirror, so footprint angles
//! keep their value; symbol text angles are written in tenths of a degree.

use protasena_geom::{Pt, Vec2};
use protasena_kicad::common::Pt as KPt;
use protasena_library::units::from_mm;

/// A length in millimetres as thou, rounded to 1/10 000 th like [`from_mm`] but so that the
/// written number is the short decimal (`31.4961`, not `31.496100000000002`), and never `-0.0`.
pub fn len(mm: f64) -> f64 {
    round(from_mm(mm))
}

/// A symbol point (KiCad y up → Protasena y up).
pub fn sym(p: KPt) -> Pt {
    Pt::new(len(p.x), len(p.y))
}

/// A footprint point (KiCad y down → Protasena y up).
pub fn fp(p: KPt) -> Pt {
    Pt::new(len(p.x), len(-p.y))
}

/// A footprint size (no mirror: sizes are magnitudes).
pub fn size(p: KPt) -> Vec2 {
    Vec2::new(len(p.x), len(p.y))
}

/// `p` turned counter-clockwise by `degrees` about `pivot` (y up), rounded like [`len`].
pub fn rotate(p: Pt, pivot: Pt, degrees: f64) -> Pt {
    if degrees == 0.0 {
        return p;
    }
    let (s, c) = degrees.to_radians().sin_cos();
    let (dx, dy) = (p.x - pivot.x, p.y - pivot.y);
    Pt::new(round(pivot.x + dx * c - dy * s), round(pivot.y + dx * s + dy * c))
}

/// A symbol text angle (tenths of a degree) in degrees.
pub fn text_angle(tenths: f64) -> f64 {
    no_negative_zero(tenths / 10.0)
}

/// Rounds to the resolution of [`from_mm`] (1/10 000 thou).
pub fn round(v: f64) -> f64 {
    let r = (v * 1e4).round() / 1e4;
    no_negative_zero(if r.is_finite() { r } else { v })
}

fn no_negative_zero(v: f64) -> f64 {
    if v == 0.0 { 0.0 } else { v }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(x: f64, y: f64) -> KPt {
        KPt { x, y }
    }

    #[test]
    fn symbol_y_points_up_and_stays() {
        assert_eq!(sym(k(1.27, 2.54)), Pt::new(50.0, 100.0));
        assert_eq!(sym(k(0.0, -3.81)), Pt::new(0.0, -150.0));
    }

    #[test]
    fn footprint_y_points_down_and_is_mirrored() {
        assert_eq!(fp(k(1.27, 2.54)), Pt::new(50.0, -100.0));
        assert_eq!(fp(k(0.0, -3.81)), Pt::new(0.0, 150.0));
        let origin = fp(k(0.0, 0.0));
        assert!(origin.y.is_sign_positive(), "no -0.0");
        assert_eq!(len(0.8).to_string(), "31.4961");
    }

    #[test]
    fn rotation_is_counter_clockwise() {
        let p = rotate(Pt::new(100.0, 0.0), Pt::new(0.0, 0.0), 90.0);
        assert_eq!(p, Pt::new(0.0, 100.0));
        assert_eq!(text_angle(900.0), 90.0);
    }
}
