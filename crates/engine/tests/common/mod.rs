//! Shared helpers for the engine tests.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use protasena_doc::{Project, Sheet};
use protasena_engine::Session;
use protasena_geom::Pt;
use serde_json::{Value, json};

/// Runs a command and panics with its error.
pub fn exec(s: &mut Session, id: &str, params: Value) -> Value {
    match s.execute(id, &params) {
        Ok(v) => v,
        Err(e) => panic!("{id} {params}: {e}"),
    }
}

/// A point.
pub fn p(x: f64, y: f64) -> Pt {
    Pt::new(x, y)
}

/// The design without the id counter (undo never lowers it).
pub fn design(project: &Project) -> (String, protasena_doc::DesignProperties, Vec<Sheet>) {
    (project.name.clone(), project.design.clone(), project.sheets.clone())
}

/// The current sheet.
pub fn sheet(s: &Session) -> &Sheet {
    s.sheet().unwrap()
}

/// A session with the resistor chosen for placement.
pub fn with_res() -> Session {
    let mut s = Session::new();
    exec(&mut s, "pick-device", json!({ "pick": "RES" }));
    s
}

/// Two resistors in series: R1 at (0, 0), R2 at (1000, 500), wired R1.2 → R2.1 (routed), and a
/// ground terminal on R1.1.
pub fn divider() -> Session {
    let mut s = with_res();
    exec(&mut s, "place-component", json!({ "at": [0, 0] }));
    exec(&mut s, "place-component", json!({ "at": [1000, 500] }));
    exec(&mut s, "wire", json!({ "from": { "component": "R1", "pin": "2" }, "to": { "component": "R2", "pin": "1" } }));
    exec(&mut s, "terminal", json!({ "at": [-400, 0], "kind": "ground" }));
    exec(&mut s, "wire", json!({ "points": [[-400, 0], [-200, 0]] }));
    s
}

/// The nets as sets of `reference.pin`, independent of names and order.
pub fn nets(s: &Session) -> BTreeSet<BTreeSet<String>> {
    let n = protasena_netlist::build(s.project(), s.library());
    n.nets.iter().map(|net| net.nodes.iter().map(|node| format!("{}.{}", node.reference, node.pin)).collect()).collect()
}

/// The net names with their nodes.
pub fn named_nets(s: &Session) -> BTreeSet<(String, BTreeSet<String>)> {
    let n = protasena_netlist::build(s.project(), s.library());
    n.nets.iter().map(|net| (net.name.clone(), net.nodes.iter().map(|node| format!("{}.{}", node.reference, node.pin)).collect())).collect()
}

/// Whether every segment of every wire on the sheet is horizontal or vertical.
pub fn all_orthogonal(s: &Session) -> bool {
    sheet(s).wires.iter().all(|w| w.points.windows(2).all(|seg| seg[0].x == seg[1].x || seg[0].y == seg[1].y))
}

/// An empty scratch directory for one test.
pub fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("protasena-engine").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
