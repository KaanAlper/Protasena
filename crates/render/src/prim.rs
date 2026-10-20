//! Drawing primitives in sheet coordinates (thou, y up), with a semantic [`Ink`] each: the
//! toolkit maps inks to its theme colours, so this crate holds no colour at all.

use protasena_doc::ItemId;
use protasena_geom::{Pt, Rect, Vec2};

/// What a primitive draws, for the theme to colour it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ink {
    /// A component's body outline.
    Body,
    /// The body fill of a closed shape (`Fill::Body`).
    BodyFill,
    /// Text drawn as part of a symbol body.
    BodyText,
    /// A pin line.
    Pin,
    /// A pin name.
    PinName,
    /// A pin number.
    PinNumber,
    /// A component's reference (`R1`).
    Reference,
    /// A component's value (`10k`).
    Value,
    /// A component whose device is not in the library.
    Missing,
    /// A wire.
    Wire,
    /// A junction dot.
    Junction,
    /// A bus or bus entry.
    Bus,
    /// A wire label.
    Label,
    /// A terminal symbol.
    Terminal,
    /// A terminal's name.
    TerminalText,
    /// Free text.
    Text,
    /// The sheet border.
    SheetBorder,
    /// A package pad.
    Pad,
    /// A package's silkscreen drawing.
    Silk,
}

/// Horizontal alignment of a text about its anchor, along the text's own direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HAlign {
    /// The text starts at the anchor.
    #[default]
    Left,
    /// The text is centred on the anchor.
    Center,
    /// The text ends at the anchor.
    Right,
}

/// Vertical alignment of a text about its anchor, across the text's direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VAlign {
    /// The text sits on the anchor (the anchor is at its bottom).
    #[default]
    Bottom,
    /// The text is centred on the anchor.
    Middle,
    /// The text hangs from the anchor.
    Top,
}

/// One geometric primitive.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// A straight line.
    Line {
        /// One end.
        a: Pt,
        /// The other end.
        b: Pt,
    },
    /// Connected segments; `closed` joins the last point to the first.
    Polyline {
        /// Vertices.
        points: Vec<Pt>,
        /// Whether the outline is closed (and may be filled).
        closed: bool,
    },
    /// A circle.
    Circle {
        /// Centre.
        center: Pt,
        /// Radius.
        radius: f64,
    },
    /// A circular arc from `start` radians, sweeping `sweep` radians (positive = counter-clockwise).
    Arc {
        /// Centre.
        center: Pt,
        /// Radius.
        radius: f64,
        /// Start angle in radians, counter-clockwise from +x.
        start: f64,
        /// Sweep in radians.
        sweep: f64,
    },
    /// A cubic Bézier curve: start, two control points, end.
    Bezier {
        /// Control polygon.
        points: [Pt; 4],
    },
    /// A text run.
    Text {
        /// Anchor.
        at: Pt,
        /// The text.
        text: String,
        /// Character height in thou.
        size: f64,
        /// Direction in degrees, counter-clockwise (0 or 90 for readable texts).
        angle: f64,
        /// Alignment along the text.
        h: HAlign,
        /// Alignment across the text.
        v: VAlign,
    },
}

impl Shape {
    /// Whether every coordinate and length is a finite number.
    pub fn is_finite(&self) -> bool {
        match self {
            Shape::Line { a, b } => a.is_finite() && b.is_finite(),
            Shape::Polyline { points, .. } => points.iter().all(|p| p.is_finite()),
            Shape::Bezier { points } => points.iter().all(|p| p.is_finite()),
            Shape::Circle { center, radius } => center.is_finite() && radius.is_finite(),
            Shape::Arc { center, radius, start, sweep } => center.is_finite() && radius.is_finite() && start.is_finite() && sweep.is_finite(),
            Shape::Text { at, size, angle, .. } => at.is_finite() && size.is_finite() && angle.is_finite(),
        }
    }
}

/// A shape with its ink, line width, fill and the design item it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct Prim {
    /// The geometry.
    pub shape: Shape,
    /// Line or text ink.
    pub ink: Ink,
    /// Line width in thou (0 = a hairline).
    pub width: f64,
    /// Fill of a closed shape (polyline, circle).
    pub fill: Option<Ink>,
    /// The design item drawn (for selection and hover highlighting).
    pub owner: Option<ItemId>,
}

impl Prim {
    /// A primitive with a hairline and no fill.
    pub fn new(shape: Shape, ink: Ink) -> Self {
        Self { shape, ink, width: 0.0, fill: None, owner: None }
    }

    /// The same primitive with a line width.
    pub fn width(self, width: f64) -> Self {
        Self { width, ..self }
    }

    /// The same primitive with a fill.
    pub fn fill(self, fill: Option<Ink>) -> Self {
        Self { fill, ..self }
    }

    /// The same primitive belonging to `owner`.
    pub fn owned(self, owner: Option<ItemId>) -> Self {
        Self { owner, ..self }
    }

    /// Bounds of the primitive (texts estimated from their length).
    pub fn bounds(&self) -> Option<Rect> {
        if !self.shape.is_finite() {
            return None;
        }
        let pad = if self.width.is_finite() { self.width.max(0.0) / 2.0 } else { 0.0 };
        let r = match &self.shape {
            Shape::Line { a, b } => Rect::from_points([*a, *b]),
            Shape::Polyline { points, .. } => Rect::from_points(points.iter().copied()),
            Shape::Bezier { points } => Rect::from_points(points.iter().copied()),
            Shape::Circle { center, radius } | Shape::Arc { center, radius, .. } => {
                let r = Vec2::new(*radius, *radius);
                Some(Rect::new(*center - r, *center + r))
            }
            Shape::Text { at, text, size, angle, h, v } => Some(crate::geometry::text_box(*at, text, *size, *angle, *h, *v)),
        }?;
        Some(r.inflate(pad))
    }
}

/// Primitives in drawing order (later ones on top), with their joint bounds.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DisplayList {
    /// The primitives.
    pub prims: Vec<Prim>,
}

impl DisplayList {
    /// Adds a primitive.
    pub fn push(&mut self, p: Prim) {
        self.prims.push(p);
    }

    /// Adds several primitives.
    pub fn extend(&mut self, ps: impl IntoIterator<Item = Prim>) {
        self.prims.extend(ps);
    }

    /// Bounds of everything drawn, if anything is.
    pub fn bounds(&self) -> Option<Rect> {
        self.prims.iter().filter_map(Prim::bounds).reduce(Rect::union)
    }

    /// The primitives drawn for `owner`.
    pub fn of(&self, owner: ItemId) -> impl Iterator<Item = &Prim> {
        self.prims.iter().filter(move |p| p.owner == Some(owner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_cover_every_primitive() {
        let mut list = DisplayList::default();
        assert!(list.bounds().is_none());
        list.push(Prim::new(Shape::Line { a: Pt::new(0.0, 0.0), b: Pt::new(100.0, 50.0) }, Ink::Wire));
        list.push(Prim::new(Shape::Circle { center: Pt::new(-100.0, 0.0), radius: 20.0 }, Ink::Junction));
        let b = list.bounds().unwrap();
        assert_eq!((b.min, b.max), (Pt::new(-120.0, -20.0), Pt::new(100.0, 50.0)));
    }

    #[test]
    fn non_finite_geometry_has_no_bounds() {
        let p = Prim::new(Shape::Line { a: Pt::new(f64::NAN, 0.0), b: Pt::new(1.0, 1.0) }, Ink::Wire);
        assert!(p.bounds().is_none());
    }
}
