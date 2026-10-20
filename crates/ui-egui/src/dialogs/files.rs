//! Dialogs about files and the design as a whole: a path prompt (when the app has no native
//! file chooser), Edit Design Properties and Edit Sheet Properties.

use egui::Ui;
use protasena_doc::{SheetId, SheetSize};
use protasena_engine::Session;
use serde_json::json;

use super::{Reply, error_line, field, ok_cancel};
use crate::tl;

/// What a path is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathPurpose {
    /// Open Project (`discard`: the unsaved changes were already given up).
    Open {
        /// Discard unsaved changes.
        discard: bool,
    },
    /// Save Project As.
    SaveAs,
    /// Netlist Compiler output.
    Netlist,
    /// Bill of Materials export in this format (`csv` or `html`).
    Bom(&'static str),
}

impl PathPurpose {
    /// The dialog title.
    pub fn title(self) -> &'static str {
        match self {
            PathPurpose::Open { .. } => tl!("Open Project"),
            PathPurpose::SaveAs => tl!("Save Project As"),
            PathPurpose::Netlist => tl!("Netlist Compiler"),
            PathPurpose::Bom(_) => tl!("Bill of Materials"),
        }
    }
}

/// The netlist format a file name implies: SPICE for `.cir`, `.spi`, `.spice`, `.net`; else JSON.
pub fn netlist_format(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if [".cir", ".spi", ".spice", ".net"].iter().any(|e| lower.ends_with(e)) { "spice" } else { "json" }
}

/// A file path prompt.
pub struct PathForm {
    /// What for.
    pub purpose: PathPurpose,
    /// The path typed.
    pub path: String,
    /// Netlist: SPICE instead of JSON.
    pub spice: bool,
    /// The last refusal.
    pub error: Option<String>,
}

impl PathForm {
    /// The prompt, with a suggested path.
    pub fn new(purpose: PathPurpose, suggested: &str) -> Self {
        Self { purpose, path: suggested.to_owned(), spice: false, error: None }
    }

    pub(super) fn ui(&mut self, ui: &mut Ui) -> Reply {
        let enter = egui::Grid::new("path").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| field(ui, tl!("File"), &mut self.path, 380.0)).inner;
        if self.purpose == PathPurpose::Netlist {
            ui.checkbox(&mut self.spice, tl!("SPICE netlist"));
        }
        error_line(ui, self.error.as_ref());
        let (ok, cancel) = ok_cancel(ui, !self.path.trim().is_empty());
        if cancel {
            return Reply::Cancel;
        }
        if (ok || enter) && !self.path.trim().is_empty() {
            let path = self.path.trim();
            return match self.purpose {
                PathPurpose::Open { discard } => Reply::Run { cmd: "open", params: json!({ "path": path, "discard": discard }), keep: false },
                PathPurpose::SaveAs => Reply::Run { cmd: "save-as", params: json!({ "path": path }), keep: false },
                PathPurpose::Netlist => {
                    let format = if self.spice { "spice" } else { netlist_format(path) };
                    Reply::Run { cmd: "export-netlist", params: json!({ "path": path, "format": format }), keep: false }
                }
                PathPurpose::Bom(format) => Reply::Run { cmd: "bill-of-materials", params: json!({ "path": path, "format": format }), keep: false },
            };
        }
        Reply::None
    }
}

/// The fields of Edit Design Properties, in order: key and label.
fn design_fields() -> [(&'static str, &'static str); 7] {
    [
        ("name", tl!("Project Name")),
        ("title", tl!("Design Title")),
        ("doc_number", tl!("Doc. No")),
        ("revision", tl!("Revision")),
        ("author", tl!("Author")),
        ("created", tl!("Created")),
        ("modified", tl!("Modified")),
    ]
}

/// Edit Design Properties.
pub struct DesignForm {
    /// Values in the order of the dialog's fields.
    pub values: [String; 7],
    /// The last refusal.
    pub error: Option<String>,
}

impl DesignForm {
    /// The form filled from the project.
    pub fn new(s: &Session) -> Self {
        let p = s.project();
        let d = &p.design;
        Self {
            values: [
                p.name.clone(),
                d.title.clone(),
                d.doc_number.clone(),
                d.revision.clone(),
                d.author.clone(),
                d.created.clone(),
                d.modified.clone(),
            ],
            error: None,
        }
    }

