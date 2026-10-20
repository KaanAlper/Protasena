//! The status bar, left to right as in Proteus: the animation buttons (Play, Step, Pause, Stop;
//! disabled until simulation exists), the messages indicator, the design variant box ("Base
//! Design", disabled), the current sheet's name, and the cursor's x/y coordinates with the unit.

use egui::{Align, Align2, Frame, Layout, Margin, Sense, Stroke, Ui, vec2};

use crate::dialogs::results::Results;
use crate::icons::{self, Icon, IconStyle};
use crate::theme::Tokens;
use crate::widgets::{self, ToolLook};
use crate::{App, LibraryStatus, control, tl};

/// Height of the status bar, in points.
pub const HEIGHT: f32 = 30.0;

/// A coordinate as the status bar shows it: sign and one decimal (`+1100.0`).
pub fn coord(v: f64) -> String {
    if v.is_finite() { format!("{v:+.1}") } else { "?".to_owned() }
}

/// Draws the status bar.
pub fn show(app: &mut App, ui: &mut Ui) {
    let t = Tokens::get();
    let mut open_messages = false;
    egui::Panel::bottom("status")
        .exact_size(HEIGHT)
        .show_separator_line(false)
        .frame(Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.border)).inner_margin(Margin::symmetric(6, 2)))
        .show(ui, |ui| {
            widgets::region(ui, "Status bar", ui.max_rect());
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (icon, label) in [(Icon::Play, tl!("Play")), (Icon::Step, tl!("Step")), (Icon::Pause, tl!("Pause")), (Icon::Stop, tl!("Stop"))] {
                    let look = ToolLook { icon, label, enabled: false, selected: false };
                    widgets::icon_button(ui, vec2(24.0, 24.0), look, "status");
                }
                widgets::separator(ui, true);
                open_messages = messages(app, ui);
                widgets::separator(ui, true);
                let variant = ui.add_enabled_ui(false, |ui| {
                    egui::ComboBox::from_id_salt("variant").width(120.0).selected_text(tl!("Base Design")).show_ui(ui, |_| {});
                });
                control::record_rect(ui.ctx(), egui::Id::new("variant"), variant.response.rect, tl!("Base Design"), "status", false, None);
                widgets::separator(ui, true);
                let sheet = app.session.sheet().map(|s| s.name.clone()).unwrap_or_default();
                field(ui, &sheet, 150.0, &format!("Sheet: {sheet}"));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    field(ui, tl!("th"), 26.0, &format!("Unit: {}", tl!("th")));
                    let at = app.session.view().display_coords(app.session.cursor());
                    let text = format!("x: {}   y: {}", coord(at.x), coord(at.y));
                    field(ui, &text, 190.0, &text);
                });
            });
        });
    if open_messages {
        app.results = Some(Results::Messages);
    }
}

/// The messages indicator; true when clicked.
fn messages(app: &App, ui: &mut Ui) -> bool {
    let t = Tokens::get();
    let n = app.session.log().len();
    let text = match &app.library {
        LibraryStatus::Loading(_) => tl!("Loading parts library…").to_owned(),
        _ if n == 0 => tl!("No Messages").to_owned(),
        _ if n == 1 => format!("1 {}", tl!("Message")),
        _ => format!("{n} {}", tl!("Messages")),
    };
    let w = ui.fonts_mut(|f| f.layout_no_wrap(text.clone(), crate::theme::small(), t.text).size().x) + 34.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 4.0, t.hover);
    }
    let icon = egui::Rect::from_center_size(egui::pos2(rect.left() + 13.0, rect.center().y), vec2(16.0, 16.0));
    icons::paint(ui.painter(), icon, Icon::Info, IconStyle { ink: t.text_dim, accent: t.accent, fill: t.chrome });
    ui.painter().text(egui::pos2(rect.left() + 26.0, rect.center().y), Align2::LEFT_CENTER, &text, crate::theme::small(), t.text_dim);
    control::record(ui, &resp, &text, "status", None);
    resp.clicked()
}

/// A sunken read-only field.
fn field(ui: &mut Ui, text: &str, width: f32, name: &str) {
    let t = Tokens::get();
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 22.0), Sense::hover());
    ui.painter().rect(rect, 4.0, t.chrome_hi, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    ui.painter().with_clip_rect(rect.shrink(2.0)).text(
        egui::pos2(rect.left() + 8.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        crate::theme::small(),
        t.text,
    );
    control::record(ui, &resp, name, "status", None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_carry_a_sign_and_one_decimal() {
        assert_eq!(coord(1100.0), "+1100.0");
        assert_eq!(coord(-1700.04), "-1700.0");
        assert_eq!(coord(0.0), "+0.0");
        assert_eq!(coord(f64::NAN), "?");
    }
}
