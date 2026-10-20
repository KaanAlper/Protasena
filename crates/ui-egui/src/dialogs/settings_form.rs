//! The Template and System menus' settings dialogs, as one form over a settings group's JSON:
//! Set Design Colours, Set Graphic Styles, Set Text Styles, Set 2D Graphics Defaults, Set
//! Junction Dot Style, Set Display Options, Set Text Editor and System Settings. OK sends the
//! whole group to its command; a value left empty (or a colour not marked as the design's own)
//! is Protasena's default. Our own layouts: Proteus's dialogs were not observable.

use std::collections::HashMap;

use egui::Ui;
use protasena_engine::Session;
use serde_json::Value;

use super::{Reply, error_line, ok_cancel};
use crate::{control, tl};

/// What a cell edits.
#[derive(Clone, Copy, Debug)]
pub enum Kind {
    /// A colour `#rrggbb`, or null for our default.
    Colour,
    /// A number, or null (empty) for the default.
    Number,
    /// A checkbox.
    Bool,
    /// A text, or null (empty).
    Text,
    /// One of these values, with their labels.
    Choice(fn() -> [(&'static str, &'static str); 3]),
}

/// One edited value: its path in the group's JSON.
#[derive(Clone, Copy, Debug)]
pub struct Cell {
    /// Keys from the group down to the value.
    pub path: &'static [&'static str],
    /// What it is.
    pub kind: Kind,
}

/// A labelled row of cells (`None`: an empty cell).
pub struct Row {
    /// The row's label.
    pub label: &'static str,
    /// Its cells, under the form's column headers.
    pub cells: Vec<Option<Cell>>,
}

/// A settings dialog.
pub struct SettingsForm {
    /// The command OK runs.
    pub cmd: &'static str,
    /// The dialog's title.
    pub title: &'static str,
    /// Column headers after the label column.
    pub header: Vec<&'static str>,
    /// The rows.
    pub rows: Vec<Row>,
    /// The group's values as edited.
    pub value: Value,
    /// What is typed in the number and text fields, by path.
    pub typed: HashMap<String, String>,
    /// The last refusal.
    pub error: Option<String>,
}

const fn c(path: &'static [&'static str], kind: Kind) -> Option<Cell> {
    Some(Cell { path, kind })
}

fn shapes() -> [(&'static str, &'static str); 3] {
    [("circle", tl!("Circle")), ("square", tl!("Square")), ("diamond", tl!("Diamond"))]
}

fn get<'v>(v: &'v Value, path: &[&str]) -> Option<&'v Value> {
    path.iter().try_fold(v, |v, k| v.get(k))
}

fn set(v: &mut Value, path: &[&str], new: Value) {
    let Some((last, parents)) = path.split_last() else { return };
    let slot = parents.iter().try_fold(v, |v, k| v.get_mut(k));
    if let Some(Value::Object(o)) = slot {
        o.insert((*last).to_owned(), new);
    }
}

fn style_row(label: &'static str, group: &'static str, fill: bool) -> Row {
    let paths: &'static [[&'static str; 2]] = match group {
        "component" => &[["component", "width"], ["component", "colour"], ["component", "fill"]],
        "pin" => &[["pin", "width"], ["pin", "colour"], ["pin", "fill"]],
        "wire" => &[["wire", "width"], ["wire", "colour"], ["wire", "fill"]],
        "bus" => &[["bus", "width"], ["bus", "colour"], ["bus", "fill"]],
        "terminal" => &[["terminal", "width"], ["terminal", "colour"], ["terminal", "fill"]],
        _ => &[["subcircuit", "width"], ["subcircuit", "colour"], ["subcircuit", "fill"]],
    };
    let cell = |i: usize, kind| paths.get(i).map(|p| Cell { path: p.as_slice(), kind });
    Row { label, cells: vec![cell(0, Kind::Number), cell(1, Kind::Colour), if fill { cell(2, Kind::Colour) } else { None }] }
}

