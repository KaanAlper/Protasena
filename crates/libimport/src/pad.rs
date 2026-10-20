//! KiCad pads → Protasena pads.
//!
//! - Positions are mirrored in y like all footprint points; the pad angle keeps its value
//!   (counter-clockwise on the board both ways, see [`crate::coords`]).
//! - Pad names must be unique in a package. KiCad lets several pads share a number (thermal
//!   vias, a pin split over several pads, unnamed mounting holes): the second and later get
//!   `N#2`, `N#3`, … and still belong to pin `N`.
//! - Shapes without a Protasena counterpart: trapezoids become rectangles; custom pads become
//!   their anchor pad plus the custom outline drawn on the pad's copper layers.
//! - A slotted (oval) drill keeps its smaller size as the drill diameter.

use std::collections::HashMap;

use protasena_geom::Pt;
use protasena_kicad::common::Pt as KPt;
use protasena_kicad::footprint::{Pad as KPad, PadShape as KShape, PadType};
use protasena_library::{Fill, LayerGraphic, Pad, PadShape};

use crate::report::Notes;
use crate::{coords, footprint, layers};

/// KiCad's corner ratio when a round-rect pad does not give one.
const DEFAULT_ROUND_RATIO: f64 = 0.25;

/// Every pad of a footprint, and the drawing of custom pad outlines.
pub fn convert_all(pads: &[KPad], notes: &mut Notes) -> (Vec<Pad>, Vec<LayerGraphic>) {
    let mut seen: HashMap<&str, u32> = HashMap::new();
    let mut out = Vec::with_capacity(pads.len());
    let mut graphics = Vec::new();
    for p in pads {
        let n = seen.entry(p.number.as_str()).or_insert(0);
        *n += 1;
        let name = if *n == 1 { p.number.clone() } else { format!("{}#{n}", p.number) };
        if *n > 1 {
            notes.note("pad number repeated (renamed N#k)");
        }
        let pad = convert(name, p, notes);
        graphics.extend(custom_outline(p, &pad, notes));
        out.push(pad);
    }
    (out, graphics)
}

/// One pad.
pub fn convert(name: String, p: &KPad, notes: &mut Notes) -> Pad {
    let drill = p.drill.as_ref().and_then(|d| {
        if d.oval {
            notes.note("oval drills (smaller size kept)");
        }
        let sizes = d.diameter.into_iter().chain(d.width.filter(|_| d.oval));
        sizes.reduce(f64::min).map(coords::len)
    });
    let drill = match p.kind {
        PadType::Smd | PadType::Connect => None,
        _ => drill,
    };
    Pad {
        name,
        at: coords::fp(p.at.pt()),
        angle: coords::round(p.at.angle),
        shape: shape(p, notes),
        size: coords::size(p.size),
        drill,
        layers: p.layers.iter().map(|l| layers::map(l)).collect(),
    }
}

fn shape(p: &KPad, notes: &mut Notes) -> PadShape {
    match &p.shape {
        KShape::Circle => PadShape::Circle,
        KShape::Rect => PadShape::Rect,
        KShape::Oval => PadShape::Oval,
        KShape::RoundRect => {
            if !p.chamfer.is_empty() {
                notes.note("chamfered pad corners (drawn rounded)");
            }
            let ratio = p.roundrect_rratio.unwrap_or(DEFAULT_ROUND_RATIO);
            PadShape::RoundRect { ratio: if ratio.is_finite() { ratio.clamp(0.0, 0.5) } else { DEFAULT_ROUND_RATIO } }
        }
        KShape::Trapezoid => {
            notes.note("trapezoid pads (drawn as rectangles)");
            PadShape::Rect
        }
        KShape::Custom => {
            notes.note("custom pads (anchor + outline on copper)");
            match p.options.as_ref().and_then(|o| o.anchor.as_deref()) {
                Some("circle") => PadShape::Circle,
                _ => PadShape::Rect,
            }
        }
        KShape::Other(_) => {
            notes.note("unknown pad shapes (drawn as rectangles)");
            PadShape::Rect
        }
    }
}

/// The primitives of a custom pad, placed (pad rotation and position applied) on each of the
/// pad's copper layers.
fn custom_outline(p: &KPad, pad: &Pad, notes: &mut Notes) -> Vec<LayerGraphic> {
    let Some(prims) = p.primitives.as_ref().filter(|_| p.shape == KShape::Custom) else { return Vec::new() };
    let (centre, angle) = (pad.at, pad.angle);
    let place = |q: KPt| coords::rotate(Pt::new(centre.x + coords::len(q.x), centre.y + coords::len(-q.y)), centre, angle);
    let copper: Vec<&String> = pad.layers.iter().filter(|l| layers::is_copper(l)).collect();
    let mut out = Vec::new();
    for item in &prims.items {
        let Some(mut g) = footprint::shape(item, place, true, notes) else { continue };
        if g.width == 0.0 {
            g.width = prims.width.map_or(0.0, coords::len);
        }
        if item.fill.or(prims.fill).unwrap_or(true) {
            g.fill = Fill::Solid;
        }
        out.extend(copper.iter().map(|l| LayerGraphic { layer: (*l).clone(), graphic: g.clone() }));
    }
    out
}
