//! Sheet ↔ screen transform and the painting of render primitives with the theme's inks.

use std::collections::HashSet;

use egui::{Color32, Mesh, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};
use protasena_doc::ItemId;
use protasena_geom::{Pt, Rect as SheetRect};
use protasena_render::geometry::{flatten_arc, flatten_bezier, is_convex, triangulate};
use protasena_render::{HAlign, Ink, Prim, Shape as PrimShape, VAlign};

use crate::theme::Tokens;

/// Largest screen coordinate drawn (far-off geometry is clamped, not lost in float overflow).
const MAX_SCREEN: f64 = 1.0e6;
/// Smallest text drawn, in points (smaller texts are skipped).
const MIN_TEXT: f32 = 3.5;
/// Text height to font size: the font's em is larger than its letters.
const FONT_PER_HEIGHT: f64 = 1.3;

/// Maps sheet coordinates (thou, y up) into a screen rectangle (points, y down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Xform {
    /// The screen area.
    pub rect: Rect,
    /// Sheet point at the middle of `rect`.
    pub center: Pt,
    /// Points per thou.
    pub zoom: f64,
}

impl Xform {
    /// The transform that shows `area` as large as fits in `rect`, with `margin` points around.
    pub fn fit(rect: Rect, area: SheetRect, margin: f32) -> Xform {
        let (w, h) = (area.width().max(1.0), area.height().max(1.0));
        let (vw, vh) = (f64::from((rect.width() - 2.0 * margin).max(1.0)), f64::from((rect.height() - 2.0 * margin).max(1.0)));
        Xform { rect, center: area.center(), zoom: (vw / w).min(vh / h) }
    }

    /// The screen point of a sheet point.
    pub fn to_screen(&self, p: Pt) -> Pos2 {
        let c = self.rect.center();
        let x = ((p.x - self.center.x) * self.zoom).clamp(-MAX_SCREEN, MAX_SCREEN);
        let y = ((p.y - self.center.y) * self.zoom).clamp(-MAX_SCREEN, MAX_SCREEN);
        pos2(c.x + x as f32, c.y - y as f32)
    }

    /// The sheet point under a screen point.
    pub fn to_sheet(&self, s: Pos2) -> Pt {
        let c = self.rect.center();
        let z = self.zoom.max(1e-12);
        Pt::new(self.center.x + f64::from(s.x - c.x) / z, self.center.y - f64::from(s.y - c.y) / z)
    }

    /// A length in thou as points.
    pub fn len(&self, thou: f64) -> f32 {
        (thou * self.zoom).clamp(0.0, MAX_SCREEN) as f32
    }

    /// The part of the sheet that is visible.
    pub fn visible(&self) -> SheetRect {
        SheetRect::new(self.to_sheet(self.rect.left_bottom()), self.to_sheet(self.rect.right_top()))
    }
}

/// How primitives are coloured.
pub struct Style<'a> {
    /// The palette.
    pub tokens: Tokens,
    /// Draw every primitive in this colour (ghosts, thumbnails).
    pub ink: Option<Color32>,
    /// Items drawn in the selection colour.
    pub selected: &'a HashSet<ItemId>,
    /// Draw texts.
    pub texts: bool,
    /// Smallest stroke, in points.
    pub min_width: f32,
    /// The design's template: its styles' colours replace the theme's.
    pub template: Option<&'a protasena_doc::Template>,
}

/// Paints primitives.
pub struct Paint<'a> {
    /// Where.
    pub painter: &'a Painter,
    /// The transform.
    pub x: Xform,
    /// The colours.
    pub style: Style<'a>,
}

