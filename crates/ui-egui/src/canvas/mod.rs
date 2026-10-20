//! The editing window: the sheet with its border and grid, the design drawn from the render
//! crate's primitives, the selection, the tool previews from the engine (ghost part, rubber-band
//! wire, dragged items, selection box), the snap cursor and transient messages.
//!
//! Pointer events go to the engine ([`input`]); the middle button pans, the wheel zooms about the
//! cursor, F5–F8 and the View menu use the engine's view commands.

mod grid;
mod input;
pub mod paint;

use std::collections::HashSet;
use std::sync::Arc;

use egui::{Color32, CornerRadius, Frame, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};
use protasena_doc::{ItemId, Sheet};
use protasena_engine::{Hit, Preview};
use protasena_geom::Pt;
use protasena_render::{DisplayList, Prim, component_prims, item_prims, sheet_display_list, sheet_rect};
use serde_json::json;

pub use paint::{Paint, Style, Xform};

use crate::theme::Tokens;
use crate::{App, widgets};

/// Editing-window state kept between frames.
#[derive(Default)]
pub struct CanvasState {
    /// The window's area in the last frame.
    pub rect: Option<Rect>,
    /// The transform used in the last frame.
    pub xform: Option<Xform>,
    /// View ▸ Zoom To Area: `Some(None)` waits for the drag, `Some(Some(p))` drags from `p`.
    pub zoom_area: Option<Option<Pt>>,
    /// Whether the sheet was fitted into the window once.
    pub fitted: bool,
    cache: Option<(Sheet, usize, Arc<DisplayList>)>,
    pub(crate) last_pointer: Option<Pos2>,
    pub(crate) pressed: bool,
    pub(crate) wheel: f32,
    viewport: Option<(f32, f32)>,
}

impl CanvasState {
    /// The display list of `sheet`, rebuilt only when the sheet or the library changed.
    fn list(&mut self, sheet: &Sheet, lib: &protasena_library::Library) -> Arc<DisplayList> {
        if let Some((s, n, list)) = &self.cache
            && s == sheet
            && *n == lib.len()
        {
            return Arc::clone(list);
        }
        let list = Arc::new(sheet_display_list(sheet, lib));
        self.cache = Some((sheet.clone(), lib.len(), Arc::clone(&list)));
        list
    }
}

/// Draws the editing window and feeds it input.
pub fn show(app: &mut App, ui: &mut Ui) {
    let t = Tokens::get();
    egui::CentralPanel::default().frame(Frame::NONE.fill(t.canvas)).show(ui, |ui| {
        let rect = ui.max_rect();
        widgets::region(ui, "Editing window", rect);
        sync_viewport(app, rect);
        let resp = ui.allocate_rect(rect, Sense::click_and_drag());
        if app.dialog.is_none() {
            input::handle(app, ui, &resp, rect);
        }
        let view = app.session.view();
        let x = Xform { rect, center: view.center, zoom: view.zoom };
        app.canvas.rect = Some(rect);
        app.canvas.xform = Some(x);
        draw(app, ui, x);
    });
}

/// Tells the engine the window size, and fits the sheet the first time.
fn sync_viewport(app: &mut App, rect: Rect) {
    let size = (rect.width().round(), rect.height().round());
    if app.canvas.viewport != Some(size) && size.0 >= 1.0 && size.1 >= 1.0 {
        let _ = app.run("set-viewport", json!({ "width": size.0, "height": size.1 }));
        app.canvas.viewport = Some(size);
    }
    if !app.canvas.fitted {
        let _ = app.run("zoom-all", json!({}));
        app.canvas.fitted = true;
    }
}

fn draw(app: &mut App, ui: &Ui, x: Xform) {
    let t = Tokens::get();
    let painter = ui.painter_at(x.rect);
    let Some(sheet) = app.session.sheet().cloned() else { return };
    let paper = sheet_rect(&sheet);
    let (a, b) = (x.to_screen(paper.min), x.to_screen(paper.max));
    let paper_rect = Rect::from_two_pos(a, b);
    painter.rect_filled(paper_rect.translate(vec2(2.0, 3.0)), 0.0, Color32::from_black_alpha(18));
    painter.rect_filled(paper_rect, 0.0, t.sheet);
    let view = app.session.view();
    if view.grid {
        grid::draw(&painter, &x, paper, view.snap.step(), app.ui.grid_style, &t);
    }
    let list = app.canvas.list(&sheet, app.session.library());
    let selected: HashSet<ItemId> = app.session.selection().iter().copied().collect();
    let p = Paint { painter: &painter, x, style: Style { tokens: t, ink: None, selected: &selected, texts: true, min_width: 1.0 } };
    p.all(&list.prims);
    previews(app, &painter, x);
    cursor(app, &painter, x);
    zoom_area(app, &painter, x);
    toast(app, &painter, x.rect);
}

