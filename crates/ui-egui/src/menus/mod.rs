//! The menu bar: Proteus's menus from [`model`], drawn with our own item rows (a check-mark
//! gutter, the label, the shortcut text on the right). Items run their command as a menu pick;
//! items without one are disabled.

pub mod model;

use egui::{Align2, Frame, Margin, Sense, Stroke, Ui, vec2};
use protasena_engine::{Session, find_command};

use self::model::{Act, Check, Entry, MENUS};
use crate::theme::Tokens;
use crate::{App, control, i18n, widgets};

/// Height of the menu bar, in points.
pub const HEIGHT: f32 = 26.0;
/// Height of a menu item row, in points.
const ROW: f32 = 24.0;
/// Width of a menu, in points.
const WIDTH: f32 = 330.0;
/// Width of the check-mark gutter, in points.
const GUTTER: f32 = 26.0;

/// What a click chose.
enum Clicked {
    Act(Act),
    Sheet(u64),
}

/// Draws the menu bar and runs what was chosen.
pub fn menu_bar(app: &mut App, ui: &mut Ui) {
    let t = Tokens::get();
    let mut clicked = None;
    let s = &app.session;
    egui::Panel::top("menubar").exact_size(HEIGHT).frame(Frame::NONE.fill(t.chrome).inner_margin(Margin::symmetric(6, 0))).show(ui, |ui| {
        widgets::region(ui, "Menu bar", ui.max_rect());
        egui::MenuBar::new().ui(ui, |ui| {
            for m in MENUS {
                let r = ui.menu_button(i18n::t(m.label), |ui| {
                    ui.set_width(WIDTH);
                    for e in m.entries {
                        entry(ui, s, e, &mut clicked);
                    }
                });
                control::record(ui, &r.response, m.label, "menu", None);
            }
        });
    });
    match clicked {
        Some(Clicked::Act(Act::Cmd(id))) => app.invoke(id),
        Some(Clicked::Act(Act::Ui(u))) => app.ui_act(u),
        Some(Clicked::Sheet(id)) => {
            if let Err(e) = app.run("goto-sheet", serde_json::json!({ "sheet": id })) {
                app.notify(e, true);
            }
        }
        Some(Clicked::Act(Act::None)) | None => {}
    }
}

fn entry(ui: &mut Ui, s: &Session, e: &Entry, clicked: &mut Option<Clicked>) {
    match e {
        Entry::Item { label, shortcut, act, check } => {
            let enabled = match act {
                Act::Cmd(id) => find_command(id).is_some_and(|c| (c.enabled)(s).is_ok()),
                Act::Ui(_) => true,
                Act::None => false,
            };
            let on = check.is_some_and(|c| is_on(s, c));
            // The keyboard mapping's key, which System ▸ Set Keyboard Mapping may have changed.
            let key = match act {
                Act::Cmd(id) => s.shortcut(id),
                _ => shortcut.map(str::to_owned),
            };
            if item(ui, i18n::t(label), key.as_deref(), enabled, check.map(|_| on)) {
                *clicked = Some(Clicked::Act(*act));
            }
        }
        Entry::Sep => {
            ui.add_space(2.0);
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 7.0), Sense::hover());
            let y = rect.center().y;
            ui.painter()
                .line_segment([egui::pos2(rect.left() + 8.0, y), egui::pos2(rect.right() - 8.0, y)], Stroke::new(1.0, Tokens::get().separator));
            ui.add_space(2.0);
        }
        Entry::Sub { label, entries } => {
            let r = ui.menu_button(i18n::t(label), |ui| {
                ui.set_width(WIDTH);
                for e in *entries {
                    entry(ui, s, e, clicked);
                }
            });
            control::record(ui, &r.response, label, "menu", None);
        }
        Entry::Sheets => {
            for (i, (id, label)) in sheet_list(s).into_iter().enumerate() {
                let label = format!("{}. {label}", i + 1);
                if item(ui, &label, None, true, Some(id == s.sheet_id().0)) {
                    *clicked = Some(Clicked::Sheet(id));
                }
            }
        }
    }
}

/// The sheets in hierarchy order with their labels: a sub-sheet is indented under its parent.
pub fn sheet_list(s: &Session) -> Vec<(u64, String)> {
    let p = s.project();
    p.sheet_tree()
        .into_iter()
        .filter_map(|n| {
            let sheet = p.sheet(n.sheet)?;
            let indent = "    ".repeat(n.depth.min(8));
            Some((sheet.id.0, if n.depth == 0 { sheet.name.clone() } else { format!("{indent}{} ({})", sheet.name, n.path) }))
        })
        .collect()
}

fn is_on(s: &Session, c: Check) -> bool {
    let v = s.view();
    match c {
        Check::Grid => v.grid,
        Check::FalseOrigin => v.false_origin.is_some(),
        Check::XCursor => v.x_cursor,
        Check::Snap(snap) => v.snap == snap,
        Check::Autorouter => s.autorouter(),
    }
}

/// A menu item row; true when it was clicked (the menu then closes).
fn item(ui: &mut Ui, label: &str, shortcut: Option<&str>, enabled: bool, checked: Option<bool>) -> bool {
    let t = Tokens::get();
    let width = WIDTH;
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(vec2(width, ROW), sense);
    let l = label.to_owned();
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, checked == Some(true), &l));
    if ui.is_rect_visible(rect) {
        if enabled && resp.hovered() {
            ui.painter().rect_filled(rect, 4.0, t.accent_soft);
        }
        let color = if enabled { t.text } else { t.text_faint };
        if checked == Some(true) {
            let c = egui::pos2(rect.left() + GUTTER / 2.0, rect.center().y);
            let mark = [c + vec2(-4.5, 0.0), c + vec2(-1.5, 3.5), c + vec2(4.5, -3.5)];
            ui.painter().add(egui::Shape::line(mark.to_vec(), Stroke::new(1.8, if enabled { t.accent } else { t.text_faint })));
        }
        ui.painter().text(egui::pos2(rect.left() + GUTTER + 2.0, rect.center().y), Align2::LEFT_CENTER, label, crate::theme::body(), color);
        if let Some(sc) = shortcut {
            let sc_color = if enabled { t.text_dim } else { t.text_faint };
            ui.painter().text(egui::pos2(rect.right() - 10.0, rect.center().y), Align2::RIGHT_CENTER, sc, crate::theme::small(), sc_color);
        }
    }
    control::record_rect(ui.ctx(), resp.id, resp.rect, label, "menu-item", enabled, checked);
    if enabled && resp.clicked() {
        ui.close();
        return true;
    }
    false
}
