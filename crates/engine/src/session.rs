//! The editing session: the open project, the library, selection, mode, placement settings,
//! view state and the undo/redo stacks.
//!
//! Every change of the design goes through [`Session::commit`], which applies one batch of
//! [`Op`]s and keeps its inverse as one undo step. Undo applies the inverse and keeps what
//! that returns as the redo step, so undo and redo are exact.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use protasena_doc::{DeviceRef, Id, Item, ItemId, Op, Project, Sheet, SheetId, TerminalKind};
use protasena_geom::{Orient, Pt, snap};
use protasena_library::Library;

use crate::error::{EngineError, Result};
use crate::hit::Hit;
use crate::interact::Machine;
use crate::mode::Mode;
use crate::view::View;

/// Undo steps kept, at most (the oldest are dropped).
pub const MAX_UNDO: usize = 1000;

/// Log lines kept, at most.
const MAX_LOG: usize = 200;

/// One undo or redo step: the op that reverses a command, with the command's label.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    /// Label of the command that made the change (`Undo <label>` in a menu).
    pub label: String,
    /// The op that reverses it.
    pub op: Op,
}

/// Kept while a command runs, so a panic can be undone: the redo stack the first commit
/// cleared.
#[derive(Debug, Clone, Default)]
pub(crate) struct Guard {
    pub(crate) redo: Option<Vec<Step>>,
}

/// Items copied to the internal clipboard, with the point they are pasted relative to.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Clipboard {
    /// The copied items (ids are replaced when pasted).
    pub items: Vec<Item>,
    /// Reference point (the snapped centre of the copied items).
    pub anchor: Pt,
}

/// An editing session on one project.
#[derive(Debug, Clone)]
pub struct Session {
    pub(crate) project: Project,
    pub(crate) library: Library,
    pub(crate) path: Option<PathBuf>,
    pub(crate) dirty: bool,
    pub(crate) sheet: SheetId,
    pub(crate) selection: Vec<ItemId>,
    pub(crate) mode: Mode,
    pub(crate) placement: Orient,
    pub(crate) autorouter: bool,
    pub(crate) device: Option<DeviceRef>,
    pub(crate) picked: Vec<DeviceRef>,
    pub(crate) terminal_kind: TerminalKind,
    pub(crate) clipboard: Clipboard,
    pub(crate) view: View,
    pub(crate) cursor: Pt,
    pub(crate) raw_cursor: Pt,
    pub(crate) hover: Option<Hit>,
    pub(crate) machine: Machine,
    pub(crate) undo: Vec<Step>,
    pub(crate) redo: Vec<Step>,
    pub(crate) revision: u64,
    /// Counts [`Session::commit`]s (one command's steps are merged into one).
    pub(crate) commits: u64,
    pub(crate) guard: Option<Guard>,
    pub(crate) log: Vec<String>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    /// A session on a new, empty project with the built-in parts.
    pub fn new() -> Self {
        Self::with_library(Library::builtin())
    }

    /// A session on a new, empty project with `library`.
    pub fn with_library(library: Library) -> Self {
        let project = Project::new("UNTITLED");
        let sheet = project.sheets.first().map_or(SheetId(1), |s| s.id);
        Self {
            project,
            library,
            path: None,
            dirty: false,
            sheet,
            selection: Vec::new(),
            mode: Mode::default(),
            placement: Orient::IDENTITY,
            autorouter: true,
            device: None,
            picked: Vec::new(),
            terminal_kind: TerminalKind::Default,
            clipboard: Clipboard::default(),
            view: View::default(),
            cursor: Pt::ORIGIN,
            raw_cursor: Pt::ORIGIN,
            hover: None,
            machine: Machine::Idle,
            undo: Vec::new(),
            redo: Vec::new(),
            revision: 0,
            commits: 0,
            guard: None,
            log: Vec::new(),
        }
    }

    // ----- read access for the UI -----

