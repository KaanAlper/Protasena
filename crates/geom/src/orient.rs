//! Orientations: rotations in 90° steps with an optional mirror (the dihedral group of order 8).

use std::ops::Mul;

use crate::error::GeomError;
use crate::point::{Pt, Vec2};

/// One of the eight orientations a schematic object can take.
///
/// An `Orient` is "mirror first, then rotate": the optional mirror negates x (a left–right
/// flip across the vertical axis), then the result is rotated counter-clockwise by
/// [`quarter_turns`](Orient::quarter_turns) × 90° about the origin (y up). Every orientation is
/// exactly one such pair, so `(angle_degrees, is_mirrored)` is a lossless description.
///
/// Applying an orientation to integer-valued coordinates is exact (it only swaps and negates).
///
/// Composition: `a.then(b)` applies `a` first and `b` second; `a * b` is the matrix-style
/// product that applies `b` first, so `a.then(b) == b * a`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Orient {
    /// 0..=3, counter-clockwise quarter turns applied after the mirror.
    quarter_turns: u8,
    /// Mirror (negate x) before rotating.
    mirrored: bool,
}

impl Orient {
    /// No rotation, no mirror.
    pub const IDENTITY: Orient = Orient { quarter_turns: 0, mirrored: false };
    /// Mirror across the vertical axis: `(x, y) → (-x, y)`.
    pub const MIRROR_X: Orient = Orient { quarter_turns: 0, mirrored: true };
    /// Mirror across the horizontal axis: `(x, y) → (x, -y)` (equal to `MIRROR_X` then 180°).
    pub const MIRROR_Y: Orient = Orient { quarter_turns: 2, mirrored: true };
    /// All eight orientations: the four rotations (0°, 90°, 180°, 270°), then the same four
    /// rotations applied after `MIRROR_X`.
    pub const ALL: [Orient; 8] = [
        Orient { quarter_turns: 0, mirrored: false },
        Orient { quarter_turns: 1, mirrored: false },
        Orient { quarter_turns: 2, mirrored: false },
        Orient { quarter_turns: 3, mirrored: false },
        Orient { quarter_turns: 0, mirrored: true },
        Orient { quarter_turns: 1, mirrored: true },
        Orient { quarter_turns: 2, mirrored: true },
        Orient { quarter_turns: 3, mirrored: true },
    ];

    /// The pure rotation by `quarter_turns` × 90° counter-clockwise (negative turns clockwise;
    /// any `i32` is accepted and reduced modulo 4).
    pub fn rotation(quarter_turns: i32) -> Orient {
        Orient { quarter_turns: quarter(quarter_turns), mirrored: false }
    }

    /// The orientation with a counter-clockwise `angle_degrees` applied after an optional
    /// `MIRROR_X`.
    ///
    /// Any multiple of 90 is accepted (negative angles turn clockwise; `450` equals `90`).
    ///
    /// # Errors
    /// [`GeomError::AngleNotQuarterTurn`] when `angle_degrees` is not a multiple of 90.
    pub fn from_angle(angle_degrees: i32, mirrored: bool) -> Result<Orient, GeomError> {
        if angle_degrees % 90 != 0 {
            return Err(GeomError::AngleNotQuarterTurn(angle_degrees));
        }
        Ok(Orient { quarter_turns: quarter(angle_degrees / 90), mirrored })
    }

    /// The rotation part in degrees counter-clockwise: 0, 90, 180 or 270.
    pub fn angle_degrees(self) -> i32 {
        i32::from(self.quarter_turns) * 90
    }

    /// The rotation part in counter-clockwise quarter turns: 0..=3.
    pub fn quarter_turns(self) -> u8 {
        self.quarter_turns
    }

    /// `true` when the orientation includes a mirror (it reverses handedness).
    pub fn is_mirrored(self) -> bool {
        self.mirrored
    }

    /// The orientation that applies `self` first and `next` second.
    pub fn then(self, next: Orient) -> Orient {
        // R^a M^m ∘ R^b M^n = R^(a ± b) M^(m xor n), since M R^b = R^-b M.
        let b = i32::from(self.quarter_turns);
        let turned = if next.mirrored { -b } else { b };
        Orient { quarter_turns: quarter(i32::from(next.quarter_turns) + turned), mirrored: next.mirrored != self.mirrored }
    }

    /// The orientation that undoes `self`: `self.then(self.inverse()) == Orient::IDENTITY`.
    pub fn inverse(self) -> Orient {
        if self.mirrored {
            // Every mirrored orientation is a reflection, hence its own inverse.
            self
        } else {
            Orient::rotation(-i32::from(self.quarter_turns))
        }
    }

    /// `self` followed by a 90° counter-clockwise rotation.
    pub fn rotate_ccw(self) -> Orient {
        self.then(Orient::rotation(1))
    }

    /// `self` followed by a 90° clockwise rotation.
    pub fn rotate_cw(self) -> Orient {
        self.then(Orient::rotation(-1))
    }

    /// `self` followed by [`Orient::MIRROR_X`] (negate x).
    pub fn mirror_x(self) -> Orient {
        self.then(Orient::MIRROR_X)
    }

    /// `self` followed by [`Orient::MIRROR_Y`] (negate y).
    pub fn mirror_y(self) -> Orient {
        self.then(Orient::MIRROR_Y)
    }

