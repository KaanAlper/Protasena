//! Unsaved changes: New Project, Open Project, File ▸ Exit Application and the window's close
//! button ask Save / Don't Save / Cancel first while the design has unsaved changes.
//!
//! **Save** saves to the project's file (or asks for one, as Save Project As does) and carries
//! on only once the save succeeded; **Don't Save** carries on and the changes are lost;
//! **Cancel** (or closing the question) leaves everything as it was. Proteus's own prompt could
//! not be observed yet: its wording, buttons and order are to compare (`docs/gaps.md`).

use serde_json::json;

use crate::App;
use crate::dialogs::{Dialog, PathPurpose};

/// What waits for the answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Then {
    /// New Project.
    New,
    /// Open Project.
    Open,
    /// Close the window.
    Exit,
}

impl Then {
    /// The step for the engine's `ConfirmDiscard { then }` (`new` or `open`).
    pub fn from_command(id: &str) -> Then {
        if id == "open" { Then::Open } else { Then::New }
    }
}

impl App {
    /// File ▸ Exit Application: asks first while there are unsaved changes.
    pub fn request_exit(&mut self) {
        if self.session.is_dirty() {
            self.dialog = Some(Dialog::SaveChanges { then: Then::Exit });
        } else {
            self.carry_on(Then::Exit);
        }
    }

    /// The window was asked to close (its close button, the window manager, Alt+F4): keeps it
    /// open and asks while there are unsaved changes.
    pub(crate) fn close_requested(&mut self, ctx: &egui::Context) {
        if self.close_confirmed || !self.session.is_dirty() {
            self.close_confirmed = true;
            return;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        if !matches!(self.dialog, Some(Dialog::SaveChanges { .. })) {
            self.dialog = Some(Dialog::SaveChanges { then: Then::Exit });
        }
    }

    /// Closes the window without asking (`app.quit`).
    pub fn quit_now(&mut self) {
        self.carry_on(Then::Exit);
    }

    /// The answer was Don't Save (or the changes were saved): does what was waiting.
    pub fn carry_on(&mut self, then: Then) {
        match then {
            Then::New => {
                if let Err(e) = self.complete("new", json!({ "discard": true })) {
                    self.notify(e, true);
                }
            }
            Then::Open => self.choose_open(true),
            Then::Exit => {
                self.close_confirmed = true;
                self.quit_requested = true;
            }
        }
    }

    /// The answer was Save: saves (asking for a file when the project has none) and carries on
    /// once the save succeeded.
    pub fn save_then(&mut self, then: Then) {
        self.after_save = Some(then);
        if self.session.path().is_none() {
            self.choose_save(PathPurpose::SaveAs);
            return;
        }
        if let Err(e) = self.complete("save", json!({})) {
            self.after_save = None;
            self.notify(e, true);
        }
    }

    /// A save finished: carries on with what was waiting for it.
    pub(crate) fn saved(&mut self) {
        if let Some(then) = self.after_save.take() {
            self.carry_on(then);
        }
    }
}
