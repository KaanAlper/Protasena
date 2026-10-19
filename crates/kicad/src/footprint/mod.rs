//! `.kicad_mod` footprints (KiCad documentation: "Footprint Library File Format" and
//! "Board Common Syntax").

mod graphic;
mod pad;
mod parts;
mod text;
mod zone;

use crate::common::At;
use crate::decode::{flag, int, text, text1, texts, yes_no};
use crate::error::{Error, ErrorKind, Result};
use crate::report::{Inspect, Report, Unknown};
use crate::sexpr::{self, Sexpr};

pub use graphic::{FpShape, FpShapeKind, FpTextBox};
pub use pad::{ChamferCorner, CustomPadOptions, Drill, Pad, PadShape, PadType, Primitives};
pub use parts::{Attr, AttrType, Group, Model, Overrides, Xyz};
pub use text::{FpProperty, FpText, FpTextType, TextAttrs};
pub use zone::{ConnectPads, FilledPolygon, Keepout, KeepoutRule, Zone, ZoneFill};

/// A graphic item of a footprint.
#[derive(Debug, Clone, PartialEq)]
pub enum FpGraphic {
    /// `fp_line`, `fp_rect`, `fp_circle`, `fp_arc`, `fp_poly`, `fp_curve`.
    Shape(FpShape),
    /// `fp_text`.
    Text(FpText),
    /// `fp_text_box` (KiCad 7+).
    TextBox(FpTextBox),
}

/// A footprint file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Footprint {
    pub name: String,
    /// `(version YYYYMMDD)`.
    pub version: Option<i64>,
    pub generator: Option<String>,
    pub generator_version: Option<String>,
    pub locked: bool,
    pub placed: bool,
    pub layer: Option<String>,
    pub tedit: Option<String>,
    pub uuid: Option<String>,
    pub at: Option<At>,
    pub descr: Option<String>,
    pub tags: Option<String>,
    pub properties: Vec<FpProperty>,
    pub path: Option<String>,
    pub autoplace_cost90: Option<i64>,
    pub autoplace_cost180: Option<i64>,
    /// Footprint-wide pad clearance and mask overrides.
    pub overrides: Overrides,
    pub attr: Option<Attr>,
    pub private_layers: Vec<String>,
    pub net_tie_pad_groups: Vec<String>,
    pub graphics: Vec<FpGraphic>,
    pub pads: Vec<Pad>,
    pub groups: Vec<Group>,
    /// Zones; in footprint libraries mostly keep-out areas.
    pub zones: Vec<Zone>,
    /// `(model "PATH" …)`: 3D model references (paths only; nothing is loaded).
    pub models: Vec<Model>,
    pub embedded_fonts: Option<bool>,
    pub unknown: Vec<Unknown>,
}

impl Footprint {
    /// The property with this key (`Reference`, `Value`, `Datasheet`, …).
    pub fn property(&self, key: &str) -> Option<&FpProperty> {
        self.properties.iter().find(|p| p.key == key)
    }

    /// Everything kept unread in the footprint.
    pub fn report(&self) -> Report {
        Inspect::report(self)
    }

    /// Reads a footprint from an already parsed `(footprint …)` tree (`module` before KiCad 6).
    pub fn from_sexpr(root: &Sexpr) -> Result<Self> {
        if !matches!(root.head(), Some("footprint" | "module")) {
            let found = root.name().to_owned();
            return Err(Error::new(ErrorKind::WrongRoot { expected: "footprint", found }, root.pos()));
        }
        let mut f = Self { name: text(root, 0, "footprint name")?, ..Self::default() };
        for item in root.args().iter().skip(1) {
            if !take_item(&mut f, item)? {
                f.unknown.push(Unknown::new("footprint", item));
            }
        }
        Ok(f)
    }
}

/// Parses a `.kicad_mod` file.
pub fn parse_footprint(input: &[u8]) -> Result<Footprint> {
    let root = sexpr::parse(input)?;
    Footprint::from_sexpr(&root)
}

fn take_item(f: &mut Footprint, item: &Sexpr) -> Result<bool> {
    let is_list = item.as_list().is_some();
    match item.name() {
        "locked" => f.locked = flag(item)?,
        "placed" => f.placed = flag(item)?,
        _ if !is_list => return Ok(false),
        "version" => f.version = Some(int(item, 0, "version")?),
        "generator" => f.generator = Some(text1(item)?),
        "generator_version" => f.generator_version = Some(text1(item)?),
        "layer" => f.layer = Some(text1(item)?),
        "tedit" => f.tedit = Some(text1(item)?),
        "uuid" | "tstamp" => f.uuid = Some(text1(item)?),
        "at" => f.at = Some(At::decode(item)?),
        "descr" => f.descr = Some(text1(item)?),
        "tags" => f.tags = Some(text1(item)?),
        "property" => f.properties.push(FpProperty::decode(item)?),
        "path" => f.path = Some(text1(item)?),
        "autoplace_cost90" => f.autoplace_cost90 = Some(int(item, 0, "autoplace_cost90")?),
        "autoplace_cost180" => f.autoplace_cost180 = Some(int(item, 0, "autoplace_cost180")?),
        "attr" => f.attr = Some(Attr::decode(item)?),
        "private_layers" => f.private_layers = texts(item, "private layer")?,
        "net_tie_pad_groups" => f.net_tie_pad_groups = texts(item, "net-tie pad group")?,
        "pad" => f.pads.push(Pad::decode(item)?),
        "group" => f.groups.push(Group::decode(item)?),
        "zone" => f.zones.push(Zone::decode(item)?),
        "model" => f.models.push(Model::decode(item)?),
        "embedded_fonts" => f.embedded_fonts = Some(yes_no(item)?),
        "fp_text" => f.graphics.push(FpGraphic::Text(FpText::decode(item)?)),
        "fp_text_box" => f.graphics.push(FpGraphic::TextBox(FpTextBox::decode(item)?)),
        name => {
            if f.overrides.take(item)? {
                return Ok(true);
            }
            match FpShape::decode(item)? {
                Some(s) if name.starts_with("fp_") => f.graphics.push(FpGraphic::Shape(s)),
                _ => return Ok(false),
            }
        }
    }
    Ok(true)
}

impl Inspect for FpGraphic {
    fn visit(&self, r: &mut Report) {
        match self {
            Self::Shape(s) => s.visit(r),
            Self::Text(t) => t.visit(r),
            Self::TextBox(t) => t.visit(r),
        }
    }
}

impl Inspect for Footprint {
    fn visit(&self, r: &mut Report) {
        self.properties.visit(r);
        self.overrides.unknown.visit(r);
        self.attr.visit(r);
        self.graphics.visit(r);
        self.pads.visit(r);
        self.groups.visit(r);
        self.zones.visit(r);
        self.models.visit(r);
        self.unknown.visit(r);
    }
}

#[cfg(test)]
mod tests;
