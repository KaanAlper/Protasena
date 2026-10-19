//! Edits as reversible operations: [`Project::apply`] performs an [`Op`] and returns the op that
//! undoes it, which the engine keeps for undo (and applying that one returns the redo).

use std::collections::HashSet;

use protasena_geom::{Transform, Vec2};

use crate::error::DocError;
use crate::ids::{ComponentId, ItemId, SheetId};
use crate::item::Item;
use crate::project::{DesignProperties, Project};
use crate::sheet::{Sheet, SheetSize};

/// How deeply [`Op::Batch`]es may nest.
pub const MAX_BATCH_DEPTH: usize = 64;

/// One edit of a project.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(tag = "op", rename_all = "snake_case"))]
pub enum Op {
    /// Adds an item to a sheet, at `index` in its kind's list (clamped; `None` appends). Its id
    /// must be unused in the project.
    Add {
        /// The sheet.
        sheet: SheetId,
        /// The new item.
        item: Item,
        /// Position in the list.
        #[cfg_attr(feature = "serde", serde(default))]
        index: Option<usize>,
    },
    /// Removes an item.
    Remove {
        /// The item.
        id: ItemId,
    },
    /// Replaces the item with the same id.
    Modify {
        /// The new state of the item.
        item: Item,
    },
    /// Moves items by an offset (duplicate ids move once).
    Move {
        /// The items.
        ids: Vec<ItemId>,
        /// The offset.
        by: Vec2,
    },
    /// Sets a component's placement (rotate, mirror, place).
    SetTransform {
        /// The component.
        id: ComponentId,
        /// The new placement.
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_impl::transform"))]
        transform: Transform,
    },
    /// Adds a sheet with its contents at `index` (clamped; `None` appends).
    AddSheet {
        /// The sheet; its id and every item id must be unused.
        sheet: Sheet,
        /// Position in the sheet list.
        #[cfg_attr(feature = "serde", serde(default))]
        index: Option<usize>,
    },
    /// Removes a sheet with its contents.
    RemoveSheet {
        /// The sheet.
        id: SheetId,
    },
    /// Renames and resizes a sheet.
    SetSheet {
        /// The sheet.
        id: SheetId,
        /// New name.
        name: String,
        /// New size.
        size: SheetSize,
    },
    /// Sets the project name and design properties.
    SetDesign {
        /// New project name.
        name: String,
        /// New design properties.
        design: DesignProperties,
    },
    /// Several ops as one step: all are applied or, on the first error, none.
    Batch {
        /// The ops, in order.
        ops: Vec<Op>,
    },
}

impl Op {
    /// A batch of ops.
    pub fn batch(ops: Vec<Op>) -> Op {
        Op::Batch { ops }
    }

    /// Whether the op changes nothing (an empty batch, or batches of empty batches).
    pub fn is_empty(&self) -> bool {
        match self {
            Op::Batch { ops } => ops.iter().all(Op::is_empty),
            Op::Move { ids, .. } => ids.is_empty(),
            _ => false,
        }
    }
}

impl Project {
    /// Applies an op and returns its inverse. On error the project is unchanged.
    ///
    /// # Errors
    /// [`DocError`] naming the first problem: a missing sheet or item, a used id, a non-finite
    /// coordinate, a malformed item, or batches nested deeper than [`MAX_BATCH_DEPTH`].
    pub fn apply(&mut self, op: Op) -> Result<Op, DocError> {
        self.apply_at(op, 0)
    }

    fn apply_at(&mut self, op: Op, depth: usize) -> Result<Op, DocError> {
        match op {
            Op::Add { sheet, item, index } => self.add(sheet, item, index),
            Op::Remove { id } => self.remove(id),
            Op::Modify { item } => self.modify(item),
            Op::Move { ids, by } => self.move_items(ids, by),
            Op::SetTransform { id, transform } => self.set_transform(id, transform),
            Op::AddSheet { sheet, index } => self.add_sheet(sheet, index),
            Op::RemoveSheet { id } => {
                let i = self.sheet_index(id).ok_or(DocError::NoSuchSheet(id))?;
                let sheet = self.sheets.remove(i);
                Ok(Op::AddSheet { sheet, index: Some(i) })
            }
            Op::SetSheet { id, name, size } => {
                size.check()?;
                let sheet = self.sheets.iter_mut().find(|s| s.id == id).ok_or(DocError::NoSuchSheet(id))?;
                let old = Op::SetSheet { id, name: std::mem::replace(&mut sheet.name, name), size: sheet.size };
                sheet.size = size;
                Ok(old)
            }
            Op::SetDesign { name, design } => {
                Ok(Op::SetDesign { name: std::mem::replace(&mut self.name, name), design: std::mem::replace(&mut self.design, design) })
            }
            Op::Batch { ops } => self.batch(ops, depth),
        }
    }

