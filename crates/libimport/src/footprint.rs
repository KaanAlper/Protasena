//! KiCad footprints → Protasena packages: the drawing per layer (this file) and the pads
//! ([`crate::pad`]).
//!
//! KiCad footprint coordinates have y pointing down; every point goes through
//! [`coords::fp`], which mirrors y (see [`crate::coords`]).

use protasena_geom::Pt;
use protasena_kicad::Footprint;
use protasena_kicad::common::{Effects, Pt as KPt};
use protasena_kicad::footprint::{FpGraphic, FpShape, FpShapeKind, FpTextBox, TextAttrs};
use protasena_library::{Fill, Graphic, LayerGraphic, Package, Shape};

use crate::report::Notes;
use crate::{coords, layers, pad};

/// KiCad's default footprint text height, 1 mm.
const DEFAULT_TEXT_MM: f64 = 1.0;

/// Converts a footprint into a package called `name`.
pub fn convert(name: &str, fp: &Footprint, notes: &mut Notes) -> Package {
    let mut graphics = Vec::new();
    for g in &fp.graphics {
        match g {
            FpGraphic::Shape(s) => graphics.extend(shape_on_layer(s, notes)),
            FpGraphic::Text(t) => graphics.extend(text(&t.text, &t.attrs)),
            FpGraphic::TextBox(t) => graphics.extend(text_box(t, notes)),
        }
    }
    for p in &fp.properties {
        graphics.extend(text(&p.value, &p.attrs));
    }
    notes.add("footprint zones skipped", fp.zones.len() as u64);
    notes.add("footprint 3D model references skipped", fp.models.len() as u64);
    let (pads, pad_graphics) = pad::convert_all(&fp.pads, notes);
    graphics.extend(pad_graphics);
    Package { name: name.to_owned(), library: String::new(), description: fp.descr.clone().unwrap_or_default(), pads, graphics }
}

/// Unknown-token count of a footprint (for the report).
pub fn unknown_tokens(fp: &Footprint) -> std::collections::BTreeMap<String, usize> {
    fp.report().unknown_tokens
}

fn shape_on_layer(s: &FpShape, notes: &mut Notes) -> Option<LayerGraphic> {
    let layer = layers::map(s.layer.as_deref().unwrap_or("Dwgs.User"));
    let graphic = shape(s, coords::fp, false, notes)?;
    Some(LayerGraphic { layer, graphic })
}

/// A footprint shape with every point sent through `to` (footprint coordinates, or a custom
/// pad's local ones). `rotated`: rectangles become closed polylines (the rotation of a custom
/// pad may turn them).
pub fn shape(s: &FpShape, to: impl Fn(KPt) -> Pt, rotated: bool, notes: &mut Notes) -> Option<Graphic> {
    let width = s.stroke.as_ref().and_then(|st| st.width).or(s.width).map_or(0.0, coords::len);
    let fill = if s.fill == Some(true) { Fill::Solid } else { Fill::None };
    let shape = match &s.kind {
        FpShapeKind::Line { start, end } => Shape::line(to(*start), to(*end)),
        FpShapeKind::Rect { start, end } if rotated => {
            let corners = [*start, KPt { x: end.x, y: start.y }, *end, KPt { x: start.x, y: end.y }, *start];
            Shape::Polyline { points: corners.iter().map(|p| to(*p)).collect() }
        }
        FpShapeKind::Rect { start, end } => Shape::Rect { a: to(*start), b: to(*end) },
        FpShapeKind::Circle { center, end } => {
            let radius = coords::len(((end.x - center.x).powi(2) + (end.y - center.y).powi(2)).sqrt());
            Shape::Circle { center: to(*center), radius }
        }
        FpShapeKind::Arc { start, mid: Some(mid), end, .. } => Shape::Arc { start: to(*start), mid: to(*mid), end: to(*end) },
        FpShapeKind::Arc { start, mid: None, end, angle } => legacy_arc(*start, *end, angle.unwrap_or(0.0), &to, notes),
        FpShapeKind::Poly { pts } => {
            let mut points: Vec<Pt> = pts.iter().map(|p| to(*p)).collect();
            if let (Some(first), Some(last)) = (points.first().copied(), points.last().copied())
                && first != last
            {
                points.push(first);
            }
            Shape::Polyline { points }
        }
        FpShapeKind::Curve { pts } => Shape::Bezier { points: pts.iter().map(|p| to(*p)).collect() },
        FpShapeKind::BBox { .. } => {
            notes.note("custom pad text boxes (gr_bbox) skipped");
            return None;
        }
    };
    Some(Graphic { shape, width, fill })
}