    /// The project.
    pub fn project(&self) -> &Project {
        &self.project
    }
    /// The device library.
    pub fn library(&self) -> &Library {
        &self.library
    }
    /// The project file, once saved or opened.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
    /// Whether the project has changes not yet saved.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    /// The id of the sheet shown in the editing window.
    pub fn sheet_id(&self) -> SheetId {
        self.sheet
    }
    /// The sheet shown in the editing window.
    pub fn sheet(&self) -> Option<&Sheet> {
        self.project.sheet(self.sheet)
    }
    /// The selected items (on the current sheet), in selection order.
    pub fn selection(&self) -> &[ItemId] {
        &self.selection
    }
    /// The active mode.
    pub fn mode(&self) -> Mode {
        self.mode
    }
    /// The Orientation toolbar's setting for the next placed object.
    pub fn placement(&self) -> Orient {
        self.placement
    }
    /// Whether the wire autorouter is on (W).
    pub fn autorouter(&self) -> bool {
        self.autorouter
    }
    /// The device chosen in the object selector for placement.
    pub fn device(&self) -> Option<&DeviceRef> {
        self.device.as_ref()
    }
    /// The DEVICES list of the object selector (devices picked into the design).
    pub fn picked(&self) -> &[DeviceRef] {
        &self.picked
    }
    /// The terminal kind chosen in the TERMINALS list.
    pub fn terminal_kind(&self) -> TerminalKind {
        self.terminal_kind
    }
    /// The internal clipboard.
    pub fn clipboard(&self) -> &Clipboard {
        &self.clipboard
    }
    /// View state (zoom, grid, snap…).
    pub fn view(&self) -> &View {
        &self.view
    }
    /// The last cursor position (snapped), in sheet coordinates.
    pub fn cursor(&self) -> Pt {
        self.cursor
    }
    /// Counts every change of the design or the view (the UI redraws when it changes).
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Messages for the status bar, oldest first.
    pub fn log(&self) -> &[String] {
        &self.log
    }
    /// The undo steps, oldest first.
    pub fn undo_steps(&self) -> &[Step] {
        &self.undo
    }
    /// The redo steps, the next one last.
    pub fn redo_steps(&self) -> &[Step] {
        &self.redo
    }

    // ----- changes -----

    /// Applies `ops` as one undo step labelled `label`. Nothing is recorded when the batch is
    /// empty. On error the project is unchanged.
    ///
    /// # Errors
    /// [`EngineError::Doc`] when the design model refuses an op.
    pub fn commit(&mut self, label: &str, ops: Vec<Op>) -> Result<()> {
        let op = Op::batch(ops);
        if op.is_empty() {
            return Ok(());
        }
        let undo = self.project.apply(op)?;
        self.undo.push(Step { label: label.to_owned(), op: undo });
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        let cleared = std::mem::take(&mut self.redo);
        if let Some(guard) = self.guard.as_mut()
            && guard.redo.is_none()
        {
            guard.redo = Some(cleared);
        }
        self.commits = self.commits.wrapping_add(1);
        self.changed();
        Ok(())
    }

    /// Reverses the last change.
    ///
    /// # Errors
    /// [`EngineError::Disabled`] when there is nothing to undo; [`EngineError::Doc`] if the
    /// step no longer applies (the step is kept).
    pub fn undo(&mut self) -> Result<()> {
        let step = self.undo.pop().ok_or_else(|| disabled("undo", "nothing to undo"))?;
        match self.project.apply(step.op.clone()) {
            Ok(redo) => {
                self.redo.push(Step { label: step.label, op: redo });
                self.machine = Machine::Idle;
                self.changed();
                Ok(())
            }
            Err(e) => {
                self.undo.push(step);
                Err(e.into())
            }
        }
    }

