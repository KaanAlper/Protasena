//! The left pane: the overview window (the whole sheet with the visible area marked; a click
//! or drag pans there) above the object selector (the P and L buttons, the caption of the
//! current mode's list, and the list itself: picked devices, terminal kinds, port kinds…).

use std::collections::HashSet;

use egui::{Align2, Frame, Margin, RichText, Sense, Stroke, Ui, vec2};
use protasena_engine::Mode;
use protasena_render::sheet_rect;
use serde_json::json;

use crate::canvas::{Paint, Style, Xform};
use crate::theme::Tokens;
use crate::{App, control, tl, widgets};

/// Height of the overview window, in points.
pub const OVERVIEW_HEIGHT: f32 = 132.0;
/// Height of an object selector row, in points.
const ROW: f32 = 20.0;

/// The object selector's caption for a mode (Proteus shows one list per mode family).
pub fn caption(m: Mode) -> Option<&'static str> {
    Some(match m {
        Mode::Selection | Mode::Component => "DEVICES",
        Mode::Terminals => "TERMINALS",
        Mode::Subcircuit => "PORTS",
        Mode::DevicePins => "PINS",
        Mode::Graph => "GRAPHS",
        Mode::Generator => "GENERATORS",
        Mode::Probe => "PROBES",
        Mode::VirtualInstruments => "INSTRUMENTS",
        Mode::Symbols2d => "SYMBOLS",
        Mode::Markers2d => "MARKERS",
        Mode::Line2d | Mode::Box2d | Mode::Circle2d | Mode::Arc2d | Mode::ClosedPath2d | Mode::Text2d => "GRAPHICS",
        Mode::JunctionDot | Mode::WireLabel | Mode::TextScript | Mode::Buses | Mode::ActivePopup => return None,
    })
}

enum Clicked {
    Pick,
    Device(String, String),
    Terminal(&'static str),
    Port(&'static str),
    Pan(protasena_geom::Pt),
}

/// Draws the left pane.
pub fn show(app: &mut App, ui: &mut Ui) {
    let t = Tokens::get();
    let mut clicked = None;
    egui::Panel::left("sidepane")
        .resizable(true)
        .default_size(190.0)
        .size_range(140.0..=380.0)
        .frame(Frame::NONE.fill(t.panel).stroke(Stroke::new(1.0, t.border)).inner_margin(Margin::same(6)))
        .show(ui, |ui| {
            overview(app, ui, &mut clicked);
            ui.add_space(6.0);
            selector(app, ui, &mut clicked);
        });
    match clicked {
        Some(Clicked::Pick) => app.invoke("pick-device"),
        Some(Clicked::Device(name, library)) => {
            if let Err(e) = app.run("select-device", json!({ "device": name, "library": library })) {
                app.notify(e, true);
            }
        }
        Some(Clicked::Terminal(kind)) => {
            if let Err(e) = app.run("select-terminal", json!({ "kind": kind })) {
                app.notify(e, true);
            }
        }
        Some(Clicked::Port(kind)) => {
            if let Err(e) = app.run("select-port", json!({ "kind": kind })) {
                app.notify(e, true);
            }
        }
        Some(Clicked::Pan(at)) => {
            let _ = app.run("pan", json!({ "center": [at.x, at.y] }));
        }
        None => {}
    }
}

fn overview(app: &mut App, ui: &mut Ui, clicked: &mut Option<Clicked>) {
    let t = Tokens::get();
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), OVERVIEW_HEIGHT), Sense::click_and_drag());
    widgets::region(ui, "Overview", rect);
    ui.painter().rect(rect, 4.0, t.canvas, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    let Some(sheet) = app.session.sheet() else { return };
    let paper = sheet_rect(sheet);
    let x = Xform::fit(rect, paper, 8.0);
    let painter = ui.painter_at(rect);
    painter.rect_filled(egui::Rect::from_two_pos(x.to_screen(paper.min), x.to_screen(paper.max)), 0.0, t.sheet);
    let list = protasena_render::sheet_display_list(sheet, app.session.library());
    let none = HashSet::new();
    let p = Paint { painter: &painter, x, style: Style { tokens: t, ink: None, selected: &none, texts: false, min_width: 0.8 } };
    p.all(&list.prims);
    if let Some(cx) = app.canvas.xform {
        let v = cx.visible();
        let r = egui::Rect::from_two_pos(x.to_screen(v.min), x.to_screen(v.max)).intersect(rect.shrink(1.0));
        painter.rect(r, 2.0, t.accent.gamma_multiply(0.10), Stroke::new(1.5, t.accent), egui::StrokeKind::Inside);
    }
    if (resp.clicked() || resp.dragged())
        && let Some(pos) = resp.interact_pointer_pos()
    {
        *clicked = Some(Clicked::Pan(x.to_sheet(pos)));
    }
    control::record(ui, &resp, tl!("Overview"), "overview", None);
}

