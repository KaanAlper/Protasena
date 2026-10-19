//! Syntax shared by symbol libraries and footprints: points, positions, strokes, fills, text
//! effects (KiCad documentation: "Common Syntax").

use crate::decode::{flag, invalid, num, num1, opt_num, pt, string, text1};
use crate::error::Result;
use crate::report::{Inspect, Report, Unknown, keyword_enum};
use crate::sexpr::Sexpr;

/// A point in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

/// `(at X Y [ANGLE])`: position and rotation in degrees (0 when absent).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct At {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
}

impl At {
    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        Ok(Self { x: num(node, 0, "at x")?, y: num(node, 1, "at y")?, angle: opt_num(node, 2, "at angle")?.unwrap_or(0.0) })
    }

    pub fn pt(&self) -> Pt {
        Pt { x: self.x, y: self.y }
    }
}

/// `(color R G B A)`: red, green, blue 0–255, alpha 0–1.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        Ok(Self { r: num(node, 0, "red")?, g: num(node, 1, "green")?, b: num(node, 2, "blue")?, a: num(node, 3, "alpha")? })
    }
}

/// `(pts (xy X Y) …)`; anything other than `xy` is kept as unknown.
pub(crate) fn decode_pts(node: &Sexpr, unknown: &mut Vec<Unknown>) -> Result<Vec<Pt>> {
    let mut pts = Vec::new();
    for item in node.args() {
        match item.head() {
            Some("xy") => pts.push(pt(item)?),
            _ => unknown.push(Unknown::new("pts", item)),
        }
    }
    Ok(pts)
}

keyword_enum!(
    /// Line style of a stroke.
    StrokeType, "stroke.type" {
        Default = "default", Solid = "solid", Dash = "dash", DashDot = "dash_dot",
        DashDotDot = "dash_dot_dot", Dot = "dot",
    }
);

/// `(stroke (width W) (type T) (color R G B A))`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stroke {
    pub width: Option<f64>,
    pub kind: Option<StrokeType>,
    pub color: Option<Color>,
    pub unknown: Vec<Unknown>,
}

impl Stroke {
    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        let mut s = Self::default();
        for item in node.args() {
            match item.head() {
                Some("width") => s.width = Some(num1(item)?),
                Some("type") => s.kind = Some(StrokeType::from_keyword(&text1(item)?)),
                Some("color") => s.color = Some(Color::decode(item)?),
                _ => s.unknown.push(Unknown::new("stroke", item)),
            }
        }
        Ok(s)
    }
}

impl Inspect for Stroke {
    fn visit(&self, r: &mut Report) {
        self.kind.visit(r);
        self.unknown.visit(r);
    }
}

keyword_enum!(
    /// How a symbol graphic is filled.
    FillType, "fill.type" { None = "none", Outline = "outline", Background = "background" }
);

/// Symbol fill: `(fill (type none|outline|background))`.
#[derive(Debug, Clone, PartialEq)]
pub struct Fill {
    pub kind: FillType,
    pub color: Option<Color>,
    pub unknown: Vec<Unknown>,
}

impl Fill {
    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        let mut f = Self { kind: FillType::None, color: None, unknown: Vec::new() };
        for item in node.args() {
            match item.head() {
                Some("type") => f.kind = FillType::from_keyword(&text1(item)?),
                Some("color") => f.color = Some(Color::decode(item)?),
                _ => f.unknown.push(Unknown::new("fill", item)),
            }
        }
        Ok(f)
    }
}

impl Inspect for Fill {
    fn visit(&self, r: &mut Report) {
        self.kind.visit(r);
        self.unknown.visit(r);
    }
}

/// Horizontal text justification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HAlign {
    Left,
    #[default]
    Center,
    Right,
}

/// Vertical text justification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VAlign {
    Top,
    #[default]
    Center,
    Bottom,
}

/// `(justify [left|right] [top|bottom] [mirror])`; centred and not mirrored by default.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Justify {
    pub h: HAlign,
    pub v: VAlign,
    pub mirror: bool,
    pub unknown: Vec<Unknown>,
}

