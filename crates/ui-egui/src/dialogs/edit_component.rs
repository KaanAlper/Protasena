//! Edit Component: part reference and value (each with a Hidden box), the gate of a multi-gate
//! part, the PCB package and the further properties as a grid. OK runs `edit-component`.

use egui::{RichText, Ui};
use protasena_doc::ComponentId;
use protasena_engine::Session;
use serde_json::{Map, Value, json};

use super::{Reply, error_line, ok_cancel};
use crate::theme::Tokens;
use crate::{control, tl};

/// One row of the properties grid.
#[derive(Clone, Debug, PartialEq)]
pub struct PropRow {
    /// Property name (`TOLERANCE`).
    pub name: String,
    /// What the device calls it (`Tolerance`), or the name.
    pub prompt: String,
    /// The value; empty removes the property.
    pub value: String,
}

/// The dialog's state.
pub struct EditComponent {
    /// The part.
    pub id: ComponentId,
    /// Its device's name (for the caption).
    pub device: String,
    /// Part Reference.
    pub reference: String,
    /// Part Value.
    pub value: String,
    /// Hide the reference.
    pub hide_reference: bool,
    /// Hide the value.
    pub hide_value: bool,
    /// The chosen package (`None`: no package).
    pub package: Option<String>,
    /// The device's packages.
    pub packages: Vec<String>,
    /// The gate.
    pub unit: u32,
    /// How many gates the device has.
    pub gates: usize,
    /// Further properties.
    pub props: Vec<PropRow>,
    /// The last refusal.
    pub error: Option<String>,
}

impl EditComponent {
    /// The dialog for part `id`, filled from the design and its device.
    pub fn new(s: &Session, id: ComponentId) -> Option<Self> {
        let c = s.project().component(id)?;
        let device = protasena_render::find_device(s.library(), c);
        let mut props: Vec<PropRow> = Vec::new();
        for d in device.iter().flat_map(|d| d.prop_defs.iter()).filter(|p| !p.hidden) {
            let skip = ["VALUE", "PACKAGE"].iter().any(|k| d.name.eq_ignore_ascii_case(k));
            if !skip {
                let prompt = if d.prompt.is_empty() { d.name.clone() } else { d.prompt.clone() };
                props.push(PropRow { name: d.name.clone(), prompt, value: c.properties.get(&d.name).unwrap_or_default().to_owned() });
            }
        }
        for (name, value) in c.properties.iter() {
            if !props.iter().any(|p| p.name.eq_ignore_ascii_case(name)) {
                props.push(PropRow { name: name.to_owned(), prompt: name.to_owned(), value: value.to_owned() });
            }
        }
        // System ▸ Set Property Definitions: the user's own properties, with their defaults.
        for d in &s.settings().properties {
            if !props.iter().any(|p| p.name.eq_ignore_ascii_case(&d.name)) {
                let prompt = if d.description.trim().is_empty() { d.name.clone() } else { format!("{} ({})", d.name, d.description.trim()) };
                props.push(PropRow { name: d.name.clone(), prompt, value: d.default.clone() });
            }
        }
        Some(Self {
            id,
            device: c.device.name.clone(),
            reference: c.reference.clone(),
            value: c.value.clone(),
            hide_reference: !c.reference_label.visible,
            hide_value: !c.value_label.visible,
            package: c.package.clone(),
            packages: device.map(|d| d.packages.iter().map(|p| p.name.clone()).collect()).unwrap_or_default(),
            unit: c.unit,
            gates: device.map_or(1, |d| d.symbol.gate_count().max(1)),
            props,
            error: None,
        })
    }

    /// The `edit-component` parameters for what the dialog holds.
    pub fn params(&self) -> Value {
        let mut props = Map::new();
        for p in &self.props {
            let v = p.value.trim();
            props.insert(p.name.clone(), if v.is_empty() { Value::Null } else { json!(v) });
        }
        json!({
            "component": self.id.0,
            "reference": self.reference.trim(),
            "value": self.value,
            "package": self.package,
            "unit": self.unit,
            "properties": props,
            "show_reference": !self.hide_reference,
            "show_value": !self.hide_value,
        })
    }

    pub(super) fn ui(&mut self, ui: &mut Ui, _s: &Session) -> Reply {
        let t = Tokens::get();
        ui.label(RichText::new(format!("{} {}", tl!("Device"), self.device)).color(t.text_dim));
        ui.add_space(4.0);
        egui::Grid::new("edit-component").num_columns(3).spacing([12.0, 8.0]).show(ui, |ui| {
            text_row(ui, tl!("Part Reference"), &mut self.reference, Some(&mut self.hide_reference));
            text_row(ui, tl!("Part Value"), &mut self.value, Some(&mut self.hide_value));
            if self.gates > 1 {
                ui.label(tl!("Element"));
                let letter = |u: u32| char::from_u32(u32::from('A').saturating_add(u)).map_or_else(|| u.to_string(), |c| c.to_string());
                egui::ComboBox::from_id_salt("gate").selected_text(letter(self.unit)).show_ui(ui, |ui| {
                    for g in 0..u32::try_from(self.gates.min(64)).unwrap_or(1) {
                        ui.selectable_value(&mut self.unit, g, letter(g));
                    }
                });
                ui.end_row();
            }
            ui.label(tl!("PCB Package"));
            let shown = self.package.clone().unwrap_or_else(|| tl!("(none)").to_owned());
            egui::ComboBox::from_id_salt("package").width(220.0).selected_text(shown).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.package, None, tl!("(none)"));
                for p in &self.packages {
                    ui.selectable_value(&mut self.package, Some(p.clone()), p);
                }
            });
            ui.end_row();
        });
        if !self.props.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new(tl!("Other Properties")).strong());
            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                egui::Grid::new("component-properties").num_columns(2).striped(true).spacing([12.0, 6.0]).show(ui, |ui| {
                    for p in &mut self.props {
                        text_row(ui, &p.prompt, &mut p.value, None);
                    }
                });
            });
        }
        error_line(ui, self.error.as_ref());
        let (ok, cancel) = ok_cancel(ui, !self.reference.trim().is_empty() || self.reference.is_empty());
        if cancel {
            return Reply::Cancel;
        }
        if ok {
            return Reply::Run { cmd: "edit-component", params: self.params(), keep: false };
        }
        Reply::None
    }
}

fn text_row(ui: &mut Ui, label: &str, value: &mut String, hidden: Option<&mut bool>) {
    ui.label(label);
    let r = ui.add(egui::TextEdit::singleline(value).desired_width(220.0));
    control::record(ui, &r, label, "field", None);
    if let Some(h) = hidden {
        let c = ui.checkbox(h, tl!("Hidden"));
        control::record(ui, &c, &format!("{label} {}", tl!("Hidden")), "checkbox", Some(*h));
    }
    ui.end_row();
}
