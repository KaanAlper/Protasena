//! Pick Devices: keywords, the category, sub-category and manufacturer lists with counts, the
//! results table (Device, Library, Description) and previews of the highlighted device's symbol
//! and package. OK (or a double click) adds the device to the object selector's DEVICES list.

use egui::{Align, Layout, RichText, Sense, Ui, vec2};
use protasena_engine::Session;
use protasena_geom::Transform;
use protasena_library::{Device, Library};
use serde_json::json;

use super::pick_model::{Counts, PickModel};
use super::{Reply, error_line, ok_cancel};
use crate::canvas::preview;
use crate::theme::Tokens;
use crate::{control, tl};

/// Height of a results row, in points.
const ROW: f32 = 20.0;
/// The results table's columns and their widths, in points.
fn columns() -> [(&'static str, f32); 3] {
    [(tl!("Device"), 120.0), (tl!("Library"), 110.0), (tl!("Description"), 400.0)]
}
/// Size of the dialog's body (the three columns), in points.
const BODY_W: f32 = 950.0;
const BODY_H: f32 = 480.0;

/// A fixed-width column of the body.
fn column(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(vec2(width, BODY_H), Layout::top_down(Align::Min), |ui| {
        ui.set_width(width);
        add(ui);
    });
}

/// A vertical line between columns.
fn divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(17.0, BODY_H), Sense::hover());
    ui.painter().vline(rect.center().x, rect.y_range(), egui::Stroke::new(1.0, Tokens::get().border));
}

/// The dialog's state.
#[derive(Default)]
pub struct PickDevices {
    /// The search.
    pub model: PickModel,
    /// The last refusal.
    pub error: Option<String>,
    focused: bool,
}

impl PickDevices {
    pub(super) fn ui(&mut self, ui: &mut Ui, s: &Session) -> Reply {
        let lib = s.library();
        self.model.refresh(lib);
        let mut reply = Reply::None;
        ui.allocate_ui_with_layout(vec2(BODY_W, BODY_H), Layout::left_to_right(Align::Min), |ui| {
            ui.set_height(BODY_H);
            column(ui, 230.0, |ui| self.filters(ui));
            divider(ui);
            column(ui, 430.0, |ui| {
                if let Some(r) = self.results(ui, lib) {
                    reply = r;
                }
            });
            divider(ui);
            column(ui, 250.0, |ui| previews(ui, self.model.device(lib), lib));
        });
        error_line(ui, self.error.as_ref());
        let (ok, cancel) = ok_cancel(ui, self.model.selected.is_some());
        if cancel {
            return Reply::Cancel;
        }
        if ok && let Some(d) = self.model.device(lib) {
            return pick(d, false);
        }
        reply
    }

    fn filters(&mut self, ui: &mut Ui) {
        ui.label(RichText::new(tl!("Keywords")).strong());
        let r = ui.add(egui::TextEdit::singleline(&mut self.model.keywords).desired_width(f32::INFINITY).hint_text(tl!("e.g. RES, 74LS00")));
        control::record(ui, &r, tl!("Keywords"), "field", None);
        if !self.focused {
            r.request_focus();
            self.focused = true;
        }
        if r.has_focus() {
            let (up, down) = ui.input(|i| (i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown)));
            if up {
                self.model.step(-1);
            }
            if down {
                self.model.step(1);
            }
        }
        ui.add_space(6.0);
        let m = &mut self.model;
        let (cats, subs, makers) = (m.categories.clone(), m.sub_categories.clone(), m.manufacturers.clone());
        if let Some(c) = list(ui, tl!("Category"), tl!("(All Categories)"), &cats, m.category.as_deref(), 150.0) {
            m.set_category(c);
        }
        if let Some(c) = list(ui, tl!("Sub-category"), tl!("(All Sub-categories)"), &subs, m.sub_category.as_deref(), 100.0) {
            m.set_sub_category(c);
        }
        if let Some(c) = list(ui, tl!("Manufacturer"), tl!("(All Manufacturers)"), &makers, m.manufacturer.as_deref(), 100.0) {
            m.manufacturer = c;
        }
    }

    fn results(&mut self, ui: &mut Ui, lib: &Library) -> Option<Reply> {
        let t = Tokens::get();
        let shown = if self.model.total > self.model.results.len() {
            format!("{} ({} {} {})", tl!("Results"), self.model.results.len(), tl!("of"), self.model.total)
        } else {
            format!("{} ({})", tl!("Results"), self.model.total)
        };
        ui.label(RichText::new(shown).strong());
        let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::hover());
        let mut x = head.left() + 7.0;
        for (name, w) in columns() {
            ui.painter().text(egui::pos2(x, head.center().y), egui::Align2::LEFT_CENTER, name, crate::theme::small(), t.text_dim);
            x += w;
        }
        let mut reply = None;
        let frame = egui::Frame::NONE.fill(t.field).stroke(egui::Stroke::new(1.0, t.border)).corner_radius(4.0);
        frame.show(ui, |ui| {
            ui.set_min_size(vec2(426.0, 410.0));
            egui::ScrollArea::vertical().id_salt("pick-results").auto_shrink([false, false]).max_height(410.0).show_rows(
                ui,
                ROW,
                self.model.results.len(),
                |ui, rows| {
                    for i in rows {
                        let Some(&di) = self.model.results.get(i) else { continue };
                        let Some(d) = lib.devices().get(di) else { continue };
                        let chosen = self.model.selected == Some(di);
                        let r = row(ui, d, chosen, &t);
                        if r.clicked() {
                            self.model.selected = Some(di);
                        }
                        if r.double_clicked() {
                            reply = Some(pick(d, true));
                        }
                    }
                },
            );
            if self.model.results.is_empty() {
                ui.add_space(12.0);
                ui.vertical_centered(|ui| ui.label(RichText::new(tl!("Type keywords or choose a category")).color(t.text_faint)));
            }
        });
        reply
    }
}

