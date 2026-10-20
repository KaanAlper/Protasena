//! KiCad symbol pins → Protasena pins (with their alternate functions), with the pin's graphic
//! style drawn as body graphics.
//!
//! The library pin model has no graphic style, so the style marks are added to the unit's
//! drawing (our own simple shapes): a bubble for inverted pins, a wedge inside the body for
//! clock pins, a small triangle outside the body for active-low IEEE pins and a cross for
//! non-logic pins. Hidden pins get no marks.

use protasena_geom::{Pt, Vec2};
use protasena_kicad::symbol::{ElectricalType, Pin as KPin, PinStyle};
use protasena_library::{Fill, Graphic, Pin, PinAlternate, PinDir, PinKind, Shape};

use crate::coords;
use crate::report::Notes;

/// Size of the style marks in thou (half a 100 th grid square).
const MARK: f64 = 25.0;

/// Visibility of pin names and numbers set on the whole symbol.
#[derive(Debug, Clone, Copy, Default)]
pub struct PinText {
    /// `(pin_names hide)`.
    pub names_hidden: bool,
    /// `(pin_numbers hide)`.
    pub numbers_hidden: bool,
}

/// The pin and the graphics that draw its style.
pub fn convert(p: &KPin, text: PinText, notes: &mut Notes) -> (Pin, Vec<Graphic>) {
    notes.add("pin alternates kept", p.alternates.len() as u64);
    notes.add("pin alternate graphic styles dropped", p.alternates.iter().filter(|a| a.style != PinStyle::Line).count() as u64);
    let alternates = p.alternates.iter().map(|a| PinAlternate { name: a.name.clone(), kind: kind(&a.electrical, notes) }).collect();
    let name = if p.name.text == "~" { String::new() } else { p.name.text.clone() };
    let hidden_text = |t: &protasena_kicad::common::TextWithEffects| t.effects.as_ref().is_some_and(|e| e.hide);
    let pin = Pin {
        number: p.number.text.clone(),
        name,
        kind: kind(&p.electrical, notes),
        at: coords::sym(p.at.pt()),
        orient: dir(p.at.angle, notes),
        length: coords::len(p.length),
        hidden: p.hide,
        name_visible: !text.names_hidden && !hidden_text(&p.name),
        number_visible: !text.numbers_hidden && !hidden_text(&p.number),
        alternates,
    };
    let marks = if pin.hidden { Vec::new() } else { marks(&pin, &p.style, notes) };
    (pin, marks)
}

/// Electrical type. KiCad's `free` (a pin without electrical rules) has no counterpart and
/// becomes `unspecified`.
pub fn kind(t: &ElectricalType, notes: &mut Notes) -> PinKind {
    match t {
        ElectricalType::Input => PinKind::Input,
        ElectricalType::Output => PinKind::Output,
        ElectricalType::Bidirectional => PinKind::Bidirectional,
        ElectricalType::TriState => PinKind::TriState,
        ElectricalType::Passive => PinKind::Passive,
        ElectricalType::PowerIn => PinKind::PowerIn,
        ElectricalType::PowerOut => PinKind::PowerOut,
        ElectricalType::OpenCollector => PinKind::OpenCollector,
        ElectricalType::OpenEmitter => PinKind::OpenEmitter,
        ElectricalType::NoConnect => PinKind::NoConnect,
        ElectricalType::Unspecified => PinKind::Unspecified,
        ElectricalType::Free => {
            notes.note("pin type free → unspecified");
            PinKind::Unspecified
        }
        ElectricalType::Other(_) => {
            notes.note("pin type unknown → unspecified");
            PinKind::Unspecified
        }
    }
}

/// Direction from the connection point into the body. KiCad symbol angles: 0 = towards +x,
/// 90 = towards +y (up), 180 = towards -x, 270 = towards -y; other angles snap to the nearest.
pub fn dir(angle: f64, notes: &mut Notes) -> PinDir {
    let quarters = (angle / 90.0).round();
    if (angle - quarters * 90.0).abs() > 1e-6 {
        notes.note("pin angle not a multiple of 90° (snapped)");
    }
    match (quarters as i64).rem_euclid(4) {
        1 => PinDir::Up,
        2 => PinDir::Left,
        3 => PinDir::Down,
        _ => PinDir::Right,
    }
}

fn marks(pin: &Pin, style: &PinStyle, notes: &mut Notes) -> Vec<Graphic> {
    let d = pin.orient.unit();
    let n = d.perp();
    let body = pin.body_end();
    let mut out = Vec::new();
    let bubble = || Graphic::new(Shape::Circle { center: body - d * MARK, radius: MARK }).filled(Fill::Body);
    let wedge = || poly(&[body + n * MARK, body + d * (2.0 * MARK), body - n * MARK]);
    let low = || poly(&[body, body - d * (2.0 * MARK) - n * MARK, body - d * (2.0 * MARK)]);
    match style {
        PinStyle::Line => {}
        PinStyle::Inverted => out.push(bubble()),
        PinStyle::Clock | PinStyle::EdgeClockHigh => out.push(wedge()),
        PinStyle::InvertedClock => out.extend([bubble(), wedge()]),
        PinStyle::InputLow => out.push(low()),
        PinStyle::ClockLow => out.extend([low(), wedge()]),
        PinStyle::OutputLow => out.push(poly(&[body - n * MARK, body - d * (2.0 * MARK)])),
        PinStyle::NonLogic => {
            let c = pin.at;
            let (a, b) = (Vec2::new(MARK, MARK), Vec2::new(MARK, -MARK));
            out.extend([poly(&[c - a, c + a]), poly(&[c - b, c + b])]);
        }
        PinStyle::Other(_) => notes.note("pin style unknown (drawn as a line)"),
    }
    if !out.is_empty() {
        notes.note("pin style marks drawn");
    }
    out
}

fn poly(points: &[Pt]) -> Graphic {
    let points = points.iter().map(|p| Pt::new(coords::round(p.x), coords::round(p.y))).collect();
    Graphic::new(Shape::Polyline { points })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kicad_symbol_angles_point_into_the_body_with_y_up() {
        let mut n = Notes::default();
        assert_eq!(dir(0.0, &mut n), PinDir::Right);
        assert_eq!(dir(90.0, &mut n), PinDir::Up);
        assert_eq!(dir(180.0, &mut n), PinDir::Left);
        assert_eq!(dir(270.0, &mut n), PinDir::Down);
        assert_eq!(dir(-90.0, &mut n), PinDir::Down);
        assert_eq!(n, Notes::default());
        assert_eq!(dir(80.0, &mut n), PinDir::Up);
        assert_eq!(n.get("pin angle not a multiple of 90° (snapped)"), 1);
    }

    #[test]
    fn every_kicad_type_has_a_kind() {
        let mut n = Notes::default();
        assert_eq!(kind(&ElectricalType::OpenEmitter, &mut n), PinKind::OpenEmitter);
        assert_eq!(kind(&ElectricalType::NoConnect, &mut n), PinKind::NoConnect);
        assert_eq!(kind(&ElectricalType::Free, &mut n), PinKind::Unspecified);
        assert_eq!(kind(&ElectricalType::Other("x".into()), &mut n), PinKind::Unspecified);
        assert_eq!(n.get("pin type free → unspecified"), 1);
    }
}
