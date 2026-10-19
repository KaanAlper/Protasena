//! Footprint graphic shapes (`fp_*`, and the `gr_*`/`bezier` primitives of custom pads) and
//! text boxes.

use crate::common::{Effects, Pt, Stroke, decode_pts};
use crate::decode::{bool_word, flag, num, num1, pt, put, required, string, text1};
use crate::error::Result;
use crate::report::{Inspect, Report, Unknown};
use crate::sexpr::Sexpr;

/// The geometry of a footprint shape.
#[derive(Debug, Clone, PartialEq)]
pub enum FpShapeKind {
    Line {
        start: Pt,
        end: Pt,
    },
    Rect {
        start: Pt,
        end: Pt,
    },
    /// `end` is a point on the circle.
    Circle {
        center: Pt,
        end: Pt,
    },
    /// KiCad 6+: start, mid, end. Before KiCad 6: `start` is the centre, `end` the start
    /// point and `angle` the sweep in degrees (`mid` absent).
    Arc {
        start: Pt,
        mid: Option<Pt>,
        end: Pt,
        angle: Option<f64>,
    },
    Poly {
        pts: Vec<Pt>,
    },
    /// Cubic Bézier: four control points.
    Curve {
        pts: Vec<Pt>,
    },
    /// `gr_bbox`: where a custom pad shows its number and net name (KiCad 7+).
    BBox {
        start: Pt,
        end: Pt,
    },
}

/// A footprint shape with its layer and line style.
#[derive(Debug, Clone, PartialEq)]
pub struct FpShape {
    pub kind: FpShapeKind,
    pub layer: Option<String>,
    /// `(width W)` before KiCad 7; later files use `stroke`.
    pub width: Option<f64>,
    pub stroke: Option<Stroke>,
    /// `(fill yes|no)`; `solid`/`none` are read as yes/no.
    pub fill: Option<bool>,
    pub locked: bool,
    pub uuid: Option<String>,
    pub unknown: Vec<Unknown>,
}

/// (token, report context, geometry tokens).
const SHAPES: &[(&str, &str, &[&str])] = &[
    ("fp_line", "fp_line", &["start", "end"]),
    ("gr_line", "gr_line", &["start", "end"]),
    ("fp_rect", "fp_rect", &["start", "end"]),
    ("gr_rect", "gr_rect", &["start", "end"]),
    ("fp_circle", "fp_circle", &["center", "end"]),
    ("gr_circle", "gr_circle", &["center", "end"]),
    ("fp_arc", "fp_arc", &["start", "mid", "end", "angle"]),
    ("gr_arc", "gr_arc", &["start", "mid", "end", "angle"]),
    ("fp_poly", "fp_poly", &["pts"]),
    ("gr_poly", "gr_poly", &["pts"]),
    ("fp_curve", "fp_curve", &["pts"]),
    ("gr_curve", "gr_curve", &["pts"]),
    ("bezier", "bezier", &["pts"]),
    ("gr_bbox", "gr_bbox", &["start", "end"]),
];

#[derive(Default)]
struct Geometry {
    start: Option<Pt>,
    mid: Option<Pt>,
    end: Option<Pt>,
    center: Option<Pt>,
    angle: Option<f64>,
    pts: Option<Vec<Pt>>,
}

/// `(fill yes|no|solid|none)`.
fn decode_fill(item: &Sexpr) -> Option<bool> {
    match item.args() {
        [v] => bool_word(v).or_else(|| match v.as_atom()? {
            "solid" => Some(true),
            "none" => Some(false),
            _ => None,
        }),
        _ => None,
    }
}

