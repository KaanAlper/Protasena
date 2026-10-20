//! System menu dialogs with lists: Set Keyboard Mapping (every command's key, conflicts shown as
//! they are typed), Set Property Definitions, Set Sheet Sizes, and the Restore Default Settings
//! question. Our own layouts: Proteus's dialogs were not observable.

use std::collections::HashMap;

use egui::{RichText, Ui};
use protasena_engine::{Session, command_specs, keymap::normalize_key};
use serde_json::{Value, json};

use super::{Reply, error_line, ok_cancel};
use crate::theme::Tokens;
use crate::{control, i18n, tl};

/// One command of Set Keyboard Mapping.
pub struct KeyRow {
    /// Command id.
    pub id: &'static str,
    /// Where it is (`Edit > Undo Changes`) or its name.
    pub label: String,
    /// Its default key.
    pub default: Option<String>,
    /// The key typed (empty: none).
    pub key: String,
}

/// Set Keyboard Mapping.
pub struct KeymapForm {
    /// Every command with a menu item or a key.
    pub rows: Vec<KeyRow>,
    /// Shows only the rows containing this.
    pub filter: String,
    /// The last refusal.
    pub error: Option<String>,
}

/// What is wrong with the typed keys: per row index, `Some(message)`.
fn problems(rows: &[KeyRow]) -> HashMap<usize, String> {
    let mut by_key: HashMap<String, Vec<usize>> = HashMap::new();
    let mut out = HashMap::new();
    for (i, r) in rows.iter().enumerate() {
        if r.key.trim().is_empty() {
            continue;
        }
        match normalize_key(&r.key) {
            Some(k) if k == "Escape" => {
                out.insert(i, tl!("Escape always cancels").to_owned());
            }
            Some(k) => by_key.entry(k).or_default().push(i),
            None => {
                out.insert(i, tl!("not a key").to_owned());
            }
        }
    }
    for idx in by_key.values().filter(|v| v.len() > 1) {
        for &i in idx {
            let others: Vec<&str> = idx.iter().filter(|&&j| j != i).filter_map(|&j| rows.get(j)).map(|r| r.label.as_str()).collect();
            out.insert(i, format!("{} {}", tl!("also"), others.join(", ")));
        }
    }
    out
}

impl KeymapForm {
    /// The form with the keys in use.
    pub fn new(s: &Session) -> Self {
        let rows = command_specs()
            .iter()
            .filter(|c| !c.menu.is_empty() || c.shortcut.is_some())
            .map(|c| {
                let label =
                    if c.menu.is_empty() { i18n::t(c.label).to_owned() } else { c.menu.iter().map(|m| i18n::t(m)).collect::<Vec<_>>().join(" > ") };
                KeyRow { id: c.id, label, default: c.shortcut.and_then(normalize_key), key: s.shortcut(c.id).unwrap_or_default() }
            })
            .collect();
        Self { rows, filter: String::new(), error: None }
    }

    /// The `set-keyboard-mapping` bindings: every row (its default as null).
    pub fn bindings(&self) -> Value {
        let map: serde_json::Map<String, Value> = self
            .rows
            .iter()
            .map(|r| {
                let typed = normalize_key(&r.key);
                let v = match (&typed, &r.default) {
                    (Some(k), Some(d)) if k == d => Value::Null,
                    (None, None) => Value::Null,
                    (Some(k), _) => Value::String(k.clone()),
                    (None, Some(_)) => Value::String(String::new()),
                };
                (r.id.to_owned(), v)
            })
            .collect();
        json!({ "bindings": map })
    }

    pub(super) fn ui(&mut self, ui: &mut Ui) -> Reply {
        let t = Tokens::get();
        ui.horizontal(|ui| {
            ui.label(tl!("Find"));
            let r = ui.add(egui::TextEdit::singleline(&mut self.filter).desired_width(220.0));
            control::record(ui, &r, tl!("Find"), "field", None);
            let d = ui.button(tl!("Restore Defaults"));
            control::record(ui, &d, tl!("Restore Defaults"), "button", None);
            if d.clicked() {
                for r in &mut self.rows {
                    r.key = r.default.clone().unwrap_or_default();
                }
            }
        });
        let bad = problems(&self.rows);
        let filter = self.filter.trim().to_lowercase();
        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
            egui::Grid::new("keymap").num_columns(3).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
                for (i, r) in self.rows.iter_mut().enumerate() {
                    if !filter.is_empty() && !r.label.to_lowercase().contains(&filter) && !r.key.to_lowercase().contains(&filter) {
                        continue;
                    }
                    ui.label(&r.label);
                    let e = ui.add_sized([110.0, 20.0], egui::TextEdit::singleline(&mut r.key).hint_text(tl!("none")));
                    control::record(ui, &e, &r.label, "field", None);
                    match bad.get(&i) {
                        Some(why) => ui.label(RichText::new(why).color(t.warn)),
                        None => ui.label(""),
                    };
                    ui.end_row();
                }
            });
        });
        if !bad.is_empty() {
            ui.label(RichText::new(tl!("Give each key to one command only.")).color(t.warn));
        }
        error_line(ui, self.error.as_ref());
        let (ok, cancel) = ok_cancel(ui, bad.is_empty());
        if cancel {
            return Reply::Cancel;
        }
        if ok && bad.is_empty() {
            return Reply::Run { cmd: "set-keyboard-mapping", params: self.bindings(), keep: false };
        }
        Reply::None
    }
}