/// A results row.
fn row(ui: &mut Ui, d: &Device, chosen: bool, t: &Tokens) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::click());
    if chosen {
        ui.painter().rect_filled(rect, 3.0, t.accent_soft);
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 3.0, t.hover);
    }
    let mut x = rect.left() + 6.0;
    let widths = columns().map(|c| c.1);
    for ((text, color), w) in
        [(d.name.as_str(), t.text), (d.library.as_str(), t.text_dim), (d.description.as_str(), t.text_dim)].into_iter().zip(widths)
    {
        let clip = egui::Rect::from_min_max(egui::pos2(x, rect.top()), egui::pos2((x + w - 6.0).min(rect.right()), rect.bottom()));
        ui.painter().with_clip_rect(clip).text(egui::pos2(x, rect.center().y), egui::Align2::LEFT_CENTER, text, crate::theme::small(), color);
        x += w;
    }
    control::record(ui, &resp, &d.name, "list-item", Some(chosen));
    resp
}

/// A filter list with an "all" row; returns the new choice when one was clicked.
fn list(ui: &mut Ui, title: &str, all: &str, items: &Counts, chosen: Option<&str>, height: f32) -> Option<Option<String>> {
    let t = Tokens::get();
    ui.label(RichText::new(title).strong());
    let mut picked = None;
    egui::Frame::NONE.fill(t.field).stroke(egui::Stroke::new(1.0, t.border)).corner_radius(4.0).show(ui, |ui| {
        egui::ScrollArea::vertical().id_salt(title).max_height(height).auto_shrink([false, false]).show(ui, |ui| {
            ui.set_min_height(height);
            let entries = std::iter::once((None, all.to_owned(), None)).chain(items.iter().map(|(n, c)| (Some(n.clone()), n.clone(), Some(*c))));
            for (value, label, count) in entries {
                let selected = value.as_deref() == chosen;
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::click());
                if selected {
                    ui.painter().rect_filled(rect, 3.0, t.accent_soft);
                } else if resp.hovered() {
                    ui.painter().rect_filled(rect, 3.0, t.hover);
                }
                let p = ui.painter().with_clip_rect(rect);
                p.text(rect.left_center() + vec2(6.0, 0.0), egui::Align2::LEFT_CENTER, &label, crate::theme::small(), t.text);
                if let Some(c) = count {
                    p.text(rect.right_center() - vec2(6.0, 0.0), egui::Align2::RIGHT_CENTER, c.to_string(), crate::theme::small(), t.text_faint);
                }
                control::record(ui, &resp, &label, "list-item", Some(selected));
                if resp.clicked() {
                    picked = Some(value);
                }
            }
        });
    });
    ui.add_space(4.0);
    picked
}

/// The symbol and package previews.
fn previews(ui: &mut Ui, d: Option<&Device>, lib: &Library) {
    let t = Tokens::get();
    ui.label(RichText::new(tl!("Preview")).strong());
    let (rect, _) = ui.allocate_exact_size(vec2(250.0, 220.0), Sense::hover());
    ui.painter().rect(rect, 4.0, t.sheet, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    if let Some(d) = d {
        let prims = protasena_render::symbol_prims(d, 0, Transform::IDENTITY);
        preview(ui, rect.shrink(10.0), &prims, true);
        ui.label(RichText::new(&d.name).strong());
        if !d.description.is_empty() {
            ui.label(RichText::new(&d.description).color(t.text_dim).size(12.0));
        }
    }
    ui.add_space(6.0);
    ui.label(RichText::new(tl!("PCB Preview")).strong());
    let (rect, _) = ui.allocate_exact_size(vec2(250.0, 150.0), Sense::hover());
    ui.painter().rect(rect, 4.0, t.sheet, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    let package = d.and_then(|d| d.default_package.clone().or_else(|| d.packages.first().map(|p| p.name.clone())));
    match package.as_deref().and_then(|name| lib.package(name)) {
        Some(p) => {
            preview(ui, rect.shrink(10.0), &protasena_render::package_prims(p), false);
            ui.label(RichText::new(&p.name).color(t.text_dim).size(12.0));
        }
        None => {
            ui.put(rect, egui::Label::new(RichText::new(tl!("No PCB package")).color(t.text_faint)));
        }
    }
}

fn pick(d: &Device, keep: bool) -> Reply {
    Reply::Run { cmd: "pick-device", params: json!({ "pick": d.name, "library": d.library }), keep }
}
