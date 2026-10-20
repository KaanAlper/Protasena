//! Placing the other schematic objects: junction dots, wire labels, terminals, buses and
//! texts, and editing labels, terminals and texts.

use protasena_doc::{Bus, Item, Junction, Label, Op, Terminal, TerminalKind, Text};
use protasena_geom::Pt;
use serde_json::{Value, json};

use crate::args::Args;
use crate::error::{EngineError, Result};
use crate::hit::closest_on;
use crate::mode::Mode;
use crate::registry::CommandSpec;
use crate::session::Session;
use crate::wiring::dedup;

/// How far from a wire a wire label's point may be, in thou.
const ON_WIRE: f64 = 1.0;

/// Largest text height, in thou.
const MAX_TEXT_SIZE: f64 = 10_000.0;

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("junction", "Junction Dot", junction).params("{at: [x, y]}").machine(Mode::JunctionDot),
        CommandSpec::new("wire-label", "Wire Label", wire_label)
            .params("{at: [x, y] on a wire, text: string, orient?: angle|{angle, mirror}}")
            .machine(Mode::WireLabel),
        CommandSpec::new("terminal", "Terminal", terminal)
            .params("{at: [x, y], kind?: \"default\"|\"input\"|\"output\"|\"bidir\"|\"power\"|\"ground\"|\"bus\", name?: string, orient?} — kind defaults to the selected one")
            .machine(Mode::Terminals),
        CommandSpec::new("select-terminal", "Select Terminal", select_terminal)
            .params("{kind: string} — the TERMINALS list: chooses the kind to place")
            .machine(Mode::Terminals),
        CommandSpec::new("bus", "Bus", bus).params("{points: [[x, y], …]} — two or more").machine(Mode::Buses),
        CommandSpec::new("text", "Text", text).params("{at: [x, y], text: string, size?: thou, orient?}").machine(Mode::Text2d),
        CommandSpec::new("edit-item", "Edit Object", edit_item)
            .params("{id, text?, name?, kind?, size?, orient?} — a wire label (text), terminal (name, kind) or text (text, size)"),
    ]
}

fn add(s: &mut Session, label: &str, item: Item) -> Result<Value> {
    let sheet = s.sheet;
    let id = item.id();
    s.commit(label, vec![Op::Add { sheet, item, index: None }])?;
    Ok(json!({ "id": id.raw() }))
}

fn junction(s: &mut Session, a: &Args) -> Result<Value> {
    let at = a.point_req("at")?;
    let id = s.fresh()?;
    add(s, "junction", Item::Junction(Junction { id, at }))
}

fn wire_label(s: &mut Session, a: &Args) -> Result<Value> {
    let at = a.point_req("at")?;
    let text = a.str_req("text")?.trim().to_owned();
    if text.is_empty() {
        return Err(a.bad("`text` is empty"));
    }
    let sheet = s.current_sheet()?;
    let on = sheet.wires.iter().filter_map(|w| closest_on(&w.points, at)).filter(|(_, d)| *d <= ON_WIRE).min_by(|x, y| x.1.total_cmp(&y.1));
    let (at, _) = on.ok_or_else(|| EngineError::Other("a wire label goes on a wire: no wire at that point".into()))?;
    let orient = a.orient("orient")?.unwrap_or_default();
    let id = s.fresh()?;
    add(s, "wire-label", Item::Label(Label { id, at, text, orient }))
}

fn kind_param(a: &Args, key: &str) -> Result<Option<TerminalKind>> {
    let Some(k) = a.str(key)? else { return Ok(None) };
    let kind = match k.to_ascii_lowercase().as_str() {
        "default" => TerminalKind::Default,
        "input" => TerminalKind::Input,
        "output" => TerminalKind::Output,
        "bidir" => TerminalKind::Bidir,
        "power" => TerminalKind::Power,
        "ground" => TerminalKind::Ground,
        "bus" => TerminalKind::Bus,
        _ => return Err(a.bad(format!("unknown terminal kind {k:?}"))),
    };
    Ok(Some(kind))
}

fn terminal(s: &mut Session, a: &Args) -> Result<Value> {
    let at = a.point_req("at")?;
    let kind = kind_param(a, "kind")?.unwrap_or(s.terminal_kind);
    let name = a.str("name")?.unwrap_or("").trim().to_owned();
    let orient = a.orient("orient")?.unwrap_or(s.placement);
    let id = s.fresh()?;
    add(s, "terminal", Item::Terminal(Terminal { id, at, kind, name, orient }))
}

fn select_terminal(s: &mut Session, a: &Args) -> Result<Value> {
    let kind = kind_param(a, "kind")?.ok_or_else(|| a.bad("`kind` is required"))?;
    s.terminal_kind = kind;
    s.mode = Mode::Terminals;
    s.machine = crate::interact::Machine::Idle;
    s.touch();
    Ok(json!({ "kind": a.str("kind")?.unwrap_or("").to_ascii_lowercase() }))
}

fn bus(s: &mut Session, a: &Args) -> Result<Value> {
    let points = dedup(a.points("points")?.ok_or_else(|| a.bad("`points` is required"))?);
    if points.len() < 2 {
        return Err(a.bad("a bus needs two or more different points"));
    }
    let id = s.fresh()?;
    add(s, "bus", Item::Bus(Bus { id, points }))
}

fn size_param(a: &Args) -> Result<Option<f64>> {
    match a.f64("size")? {
        Some(v) if !(0.0..=MAX_TEXT_SIZE).contains(&v) => Err(a.bad(format!("`size` must be between 0 and {MAX_TEXT_SIZE} thou"))),
        other => Ok(other),
    }
}

fn text(s: &mut Session, a: &Args) -> Result<Value> {
    let at: Pt = a.point_req("at")?;
    let text = a.str_req("text")?.to_owned();
    if text.trim().is_empty() {
        return Err(a.bad("`text` is empty"));
    }
    let size = size_param(a)?.unwrap_or(0.0);
    let orient = a.orient("orient")?.unwrap_or(s.placement);
    let id = s.fresh()?;
    add(s, "text", Item::Text(Text { id, at, text, size, orient }))
}

fn edit_item(s: &mut Session, a: &Args) -> Result<Value> {
    let id = a.item(s, "id")?.ok_or_else(|| a.bad("`id` (a label, terminal or text) is required"))?;
    let item = s.project.item(id).ok_or_else(|| a.bad("no such item"))?;
    let orient = a.orient("orient")?;
    let new = match item {
        Item::Label(mut l) => {
            if let Some(t) = a.str("text")? {
                let t = t.trim();
                if t.is_empty() {
                    return Err(a.bad("`text` is empty"));
                }
                l.text = t.to_owned();
            }
            l.orient = orient.unwrap_or(l.orient);
            Item::Label(l)
        }
        Item::Terminal(mut t) => {
            if let Some(n) = a.str("name")? {
                t.name = n.trim().to_owned();
            }
            t.kind = kind_param(a, "kind")?.unwrap_or(t.kind);
            t.orient = orient.unwrap_or(t.orient);
            Item::Terminal(t)
        }
        Item::Text(mut t) => {
            if let Some(x) = a.str("text")? {
                t.text = x.to_owned();
            }
            t.size = size_param(a)?.unwrap_or(t.size);
            t.orient = orient.unwrap_or(t.orient);
            Item::Text(t)
        }
        _ => return Err(a.bad("edit-item edits wire labels, terminals and texts (components: edit-component)")),
    };
    let raw = id.raw();
    s.commit("edit-item", vec![Op::Modify { item: new }])?;
    Ok(json!({ "id": raw }))
}