/// A pre-KiCad 6 arc: centre, start point and sweep. The sweep turns the start point about the
/// centre in KiCad's own (y down) coordinates; the end and middle points are computed there and
/// then converted like every other point.
fn legacy_arc(centre: KPt, start: KPt, sweep: f64, to: &impl Fn(KPt) -> Pt, notes: &mut Notes) -> Shape {
    notes.note("legacy footprint arcs (centre + sweep)");
    let turn = |deg: f64| {
        let (s, c) = deg.to_radians().sin_cos();
        let (dx, dy) = (start.x - centre.x, start.y - centre.y);
        KPt { x: centre.x + dx * c - dy * s, y: centre.y + dx * s + dy * c }
    };
    Shape::Arc { start: to(start), mid: to(turn(sweep / 2.0)), end: to(turn(sweep)) }
}

/// A visible text on its layer.
fn text(value: &str, attrs: &TextAttrs) -> Option<LayerGraphic> {
    if attrs.is_hidden() {
        return None;
    }
    let (at, layer) = (attrs.at?, attrs.layer.as_deref()?);
    let shape =
        Shape::Text { at: coords::fp(at.pt()), text: value.to_owned(), size: text_size(attrs.effects.as_ref()), angle: coords::round(at.angle) };
    Some(LayerGraphic { layer: layers::map(layer), graphic: Graphic::new(shape) })
}

fn text_box(t: &FpTextBox, notes: &mut Notes) -> Vec<LayerGraphic> {
    let layer = layers::map(t.layer.as_deref().unwrap_or("Dwgs.User"));
    let corners: Vec<Pt> = match (&t.start, &t.end, &t.pts) {
        (Some(a), Some(b), _) => vec![coords::fp(*a), coords::fp(*b)],
        (_, _, Some(pts)) => pts.iter().map(|p| coords::fp(*p)).collect(),
        _ => Vec::new(),
    };
    if corners.is_empty() {
        notes.note("footprint text box without position skipped");
        return Vec::new();
    }
    let n = corners.len() as f64;
    let centre = Pt::new(coords::round(corners.iter().map(|p| p.x).sum::<f64>() / n), coords::round(corners.iter().map(|p| p.y).sum::<f64>() / n));
    let width = t.stroke.as_ref().and_then(|s| s.width).map_or(0.0, coords::len);
    let mut out = Vec::new();
    if t.border != Some(false) {
        let frame = match corners.as_slice() {
            [a, b] => Shape::Rect { a: *a, b: *b },
            pts => Shape::Polyline { points: pts.iter().copied().chain(pts.first().copied()).collect() },
        };
        out.push(LayerGraphic { layer: layer.clone(), graphic: Graphic { shape: frame, width, fill: Fill::None } });
    }
    let angle = coords::round(t.angle.unwrap_or(0.0));
    out.push(LayerGraphic {
        layer,
        graphic: Graphic::new(Shape::Text { at: centre, text: t.text.clone(), size: text_size(t.effects.as_ref()), angle }),
    });
    out
}

fn text_size(effects: Option<&Effects>) -> f64 {
    coords::len(effects.and_then(|e| e.font.height).unwrap_or(DEFAULT_TEXT_MM))
}
