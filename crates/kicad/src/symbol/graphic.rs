//! Symbol graphic items: arc, circle, rectangle, polyline, bezier, text, text box.

use crate::common::{At, Effects, Fill, Pt, Stroke, decode_pts};
use crate::decode::{num, num1, pt, put, required, string};
use crate::error::Result;
use crate::report::{Inspect, Report, Unknown};
use crate::sexpr::Sexpr;

/// Where a text box sits: KiCad writes `(at X Y A) (size W H)`; `start`/`end` are accepted
/// too (the footprint text box form).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextBoxGeometry {
    pub at: Option<At>,
    pub size: Option<Pt>,
    pub start: Option<Pt>,
    pub end: Option<Pt>,
}

/// The shape of a graphic item.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphicKind {
    /// `mid` is the point halfway along the arc.
    Arc {
        start: Pt,
        mid: Option<Pt>,
        end: Pt,
    },
    Circle {
        center: Pt,
        radius: f64,
    },
    Rectangle {
        start: Pt,
        end: Pt,
    },
    /// One or more connected lines (a polygon when closed).
    Polyline {
        pts: Vec<Pt>,
    },
    /// Cubic Bézier: four control points.
    Bezier {
        pts: Vec<Pt>,
    },
    /// Note: symbol text angles are in tenths of a degree (KiCad documentation).
    Text {
        text: String,
        at: At,
        effects: Option<Effects>,
    },
    TextBox {
        text: String,
        geometry: TextBoxGeometry,
        effects: Option<Effects>,
        margins: Option<[f64; 4]>,
    },
}

/// A graphic item of a symbol unit.
#[derive(Debug, Clone, PartialEq)]
pub struct Graphic {
    pub kind: GraphicKind,
    pub stroke: Option<Stroke>,
    pub fill: Option<Fill>,
    /// `private`: drawn only in the symbol editor (KiCad 7+).
    pub private: bool,
    pub unknown: Vec<Unknown>,
}

/// Fields read from a graphic item before its kind is assembled.
#[derive(Default)]
struct Bag {
    text: Option<String>,
    start: Option<Pt>,
    mid: Option<Pt>,
    end: Option<Pt>,
    center: Option<Pt>,
    radius: Option<f64>,
    pts: Option<Vec<Pt>>,
    at: Option<At>,
    size: Option<Pt>,
    margins: Option<[f64; 4]>,
    effects: Option<Effects>,
}

fn keys(kind: &str) -> Option<&'static [&'static str]> {
    Some(match kind {
        "arc" => &["start", "mid", "end"],
        "circle" => &["center", "radius"],
        "rectangle" => &["start", "end"],
        "polyline" | "bezier" => &["pts"],
        "text" => &["at", "effects"],
        "text_box" => &["at", "size", "start", "end", "margins", "effects"],
        _ => return None,
    })
}

impl Graphic {
    /// `Ok(None)` when `node` is not a symbol graphic item.
    pub(crate) fn decode(node: &Sexpr) -> Result<Option<Self>> {
        let Some(kind) = node.head() else { return Ok(None) };
        let Some(allowed) = keys(kind) else { return Ok(None) };
        let mut g = Self { kind: GraphicKind::Polyline { pts: Vec::new() }, stroke: None, fill: None, private: false, unknown: Vec::new() };
        let mut bag = Bag::default();
        let has_text = matches!(kind, "text" | "text_box");
        for (i, item) in node.args().iter().enumerate() {
            let name = item.name();
            if i == 0 && has_text && item.as_list().is_none() {
                bag.text = Some(string(item, kind)?);
                continue;
            }
            match name {
                "stroke" if item.as_list().is_some() => put(&mut g.stroke, Stroke::decode(item)?, item, &mut g.unknown),
                "fill" if item.as_list().is_some() => put(&mut g.fill, Fill::decode(item)?, item, &mut g.unknown),
                "private" if item.as_atom().is_some() => g.private = true,
                _ if item.as_list().is_some() && allowed.contains(&name) => read_field(&mut bag, item, &mut g.unknown)?,
                _ => g.unknown.push(Unknown::new(context(kind), item)),
            }
        }
        g.kind = assemble(kind, bag, node)?;
        Ok(Some(g))
    }
}

fn context(kind: &str) -> &'static str {
    match kind {
        "arc" => "arc",
        "circle" => "circle",
        "rectangle" => "rectangle",
        "polyline" => "polyline",
        "bezier" => "bezier",
        "text" => "text",
        _ => "text_box",
    }
}

fn read_field(bag: &mut Bag, item: &Sexpr, unknown: &mut Vec<Unknown>) -> Result<()> {
    match item.name() {
        "start" => put(&mut bag.start, pt(item)?, item, unknown),
        "mid" => put(&mut bag.mid, pt(item)?, item, unknown),
        "end" => put(&mut bag.end, pt(item)?, item, unknown),
        "center" => put(&mut bag.center, pt(item)?, item, unknown),
        "radius" => put(&mut bag.radius, num1(item)?, item, unknown),
        "pts" => {
            let pts = decode_pts(item, unknown)?;
            put(&mut bag.pts, pts, item, unknown);
        }
        "at" => put(&mut bag.at, At::decode(item)?, item, unknown),
        "size" => put(&mut bag.size, pt(item)?, item, unknown),
        "effects" => put(&mut bag.effects, Effects::decode(item)?, item, unknown),
        "margins" => {
            let m = [num(item, 0, "margin")?, num(item, 1, "margin")?, num(item, 2, "margin")?, num(item, 3, "margin")?];
            put(&mut bag.margins, m, item, unknown);
        }
        _ => unknown.push(Unknown::new("graphic", item)),
    }
    Ok(())
}

fn assemble(kind: &str, b: Bag, node: &Sexpr) -> Result<GraphicKind> {
    Ok(match kind {
        "arc" => GraphicKind::Arc { start: required(b.start, node, "start")?, mid: b.mid, end: required(b.end, node, "end")? },
        "circle" => GraphicKind::Circle { center: required(b.center, node, "center")?, radius: required(b.radius, node, "radius")? },
        "rectangle" => GraphicKind::Rectangle { start: required(b.start, node, "start")?, end: required(b.end, node, "end")? },
        "polyline" => GraphicKind::Polyline { pts: required(b.pts, node, "pts")? },
        "bezier" => GraphicKind::Bezier { pts: required(b.pts, node, "pts")? },
        "text" => GraphicKind::Text { text: required(b.text, node, "text")?, at: required(b.at, node, "at")?, effects: b.effects },
        _ => GraphicKind::TextBox {
            text: required(b.text, node, "text")?,
            geometry: TextBoxGeometry { at: b.at, size: b.size, start: b.start, end: b.end },
            effects: b.effects,
            margins: b.margins,
        },
    })
}

impl Inspect for Graphic {
    fn visit(&self, r: &mut Report) {
        if let GraphicKind::Text { effects, .. } | GraphicKind::TextBox { effects, .. } = &self.kind {
            effects.visit(r);
        }
        self.stroke.visit(r);
        self.fill.visit(r);
        self.unknown.visit(r);
    }
}
