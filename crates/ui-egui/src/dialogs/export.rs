//! File ▸ Export Graphics and Print Design dialogs: what to draw (the current sheet or every
//! sheet), the options of the format (resolution for bitmaps, scale for vector files, mono)
//! and the file. Our own layout (Proteus's export and print dialogs are to compare).

use egui::Ui;
use protasena_engine::Session;
use serde_json::{Value, json};

use super::{Reply, error_line, field, ok_cancel};
use crate::{control, tl};

/// An Export Graphics item: its command, title and the formats it writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportKind {
    /// The engine command.
    pub cmd: &'static str,
    /// The dialog title (the menu item).
    pub title: &'static str,
    /// The formats offered, the default first.
    pub formats: &'static [&'static str],
}

/// The Export Graphics items by the format the engine asks for.
pub fn export_kind(format: &str) -> Option<ExportKind> {
    let k = |cmd, title, formats| Some(ExportKind { cmd, title, formats });
    match format {
        "png" | "bmp" => k("export-bitmap", tl!("Export Bitmap"), &["png", "bmp"]),
        "dxf" => k("export-dxf", tl!("Export DXF File"), &["dxf"]),
        "eps" => k("export-eps", tl!("Export Encapsulated Postscript"), &["eps"]),
        "pdf" => k("export-pdf", tl!("Export Adobe PDF File"), &["pdf"]),
        "svg" => k("export-svg", tl!("Export SVG File"), &["svg"]),
        _ => None,
    }
}

/// The file suggested for an export: the project file with the format's extension, else the
/// project name in the working directory.
pub fn suggested_path(s: &Session, ext: &str) -> String {
    match s.path() {
        Some(p) => p.with_extension(ext).display().to_string(),
        None => format!("{}.{ext}", safe_name(&s.project().name)),
    }
}

/// A project name usable as a file name.
fn safe_name(name: &str) -> String {
    let n: String = name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).take(64).collect();
    if n.is_empty() { "UNTITLED".to_owned() } else { n }
}

/// The options both dialogs share.
pub struct Output {
    /// Every sheet instead of the current one.
    pub all_sheets: bool,
    /// Black on white.
    pub mono: bool,
    /// The file.
    pub path: String,
    /// The Browse button was pressed (the window then asks its file chooser).
    pub browse: bool,
    /// The last refusal.
    pub error: Option<String>,
}

impl Output {
    fn new(path: String, all_sheets: bool) -> Self {
        Self { all_sheets, mono: false, path, browse: false, error: None }
    }

    fn scope(&mut self, ui: &mut Ui, pages: bool) {
        ui.label(tl!("Scope"));
        let a = ui.radio_value(&mut self.all_sheets, false, tl!("Current Sheet"));
        control::record(ui, &a, tl!("Current Sheet"), "radio", Some(!self.all_sheets));
        let b = ui.add_enabled(pages, egui::RadioButton::new(self.all_sheets, tl!("All Sheets")));
        if b.clicked() {
            self.all_sheets = true;
        }
        control::record(ui, &b, tl!("All Sheets"), "radio", Some(self.all_sheets));
        ui.add_space(6.0);
        let m = ui.checkbox(&mut self.mono, tl!("Mono (black on white)"));
        control::record(ui, &m, tl!("Mono (black on white)"), "checkbox", Some(self.mono));
    }

    fn file(&mut self, ui: &mut Ui) -> bool {
        ui.add_space(6.0);
        let enter = ui
            .horizontal(|ui| {
                let enter = egui::Grid::new("output-file")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| field(ui, tl!("File"), &mut self.path, 320.0))
                    .inner;
                let b = ui.button(tl!("Browse..."));
                control::record(ui, &b, tl!("Browse..."), "button", None);
                self.browse |= b.clicked();
                enter
            })
            .inner;
        error_line(ui, self.error.as_ref());
        enter
    }
}

/// Export Graphics.
pub struct ExportForm {
    /// Which item.
    pub kind: ExportKind,
    /// The format chosen.
    pub format: &'static str,
    /// Bitmap resolution typed, in dpi.
    pub dpi: String,
    /// Vector scale typed.
    pub scale: String,
    /// Scope, mono, file.
    pub out: Output,
}

impl ExportForm {
    /// The dialog for an item, with the suggested file.
    pub fn new(s: &Session, kind: ExportKind) -> Self {
        let format = kind.formats.first().copied().unwrap_or("svg");
        Self { kind, format, dpi: "200".into(), scale: "1".into(), out: Output::new(suggested_path(s, format), false) }
    }

