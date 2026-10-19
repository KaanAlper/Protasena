//! Footprint attributes, clearance overrides, 3D model references and groups.

use crate::decode::{flag, int, num, num1, text, text1, texts};
use crate::error::Result;
use crate::report::{Inspect, Report, Unknown, keyword_enum};
use crate::sexpr::Sexpr;

keyword_enum!(
    /// Footprint type in `(attr …)`.
    AttrType, "attr.type" { Smd = "smd", ThroughHole = "through_hole" }
);

/// `(attr TYPE [board_only] [exclude_from_pos_files] [exclude_from_bom] …)`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Attr {
    pub kind: Option<AttrType>,
    pub board_only: bool,
    pub exclude_from_pos_files: bool,
    pub exclude_from_bom: bool,
    pub allow_missing_courtyard: bool,
    pub allow_soldermask_bridges: bool,
    pub dnp: bool,
    pub unknown: Vec<Unknown>,
}

impl Attr {
    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        let mut a = Self::default();
        for item in node.args() {
            match item.as_atom() {
                Some("smd") => a.kind = Some(AttrType::Smd),
                Some("through_hole") => a.kind = Some(AttrType::ThroughHole),
                Some("board_only") => a.board_only = true,
                Some("exclude_from_pos_files") => a.exclude_from_pos_files = true,
                Some("exclude_from_bom") => a.exclude_from_bom = true,
                Some("allow_missing_courtyard") => a.allow_missing_courtyard = true,
                Some("allow_soldermask_bridges") => a.allow_soldermask_bridges = true,
                Some("dnp") => a.dnp = true,
                _ => a.unknown.push(Unknown::new("attr", item)),
            }
        }
        Ok(a)
    }
}

impl Inspect for Attr {
    fn visit(&self, r: &mut Report) {
        self.kind.visit(r);
        self.unknown.visit(r);
    }
}

/// Clearance, mask, paste and zone-connection overrides of a footprint or a pad.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Overrides {
    pub solder_mask_margin: Option<f64>,
    pub solder_paste_margin: Option<f64>,
    /// `solder_paste_ratio` (footprint) or `solder_paste_margin_ratio` (pad).
    pub solder_paste_ratio: Option<f64>,
    pub clearance: Option<f64>,
    /// 0 none, 1 thermal relief, 2 solid, 3 thermal for through-hole pads only.
    pub zone_connect: Option<i64>,
    /// `thermal_width` (KiCad 6) or `thermal_bridge_width` (KiCad 7+).
    pub thermal_width: Option<f64>,
    pub thermal_gap: Option<f64>,
    /// `thermal_bridge_angle` in degrees (KiCad 7+).
    pub thermal_bridge_angle: Option<f64>,
    pub unknown: Vec<Unknown>,
}

impl Overrides {
    /// Takes `item` when it is one of the override tokens.
    pub(crate) fn take(&mut self, item: &Sexpr) -> Result<bool> {
        let slot = match item.name() {
            "solder_mask_margin" => &mut self.solder_mask_margin,
            "solder_paste_margin" => &mut self.solder_paste_margin,
            "solder_paste_ratio" | "solder_paste_margin_ratio" => &mut self.solder_paste_ratio,
            "clearance" => &mut self.clearance,
            "thermal_width" | "thermal_bridge_width" => &mut self.thermal_width,
            "thermal_gap" => &mut self.thermal_gap,
            "thermal_bridge_angle" => &mut self.thermal_bridge_angle,
            "zone_connect" => {
                self.zone_connect = Some(int(item, 0, "zone_connect")?);
                return Ok(true);
            }
            _ => return Ok(false),
        };
        *slot = Some(num1(item)?);
        Ok(true)
    }
}

/// `(xyz X Y Z)`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Xyz {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Xyz {
    /// The `(xyz …)` inside `node`, e.g. `(offset (xyz 0 0 0))`.
    fn decode_in(node: &Sexpr) -> Result<Option<Self>> {
        let Some(xyz) = node.child("xyz") else { return Ok(None) };
        Ok(Some(Self { x: num(xyz, 0, "x")?, y: num(xyz, 1, "y")?, z: num(xyz, 2, "z")? }))
    }
}

/// `(model "PATH" (offset (xyz …)) (scale (xyz …)) (rotate (xyz …)))`. Only the reference
/// is read; the model file is never opened.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Model {
    pub path: String,
    /// `(offset …)`, or `(at …)` in files before KiCad 6.
    pub offset: Option<Xyz>,
    pub scale: Option<Xyz>,
    pub rotate: Option<Xyz>,
    pub hide: bool,
    pub opacity: Option<f64>,
    pub unknown: Vec<Unknown>,
}

impl Model {
    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        let mut m = Self { path: text(node, 0, "model path")?, ..Self::default() };
        for item in node.args().iter().skip(1) {
            let is_xyz_holder = item.as_list().is_some_and(|l| l.len() == 2) && item.child("xyz").is_some();
            match item.name() {
                "offset" | "at" if is_xyz_holder => m.offset = Xyz::decode_in(item)?,
                "scale" if is_xyz_holder => m.scale = Xyz::decode_in(item)?,
                "rotate" if is_xyz_holder => m.rotate = Xyz::decode_in(item)?,
                "hide" => m.hide = flag(item)?,
                "opacity" => m.opacity = Some(num1(item)?),
                _ => m.unknown.push(Unknown::new("model", item)),
            }
        }
        Ok(m)
    }
}

impl Inspect for Model {
    fn visit(&self, r: &mut Report) {
        self.unknown.visit(r);
    }
}

/// `(group "NAME" (id UUID) (members UUID …))`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Group {
    pub name: String,
    pub id: Option<String>,
    pub members: Vec<String>,
    pub locked: bool,
    pub unknown: Vec<Unknown>,
}

impl Group {
    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        let mut g = Self::default();
        for (i, item) in node.args().iter().enumerate() {
            match item.name() {
                _ if i == 0 && item.as_list().is_none() && item.as_atom() != Some("locked") => g.name = text(node, 0, "group name")?,
                "id" | "uuid" => g.id = Some(text1(item)?),
                "members" => g.members = texts(item, "group member")?,
                "locked" => g.locked = flag(item)?,
                _ => g.unknown.push(Unknown::new("group", item)),
            }
        }
        Ok(g)
    }
}

impl Inspect for Group {
    fn visit(&self, r: &mut Report) {
        self.unknown.visit(r);
    }
}
