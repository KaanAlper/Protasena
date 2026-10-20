//! Symbols: a device's gate placed by a transform, as body graphics, pins, pin names and
//! numbers.

use protasena_geom::{Orient, Pt, Transform, Vec2};
use protasena_library::{Device, Fill, Graphic, Pin, Shape as LibShape, Unit};

use crate::geometry::{arc_through, flip_h, flip_v, readable};
use crate::prim::{HAlign, Ink, Prim, Shape, VAlign};

/// Line width of body graphics whose own width is 0 (the default), in thou.
pub const BODY_WIDTH: f64 = 8.0;
/// Line width of pins, in thou.
pub const PIN_WIDTH: f64 = 8.0;
/// Height of pin names and numbers, in thou.
pub const PIN_TEXT: f64 = 50.0;
/// Gap between a pin's body end and its name, in thou.
const NAME_GAP: f64 = 30.0;
/// Gap between a pin line and its number, in thou.
const NUMBER_GAP: f64 = 12.0;
/// Most graphics drawn for one unit (a hostile pack cannot stall the drawing).
const MAX_GRAPHICS: usize = 4096;

/// The primitives of gate `gate` of `device` placed by `t` (body, then pins).
pub fn symbol_prims(device: &Device, gate: u32, t: Transform) -> Vec<Prim> {
    let units = device.symbol.gate(gate);
    let mut out = Vec::new();
    for u in &units {
        out.extend(body_prims(u, t));
    }
    let show_names = !device.symbol.power;
    for u in &units {
        for p in u.pins.iter().filter(|p| !p.hidden) {
            out.extend(pin_prims(p, t, show_names));
        }
    }
    out
}

/// The body graphics of one unit.
pub fn body_prims(unit: &Unit, t: Transform) -> Vec<Prim> {
    unit.graphics.iter().take(MAX_GRAPHICS).filter(|g| g.shape.is_finite()).filter_map(|g| graphic(g, t)).collect()
}

fn fill_ink(f: Fill) -> Option<Ink> {
    match f {
        Fill::None => None,
        Fill::Solid => Some(Ink::Body),
        Fill::Body => Some(Ink::BodyFill),
    }
}

/// One library graphic, placed.
pub fn graphic(g: &Graphic, t: Transform) -> Option<Prim> {
    let width = if g.width > 0.0 && g.width.is_finite() { g.width } else { BODY_WIDTH };
    let fill = fill_ink(g.fill);
    let p = |q: Pt| t.apply_pt(q);
    let shape = match &g.shape {
        LibShape::Polyline { points } => {
            let closed = points.len() > 2 && points.first() == points.last();
            let mut pts: Vec<Pt> = points.iter().map(|&q| p(q)).collect();
            if closed {
                pts.pop();
            }
            Shape::Polyline { points: pts, closed }
        }
        LibShape::Rect { a, b } => {
            let (a, b) = (*a, *b);
            Shape::Polyline { points: vec![p(a), p(Pt::new(b.x, a.y)), p(b), p(Pt::new(a.x, b.y))], closed: true }
        }
        LibShape::Circle { center, radius } => Shape::Circle { center: p(*center), radius: radius.abs() },
        LibShape::Arc { start, mid, end } => match arc_through(p(*start), p(*mid), p(*end)) {
            Some(a) => Shape::Arc { center: a.center, radius: a.radius, start: a.start, sweep: a.sweep },
            None => Shape::Line { a: p(*start), b: p(*end) },
        },
        LibShape::Bezier { points } => match points.as_slice() {
            [a, b, c, d] => Shape::Bezier { points: [p(*a), p(*b), p(*c), p(*d)] },
            other => Shape::Polyline { points: other.iter().map(|&q| p(q)).collect(), closed: false },
        },
        LibShape::Text { at, text, size, angle } => {
            let (angle, turned) = readable(turned_angle(*angle, t.orient));
            let (h, v) = if turned { (flip_h(HAlign::Center), flip_v(VAlign::Middle)) } else { (HAlign::Center, VAlign::Middle) };
            return Some(Prim::new(Shape::Text { at: p(*at), text: text.clone(), size: *size, angle, h, v }, Ink::BodyText));
        }
    };
    Some(Prim::new(shape, Ink::Body).width(width).fill(fill))
}

/// A text angle (degrees) after placing with `o`.
fn turned_angle(angle: f64, o: Orient) -> f64 {
    let (sin, cos) = (angle.to_radians().sin(), angle.to_radians().cos());
    let d = o.apply_vec(Vec2::new(cos, sin));
    d.y.atan2(d.x).to_degrees()
}

