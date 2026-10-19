//! Error type of the geometry crate.

use std::fmt;

/// Invalid input to a geometry constructor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeomError {
    /// An orientation angle (degrees) that is not a multiple of 90.
    AngleNotQuarterTurn(i32),
    /// A grid step that is not a finite number greater than zero.
    InvalidGrid(f64),
}

impl fmt::Display for GeomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AngleNotQuarterTurn(deg) => write!(f, "orientation angle {deg}° is not a multiple of 90°"),
            Self::InvalidGrid(step) => write!(f, "grid step {step} is not a finite number greater than zero"),
        }
    }
}

impl std::error::Error for GeomError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_bad_value() {
        assert_eq!(GeomError::AngleNotQuarterTurn(45).to_string(), "orientation angle 45° is not a multiple of 90°");
        assert!(GeomError::InvalidGrid(-1.0).to_string().contains("-1"));
    }
}
