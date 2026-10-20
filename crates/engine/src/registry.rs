//! The command registry and the two ways to run a command: [`Session::execute`] (JSON form,
//! never a dialog) and [`Session::invoke`] (menu, toolbar or key: may ask the UI for a dialog
//! first).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::OnceLock;

use serde::Serialize;
use serde_json::Value;

use crate::args::Args;
use crate::cmd;
use crate::error::{EngineError, Result};
use crate::mode::Mode;
use crate::request::Request;
use crate::session::{Session, Step};

/// Runs a command with its parameters.
pub type Run = fn(&mut Session, &Args) -> Result<Value>;
/// Whether a command can run now; `Err` says why not.
pub type Enabled = fn(&Session) -> std::result::Result<(), String>;
/// The dialog a menu or key opens before the command runs, if any.
pub type DialogFn = fn(&Session) -> Option<Request>;

/// One command: what menus, toolbars, keys, the CLI and the control channel call.
pub struct CommandSpec {
    /// Stable lower-case id (`place-component`).
    pub id: &'static str,
    /// Proteus's menu or tool name.
    pub label: &'static str,
    /// Menu placement (`["Edit", "Undo Changes"]`); empty for toolbar-only or programmatic
    /// commands.
    pub menu: &'static [&'static str],
    /// Proteus's keyboard shortcut (`Ctrl+Z`, `F6`, `W`).
    pub shortcut: Option<&'static str>,
    /// The JSON parameters in brief.
    pub params: &'static str,
    /// Enabled-state rule.
    pub enabled: Enabled,
    /// The JSON form.
    pub run: Run,
    /// The dialog [`Session::invoke`] asks for instead of running.
    pub dialog: Option<DialogFn>,
    /// The mode whose clicks are this command's interactive form.
    pub machine: Option<Mode>,
}

impl CommandSpec {
    /// A command that is always enabled, without menu, shortcut or dialog.
    pub const fn new(id: &'static str, label: &'static str, run: Run) -> Self {
        Self { id, label, menu: &[], shortcut: None, params: "", enabled: always, run, dialog: None, machine: None }
    }
    /// Sets the menu path.
    pub const fn menu(mut self, menu: &'static [&'static str]) -> Self {
        self.menu = menu;
        self
    }
    /// Sets the shortcut.
    pub const fn key(mut self, key: &'static str) -> Self {
        self.shortcut = Some(key);
        self
    }
    /// Sets the params doc.
    pub const fn params(mut self, params: &'static str) -> Self {
        self.params = params;
        self
    }
    /// Sets the enabled rule.
    pub const fn enabled(mut self, enabled: Enabled) -> Self {
        self.enabled = enabled;
        self
    }
    /// Sets the dialog.
    pub const fn dialog(mut self, dialog: DialogFn) -> Self {
        self.dialog = Some(dialog);
        self
    }
    /// Sets the interactive mode.
    pub const fn machine(mut self, mode: Mode) -> Self {
        self.machine = Some(mode);
        self
    }

    /// The command as the UI and the control channel list it.
    pub fn info(&self, s: &Session) -> CommandInfo {
        let e = (self.enabled)(s);
        CommandInfo {
            id: self.id,
            label: self.label,
            menu: self.menu.to_vec(),
            shortcut: self.shortcut,
            params: self.params,
            mode: self.machine.map(Mode::id),
            enabled: e.is_ok(),
            disabled_reason: e.err(),
        }
    }
}

/// A command's description with its current enabled state.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CommandInfo {
    /// Id.
    pub id: &'static str,
    /// Label.
    pub label: &'static str,
    /// Menu path.
    pub menu: Vec<&'static str>,
    /// Shortcut.
    pub shortcut: Option<&'static str>,
    /// Params doc.
    pub params: &'static str,
    /// Interactive mode id.
    pub mode: Option<&'static str>,
    /// Enabled now.
    pub enabled: bool,
    /// Why not, when disabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,
}

/// What [`Session::invoke`] did.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The command ran; its JSON result.
    Done(Value),
    /// The UI must show this dialog, then call [`Session::execute`] with its values.
    Dialog(Request),
}

