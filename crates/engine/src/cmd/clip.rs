//! Edit menu: Cut To Clipboard, Copy To Clipboard, Paste From Clipboard (the internal
//! clipboard), and the duplication Block Copy shares.

use std::collections::{HashMap, HashSet};

use protasena_doc::{Item, ItemId, Op, Project, split_reference};
use protasena_geom::{Pt, Vec2};
use serde_json::{Value, json};

use super::edit::delete_items;
use super::{ids_json, with_id};
use crate::args::Args;
use crate::error::{EngineError, Result};
use crate::follow::items_on;
use crate::hit::bounds_of;
use crate::registry::{CommandSpec, has_selection};
use crate::session::{Clipboard, Session};

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("cut", "Cut To Clipboard", cut).menu(&["Edit", "Cut To Clipboard"]).params("{}").enabled(has_selection),
        CommandSpec::new("copy", "Copy To Clipboard", copy).menu(&["Edit", "Copy To Clipboard"]).params("{}").enabled(has_selection),
        CommandSpec::new("paste", "Paste From Clipboard", paste)
            .menu(&["Edit", "Paste From Clipboard"])
            .params("{at?: [x, y]} — where the copied items' centre goes; defaults to the cursor")
            .enabled(has_clipboard),
    ]
}

fn has_clipboard(s: &Session) -> std::result::Result<(), String> {
    if s.clipboard.items.is_empty() { Err("the clipboard is empty".into()) } else { Ok(()) }
}

/// The snapped centre of some items on the current sheet.
pub(super) fn anchor_of(s: &Session, ids: &[ItemId]) -> Result<Pt> {
    let sheet = s.current_sheet()?;
    let bounds = bounds_of(sheet, &s.library, ids).ok_or_else(|| EngineError::Other("the items have no extent".into()))?;
    Ok(s.snap(bounds.center()))
}

fn copy(s: &mut Session, _: &Args) -> Result<Value> {
    let ids = s.selection.clone();
    let items = items_on(s.current_sheet()?, &ids);
    let anchor = anchor_of(s, &ids)?;
    let count = items.len();
    s.clipboard = Clipboard { items, anchor };
    s.touch();
    Ok(json!({ "copied": count }))
}

fn cut(s: &mut Session, a: &Args) -> Result<Value> {
    copy(s, a)?;
    let ids = s.selection.clone();
    delete_items(s, ids)
}

fn paste(s: &mut Session, a: &Args) -> Result<Value> {
    let at = match a.point("at")? {
        Some(p) => p,
        None => s.cursor,
    };
    let by = s.snap(at) - s.clipboard.anchor;
    let items = s.clipboard.items.clone();
    let ids = add_copies(s, &items, by, "paste")?;
    Ok(json!({ "ids": ids_json(&ids) }))
}

/// Adds copies of `items` moved by `by` to the current sheet (fresh ids; parts whose reference
/// is taken get the next free one) as one step, and selects them.
pub(super) fn add_copies(s: &mut Session, items: &[Item], by: Vec2, label: &str) -> Result<Vec<ItemId>> {
    let sheet = s.sheet;
    let raws = s.fresh_raws(items.len())?;
    let mut copies = Vec::with_capacity(items.len());
    for (item, raw) in items.iter().zip(raws) {
        copies.push(with_id(&item.translated(by)?, raw));
    }
    let copies = fresh_references(&s.project, copies);
    let ids: Vec<ItemId> = copies.iter().map(Item::id).collect();
    let ops = copies.into_iter().map(|item| Op::Add { sheet, item, index: None }).collect();
    s.commit(label, ops)?;
    s.set_selection(ids.clone());
    Ok(ids)
}

/// Gives copied parts whose reference the project already uses the next free number of their
/// prefix; gates that shared a reference keep sharing the new one.
fn fresh_references(project: &Project, items: Vec<Item>) -> Vec<Item> {
    let mut used: HashMap<String, HashSet<u64>> = HashMap::new();
    for (_, c) in project.components() {
        if let Some((prefix, Some(n))) = split_reference(&c.reference) {
            used.entry(prefix.to_owned()).or_default().insert(n);
        }
    }
    let mut renamed: HashMap<String, String> = HashMap::new();
    items
        .into_iter()
        .map(|item| match item {
            Item::Component(mut c) => {
                if let Some((prefix, Some(n))) = split_reference(&c.reference) {
                    let prefix = prefix.to_owned();
                    let new = match renamed.get(c.reference.trim()) {
                        Some(r) => r.clone(),
                        None => {
                            let taken = used.entry(prefix.clone()).or_default();
                            let number = if taken.contains(&n) { (1u64..).find(|k| !taken.contains(k)).unwrap_or(n) } else { n };
                            taken.insert(number);
                            let r = format!("{prefix}{number}");
                            renamed.insert(c.reference.trim().to_owned(), r.clone());
                            r
                        }
                    };
                    c.reference = new;
                }
                Item::Component(c)
            }
            other => other,
        })
        .collect()
}
