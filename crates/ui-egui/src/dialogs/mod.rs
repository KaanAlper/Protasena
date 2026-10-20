//! Dialogs. The engine asks for one with a [`Request`]; the dialog collects values and runs the
//! command's JSON form. Layouts are our own, following each dialog's purpose in Proteus (its
//! dialogs could not be observed yet: field sets and defaults are to compare, see
//! `docs/gaps.md`).

pub mod edit_component;
pub mod export;
mod files;
mod forms;
pub mod pick_devices;
pub mod pick_model;
pub mod pins;
pub mod results;
pub mod settings_form;
pub mod subcircuit;
pub mod system;
pub mod tools;

use egui::{Align, Layout, RichText, Ui, vec2};
use protasena_engine::{Request, Session};
use serde_json::Value;

pub use files::{DesignForm, PathForm, PathPurpose, SheetForm, netlist_format};
pub use forms::{ConfirmForm, FindForm, GotoForm, LabelForm, TERMINAL_KINDS, Target, TerminalForm, TextForm};

use crate::theme::Tokens;
use crate::unsaved::Then;
use crate::{App, control, tl};

/// The open dialog.
pub enum Dialog {
    /// Pick Devices.
    PickDevices(Box<pick_devices::PickDevices>),
    /// Find/Edit Component without a selected part: asks for the reference.
    FindComponent(FindForm),
    /// Edit Component.
    EditComponent(Box<edit_component::EditComponent>),
    /// Edit Wire Label (new or existing).
    Label(LabelForm),
    /// Edit Terminal.
    Terminal(TerminalForm),
    /// A text or text script (new or existing).
    Text(TextForm),
    /// A file path, when the app has no native chooser.
    Path(PathForm),
    /// Edit Design Properties.
    Design(DesignForm),
    /// Edit Sheet Properties.
    Sheet(SheetForm),
    /// Goto Sheet.
    GotoSheet(GotoForm),
    /// Global Annotator.
    Annotator {
        /// Total (renumber all) instead of incremental.
        total: bool,
    },
    /// Export Graphics (one of its items).
    Export(Box<export::ExportForm>),
    /// Print Design.
    Print(Box<export::PrintForm>),
    /// Align Objects (the index of the edge chosen).
    Align {
        /// Which edge or centre.
        edge: usize,
    },
    /// Search & Tag.
    SearchTag(tools::SearchTagForm),
    /// Property Assignment Tool.
    PropertyAssign(tools::PropertyAssignForm),
    /// A new subcircuit port's name and kind.
    Port(subcircuit::PortForm),
    /// Edit Subcircuit.
    Subcircuit(subcircuit::SubcircuitForm),
    /// A Template or System menu settings dialog.
    Settings(Box<settings_form::SettingsForm>),
    /// Set Keyboard Mapping.
    Keymap(Box<system::KeymapForm>),
    /// Set Property Definitions or Set Sheet Sizes.
    List(system::ListForm),
    /// Restore Default Settings: asks first.
    RestoreDefaults,
    /// Unsaved changes: Save, Don't Save or Cancel?
    SaveChanges {
        /// What waits for the answer.
        then: Then,
    },
}

/// What a dialog's frame decided.
pub enum Reply {
    /// Still open.
    None,
    /// Cancelled.
    Cancel,
    /// Run a command; `keep` leaves the dialog open (Pick Devices' double click).
    Run {
        /// Command id.
        cmd: &'static str,
        /// Its JSON form.
        params: Value,
        /// Keep the dialog open.
        keep: bool,
    },
    /// Show another dialog instead.
    Then(Dialog),
    /// The unsaved-changes answer: save first (`save`) or not, then carry on with `then`.
    SaveChanges {
        /// What waits.
        then: Then,
        /// Save before carrying on.
        save: bool,
    },
}

