//! Grid snapping.
//!
//! Snapping rounds each coordinate to the nearest multiple of the grid step (halves round away
//! from zero). It is total: for input it cannot snap it returns the input unchanged instead of
//! failing, so callers on hot paths (cursor tracking) need no error handling:
//! - a grid step that is not finite and greater than zero (`<= 0`, NaN, ±∞) snaps nothing;
//! - a NaN or infinite coordinate is returned as is;
//! - a coordinate whose quotient by the step overflows (a tiny step against a huge value) is
//!   returned as is.
//!
//! Where a bad step should be reported, validate it once with [`Grid::new`].

use crate::error::GeomError;
use crate::point::Pt;

/// A validated grid step: finite and greater than zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid(f64);

impl Grid {
    /// A grid with the given step.
    ///
    /// # Errors
    /// [`GeomError::InvalidGrid`] unless `step` is finite and greater than zero.
    pub fn new(step: f64) -> Result<Grid, GeomError> {
        if is_valid_step(step) { Ok(Grid(step)) } else { Err(GeomError::InvalidGrid(step)) }
    }

    /// The grid step.
    pub fn step(self) -> f64 {
        self.0
    }

    /// `p` snapped to this grid (see [`snap`]).
    pub fn snap(self, p: Pt) -> Pt {
        snap(p, self.0)
    }
}

/// `p` with each coordinate rounded to the nearest multiple of `grid` (see the module docs for
/// non-finite and invalid input, which is returned unchanged).
pub fn snap(p: Pt, grid: f64) -> Pt {
    Pt::new(snap_scalar(p.x, grid), snap_scalar(p.y, grid))
}

/// `v` rounded to the nearest multiple of `grid`; `v` itself when either cannot be snapped.
/// A result of zero is always `+0.0`.
pub fn snap_scalar(v: f64, grid: f64) -> f64 {
    if !is_valid_step(grid) || !v.is_finite() {
        return v;
    }
    let snapped = (v / grid).round() * grid;
    // `+ 0.0` turns -0.0 into +0.0 so snapped values compare and hash alike.
    if snapped.is_finite() { snapped + 0.0 } else { v }
}

fn is_valid_step(step: f64) -> bool {
    step.is_finite() && step > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_to_nearest_multiple() {
        assert_eq!(snap(Pt::new(12.0, -12.0), 10.0), Pt::new(10.0, -10.0));
        assert_eq!(snap(Pt::new(15.0, -15.0), 10.0), Pt::new(20.0, -20.0));
        assert_eq!(snap(Pt::new(0.26, 0.24), 0.5), Pt::new(0.5, 0.0));
        assert_eq!(snap_scalar(-0.4, 1.0).to_bits(), 0.0f64.to_bits());
    }

    #[test]
    fn invalid_grid_is_identity() {
        let p = Pt::new(1.3, 2.7);
        for grid in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(snap(p, grid), p, "grid {grid}");
        }
    }

    #[test]
    fn hostile_coordinates_never_panic() {
        let s = snap(Pt::new(f64::NAN, f64::INFINITY), 10.0);
        assert!(s.x.is_nan());
        assert_eq!(s.y, f64::INFINITY);
        assert_eq!(snap_scalar(f64::NEG_INFINITY, 1.0), f64::NEG_INFINITY);
        // Quotient overflows: returned unchanged.
        assert_eq!(snap_scalar(1.0e300, 1.0e-300), 1.0e300);
        assert_eq!(snap_scalar(f64::MAX, f64::MIN_POSITIVE), f64::MAX);
        // Product overflows after rounding up: returned unchanged.
        assert_eq!(snap_scalar(f64::MAX, f64::MAX / 1.5), f64::MAX);
    }

    #[test]
    fn grid_validates_its_step() {
        assert_eq!(Grid::new(2.5).map(Grid::step), Ok(2.5));
        assert_eq!(Grid::new(0.0), Err(GeomError::InvalidGrid(0.0)));
        assert!(Grid::new(f64::NAN).is_err());
        assert!(Grid::new(-5.0).is_err());
        assert_eq!(Grid::new(10.0).map(|g| g.snap(Pt::new(14.0, 26.0))), Ok(Pt::new(10.0, 30.0)));
    }
}
