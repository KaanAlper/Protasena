//! The toolbar band under the menu bar — Global, View, Edit and Design toolbars, left to right,
//! as in Proteus — and the tab strip with the Schematic Capture tab. Buttons run the command of
//! the menu item they mirror; the others are disabled.

use egui::{Align2, Frame, Margin, Sense, Stroke, Ui, vec2};
use protasena_engine::{Session, find_command};

use crate::icons::{self, Icon, IconStyle};
use crate::menus::model::{Act, UiAct};
use crate::theme::Tokens;
use crate::widgets::{self, ToolLook};
use crate::{App, control, i18n, tl};

/// Height of the toolbar band, in points.
pub const HEIGHT: f32 = 36.0;
/// Height of the tab strip, in points.
pub const TABS_HEIGHT: f32 = 30.0;

/// Whether a toggle button shows pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pressed {
    /// Toggle Grid is on.
    Grid,
    /// Toggle False Origin is set.
    FalseOrigin,
    /// Wire Autorouter is on.
    Autorouter,
    /// The Schematic Capture module (always: it is the one shown).
    Schematic,
}

/// One toolbar entry.
#[derive(Clone, Copy, Debug)]
pub enum Tool {
    /// A button.
    Button {
        /// Its name (tooltip).
        label: &'static str,
        /// Its pictogram.
        icon: Icon,
        /// What it runs.
        act: Act,
        /// Its pressed state, for toggles.
        pressed: Option<Pressed>,
    },
    /// A separator.
    Sep,
}

const fn b(label: &'static str, icon: Icon, act: Act) -> Tool {
    Tool::Button { label, icon, act, pressed: None }
}

const fn toggle(label: &'static str, icon: Icon, act: Act, pressed: Pressed) -> Tool {
    Tool::Button { label, icon, act, pressed: Some(pressed) }
}

use Act::{Cmd, None as Off};
use Tool::Sep;

/// The Global Toolbar.
pub const GLOBAL: &[Tool] = &[
    b("New Project", Icon::New, Cmd("new")),
    b("Open Project", Icon::Open, Cmd("open")),
    b("Save Project", Icon::Save, Cmd("save")),
    b("Close Project", Icon::Close, Off),
    Sep,
    b("Home Page", Icon::Home, Off),
    toggle("Schematic Capture", Icon::Schematic, Off, Pressed::Schematic),
    b("PCB Layout", Icon::Pcb, Off),
    b("3D Visualizer", Icon::View3d, Off),
    b("Gerber Viewer", Icon::Gerber, Off),
    b("Design Explorer", Icon::Explorer, Off),
    b("Bill of Materials", Icon::Bom, Cmd("bill-of-materials")),
    b("Source Code", Icon::SourceCode, Off),
    b("Project Notes", Icon::Notes, Off),
    Sep,
    b("Help", Icon::Help, Off),
];

/// The View Toolbar.
pub const VIEW: &[Tool] = &[
    b("Redraw Display", Icon::Redraw, Cmd("redraw")),
    toggle("Toggle Grid", Icon::Grid, Cmd("toggle-grid"), Pressed::Grid),
    Sep,
    toggle("Toggle False Origin", Icon::FalseOrigin, Cmd("toggle-false-origin"), Pressed::FalseOrigin),
    Sep,
    b("Center At Cursor", Icon::Center, Cmd("center-at-cursor")),
    b("Zoom In", Icon::ZoomIn, Cmd("zoom-in")),
    b("Zoom Out", Icon::ZoomOut, Cmd("zoom-out")),
    b("Zoom To View Entire Sheet", Icon::ZoomAll, Cmd("zoom-all")),
    b("Zoom To Area", Icon::ZoomArea, Act::Ui(UiAct::ZoomArea)),
];

