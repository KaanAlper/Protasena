//! The left toolbars: the Mode toolbar (Proteus's 22 modes, top to bottom, in three groups;
//! modes the engine has not built yet are disabled) and below it the Orientation toolbar
//! (rotate clockwise, rotate anti-clockwise, the angle box, X-mirror, Y-mirror).

use egui::{Frame, Margin, Ui, vec2};
use protasena_engine::Mode;
use serde_json::json;

use crate::icons::Icon;
use crate::theme::Tokens;
use crate::widgets::{self, ToolLook};
use crate::{App, control, i18n, tl};

/// Width of the left toolbars, in points.
pub const WIDTH: f32 = 36.0;
/// Side of a mode button, in points.
const BUTTON: f32 = 26.0;

/// The pictogram of a mode.
pub fn icon(m: Mode) -> Icon {
    match m {
        Mode::Selection => Icon::Selection,
        Mode::Component => Icon::Component,
        Mode::JunctionDot => Icon::JunctionDot,
        Mode::WireLabel => Icon::WireLabel,
        Mode::TextScript => Icon::TextScript,
        Mode::Buses => Icon::Buses,
        Mode::Subcircuit => Icon::Subcircuit,
        Mode::Terminals => Icon::Terminals,
        Mode::DevicePins => Icon::DevicePins,
        Mode::Graph => Icon::Graph,
        Mode::ActivePopup => Icon::ActivePopup,
        Mode::Generator => Icon::Generator,
        Mode::Probe => Icon::Probe,
        Mode::VirtualInstruments => Icon::Instruments,
        Mode::Line2d => Icon::Line2d,
        Mode::Box2d => Icon::Box2d,
        Mode::Circle2d => Icon::Circle2d,
        Mode::Arc2d => Icon::Arc2d,
        Mode::ClosedPath2d => Icon::Path2d,
        Mode::Text2d => Icon::Text2d,
        Mode::Symbols2d => Icon::Symbols2d,
        Mode::Markers2d => Icon::Markers2d,
    }
}

/// Where the groups of the Mode toolbar end (a separator follows these modes).
fn ends_group(m: Mode) -> bool {
    matches!(m, Mode::Subcircuit | Mode::VirtualInstruments)
}

enum Clicked {
    Mode(Mode),
    Cmd(&'static str),
    Angle(i32),
}

/// One slot of the left toolbars.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Slot {
    Mode(Mode),
    Sep,
    /// The gap between the Mode and the Orientation toolbar.
    Gap,
    Rotate(Icon, &'static str, &'static str),
    Angle,
    Mirror(Icon, &'static str, &'static str),
}

impl Slot {
    fn height(self) -> f32 {
        match self {
            Slot::Mode(_) | Slot::Rotate(..) | Slot::Mirror(..) => BUTTON + 1.0,
            Slot::Sep => 8.0,
            Slot::Gap => 10.0,
            Slot::Angle => 23.0,
        }
    }

    fn is_orientation(self) -> bool {
        matches!(self, Slot::Rotate(..) | Slot::Angle | Slot::Mirror(..))
    }
}

/// The slots top to bottom: the modes in their groups, then the Orientation toolbar.
fn slots() -> Vec<Slot> {
    let mut v = Vec::new();
    for m in Mode::ALL {
        v.push(Slot::Mode(m));
        if ends_group(m) {
            v.push(Slot::Sep);
        }
    }
    v.push(Slot::Gap);
    v.push(Slot::Rotate(Icon::RotateCw, tl!("Rotate Clockwise"), "rotate-cw"));
    v.push(Slot::Rotate(Icon::RotateCcw, tl!("Rotate Anti-clockwise"), "rotate-ccw"));
    v.push(Slot::Angle);
    v.push(Slot::Sep);
    v.push(Slot::Mirror(Icon::MirrorX, tl!("X-Mirror"), "mirror-x"));
    v.push(Slot::Mirror(Icon::MirrorY, tl!("Y-Mirror"), "mirror-y"));
    v
}

/// The slots in columns of at most `height` points (the toolbars wrap into a further column
/// when the window is too low, as Proteus's left rebar does).
fn columns(height: f32) -> Vec<Vec<Slot>> {
    let mut cols: Vec<Vec<Slot>> = vec![Vec::new()];
    let mut used = 0.0;
    for s in slots() {
        let h = s.height();
        let fresh = cols.last().is_none_or(Vec::is_empty);
        if used + h > height && !fresh {
            cols.push(Vec::new());
            used = 0.0;
        }
        if matches!(s, Slot::Sep | Slot::Gap) && cols.last().is_some_and(Vec::is_empty) {
            continue;
        }
        if let Some(c) = cols.last_mut() {
            c.push(s);
        }
        used += h;
    }
    cols
}