impl Justify {
    fn decode(node: &Sexpr) -> Self {
        let mut j = Self::default();
        for item in node.args() {
            match item.as_atom() {
                Some("left") => j.h = HAlign::Left,
                Some("right") => j.h = HAlign::Right,
                Some("top") => j.v = VAlign::Top,
                Some("bottom") => j.v = VAlign::Bottom,
                Some("mirror") => j.mirror = true,
                _ => j.unknown.push(Unknown::new("justify", item)),
            }
        }
        j
    }
}

/// `(font …)`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Font {
    /// TrueType family, or `"KiCad Font"` (from KiCad 7).
    pub face: Option<String>,
    /// `(size HEIGHT WIDTH)`.
    pub height: Option<f64>,
    pub width: Option<f64>,
    pub thickness: Option<f64>,
    pub bold: bool,
    pub italic: bool,
    pub line_spacing: Option<f64>,
    pub color: Option<Color>,
    pub unknown: Vec<Unknown>,
}

impl Font {
    fn decode(node: &Sexpr) -> Result<Self> {
        let mut f = Self::default();
        for item in node.args() {
            match item.name() {
                "face" => f.face = Some(text1(item)?),
                "size" => {
                    f.height = Some(num(item, 0, "font height")?);
                    f.width = Some(num(item, 1, "font width")?);
                }
                "thickness" => f.thickness = Some(num1(item)?),
                "bold" => f.bold = flag(item)?,
                "italic" => f.italic = flag(item)?,
                "line_spacing" => f.line_spacing = Some(num1(item)?),
                "color" => f.color = Some(Color::decode(item)?),
                _ => f.unknown.push(Unknown::new("font", item)),
            }
        }
        Ok(f)
    }
}

impl Inspect for Font {
    fn visit(&self, r: &mut Report) {
        self.unknown.visit(r);
    }
}

/// `(effects (font …) (justify …) [hide])`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Effects {
    pub font: Font,
    pub justify: Justify,
    /// Bare `hide` (KiCad 6–7) or `(hide yes)` (KiCad 8+).
    pub hide: bool,
    /// `(href "URL")`: a hyperlink on the text.
    pub href: Option<String>,
    pub unknown: Vec<Unknown>,
}

impl Effects {
    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        let mut e = Self::default();
        for item in node.args() {
            match item.name() {
                "font" if item.as_list().is_some() => e.font = Font::decode(item)?,
                "justify" if item.as_list().is_some() => e.justify = Justify::decode(item),
                "hide" => e.hide = flag(item)?,
                "href" => e.href = Some(text1(item)?),
                _ => e.unknown.push(Unknown::new("effects", item)),
            }
        }
        Ok(e)
    }
}

impl Inspect for Effects {
    fn visit(&self, r: &mut Report) {
        self.font.visit(r);
        self.justify.unknown.visit(r);
        self.unknown.visit(r);
    }
}

/// A text value with its effects, e.g. a pin's `(name "CLK" (effects …))`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextWithEffects {
    pub text: String,
    pub effects: Option<Effects>,
    pub unknown: Vec<Unknown>,
}

impl TextWithEffects {
    pub(crate) fn decode(node: &Sexpr, context: &'static str) -> Result<Self> {
        let mut args = node.args().iter();
        let first = args.next().ok_or_else(|| invalid(node, format!("({}) is missing its text", node.name())))?;
        let mut t = Self { text: string(first, context)?, effects: None, unknown: Vec::new() };
        for item in args {
            match item.head() {
                Some("effects") => t.effects = Some(Effects::decode(item)?),
                _ => t.unknown.push(Unknown::new(context, item)),
            }
        }
        Ok(t)
    }
}

impl Inspect for TextWithEffects {
    fn visit(&self, r: &mut Report) {
        self.effects.visit(r);
        self.unknown.visit(r);
    }
}
