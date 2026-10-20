//! Subcircuit mode and hierarchical design: subcircuit boxes and their ports, the PORTS list,
//! Edit Subcircuit, Goto Child Sheet (which makes the child sheet the first time) and Design ▸
//! Exit to Parent Sheet.
//!
//! Built the conventional way (Proteus's own clicks and dialogs were not observable): a box is
//! drawn like a 2D box, a click on its left or right edge adds a port of the kind chosen in the
//! PORTS list, and a child sheet is made empty and named after the subcircuit.

use std::collections::HashSet;

use protasena_doc::{ItemId, Op, Port, PortKind, Sheet, SheetId, Subcircuit, SubcircuitId, instance_name};
use protasena_geom::Pt;
use serde_json::{Value, json};

use super::sheets::go_to;
use crate::args::Args;
use crate::error::{EngineError, Result};
use crate::mode::Mode;
use crate::registry::CommandSpec;
use crate::session::Session;

/// How far from an edge a port's point may be, in thou.
pub const PORT_REACH: f64 = 1.0;
/// Longest subcircuit or port name, in bytes.
const MAX_NAME: usize = 256;

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("subcircuit", "Subcircuit", subcircuit)
            .params("{from: [x, y], to: [x, y], name?: string} — opposite corners; the name defaults to the next free SUBn")
            .machine(Mode::Subcircuit),
        CommandSpec::new("subcircuit-port", "Subcircuit Port", port)
            .params("{subcircuit: id, at: [x, y] on its left or right edge, name: string, kind?: \"default\"|\"input\"|\"output\"|\"bidir\"} — kind defaults to the selected one")
            .machine(Mode::Subcircuit),
        CommandSpec::new("select-port", "Select Port", select_port)
            .params("{kind: string} — the PORTS list: chooses the kind of port to place")
            .machine(Mode::Subcircuit),
        CommandSpec::new("edit-subcircuit", "Edit Subcircuit", edit)
            .params("{id, name?: string, ports?: [{name, kind?, at?}]} — the whole port list; `at` defaults to the port's at the same index"),
        CommandSpec::new("enter-child-sheet", "Goto Child Sheet", enter_child)
            .key("Ctrl+C")
            .params("{id?} — the subcircuit (default: the selected one); its child sheet is made when it has none")
            .enabled(sheet_has_subcircuits),
        CommandSpec::new("exit-to-parent", "Exit to Parent Sheet", exit_to_parent)
            .menu(&["Design", "Exit to Parent Sheet"])
            .key("Ctrl+X")
            .params("{}")
            .enabled(is_child_sheet),
    ]
}

fn sheet_has_subcircuits(s: &Session) -> std::result::Result<(), String> {
    if s.sheet().is_some_and(|sh| !sh.subcircuits.is_empty()) { Ok(()) } else { Err("there is no subcircuit on this sheet".into()) }
}

fn is_child_sheet(s: &Session) -> std::result::Result<(), String> {
    if s.project.parent_of(s.sheet).is_some() { Ok(()) } else { Err("this is a root sheet".into()) }
}

fn kind_param(a: &Args, key: &str) -> Result<Option<PortKind>> {
    let Some(k) = a.str(key)? else { return Ok(None) };
    Ok(Some(match k.to_ascii_lowercase().as_str() {
        "default" => PortKind::Default,
        "input" => PortKind::Input,
        "output" => PortKind::Output,
        "bidir" => PortKind::Bidir,
        _ => return Err(a.bad(format!("unknown port kind {k:?}"))),
    }))
}

/// The id of a port kind, as `kind` parameters take it.
pub fn port_kind_id(k: PortKind) -> &'static str {
    match k {
        PortKind::Default => "default",
        PortKind::Input => "input",
        PortKind::Output => "output",
        PortKind::Bidir => "bidir",
    }
}

fn name(a: &Args, v: &str, what: &str) -> Result<String> {
    let n = v.trim();
    if n.is_empty() {
        return Err(a.bad(format!("the {what} name is empty")));
    }
    if n.len() > MAX_NAME {
        return Err(a.bad(format!("the {what} name is longer than {MAX_NAME} bytes")));
    }
    Ok(n.to_owned())
}