    fn add(&mut self, sheet: SheetId, item: Item, index: Option<usize>) -> Result<Op, DocError> {
        item.check()?;
        let id = item.id();
        if self.used_ids().contains(&id.raw()) {
            return Err(DocError::DuplicateItem(id));
        }
        let target = self.sheets.iter_mut().find(|s| s.id == sheet).ok_or(DocError::NoSuchSheet(sheet))?;
        target.insert(item, index);
        self.reserve(id.raw());
        Ok(Op::Remove { id })
    }

    fn remove(&mut self, id: ItemId) -> Result<Op, DocError> {
        let sheet = self.sheets.iter_mut().find(|s| s.index_of(id).is_some()).ok_or(DocError::NoSuchItem(id))?;
        let (item, index) = sheet.remove(id).ok_or(DocError::NoSuchItem(id))?;
        Ok(Op::Add { sheet: sheet.id, item, index: Some(index) })
    }

    fn modify(&mut self, item: Item) -> Result<Op, DocError> {
        item.check()?;
        let id = item.id();
        let sheet = self.sheets.iter_mut().find(|s| s.index_of(id).is_some()).ok_or(DocError::NoSuchItem(id))?;
        let old = sheet.replace(item).ok_or(DocError::NoSuchItem(id))?;
        Ok(Op::Modify { item: old })
    }

    fn move_items(&mut self, ids: Vec<ItemId>, by: Vec2) -> Result<Op, DocError> {
        let mut seen = HashSet::new();
        let ids: Vec<ItemId> = ids.into_iter().filter(|id| seen.insert(*id)).collect();
        // Work out every moved item first, so a refused move changes nothing.
        let mut moves = Vec::with_capacity(ids.len());
        for &id in &ids {
            let old = self.item(id).ok_or(DocError::NoSuchItem(id))?;
            let new = old.translated(by)?;
            moves.push((old, new));
        }
        let exact = moves.iter().all(|(old, new)| new.translated(-by).as_ref() == Ok(old));
        let mut undo = Vec::with_capacity(moves.len());
        for (old, new) in moves {
            if let Some(sheet) = self.sheets.iter_mut().find(|s| s.index_of(new.id()).is_some()) {
                sheet.replace(new);
            }
            undo.push(Op::Modify { item: old });
        }
        // Moving back is exact for grid coordinates; otherwise restore the old items verbatim.
        Ok(if exact { Op::Move { ids, by: -by } } else { Op::batch(undo) })
    }

    fn set_transform(&mut self, id: ComponentId, transform: Transform) -> Result<Op, DocError> {
        if !transform.offset.is_finite() {
            return Err(DocError::NonFinite("component position"));
        }
        let c = self.sheets.iter_mut().flat_map(|s| &mut s.components).find(|c| c.id == id);
        let c = c.ok_or(DocError::NoSuchItem(ItemId::Component(id)))?;
        Ok(Op::SetTransform { id, transform: std::mem::replace(&mut c.transform, transform) })
    }

    fn add_sheet(&mut self, sheet: Sheet, index: Option<usize>) -> Result<Op, DocError> {
        sheet.size.check()?;
        let mut used = self.used_ids();
        if !used.insert(sheet.id.0) {
            return Err(DocError::DuplicateSheet(sheet.id));
        }
        let mut max_raw = sheet.id.0;
        for id in sheet.item_ids() {
            if !used.insert(id.raw()) {
                return Err(DocError::DuplicateItem(id));
            }
            max_raw = max_raw.max(id.raw());
            sheet.item(id).ok_or(DocError::NoSuchItem(id))?.check()?;
        }
        let id = sheet.id;
        let at = index.map_or(self.sheets.len(), |i| i.min(self.sheets.len()));
        self.sheets.insert(at, sheet);
        self.reserve(max_raw);
        Ok(Op::RemoveSheet { id })
    }

    fn batch(&mut self, ops: Vec<Op>, depth: usize) -> Result<Op, DocError> {
        if depth >= MAX_BATCH_DEPTH {
            return Err(DocError::BatchTooDeep);
        }
        // Undo never lowers the id counter (a redo must not clash with a newer item), but a
        // refused batch never happened, so its reservations are dropped.
        let next_id = self.next_id;
        let mut undo = Vec::with_capacity(ops.len());
        for op in ops {
            match self.apply_at(op, depth + 1) {
                Ok(inverse) => undo.push(inverse),
                Err(e) => {
                    for inverse in undo.into_iter().rev() {
                        if let Err(rollback) = self.apply_at(inverse, depth + 1) {
                            return Err(DocError::Invalid(format!("{e}; undoing the batch then failed: {rollback}")));
                        }
                    }
                    self.next_id = next_id;
                    return Err(e);
                }
            }
        }
        undo.reverse();
        Ok(Op::batch(undo))
    }
}
