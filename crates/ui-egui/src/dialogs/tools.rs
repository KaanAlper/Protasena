//! Tool ▸ Search & Tag and Property Assignment Tool dialogs. Our own layouts (Proteus's were not
//! observable: fields, order and defaults are to compare).

use egui::Ui;
use serde_json::json;

use super::{Reply, error_line, field, ok_cancel};
use crate::{control, tl};

/// Search & Tag's operators: parameter and label.
pub fn operators() -> [(&'static str, &'static str); 9] {
    [
        ("equals", tl!("Equals")),
        ("not-equals", tl!("Not Equals")),
        ("begins", tl!("Begins")),
        ("not-begins", tl!("Not Begins")),
        ("ends", tl!("Ends")),
        ("contains", tl!("Contains")),
        ("not-contains", tl!("Not Contains")),
        ("less", tl!("Less Than")),
        ("greater", tl!("Greater Than")),
    ]
}

/// How a search combines with the parts already tagged: parameter and label.
fn modes() -> [(&'static str, &'static str); 3] {
    [("new", tl!("New Search")), ("and", tl!("AND (narrow)")), ("or", tl!("OR (widen)"))]
}

/// Search & Tag.
pub struct SearchTagForm {
    /// The property name.
    pub property: String,
    /// The value compared with.
    pub value: String,
    /// Index into [`operators`].
    pub op: usize,
    /// Index into the search modes.
    pub mode: usize,
    /// Match case.
    pub case: bool,
    /// The last refusal.
    pub error: Option<String>,
}

impl Default for SearchTagForm {
    fn default() -> Self {
        Self { property: "REF".into(), value: String::new(), op: 0, mode: 0, case: false, error: None }
    }
}

fn choice(ui: &mut Ui, id: &str, label: &str, items: &[(&str, &str)], at: &mut usize) {
    ui.label(label);
    let shown = items.get(*at).map_or("", |i| i.1);
    let r = egui::ComboBox::from_id_salt(id).selected_text(shown).show_ui(ui, |ui| {
        for (i, (_, text)) in items.iter().enumerate() {
            ui.selectable_value(at, i, *text);
        }
    });
    control::record(ui, &r.response, label, "combo", None);
    ui.end_row();
}

impl SearchTagForm {
    pub(super) fn ui(&mut self, ui: &mut Ui) -> Reply {
        let enter = egui::Grid::new("search-tag")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                let a = field(ui, tl!("Property"), &mut self.property, 200.0);
                choice(ui, "search-op", tl!("Operator"), &operators(), &mut self.op);
                let b = field(ui, tl!("String"), &mut self.value, 200.0);
                choice(ui, "search-mode", tl!("Search Mode"), &modes(), &mut self.mode);
                a || b
            })
            .inner;
        let c = ui.checkbox(&mut self.case, tl!("Match Case"));
        control::record(ui, &c, tl!("Match Case"), "checkbox", Some(self.case));
        error_line(ui, self.error.as_ref());
        let (ok, cancel) = ok_cancel(ui, !self.property.trim().is_empty());
        if cancel {
            return Reply::Cancel;
        }
        if (ok || enter) && !self.property.trim().is_empty() {
            let op = operators().get(self.op).map_or("equals", |o| o.0);
            let mode = modes().get(self.mode).map_or("new", |m| m.0);
            let params = json!({ "property": self.property.trim(), "value": self.value, "op": op, "mode": mode, "case": self.case });
            return Reply::Run { cmd: "search-tag", params, keep: false };
        }
        Reply::None
    }
}

/// The Property Assignment Tool's actions: parameter and label.
fn actions() -> [(&'static str, &'static str); 3] {
    [("assign", tl!("Assign")), ("rename", tl!("Rename")), ("remove", tl!("Remove"))]
}

/// What it applies to: parameter and label.
fn apply_to() -> [(&'static str, &'static str); 3] {
    [("tagged", tl!("Tagged Parts")), ("sheet", tl!("All Parts on the Sheet")), ("all", tl!("All Parts in the Design"))]
}

/// Property Assignment Tool.
pub struct PropertyAssignForm {
    /// `NAME=VALUE`, `OLD=NEW` or `NAME`.
    pub string: String,
    /// Index into the actions.
    pub action: usize,
    /// Index into what it applies to.
    pub apply_to: usize,
    /// Counter start for `#`.
    pub count: String,
    /// Counter step.
    pub increment: String,
    /// The last refusal.
    pub error: Option<String>,
}

impl PropertyAssignForm {
    /// The dialog; it applies to the tagged parts when there are any, else to the sheet.
    pub fn new(any_tagged: bool) -> Self {
        Self { string: String::new(), action: 0, apply_to: if any_tagged { 0 } else { 1 }, count: "1".into(), increment: "1".into(), error: None }
    }

    pub(super) fn ui(&mut self, ui: &mut Ui) -> Reply {
        ui.label(tl!("NAME=VALUE to assign or rename, NAME to remove; # counts up."));
        ui.add_space(6.0);
        let enter = egui::Grid::new("property-assign")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                let e = field(ui, tl!("String"), &mut self.string, 260.0);
                choice(ui, "assign-action", tl!("Action"), &actions(), &mut self.action);
                choice(ui, "assign-apply", tl!("Apply To"), &apply_to(), &mut self.apply_to);
                field(ui, tl!("Count"), &mut self.count, 80.0);
                field(ui, tl!("Increment"), &mut self.increment, 80.0);
                e
            })
            .inner;
        error_line(ui, self.error.as_ref());
        let (ok, cancel) = ok_cancel(ui, !self.string.trim().is_empty());
        if cancel {
            return Reply::Cancel;
        }
        if (ok || enter) && !self.string.trim().is_empty() {
            let (Ok(count), Ok(increment)) = (self.count.trim().parse::<i64>(), self.increment.trim().parse::<i64>()) else {
                self.error = Some(tl!("Count and Increment must be whole numbers").to_owned());
                return Reply::None;
            };
            let action = actions().get(self.action).map_or("assign", |a| a.0);
            let apply_to = apply_to().get(self.apply_to).map_or("tagged", |a| a.0);
            let params = json!({ "string": self.string.trim(), "action": action, "apply_to": apply_to, "count": count, "increment": increment });
            return Reply::Run { cmd: "property-assign", params, keep: false };
        }
        Reply::None
    }
}