/// Always enabled.
pub fn always(_: &Session) -> std::result::Result<(), String> {
    Ok(())
}

/// Enabled with a selection.
pub fn has_selection(s: &Session) -> std::result::Result<(), String> {
    if s.selection.is_empty() { Err("nothing selected".into()) } else { Ok(()) }
}

/// Every command, in menu order within each area.
pub fn command_specs() -> &'static [CommandSpec] {
    static SPECS: OnceLock<Vec<CommandSpec>> = OnceLock::new();
    SPECS.get_or_init(cmd::all_specs)
}

/// The command with this id (ASCII case ignored).
pub fn find_command(id: &str) -> Option<&'static CommandSpec> {
    command_specs().iter().find(|c| c.id.eq_ignore_ascii_case(id.trim()))
}

/// The command bound to a key (`Ctrl+Z`, `f6`, `w`; case and spaces ignored).
pub fn command_for_key(key: &str) -> Option<&'static CommandSpec> {
    let norm = normalize_key(key);
    command_specs().iter().find(|c| c.shortcut.is_some_and(|k| normalize_key(k) == norm))
}

fn normalize_key(key: &str) -> String {
    key.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_lowercase()
}

/// What `execute` restores when a command panics (the design itself is restored by undoing
/// the steps the command pushed, so nothing large is copied).
struct Saved {
    selection: Vec<protasena_doc::ItemId>,
    sheet: protasena_doc::SheetId,
    dirty: bool,
    commits: u64,
}

impl Session {
    /// Every command with its enabled state.
    pub fn commands(&self) -> Vec<CommandInfo> {
        command_specs().iter().map(|c| c.info(self)).collect()
    }

    /// Runs a command's JSON form (no dialogs; defaults for everything not given). A command
    /// that changes the design is one undo step. A panic inside a command is caught and the
    /// design restored.
    ///
    /// # Errors
    /// [`EngineError::UnknownCommand`], [`EngineError::Disabled`], [`EngineError::BadParams`],
    /// the command's own errors, or [`EngineError::Internal`] after a caught panic.
    pub fn execute(&mut self, id: &str, params: &Value) -> Result<Value> {
        let spec = find_command(id).ok_or_else(|| EngineError::UnknownCommand(id.chars().take(64).collect()))?;
        self.run_spec(spec, params)
    }

    /// Runs one command under the panic guard.
    fn run_spec(&mut self, spec: &CommandSpec, params: &Value) -> Result<Value> {
        (spec.enabled)(self).map_err(|why| EngineError::Disabled { cmd: spec.id.into(), why })?;
        let args = Args::new(spec.id, params)?;
        let saved = Saved { selection: self.selection.clone(), sheet: self.sheet, dirty: self.dirty, commits: self.commits };
        let outer = self.guard.replace(crate::session::Guard::default());
        let result = catch_unwind(AssertUnwindSafe(|| (spec.run)(self, &args)));
        let guard = std::mem::replace(&mut self.guard, outer).unwrap_or_default();
        match result {
            Ok(result) => {
                self.merge_steps(saved.commits, spec.label);
                result
            }
            Err(panic) => {
                let msg = panic.downcast_ref::<&str>().map(|s| (*s).to_owned()).or_else(|| panic.downcast_ref::<String>().cloned());
                self.roll_back(saved, guard);
                Err(EngineError::Internal { cmd: spec.id.into(), msg: msg.unwrap_or_else(|| "panic".into()) })
            }
        }
    }

    /// Undoes what a panicking command committed and restores the rest of its state.
    fn roll_back(&mut self, saved: Saved, guard: crate::session::Guard) {
        let pushed = usize::try_from(self.commits.saturating_sub(saved.commits)).unwrap_or(usize::MAX).min(self.undo.len());
        for _ in 0..pushed {
            if let Some(step) = self.undo.pop() {
                let _ = self.project.apply(step.op);
            }
        }
        if let Some(redo) = guard.redo {
            self.redo = redo;
        }
        self.selection = saved.selection;
        self.sheet = saved.sheet;
        self.dirty = saved.dirty;
        self.machine = crate::interact::Machine::Idle;
        self.fix_sheet();
        self.prune_selection();
        self.touch();
    }

