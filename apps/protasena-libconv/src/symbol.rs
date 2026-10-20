//! KiCad symbols (already resolved: `extends` applied) → Protasena symbols.
//!
//! KiCad keeps a symbol's drawing in sub-units numbered `UNIT_STYLE`, where unit 0 is shared by
//! every unit and style 0 by every body style. Protasena stores each (unit, body style) pair as
//! one complete [`Unit`], so the shared parts are copied into every pair they belong to.

use std::collections::{BTreeSet, HashMap};

use protasena_geom::Pt;
use protasena_kicad::common::{Effects, FillType, Stroke};
use protasena_kicad::symbol::{Graphic as KGraphic, GraphicKind, Symbol as KSymbol, TextBoxGeometry};
use protasena_library::{Fill, Graphic, Shape, Symbol, Unit};

use crate::coords;
use crate::pin::{self, PinText};
use crate::report::Notes;

/// KiCad's default text height, 1.27 mm (50 th).
pub const DEFAULT_TEXT_MM: f64 = 1.27;

/// Converts a resolved symbol.
pub fn convert(s: &KSymbol, notes: &mut Notes) -> Symbol {
    let text = PinText { names_hidden: s.pin_names.as_ref().is_some_and(|p| p.hide), numbers_hidden: s.pin_numbers_hide.unwrap_or(false) };
    let numbers = non_zero(s.units.iter().map(|u| u.unit));
    let styles = non_zero(s.units.iter().map(|u| u.style));
    let names: HashMap<u32, &str> = s.units.iter().filter_map(|u| u.unit_name.as_deref().map(|n| (u.unit, n))).collect();
    let mut units = Vec::new();
    for &number in &numbers {
        for &style in &styles {
            let mut unit =
                Unit { name: unit_name(number, numbers.len(), names.get(&number).copied()), body_style: body_style(style), ..Unit::default() };
            let parts = s.units.iter().filter(|k| (k.unit == number || k.unit == 0) && (k.style == style || k.style == 0));
            for part in parts {
                for g in &part.graphics {
                    unit.graphics.extend(graphic(g, notes));
                }
                for p in &part.pins {
                    let (pin, marks) = pin::convert(p, text, notes);
                    unit.pins.push(pin);
                    unit.graphics.extend(marks);
                }
            }
            dedupe_pin_numbers(&mut unit, notes);
            units.push(unit);
        }
    }
    Symbol { units, power: s.power }
}

/// The distinct non-zero numbers, or just 1 when there are none.
fn non_zero(values: impl Iterator<Item = u32>) -> BTreeSet<u32> {
    let set: BTreeSet<u32> = values.filter(|&v| v != 0).collect();
    if set.is_empty() { BTreeSet::from([1]) } else { set }
}

fn body_style(style: u32) -> u8 {
    u8::try_from(style).unwrap_or(u8::MAX)
}

/// The unit's display name: KiCad's `unit_name`, else A, B, …, Z, AA, … for multi-unit
/// symbols, else empty.
pub fn unit_name(number: u32, count: usize, kicad: Option<&str>) -> String {
    if let Some(name) = kicad.filter(|n| !n.is_empty()) {
        return name.to_owned();
    }
    if count <= 1 {
        return String::new();
    }
    let mut n = number;
    let mut letters = Vec::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        letters.push(char::from(b'A' + u8::try_from(rem).unwrap_or(0)));
        n = (n - 1) / 26;
    }
    letters.iter().rev().collect()
}

/// A unit may not hold two pins with one number. KiCad allows it (several pins on one pad);
/// the second and later get `N#2`, `N#3`, … (the device's pin maps send them to pad `N`).
fn dedupe_pin_numbers(unit: &mut Unit, notes: &mut Notes) {
    let mut seen: HashMap<String, u32> = HashMap::new();
    for pin in &mut unit.pins {
        let n = seen.entry(pin.number.clone()).or_insert(0);
        *n += 1;
        if *n > 1 {
            pin.number = format!("{}#{n}", pin.number);
            notes.note("pin number repeated in a unit (renamed N#k)");
        }
    }
}

/// The pad a (possibly renamed) pin number lands on: `N#k` → `N`.
pub fn pad_number(pin_number: &str) -> &str {
    match pin_number.rsplit_once('#') {
        Some((base, k)) if !base.is_empty() && !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit()) => base,
        _ => pin_number,
    }
}

