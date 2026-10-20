//! Geometry helpers: arcs through three points, flattening of arcs and curves into polylines,
//! readable text directions and estimated text boxes.

use std::f64::consts::PI;

pub use protasena_geom::{ArcGeom, arc_through};
use protasena_geom::{Pt, Rect, Vec2};

use crate::prim::{HAlign, VAlign};

/// Width of one character relative to the text height (an estimate for bounds and alignment).
pub const CHAR_WIDTH: f64 = 0.6;

/// Most segments one flattened arc or curve gets.
const MAX_SEGMENTS: usize = 256;

/// Segments for an arc of `radius` sweeping `sweep` radians, so each deviates at most
/// `tolerance` from the true arc.
pub fn segments_for(radius: f64, sweep: f64, tolerance: f64) -> usize {
    let r = radius.abs();
    if !(r.is_finite() && sweep.is_finite()) || r <= tolerance {
        return 1;
    }
    let step = 2.0 * (1.0 - (tolerance / r).min(1.0)).acos();
    let n = (sweep.abs() / step.max(1e-6)).ceil();
    if n.is_finite() { (n as usize).clamp(1, MAX_SEGMENTS) } else { MAX_SEGMENTS }
}

/// The arc as a polyline (deviation at most `tolerance`).
pub fn flatten_arc(center: Pt, radius: f64, start: f64, sweep: f64, tolerance: f64) -> Vec<Pt> {
    let n = segments_for(radius, sweep, tolerance);
    (0..=n)
        .map(|i| {
            let a = start + sweep * (i as f64 / n as f64);
            Pt::new(center.x + radius * a.cos(), center.y + radius * a.sin())
        })
        .collect()
}

/// A cubic Bézier as a polyline of `n` segments (at least 1).
pub fn flatten_bezier(p: [Pt; 4], n: usize) -> Vec<Pt> {
    let n = n.clamp(1, MAX_SEGMENTS);
    (0..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            Pt::new(a * p[0].x + b * p[1].x + c * p[2].x + d * p[3].x, a * p[0].y + b * p[1].y + c * p[2].y + d * p[3].y)
        })
        .collect()
}

/// A text direction that never reads upside down: angles in (90°, 270°] are turned by 180°.
/// Returns the angle in [0, 360) and whether it was turned (the alignment flips with it).
pub fn readable(angle: f64) -> (f64, bool) {
    let a = if angle.is_finite() { angle.rem_euclid(360.0) } else { 0.0 };
    if a > 90.0 + 1e-6 && a <= 270.0 + 1e-6 { ((a - 180.0).rem_euclid(360.0), true) } else { (a, false) }
}

/// The opposite horizontal alignment (for a text turned by 180°).
pub fn flip_h(h: HAlign) -> HAlign {
    match h {
        HAlign::Left => HAlign::Right,
        HAlign::Center => HAlign::Center,
        HAlign::Right => HAlign::Left,
    }
}

/// The opposite vertical alignment (for a text turned by 180°).
pub fn flip_v(v: VAlign) -> VAlign {
    match v {
        VAlign::Bottom => VAlign::Top,
        VAlign::Middle => VAlign::Middle,
        VAlign::Top => VAlign::Bottom,
    }
}

/// The estimated box of a text run.
pub fn text_box(at: Pt, text: &str, size: f64, angle: f64, h: HAlign, v: VAlign) -> Rect {
    let longest = text.lines().map(|l| l.chars().count()).max().unwrap_or(0).min(4096) as f64;
    let lines = text.lines().count().clamp(1, 4096) as f64;
    let (w, ht) = (longest * size * CHAR_WIDTH, lines * size);
    let x0 = match h {
        HAlign::Left => 0.0,
        HAlign::Center => -w / 2.0,
        HAlign::Right => -w,
    };
    let y0 = match v {
        VAlign::Bottom => 0.0,
        VAlign::Middle => -ht / 2.0,
        VAlign::Top => -ht,
    };
    let (sin, cos) = (angle * PI / 180.0).sin_cos();
    let turn = |x: f64, y: f64| at + Vec2::new(x * cos - y * sin, x * sin + y * cos);
    let corners = [turn(x0, y0), turn(x0 + w, y0), turn(x0, y0 + ht), turn(x0 + w, y0 + ht)];
    Rect::from_points(corners).unwrap_or_else(|| Rect::from_point(at))
}

/// Whether a closed outline is convex (collinear points allowed).
pub fn is_convex(points: &[Pt]) -> bool {
    let n = points.len();
    if n < 4 {
        return true;
    }
    let mut sign = 0.0_f64;
    for i in 0..n {
        let (Some(a), Some(b), Some(c)) = (points.get(i), points.get((i + 1) % n), points.get((i + 2) % n)) else { return false };
        let cross = (*b - *a).cross(*c - *b);
        if cross.abs() > 1e-9 {
            if sign != 0.0 && cross.signum() != sign {
                return false;
            }
            sign = cross.signum();
        }
    }
    true
}