fn selector(app: &mut App, ui: &mut Ui, clicked: &mut Option<Clicked>) {
    let t = Tokens::get();
    let mode = app.session.mode();
    let cap = caption(mode);
    let header = ui.horizontal(|ui| {
        let devices = cap == Some("DEVICES");
        if letter_button(ui, "P", tl!("Pick Devices"), devices).clicked() {
            *clicked = Some(Clicked::Pick);
        }
        letter_button(ui, "L", tl!("Library Manager"), false);
        if let Some(c) = cap {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(crate::i18n::t(c)).size(11.5).color(t.text_dim).strong());
            });
        }
    });
    widgets::region(
        ui,
        "Object selector",
        header.response.rect.union(egui::Rect::from_min_size(header.response.rect.left_bottom(), vec2(ui.available_width(), ui.available_height()))),
    );
    ui.add_space(2.0);
    let frame = Frame::NONE.fill(t.field).stroke(Stroke::new(1.0, t.border)).corner_radius(4.0).inner_margin(Margin::same(2));
    frame.show(ui, |ui| {
        ui.set_min_size(vec2(ui.available_width(), ui.available_height().max(40.0)));
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match cap {
            Some("DEVICES") => {
                let chosen = app.session.device().cloned();
                for d in app.session.picked() {
                    let selected = chosen.as_ref() == Some(d);
                    if row(ui, &d.name, selected).clicked() {
                        *clicked = Some(Clicked::Device(d.name.clone(), d.library.clone()));
                    }
                }
            }
            Some("PORTS") => {
                let kind = app.session.port_kind();
                for (k, name) in crate::dialogs::subcircuit::PORT_KINDS {
                    if row(ui, name, k == kind).clicked() {
                        *clicked = Some(Clicked::Port(protasena_engine::port_kind_id(k)));
                    }
                }
            }
            Some("TERMINALS") => {
                let kind = app.session.terminal_kind();
                for (k, id, name) in crate::dialogs::TERMINAL_KINDS {
                    if row(ui, name, k == kind).clicked() {
                        *clicked = Some(Clicked::Terminal(id));
                    }
                }
            }
            _ => {}
        });
    });
}

/// A small lettered button of the selector header (P, L).
fn letter_button(ui: &mut Ui, letter: &str, label: &str, enabled: bool) -> egui::Response {
    let t = Tokens::get();
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(vec2(24.0, 22.0), sense);
    let (bg, fg) = match (enabled, resp.hovered()) {
        (true, true) => (t.accent, t.on_accent),
        (true, false) => (t.accent_soft, t.accent),
        (false, _) => (t.chrome, t.text_faint),
    };
    ui.painter().rect_filled(rect, 4.0, bg);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, letter, egui::FontId::proportional(13.0), fg);
    let l = label.to_owned();
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &l));
    control::record_rect(ui.ctx(), resp.id, resp.rect, label, "button", enabled, None);
    if enabled { resp.on_hover_text(label) } else { resp.on_disabled_hover_text(label) }
}

fn row(ui: &mut Ui, text: &str, selected: bool) -> egui::Response {
    let t = Tokens::get();
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::click());
    if selected {
        ui.painter().rect_filled(rect, 3.0, t.accent);
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 3.0, t.hover);
    }
    let color = if selected { t.on_accent } else { t.text };
    ui.painter().with_clip_rect(rect).text(rect.left_center() + vec2(6.0, 0.0), Align2::LEFT_CENTER, text, crate::theme::small(), color);
    let l = text.to_owned();
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, &l));
    control::record(ui, &resp, text, "list-item", Some(selected));
    resp
}
