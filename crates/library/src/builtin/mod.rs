//! The built-in mini library: generic parts drawn here in code (our own drawings, no external
//! data), so Protasena and its tests work without a library pack.
//!
//! Symbols sit on the 100 th grid with their pin ends on it; two-terminal parts run left to
//! right with pin 1 on the left.

mod packages;
mod passive;
mod semi;

use std::sync::Arc;

use protasena_geom::Pt;

use crate::device::{Device, PackageRef, PropDef, PropKind, SimModelRef};
use crate::library::Library;
use crate::symbol::{Graphic, Pin, PinDir, PinKind, Shape, Symbol, Unit};

/// Library name of the built-in parts.
pub const BUILTIN_LIBRARY: &str = "GENERIC";

impl Library {
    /// The built-in generic parts: RES, CAP, CAP-ELEC, DIODE, LED, NPN, PNP, GROUND, POWER,
    /// OPAMP and BATTERY, with their packages (RES40, CAP10, ELEC-RAD10, DO41, LED5MM, TO92,
    /// DIL08). All in library [`BUILTIN_LIBRARY`].
    pub fn builtin() -> Library {
        let devices = vec![
            passive::res(),
            passive::cap(),
            passive::cap_elec(),
            semi::diode(),
            semi::led(),
            semi::npn(),
            semi::pnp(),
            passive::ground(),
            passive::power(),
            semi::opamp(),
            passive::battery(),
        ];
        Library::index(devices, packages::all())
    }
}

/// A device in the built-in library with its index fields filled in.
fn device(name: &str, prefix: &str, category: &str, sub_category: &str, description: &str, keywords: &[&str]) -> Device {
    Device {
        name: name.to_owned(),
        library: BUILTIN_LIBRARY.to_owned(),
        prefix: prefix.to_owned(),
        description: description.to_owned(),
        category: category.to_owned(),
        sub_category: sub_category.to_owned(),
        keywords: keywords.iter().map(|k| (*k).to_owned()).collect(),
        ..Device::default()
    }
}

fn p(x: f64, y: f64) -> Pt {
    Pt::new(x, y)
}

fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> Graphic {
    Graphic::new(Shape::line(p(x1, y1), p(x2, y2)))
}

fn poly(points: &[(f64, f64)]) -> Graphic {
    Graphic::new(Shape::Polyline { points: points.iter().map(|&(x, y)| p(x, y)).collect() })
}

fn rect(x1: f64, y1: f64, x2: f64, y2: f64) -> Graphic {
    Graphic::new(Shape::Rect { a: p(x1, y1), b: p(x2, y2) })
}

fn circle(x: f64, y: f64, radius: f64) -> Graphic {
    Graphic::new(Shape::Circle { center: p(x, y), radius })
}

fn arc(start: (f64, f64), mid: (f64, f64), end: (f64, f64)) -> Graphic {
    Graphic::new(Shape::Arc { start: p(start.0, start.1), mid: p(mid.0, mid.1), end: p(end.0, end.1) })
}

fn text(x: f64, y: f64, s: &str, size: f64) -> Graphic {
    Graphic::new(Shape::Text { at: p(x, y), text: s.to_owned(), size, angle: 0.0 })
}

fn pin(number: &str, name: &str, kind: PinKind, at: (f64, f64), orient: PinDir, length: f64) -> Pin {
    Pin::new(number, name, kind, p(at.0, at.1), orient, length)
}

/// A one-unit symbol.
fn symbol(graphics: Vec<Graphic>, pins: Vec<Pin>) -> Arc<Symbol> {
    Arc::new(Symbol { units: vec![Unit { name: String::new(), body_style: 1, graphics, pins }], power: false })
}

/// A package reference with its pin map.
fn package(name: &str, pin_map: &[(&str, &str)]) -> PackageRef {
    PackageRef::new(name, pin_map.iter().map(|&(a, b)| (a.to_owned(), b.to_owned())).collect())
}

/// The value property (`VALUE`), a number with a unit.
fn value(prompt: &str, unit: &str, default: &str) -> PropDef {
    PropDef::new("VALUE", prompt, PropKind::Float { unit: Some(unit.to_owned()) }, Some(default))
}

/// The PCB package property (`PACKAGE`), hidden on the sheet.
fn package_prop(default: &str) -> PropDef {
    PropDef::new("PACKAGE", "PCB Package", PropKind::Package, Some(default)).hidden()
}

/// An analogue simulation primitive.
fn analog(model: &str) -> Option<SimModelRef> {
    Some(SimModelRef { primitive: "ANALOG".to_owned(), model: model.to_owned(), params: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_set_is_complete_and_valid() {
        let lib = Library::builtin();
        assert_eq!(lib.issues(), Vec::new());
        let names: Vec<&str> = lib.devices().iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["RES", "CAP", "CAP-ELEC", "DIODE", "LED", "NPN", "PNP", "GROUND", "POWER", "OPAMP", "BATTERY"]);
    }
}