    /// Runs a command as a menu pick, toolbar button or key would: when the command has a
    /// dialog the UI is asked to show it instead.
    ///
    /// # Errors
    /// As [`Session::execute`].
    pub fn invoke(&mut self, id: &str) -> Result<Outcome> {
        let spec = find_command(id).ok_or_else(|| EngineError::UnknownCommand(id.chars().take(64).collect()))?;
        (spec.enabled)(self).map_err(|why| EngineError::Disabled { cmd: spec.id.into(), why })?;
        if let Some(request) = spec.dialog.and_then(|d| d(self)) {
            return Ok(Outcome::Dialog(request));
        }
        self.execute(spec.id, &Value::Null).map(Outcome::Done)
    }

    /// Makes the undo steps one command pushed into a single step labelled with the command.
    fn merge_steps(&mut self, commits_before: u64, label: &str) {
        let pushed = usize::try_from(self.commits.saturating_sub(commits_before)).unwrap_or(usize::MAX).min(self.undo.len());
        if pushed == 1
            && let Some(last) = self.undo.last_mut()
        {
            last.label = label.to_owned();
        }
        if pushed <= 1 {
            return;
        }
        let from = self.undo.len().saturating_sub(pushed);
        let ops: Vec<_> = self.undo.drain(from..).rev().map(|s| s.op).collect();
        self.undo.push(Step { label: label.to_owned(), op: protasena_doc::Op::batch(ops) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn explode(s: &mut Session, _: &Args) -> Result<Value> {
        s.commit("x", vec![protasena_doc::Op::RemoveSheet { id: s.sheet }])?;
        panic!("boom");
    }

    #[test]
    fn a_panicking_command_restores_the_design() {
        let mut s = Session::new();
        s.execute("junction", &json!({ "at": [0, 0] })).unwrap();
        let before = s.project.clone();
        let steps = s.undo.len();
        let spec = CommandSpec::new("explode", "Explode", explode);
        let e = s.run_spec(&spec, &Value::Null).unwrap_err();
        assert!(matches!(e, EngineError::Internal { ref msg, .. } if msg == "boom"), "{e:?}");
        assert_eq!(s.project, before);
        assert_eq!(s.undo.len(), steps);
        s.undo().unwrap();
        assert!(s.sheet().unwrap().junctions.is_empty());
    }

    #[test]
    fn rollback_is_exact_with_a_full_undo_stack() {
        let mut s = Session::new();
        for i in 0..crate::session::MAX_UNDO + 5 {
            s.execute("junction", &json!({ "at": [i, 0] })).unwrap();
        }
        s.undo().unwrap();
        let redo = s.redo.clone();
        let before = s.project.clone();
        let spec = CommandSpec::new("explode", "Explode", explode);
        assert!(s.run_spec(&spec, &Value::Null).is_err());
        assert_eq!(s.project.sheets, before.sheets);
        assert_eq!(s.undo.len(), crate::session::MAX_UNDO - 1);
        assert_eq!(s.redo, redo, "the redo stack the panicking command cleared is back");
        s.undo().unwrap();
        s.redo().unwrap();
        s.redo().unwrap();
        assert_eq!(s.sheet().unwrap().junctions.len(), crate::session::MAX_UNDO + 5);
    }

    #[test]
    fn several_commits_merge_into_one_labelled_step() {
        fn twice(s: &mut Session, _: &Args) -> Result<Value> {
            let sheet = s.sheet;
            for x in [0.0, 100.0] {
                let id = s.fresh()?;
                let item = protasena_doc::Item::Junction(protasena_doc::Junction { id, at: protasena_geom::Pt::new(x, 0.0) });
                s.commit("part", vec![protasena_doc::Op::Add { sheet, item, index: None }])?;
            }
            Ok(Value::Null)
        }
        let mut s = Session::new();
        s.run_spec(&CommandSpec::new("twice", "Twice", twice), &Value::Null).unwrap();
        assert_eq!(s.undo.len(), 1);
        assert_eq!(s.undo[0].label, "Twice");
        s.undo().unwrap();
        assert!(s.sheet().unwrap().junctions.is_empty());
    }
}