/// The subcircuit named by `key` (a raw id), with a copy of it.
fn find(s: &Session, a: &Args, key: &str) -> Result<Option<Subcircuit>> {
    match a.item(s, key)? {
        None => Ok(None),
        Some(ItemId::Subcircuit(id)) => Ok(s.project.subcircuit(id).map(|(_, x)| x.clone())),
        Some(_) => Err(a.bad(format!("`{key}` is not a subcircuit"))),
    }
}

/// `SUBn` with the smallest n no subcircuit of the design uses.
fn default_name(s: &Session) -> String {
    let used: HashSet<String> = s.project.sheets.iter().flat_map(|sh| sh.subcircuits.iter().map(|x| x.name.clone())).collect();
    (1..=used.len().saturating_add(1)).map(|n| format!("SUB{n}")).find(|n| !used.contains(n)).unwrap_or_else(|| "SUB".to_owned())
}

fn subcircuit(s: &mut Session, a: &Args) -> Result<Value> {
    let (from, to) = (a.point_req("from")?, a.point_req("to")?);
    if from.x == to.x || from.y == to.y {
        return Err(a.bad("a subcircuit needs a width and a height"));
    }
    let name = match a.str("name")? {
        Some(n) => name(a, n, "subcircuit")?,
        None => default_name(s),
    };
    let id: SubcircuitId = s.fresh()?;
    let sheet = s.sheet;
    s.commit("subcircuit", vec![Op::Add { sheet, item: protasena_doc::Item::Subcircuit(Subcircuit::new(id, from, to, &name)), index: None }])?;
    Ok(json!({ "id": id.0, "name": name }))
}

/// Checks a port list: every point on an edge, names given and distinct, no two ports at one
/// point.
fn check_ports(a: &Args, x: &Subcircuit) -> Result<()> {
    let (mut names, mut points) = (HashSet::new(), HashSet::new());
    for p in &x.ports {
        if x.edge_point(p.at, PORT_REACH).is_none_or(|e| e != p.at) {
            return Err(a.bad(format!("port {} is not on the left or right edge of {}", p.name, x.name)));
        }
        if !names.insert(p.name.as_str()) {
            return Err(a.bad(format!("{} already has a port called {}", x.name, p.name)));
        }
        if !points.insert((p.at.x.to_bits(), p.at.y.to_bits())) {
            return Err(a.bad(format!("{} already has a port at that point", x.name)));
        }
    }
    if x.ports.len() > protasena_doc::MAX_PORTS {
        return Err(a.bad(format!("a subcircuit takes at most {} ports", protasena_doc::MAX_PORTS)));
    }
    Ok(())
}

fn port(s: &mut Session, a: &Args) -> Result<Value> {
    let mut x = find(s, a, "subcircuit")?.ok_or_else(|| a.bad("`subcircuit` (id) is required"))?;
    let at = a.point_req("at")?;
    let at = x.edge_point(at, PORT_REACH).ok_or_else(|| a.bad("a port goes on the left or right edge of the subcircuit"))?;
    let kind = kind_param(a, "kind")?.unwrap_or(s.port_kind);
    let port_name = name(a, a.str_req("name")?, "port")?;
    x.ports.push(Port { at, name: port_name.clone(), kind });
    check_ports(a, &x)?;
    let index = x.ports.len() - 1;
    s.commit("subcircuit-port", vec![Op::Modify { item: protasena_doc::Item::Subcircuit(x) }])?;
    Ok(json!({ "index": index, "name": port_name, "at": [at.x, at.y] }))
}

fn select_port(s: &mut Session, a: &Args) -> Result<Value> {
    let kind = kind_param(a, "kind")?.ok_or_else(|| a.bad("`kind` is required"))?;
    s.port_kind = kind;
    s.mode = Mode::Subcircuit;
    s.machine = crate::interact::Machine::Idle;
    s.touch();
    Ok(json!({ "kind": port_kind_id(kind) }))
}