    /// Applies the orientation to a vector.
    pub fn apply_vec(self, v: Vec2) -> Vec2 {
        let x = if self.mirrored { -v.x } else { v.x };
        match self.quarter_turns {
            1 => Vec2::new(-v.y, x),
            2 => Vec2::new(-x, -v.y),
            3 => Vec2::new(v.y, -x),
            _ => Vec2::new(x, v.y),
        }
    }

    /// Applies the orientation to a point, about the origin.
    pub fn apply_pt(self, p: Pt) -> Pt {
        self.apply_vec(p.to_vec()).to_pt()
    }

    /// The 2×2 matrix `[[a, b], [c, d]]` with `apply_vec(v) == (a·x + b·y, c·x + d·y)`; every
    /// entry is −1, 0 or 1.
    pub fn matrix(self) -> [[f64; 2]; 2] {
        let ex = self.apply_vec(Vec2::new(1.0, 0.0));
        let ey = self.apply_vec(Vec2::new(0.0, 1.0));
        [[ex.x, ey.x], [ex.y, ey.y]]
    }
}

impl Mul for Orient {
    type Output = Orient;
    /// Matrix-style product: `(a * b).apply_vec(v) == a.apply_vec(b.apply_vec(v))`.
    fn mul(self, rhs: Orient) -> Orient {
        rhs.then(self)
    }
}

/// Reduces any number of quarter turns to 0..=3.
fn quarter(turns: i32) -> u8 {
    match turns.rem_euclid(4) {
        1 => 1,
        2 => 2,
        3 => 3,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: Vec2 = Vec2::new(2.0, 1.0);

    #[test]
    fn rotations_turn_counter_clockwise() {
        assert_eq!(Orient::rotation(1).apply_vec(P), Vec2::new(-1.0, 2.0));
        assert_eq!(Orient::rotation(2).apply_vec(P), Vec2::new(-2.0, -1.0));
        assert_eq!(Orient::rotation(3).apply_vec(P), Vec2::new(1.0, -2.0));
        assert_eq!(Orient::rotation(-1), Orient::rotation(3));
        assert_eq!(Orient::rotation(i32::MIN), Orient::IDENTITY);
        assert_eq!(Orient::IDENTITY.rotate_cw().apply_vec(P), Vec2::new(1.0, -2.0));
        assert_eq!(Orient::IDENTITY.rotate_ccw(), Orient::rotation(1));
    }

    #[test]
    fn mirrors_negate_one_axis() {
        assert_eq!(Orient::MIRROR_X.apply_vec(P), Vec2::new(-2.0, 1.0));
        assert_eq!(Orient::MIRROR_Y.apply_vec(P), Vec2::new(2.0, -1.0));
        assert_eq!(Orient::IDENTITY.mirror_x(), Orient::MIRROR_X);
        assert_eq!(Orient::IDENTITY.mirror_y(), Orient::MIRROR_Y);
        // Rotate 90° then mirror x: (2,1) → (-1,2) → (1,2).
        assert_eq!(Orient::rotation(1).mirror_x().apply_vec(P), Vec2::new(1.0, 2.0));
    }

    #[test]
    fn mirror_then_rotate_is_the_canonical_pair() {
        let o = Orient::from_angle(90, true);
        assert_eq!(o, Ok(Orient::MIRROR_X.rotate_ccw()));
        // Mirror (2,1) → (-2,1), then 90° CCW → (-1,-2).
        assert_eq!(o.map(|o| o.apply_vec(P)), Ok(Vec2::new(-1.0, -2.0)));
    }

    #[test]
    fn angle_conversion() {
        assert_eq!(Orient::from_angle(-90, false), Ok(Orient::rotation(3)));
        assert_eq!(Orient::from_angle(450, true).map(Orient::angle_degrees), Ok(90));
        assert_eq!(Orient::from_angle(-2_147_483_520, false).map(Orient::quarter_turns), Ok(0));
        assert_eq!(Orient::from_angle(45, false), Err(GeomError::AngleNotQuarterTurn(45)));
        assert_eq!(Orient::from_angle(i32::MAX, true), Err(GeomError::AngleNotQuarterTurn(i32::MAX)));
        assert_eq!(Orient::from_angle(i32::MIN, false), Err(GeomError::AngleNotQuarterTurn(i32::MIN)));
        assert!(Orient::MIRROR_Y.is_mirrored());
        assert_eq!(Orient::MIRROR_Y.angle_degrees(), 180);
    }

    #[test]
    fn all_lists_eight_distinct_orientations() {
        for (i, a) in Orient::ALL.iter().enumerate() {
            for b in Orient::ALL.iter().skip(i + 1) {
                assert_ne!(a, b);
                assert_ne!(a.apply_vec(P), b.apply_vec(P));
            }
        }
    }

    #[test]
    fn mul_is_then_reversed_and_matrix_matches_apply() {
        for a in Orient::ALL {
            for b in Orient::ALL {
                assert_eq!(a * b, b.then(a));
            }
            let [[m00, m01], [m10, m11]] = a.matrix();
            assert_eq!(a.apply_vec(P), Vec2::new(m00 * P.x + m01 * P.y, m10 * P.x + m11 * P.y));
        }
    }
}
