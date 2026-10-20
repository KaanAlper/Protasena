//! How the window reaches the engine: menu picks, toolbar buttons and keys invoke commands (a
//! command with a dialog asks for it first), dialogs run the command's JSON form with the values
//! entered, and the engine's requests and messages become dialogs and toasts.

use protasena_engine::{Feedback, Outcome, Request};
use serde_json::{Value, json};

use crate::dialogs::results::Results;
use crate::dialogs::{Dialog, PathPurpose};
use crate::menus::model::UiAct;
use crate::unsaved::Then;
use crate::{App, tl};

impl App {
    /// Runs a command's JSON form (what the control channel and dialogs use). Never opens a
    /// dialog.
    ///
    /// # Errors
    /// The engine's refusal, as text.
    pub fn run(&mut self, id: &str, params: Value) -> Result<Value, String> {
        self.session.execute(id, &params).map_err(|e| e.to_string())
    }

    /// Runs a command as a menu pick, toolbar button or key would: its dialog opens first when
    /// it has one; a refusal shows as a message.
    pub fn invoke(&mut self, id: &str) {
        match self.session.invoke(id) {
            Ok(Outcome::Done(v)) => self.after(id, &v),
            Ok(Outcome::Dialog(r)) => self.request(r),
            Err(e) => self.notify(e.to_string(), true),
        }
    }

    /// Runs the command a dialog was for; shows a result window where the command reports one.
    ///
    /// # Errors
    /// The engine's refusal (the dialog stays open and shows it).
    pub fn complete(&mut self, id: &str, params: Value) -> Result<(), String> {
        let v = self.run(id, params)?;
        self.after(id, &v);
        Ok(())
    }

    /// What follows a command run from the window.
    fn after(&mut self, id: &str, v: &Value) {
        match id {
            "erc" => self.results = Some(Results::Erc(v.clone())),
            "export-netlist" => self.results = Some(Results::Netlist(v.clone())),
            "save" | "save-as" => {
                self.notify(format!("{} {}", tl!("Saved"), v["path"].as_str().unwrap_or_default()), false);
                self.saved();
            }
            "annotate" => self.notify(format!("{}: {}", tl!("Annotated"), v["changed"]), false),
            _ => {}
        }
    }

    /// Handles what an editing-window event or key did.
    pub fn feedback(&mut self, fb: Feedback) {
        if let Some(r) = fb.request {
            self.request(r);
        } else if let Some(m) = fb.message {
            self.notify(m, !fb.handled);
        }
    }

    /// Shows the dialog the engine asks for.
    pub fn request(&mut self, r: Request) {
        match r {
            Request::OpenProject if self.session.is_dirty() => self.dialog = Some(Dialog::SaveChanges { then: Then::Open }),
            Request::OpenProject => self.choose_open(false),
            Request::SaveProjectAs => self.choose_save(PathPurpose::SaveAs),
            Request::ExportNetlist => self.choose_save(PathPurpose::Netlist),
            other => self.dialog = Dialog::from_request(&self.session, other),
        }
    }

    /// Asks for a project to open (the native chooser when there is one) and opens it.
    pub fn choose_open(&mut self, discard: bool) {
        let picked = self.services.pick_open.as_ref().map(|f| f());
        match picked {
            Some(Some(path)) => {
                if let Err(e) = self.complete("open", json!({ "path": path.display().to_string(), "discard": discard })) {
                    self.notify(e, true);
                }
            }
            Some(None) => {}
            None => self.dialog = Some(Dialog::path(PathPurpose::Open { discard }, "")),
        }
    }

    /// Asks where to save (the native chooser when there is one) and saves or exports there.
    pub(crate) fn choose_save(&mut self, purpose: PathPurpose) {
        let name = self.session.project().name.clone();
        let (suggested, exts): (String, &[&str]) = match purpose {
            PathPurpose::Netlist => (format!("{name}.json"), &["json", "cir"]),
            _ => (format!("{name}.psproj"), &["psproj"]),
        };
        let picked = self.services.pick_save.as_ref().map(|f| f(&suggested, exts));
        match picked {
            Some(Some(path)) => {
                let path = path.display().to_string();
                let r = match purpose {
                    PathPurpose::Netlist => self.complete("export-netlist", json!({ "path": path, "format": crate::dialogs::netlist_format(&path) })),
                    _ => self.complete("save-as", json!({ "path": path })),
                };
                if let Err(e) = r {
                    self.after_save = None;
                    self.notify(e, true);
                }
            }
            Some(None) => self.after_save = None,
            None => self.dialog = Some(Dialog::path(purpose, &suggested)),
        }
    }

    /// A window action from a menu item.
    pub fn ui_act(&mut self, act: UiAct) {
        match act {
            UiAct::Exit => self.request_exit(),
            UiAct::ZoomArea => {
                self.canvas.zoom_area = Some(None);
                self.notify(tl!("Drag a rectangle around the area to show"), false);
            }
            UiAct::About => self.results = Some(Results::About),
        }
    }
}
