//! The grid on the sheet: dots or lines at the snap step, coarsened (×5, ×2, …) until the
//! points are far enough apart to read.

use egui::{Color32, Mesh, Painter, Rect, Stroke, pos2, vec2};
use protasena_geom::Rect as SheetRect;

use super::Xform;
use crate::GridStyle;
use crate::theme::Tokens;

/// Closest two grid points may be drawn, in points.
const MIN_GAP: f64 = 12.0;
/// Most dots or lines drawn in one frame.
const MAX_MARKS: f64 = 40_000.0;

/// The grid spacing in thou for snap `step` at `zoom` (points per thou).
pub fn spacing(step: f64, zoom: f64) -> f64 {
    let mut s = if step.is_finite() && step > 0.0 { step } else { 100.0 };
    let mut five = true;
    while s * zoom < MIN_GAP && s < 1.0e9 {
        s *= if five { 5.0 } else { 2.0 };
        five = !five;
    }
    s
}

/// Draws the grid over the visible part of `sheet`.
pub fn draw(painter: &Painter, x: &Xform, sheet: SheetRect, step: f64, style: GridStyle, t: &Tokens) {
    let Some(area) = x.visible().intersection(sheet) else { return };
    let s = spacing(step, x.zoom);
    let (nx, ny) = ((area.width() / s).floor() + 1.0, (area.height() / s).floor() + 1.0);
    if !(nx.is_finite() && ny.is_finite()) || nx * ny > MAX_MARKS * MAX_MARKS {
        return;
    }
    let x0 = (area.min.x / s).ceil() * s;
    let y0 = (area.min.y / s).ceil() * s;
    let xs: Vec<f64> = (0..nx.min(4000.0) as usize).map(|i| x0 + i as f64 * s).filter(|v| *v <= area.max.x).collect();
    let ys: Vec<f64> = (0..ny.min(4000.0) as usize).map(|i| y0 + i as f64 * s).filter(|v| *v <= area.max.y).collect();
    match style {
        GridStyle::Dots => dots(painter, x, &xs, &ys, t.grid),
        GridStyle::Lines => lines(painter, x, area, &xs, &ys, s, t),
    }
}

fn dots(painter: &Painter, x: &Xform, xs: &[f64], ys: &[f64], color: Color32) {
    if (xs.len() * ys.len()) as f64 > MAX_MARKS {
        return;
    }
    let mut mesh = Mesh::default();
    let r = 0.8;
    for &gx in xs {
        for &gy in ys {
            let c = x.to_screen(protasena_geom::Pt::new(gx, gy));
            mesh.add_colored_rect(Rect::from_center_size(c, vec2(2.0 * r, 2.0 * r)), color);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn lines(painter: &Painter, x: &Xform, area: SheetRect, xs: &[f64], ys: &[f64], s: f64, t: &Tokens) {
    let major = s * 10.0;
    let is_major = |v: f64| ((v / major).round() * major - v).abs() < s * 1e-6;
    let (top, bottom) = (x.to_screen(protasena_geom::Pt::new(0.0, area.max.y)).y, x.to_screen(protasena_geom::Pt::new(0.0, area.min.y)).y);
    let (left, right) = (x.to_screen(protasena_geom::Pt::new(area.min.x, 0.0)).x, x.to_screen(protasena_geom::Pt::new(area.max.x, 0.0)).x);
    for &gx in xs {
        let sx = x.to_screen(protasena_geom::Pt::new(gx, 0.0)).x;
        let c = if is_major(gx) { t.grid } else { t.grid_line };
        painter.line_segment([pos2(sx, top), pos2(sx, bottom)], Stroke::new(1.0, c));
    }
    for &gy in ys {
        let sy = x.to_screen(protasena_geom::Pt::new(0.0, gy)).y;
        let c = if is_major(gy) { t.grid } else { t.grid_line };
        painter.line_segment([pos2(left, sy), pos2(right, sy)], Stroke::new(1.0, c));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_coarsens_until_readable() {
        assert_eq!(spacing(100.0, 0.2), 100.0);
        assert_eq!(spacing(100.0, 0.1), 500.0);
        assert_eq!(spacing(100.0, 0.01), 5000.0);
        assert_eq!(spacing(f64::NAN, 1.0), 100.0);
        assert!(spacing(10.0, 0.0).is_finite());
    }
}