fn text_row(label: &'static str, path: &'static [[&'static str; 2]; 2]) -> Row {
    Row { label, cells: vec![c(path[0].as_slice(), Kind::Number), c(path[1].as_slice(), Kind::Colour)] }
}

impl SettingsForm {
    /// The dialog for a Template menu group (`colours`, `graphic_styles`, `text_styles`,
    /// `graphics_2d`, `junction`) or a System menu group (`folders`, `display`,
    /// `text_editor`).
    pub fn new(s: &Session, section: &str) -> Option<Self> {
        let t = &s.project().template;
        let st = s.settings();
        let json = |v: Result<Value, serde_json::Error>| v.unwrap_or(Value::Null);
        let (cmd, title, header, rows, value) = match section {
            "colours" => (
                "set-design-colours",
                tl!("Set Design Colours"),
                vec![tl!("Colour")],
                vec![
                    Row { label: tl!("Paper"), cells: vec![c(&["paper"], Kind::Colour)] },
                    Row { label: tl!("Grid"), cells: vec![c(&["grid"], Kind::Colour)] },
                    Row { label: tl!("Highlight"), cells: vec![c(&["highlight"], Kind::Colour)] },
                    Row { label: tl!("Drag"), cells: vec![c(&["drag"], Kind::Colour)] },
                ],
                json(serde_json::to_value(t.colours)),
            ),
            "graphic_styles" => (
                "set-graphic-styles",
                tl!("Set Graphic Styles"),
                vec![tl!("Width (th)"), tl!("Colour"), tl!("Fill")],
                vec![
                    style_row(tl!("Component"), "component", true),
                    style_row(tl!("Pin"), "pin", false),
                    style_row(tl!("Wire"), "wire", false),
                    style_row(tl!("Bus"), "bus", false),
                    style_row(tl!("Terminal"), "terminal", false),
                    style_row(tl!("Subcircuit"), "subcircuit", true),
                ],
                json(serde_json::to_value(t.graphic_styles)),
            ),
            "text_styles" => (
                "set-text-styles",
                tl!("Set Text Styles"),
                vec![tl!("Height (th)"), tl!("Colour")],
                vec![
                    text_row(tl!("Reference"), &[["reference", "height"], ["reference", "colour"]]),
                    text_row(tl!("Value"), &[["value", "height"], ["value", "colour"]]),
                    text_row(tl!("Pin Name"), &[["pin_name", "height"], ["pin_name", "colour"]]),
                    text_row(tl!("Pin Number"), &[["pin_number", "height"], ["pin_number", "colour"]]),
                    text_row(tl!("Wire Label"), &[["label", "height"], ["label", "colour"]]),
                    text_row(tl!("Terminal"), &[["terminal", "height"], ["terminal", "colour"]]),
                    text_row(tl!("Text"), &[["text", "height"], ["text", "colour"]]),
                    text_row(tl!("Subcircuit"), &[["subcircuit", "height"], ["subcircuit", "colour"]]),
                ],
                json(serde_json::to_value(t.text_styles)),
            ),
            "graphics_2d" => (
                "set-2d-graphics-defaults",
                tl!("Set 2D Graphics Defaults"),
                vec![""],
                vec![
                    Row { label: tl!("Line Width (th)"), cells: vec![c(&["width"], Kind::Number)] },
                    Row { label: tl!("Line Colour"), cells: vec![c(&["colour"], Kind::Colour)] },
                    Row { label: tl!("Fill"), cells: vec![c(&["fill"], Kind::Colour)] },
                ],
                json(serde_json::to_value(t.graphics_2d)),
            ),
            "junction" => (
                "set-junction-style",
                tl!("Set Junction Dot Style"),
                vec![""],
                vec![
                    Row { label: tl!("Size (th)"), cells: vec![c(&["size"], Kind::Number)] },
                    Row { label: tl!("Shape"), cells: vec![c(&["shape"], Kind::Choice(shapes))] },
                ],
                json(serde_json::to_value(t.junction)),
            ),
            "display" => (
                "set-display-options",
                tl!("Set Display Options"),
                vec![""],
                vec![
                    Row { label: tl!("Grid as lines"), cells: vec![c(&["grid_lines"], Kind::Bool)] },
                    Row { label: tl!("Wheel zoom factor"), cells: vec![c(&["wheel_zoom"], Kind::Number)] },
                    Row { label: tl!("Pick distance (pixels)"), cells: vec![c(&["pick_pixels"], Kind::Number)] },
                ],
                json(serde_json::to_value(st.display)),
            ),
            "text_editor" => (
                "set-text-editor",
                tl!("Set Text Editor"),
                vec![""],
                vec![
                    Row { label: tl!("Font size (points)"), cells: vec![c(&["font_size"], Kind::Number)] },
                    Row { label: tl!("Fixed-width font"), cells: vec![c(&["monospace"], Kind::Bool)] },
                ],
                json(serde_json::to_value(st.text_editor)),
            ),
            "folders" => (
                "system-settings",
                tl!("System Settings"),
                vec![""],
                vec![
                    Row { label: tl!("Library folder"), cells: vec![c(&["library_dir"], Kind::Text)] },
                    Row { label: tl!("Project folder"), cells: vec![c(&["project_dir"], Kind::Text)] },
                ],
                serde_json::json!({ "library_dir": st.library_dir, "project_dir": st.project_dir }),
            ),
            _ => return None,
        };
        let mut typed = HashMap::new();
        for cell in rows.iter().flat_map(|r| r.cells.iter().flatten()) {
            if matches!(cell.kind, Kind::Number | Kind::Text) {
                let shown = match get(&value, cell.path) {
                    Some(Value::Number(n)) => n.to_string(),
                    Some(Value::String(s)) => s.clone(),
                    _ => String::new(),
                };
                typed.insert(cell.path.join("."), shown);
            }
        }
        Some(Self { cmd, title, header, rows, value, typed, error: None })
    }

    /// The group's JSON with what is typed, or why it cannot be sent.
    pub fn params(&self) -> Result<Value, String> {
        let mut v = self.value.clone();
        for cell in self.rows.iter().flat_map(|r| r.cells.iter().flatten()) {
            let typed = self.typed.get(&cell.path.join(".")).map(|t| t.trim()).unwrap_or_default();
            let new = match cell.kind {
                Kind::Number if typed.is_empty() => Value::Null,
                Kind::Number => match typed.parse::<f64>() {
                    Ok(n) if n.is_finite() => serde_json::json!(n),
                    _ => return Err(format!("{typed:?} {}", tl!("is not a number"))),
                },
                Kind::Text if typed.is_empty() => Value::Null,
                Kind::Text => Value::String(typed.to_owned()),
                Kind::Colour | Kind::Bool | Kind::Choice(_) => continue,
            };
            set(&mut v, cell.path, new);
        }
        Ok(v)
    }

    pub(super) fn ui(&mut self, ui: &mut Ui) -> Reply {
        let cols = self.header.len() + 1;
        egui::Grid::new(("settings", self.title)).num_columns(cols).spacing([14.0, 8.0]).show(ui, |ui| {
            if self.header.iter().any(|h| !h.is_empty()) {
                ui.label("");
                for h in &self.header {
                    ui.label(egui::RichText::new(*h).strong());
                }
                ui.end_row();
            }
            let rows: Vec<(&'static str, Vec<Option<Cell>>)> = self.rows.iter().map(|r| (r.label, r.cells.clone())).collect();
            for (label, cells) in rows {
                ui.label(label);
                for cell in &cells {
                    match cell {
                        Some(cell) => self.cell(ui, label, *cell),
                        None => {
                            ui.label("");
                        }
                    }
                }
                ui.end_row();
            }
        });
        error_line(ui, self.error.as_ref());
        let (ok, cancel) = ok_cancel(ui, true);
        if cancel {
            return Reply::Cancel;
        }
        if ok {
            match self.params() {
                Ok(params) => return Reply::Run { cmd: self.cmd, params, keep: false },
                Err(e) => self.error = Some(e),
            }
        }
        Reply::None
    }

    fn cell(&mut self, ui: &mut Ui, row: &str, cell: Cell) {
        let name = format!("{row} {}", cell.path.last().copied().unwrap_or_default());
        match cell.kind {
            Kind::Number | Kind::Text => {
                let text = self.typed.entry(cell.path.join(".")).or_default();
                let width = if matches!(cell.kind, Kind::Text) { 280.0 } else { 70.0 };
                let r = ui.add(egui::TextEdit::singleline(text).desired_width(width).hint_text(tl!("default")));
                control::record(ui, &r, &name, "field", None);
            }
            Kind::Bool => {
                let mut on = get(&self.value, cell.path).and_then(Value::as_bool).unwrap_or(false);
                let r = ui.checkbox(&mut on, "");
                control::record(ui, &r, &name, "checkbox", Some(on));
                if r.changed() {
                    set(&mut self.value, cell.path, Value::Bool(on));
                }
            }
            Kind::Choice(options) => {
                let options = options();
                let current = get(&self.value, cell.path).and_then(Value::as_str).unwrap_or_default().to_owned();
                let shown = options.iter().find(|o| o.0 == current).map_or(current.as_str(), |o| o.1);
                let mut pick = None;
                let r = egui::ComboBox::from_id_salt(("choice", cell.path)).selected_text(shown).show_ui(ui, |ui| {
                    for (v, label) in options {
                        if ui.selectable_label(current == v, label).clicked() {
                            pick = Some(v);
                        }
                    }
                });
                control::record(ui, &r.response, &name, "combo", None);
                if let Some(v) = pick {
                    set(&mut self.value, cell.path, Value::String(v.to_owned()));
                }
            }
            Kind::Colour => self.colour(ui, &name, cell.path),
        }
    }

    /// A colour cell: "Own" on uses the colour button's colour, off is our default.
    fn colour(&mut self, ui: &mut Ui, name: &str, path: &'static [&'static str]) {
        let current = get(&self.value, path).and_then(Value::as_str).and_then(protasena_doc::Colour::parse);
        ui.horizontal(|ui| {
            let mut own = current.is_some();
            let r = ui.checkbox(&mut own, tl!("Own"));
            control::record(ui, &r, name, "checkbox", Some(own));
            let shown = current.unwrap_or(protasena_doc::Colour([128, 128, 128]));
            let mut rgb = shown.0;
            let b = ui.add_enabled_ui(own, |ui| ui.color_edit_button_srgb(&mut rgb)).inner;
            control::record(ui, &b, &format!("{name} colour"), "button", None);
            let new = match (own, current) {
                (false, _) => None,
                (true, _) if rgb != shown.0 => Some(protasena_doc::Colour(rgb)),
                (true, c) => c.or(Some(shown)),
            };
            if new != current {
                set(&mut self.value, path, new.map_or(Value::Null, |c| Value::String(c.to_string())));
            }
        });
    }
}
