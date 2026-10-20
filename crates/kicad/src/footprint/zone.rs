//! Zones in footprints: mostly keep-out areas (KiCad documentation: "Zone").

use crate::common::{Pt, decode_pts};
use crate::decode::{flag, int, num, num1, put, text, text1, texts};
use crate::error::Result;
use crate::report::{Inspect, Report, Unknown, keyword_enum};
use crate::sexpr::Sexpr;

keyword_enum!(
    /// One rule of a keep-out zone.
    KeepoutRule, "zone.keepout" { Allowed = "allowed", NotAllowed = "not_allowed" }
);

/// `(keepout (tracks …) (vias …) (pads …) (copperpour …) (footprints …))`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Keepout {
    pub tracks: Option<KeepoutRule>,
    pub vias: Option<KeepoutRule>,
    pub pads: Option<KeepoutRule>,
    pub copperpour: Option<KeepoutRule>,
    pub footprints: Option<KeepoutRule>,
    pub unknown: Vec<Unknown>,
}

/// `(connect_pads [TYPE] (clearance C))`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConnectPads {
    /// `thru_hole_only`, `full` or `no`; thermal reliefs when absent.
    pub kind: Option<String>,
    pub clearance: Option<f64>,
    pub unknown: Vec<Unknown>,
}

/// `(fill [yes] (mode …) (thermal_gap …) (thermal_bridge_width …) …)`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ZoneFill {
    pub filled: bool,
    /// `hatched`; solid when absent.
    pub mode: Option<String>,
    pub thermal_gap: Option<f64>,
    pub thermal_bridge_width: Option<f64>,
    pub smoothing: Option<String>,
    pub radius: Option<f64>,
    pub island_removal_mode: Option<i64>,
    pub island_area_min: Option<f64>,
    /// The `hatch_*` settings of a hatched fill, by token name.
    pub hatch: Vec<(String, f64)>,
    pub unknown: Vec<Unknown>,
}

/// `(filled_polygon (layer L) (pts …))`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FilledPolygon {
    pub layer: Option<String>,
    pub pts: Vec<Pt>,
    pub unknown: Vec<Unknown>,
}

/// `(zone …)`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Zone {
    pub net: Option<i64>,
    pub net_name: Option<String>,
    /// `(layer L)` or `(layers L …)`.
    pub layers: Vec<String>,
    pub uuid: Option<String>,
    pub name: Option<String>,
    /// `(hatch none|edge|full PITCH)`.
    pub hatch: Option<(String, f64)>,
    pub priority: Option<i64>,
    pub connect_pads: Option<ConnectPads>,
    pub min_thickness: Option<f64>,
    pub filled_areas_thickness: Option<bool>,
    pub keepout: Option<Keepout>,
    pub fill: Option<ZoneFill>,
    /// Outline polygons, `(polygon (pts …))`.
    pub polygons: Vec<Vec<Pt>>,
    pub filled_polygons: Vec<FilledPolygon>,
    pub unknown: Vec<Unknown>,
}

impl Zone {
    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        let mut z = Self::default();
        for item in node.args() {
            let u = &mut z.unknown;
            match item.name() {
                _ if item.as_list().is_none() => u.push(Unknown::new("zone", item)),
                "net" => put(&mut z.net, int(item, 0, "zone net")?, item, u),
                "net_name" => put(&mut z.net_name, text1(item)?, item, u),
                "layer" | "layers" => z.layers.extend(texts(item, "zone layer")?),
                "uuid" | "tstamp" => put(&mut z.uuid, text1(item)?, item, u),
                "name" => put(&mut z.name, text1(item)?, item, u),
                "hatch" => put(&mut z.hatch, (text(item, 0, "hatch style")?, num(item, 1, "hatch pitch")?), item, u),
                "priority" => put(&mut z.priority, int(item, 0, "zone priority")?, item, u),
                "connect_pads" => put(&mut z.connect_pads, decode_connect(item)?, item, u),
                "min_thickness" => put(&mut z.min_thickness, num1(item)?, item, u),
                "filled_areas_thickness" => put(&mut z.filled_areas_thickness, flag(item)?, item, u),
                "keepout" => put(&mut z.keepout, decode_keepout(item)?, item, u),
                "fill" => put(&mut z.fill, decode_fill(item)?, item, u),
                "polygon" => match item.child("pts") {
                    Some(pts) => {
                        z.polygons.push(decode_pts(pts, u)?);
                        u.extend(item.args().iter().filter(|a| a.head() != Some("pts")).map(|a| Unknown::new("polygon", a)));
                    }
                    None => u.push(Unknown::new("zone", item)),
                },
                "filled_polygon" => z.filled_polygons.push(decode_filled(item)?),
                _ => u.push(Unknown::new("zone", item)),
            }
        }
        Ok(z)
    }
}

