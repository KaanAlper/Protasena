//! Edit menu: Undo/Redo Changes, Select All Objects, Clear Selection; selecting by point, box
//! or id; deleting.

use std::collections::HashSet;

use protasena_doc::{ItemId, Op};
use protasena_geom::Rect;
use serde_json::{Value, json};

use super::ids_json;
use crate::args::Args;
use crate::error::{EngineError, Result};
use crate::hit::{hit, items_in};
use crate::registry::{CommandSpec, has_selection};
use crate::session::Session;

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("undo", "Undo Changes", undo).menu(&["Edit", "Undo Changes"]).key("Ctrl+Z").params("{}").enabled(can_undo),
        CommandSpec::new("redo", "Redo Changes", redo).menu(&["Edit", "Redo Changes"]).key("Ctrl+Y").params("{}").enabled(can_redo),
        CommandSpec::new("select", "Select", select)
            .params("{at?: [x, y], box?: [[x, y], [x, y]], ids?: [id], add?: bool, toggle?: bool} — one of at, box, ids")
            .machine(crate::mode::Mode::Selection),
        CommandSpec::new("select-all", "Select All Objects", select_all).menu(&["Edit", "Select All Objects"]).params("{}"),
        CommandSpec::new("clear-selection", "Clear Selection", clear_selection)
            .menu(&["Edit", "Clear Selection"])
            .params("{}")
            .enabled(has_selection),
        CommandSpec::new("delete", "Delete", delete).key("Delete").params("{ids?: [id]} — defaults to the selection; attached wires are kept"),
    ]
}

fn can_undo(s: &Session) -> std::result::Result<(), String> {
    if s.undo.is_empty() { Err("nothing to undo".into()) } else { Ok(()) }
}

fn can_redo(s: &Session) -> std::result::Result<(), String> {
    if s.redo.is_empty() { Err("nothing to redo".into()) } else { Ok(()) }
}

fn undo(s: &mut Session, _: &Args) -> Result<Value> {
    let label = s.undo.last().map(|st| st.label.clone());
    s.undo()?;
    Ok(json!({ "undone": label }))
}

fn redo(s: &mut Session, _: &Args) -> Result<Value> {
    let label = s.redo.last().map(|st| st.label.clone());
    s.redo()?;
    Ok(json!({ "redone": label }))
}

fn select(s: &mut Session, a: &Args) -> Result<Value> {
    let sheet = s.current_sheet()?;
    let picked: Vec<ItemId> = if let Some(ids) = a.ids(s, "ids")? {
        ids
    } else if let Some(p) = a.point("at")? {
        hit(sheet, &s.library, p, s.view.pick_tolerance()).map(|h| h.item()).into_iter().collect()
    } else if let Some(corners) = a.points("box")? {
        let [p, q] = corners.as_slice() else { return Err(a.bad("`box` must be two corners [[x, y], [x, y]]")) };
        items_in(sheet, &s.library, Rect::new(*p, *q))
    } else {
        return Err(a.bad("give `at`, `box` or `ids`"));
    };
    let toggle = a.bool("toggle")?.unwrap_or(false);
    let keep = a.bool("add")?.unwrap_or(false) || toggle;
    let mut selection = if keep { s.selection.clone() } else { Vec::new() };
    let mut present: HashSet<ItemId> = selection.iter().copied().collect();
    let mut removed: HashSet<ItemId> = HashSet::new();
    for id in picked {
        if !present.insert(id) {
            if toggle {
                removed.insert(id);
            }
        } else {
            selection.push(id);
        }
    }
    selection.retain(|id| !removed.contains(id));
    s.set_selection(selection);
    Ok(json!({ "ids": ids_json(&s.selection) }))
}

fn select_all(s: &mut Session, _: &Args) -> Result<Value> {
    let ids: Vec<ItemId> = s.current_sheet()?.item_ids().collect();
    s.set_selection(ids);
    Ok(json!({ "ids": ids_json(&s.selection) }))
}

fn clear_selection(s: &mut Session, _: &Args) -> Result<Value> {
    s.set_selection(Vec::new());
    Ok(json!({ "ids": [] }))
}

/// Deletes items (Proteus keeps the wires attached to a deleted part).
pub(super) fn delete_items(s: &mut Session, ids: Vec<ItemId>) -> Result<Value> {
    if ids.is_empty() {
        return Err(EngineError::Other("nothing to delete".into()));
    }
    let mut seen = HashSet::new();
    let unique: Vec<ItemId> = ids.into_iter().filter(|id| seen.insert(*id)).collect();
    let ops = unique.iter().map(|&id| Op::Remove { id }).collect();
    s.commit("Delete", ops)?;
    Ok(json!({ "deleted": ids_json(&unique) }))
}

fn delete(s: &mut Session, a: &Args) -> Result<Value> {
    let ids = a.targets(s)?;
    delete_items(s, ids)
}
