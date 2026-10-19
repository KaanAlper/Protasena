//! Footprint pads, drills and custom pad shapes.

use crate::common::{At, Pt};
use crate::decode::{flag, int, invalid, is_word, num1, number, pt, put, required, string, text, text1, texts};
use crate::error::Result;
use crate::report::{Inspect, Report, Unknown, keyword_enum};
use crate::sexpr::Sexpr;

use super::FpShape;
use super::parts::Overrides;

keyword_enum!(
    /// Pad type.
    PadType, "pad.type" { ThruHole = "thru_hole", Smd = "smd", Connect = "connect", NpThruHole = "np_thru_hole" }
);

keyword_enum!(
    /// Pad shape.
    PadShape, "pad.shape" {
        Circle = "circle", Rect = "rect", Oval = "oval", Trapezoid = "trapezoid",
        RoundRect = "roundrect", Custom = "custom",
    }
);

keyword_enum!(
    /// A chamfered corner of a rectangular pad.
    ChamferCorner, "pad.chamfer" {
        TopLeft = "top_left", TopRight = "top_right", BottomLeft = "bottom_left", BottomRight = "bottom_right",
    }
);

/// `(drill [oval] DIAMETER [WIDTH] [(offset X Y)])`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Drill {
    pub oval: bool,
    /// Hole diameter; for an oval hole, its size along X.
    pub diameter: Option<f64>,
    /// Size along Y of an oval hole.
    pub width: Option<f64>,
    pub offset: Option<Pt>,
    pub unknown: Vec<Unknown>,
}

impl Drill {
    fn decode(node: &Sexpr) -> Result<Self> {
        let mut d = Self::default();
        for item in node.args() {
            if is_word(item, "oval") {
                d.oval = true;
            } else if item.head() == Some("offset") {
                put(&mut d.offset, pt(item)?, item, &mut d.unknown);
            } else if item.as_atom().is_some() && d.diameter.is_none() {
                d.diameter = Some(number(item, "drill diameter")?);
            } else if item.as_atom().is_some() && d.width.is_none() {
                d.width = Some(number(item, "drill width")?);
            } else {
                d.unknown.push(Unknown::new("drill", item));
            }
        }
        Ok(d)
    }
}

/// `(options (clearance outline|convexhull) (anchor rect|circle))` of a custom pad.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CustomPadOptions {
    pub clearance: Option<String>,
    pub anchor: Option<String>,
    pub unknown: Vec<Unknown>,
}

/// `(primitives GRAPHIC_ITEMS… (width W) [(fill yes)])` of a custom pad.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Primitives {
    pub items: Vec<FpShape>,
    pub width: Option<f64>,
    pub fill: Option<bool>,
    pub unknown: Vec<Unknown>,
}

/// `(pad "NUMBER" TYPE SHAPE (at …) (size X Y) …)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Pad {
    pub number: String,
    pub kind: PadType,
    pub shape: PadShape,
    pub at: At,
    pub locked: bool,
    /// Width and height.
    pub size: Pt,
    pub drill: Option<Drill>,
    pub layers: Vec<String>,
    /// `pad_prop_bga`, `pad_prop_fiducial_glob`, … .
    pub property: Option<String>,
    /// `(remove_unused_layer)` (KiCad 6) or `(remove_unused_layers yes|no)` (KiCad 7+).
    pub remove_unused_layers: Option<bool>,
    pub keep_end_layers: Option<bool>,
    pub roundrect_rratio: Option<f64>,
    pub chamfer_ratio: Option<f64>,
    pub chamfer: Vec<ChamferCorner>,
    /// `(rect_delta DX DY)`: the trapezoid's deformation.
    pub rect_delta: Option<Pt>,
    pub net: Option<(i64, String)>,
    pub uuid: Option<String>,
    pub pinfunction: Option<String>,
    pub pintype: Option<String>,
    pub die_length: Option<f64>,
    pub overrides: Overrides,
    pub options: Option<CustomPadOptions>,
    pub primitives: Option<Primitives>,
    pub unknown: Vec<Unknown>,
}

