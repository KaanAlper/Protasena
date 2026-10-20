//! Edit ▸ Align Objects (Ctrl+A), Send To Back (Ctrl+B), Bring To Front (Ctrl+F) and Tidy
//! Design. Built the conventional way (Proteus's behaviour was not observable):
//! - **Align** moves each selected object so that its left, right, top or bottom edge, or its
//!   horizontal or vertical centre, lines up with that of the whole selection; each move is
//!   rounded to the snap grid (parts stay on the grid), and attached wires follow.
//! - **Send To Back / Bring To Front** put the selected objects first or last in the drawing
//!   order of their kind (objects of different kinds keep the order the sheet is drawn in:
//!   wires under parts, texts and junction dots on top), keeping their order among themselves.
//! - **Tidy Design** removes what nothing uses: devices in the DEVICES list that no part of
//!   the design uses (the list is not part of the design, so this is not undone), and from
//!   every sheet junction dots on no wire or bus, wires of zero length and empty texts (one
//!   undo step).

use std::collections::HashMap;

use protasena_doc::{DeviceRef, ItemId, Op, Project, Sheet};
use protasena_geom::{Rect, Vec2};
use serde_json::{Value, json};

use super::block::follow_changes;
use super::ids_json;
use crate::args::Args;
use crate::error::{EngineError, Result};
use crate::hit::{closest_on, item_bounds};
use crate::registry::{CommandSpec, has_selection};
use crate::request::Request;
use crate::session::Session;

/// How far from a wire a junction dot may be and still be on it, in thou.
const ON_WIRE: f64 = 1.0;

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("align", "Align Objects", align)
            .menu(&["Edit", "Align Objects"])
            .key("Ctrl+A")
            .params("{edge?: \"left\"|\"right\"|\"top\"|\"bottom\"|\"center-x\"|\"center-y\" (default left), ids?: [id]} — lines the objects up with the selection's edge or centre; moves snap to the grid")
            .enabled(has_selection)
            .dialog(|_| Some(Request::Align)),
        CommandSpec::new("send-to-back", "Send To Back", send_to_back)
            .menu(&["Edit", "Send To Back"])
            .key("Ctrl+B")
            .params("{ids?: [id]} — first in the drawing order of their kind")
            .enabled(has_selection),
        CommandSpec::new("bring-to-front", "Bring To Front", bring_to_front)
            .menu(&["Edit", "Bring To Front"])
            .key("Ctrl+F")
            .params("{ids?: [id]} — last in the drawing order of their kind")
            .enabled(has_selection),
        CommandSpec::new("tidy", "Tidy Design", tidy)
            .menu(&["Edit", "Tidy Design"])
            .params("{} — drops unused devices from the DEVICES list; removes junction dots on no wire, zero-length wires and empty texts"),
    ]
}

/// An alignment: which coordinate of a box lines up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    Left,
    Right,
    Top,
    Bottom,
    CenterX,
    CenterY,
}

impl Edge {
    fn parse(name: &str) -> Option<Edge> {
        Some(match name.trim().to_ascii_lowercase().as_str() {
            "left" => Edge::Left,
            "right" => Edge::Right,
            "top" => Edge::Top,
            "bottom" => Edge::Bottom,
            "center-x" | "centre-x" => Edge::CenterX,
            "center-y" | "centre-y" => Edge::CenterY,
            _ => return None,
        })
    }

    /// The coordinate of `r` that lines up, and whether it is an x coordinate.
    fn of(self, r: Rect) -> (f64, bool) {
        match self {
            Edge::Left => (r.min.x, true),
            Edge::Right => (r.max.x, true),
            Edge::CenterX => (r.center().x, true),
            Edge::Bottom => (r.min.y, false),
            Edge::Top => (r.max.y, false),
            Edge::CenterY => (r.center().y, false),
        }
    }
}

