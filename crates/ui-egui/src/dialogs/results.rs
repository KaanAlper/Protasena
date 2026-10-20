//! Result windows: the Electrical Rules Check report, the Netlist Compiler summary, the message
//! log and About. They do not block the editing window.

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
    let title = r.title();
    let w = egui::Window::new(title)
        .id(egui::Id::new("results"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size([460.0, 320.0])
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
