//! Result windows: the Electrical Rules Check report, the Netlist Compiler summary, the Bill of
//! Materials, the message log and About. They do not block the editing window.

use egui::{RichText, ScrollArea};
use serde_json::Value;

use crate::theme::Tokens;
use crate::{App, control, tl};

/// A result window's content.
#[derive(Clone, Debug, PartialEq)]
pub enum Results {
    /// The `erc` command's result.
    Erc(Value),
    /// The `export-netlist` command's result.
    Netlist(Value),
    /// The `bill-of-materials` command's result.
    Bom(Value),
    /// The session's messages.
    Messages,
    /// About Protasena.
    About,
}

impl Results {
    /// The window title.
    pub fn title(&self) -> &'static str {
        match self {
            Results::Erc(_) => tl!("Electrical Rules Check"),
            Results::Netlist(_) => tl!("Netlist Compiler"),
            Results::Bom(_) => tl!("Bill of Materials"),
            Results::Messages => tl!("Messages"),
            Results::About => tl!("About Protasena"),
        }
    }
}

/// Shows the result window, if one is open.
pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(r) = app.results.clone() else { return };
    let t = Tokens::get();
    let mut open = true;
    let mut export = None;
    let title = r.title();
    let w = egui::Window::new(title)
        .id(egui::Id::new("results"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size(if matches!(r, Results::Bom(_)) { [760.0, 420.0] } else { [460.0, 320.0] })
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .show(ctx, |ui| match &r {
            Results::Erc(v) => erc(ui, v, &t),
            Results::Netlist(v) => {
                ui.label(format!("{}: {}", tl!("File"), v["path"].as_str().unwrap_or_default()));
                ui.label(format!("{}: {}", tl!("Nets"), v["nets"]));
                ui.label(format!("{}: {}", tl!("Parts"), v["parts"]));
                ui.label(format!("{}: {}", tl!("Rules check findings"), v["erc"]));
            }
            Results::Bom(v) => bom(ui, v, &t, &mut export),
            Results::Messages => {
                ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                    if app.session.log().is_empty() {
                        ui.label(RichText::new(tl!("No Messages")).color(t.text_dim));
                    }
                    for line in app.session.log() {
                        ui.label(line);
                    }
                });
            }
            Results::About => {
                ui.label(RichText::new("Protasena").size(18.0).strong());
                ui.label(format!("{} {}", tl!("Version"), env!("CARGO_PKG_VERSION")));
                ui.label(tl!("Open-source schematic capture whose layout and workflow follow Proteus 8 Schematic Capture."));
                ui.label(tl!("Not affiliated with Labcenter Electronics. Proteus is a trademark of Labcenter Electronics."));
                ui.label(tl!("Licence: MIT OR Apache-2.0"));
            }
        });
    if let Some(w) = w {
        control::record_rect(ctx, egui::Id::new("results-area"), w.response.rect, title, "window", true, None);
    }
    if !open {
        app.results = None;
    }
    if let Some(format) = export {
        app.choose_save(crate::dialogs::PathPurpose::Bom(format));
    }
}

/// The Bill of Materials: the parts table and its export buttons.
fn bom(ui: &mut egui::Ui, v: &Value, t: &Tokens, export: &mut Option<&'static str>) {
    let lines = v["lines"].as_array().cloned().unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{} {}, {} {}", v["total"], tl!("parts"), lines.len(), tl!("lines"))).strong());
        for (label, format) in [(tl!("Export CSV..."), "csv"), (tl!("Export HTML..."), "html")] {
            let b = ui.button(label);
            control::record(ui, &b, label, "button", None);
            if b.clicked() {
                *export = Some(format);
            }
        }
    });
    ui.separator();
    if lines.is_empty() {
        ui.label(RichText::new(tl!("The design has no parts.")).color(t.text_dim));
        return;
    }
    ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("bom").num_columns(6).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
            for h in [tl!("Quantity"), tl!("References"), tl!("Value"), tl!("Device"), tl!("Package"), tl!("Description")] {
                ui.label(RichText::new(h).strong());
            }
            ui.end_row();
            for line in lines.iter().take(5000) {
                ui.label(line["quantity"].to_string());
                let refs: Vec<&str> = line["references"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
                ui.label(refs.join(", "));
                for key in ["value", "device", "package", "description"] {
                    ui.label(line[key].as_str().unwrap_or_default());
                }
                ui.end_row();
            }
        });
    });
}

fn erc(ui: &mut egui::Ui, v: &Value, t: &Tokens) {
    let findings = v["erc"].as_array().cloned().unwrap_or_default();
    let head = if findings.is_empty() { tl!("No errors found.").to_owned() } else { format!("{} {}", findings.len(), tl!("findings")) };
    ui.label(RichText::new(head).strong());
    ui.label(format!("{}: {}   {}: {}", tl!("Nets"), v["nets"], tl!("Parts"), v["parts"]));
    ui.separator();
    ScrollArea::vertical().show(ui, |ui| {
        for f in findings.iter().take(2000) {
            let warn = f["severity"].as_str() == Some("warning");
            let mark = if warn { tl!("Warning") } else { tl!("Error") };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(mark).color(if warn { t.warn } else { t.missing }).strong());
                ui.label(f["message"].as_str().unwrap_or_default());
            });
        }
    });
}