fn decode_connect(node: &Sexpr) -> Result<ConnectPads> {
    let mut c = ConnectPads::default();
    for item in node.args() {
        match (item.as_atom(), item.head()) {
            (Some(kind), _) if c.kind.is_none() => c.kind = Some(kind.to_owned()),
            (_, Some("clearance")) => put(&mut c.clearance, num1(item)?, item, &mut c.unknown),
            _ => c.unknown.push(Unknown::new("connect_pads", item)),
        }
    }
    Ok(c)
}

fn decode_keepout(node: &Sexpr) -> Result<Keepout> {
    let mut k = Keepout::default();
    for item in node.args() {
        let slot = match item.head() {
            Some("tracks") => &mut k.tracks,
            Some("vias") => &mut k.vias,
            Some("pads") => &mut k.pads,
            Some("copperpour") => &mut k.copperpour,
            Some("footprints") => &mut k.footprints,
            _ => {
                k.unknown.push(Unknown::new("keepout", item));
                continue;
            }
        };
        let rule = KeepoutRule::from_keyword(&text1(item)?);
        put(slot, rule, item, &mut k.unknown);
    }
    Ok(k)
}

fn decode_fill(node: &Sexpr) -> Result<ZoneFill> {
    let mut f = ZoneFill::default();
    for item in node.args() {
        let u = &mut f.unknown;
        match item.name() {
            "yes" if item.as_atom().is_some() => f.filled = true,
            _ if item.as_list().is_none() => u.push(Unknown::new("zone.fill", item)),
            "mode" => put(&mut f.mode, text1(item)?, item, u),
            "thermal_gap" => put(&mut f.thermal_gap, num1(item)?, item, u),
            "thermal_bridge_width" => put(&mut f.thermal_bridge_width, num1(item)?, item, u),
            "smoothing" => put(&mut f.smoothing, text1(item)?, item, u),
            "radius" => put(&mut f.radius, num1(item)?, item, u),
            "island_removal_mode" => put(&mut f.island_removal_mode, int(item, 0, "island_removal_mode")?, item, u),
            "island_area_min" => put(&mut f.island_area_min, num1(item)?, item, u),
            name if name.starts_with("hatch_") => f.hatch.push((name.to_owned(), num1(item)?)),
            _ => u.push(Unknown::new("zone.fill", item)),
        }
    }
    Ok(f)
}

fn decode_filled(node: &Sexpr) -> Result<FilledPolygon> {
    let mut p = FilledPolygon::default();
    for item in node.args() {
        match item.head() {
            Some("layer") => put(&mut p.layer, text1(item)?, item, &mut p.unknown),
            Some("pts") => p.pts = decode_pts(item, &mut p.unknown)?,
            _ => p.unknown.push(Unknown::new("filled_polygon", item)),
        }
    }
    Ok(p)
}

impl Inspect for Zone {
    fn visit(&self, r: &mut Report) {
        if let Some(k) = &self.keepout {
            for rule in [&k.tracks, &k.vias, &k.pads, &k.copperpour, &k.footprints] {
                rule.visit(r);
            }
            k.unknown.visit(r);
        }
        self.connect_pads.as_ref().map(|c| &c.unknown).visit(r);
        self.fill.as_ref().map(|f| &f.unknown).visit(r);
        for p in &self.filled_polygons {
            p.unknown.visit(r);
        }
        self.unknown.visit(r);
    }
}
