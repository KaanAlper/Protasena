//! Schematic symbols: units with graphics and pins, in [thou](crate::units), y up.

use protasena_geom::{Orient, Pt, Vec2};
use serde::{Deserialize, Serialize};

/// The schematic drawing of a device: one or more units (gates of a multi-gate part).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Symbol {
    /// The units. A single-gate device has one unit; De Morgan variants of a unit are further
    /// entries with the same `name` and another `body_style`.
    #[serde(default, deserialize_with = "crate::bounded::units")]
    pub units: Vec<Unit>,
    /// A power symbol (ground, supply rail): its pin name is the net it joins.
    #[serde(default, skip_serializing_if = "is_false")]
    pub power: bool,
}

impl Symbol {
    /// Every pin of every unit.
    pub fn pins(&self) -> impl Iterator<Item = &Pin> {
        self.units.iter().flat_map(|u| u.pins.iter())
    }

    /// Whether some unit has a pin with this number.
    pub fn has_pin(&self, number: &str) -> bool {
        self.pins().any(|p| p.number == number)
    }

    /// The units that make up gate `gate`: the `gate`-th distinct unit name, in body style 1
    /// when it has one (else its first body style). Empty when the gate does not exist.
    pub fn gate(&self, gate: u32) -> Vec<&Unit> {
        let mut names: Vec<&str> = Vec::new();
        for u in &self.units {
            if !names.contains(&u.name.as_str()) {
                names.push(&u.name);
            }
        }
        let Some(name) = usize::try_from(gate).ok().and_then(|g| names.get(g).copied()) else { return Vec::new() };
        let of_name = || self.units.iter().filter(move |u| u.name == name);
        let style = if of_name().any(|u| u.body_style == 1) { 1 } else { of_name().map(|u| u.body_style).next().unwrap_or(1) };
        of_name().filter(|u| u.body_style == style).collect()
    }

    /// How many gates (distinct unit names) the symbol has.
    pub fn gate_count(&self) -> usize {
        let mut names: Vec<&str> = Vec::new();
        for u in &self.units {
            if !names.contains(&u.name.as_str()) {
                names.push(&u.name);
            }
        }
        names.len()
    }
}

/// One unit (gate) of a symbol in one body style.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Unit {
    /// Unit name (`"A"`, `"B"`, … for gates; may be empty for a single-unit symbol).
    #[serde(default)]
    pub name: String,
    /// Body style: 1 = normal, 2 = De Morgan alternate.
    #[serde(default = "one")]
    pub body_style: u8,
    /// Body drawing.
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "crate::bounded::graphics")]
    pub graphics: Vec<Graphic>,
    /// Pins.
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "crate::bounded::pins")]
    pub pins: Vec<Pin>,
}

/// One drawn shape with its stroke and fill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Graphic {
    /// The shape.
    pub shape: Shape,
    /// Line width in thou; 0 = the default width of the drawing style.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub width: f64,
    /// How a closed shape is filled.
    #[serde(default, skip_serializing_if = "Fill::is_none")]
    pub fill: Fill,
}

impl Graphic {
    /// A shape with default width and no fill.
    pub fn new(shape: Shape) -> Self {
        Self { shape, width: 0.0, fill: Fill::None }
    }

    /// The same graphic with a fill.
    pub fn filled(self, fill: Fill) -> Self {
        Self { fill, ..self }
    }
}

/// Geometric primitives of symbols and packages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Shape {
    /// Connected line segments (two points: a line). Closed when first == last.
    Polyline {
        /// Vertices.
        #[serde(deserialize_with = "crate::bounded::points")]
        points: Vec<Pt>,
    },
    /// Axis-aligned rectangle given by two opposite corners.
    Rect {
        /// One corner.
        a: Pt,
        /// The opposite corner.
        b: Pt,
    },
    /// Circle.
    Circle {
        /// Centre.
        center: Pt,
        /// Radius.
        radius: f64,
    },
    /// Circular arc through three points.
    Arc {
        /// Start point.
        start: Pt,
        /// A point on the arc between start and end.
        mid: Pt,
        /// End point.
        end: Pt,
    },
    /// Cubic Bézier curve: start, two control points, end (chains share end points).
    Bezier {
        /// Control polygon.
        #[serde(deserialize_with = "crate::bounded::points")]
        points: Vec<Pt>,
    },
    /// Text drawn as part of the body (not a property).
    Text {
        /// Anchor (centre of the text).
        at: Pt,
        /// The text.
        text: String,
        /// Character height in thou.
        size: f64,
        /// Rotation in degrees, counter-clockwise.
        #[serde(default, skip_serializing_if = "is_zero")]
        angle: f64,
    },
}

impl Shape {
    /// A two-point line.
    pub fn line(a: Pt, b: Pt) -> Self {
        Shape::Polyline { points: vec![a, b] }
    }

    /// Whether every coordinate and length of the shape is a finite number.
    pub fn is_finite(&self) -> bool {
        match self {
            Shape::Polyline { points } | Shape::Bezier { points } => points.iter().all(|p| p.is_finite()),
            Shape::Rect { a, b } => a.is_finite() && b.is_finite(),
            Shape::Circle { center, radius } => center.is_finite() && radius.is_finite(),
            Shape::Arc { start, mid, end } => start.is_finite() && mid.is_finite() && end.is_finite(),
            Shape::Text { at, size, angle, .. } => at.is_finite() && size.is_finite() && angle.is_finite(),
        }
    }
}