/// The Edit Toolbar.
pub const EDIT: &[Tool] = &[
    b("Undo Changes", Icon::Undo, Cmd("undo")),
    b("Redo Changes", Icon::Redo, Cmd("redo")),
    Sep,
    b("Cut To Clipboard", Icon::Cut, Cmd("cut")),
    b("Copy To Clipboard", Icon::Copy, Cmd("copy")),
    b("Paste From Clipboard", Icon::Paste, Cmd("paste")),
    Sep,
    b("Block Copy", Icon::BlockCopy, Cmd("block-copy")),
    b("Block Move", Icon::BlockMove, Cmd("block-move")),
    b("Block Rotate", Icon::BlockRotate, Cmd("block-rotate")),
    b("Block Delete", Icon::BlockDelete, Cmd("block-delete")),
    Sep,
    b("Pick Parts", Icon::PickParts, Cmd("pick-device")),
    b("Make Device", Icon::MakeDevice, Off),
    b("Packaging Tool", Icon::Packaging, Off),
    b("Decompose", Icon::Decompose, Off),
];

/// The Design Toolbar.
pub const DESIGN: &[Tool] = &[
    toggle("Wire Autorouter", Icon::Autorouter, Cmd("toggle-autorouter"), Pressed::Autorouter),
    Sep,
    b("Search & Tag", Icon::SearchTag, Cmd("search-tag")),
    b("Property Assignment Tool", Icon::PropertyTool, Cmd("property-assign")),
    Sep,
    b("New (Root) Sheet", Icon::NewSheet, Cmd("new-sheet")),
    b("Remove/Delete Sheet", Icon::RemoveSheet, Cmd("remove-sheet")),
    b("Exit to Parent Sheet", Icon::ExitParent, Cmd("exit-to-parent")),
    Sep,
    b("Electrical Rules Check", Icon::Erc, Cmd("erc")),
];

/// The toolbars in Proteus's order with their names.
pub const TOOLBARS: [(&str, &[Tool]); 4] = [("Global Toolbar", GLOBAL), ("View Toolbar", VIEW), ("Edit Toolbar", EDIT), ("Design Toolbar", DESIGN)];

/// Whether an action can run now.
pub fn enabled(s: &Session, act: Act) -> bool {
    match act {
        Act::Cmd(id) => find_command(id).is_some_and(|c| (c.enabled)(s).is_ok()),
        Act::Ui(_) => true,
        Act::None => false,
    }
}

fn pressed(s: &Session, p: Pressed) -> bool {
    match p {
        Pressed::Grid => s.view().grid,
        Pressed::FalseOrigin => s.view().false_origin.is_some(),
        Pressed::Autorouter => s.autorouter(),
        Pressed::Schematic => true,
    }
}

/// The width a toolbar takes, in points.
pub fn toolbar_width(tools: &[Tool]) -> f32 {
    let buttons = tools.iter().filter(|t| matches!(t, Tool::Button { .. })).count() as f32;
    let seps = tools.iter().filter(|t| matches!(t, Tool::Sep)).count() as f32;
    buttons * widgets::TOOL + seps * 7.0 + (buttons + seps - 1.0).max(0.0) * GAP + 8.0
}

/// Gap between buttons, in points.
const GAP: f32 = 1.0;
/// Gap between toolbars, in points.
const BETWEEN: f32 = 8.0;

/// The toolbars on each row of the band for a window `width` points wide: as many as fit, in
/// order (like Proteus's rebar, which wraps bands that do not fit).
pub fn rows(width: f32) -> Vec<Vec<usize>> {
    let mut rows: Vec<Vec<usize>> = vec![Vec::new()];
    let mut used = 0.0;
    for (i, (_, tools)) in TOOLBARS.iter().enumerate() {
        let w = toolbar_width(tools) + BETWEEN;
        if used + w > width && rows.last().is_some_and(|r| !r.is_empty()) {
            rows.push(Vec::new());
            used = 0.0;
        }
        if let Some(r) = rows.last_mut() {
            r.push(i);
        }
        used += w;
    }
    rows
}