/// A pin: its line, its number and (unless `show_names` is off) its name.
pub fn pin_prims(pin: &Pin, t: Transform, show_names: bool) -> Vec<Prim> {
    let mut out = Vec::new();
    if !(pin.at.is_finite() && pin.length.is_finite()) {
        return out;
    }
    let at = t.apply_pt(pin.at);
    let dir = t.orient.apply_vec(pin.orient.unit());
    let body_end = at + dir * pin.length.max(0.0);
    if pin.length > 0.0 {
        out.push(Prim::new(Shape::Line { a: at, b: body_end }, Ink::Pin).width(PIN_WIDTH));
    }
    let horizontal = dir.x.abs() >= dir.y.abs();
    let angle = if horizontal { 0.0 } else { 90.0 };
    // Along the text: towards +x (horizontal) or +y (vertical) reads forward.
    let forward = if horizontal { dir.x >= 0.0 } else { dir.y >= 0.0 };
    if show_names && pin.name_visible && has_text(&pin.name) {
        let anchor = body_end + dir * NAME_GAP;
        let h = if forward { HAlign::Left } else { HAlign::Right };
        out.push(Prim::new(Shape::Text { at: anchor, text: pin.name.clone(), size: PIN_TEXT, angle, h, v: VAlign::Middle }, Ink::PinName));
    }
    if pin.number_visible && has_text(&pin.number) && pin.length > 0.0 {
        let mid = at.midpoint(body_end);
        let side = if horizontal { Vec2::new(0.0, NUMBER_GAP) } else { Vec2::new(-NUMBER_GAP, 0.0) };
        let text = Shape::Text { at: mid + side, text: pin.number.clone(), size: PIN_TEXT, angle, h: HAlign::Center, v: VAlign::Bottom };
        out.push(Prim::new(text, Ink::PinNumber));
    }
    out
}

/// Whether a pin text is worth drawing (KiCad writes `~` for "no name").
fn has_text(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty() && s != "~"
}

#[cfg(test)]
mod tests {
    use super::*;
    use protasena_library::{Library, PinDir, PinKind};

    fn texts(prims: &[Prim]) -> Vec<(String, Pt, f64, HAlign)> {
        prims
            .iter()
            .filter_map(|p| match &p.shape {
                Shape::Text { at, text, angle, h, .. } => Some((text.clone(), *at, *angle, *h)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn resistor_symbol_follows_the_placement() {
        let lib = Library::builtin();
        let res = lib.get("RES").unwrap();
        let flat = symbol_prims(res, 0, Transform::translation(Vec2::new(1000.0, 0.0)));
        let pins: Vec<_> = flat.iter().filter(|p| p.ink == Ink::Pin).collect();
        assert_eq!(pins.len(), 2);
        assert!(matches!(pins[0].shape, Shape::Line { a, .. } if a == Pt::new(800.0, 0.0)), "{:?}", pins[0]);
        let turned = symbol_prims(res, 0, Transform::new(Orient::rotation(1), Vec2::new(1000.0, 0.0)));
        let pins: Vec<_> = turned.iter().filter(|p| p.ink == Ink::Pin).collect();
        assert!(matches!(pins[0].shape, Shape::Line { a, .. } if a == Pt::new(1000.0, -200.0)), "{:?}", pins[0]);
        assert!(flat.iter().any(|p| p.ink == Ink::Body));
        assert!(symbol_prims(res, 7, Transform::IDENTITY).is_empty(), "a gate that does not exist draws nothing");
    }

    #[test]
    fn pin_texts_sit_inside_the_body_and_turn_with_the_pin() {
        let pin = Pin::new("3", "OUT", PinKind::Output, Pt::new(300.0, 0.0), PinDir::Left, 100.0);
        let prims = pin_prims(&pin, Transform::IDENTITY, true);
        let t = texts(&prims);
        assert_eq!(t[0], ("OUT".into(), Pt::new(170.0, 0.0), 0.0, HAlign::Right));
        assert_eq!(t[1].0, "3");
        let up = pin_prims(&pin, Transform::from_orient(Orient::rotation(1)), true);
        let t = texts(&up);
        assert_eq!((t[0].2, t[0].3), (90.0, HAlign::Right), "a pin above the body: its name reads upwards, ending inside");
        assert!(texts(&pin_prims(&pin, Transform::IDENTITY, false)).iter().all(|t| t.0 != "OUT"));
        let hidden_name = Pin { name: "~".into(), ..pin };
        assert_eq!(texts(&pin_prims(&hidden_name, Transform::IDENTITY, true)).len(), 1);
    }

    #[test]
    fn arcs_rects_and_closed_polylines_are_converted() {
        let rect = Graphic::new(LibShape::Rect { a: Pt::new(-10.0, -5.0), b: Pt::new(10.0, 5.0) }).filled(Fill::Body);
        let p = graphic(&rect, Transform::IDENTITY).unwrap();
        assert!(matches!(&p.shape, Shape::Polyline { points, closed: true } if points.len() == 4));
        assert_eq!(p.fill, Some(Ink::BodyFill));
        let arc = Graphic::new(LibShape::Arc { start: Pt::new(10.0, 0.0), mid: Pt::new(0.0, 10.0), end: Pt::new(-10.0, 0.0) });
        let mirrored = graphic(&arc, Transform::from_orient(Orient::MIRROR_X)).unwrap();
        assert!(matches!(mirrored.shape, Shape::Arc { radius, .. } if (radius - 10.0).abs() < 1e-9));
        let tri = Graphic::new(LibShape::Polyline { points: vec![Pt::ORIGIN, Pt::new(1.0, 0.0), Pt::new(0.0, 1.0), Pt::ORIGIN] });
        assert!(matches!(graphic(&tri, Transform::IDENTITY).unwrap().shape, Shape::Polyline { ref points, closed: true } if points.len() == 3));
        let text = Graphic::new(LibShape::Text { at: Pt::ORIGIN, text: "+".into(), size: 50.0, angle: 0.0 });
        let upside = graphic(&text, Transform::from_orient(Orient::rotation(2))).unwrap();
        assert!(matches!(upside.shape, Shape::Text { angle, .. } if angle.abs() < 1e-9), "turned texts stay readable");
    }
}
