//! Optional `serde` support (feature `serde`): [`Pt`] and [`Vec2`] are written as compact
//! `[x, y]` arrays, which keeps library packs and project files small and readable.

use crate::point::{Pt, Vec2};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for Pt {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        (self.x, self.y).serialize(s)
    }
}

impl<'de> Deserialize<'de> for Pt {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let (x, y) = <(f64, f64)>::deserialize(d)?;
        Ok(Pt::new(x, y))
    }
}

impl Serialize for Vec2 {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        (self.x, self.y).serialize(s)
    }
}

impl<'de> Deserialize<'de> for Vec2 {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let (x, y) = <(f64, f64)>::deserialize(d)?;
        Ok(Vec2::new(x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_and_vectors_are_two_element_arrays() {
        assert_eq!(serde_json::to_string(&Pt::new(1.5, -2.0)).unwrap(), "[1.5,-2.0]");
        assert_eq!(serde_json::from_str::<Pt>("[100, 200]").unwrap(), Pt::new(100.0, 200.0));
        assert_eq!(serde_json::from_str::<Vec2>("[0.25,4]").unwrap(), Vec2::new(0.25, 4.0));
    }

    #[test]
    fn malformed_points_are_errors() {
        for bad in ["[1]", "[1,2,3]", "{\"x\":1,\"y\":2}", "\"1,2\"", "[1,\"a\"]", "null"] {
            assert!(serde_json::from_str::<Pt>(bad).is_err(), "{bad}");
        }
    }
}