    pub(super) fn ui(&mut self, ui: &mut Ui) -> Reply {
        if self.kind.formats.len() > 1 {
            ui.label(tl!("Format"));
            ui.horizontal(|ui| {
                for f in self.kind.formats {
                    let label = f.to_ascii_uppercase();
                    let r = ui.radio(self.format == *f, &label);
                    if r.clicked() && self.format != *f {
                        self.out.path = swap_extension(&self.out.path, f);
                        self.format = *f;
                    }
                    control::record(ui, &r, &label, "radio", Some(self.format == *f));
                }
            });
            ui.add_space(6.0);
        }
        let pdf = self.format == "pdf";
        self.out.scope(ui, pdf);
        ui.add_space(6.0);
        let bitmap = matches!(self.format, "png" | "bmp");
        egui::Grid::new("export-options").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
            if bitmap {
                field(ui, tl!("Resolution (dpi)"), &mut self.dpi, 80.0);
            } else {
                field(ui, tl!("Scale"), &mut self.scale, 80.0);
            }
        });
        let enter = self.out.file(ui);
        let (ok, cancel) = ok_cancel(ui, !self.out.path.trim().is_empty());
        if cancel {
            return Reply::Cancel;
        }
        if (ok || enter) && !self.out.path.trim().is_empty() {
            let mut params =
                json!({ "path": self.out.path.trim(), "format": self.format, "mono": self.out.mono, "all_sheets": pdf && self.out.all_sheets });
            let (key, text) = if bitmap { ("dpi", &self.dpi) } else { ("scale", &self.scale) };
            match text.trim().parse::<f64>() {
                Ok(v) if v.is_finite() => insert(&mut params, key, json!(v)),
                _ => {
                    self.out.error = Some(tl!("Enter a number").to_owned());
                    return Reply::None;
                }
            }
            return Reply::Run { cmd: self.kind.cmd, params, keep: false };
        }
        Reply::None
    }
}

/// Print Design: printed to a PDF file, which the window may open in the system viewer.
pub struct PrintForm {
    /// Scope, mono, file.
    pub out: Output,
    /// Open the PDF in the system viewer afterwards.
    pub view: bool,
    /// Whether the app can open files in the system viewer.
    pub can_view: bool,
}

impl PrintForm {
    /// The dialog, printing every sheet to a file in the system's temporary directory.
    pub fn new(s: &Session, can_view: bool) -> Self {
        let path = std::env::temp_dir().join(format!("{}-print.pdf", safe_name(&s.project().name))).display().to_string();
        Self { out: Output::new(path, true), view: can_view, can_view }
    }

    pub(super) fn ui(&mut self, ui: &mut Ui) -> Reply {
        ui.label(tl!("Prints to a PDF file."));
        ui.add_space(6.0);
        self.out.scope(ui, true);
        let v = ui.add_enabled(self.can_view, egui::Checkbox::new(&mut self.view, tl!("Open in the system viewer")));
        control::record(ui, &v, tl!("Open in the system viewer"), "checkbox", Some(self.view));
        let enter = self.out.file(ui);
        let (ok, cancel) = ok_cancel(ui, !self.out.path.trim().is_empty());
        if cancel {
            return Reply::Cancel;
        }
        if (ok || enter) && !self.out.path.trim().is_empty() {
            let params =
                json!({ "path": self.out.path.trim(), "all_sheets": self.out.all_sheets, "mono": self.out.mono, "view": self.view && self.can_view });
            return Reply::Run { cmd: "print-design", params, keep: false };
        }
        Reply::None
    }
}

fn insert(params: &mut Value, key: &str, v: Value) {
    if let Some(o) = params.as_object_mut() {
        o.insert(key.to_owned(), v);
    }
}

/// `path` with its extension replaced by `ext`.
fn swap_extension(path: &str, ext: &str) -> String {
    std::path::Path::new(path).with_extension(ext).display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggested_files_follow_the_project() {
        let s = Session::new();
        assert_eq!(suggested_path(&s, "svg"), "UNTITLED.svg");
        assert_eq!(safe_name("a/b c"), "a_b_c");
        assert_eq!(safe_name(""), "UNTITLED");
        assert_eq!(swap_extension("x/y.png", "bmp"), "x/y.bmp");
        assert_eq!(export_kind("bmp").map(|k| k.cmd), Some("export-bitmap"));
        assert_eq!(export_kind("wmf"), None);
    }
}