impl FpShape {
    /// `Ok(None)` when `node` is not a shape token.
    pub(crate) fn decode(node: &Sexpr) -> Result<Option<Self>> {
        let Some(head) = node.head() else { return Ok(None) };
        let Some(&(token, context, geometry)) = SHAPES.iter().find(|(t, _, _)| *t == head) else { return Ok(None) };
        let mut s = Self {
            kind: FpShapeKind::Poly { pts: Vec::new() },
            layer: None,
            width: None,
            stroke: None,
            fill: None,
            locked: false,
            uuid: None,
            unknown: Vec::new(),
        };
        let mut g = Geometry::default();
        for item in node.args() {
            let name = item.name();
            let is_list = item.as_list().is_some();
            match name {
                "locked" => s.locked = flag(item)?,
                _ if !is_list => s.unknown.push(Unknown::new(context, item)),
                "layer" => put(&mut s.layer, text1(item)?, item, &mut s.unknown),
                "width" => put(&mut s.width, num1(item)?, item, &mut s.unknown),
                "stroke" => put(&mut s.stroke, Stroke::decode(item)?, item, &mut s.unknown),
                "uuid" | "tstamp" => put(&mut s.uuid, text1(item)?, item, &mut s.unknown),
                "fill" => match decode_fill(item) {
                    Some(f) => put(&mut s.fill, f, item, &mut s.unknown),
                    None => s.unknown.push(Unknown::new(context, item)),
                },
                _ if geometry.contains(&name) => read_geometry(&mut g, item, &mut s.unknown)?,
                _ => s.unknown.push(Unknown::new(context, item)),
            }
        }
        s.kind = match token.trim_start_matches("fp_").trim_start_matches("gr_") {
            "line" => FpShapeKind::Line { start: required(g.start, node, "start")?, end: required(g.end, node, "end")? },
            "rect" => FpShapeKind::Rect { start: required(g.start, node, "start")?, end: required(g.end, node, "end")? },
            "circle" => FpShapeKind::Circle { center: required(g.center, node, "center")?, end: required(g.end, node, "end")? },
            "arc" => FpShapeKind::Arc { start: required(g.start, node, "start")?, mid: g.mid, end: required(g.end, node, "end")?, angle: g.angle },
            "poly" => FpShapeKind::Poly { pts: required(g.pts, node, "pts")? },
            "bbox" => FpShapeKind::BBox { start: required(g.start, node, "start")?, end: required(g.end, node, "end")? },
            _ => FpShapeKind::Curve { pts: required(g.pts, node, "pts")? },
        };
        Ok(Some(s))
    }
}

fn read_geometry(g: &mut Geometry, item: &Sexpr, unknown: &mut Vec<Unknown>) -> Result<()> {
    match item.name() {
        "start" => put(&mut g.start, pt(item)?, item, unknown),
        "mid" => put(&mut g.mid, pt(item)?, item, unknown),
        "end" => put(&mut g.end, pt(item)?, item, unknown),
        "center" => put(&mut g.center, pt(item)?, item, unknown),
        "angle" => put(&mut g.angle, num1(item)?, item, unknown),
        _ => {
            let pts = decode_pts(item, unknown)?;
            put(&mut g.pts, pts, item, unknown);
        }
    }
    Ok(())
}

impl Inspect for FpShape {
    fn visit(&self, r: &mut Report) {
        self.stroke.visit(r);
        self.unknown.visit(r);
    }
}

/// `(fp_text_box [locked] "TEXT" (start …) (end …) | (pts …) [(angle A)] (layer …) …)`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FpTextBox {
    pub text: String,
    pub locked: bool,
    pub start: Option<Pt>,
    pub end: Option<Pt>,
    /// Four corners of a box that is not axis-aligned.
    pub pts: Option<Vec<Pt>>,
    pub angle: Option<f64>,
    pub layer: Option<String>,
    pub uuid: Option<String>,
    pub effects: Option<Effects>,
    pub stroke: Option<Stroke>,
    pub margins: Option<[f64; 4]>,
    pub border: Option<bool>,
    pub knockout: Option<bool>,
    pub unknown: Vec<Unknown>,
}

impl FpTextBox {
    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        let mut t = Self::default();
        let mut have_text = false;
        for item in node.args() {
            let u = &mut t.unknown;
            match item.name() {
                "locked" => t.locked = flag(item)?,
                _ if item.as_list().is_none() && !have_text => {
                    t.text = string(item, "text box text")?;
                    have_text = true;
                }
                _ if item.as_list().is_none() => u.push(Unknown::new("fp_text_box", item)),
                "start" => put(&mut t.start, pt(item)?, item, u),
                "end" => put(&mut t.end, pt(item)?, item, u),
                "pts" => {
                    let pts = decode_pts(item, u)?;
                    put(&mut t.pts, pts, item, u);
                }
                "angle" => put(&mut t.angle, num1(item)?, item, u),
                "layer" => put(&mut t.layer, text1(item)?, item, u),
                "uuid" | "tstamp" => put(&mut t.uuid, text1(item)?, item, u),
                "effects" => put(&mut t.effects, Effects::decode(item)?, item, u),
                "stroke" => put(&mut t.stroke, Stroke::decode(item)?, item, u),
                "margins" => {
                    let m = [num(item, 0, "margin")?, num(item, 1, "margin")?, num(item, 2, "margin")?, num(item, 3, "margin")?];
                    put(&mut t.margins, m, item, u);
                }
                "border" => put(&mut t.border, flag(item)?, item, u),
                "knockout" => put(&mut t.knockout, flag(item)?, item, u),
                _ => u.push(Unknown::new("fp_text_box", item)),
            }
        }
        if !have_text {
            return Err(crate::decode::invalid(node, "(fp_text_box) is missing its text"));
        }
        Ok(t)
    }
}

impl Inspect for FpTextBox {
    fn visit(&self, r: &mut Report) {
        self.effects.visit(r);
        self.stroke.visit(r);
        self.unknown.visit(r);
    }
}
