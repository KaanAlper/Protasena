//! Small dialogs: Find/Edit Component's reference prompt, Edit Wire Label, Edit Terminal, text,
//! Goto Sheet, Global Annotator and the unsaved-changes question.

use egui::Ui;
use protasena_doc::{Id, LabelId, TerminalId, TerminalKind, TextId};
use protasena_engine::Session;
use protasena_geom::Pt;
use serde_json::json;

use super::{Dialog, Reply, error_line, field, ok_cancel};
use crate::{control, tl};

/// Find/Edit Component: the reference of the part to edit.
#[derive(Default)]
pub struct FindForm {
    /// The reference typed.
    pub reference: String,
    /// The last refusal.
    pub error: Option<String>,
}

impl FindForm {
    pub(super) fn ui(&mut self, ui: &mut Ui, s: &Session) -> Reply {
        let enter = egui::Grid::new("find-component")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| field(ui, tl!("Part Reference"), &mut self.reference, 200.0))
            .inner;
        error_line(ui, self.error.as_ref());
        let (ok, cancel) = ok_cancel(ui, !self.reference.trim().is_empty());
        if cancel {
            return Reply::Cancel;
        }
        if ok || enter {
            let wanted = self.reference.trim();
            let found = s.sheet().and_then(|sh| sh.components.iter().find(|c| c.reference.eq_ignore_ascii_case(wanted)).map(|c| c.id));
            return match found.and_then(|id| super::edit_component::EditComponent::new(s, id)) {
                Some(d) => Reply::Then(Dialog::EditComponent(Box::new(d))),
                None => {
                    self.error = Some(format!("{} {wanted}", tl!("No part on this sheet is called")));
                    Reply::None
                }
            };
        }
        Reply::None
    }
}

/// Where a label or text goes: a new one at a point, or an existing one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target<I> {
    /// A new one at this point.
    New(Pt),
    /// An existing one.
    Edit(I),
}

/// Edit Wire Label.
pub struct LabelForm {
    /// New or existing.
    pub target: Target<LabelId>,
    /// The label text (net name).
    pub text: String,
    /// The last refusal.
    pub error: Option<String>,
}

impl LabelForm {
    /// A new label on the wire at `at`.
    pub fn new_at(at: Pt) -> Self {
        Self { target: Target::New(at), text: String::new(), error: None }
    }

    /// An existing label.
    pub fn edit(s: &Session, id: LabelId) -> Option<Self> {
        let l = s.sheet()?.labels.iter().find(|l| l.id == id)?;
        Some(Self { target: Target::Edit(id), text: l.text.clone(), error: None })
    }

    pub(super) fn ui(&mut self, ui: &mut Ui) -> Reply {
        let enter =
            egui::Grid::new("wire-label").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| field(ui, tl!("String"), &mut self.text, 260.0)).inner;
        error_line(ui, self.error.as_ref());
        let (ok, cancel) = ok_cancel(ui, !self.text.trim().is_empty());
        if cancel {
            return Reply::Cancel;
        }
        if (ok || enter) && !self.text.trim().is_empty() {
            let text = self.text.trim();
            return match self.target {
                Target::New(at) => Reply::Run { cmd: "wire-label", params: json!({ "at": [at.x, at.y], "text": text }), keep: false },
                Target::Edit(id) => Reply::Run { cmd: "edit-item", params: json!({ "id": id.raw(), "text": text }), keep: false },
            };
        }
        Reply::None
    }
}

/// The terminal kinds in the TERMINALS list's order, with their ids and names.
pub const TERMINAL_KINDS: [(TerminalKind, &str, &str); 7] = [
    (TerminalKind::Default, "default", "DEFAULT"),
    (TerminalKind::Input, "input", "INPUT"),
    (TerminalKind::Output, "output", "OUTPUT"),
    (TerminalKind::Bidir, "bidir", "BIDIR"),
    (TerminalKind::Power, "power", "POWER"),
    (TerminalKind::Ground, "ground", "GROUND"),
    (TerminalKind::Bus, "bus", "BUS"),
];

/// Edit Terminal.
pub struct TerminalForm {
    /// The terminal.
    pub id: TerminalId,
    /// Its net name.
    pub name: String,
    /// Its kind.
    pub kind: TerminalKind,
    /// The last refusal.
    pub error: Option<String>,
}

impl TerminalForm {
    /// The form for terminal `id`.
    pub fn new(s: &Session, id: TerminalId) -> Option<Self> {
        let t = s.sheet()?.terminals.iter().find(|t| t.id == id)?;
        Some(Self { id, name: t.name.clone(), kind: t.kind, error: None })
    }

