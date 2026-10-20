//! Moving and turning: the Edit toolbar's Block Copy/Move/Rotate/Delete, dragging (wires
//! follow), plain moves, and the Orientation toolbar (rotate, mirror, angle).

use protasena_doc::{Item, ItemId, Op};
use protasena_geom::{Orient, Transform, Vec2};
use serde_json::{Value, json};

use super::clip::{add_copies, anchor_of};
use super::edit::delete_items;
use super::ids_json;
use crate::args::Args;
use crate::error::{EngineError, Result};
use crate::follow::{follow_ops, items_on, transformed, turned_in_place};
use crate::registry::{CommandSpec, has_selection};
use crate::session::Session;

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("block-copy", "Block Copy", block_copy)
            .params("{by?: [dx, dy]} — copies the selection; defaults to the cursor's offset from its centre")
            .enabled(has_selection),
        CommandSpec::new("block-move", "Block Move", block_move)
            .params("{by?: [dx, dy]} — moves the selection, attached wires follow; defaults to the cursor's offset")
            .enabled(has_selection),
        CommandSpec::new("block-rotate", "Block Rotate", block_rotate)
            .params("{angle?: 90|180|270|-90, mirror?: bool} — turns the selection about its centre")
            .enabled(has_selection),
        CommandSpec::new("block-delete", "Block Delete", block_delete).params("{}").enabled(has_selection),
        CommandSpec::new("move", "Move", move_items).params("{ids?: [id], by: [dx, dy]} — moves items exactly; wires do not follow"),
        CommandSpec::new("drag", "Drag", drag)
            .params("{ids?: [id], by?: [dx, dy], to?: [x, y]} — attached wires follow; `to` puts the first item's anchor there")
            .machine(crate::mode::Mode::Selection),
        CommandSpec::new("rotate-cw", "Rotate Clockwise", rotate_cw)
            .params("{ids?: [id]} — the selection, or the orientation for the next placement when nothing is selected"),
        CommandSpec::new("rotate-ccw", "Rotate Anti-clockwise", rotate_ccw).params("{ids?: [id]} — as rotate-cw"),
        CommandSpec::new("mirror-x", "X-Mirror", mirror_x).params("{ids?: [id]} — as rotate-cw"),
        CommandSpec::new("mirror-y", "Y-Mirror", mirror_y).params("{ids?: [id]} — as rotate-cw"),
        CommandSpec::new("set-orientation", "Orientation", set_orientation)
            .params("{angle: 0|90|180|270, mirror?: bool} — the Orientation toolbar's angle box (next placement)"),
    ]
}

/// The ops that replace items by new states with attached wires following.
pub(crate) fn follow_changes(s: &Session, ids: &[ItemId], change: impl Fn(&Item) -> Result<Item>) -> Result<Vec<Op>> {
    let sheet = s.current_sheet()?;
    let mut moves = Vec::new();
    for old in items_on(sheet, ids) {
        let new = change(&old)?;
        moves.push((old, new));
    }
    if moves.is_empty() {
        return Err(EngineError::Other("none of the items is on the current sheet".into()));
    }
    Ok(follow_ops(sheet, &s.library, &moves, s.view.snap.step()))
}

/// The ops of dragging `ids` by `by`.
pub(crate) fn drag_ops(s: &Session, ids: &[ItemId], by: Vec2) -> Result<Vec<Op>> {
    follow_changes(s, ids, |item| Ok(item.translated(by)?))
}

fn cursor_offset(s: &Session, a: &Args, ids: &[ItemId]) -> Result<Vec2> {
    match a.vec("by")? {
        Some(v) => Ok(v),
        None => Ok(s.snap(s.cursor) - anchor_of(s, ids)?),
    }
}

fn block_copy(s: &mut Session, a: &Args) -> Result<Value> {
    let ids = s.selection.clone();
    let by = cursor_offset(s, a, &ids)?;
    let items = items_on(s.current_sheet()?, &ids);
    let copies = add_copies(s, &items, by, "block-copy")?;
    Ok(json!({ "ids": ids_json(&copies) }))
}

fn block_move(s: &mut Session, a: &Args) -> Result<Value> {
    let ids = s.selection.clone();
    let by = cursor_offset(s, a, &ids)?;
    let ops = drag_ops(s, &ids, by)?;
    s.commit("block-move", ops)?;
    Ok(json!({ "ids": ids_json(&ids), "by": [by.x, by.y] }))
}

