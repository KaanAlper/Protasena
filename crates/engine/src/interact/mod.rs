//! The interactive layer: pointer and key events from the editing window drive per-mode state
//! machines. Every change of the design goes through a command ([`Session::execute`]), so a
//! click does exactly what the command's JSON form does, with the same undo step.
//!
//! - **Wires** start in any placement mode (and Selection mode) by clicking a pin end, wire
//!   end, junction or terminal. While the wire follows the mouse the preview shows the routed
//!   path (the autorouter, W, routes orthogonally); a click on empty space fixes a corner; a
//!   click on a pin, wire or junction (or a double click anywhere) finishes it; Escape or the
//!   right button cancels it.
//! - **Selection**: click selects (Ctrl/Shift add), dragging an object drags the selection with
//!   its wires following, dragging empty space selects by box, a click on empty space clears,
//!   a double click asks for the object's edit dialog.
//! - **Component**: a ghost of the chosen device follows the cursor; a click places it with the
//!   Orientation toolbar's setting; a click on a placed part selects it.
//! - **Junction Dot**, **Terminals**: a click places one. **Wire Label**: a click on a wire asks
//!   for the label text. **Buses**: clicks add corners, a double click or the right button
//!   finishes. **Text Script**, **2D Graphics Text**: a click asks for the text.

mod machine;
mod modes;
mod preview;

use std::panic::{AssertUnwindSafe, catch_unwind};

pub use machine::{Machine, WireDraft};
pub use preview::Preview;

use protasena_geom::Pt;
use serde::Serialize;
use serde_json::Value;

use crate::error::{EngineError, Result};
use crate::hit::{Hit, hit};
use crate::registry::{Outcome, command_for_key};
use crate::request::Request;
use crate::session::Session;

/// A mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Button {
    /// Left button.
    #[default]
    Left,
    /// Right button.
    Right,
    /// Middle button (the UI pans with it; the engine ignores it).
    Middle,
}

/// What happened to the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PointerKind {
    /// It moved (with or without a button held).
    #[default]
    Move,
    /// A button went down.
    Press,
    /// A button went up.
    Release,
    /// A double click (sent after the second press and release).
    DoubleClick,
}

/// Keyboard modifiers held during a pointer event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    /// Shift.
    pub shift: bool,
    /// Ctrl (Command on macOS).
    pub ctrl: bool,
    /// Alt.
    pub alt: bool,
}

/// A pointer event in sheet coordinates (thou, y up; not snapped).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PointerEvent {
    /// What happened.
    pub kind: PointerKind,
    /// Where.
    pub at: Pt,
    /// Which button.
    pub button: Button,
    /// Modifiers.
    pub modifiers: Modifiers,
}

impl PointerEvent {
    /// A pointer move.
    pub fn moved(at: Pt) -> Self {
        Self { kind: PointerKind::Move, at, ..Self::default() }
    }
    /// A left press.
    pub fn press(at: Pt) -> Self {
        Self { kind: PointerKind::Press, at, ..Self::default() }
    }
    /// A left release.
    pub fn release(at: Pt) -> Self {
        Self { kind: PointerKind::Release, at, ..Self::default() }
    }
    /// A left double click.
    pub fn double_click(at: Pt) -> Self {
        Self { kind: PointerKind::DoubleClick, at, ..Self::default() }
    }
    /// A right press.
    pub fn right_press(at: Pt) -> Self {
        Self { kind: PointerKind::Press, at, button: Button::Right, ..Self::default() }
    }
}

/// What an event did, for the UI.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Feedback {
    /// The event was used.
    pub handled: bool,
    /// The command result, when the event ran one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// A dialog to show.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<Request>,
    /// A status-bar message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Feedback {
    pub(crate) fn handled() -> Self {
        Self { handled: true, ..Self::default() }
    }
    pub(crate) fn result(v: Value) -> Self {
        Self { handled: true, result: Some(v), ..Self::default() }
    }
    pub(crate) fn request(r: Request) -> Self {
        Self { handled: true, request: Some(r), ..Self::default() }
    }
    pub(crate) fn message(m: impl Into<String>) -> Self {
        Self { handled: true, message: Some(m.into()), ..Self::default() }
    }
}

impl Session {
    /// Feeds a pointer event to the active mode.
    ///
    /// # Errors
    /// A command the event ran was refused (the machine stays usable), or the position is not
    /// a finite point.
    pub fn pointer(&mut self, ev: PointerEvent) -> Result<Feedback> {
        if !(ev.at.is_finite() && ev.at.x.abs() <= crate::args::MAX_COORD && ev.at.y.abs() <= crate::args::MAX_COORD) {
            return Err(EngineError::Other("the pointer is not at a finite point".into()));
        }
        match catch_unwind(AssertUnwindSafe(|| self.pointer_inner(ev))) {
            Ok(r) => r,
            Err(_) => {
                self.machine = Machine::Idle;
                Err(EngineError::Internal { cmd: "pointer".into(), msg: "the event was dropped".into() })
            }
        }
    }

    fn pointer_inner(&mut self, ev: PointerEvent) -> Result<Feedback> {
        let found = self.sheet().and_then(|sh| hit(sh, &self.library, ev.at, self.view.pick_tolerance()));
        self.cursor = found.as_ref().filter(|h| h.starts_wire()).and_then(Hit::attach_point).unwrap_or_else(|| self.snap(ev.at));
        self.raw_cursor = ev.at;
        self.hover = found.clone();
        self.touch();
        match (ev.kind, ev.button) {
            (PointerKind::Move, _) => Ok(self.on_move(ev)),
            (PointerKind::Press, Button::Left) => self.on_press(ev, found),
            (PointerKind::Press, Button::Right) => self.on_right(),
            (PointerKind::Release, Button::Left) => self.on_release(ev),
            (PointerKind::DoubleClick, Button::Left) => self.on_double(found),
            _ => Ok(Feedback::default()),
        }
    }

    /// Feeds a key (in shortcut notation: `Escape`, `Delete`, `Ctrl+Z`, `F6`, `W`). Escape
    /// cancels what is in progress; other keys run the command bound to them (a command with a
    /// dialog returns the request instead).
    ///
    /// # Errors
    /// The command bound to the key was refused.
    pub fn key(&mut self, key: &str) -> Result<Feedback> {
        let k = key.trim();
        if k.eq_ignore_ascii_case("escape") || k.eq_ignore_ascii_case("esc") {
            let was_busy = !matches!(self.machine, Machine::Idle);
            self.cancel();
            return Ok(if was_busy { Feedback::handled() } else { Feedback::default() });
        }
        let Some(spec) = command_for_key(k) else { return Ok(Feedback::default()) };
        if spec.id == "delete" && self.selection.is_empty() {
            return Ok(Feedback::default());
        }
        match self.invoke(spec.id) {
            Ok(Outcome::Done(v)) => Ok(Feedback::result(v)),
            Ok(Outcome::Dialog(r)) => Ok(Feedback::request(r)),
            // A key for a command that cannot run now (Ctrl+Z with nothing to undo) does nothing.
            Err(EngineError::Disabled { why, .. }) => Ok(Feedback { handled: false, message: Some(why), ..Feedback::default() }),
            Err(e) => Err(e),
        }
    }

    /// Cancels the wire, bus, drag or box in progress.
    pub fn cancel(&mut self) {
        self.machine = Machine::Idle;
        self.touch();
    }

    /// What is in progress, for the UI.
    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    /// What lies under the pointer.
    pub fn hover(&self) -> Option<&Hit> {
        self.hover.as_ref()
    }
}