impl Dialog {
    /// The dialog for an engine request (file choosers are handled by the app).
    pub fn from_request(s: &Session, r: Request) -> Option<Dialog> {
        Some(match r {
            Request::PickDevices => Dialog::PickDevices(Box::default()),
            Request::FindComponent => Dialog::FindComponent(FindForm::default()),
            Request::EditComponent { id } => Dialog::EditComponent(Box::new(edit_component::EditComponent::new(s, id)?)),
            Request::WireLabel { at } => Dialog::Label(LabelForm::new_at(at)),
            Request::EditLabel { id } => Dialog::Label(LabelForm::edit(s, id)?),
            Request::EditTerminal { id } => Dialog::Terminal(TerminalForm::new(s, id)?),
            Request::Text { at } => Dialog::Text(TextForm::new_at(at).editor(s)),
            Request::EditText { id } => Dialog::Text(TextForm::edit(s, id)?.editor(s)),
            Request::DesignProperties => Dialog::Design(DesignForm::new(s)),
            Request::SheetProperties { sheet } => Dialog::Sheet(SheetForm::new(s, sheet)?),
            Request::GotoSheet => Dialog::GotoSheet(GotoForm::new(s)),
            Request::GlobalAnnotator => Dialog::Annotator { total: false },
            Request::Align => Dialog::Align { edge: 0 },
            Request::SearchTag => Dialog::SearchTag(tools::SearchTagForm::default()),
            Request::PropertyAssign => Dialog::PropertyAssign(tools::PropertyAssignForm::new(!s.selection().is_empty())),
            Request::SubcircuitPort { subcircuit, at } => Dialog::Port(subcircuit::PortForm::new(s, subcircuit, at)),
            Request::EditSubcircuit { id } => Dialog::Subcircuit(subcircuit::SubcircuitForm::new(s, id)?),
            Request::Template { section } | Request::System { section } => Dialog::Settings(Box::new(settings_form::SettingsForm::new(s, section)?)),
            Request::KeyboardMapping => Dialog::Keymap(Box::new(system::KeymapForm::new(s))),
            Request::PropertyDefinitions => Dialog::List(system::ListForm::properties(s)),
            Request::SheetSizes => Dialog::List(system::ListForm::sheet_sizes(s)),
            Request::RestoreDefaults => Dialog::RestoreDefaults,
            // A result window, not a dialog: the app runs the command and shows it.
            Request::BillOfMaterials => return None,
            Request::ConfirmDiscard { then } => Dialog::SaveChanges { then: Then::from_command(then) },
            Request::OpenProject => Dialog::path(PathPurpose::Open { discard: false }, ""),
            Request::SaveProjectAs => Dialog::path(PathPurpose::SaveAs, ""),
            Request::ExportNetlist => Dialog::path(PathPurpose::Netlist, ""),
            Request::ExportGraphics { format } => Dialog::Export(Box::new(export::ExportForm::new(s, export::export_kind(format)?))),
            Request::PrintDesign => Dialog::Print(Box::new(export::PrintForm::new(s, false))),
        })
    }

    /// A path dialog.
    pub fn path(purpose: PathPurpose, suggested: &str) -> Dialog {
        Dialog::Path(PathForm::new(purpose, suggested))
    }

    /// The dialog's title (also its name for `ui.dialog`).
    pub fn title(&self) -> &'static str {
        match self {
            Dialog::PickDevices(_) => tl!("Pick Devices"),
            Dialog::FindComponent(_) => tl!("Find/Edit Component"),
            Dialog::EditComponent(_) => tl!("Edit Component"),
            Dialog::Label(_) => tl!("Edit Wire Label"),
            Dialog::Terminal(_) => tl!("Edit Terminal"),
            Dialog::Text(_) => tl!("Edit Text"),
            Dialog::Path(p) => p.purpose.title(),
            Dialog::Design(_) => tl!("Edit Design Properties"),
            Dialog::Sheet(_) => tl!("Edit Sheet Properties"),
            Dialog::GotoSheet(_) => tl!("Goto Sheet"),
            Dialog::Annotator { .. } => tl!("Global Annotator"),
            Dialog::Align { .. } => tl!("Align Objects"),
            Dialog::SearchTag(_) => tl!("Search & Tag"),
            Dialog::PropertyAssign(_) => tl!("Property Assignment Tool"),
            Dialog::Export(d) => d.kind.title,
            Dialog::Print(_) => tl!("Print Design"),
            Dialog::Port(_) => tl!("Subcircuit Port"),
            Dialog::Subcircuit(_) => tl!("Edit Subcircuit"),
            Dialog::Settings(d) => d.title,
            Dialog::Keymap(_) => tl!("Set Keyboard Mapping"),
            Dialog::List(d) if d.cmd == "set-sheet-sizes" => tl!("Set Sheet Sizes"),
            Dialog::List(_) => tl!("Set Property Definitions"),
            Dialog::RestoreDefaults => tl!("Restore Default Settings"),
            Dialog::SaveChanges { .. } => tl!("Unsaved Changes"),
        }
    }

    /// Shows a refusal from the engine inside the dialog.
    pub fn set_error(&mut self, e: String) {
        let slot = match self {
            Dialog::PickDevices(d) => &mut d.error,
            Dialog::FindComponent(d) => &mut d.error,
            Dialog::EditComponent(d) => &mut d.error,
            Dialog::Label(d) => &mut d.error,
            Dialog::Terminal(d) => &mut d.error,
            Dialog::Text(d) => &mut d.error,
            Dialog::Path(d) => &mut d.error,
            Dialog::Design(d) => &mut d.error,
            Dialog::Sheet(d) => &mut d.error,
            Dialog::Export(d) => &mut d.out.error,
            Dialog::SearchTag(d) => &mut d.error,
            Dialog::PropertyAssign(d) => &mut d.error,
            Dialog::Print(d) => &mut d.out.error,
            Dialog::Port(d) => &mut d.error,
            Dialog::Subcircuit(d) => &mut d.error,
            Dialog::Settings(d) => &mut d.error,
            Dialog::Keymap(d) => &mut d.error,
            Dialog::List(d) => &mut d.error,
            Dialog::RestoreDefaults => return,
            Dialog::GotoSheet(_) | Dialog::Annotator { .. } | Dialog::Align { .. } | Dialog::SaveChanges { .. } => return,
        };
        *slot = Some(e);
    }

    /// The file options of an export or print dialog.
    fn output(&mut self) -> Option<&mut export::Output> {
        match self {
            Dialog::Export(d) => Some(&mut d.out),
            Dialog::Print(d) => Some(&mut d.out),
            _ => None,
        }
    }

    fn body(&mut self, ui: &mut Ui, s: &Session) -> Reply {
        match self {
            Dialog::PickDevices(d) => d.ui(ui, s),
            Dialog::FindComponent(d) => d.ui(ui, s),
            Dialog::EditComponent(d) => d.ui(ui, s),
            Dialog::Label(d) => d.ui(ui),
            Dialog::Terminal(d) => d.ui(ui),
            Dialog::Text(d) => d.ui(ui),
            Dialog::Path(d) => d.ui(ui),
            Dialog::Design(d) => d.ui(ui),
            Dialog::Sheet(d) => d.ui(ui),
            Dialog::GotoSheet(d) => d.ui(ui, s),
            Dialog::Annotator { total } => forms::annotator(ui, total),
            Dialog::Export(d) => d.ui(ui),
            Dialog::Align { edge } => tools::align(ui, edge),
            Dialog::SearchTag(d) => d.ui(ui),
            Dialog::PropertyAssign(d) => d.ui(ui),
            Dialog::Print(d) => d.ui(ui),
            Dialog::Port(d) => d.ui(ui),
            Dialog::Subcircuit(d) => d.ui(ui),
            Dialog::Settings(d) => d.ui(ui),
            Dialog::Keymap(d) => d.ui(ui),
            Dialog::List(d) => d.ui(ui),
            Dialog::RestoreDefaults => system::restore(ui),
            Dialog::SaveChanges { then } => ConfirmForm::ui(ui, *then),
        }
    }
}