fn edit(s: &mut Session, a: &Args) -> Result<Value> {
    let mut x = find(s, a, "id")?.ok_or_else(|| a.bad("`id` (a subcircuit) is required"))?;
    if let Some(n) = a.str("name")? {
        x.name = name(a, n, "subcircuit")?;
    }
    if let Some(list) = a.get("ports") {
        let list = list.as_array().ok_or_else(|| a.bad("`ports` must be a list"))?;
        if list.len() > protasena_doc::MAX_PORTS {
            return Err(a.bad(format!("a subcircuit takes at most {} ports", protasena_doc::MAX_PORTS)));
        }
        let mut ports = Vec::with_capacity(list.len());
        for (i, v) in list.iter().enumerate() {
            let p = Args::new(a.cmd(), v)?;
            let at = match p.point("at")? {
                Some(at) => x.edge_point(at, PORT_REACH).ok_or_else(|| a.bad(format!("port {i} is not on the left or right edge")))?,
                None => x.ports.get(i).map(|old| old.at).ok_or_else(|| a.bad(format!("port {i} needs `at`")))?,
            };
            let kind = kind_param(&p, "kind")?.or_else(|| x.ports.get(i).map(|old| old.kind)).unwrap_or_default();
            ports.push(Port { at, name: name(a, p.str_req("name")?, "port")?, kind });
        }
        x.ports = ports;
    }
    check_ports(a, &x)?;
    let id = x.id;
    s.commit("edit-subcircuit", vec![Op::Modify { item: protasena_doc::Item::Subcircuit(x) }])?;
    Ok(json!({ "id": id.0 }))
}

/// A sheet name no sheet has: `base`, else `base (2)`, `base (3)`, …
fn free_sheet_name(s: &Session, base: &str) -> String {
    let taken = |n: &str| s.project.sheets.iter().any(|sh| sh.name == n);
    if !taken(base) {
        return base.to_owned();
    }
    (2..=s.project.sheets.len().saturating_add(2)).map(|i| format!("{base} ({i})")).find(|n| !taken(n)).unwrap_or_else(|| base.to_owned())
}

fn enter_child(s: &mut Session, a: &Args) -> Result<Value> {
    let x = match find(s, a, "id")? {
        Some(x) => x,
        None => {
            let selected = s.selection.iter().find_map(|id| if let ItemId::Subcircuit(x) = id { Some(*x) } else { None });
            let id = selected.ok_or_else(|| EngineError::Other("select a subcircuit to go to its child sheet".into()))?;
            s.project.subcircuit(id).map(|(_, x)| x.clone()).ok_or_else(|| a.bad("no such subcircuit"))?
        }
    };
    if let Some(child) = x.child.filter(|c| s.project.sheet(*c).is_some()) {
        go_to(s, child);
        return Ok(json!({ "sheet": child.0, "created": false }));
    }
    let (parent, _) = s.project.subcircuit(x.id).ok_or_else(|| a.bad("no such subcircuit"))?;
    let (parent, size) = (parent.id, parent.size);
    let child: SheetId = s.fresh()?;
    let name = free_sheet_name(s, &instance_name(&x));
    let index = s.project.sheets.iter().position(|sh| sh.id == parent).map(|i| i.saturating_add(1));
    let linked = Subcircuit { child: Some(child), ..x };
    let ops = vec![Op::AddSheet { sheet: Sheet::new(child, &name, size), index }, Op::Modify { item: protasena_doc::Item::Subcircuit(linked) }];
    s.commit("enter-child-sheet", ops)?;
    go_to(s, child);
    Ok(json!({ "sheet": child.0, "created": true, "name": name }))
}

fn exit_to_parent(s: &mut Session, _: &Args) -> Result<Value> {
    let (parent, sub) = s.project.parent_of(s.sheet).ok_or_else(|| EngineError::Other("this is a root sheet".into()))?;
    go_to(s, parent);
    s.set_selection(vec![ItemId::Subcircuit(sub)]);
    Ok(json!({ "sheet": parent.0, "subcircuit": sub.0 }))
}

/// Where a click at `p` in Subcircuit mode adds a port: the edge point of the subcircuit under
/// it, if `p` is within `reach` of a vertical edge.
pub(crate) fn port_target(s: &Session, p: Pt, reach: f64) -> Option<(SubcircuitId, Pt)> {
    s.sheet()?.subcircuits.iter().rev().find_map(|x| x.edge_point(p, reach).map(|at| (x.id, at)))
}