/// A list dialog: Set Property Definitions (name, default, description) or Set Sheet Sizes
/// (name, width, height in thou), rows of text fields.
pub struct ListForm {
    /// `set-property-definitions` or `set-sheet-sizes`.
    pub cmd: &'static str,
    /// The rows as typed.
    pub rows: Vec<[String; 3]>,
    /// The last refusal.
    pub error: Option<String>,
}

impl ListForm {
    /// Set Property Definitions with the definitions in use.
    pub fn properties(s: &Session) -> Self {
        let rows = s.settings().properties.iter().map(|p| [p.name.clone(), p.default.clone(), p.description.clone()]).collect();
        Self { cmd: "set-property-definitions", rows, error: None }
    }

    /// Set Sheet Sizes with the sizes in use.
    pub fn sheet_sizes(s: &Session) -> Self {
        let rows = s.settings().sheet_sizes.iter().map(|z| [z.name.clone(), z.width.to_string(), z.height.to_string()]).collect();
        Self { cmd: "set-sheet-sizes", rows, error: None }
    }

    fn headers(&self) -> [&'static str; 3] {
        if self.cmd == "set-sheet-sizes" {
            [tl!("Name"), tl!("Width (th)"), tl!("Height (th)")]
        } else {
            [tl!("Name"), tl!("Default"), tl!("Description")]
        }
    }

    /// The command's parameters, or why the rows cannot be sent.
    pub fn params(&self) -> Result<Value, String> {
        if self.cmd == "set-sheet-sizes" {
            let sizes = self
                .rows
                .iter()
                .map(|[name, w, h]| match (w.trim().parse::<f64>(), h.trim().parse::<f64>()) {
                    (Ok(width), Ok(height)) => Ok(json!({ "name": name.trim(), "width": width, "height": height })),
                    _ => Err(format!("{}: {}", name.trim(), tl!("width and height must be numbers"))),
                })
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(json!({ "sizes": sizes }));
        }
        let defs: Vec<Value> = self.rows.iter().map(|[n, d, x]| json!({ "name": n.trim(), "default": d, "description": x })).collect();
        Ok(json!({ "definitions": defs }))
    }

    pub(super) fn ui(&mut self, ui: &mut Ui) -> Reply {
        let headers = self.headers();
        let mut remove = None;
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            egui::Grid::new(("list", self.cmd)).num_columns(4).spacing([8.0, 6.0]).show(ui, |ui| {
                for h in headers {
                    ui.label(RichText::new(h).strong());
                }
                ui.label("");
                ui.end_row();
                for (i, row) in self.rows.iter_mut().enumerate() {
                    for (j, cell) in row.iter_mut().enumerate() {
                        let r = ui.add_sized([if j == 0 { 150.0 } else { 110.0 }, 22.0], egui::TextEdit::singleline(cell));
                        control::record(ui, &r, &format!("{} {}", headers.get(j).copied().unwrap_or_default(), i + 1), "field", None);
                    }
                    let x = ui.button(tl!("Remove"));
                    control::record(ui, &x, &format!("{} {}", tl!("Remove"), i + 1), "button", None);
                    if x.clicked() {
                        remove = Some(i);
                    }
                    ui.end_row();
                }
            });
        });
        if let Some(i) = remove {
            self.rows.remove(i);
        }
        ui.horizontal(|ui| {
            let a = ui.button(tl!("Add"));
            control::record(ui, &a, tl!("Add"), "button", None);
            if a.clicked() {
                self.rows.push(if self.cmd == "set-sheet-sizes" { [String::new(), "10000".into(), "8000".into()] } else { Default::default() });
            }
            if self.cmd == "set-sheet-sizes" {
                let d = ui.button(tl!("Restore Defaults"));
                control::record(ui, &d, tl!("Restore Defaults"), "button", None);
                if d.clicked() {
                    self.rows =
                        protasena_engine::default_sheet_sizes().into_iter().map(|z| [z.name, z.width.to_string(), z.height.to_string()]).collect();
                }
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
}

/// Restore Default Settings: asks first.
pub(super) fn restore(ui: &mut Ui) -> Reply {
    ui.label(tl!("Put every system setting (folders, display, keys, property definitions, sheet sizes, text editor) back to Protasena's defaults?"));
    let (ok, cancel) = ok_cancel(ui, true);
    if cancel {
        return Reply::Cancel;
    }
    if ok {
        return Reply::Run { cmd: "restore-default-settings", params: json!({}), keep: false };
    }
    Reply::None
}