    pub(super) fn ui(&mut self, ui: &mut Ui) -> Reply {
        egui::Grid::new("design-properties").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
            for ((_, label), value) in design_fields().iter().zip(self.values.iter_mut()) {
                field(ui, label, value, 300.0);
            }
        });
        error_line(ui, self.error.as_ref());
        let (ok, cancel) = ok_cancel(ui, true);
        if cancel {
            return Reply::Cancel;
        }
        if ok {
            let mut params = serde_json::Map::new();
            for ((key, _), value) in design_fields().iter().zip(&self.values) {
                params.insert((*key).to_owned(), json!(value));
            }
            return Reply::Run { cmd: "design-properties", params: params.into(), keep: false };
        }
        Reply::None
    }
}

/// The named sheet sizes, with their parameter names (other sizes are sent as custom sizes).
const NAMED: [(SheetSize, &str); 10] = [
    (SheetSize::A4, "a4"),
    (SheetSize::A3, "a3"),
    (SheetSize::A2, "a2"),
    (SheetSize::A1, "a1"),
    (SheetSize::A0, "a0"),
    (SheetSize::A, "a"),
    (SheetSize::B, "b"),
    (SheetSize::C, "c"),
    (SheetSize::D, "d"),
    (SheetSize::E, "e"),
];

/// The `size` parameter for a width and height in thou.
fn size_param(width: f64, height: f64) -> serde_json::Value {
    match NAMED.iter().find(|(z, _)| z.dimensions() == (width, height)) {
        Some((_, id)) => json!(id),
        None => json!({ "custom": { "width": width, "height": height } }),
    }
}

/// Edit Sheet Properties: the sizes offered are System ▸ Set Sheet Sizes'.
pub struct SheetForm {
    /// The sheet.
    pub sheet: SheetId,
    /// Its name.
    pub name: String,
    /// The sizes offered: name, width and height in thou.
    pub sizes: Vec<(String, f64, f64)>,
    /// Index into the sizes (`None`: the sheet's own size, kept).
    pub size: Option<usize>,
    /// The last refusal.
    pub error: Option<String>,
}

impl SheetForm {
    /// The form for `sheet`.
    pub fn new(s: &Session, sheet: SheetId) -> Option<Self> {
        let sh = s.project().sheet(sheet)?;
        let sizes: Vec<(String, f64, f64)> = s.settings().sheet_sizes.iter().map(|z| (z.name.clone(), z.width, z.height)).collect();
        let (w, h) = sh.size.dimensions();
        let size = sizes.iter().position(|z| (z.1 - w).abs() < 0.5 && (z.2 - h).abs() < 0.5);
        Some(Self { sheet, name: sh.name.clone(), sizes, size, error: None })
    }

    pub(super) fn ui(&mut self, ui: &mut Ui) -> Reply {
        egui::Grid::new("sheet-properties").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
            field(ui, tl!("Sheet Name"), &mut self.name, 260.0);
            ui.label(tl!("Sheet Size"));
            let shown = self.size.and_then(|i| self.sizes.get(i)).map_or(tl!("Custom").to_owned(), |z| z.0.clone());
            egui::ComboBox::from_id_salt("sheet-size").selected_text(shown).show_ui(ui, |ui| {
                for (i, z) in self.sizes.iter().enumerate() {
                    ui.selectable_value(&mut self.size, Some(i), &z.0);
                }
            });
            ui.end_row();
        });
        error_line(ui, self.error.as_ref());
        let (ok, cancel) = ok_cancel(ui, !self.name.trim().is_empty());
        if cancel {
            return Reply::Cancel;
        }
        if ok {
            let mut params = json!({ "sheet": self.sheet.0, "name": self.name.trim() });
            if let (Some(z), Some(o)) = (self.size.and_then(|i| self.sizes.get(i)), params.as_object_mut()) {
                o.insert("size".into(), size_param(z.1, z.2));
            }
            return Reply::Run { cmd: "sheet-properties", params, keep: false };
        }
        Reply::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netlist_formats_follow_the_extension() {
        assert_eq!(netlist_format("a/b.CIR"), "spice");
        assert_eq!(netlist_format("x.json"), "json");
        assert_eq!(netlist_format("x"), "json");
    }

    #[test]
    fn named_sizes_keep_their_names_and_others_are_custom() {
        assert_eq!(size_param(11_693.0, 8_268.0), json!("a4"));
        assert_eq!(size_param(20_000.0, 5_000.0), json!({ "custom": { "width": 20_000.0, "height": 5_000.0 } }));
    }
}