    /// Repeats the last undone change.
    ///
    /// # Errors
    /// [`EngineError::Disabled`] when there is nothing to redo; [`EngineError::Doc`] if the
    /// step no longer applies (the step is kept).
    pub fn redo(&mut self) -> Result<()> {
        let step = self.redo.pop().ok_or_else(|| disabled("redo", "nothing to redo"))?;
        match self.project.apply(step.op.clone()) {
            Ok(undo) => {
                self.undo.push(Step { label: step.label, op: undo });
                self.machine = Machine::Idle;
                self.changed();
                Ok(())
            }
            Err(e) => {
                self.redo.push(step);
                Err(e.into())
            }
        }
    }

    /// A new id, never used in the project.
    pub(crate) fn fresh<I: Id>(&mut self) -> Result<I> {
        Ok(self.project.fresh_id()?)
    }

    /// `n` new raw ids at once (one scan of the project instead of one per id).
    pub(crate) fn fresh_raws(&mut self, n: usize) -> Result<Vec<u64>> {
        let used: HashSet<u64> = self.project.sheets.iter().flat_map(|s| std::iter::once(s.id.0).chain(s.item_ids().map(ItemId::raw))).collect();
        let mut raw = self.project.next_id.max(1);
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            if !used.contains(&raw) {
                out.push(raw);
            }
            raw = raw.checked_add(1).ok_or(protasena_doc::DocError::IdsExhausted)?;
        }
        self.project.next_id = raw;
        Ok(out)
    }

    /// Replaces the project (New/Open): selection, undo and the interactive state start over.
    pub(crate) fn replace_project(&mut self, project: Project, path: Option<PathBuf>) {
        self.project = project;
        self.path = path;
        self.dirty = false;
        self.undo.clear();
        self.redo.clear();
        self.selection.clear();
        self.machine = Machine::Idle;
        self.sheet = self.project.sheets.first().map_or(SheetId(1), |s| s.id);
        self.revision = self.revision.wrapping_add(1);
    }

    /// Marks the design changed and keeps the current sheet and the selection valid.
    pub(crate) fn changed(&mut self) {
        self.dirty = true;
        self.hover = None;
        self.revision = self.revision.wrapping_add(1);
        self.fix_sheet();
        self.prune_selection();
    }

    /// Makes sure the current sheet exists (the first sheet otherwise).
    pub(crate) fn fix_sheet(&mut self) {
        if self.project.sheet(self.sheet).is_none()
            && let Some(first) = self.project.sheets.first()
        {
            self.sheet = first.id;
            self.selection.clear();
            self.machine = Machine::Idle;
            self.hover = None;
        }
    }

    /// Drops selected items that are no longer on the current sheet.
    pub(crate) fn prune_selection(&mut self) {
        let on_sheet: HashSet<ItemId> = self.sheet().map(|s| s.item_ids().collect()).unwrap_or_default();
        self.selection.retain(|id| on_sheet.contains(id));
    }

    /// Sets the selection (duplicates and items not on the current sheet are dropped).
    pub(crate) fn set_selection(&mut self, ids: Vec<ItemId>) {
        let on_sheet: HashSet<ItemId> = self.sheet().map(|s| s.item_ids().collect()).unwrap_or_default();
        let mut seen = HashSet::new();
        self.selection = ids.into_iter().filter(|id| on_sheet.contains(id) && seen.insert(*id)).collect();
        self.revision = self.revision.wrapping_add(1);
    }

    /// The current sheet, or an error when the project has no sheets.
    pub(crate) fn current_sheet(&self) -> Result<&Sheet> {
        self.sheet().ok_or_else(|| EngineError::Other("the project has no sheets".into()))
    }

    /// Snaps a point to the current snap grid.
    pub(crate) fn snap(&self, p: Pt) -> Pt {
        snap(p, self.view.snap.step())
    }

    /// Adds a status-bar message.
    pub(crate) fn note(&mut self, msg: impl Into<String>) {
        self.log.push(msg.into());
        if self.log.len() > MAX_LOG {
            self.log.remove(0);
        }
    }

    /// Bumps the revision for a view-only change.
    pub(crate) fn touch(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}

fn disabled(cmd: &str, why: &str) -> EngineError {
    EngineError::Disabled { cmd: cmd.into(), why: why.into() }
}