fn block_rotate(s: &mut Session, a: &Args) -> Result<Value> {
    let ids = s.selection.clone();
    let angle = a.f64("angle")?.unwrap_or(90.0);
    let turns = quarter_turns(a, angle)?;
    let orient = if a.bool("mirror")?.unwrap_or(false) { Orient::MIRROR_X.then(Orient::rotation(turns)) } else { Orient::rotation(turns) };
    let pivot = anchor_of(s, &ids)?;
    let t = Transform::about(pivot, orient);
    let ops = follow_changes(s, &ids, |item| Ok(transformed(item, t)?))?;
    s.commit("block-rotate", ops)?;
    Ok(json!({ "ids": ids_json(&ids), "pivot": [pivot.x, pivot.y] }))
}

fn quarter_turns(a: &Args, angle: f64) -> Result<i32> {
    let turns = angle / 90.0;
    if turns.fract() != 0.0 {
        return Err(a.bad("`angle` must be a multiple of 90"));
    }
    Ok((turns.rem_euclid(4.0)) as i32)
}

fn block_delete(s: &mut Session, _: &Args) -> Result<Value> {
    let ids = s.selection.clone();
    delete_items(s, ids)
}

fn move_items(s: &mut Session, a: &Args) -> Result<Value> {
    let ids = a.targets(s)?;
    let by = a.vec("by")?.ok_or_else(|| a.bad("`by` ([dx, dy]) is required"))?;
    if ids.is_empty() {
        return Err(EngineError::Other("nothing to move".into()));
    }
    s.commit("move", vec![Op::Move { ids: ids.clone(), by }])?;
    Ok(json!({ "ids": ids_json(&ids) }))
}

fn drag(s: &mut Session, a: &Args) -> Result<Value> {
    let ids = a.targets(s)?;
    if ids.is_empty() {
        return Err(EngineError::Other("nothing to drag".into()));
    }
    let by = match (a.vec("by")?, a.point("to")?) {
        (Some(by), _) => by,
        (None, Some(to)) => to - anchor_of(s, &ids)?,
        (None, None) => return Err(a.bad("give `by` or `to`")),
    };
    let ops = drag_ops(s, &ids, by)?;
    s.commit("drag", ops)?;
    Ok(json!({ "ids": ids_json(&ids), "by": [by.x, by.y] }))
}

/// Turns the targets in place, or the placement orientation when there are none.
fn orient_targets(s: &mut Session, a: &Args, o: Orient) -> Result<Value> {
    let ids = a.targets(s)?;
    if ids.is_empty() {
        s.placement = s.placement.then(o);
        s.touch();
        return Ok(json!({ "placement": { "angle": s.placement.angle_degrees(), "mirror": s.placement.is_mirrored() } }));
    }
    let ops = follow_changes(s, &ids, |item| Ok(turned_in_place(item, o)?))?;
    s.commit(a.cmd(), ops)?;
    Ok(json!({ "ids": ids_json(&ids) }))
}

fn rotate_cw(s: &mut Session, a: &Args) -> Result<Value> {
    orient_targets(s, a, Orient::rotation(-1))
}

fn rotate_ccw(s: &mut Session, a: &Args) -> Result<Value> {
    orient_targets(s, a, Orient::rotation(1))
}

fn mirror_x(s: &mut Session, a: &Args) -> Result<Value> {
    orient_targets(s, a, Orient::MIRROR_X)
}

fn mirror_y(s: &mut Session, a: &Args) -> Result<Value> {
    orient_targets(s, a, Orient::MIRROR_Y)
}

fn set_orientation(s: &mut Session, a: &Args) -> Result<Value> {
    let angle = a.f64("angle")?.ok_or_else(|| a.bad("`angle` (degrees) is required"))?;
    let turns = quarter_turns(a, angle)?;
    let base = if a.bool("mirror")?.unwrap_or(false) { Orient::MIRROR_X } else { Orient::IDENTITY };
    s.placement = base.then(Orient::rotation(turns));
    s.touch();
    Ok(json!({ "placement": { "angle": s.placement.angle_degrees(), "mirror": s.placement.is_mirrored() } }))
}