/// Triangles covering a simple closed outline (ear clipping), as index triples into `points`.
/// Outlines that are not simple get as many triangles as could be cut; at most 4096 points.
pub fn triangulate(points: &[Pt]) -> Vec<[usize; 3]> {
    let n = points.len();
    if !(3..=4096).contains(&n) || !points.iter().all(|p| p.is_finite()) {
        return Vec::new();
    }
    let area: f64 = (0..n).filter_map(|i| Some(points.get(i)?.to_vec().cross(points.get((i + 1) % n)?.to_vec()))).sum();
    let ccw = area > 0.0;
    let mut idx: Vec<usize> = (0..n).collect();
    let mut out = Vec::with_capacity(n.saturating_sub(2));
    let at = |i: usize| points.get(i).copied().unwrap_or(Pt::ORIGIN);
    let mut guard = 0usize;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let m = idx.len();
        let ear = (0..m).find(|&k| {
            let get = |o: usize| idx.get((k + o) % m).copied().unwrap_or(0);
            let (ia, ib, ic) = (get(0), get(1), get(2));
            let (a, b, c) = (at(ia), at(ib), at(ic));
            let cross = (b - a).cross(c - b);
            let convex = if ccw { cross > 1e-12 } else { cross < -1e-12 };
            convex && !idx.iter().any(|&j| j != ia && j != ib && j != ic && inside(at(j), a, b, c))
        });
        let Some(k) = ear else { break };
        let get = |o: usize| idx.get((k + o) % m).copied().unwrap_or(0);
        out.push([get(0), get(1), get(2)]);
        idx.remove((k + 1) % m);
    }
    if let [a, b, c] = idx.as_slice() {
        out.push([*a, *b, *c]);
    }
    out
}

fn inside(p: Pt, a: Pt, b: Pt, c: Pt) -> bool {
    let d1 = (b - a).cross(p - a);
    let d2 = (c - b).cross(p - b);
    let d3 = (a - c).cross(p - c);
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn flattening_ends_on_the_curve_ends() {
        let pts = flatten_arc(Pt::ORIGIN, 100.0, 0.0, PI / 2.0, 0.5);
        let (first, last) = (pts.first().unwrap(), pts.last().unwrap());
        assert!(close(first.x, 100.0) && close(last.y, 100.0) && pts.len() > 4);
        let b = flatten_bezier([Pt::new(0.0, 0.0), Pt::new(0.0, 100.0), Pt::new(100.0, 100.0), Pt::new(100.0, 0.0)], 8);
        assert_eq!((b.len(), b[0], b[8]), (9, Pt::new(0.0, 0.0), Pt::new(100.0, 0.0)));
        assert_eq!(segments_for(f64::INFINITY, 1.0, 0.5), 1);
        assert_eq!(segments_for(1e12, 1e12, 1e-12), MAX_SEGMENTS);
    }

    #[test]
    fn concave_outlines_are_cut_into_triangles() {
        let l = [Pt::new(0.0, 0.0), Pt::new(2.0, 0.0), Pt::new(2.0, 1.0), Pt::new(1.0, 1.0), Pt::new(1.0, 2.0), Pt::new(0.0, 2.0)];
        assert!(!is_convex(&l));
        let tris = triangulate(&l);
        assert_eq!(tris.len(), 4);
        let area: f64 = tris.iter().map(|t| ((l[t[1]] - l[t[0]]).cross(l[t[2]] - l[t[0]]) / 2.0).abs()).sum();
        assert!(close(area, 3.0), "{area}");
        let mut cw = l;
        cw.reverse();
        assert_eq!(triangulate(&cw).len(), 4);
        assert!(is_convex(&[Pt::new(0.0, 0.0), Pt::new(1.0, 0.0), Pt::new(1.0, 1.0), Pt::new(0.0, 1.0)]));
        assert!(triangulate(&[Pt::new(f64::NAN, 0.0), Pt::ORIGIN, Pt::new(1.0, 1.0)]).is_empty());
    }

    #[test]
    fn texts_never_read_upside_down() {
        assert_eq!(readable(0.0), (0.0, false));
        assert_eq!(readable(90.0), (90.0, false));
        assert_eq!(readable(180.0), (0.0, true));
        assert_eq!(readable(270.0), (90.0, true));
        assert_eq!(readable(-90.0), (90.0, true));
        assert_eq!(readable(f64::NAN), (0.0, false));
    }

    #[test]
    fn text_boxes_follow_alignment_and_angle() {
        let b = text_box(Pt::ORIGIN, "AB", 100.0, 0.0, HAlign::Center, VAlign::Bottom);
        assert!(close(b.min.x, -60.0) && close(b.max.x, 60.0) && close(b.min.y, 0.0) && close(b.max.y, 100.0));
        let v = text_box(Pt::ORIGIN, "AB", 100.0, 90.0, HAlign::Left, VAlign::Bottom);
        assert!(close(v.min.y, 0.0) && close(v.max.y, 120.0) && close(v.min.x, -100.0) && close(v.max.x, 0.0), "{v:?}");
    }
}