fn align(s: &mut Session, a: &Args) -> Result<Value> {
    let edge = match a.str("edge")? {
        None => Edge::Left,
        Some(e) => Edge::parse(e).ok_or_else(|| a.bad(format!("unknown `edge` {e:?}: left, right, top, bottom, center-x or center-y")))?,
    };
    let ids = a.targets(s)?;
    let sheet = s.current_sheet()?;
    let boxes: Vec<(ItemId, Rect)> = ids.iter().filter_map(|&id| Some((id, item_bounds(sheet, &s.library, id)?))).collect();
    if boxes.len() < 2 {
        return Err(EngineError::Other("select two or more objects to align".into()));
    }
    let all = boxes.iter().map(|(_, r)| *r).reduce(Rect::union).ok_or_else(|| EngineError::Other("nothing to align".into()))?;
    let (target, horizontal) = edge.of(all);
    let step = s.view.snap.step();
    let moves: HashMap<ItemId, Vec2> = boxes
        .iter()
        .map(|&(id, r)| {
            let d = ((target - edge.of(r).0) / step).round() * step;
            (id, if horizontal { Vec2::new(d, 0.0) } else { Vec2::new(0.0, d) })
        })
        .filter(|(_, v)| v.x != 0.0 || v.y != 0.0)
        .collect();
    if moves.is_empty() {
        return Ok(json!({ "moved": [] }));
    }
    let moved: Vec<ItemId> = boxes.iter().map(|(id, _)| *id).filter(|id| moves.contains_key(id)).collect();
    let ops = follow_changes(s, &moved, |item| Ok(item.translated(moves.get(&item.id()).copied().unwrap_or(Vec2::ZERO))?))?;
    s.commit("align", ops)?;
    Ok(json!({ "moved": ids_json(&moved) }))
}

/// Moves the items to the start (`front` false) or end of their kind's list.
fn restack(s: &mut Session, a: &Args, front: bool) -> Result<Value> {
    let ids = a.targets(s)?;
    let sheet = s.current_sheet()?;
    let mut placed: Vec<(usize, ItemId)> = ids.iter().filter_map(|&id| Some((sheet.index_of(id)?, id))).collect();
    if placed.is_empty() {
        return Err(EngineError::Other("nothing selected on this sheet".into()));
    }
    placed.sort_by_key(|(i, _)| *i);
    if !front {
        placed.reverse();
    }
    let index = if front { None } else { Some(0) };
    let mut ops = Vec::new();
    for (_, id) in &placed {
        let item = sheet.item(*id).ok_or_else(|| EngineError::Other("the object is gone".into()))?;
        ops.push(Op::Remove { id: *id });
        ops.push(Op::Add { sheet: sheet.id, item, index });
    }
    let mut trial: Project = s.project.clone();
    trial.apply(Op::batch(ops.clone()))?;
    if trial.sheet(s.sheet) == s.sheet() {
        return Ok(json!({ "ids": [], "unchanged": true }));
    }
    s.commit(a.cmd(), ops)?;
    let ids: Vec<ItemId> = placed.into_iter().map(|(_, id)| id).collect();
    Ok(json!({ "ids": ids_json(&ids) }))
}

fn send_to_back(s: &mut Session, a: &Args) -> Result<Value> {
    restack(s, a, false)
}

fn bring_to_front(s: &mut Session, a: &Args) -> Result<Value> {
    restack(s, a, true)
}

/// Whether a part of the design uses `d`.
fn used(project: &Project, d: &DeviceRef) -> bool {
    project.components().any(|(_, c)| {
        c.device.name.eq_ignore_ascii_case(&d.name) && (c.device.library.is_empty() || d.library.is_empty() || c.device.library == d.library)
    })
}

/// What nothing uses on a sheet: junction dots on no wire or bus, zero-length wires, empty texts.
fn unused(sheet: &Sheet) -> Vec<ItemId> {
    let on_line = |at| {
        sheet
            .wires
            .iter()
            .map(|w| &w.points)
            .chain(sheet.buses.iter().map(|b| &b.points))
            .any(|pts| closest_on(pts, at).is_some_and(|(_, d)| d <= ON_WIRE))
    };
    let dots = sheet.junctions.iter().filter(|j| !on_line(j.at)).map(|j| ItemId::Junction(j.id));
    let wires = sheet.wires.iter().filter(|w| w.points.windows(2).all(|p| p.first() == p.last())).map(|w| ItemId::Wire(w.id));
    let texts = sheet.texts.iter().filter(|t| t.text.trim().is_empty()).map(|t| ItemId::Text(t.id));
    dots.chain(wires).chain(texts).collect()
}

fn tidy(s: &mut Session, _: &Args) -> Result<Value> {
    let gone: Vec<ItemId> = s.project.sheets.iter().flat_map(unused).collect();
    s.commit("tidy", gone.iter().map(|&id| Op::Remove { id }).collect())?;
    let (keep, drop): (Vec<DeviceRef>, Vec<DeviceRef>) = s.picked.iter().cloned().partition(|d| used(&s.project, d));
    if s.device.as_ref().is_some_and(|d| drop.contains(d)) {
        s.device = None;
    }
    s.picked = keep;
    s.touch();
    let devices: Vec<&str> = drop.iter().map(|d| d.name.as_str()).collect();
    Ok(json!({ "removed": ids_json(&gone), "devices_removed": devices }))
}