/// Fill of a closed shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fill {
    /// Outline only.
    #[default]
    None,
    /// Filled with the line colour.
    Solid,
    /// Filled with the body (background) colour.
    Body,
}

impl Fill {
    fn is_none(&self) -> bool {
        *self == Fill::None
    }
}

/// A connection point of a symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pin {
    /// Pin number, unique within a unit: what the package pin map refers to.
    pub number: String,
    /// Pin name (`B`, `OUT`, `VCC`, …).
    #[serde(default)]
    pub name: String,
    /// Electrical type (for the electrical rules check and the netlist).
    #[serde(default)]
    pub kind: PinKind,
    /// The connection point (outer end, where wires attach).
    pub at: Pt,
    /// Direction from the connection point to the body end of the pin.
    pub orient: PinDir,
    /// Length in thou (0 = a bare connection point).
    #[serde(default)]
    pub length: f64,
    /// Hidden pin (not drawn; typically power pins joined by name).
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
    /// Whether the name is drawn.
    #[serde(default = "yes")]
    pub name_visible: bool,
    /// Whether the number is drawn.
    #[serde(default = "yes")]
    pub number_visible: bool,
}

impl Pin {
    /// A visible pin with name and number shown.
    pub fn new(number: &str, name: &str, kind: PinKind, at: Pt, orient: PinDir, length: f64) -> Self {
        Self { number: number.to_owned(), name: name.to_owned(), kind, at, orient, length, hidden: false, name_visible: true, number_visible: true }
    }

    /// The same pin with name and number hidden (two-terminal parts).
    pub fn bare(self) -> Self {
        Self { name_visible: false, number_visible: false, ..self }
    }

    /// The end of the pin that touches the body.
    pub fn body_end(&self) -> Pt {
        self.at + self.orient.unit() * self.length
    }
}

/// Electrical type of a pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinKind {
    /// Input.
    Input,
    /// Output.
    Output,
    /// Bidirectional (input/output).
    Bidirectional,
    /// Tri-state output.
    TriState,
    /// Passive (resistor, capacitor terminals).
    Passive,
    /// Power input (supply pin of a part, or the pin of a power symbol).
    PowerIn,
    /// Power output (regulator output, source).
    PowerOut,
    /// Open collector / open drain.
    OpenCollector,
    /// Open emitter / open source.
    OpenEmitter,
    /// Not specified.
    #[default]
    Unspecified,
    /// Not connected inside the part.
    NoConnect,
}

/// Axis direction of a pin, from its connection point towards the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinDir {
    /// +x: the pin sits on the left of the body.
    Right,
    /// +y: the pin sits below the body.
    Up,
    /// -x: the pin sits on the right of the body.
    Left,
    /// -y: the pin sits above the body.
    Down,
}

impl PinDir {
    /// Unit vector of the direction.
    pub fn unit(self) -> Vec2 {
        match self {
            PinDir::Right => Vec2::new(1.0, 0.0),
            PinDir::Up => Vec2::new(0.0, 1.0),
            PinDir::Left => Vec2::new(-1.0, 0.0),
            PinDir::Down => Vec2::new(0.0, -1.0),
        }
    }

    /// The direction after placing the symbol with orientation `o`.
    pub fn transformed(self, o: Orient) -> PinDir {
        let v = o.apply_vec(self.unit());
        if v.x.abs() >= v.y.abs() {
            if v.x >= 0.0 { PinDir::Right } else { PinDir::Left }
        } else if v.y >= 0.0 {
            PinDir::Up
        } else {
            PinDir::Down
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

fn yes() -> bool {
    true
}

fn one() -> u8 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_body_end_follows_direction() {
        let p = Pin::new("1", "A", PinKind::Input, Pt::new(-200.0, 0.0), PinDir::Right, 100.0);
        assert_eq!(p.body_end(), Pt::new(-100.0, 0.0));
        let q = Pin { orient: PinDir::Down, at: Pt::new(0.0, 300.0), ..p };
        assert_eq!(q.body_end(), Pt::new(0.0, 200.0));
    }

    #[test]
    fn pin_dir_rotates_with_the_part() {
        assert_eq!(PinDir::Right.transformed(Orient::rotation(1)), PinDir::Up);
        assert_eq!(PinDir::Up.transformed(Orient::rotation(1)), PinDir::Left);
        assert_eq!(PinDir::Right.transformed(Orient::MIRROR_X), PinDir::Left);
        assert_eq!(PinDir::Down.transformed(Orient::IDENTITY), PinDir::Down);
    }

    #[test]
    fn defaults_fill_in_missing_fields() {
        let pin: Pin = serde_json::from_str(r#"{"number":"1","at":[0,0],"orient":"left"}"#).unwrap();
        assert!(pin.name_visible && pin.number_visible && !pin.hidden);
        assert_eq!(pin.kind, PinKind::Unspecified);
        let unit: Unit = serde_json::from_str("{}").unwrap();
        assert_eq!(unit.body_style, 1);
    }
}