impl Pad {
    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        let mut p = Self {
            number: text(node, 0, "pad number")?,
            kind: PadType::from_keyword(&text(node, 1, "pad type")?),
            shape: PadShape::from_keyword(&text(node, 2, "pad shape")?),
            at: At::default(),
            locked: false,
            size: Pt::default(),
            drill: None,
            layers: Vec::new(),
            property: None,
            remove_unused_layers: None,
            keep_end_layers: None,
            roundrect_rratio: None,
            chamfer_ratio: None,
            chamfer: Vec::new(),
            rect_delta: None,
            net: None,
            uuid: None,
            pinfunction: None,
            pintype: None,
            die_length: None,
            overrides: Overrides::default(),
            options: None,
            primitives: None,
            unknown: Vec::new(),
        };
        let (mut at, mut size) = (None, None);
        for item in node.args().iter().skip(3) {
            let u = &mut p.unknown;
            match item.name() {
                "locked" => p.locked = flag(item)?,
                _ if item.as_list().is_none() => u.push(Unknown::new("pad", item)),
                "at" => put(&mut at, At::decode(item)?, item, u),
                "size" => put(&mut size, pt(item)?, item, u),
                "drill" => put(&mut p.drill, Drill::decode(item)?, item, u),
                "layers" => p.layers = texts(item, "pad layer")?,
                "property" => put(&mut p.property, text1(item)?, item, u),
                "remove_unused_layer" | "remove_unused_layers" => p.remove_unused_layers = Some(flag(item)?),
                "keep_end_layers" => p.keep_end_layers = Some(flag(item)?),
                "roundrect_rratio" => put(&mut p.roundrect_rratio, num1(item)?, item, u),
                "chamfer_ratio" => put(&mut p.chamfer_ratio, num1(item)?, item, u),
                "chamfer" => {
                    for c in item.args() {
                        p.chamfer.push(ChamferCorner::from_keyword(&string(c, "chamfer corner")?));
                    }
                }
                "rect_delta" => put(&mut p.rect_delta, pt(item)?, item, u),
                "net" => put(&mut p.net, (int(item, 0, "net number")?, text(item, 1, "net name")?), item, u),
                "uuid" | "tstamp" => put(&mut p.uuid, text1(item)?, item, u),
                "pinfunction" => put(&mut p.pinfunction, text1(item)?, item, u),
                "pintype" => put(&mut p.pintype, text1(item)?, item, u),
                "die_length" => put(&mut p.die_length, num1(item)?, item, u),
                "options" => put(&mut p.options, decode_options(item)?, item, u),
                "primitives" => put(&mut p.primitives, decode_primitives(item)?, item, u),
                _ => {
                    if !p.overrides.take(item)? {
                        u.push(Unknown::new("pad", item));
                    }
                }
            }
        }
        p.at = required(at, node, "at")?;
        p.size = match size {
            Some(s) => s,
            None if p.shape == PadShape::Custom => Pt::default(),
            None => return Err(invalid(node, "(pad) is missing (size)")),
        };
        Ok(p)
    }
}

fn decode_options(node: &Sexpr) -> Result<CustomPadOptions> {
    let mut o = CustomPadOptions::default();
    for item in node.args() {
        match item.head() {
            Some("clearance") => o.clearance = Some(text1(item)?),
            Some("anchor") => o.anchor = Some(text1(item)?),
            _ => o.unknown.push(Unknown::new("options", item)),
        }
    }
    Ok(o)
}

fn decode_primitives(node: &Sexpr) -> Result<Primitives> {
    let mut p = Primitives::default();
    for item in node.args() {
        match item.head() {
            Some("width") => p.width = Some(num1(item)?),
            Some("fill") => p.fill = Some(flag(item)?),
            _ => match FpShape::decode(item)? {
                Some(s) => p.items.push(s),
                None => p.unknown.push(Unknown::new("primitives", item)),
            },
        }
    }
    Ok(p)
}

impl Inspect for Pad {
    fn visit(&self, r: &mut Report) {
        self.kind.visit(r);
        self.shape.visit(r);
        self.chamfer.visit(r);
        self.drill.as_ref().map(|d| &d.unknown).visit(r);
        self.overrides.unknown.visit(r);
        self.options.as_ref().map(|o| &o.unknown).visit(r);
        if let Some(p) = &self.primitives {
            p.items.visit(r);
            p.unknown.visit(r);
        }
        self.unknown.visit(r);
    }
}