/// One KiCad graphic as zero or more Protasena graphics.
pub fn graphic(g: &KGraphic, notes: &mut Notes) -> Vec<Graphic> {
    if g.private {
        notes.note("private symbol graphics skipped");
        return Vec::new();
    }
    let width = width(g.stroke.as_ref());
    let fill = g.fill.as_ref().map_or(Fill::None, |f| fill(&f.kind));
    let one = |shape| vec![Graphic { shape, width, fill }];
    match &g.kind {
        GraphicKind::Arc { start, mid: Some(mid), end } => {
            one(Shape::Arc { start: coords::sym(*start), mid: coords::sym(*mid), end: coords::sym(*end) })
        }
        GraphicKind::Arc { start, mid: None, end } => {
            notes.note("symbol arc without mid point (drawn as a line)");
            one(Shape::line(coords::sym(*start), coords::sym(*end)))
        }
        GraphicKind::Circle { center, radius } => one(Shape::Circle { center: coords::sym(*center), radius: coords::len(*radius) }),
        GraphicKind::Rectangle { start, end } => one(Shape::Rect { a: coords::sym(*start), b: coords::sym(*end) }),
        GraphicKind::Polyline { pts } => one(Shape::Polyline { points: pts.iter().map(|p| coords::sym(*p)).collect() }),
        GraphicKind::Bezier { pts } => one(Shape::Bezier { points: pts.iter().map(|p| coords::sym(*p)).collect() }),
        GraphicKind::Text { text, at, effects } => {
            let shape =
                Shape::Text { at: coords::sym(at.pt()), text: text.clone(), size: text_size(effects.as_ref()), angle: coords::text_angle(at.angle) };
            vec![Graphic::new(shape)]
        }
        GraphicKind::TextBox { text, geometry, effects, .. } => text_box(text, geometry, effects.as_ref(), (width, fill), notes),
    }
}

/// A text box: its frame (with the box's stroke and fill) and the text at its centre.
fn text_box(text: &str, geometry: &TextBoxGeometry, effects: Option<&Effects>, (width, fill): (f64, Fill), notes: &mut Notes) -> Vec<Graphic> {
    let corners = match (geometry.at, geometry.size, geometry.start, geometry.end) {
        (Some(at), Some(size), _, _) => Some((at.pt(), protasena_kicad::common::Pt { x: at.x + size.x, y: at.y + size.y })),
        (_, _, Some(a), Some(b)) => Some((a, b)),
        _ => None,
    };
    let Some((a, b)) = corners else {
        notes.note("symbol text box without position skipped");
        return Vec::new();
    };
    let (a, b) = (coords::sym(a), coords::sym(b));
    let angle = geometry.at.map_or(0.0, |at| coords::text_angle(at.angle));
    let centre = Pt::new(coords::round((a.x + b.x) / 2.0), coords::round((a.y + b.y) / 2.0));
    notes.note("symbol text boxes (frame + centred text)");
    vec![
        Graphic { shape: Shape::Rect { a, b }, width, fill },
        Graphic::new(Shape::Text { at: centre, text: text.to_owned(), size: text_size(effects), angle }),
    ]
}

/// Stroke width in thou (0 = default).
pub fn width(stroke: Option<&Stroke>) -> f64 {
    stroke.and_then(|s| s.width).map_or(0.0, coords::len)
}

/// KiCad fill → Protasena fill: `outline` is filled with the line colour, `background` with
/// the body colour.
pub fn fill(kind: &FillType) -> Fill {
    match kind {
        FillType::Outline => Fill::Solid,
        FillType::Background => Fill::Body,
        FillType::None | FillType::Other(_) => Fill::None,
    }
}

/// Character height in thou.
pub fn text_size(effects: Option<&Effects>) -> f64 {
    coords::len(effects.and_then(|e| e.font.height).unwrap_or(DEFAULT_TEXT_MM))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_names() {
        assert_eq!(unit_name(1, 1, None), "");
        assert_eq!(unit_name(1, 4, None), "A");
        assert_eq!(unit_name(26, 30, None), "Z");
        assert_eq!(unit_name(27, 30, None), "AA");
        assert_eq!(unit_name(2, 2, Some("Power")), "Power");
    }

    #[test]
    fn renamed_pins_land_on_their_pad() {
        assert_eq!(pad_number("5#2"), "5");
        assert_eq!(pad_number("5"), "5");
        assert_eq!(pad_number("#2"), "#2");
        assert_eq!(pad_number("A#x"), "A#x");
    }
}