/// Draws the toolbar band.
pub fn show(app: &mut App, ui: &mut Ui) {
    let t = Tokens::get();
    let mut clicked = None;
    let layout = rows(ui.ctx().content_rect().width() - 12.0);
    let height = HEIGHT * layout.len().max(1) as f32;
    egui::Panel::top("toolbars")
        .exact_size(height)
        .show_separator_line(false)
        .frame(Frame::NONE.fill(t.chrome).inner_margin(Margin::symmetric(6, 4)))
        .show(ui, |ui| {
            widgets::region(ui, "Toolbars", ui.max_rect());
            ui.spacing_mut().item_spacing.y = 6.0;
            for row in &layout {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    for (name, tools) in row.iter().filter_map(|&i| TOOLBARS.get(i)) {
                        toolbar(app, ui, name, tools, &mut clicked);
                        ui.add_space(BETWEEN);
                    }
                });
            }
        });
    match clicked {
        Some(Act::Cmd(id)) => app.invoke(id),
        Some(Act::Ui(u)) => app.ui_act(u),
        _ => {}
    }
}

fn toolbar(app: &App, ui: &mut Ui, name: &str, tools: &[Tool], clicked: &mut Option<Act>) {
    let t = Tokens::get();
    let group = Frame::NONE.fill(t.chrome_hi).stroke(Stroke::new(1.0, t.border)).corner_radius(6.0).inner_margin(Margin::symmetric(3, 1));
    let r = group.show(ui, |ui| {
        for tool in tools {
            match tool {
                Tool::Sep => widgets::separator(ui, true),
                Tool::Button { label, icon, act, pressed: p } => {
                    let look = ToolLook {
                        icon: *icon,
                        label: i18n::t(label),
                        enabled: enabled(&app.session, *act) || *p == Some(Pressed::Schematic),
                        selected: p.is_some_and(|p| pressed(&app.session, p)),
                    };
                    if widgets::tool(ui, look, "tool").clicked() {
                        *clicked = Some(*act);
                    }
                }
            }
        }
    });
    widgets::region(ui, name, r.response.rect);
}

/// Draws the tab strip with the Schematic Capture tab.
pub fn tab_strip(ui: &mut Ui) {
    let t = Tokens::get();
    egui::Panel::top("tabs")
        .exact_size(TABS_HEIGHT)
        .show_separator_line(false)
        .frame(Frame::NONE.fill(t.chrome).inner_margin(Margin { left: 8, right: 8, top: 4, bottom: 0 }))
        .show(ui, |ui| {
            widgets::region(ui, "Tabs", ui.max_rect());
            let label = tl!("Schematic Capture");
            let galley_w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), crate::theme::body(), t.text).size().x);
            let (rect, resp) = ui.allocate_exact_size(vec2(galley_w + 70.0, TABS_HEIGHT - 4.0), Sense::click());
            let r = egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 };
            ui.painter().rect(rect, r, t.canvas, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
            ui.painter().line_segment([rect.left_top() + vec2(6.0, 1.0), rect.right_top() + vec2(-6.0, 1.0)], Stroke::new(2.0, t.accent));
            let icon = egui::Rect::from_center_size(egui::pos2(rect.left() + 16.0, rect.center().y), vec2(16.0, 16.0));
            icons::paint(ui.painter(), icon, Icon::Schematic, IconStyle { ink: t.text, accent: t.accent, fill: t.chrome_hi });
            ui.painter().text(egui::pos2(rect.left() + 30.0, rect.center().y), Align2::LEFT_CENTER, label, crate::theme::body(), t.text);
            let close = egui::Rect::from_center_size(egui::pos2(rect.right() - 14.0, rect.center().y), vec2(12.0, 12.0));
            icons::paint(ui.painter(), close, Icon::Cross, IconStyle { ink: t.text_faint, accent: t.text_faint, fill: t.chrome });
            resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, true, label));
            control::record(ui, &resp, label, "tab", Some(true));
        });
}
