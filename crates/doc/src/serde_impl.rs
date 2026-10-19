//! `serde` helpers (feature `serde`) for the geometry types `protasena-geom` does not serialize:
//! an [`Orient`] is `{"angle": 90, "mirror": true}` (both optional), a [`Transform`] adds
//! `"at": [x, y]`. An angle that is not a multiple of 90° is an error.

use protasena_geom::{Orient, Transform, Vec2};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
struct OrientRepr {
    #[serde(default, skip_serializing_if = "is_zero")]
    angle: i32,
    #[serde(default, skip_serializing_if = "is_false")]
    mirror: bool,
}

#[derive(Serialize, Deserialize)]
struct TransformRepr {
    at: Vec2,
    #[serde(default, skip_serializing_if = "is_zero")]
    angle: i32,
    #[serde(default, skip_serializing_if = "is_false")]
    mirror: bool,
}

fn is_zero(v: &i32) -> bool {
    *v == 0
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// `#[serde(with = "…::orient")]` for [`Orient`] fields.
pub(crate) mod orient {
    use super::*;

    pub(crate) fn serialize<S: Serializer>(o: &Orient, s: S) -> Result<S::Ok, S::Error> {
        OrientRepr { angle: o.angle_degrees(), mirror: o.is_mirrored() }.serialize(s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Orient, D::Error> {
        let r = OrientRepr::deserialize(d)?;
        Orient::from_angle(r.angle, r.mirror).map_err(D::Error::custom)
    }
}

/// `#[serde(with = "…::transform")]` for [`Transform`] fields.
pub(crate) mod transform {
    use super::*;

    pub(crate) fn serialize<S: Serializer>(t: &Transform, s: S) -> Result<S::Ok, S::Error> {
        TransformRepr { at: t.offset, angle: t.orient.angle_degrees(), mirror: t.orient.is_mirrored() }.serialize(s)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Transform, D::Error> {
        let r = TransformRepr::deserialize(d)?;
        let orient = Orient::from_angle(r.angle, r.mirror).map_err(D::Error::custom)?;
        Ok(Transform::new(orient, r.at))
    }
}

#[cfg(test)]
mod tests {
    use crate::items::{Component, DeviceRef, Label};
    use crate::{ComponentId, LabelId};
    use protasena_geom::{Orient, Pt, Transform, Vec2};

    #[test]
    fn transforms_and_orients_are_compact() {
        let c = Component::new(ComponentId(1), DeviceRef::new("GENERIC", "RES"), "R1", "10k")
            .placed(Transform::new(Orient::rotation(1).mirror_x(), Vec2::new(100.0, 200.0)));
        let json = serde_json::to_value(&c).unwrap();
        assert_eq!(json["transform"], serde_json::json!({"at": [100.0, 200.0], "angle": 270, "mirror": true}));
        assert_eq!(serde_json::from_value::<Component>(json).unwrap(), c);
        let l: Label = serde_json::from_str(r#"{"id":3,"at":[0,0],"text":"SDA"}"#).unwrap();
        assert_eq!((l.id, l.at, l.orient), (LabelId(3), Pt::ORIGIN, Orient::IDENTITY));
    }

    #[test]
    fn bad_angles_are_errors() {
        assert!(serde_json::from_str::<Label>(r#"{"id":3,"at":[0,0],"text":"A","orient":{"angle":45}}"#).is_err());
        let bad = r#"{"id":1,"device":{"name":"RES"},"transform":{"at":[0,0],"angle":2147483647}}"#;
        assert!(serde_json::from_str::<Component>(bad).is_err());
        let ok: Component = serde_json::from_str(r#"{"id":1,"device":{"name":"RES"}}"#).unwrap();
        assert_eq!(ok.transform, Transform::IDENTITY);
        assert!(ok.reference_label.visible);
    }
}