impl Paint<'_> {
    fn color(&self, ink: Ink, owner: Option<ItemId>) -> Color32 {
        if let Some(c) = self.style.ink {
            return c;
        }
        if owner.is_some_and(|o| self.style.selected.contains(&o)) {
            return self.style.tokens.selection;
        }
        match self.style.template.and_then(|t| protasena_render::ink_colour(t, ink)) {
            Some(c) => crate::theme::colour(c),
            None => self.style.tokens.ink(ink),
        }
    }

    /// The fill of a closed shape: a selected item's body fill is a soft tint of the selection
    /// colour (a fill in the outline's own ink, such as a junction dot, stays solid).
    fn fill_color(&self, p: &Prim, fill: Ink) -> Color32 {
        let selected = self.style.ink.is_none() && p.owner.is_some_and(|o| self.style.selected.contains(&o));
        if selected && fill != p.ink { self.style.tokens.selection.gamma_multiply(0.18) } else { self.color(fill, p.owner) }
    }

    fn stroke(&self, p: &Prim) -> Stroke {
        let selected = p.owner.is_some_and(|o| self.style.selected.contains(&o));
        let w = self.x.len(p.width).max(self.style.min_width) + if selected { 1.0 } else { 0.0 };
        Stroke::new(w.min(64.0), self.color(p.ink, p.owner))
    }

    /// Paints every primitive whose bounds meet the visible area.
    pub fn all<'p>(&self, prims: impl IntoIterator<Item = &'p Prim>) {
        let vis = self.x.visible();
        for p in prims {
            if p.bounds().is_some_and(|b| b.intersects(vis)) {
                self.prim(p);
            }
        }
    }

    /// Paints one primitive.
    pub fn prim(&self, p: &Prim) {
        let tol = 0.35 / self.x.zoom.max(1e-9);
        match &p.shape {
            PrimShape::Line { a, b } => {
                self.painter.line_segment([self.x.to_screen(*a), self.x.to_screen(*b)], self.stroke(p));
            }
            PrimShape::Polyline { points, closed } => self.path(p, points, *closed),
            PrimShape::Circle { center, radius } => {
                let (c, r) = (self.x.to_screen(*center), self.x.len(*radius).max(0.5));
                let fill = p.fill.map_or(Color32::TRANSPARENT, |f| self.fill_color(p, f));
                self.painter.add(Shape::circle_filled(c, r, fill));
                if p.fill != Some(p.ink) {
                    self.painter.add(Shape::circle_stroke(c, r, self.stroke(p)));
                }
            }
            PrimShape::Arc { center, radius, start, sweep } => {
                let pts = flatten_arc(*center, *radius, *start, *sweep, tol);
                self.path(p, &pts, false);
            }
            PrimShape::Bezier { points } => {
                let span = points.windows(2).filter_map(|w| if let [a, b] = w { Some(a.distance(*b)) } else { None }).sum::<f64>();
                let n = ((span * self.x.zoom) / 6.0).ceil().clamp(4.0, 64.0) as usize;
                self.path(p, &flatten_bezier(*points, n), false);
            }
            PrimShape::Text { at, text, size, angle, h, v } => self.text(p, *at, text, *size, *angle, *h, *v),
        }
    }

    fn path(&self, p: &Prim, points: &[Pt], closed: bool) {
        let screen: Vec<Pos2> = points.iter().map(|q| self.x.to_screen(*q)).collect();
        if screen.len() < 2 {
            return;
        }
        if closed && let Some(fill) = p.fill {
            let color = self.fill_color(p, fill);
            if is_convex(points) {
                self.painter.add(Shape::convex_polygon(screen.clone(), color, Stroke::NONE));
            } else {
                let mut mesh = Mesh::default();
                for s in &screen {
                    mesh.colored_vertex(*s, color);
                }
                for t in triangulate(points) {
                    let [a, b, c] = t.map(|i| u32::try_from(i).unwrap_or(0));
                    mesh.add_triangle(a, b, c);
                }
                self.painter.add(Shape::mesh(mesh));
            }
        }
        if p.fill == Some(p.ink) && closed {
            return;
        }
        let stroke = self.stroke(p);
        if closed {
            self.painter.add(Shape::closed_line(screen, stroke));
        } else {
            self.painter.add(Shape::line(screen, stroke));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn text(&self, p: &Prim, at: Pt, text: &str, size: f64, angle: f64, h: HAlign, v: VAlign) {
        if !self.style.texts {
            return;
        }
        let px = self.x.len(size * FONT_PER_HEIGHT);
        if !(MIN_TEXT..=400.0).contains(&px) || text.is_empty() {
            return;
        }
        let color = self.color(p.ink, p.owner);
        let galley = self.painter.layout_no_wrap(text.chars().take(1024).collect(), egui::FontId::proportional(px), color);
        let size = galley.size();
        let x0 = match h {
            HAlign::Left => 0.0,
            HAlign::Center => -size.x / 2.0,
            HAlign::Right => -size.x,
        };
        let y0 = match v {
            VAlign::Bottom => -size.y,
            VAlign::Middle => -size.y / 2.0,
            VAlign::Top => 0.0,
        };
        let a = -(angle.to_radians()) as f32;
        let (sin, cos) = a.sin_cos();
        let anchor = self.x.to_screen(at);
        let pos = anchor + vec2(x0 * cos - y0 * sin, x0 * sin + y0 * cos);
        self.painter.add(egui::epaint::TextShape::new(pos, galley, color).with_angle(a));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_round_trips_and_flips_y() {
        let x = Xform { rect: Rect::from_min_size(pos2(100.0, 50.0), vec2(800.0, 600.0)), center: Pt::new(1000.0, 500.0), zoom: 0.1 };
        assert_eq!(x.to_screen(Pt::new(1000.0, 500.0)), pos2(500.0, 350.0));
        assert_eq!(x.to_screen(Pt::new(1100.0, 600.0)), pos2(510.0, 340.0), "y goes up on the sheet, down on the screen");
        let back = x.to_sheet(pos2(510.0, 340.0));
        assert!((back.x - 1100.0).abs() < 1e-3 && (back.y - 600.0).abs() < 1e-3);
        let v = x.visible();
        assert!((v.width() - 8000.0).abs() < 1e-6 && (v.height() - 6000.0).abs() < 1e-6);
        let far = x.to_screen(Pt::new(1e300, -1e300));
        assert!(far.x.is_finite() && far.y.is_finite());
    }

    #[test]
    fn fit_shows_the_whole_area() {
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(200.0, 100.0));
        let x = Xform::fit(rect, SheetRect::new(Pt::new(-1000.0, -1000.0), Pt::new(1000.0, 1000.0)), 0.0);
        assert!((x.zoom - 0.05).abs() < 1e-9);
        assert_eq!(x.center, Pt::ORIGIN);
    }
}
