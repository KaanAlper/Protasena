//! Where a placed component sits on the sheet: its body outline and its visible pin ends.

use protasena_doc::Component;
use protasena_geom::{Pt, Rect, Vec2};
use protasena_library::{Device, Library, Shape};
use protasena_netlist::{DeviceSource, PlacedPin, gate_units, placed_pins};

/// Half size of the box that stands in for a component whose device is not in the library.
pub const UNKNOWN_HALF: f64 = 100.0;

/// The device a component names, if the library has it.
pub fn device_of<'a>(library: &'a Library, c: &Component) -> Option<&'a Device> {
    DeviceSource::device(library, &c.device)
}

/// Bounds of the body graphics of gate `unit`, in symbol coordinates (`None` without graphics).
pub fn body_local(device: &Device, unit: u32) -> Option<Rect> {
    let mut points: Vec<Pt> = Vec::new();
    for g in gate_units(device, unit).into_iter().flat_map(|u| &u.graphics) {
        match &g.shape {
            Shape::Polyline { points: p } | Shape::Bezier { points: p } => points.extend(p.iter().copied()),
            Shape::Rect { a, b } => points.extend([*a, *b]),
            Shape::Circle { center, radius } => {
                let r = Vec2::new(*radius, *radius);
                points.extend([*center - r, *center + r]);
            }
            Shape::Arc { start, mid, end } => points.extend([*start, *mid, *end]),
            Shape::Text { at, text, size, .. } => {
                let half = Vec2::new(*size * 0.3 * text.chars().count().min(256) as f64, *size * 0.5);
                points.extend([*at - half, *at + half]);
            }
        }
    }
    Rect::from_points(points.into_iter().filter(|p| p.is_finite()))
}

/// The component's body on the sheet. A device missing from the library gets a box around the
/// origin; a symbol without graphics gets the bounds of its pins.
pub fn body_bounds(library: &Library, c: &Component) -> Rect {
    let local = device_of(library, c)
        .and_then(|d| body_local(d, c.unit).or_else(|| Rect::from_points(gate_units(d, c.unit).into_iter().flat_map(|u| &u.pins).map(|p| p.at))));
    let local = local.unwrap_or_else(|| Rect::new(Pt::new(-UNKNOWN_HALF, -UNKNOWN_HALF), Pt::new(UNKNOWN_HALF, UNKNOWN_HALF)));
    c.transform.apply_rect(local)
}

/// The visible pins of a placed component, in sheet coordinates.
pub fn visible_pins(library: &Library, c: &Component) -> Vec<PlacedPin> {
    device_of(library, c).map(|d| placed_pins(c, d).into_iter().filter(|p| !p.hidden).collect()).unwrap_or_default()
}

/// Body and visible pin ends together: what a click on the component hits.
pub fn outline_bounds(library: &Library, c: &Component) -> Rect {
    visible_pins(library, c).iter().fold(body_bounds(library, c), |r, p| r.include(p.at))
}

/// The direction a wire leaves a pin end: away from the body.
pub fn outward(pin: &PlacedPin) -> Vec2 {
    -pin.dir.unit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use protasena_doc::{ComponentId, DeviceRef};
    use protasena_geom::{Orient, Transform};

    #[test]
    fn resistor_body_and_pins_follow_the_placement() {
        let lib = Library::builtin();
        let c = Component::new(ComponentId(1), DeviceRef::new("", "RES"), "R1", "10k")
            .placed(Transform::new(Orient::rotation(1), Vec2::new(1000.0, 0.0)));
        let body = body_bounds(&lib, &c);
        assert_eq!((body.min, body.max), (Pt::new(960.0, -100.0), Pt::new(1040.0, 100.0)));
        let pins = visible_pins(&lib, &c);
        assert_eq!(pins.iter().map(|p| p.at).collect::<Vec<_>>(), [Pt::new(1000.0, -200.0), Pt::new(1000.0, 200.0)]);
        assert_eq!(outward(&pins[0]), Vec2::new(0.0, -1.0));
        let outline = outline_bounds(&lib, &c);
        assert_eq!((outline.min.y, outline.max.y), (-200.0, 200.0));
    }

    #[test]
    fn unknown_device_gets_a_box() {
        let lib = Library::builtin();
        let c = Component::new(ComponentId(1), DeviceRef::new("", "NOPE"), "U1", "");
        let body = body_bounds(&lib, &c);
        assert_eq!(body.width(), 2.0 * UNKNOWN_HALF);
        assert!(visible_pins(&lib, &c).is_empty());
    }
}
