//! Subcircuit dialogs: naming a new port, and Edit Subcircuit (its name and ports, and the way
//! to its child sheet). Our own layouts: Proteus's were not observable.

use egui::Ui;
use protasena_doc::{PortKind, SubcircuitId};
use protasena_engine::{Session, port_kind_id};
use protasena_geom::Pt;
use serde_json::json;

use super::{Reply, error_line, field, ok_cancel};
use crate::{control, tl};

/// The port kinds in the PORTS list's order, with their names.
pub const PORT_KINDS: [(PortKind, &str); 4] =
    [(PortKind::Default, "DEFAULT"), (PortKind::Input, "INPUT"), (PortKind::Output, "OUTPUT"), (PortKind::Bidir, "BIDIR")];

fn kind_name(k: PortKind) -> &'static str {
    PORT_KINDS.iter().find(|p| p.0 == k).map_or("DEFAULT", |p| p.1)
}

fn kind_choice(ui: &mut Ui, salt: egui::Id, kind: &mut PortKind) {
    let r = egui::ComboBox::from_id_salt(salt).selected_text(kind_name(*kind)).show_ui(ui, |ui| {
        for (k, name) in PORT_KINDS {
            ui.selectable_value(kind, k, name);
        }
    });
    control::record(ui, &r.response, tl!("Type"), "combo", None);
}

/// A new port on a subcircuit's edge: its name and kind.
pub struct PortForm {
    /// The subcircuit.
    pub subcircuit: SubcircuitId,
    /// The point on its edge.
    pub at: Pt,
    /// The name typed.
    pub name: String,
    /// The kind (the PORTS list's choice to start with).
    pub kind: PortKind,
    /// The last refusal.
    pub error: Option<String>,
}

impl PortForm {
    /// The form for a port at `at` on `subcircuit`.
    pub fn new(s: &Session, subcircuit: SubcircuitId, at: Pt) -> Self {
        Self { subcircuit, at, name: String::new(), kind: s.port_kind(), error: None }
    }

    pub(super) fn ui(&mut self, ui: &mut Ui) -> Reply {
        let enter = egui::Grid::new("port")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                let e = field(ui, tl!("Port Name"), &mut self.name, 220.0);
                ui.label(tl!("Type"));
                kind_choice(ui, egui::Id::new("port-kind"), &mut self.kind);
                ui.end_row();
                e
            })
            .inner;
        error_line(ui, self.error.as_ref());
        let (ok, cancel) = ok_cancel(ui, !self.name.trim().is_empty());
        if cancel {
            return Reply::Cancel;
        }
        if (ok || enter) && !self.name.trim().is_empty() {
            let params =
                json!({ "subcircuit": self.subcircuit.0, "at": [self.at.x, self.at.y], "name": self.name.trim(), "kind": port_kind_id(self.kind) });
            return Reply::Run { cmd: "subcircuit-port", params, keep: false };
        }
        Reply::None
    }
}

/// One port row of Edit Subcircuit.
pub struct PortRow {
    /// Name.
    pub name: String,
    /// Kind.
    pub kind: PortKind,
    /// Its connection point (kept).
    pub at: Pt,
}

/// Edit Subcircuit.
pub struct SubcircuitForm {
    /// The subcircuit.
    pub id: SubcircuitId,
    /// Its name.
    pub name: String,
    /// Its ports.
    pub ports: Vec<PortRow>,
    /// Whether it has a child sheet already.
    pub has_child: bool,
    /// The last refusal.
    pub error: Option<String>,
}

impl SubcircuitForm {
    /// The form for subcircuit `id`.
    pub fn new(s: &Session, id: SubcircuitId) -> Option<Self> {
        let (_, x) = s.project().subcircuit(id)?;
        let ports = x.ports.iter().map(|p| PortRow { name: p.name.clone(), kind: p.kind, at: p.at }).collect();
        let has_child = x.child.is_some_and(|c| s.project().sheet(c).is_some());
        Some(Self { id, name: x.name.clone(), ports, has_child, error: None })
    }

    fn params(&self) -> serde_json::Value {
        let ports: Vec<_> =
            self.ports.iter().map(|p| json!({ "name": p.name.trim(), "kind": port_kind_id(p.kind), "at": [p.at.x, p.at.y] })).collect();
        json!({ "id": self.id.0, "name": self.name.trim(), "ports": ports })
    }

    pub(super) fn ui(&mut self, ui: &mut Ui) -> Reply {
        egui::Grid::new("subcircuit").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| field(ui, tl!("Name"), &mut self.name, 220.0));
        ui.add_space(6.0);
        ui.label(tl!("Ports"));
        let mut remove = None;
        egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
            egui::Grid::new("subcircuit-ports").num_columns(3).spacing([8.0, 6.0]).show(ui, |ui| {
                for (i, p) in self.ports.iter_mut().enumerate() {
                    let r = ui.add_sized([160.0, 22.0], egui::TextEdit::singleline(&mut p.name));
                    control::record(ui, &r, &format!("{} {}", tl!("Port"), i + 1), "field", None);
                    kind_choice(ui, egui::Id::new(("port-row", i)), &mut p.kind);
                    let x = ui.button(tl!("Remove"));
                    control::record(ui, &x, &format!("{} {}", tl!("Remove"), i + 1), "button", None);
                    if x.clicked() {
                        remove = Some(i);
                    }
                    ui.end_row();
                }
            });
        });
        if self.ports.is_empty() {
            ui.label(tl!("Click on the left or right edge in Subcircuit mode to add a port."));
        }
        if let Some(i) = remove {
            self.ports.remove(i);
        }
        ui.add_space(6.0);
        let label = if self.has_child { tl!("Goto Child Sheet") } else { tl!("Create Child Sheet") };
        let child = ui.button(label);
        control::record(ui, &child, label, "button", None);
        error_line(ui, self.error.as_ref());
        let names_ok = !self.name.trim().is_empty() && self.ports.iter().all(|p| !p.name.trim().is_empty());
        let (ok, cancel) = ok_cancel(ui, names_ok);
        if cancel {
            return Reply::Cancel;
        }
        if child.clicked() {
            return Reply::Run { cmd: "enter-child-sheet", params: json!({ "id": self.id.0 }), keep: false };
        }
        if ok && names_ok {
            return Reply::Run { cmd: "edit-subcircuit", params: self.params(), keep: false };
        }
        Reply::None
    }
}