    pub(super) fn ui(&mut self, ui: &mut Ui) -> Reply {
        let enter = egui::Grid::new("terminal").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
            let enter = field(ui, tl!("String"), &mut self.name, 220.0);
            ui.label(tl!("Type"));
            let shown = TERMINAL_KINDS.iter().find(|k| k.0 == self.kind).map_or("DEFAULT", |k| k.2);
            egui::ComboBox::from_id_salt("terminal-kind").selected_text(shown).show_ui(ui, |ui| {
                for (kind, _, name) in TERMINAL_KINDS {
                    ui.selectable_value(&mut self.kind, kind, name);
                }
            });
            ui.end_row();
            enter
        });
        error_line(ui, self.error.as_ref());
        let (ok, cancel) = ok_cancel(ui, true);
        if cancel {
            return Reply::Cancel;
        }
        if ok || enter.inner {
            let kind = TERMINAL_KINDS.iter().find(|k| k.0 == self.kind).map_or("default", |k| k.1);
            return Reply::Run { cmd: "edit-item", params: json!({ "id": self.id.raw(), "name": self.name.trim(), "kind": kind }), keep: false };
        }
        Reply::None
    }
}

/// A text or text script.
pub struct TextForm {
    /// New or existing.
    pub target: Target<TextId>,
    /// The text.
    pub text: String,
    /// Character height in thou, as typed (empty: the default).
    pub size: String,
    /// The last refusal.
    pub error: Option<String>,
}

impl TextForm {
    /// A new text at `at`.
    pub fn new_at(at: Pt) -> Self {
        Self { target: Target::New(at), text: String::new(), size: String::new(), error: None }
    }

    /// An existing text.
    pub fn edit(s: &Session, id: TextId) -> Option<Self> {
        let t = s.sheet()?.texts.iter().find(|t| t.id == id)?;
        let size = if t.size > 0.0 { format!("{}", t.size) } else { String::new() };
        Some(Self { target: Target::Edit(id), text: t.text.clone(), size, error: None })
    }

    pub(super) fn ui(&mut self, ui: &mut Ui) -> Reply {
        ui.label(tl!("Text"));
        let r = ui.add(egui::TextEdit::multiline(&mut self.text).desired_rows(4).desired_width(340.0));
        control::record(ui, &r, tl!("Text"), "field", None);
        egui::Grid::new("text-size").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| field(ui, tl!("Height (th)"), &mut self.size, 80.0));
        error_line(ui, self.error.as_ref());
        let size: Option<f64> = self.size.trim().parse().ok().filter(|v: &f64| v.is_finite() && *v > 0.0);
        let size_ok = self.size.trim().is_empty() || size.is_some();
        let (ok, cancel) = ok_cancel(ui, !self.text.trim().is_empty() && size_ok);
        if cancel {
            return Reply::Cancel;
        }
        if ok {
            let mut params = match self.target {
                Target::New(at) => json!({ "at": [at.x, at.y], "text": self.text }),
                Target::Edit(id) => json!({ "id": id.raw(), "text": self.text }),
            };
            if let (Some(v), Some(o)) = (size, params.as_object_mut()) {
                o.insert("size".into(), json!(v));
            }
            let cmd = if matches!(self.target, Target::New(_)) { "text" } else { "edit-item" };
            return Reply::Run { cmd, params, keep: false };
        }
        Reply::None
    }
}

/// Goto Sheet: the sheet to show.
pub struct GotoForm {
    /// The chosen sheet's raw id.
    pub choice: u64,
}

impl GotoForm {
    /// The form, on the current sheet.
    pub fn new(s: &Session) -> Self {
        Self { choice: s.sheet_id().0 }
    }

    pub(super) fn ui(&mut self, ui: &mut Ui, s: &Session) -> Reply {
        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            for (i, sheet) in s.project().sheets.iter().enumerate() {
                let r = ui.selectable_value(&mut self.choice, sheet.id.0, format!("{}. {}", i + 1, sheet.name));
                control::record(ui, &r, &sheet.name, "list-item", Some(self.choice == sheet.id.0));
            }
        });
        let (ok, cancel) = ok_cancel(ui, true);
        if cancel {
            return Reply::Cancel;
        }
        if ok {
            return Reply::Run { cmd: "goto-sheet", params: json!({ "sheet": self.choice }), keep: false };
        }
        Reply::None
    }
}

/// Global Annotator: incremental or total.
pub(super) fn annotator(ui: &mut Ui, total: &mut bool) -> Reply {
    ui.label(tl!("Mode"));
    let a = ui.radio_value(total, false, tl!("Incremental"));
    control::record(ui, &a, tl!("Incremental"), "radio", Some(!*total));
    let b = ui.radio_value(total, true, tl!("Total"));
    control::record(ui, &b, tl!("Total"), "radio", Some(*total));
    let (ok, cancel) = ok_cancel(ui, true);
    if cancel {
        return Reply::Cancel;
    }
    if ok {
        return Reply::Run { cmd: "annotate", params: json!({ "mode": if *total { "total" } else { "incremental" } }), keep: false };
    }
    Reply::None
}

/// The unsaved-changes question.
pub struct ConfirmForm;

impl ConfirmForm {
    pub(super) fn ui(ui: &mut Ui, then: &mut &'static str) -> Reply {
        ui.label(tl!("The design has unsaved changes. Discard them?"));
        let (ok, cancel) = ok_cancel(ui, true);
        if cancel {
            return Reply::Cancel;
        }
        if ok {
            return Reply::Discard(then);
        }
        Reply::None
    }
}