/// Shows the open dialog and carries out what it decided.
pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(mut d) = app.dialog.take() else { return };
    let title = d.title();
    let t = Tokens::get();
    let mut reply = Reply::None;
    let modal = egui::Modal::new(egui::Id::new(("dialog", title)))
        .frame(egui::Frame::window(&ctx.global_style()).fill(t.panel).inner_margin(egui::Margin::same(16)))
        .show(ctx, |ui| {
            ui.label(RichText::new(title).size(15.0).strong().color(t.text));
            ui.add_space(8.0);
            reply = d.body(ui, &app.session);
        });
    control::record_rect(ctx, egui::Id::new(("dialog-area", title)), modal.response.rect, title, "dialog", true, None);
    if modal.should_close() && matches!(reply, Reply::None) {
        reply = Reply::Cancel;
    }
    if let Some(out) = d.output() {
        app.browse(out);
    }
    match reply {
        Reply::None => app.dialog = Some(d),
        Reply::Cancel => app.after_save = None,
        Reply::Then(next) => app.dialog = Some(next),
        Reply::SaveChanges { then, save: true } => app.save_then(then),
        Reply::SaveChanges { then, save: false } => app.carry_on(then),
        Reply::Run { cmd, params, keep } => match app.complete(cmd, params) {
            Ok(()) if keep => app.dialog = Some(d),
            Ok(()) => {}
            Err(e) => {
                d.set_error(e);
                app.dialog = Some(d);
            }
        },
    }
}

/// The OK / Cancel row at the bottom of a dialog: `(ok, cancel)`.
pub fn ok_cancel(ui: &mut Ui, ok_enabled: bool) -> (bool, bool) {
    let mut out = (false, false);
    ui.add_space(10.0);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let cancel = ui.add_sized(vec2(84.0, 26.0), egui::Button::new(tl!("Cancel")));
        control::record(ui, &cancel, tl!("Cancel"), "button", None);
        let t = Tokens::get();
        let ok = ui.add_enabled(ok_enabled, egui::Button::new(RichText::new(tl!("OK")).color(t.on_accent)).fill(t.accent).min_size(vec2(84.0, 26.0)));
        control::record(ui, &ok, tl!("OK"), "button", None);
        out = (ok.clicked(), cancel.clicked());
    });
    out
}

/// A refusal shown in a dialog.
pub fn error_line(ui: &mut Ui, error: Option<&String>) {
    if let Some(e) = error {
        ui.add_space(4.0);
        ui.label(RichText::new(e).color(Tokens::get().warn));
    }
}

/// A labelled single-line field in a form grid; true when Enter was pressed in it.
pub fn field(ui: &mut Ui, label: &str, value: &mut String, width: f32) -> bool {
    ui.label(label);
    let r = ui.add(egui::TextEdit::singleline(value).desired_width(width));
    control::record(ui, &r, label, "field", None);
    ui.end_row();
    r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
}
