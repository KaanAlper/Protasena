//! Small shared controls: icon buttons for the toolbars, separators, captions.

use egui::{Color32, CornerRadius, Response, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType, vec2};

use crate::control;
use crate::icons::{self, Icon, IconStyle};
use crate::theme::Tokens;

/// Side of a toolbar button, in points.
pub const TOOL: f32 = 26.0;
/// Side of the icon inside a toolbar button, in points.
pub const ICON: f32 = 18.0;

/// What a toolbar button shows.
#[derive(Clone, Copy, Debug)]
pub struct ToolLook<'a> {
    /// The pictogram.
    pub icon: Icon,
    /// Its name (tooltip and accessibility label).
    pub label: &'a str,
    /// Whether it can be clicked.
    pub enabled: bool,
    /// Whether it is pressed (a toggle that is on, the active mode).
    pub selected: bool,
}

/// An icon button of `size` points, recorded for the control channel under `role`.
pub fn icon_button(ui: &mut Ui, size: Vec2, look: ToolLook<'_>, role: &'static str) -> Response {
    let t = Tokens::get();
    let sense = if look.enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    let label = look.label.to_owned();
    let (enabled, selected) = (look.enabled, look.selected);
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, selected, &label));
    if ui.is_rect_visible(rect) {
        let r = CornerRadius::same(5);
        let hovered = look.enabled && resp.hovered();
        if look.selected {
            ui.painter().rect(rect.shrink(1.0), r, t.accent_soft, Stroke::new(1.0, t.accent.gamma_multiply(0.6)), egui::StrokeKind::Inside);
        } else if hovered {
            ui.painter().rect_filled(rect.shrink(1.0), r, t.hover);
        }
        let style = if look.enabled {
            IconStyle { ink: t.text, accent: t.accent, fill: t.chrome_hi }
        } else {
            IconStyle { ink: t.text_faint, accent: t.text_faint, fill: Color32::TRANSPARENT }
        };
        let icon_side = (size.x.min(size.y) * ICON / TOOL).round();
        icons::paint(ui.painter(), egui::Rect::from_center_size(rect.center(), vec2(icon_side, icon_side)), look.icon, style);
    }
    control::record_rect(ui.ctx(), resp.id, resp.rect, look.label, role, look.enabled, Some(look.selected));
    if look.enabled { resp.on_hover_text(look.label) } else { resp.on_disabled_hover_text(look.label) }
}

/// A toolbar button of the standard size.
pub fn tool(ui: &mut Ui, look: ToolLook<'_>, role: &'static str) -> Response {
    icon_button(ui, vec2(TOOL, TOOL), look, role)
}

/// A thin separator between toolbar groups; `vertical` for a horizontal toolbar.
pub fn separator(ui: &mut Ui, vertical: bool) {
    let t = Tokens::get();
    let size = if vertical { vec2(7.0, TOOL) } else { vec2(TOOL, 7.0) };
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let c = rect.center();
    let (a, b) = if vertical {
        (egui::pos2(c.x, rect.top() + 5.0), egui::pos2(c.x, rect.bottom() - 5.0))
    } else {
        (egui::pos2(rect.left() + 5.0, c.y), egui::pos2(rect.right() - 5.0, c.y))
    };
    ui.painter().line_segment([a, b], Stroke::new(1.0, t.separator));
}

/// Records an area of the window for the control channel and the layout tests.
pub fn region(ui: &Ui, name: &str, rect: egui::Rect) {
    control::record_rect(ui.ctx(), egui::Id::new(("region", name)), rect, name, "region", true, None);
}
