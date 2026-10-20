//! A device's pin alternate functions (KiCad's `alternate`), as Pick Devices' preview and Edit
//! Component list them: one line per pin that has any.

use egui::{RichText, Ui};
use protasena_library::{Device, PinKind};

use crate::theme::Tokens;
use crate::{control, tl};

/// Most lines listed (a large MCU has hundreds of pins with alternates).
const MAX_LINES: usize = 500;

/// One pin with alternate functions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlternateRow {
    /// Pin number.
    pub number: String,
    /// The pin's own (primary) name.
    pub name: String,
    /// `NAME (type)` of each alternate function.
    pub alternates: Vec<String>,
}

/// The pins of `d` with alternate functions, in symbol order, each pin number once.
pub fn alternate_rows(d: &Device) -> Vec<AlternateRow> {
    let mut seen = std::collections::HashSet::new();
    d.symbol
        .pins()
        .filter(|p| !p.alternates.is_empty() && seen.insert(p.number.as_str()))
        .map(|p| AlternateRow {
            number: p.number.clone(),
            name: p.name.clone(),
            alternates: p.alternates.iter().map(|a| format!("{} ({})", a.name, kind_label(a.kind))).collect(),
        })
        .collect()
}

/// The electrical type's name as lists show it.
pub fn kind_label(k: PinKind) -> &'static str {
    match k {
        PinKind::Input => tl!("input"),
        PinKind::Output => tl!("output"),
        PinKind::Bidirectional => tl!("bidirectional"),
        PinKind::TriState => tl!("tri-state"),
        PinKind::Passive => tl!("passive"),
        PinKind::PowerIn => tl!("power input"),
        PinKind::PowerOut => tl!("power output"),
        PinKind::OpenCollector => tl!("open collector"),
        PinKind::OpenEmitter => tl!("open emitter"),
        PinKind::Unspecified => tl!("unspecified"),
        PinKind::NoConnect => tl!("not connected"),
    }
}

/// A collapsible "Pin alternates" list (nothing when there are none).
pub fn show(ui: &mut Ui, rows: &[AlternateRow], id: &str, height: f32) {
    if rows.is_empty() {
        return;
    }
    let t = Tokens::get();
    let title = format!("{} ({})", tl!("Pin alternates"), rows.len());
    let r = egui::CollapsingHeader::new(RichText::new(&title).strong()).id_salt(("alternates", id)).show(ui, |ui| {
        egui::ScrollArea::vertical().id_salt(("alternates-list", id)).max_height(height).show(ui, |ui| {
            for row in rows.iter().take(MAX_LINES) {
                let pin = if row.name.is_empty() { row.number.clone() } else { format!("{} {}", row.number, row.name) };
                ui.label(RichText::new(format!("{pin}: {}", row.alternates.join(", "))).size(12.0).color(t.text_dim));
            }
            if rows.len() > MAX_LINES {
                ui.label(RichText::new(format!("… {}", rows.len() - MAX_LINES)).size(12.0).color(t.text_faint));
            }
        });
    });
    control::record(ui, &r.header_response, tl!("Pin alternates"), "button", None);
}
