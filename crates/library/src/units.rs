//! Protasena's length unit for library data: the **thou** (1/1000 inch, also called a mil).
//!
//! Every coordinate, length and size in a [`Device`](crate::Device) symbol or a
//! [`Package`](crate::Package) is an `f64` number of thou, with the y axis pointing up (the
//! convention of `protasena-geom`).
//!
//! Why thou: Proteus works on an inch grid (the default schematic snap is 0.1 in = 100 th) and
//! KiCad draws library symbols on a 50 mil (= 50 th) pin grid, so the pins and grid-aligned
//! graphics of both land on whole numbers. Converting KiCad's millimetres (1 mm = 1000/25.4 th)
//! is exact for grid-aligned values once the float noise of the division is rounded off, which
//! [`from_mm`] does (to 1/10 000 th, about 2.5 nm).

/// Thou per inch.
pub const PER_INCH: f64 = 1000.0;

/// Thou per millimetre (1000 / 25.4).
pub const PER_MM: f64 = 1000.0 / 25.4;

/// The default schematic grid, 0.1 in.
pub const GRID: f64 = 100.0;

/// Resolution [`from_mm`] rounds to, in thou.
const ROUND_TO: f64 = 1e-4;

/// Millimetres to thou, rounded to 1/10 000 th so that values on an inch grid come out whole
/// (`from_mm(1.27) == 50.0`, `from_mm(2.54) == 100.0`). Non-finite input is returned as is.
pub fn from_mm(mm: f64) -> f64 {
    let thou = mm * PER_MM;
    let rounded = (thou / ROUND_TO).round() * ROUND_TO;
    if rounded.is_finite() { rounded } else { thou }
}

/// Thou to millimetres.
pub fn to_mm(thou: f64) -> f64 {
    thou / PER_MM
}

/// Inches to thou.
pub fn from_inch(inch: f64) -> f64 {
    inch * PER_INCH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kicad_pin_grid_converts_exactly() {
        assert_eq!(from_mm(1.27), 50.0);
        assert_eq!(from_mm(2.54), 100.0);
        assert_eq!(from_mm(-3.81), -150.0);
        assert_eq!(from_mm(25.4), 1000.0);
        for n in -200..=200 {
            assert_eq!(from_mm(f64::from(n) * 1.27), f64::from(n) * 50.0, "{n}");
        }
    }

    #[test]
    fn round_trips_and_non_finite() {
        assert!((to_mm(from_mm(1.6)) - 1.6).abs() < 1e-6);
        assert_eq!(from_inch(0.1), GRID);
        assert!(from_mm(f64::NAN).is_nan());
        assert_eq!(from_mm(f64::INFINITY), f64::INFINITY);
        assert_eq!(from_mm(f64::MAX), f64::MAX * PER_MM);
    }
}