/// Draws the Mode and Orientation toolbars.
pub fn show(app: &mut App, ui: &mut Ui) {
    let t = Tokens::get();
    let mut clicked = None;
    let cols = columns((ui.available_height() - 10.0).max(BUTTON));
    let width = WIDTH * cols.len().max(1) as f32;
    egui::Panel::left("modebar")
        .exact_size(width)
        .show_separator_line(false)
        .frame(Frame::NONE.fill(t.chrome).inner_margin(Margin::symmetric(4, 4)))
        .show(ui, |ui| {
            let mut modes = egui::Rect::NOTHING;
            let mut orient = egui::Rect::NOTHING;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 1.0);
                for col in &cols {
                    ui.vertical(|ui| {
                        ui.set_width(BUTTON);
                        for slot in col {
                            let r = slot_ui(app, ui, *slot, &mut clicked);
                            if matches!(slot, Slot::Mode(_)) {
                                modes = modes.union(r);
                            } else if slot.is_orientation() {
                                orient = orient.union(r);
                            }
                        }
                    });
                }
            });
            widgets::region(ui, "Mode toolbar", modes);
            widgets::region(ui, "Orientation toolbar", orient);
        });
    match clicked {
        Some(Clicked::Mode(m)) => {
            if let Err(e) = app.run("set-mode", json!({ "mode": m.id() })) {
                app.notify(e, true);
            }
        }
        Some(Clicked::Cmd(id)) => {
            if let Err(e) = app.run(id, json!({})) {
                app.notify(e, true);
            }
        }
        Some(Clicked::Angle(a)) => {
            let mirror = app.session.placement().is_mirrored();
            if let Err(e) = app.run("set-orientation", json!({ "angle": a, "mirror": mirror })) {
                app.notify(e, true);
            }
        }
        None => {}
    }
}

/// Draws one slot; returns its rectangle.
fn slot_ui(app: &App, ui: &mut Ui, slot: Slot, clicked: &mut Option<Clicked>) -> egui::Rect {
    let p = app.session.placement();
    match slot {
        Slot::Mode(m) => {
            let look = ToolLook { icon: icon(m), label: i18n::t(m.label()), enabled: m.is_built(), selected: app.session.mode() == m };
            let r = widgets::icon_button(ui, vec2(BUTTON, BUTTON), look, "mode");
            if r.clicked() {
                *clicked = Some(Clicked::Mode(m));
            }
            r.rect
        }
        Slot::Sep => {
            widgets::separator(ui, false);
            egui::Rect::NOTHING
        }
        Slot::Gap => {
            ui.add_space(10.0);
            egui::Rect::NOTHING
        }
        Slot::Rotate(icon, label, id) | Slot::Mirror(icon, label, id) => {
            let selected = id == "mirror-x" && p.is_mirrored();
            let look = ToolLook { icon, label, enabled: true, selected };
            let r = widgets::icon_button(ui, vec2(BUTTON, BUTTON), look, "tool");
            if r.clicked() {
                *clicked = Some(Clicked::Cmd(id));
            }
            r.rect
        }
        Slot::Angle => angle_box(app, ui, clicked),
    }
}

fn angle_box(app: &App, ui: &mut Ui, clicked: &mut Option<Clicked>) -> egui::Rect {
    let p = app.session.placement();
    let id = egui::Id::new("angle-box");
    let mut text = ui.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_else(|| format!("{}°", p.angle_degrees()));
    let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(BUTTON).horizontal_align(egui::Align::Center).font(crate::theme::small()));
    control::record(ui, &r, tl!("Angle"), "field", None);
    if r.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, text.clone()));
    } else {
        ui.data_mut(|d| d.remove::<String>(id));
    }
    if r.lost_focus() {
        let digits: String = text.chars().filter(|c| c.is_ascii_digit() || *c == '-').collect();
        if let Ok(a) = digits.parse::<i32>() {
            *clicked = Some(Clicked::Angle(a.rem_euclid(360)));
        }
    }
    r.on_hover_text(tl!("Angle")).rect
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_low_window_wraps_the_toolbars_into_columns_without_losing_a_slot() {
        let all = slots();
        assert_eq!(columns(10_000.0), vec![all.clone()]);
        let cols = columns(400.0);
        assert!(cols.len() >= 2);
        let flat: Vec<Slot> = cols.iter().flatten().copied().collect();
        let buttons = |v: &[Slot]| v.iter().filter(|s| !matches!(s, Slot::Sep | Slot::Gap)).count();
        assert_eq!(buttons(&flat), buttons(&all));
        assert!(cols.iter().all(|c| c.iter().map(|s| s.height()).sum::<f32>() <= 400.0));
        assert!(cols.iter().all(|c| !matches!(c.first(), Some(Slot::Sep | Slot::Gap))));
    }
}