fn previews(app: &App, painter: &egui::Painter, x: Xform) {
    let t = Tokens::get();
    let pv: Preview = app.session.preview();
    let none = HashSet::new();
    let ghost =
        Paint { painter, x, style: Style { tokens: t, ink: Some(t.preview.gamma_multiply(0.75)), selected: &none, texts: true, min_width: 1.2 } };
    if let Some(c) = &pv.ghost {
        ghost.all(&component_prims(c, app.session.library()));
    }
    for item in &pv.moved {
        ghost.all(&item_prims(item, app.session.library()));
    }
    for (pts, width) in [(&pv.wire, 2.0), (&pv.bus, 4.0)] {
        if pts.len() >= 2 {
            let screen: Vec<Pos2> = pts.iter().map(|p| x.to_screen(*p)).collect();
            painter.add(egui::Shape::line(screen, Stroke::new(width, t.preview)));
        }
    }
    if let Some([a, b]) = pv.rect {
        let r = Rect::from_two_pos(x.to_screen(a), x.to_screen(b));
        painter.rect(r, CornerRadius::ZERO, t.accent.gamma_multiply(0.08), Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
    }
    let starts = app.session.mode().starts_wires();
    if let Some(at) = pv.hover.as_ref().filter(|h| h.starts_wire() && starts).and_then(Hit::attach_point) {
        painter.circle_stroke(x.to_screen(at), 6.0, Stroke::new(1.5, t.selection));
    }
}

fn cursor(app: &App, painter: &egui::Painter, x: Xform) {
    if !app.canvas.last_pointer.is_some_and(|p| x.rect.contains(p)) {
        return;
    }
    let t = Tokens::get();
    let c = x.to_screen(app.session.cursor());
    let stroke = Stroke::new(1.0, t.cursor);
    if app.session.view().x_cursor {
        painter.line_segment([pos2(x.rect.left(), c.y), pos2(x.rect.right(), c.y)], stroke);
        painter.line_segment([pos2(c.x, x.rect.top()), pos2(c.x, x.rect.bottom())], stroke);
    } else {
        let arm = 6.0;
        painter.line_segment([c - vec2(arm, 0.0), c + vec2(arm, 0.0)], stroke);
        painter.line_segment([c - vec2(0.0, arm), c + vec2(0.0, arm)], stroke);
    }
}

fn zoom_area(app: &App, painter: &egui::Painter, x: Xform) {
    let (Some(Some(from)), Some(p)) = (app.canvas.zoom_area, app.canvas.last_pointer) else { return };
    let t = Tokens::get();
    let r = Rect::from_two_pos(x.to_screen(from), p);
    painter.rect(r, CornerRadius::ZERO, t.accent.gamma_multiply(0.08), Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
}

fn toast(app: &App, painter: &egui::Painter, rect: Rect) {
    let Some(toast) = &app.toast else { return };
    let t = Tokens::get();
    let galley = painter.layout(toast.text.clone(), crate::theme::body(), t.text, (rect.width() - 80.0).max(100.0));
    let size = galley.size() + vec2(24.0, 14.0);
    let r = Rect::from_min_size(pos2(rect.left() + 16.0, rect.bottom() - 16.0 - size.y), size);
    let edge = if toast.error { t.warn } else { t.border };
    painter.rect(r, CornerRadius::same(6), t.panel, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
    painter.galley(r.min + vec2(12.0, 7.0), galley, t.text);
}

/// Draws `prims` fitted into `rect` (previews, the overview); returns the transform used.
pub fn preview(ui: &Ui, rect: Rect, prims: &[Prim], texts: bool) -> Option<Xform> {
    let bounds = prims.iter().filter_map(Prim::bounds).reduce(protasena_geom::Rect::union)?;
    let x = Xform::fit(rect, bounds.inflate(20.0), 0.0);
    let none = HashSet::new();
    let painter = ui.painter_at(rect);
    let p = Paint { painter: &painter, x, style: Style { tokens: Tokens::get(), ink: None, selected: &none, texts, min_width: 1.0 } };
    p.all(prims);
    Some(x)
}
