//! Hierarchical designs: subcircuit ports join the child sheet's terminals by name, child
//! sheets' names are prefixed by the instance path unless global, and broken hierarchies are
//! reported.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::*;
use protasena_doc::{PortKind, SheetId, TerminalKind};
use protasena_netlist::{ErcKind, Netlist};

/// R1 on the root drives port IN of SUB1 (box -500..500 at x = 1000..2000); the child has an
/// input terminal IN wired to R2 pin 1 and an output terminal OUT from R2 pin 2 to port OUT,
/// which is wired to R3 on the root.
fn two_level() -> (Design, SheetId) {
    let mut d = Design::new();
    let child = d.sheet_named("SUB1");
    d.part("RES", "R1", (0.0, 0.0)); // pins at x = -200 and 200
    d.wire(&[(200.0, 0.0), (1000.0, 0.0)]);
    d.subcircuit(
        (1000.0, -500.0),
        (2000.0, 500.0),
        "SUB1",
        &[((1000.0, 0.0), "IN", PortKind::Input), ((2000.0, 0.0), "OUT", PortKind::Output)],
        Some(child),
    );
    d.wire(&[(2000.0, 0.0), (2800.0, 0.0)]);
    d.part("RES", "R3", (3000.0, 0.0));
    let root = d.sheet;
    d.sheet = child;
    d.terminal((-600.0, 0.0), TerminalKind::Input, "IN");
    d.wire(&[(-600.0, 0.0), (-200.0, 0.0)]);
    d.part("RES", "R2", (0.0, 0.0));
    d.wire(&[(200.0, 0.0), (600.0, 0.0)]);
    d.terminal((600.0, 0.0), TerminalKind::Output, "OUT");
    d.sheet = root;
    (d, child)
}

fn hierarchy_messages(n: &Netlist) -> Vec<String> {
    n.erc_of(ErcKind::Hierarchy).map(|e| e.message.clone()).collect()
}

#[test]
fn ports_join_the_child_sheets_terminals_by_name() {
    let (d, _) = two_level();
    let n = d.netlist();
    assert_eq!(net_nodes(&n, "R1", "2"), ["R1.2", "R2.1"]);
    assert_eq!(net_nodes(&n, "R2", "2"), ["R2.2", "R3.1"]);
    assert!(hierarchy_messages(&n).is_empty(), "{:?}", hierarchy_messages(&n));
    assert!(n.net_of("R1", "2").unwrap().name.starts_with('#'), "port terminals give no name of their own");
}

#[test]
fn child_names_are_local_to_the_instance_unless_global() {
    let (mut d, child) = two_level();
    d.label((400.0, 0.0), "LINK"); // on the root wire R1 → SUB1.IN
    d.sheet = child;
    d.label((400.0, 0.0), "LINK"); // on the child wire R2 → OUT: local, SUB1.LINK
    d.part("RES", "R4", (0.0, 1000.0));
    d.wire(&[(200.0, 1000.0), (600.0, 1000.0)]);
    d.global_label((600.0, 1000.0), "CLK");
    d.terminal((-200.0, 1000.0), TerminalKind::Ground, "");
    d.part("RES", "R5", (0.0, 2000.0));
    d.terminal((-200.0, 2000.0), TerminalKind::Default, "LOCAL");
    d.sheet = d.p.sheets[0].id;
    d.part("RES", "R6", (0.0, 3000.0));
    d.wire(&[(200.0, 3000.0), (600.0, 3000.0)]);
    d.label((600.0, 3000.0), "CLK");
    d.terminal((-200.0, 3000.0), TerminalKind::Ground, "");
    let n = d.netlist();
    assert_eq!(net_name(&n, "R1", "2").as_deref(), Some("LINK"));
    assert_eq!(net_name(&n, "R2", "2").as_deref(), Some("SUB1.LINK"));
    assert_eq!(net_nodes(&n, "R4", "2"), ["R4.2", "R6.2"], "a global label joins the root's label");
    assert_eq!(net_name(&n, "R4", "2").as_deref(), Some("CLK"));
    assert_eq!(net_nodes(&n, "R4", "1"), ["R4.1", "R6.1"], "ground is global");
    assert_eq!(net_name(&n, "R5", "1").as_deref(), Some("SUB1.LOCAL"));
}

#[test]
fn nested_instances_prefix_the_whole_path() {
    let (mut d, child) = two_level();
    let grandchild = d.sheet_named("INNER");
    d.sheet = child;
    d.subcircuit((2000.0, -500.0), (3000.0, 500.0), "CORE", &[((2000.0, 0.0), "A", PortKind::Bidir)], Some(grandchild));
    d.part("RES", "R7", (2000.0, 1000.0));
    d.wire(&[(1800.0, 1000.0), (1800.0, 0.0), (2000.0, 0.0)]);
    d.sheet = grandchild;
    d.terminal((-600.0, 0.0), TerminalKind::Bidir, "A");
    d.wire(&[(-600.0, 0.0), (-200.0, 0.0)]);
    d.part("RES", "R8", (0.0, 0.0));
    d.wire(&[(200.0, 0.0), (600.0, 0.0)]);
    d.label((600.0, 0.0), "TAP");
    let n = d.netlist();
    assert_eq!(net_nodes(&n, "R7", "1"), ["R7.1", "R8.1"]);
    assert_eq!(net_name(&n, "R8", "2").as_deref(), Some("SUB1.CORE.TAP"));
}

#[test]
fn broken_hierarchies_are_reported_and_still_netlisted() {
    let (mut d, child) = two_level();
    // A port the child has no terminal for, a subcircuit without a child, a second subcircuit
    // naming the same child sheet and a repeated instance name.
    let root = d.sheet;
    let sub = d.p.sheets[0].subcircuits[0].id;
    let mut s = d.p.sheets[0].subcircuits[0].clone();
    s.ports.push(protasena_doc::Port { at: common::pt(1000.0, 200.0), name: "EN".into(), kind: PortKind::Input });
    d.p.apply(protasena_doc::Op::Modify { item: protasena_doc::Item::Subcircuit(s) }).unwrap();
    d.subcircuit((0.0, 2000.0), (500.0, 2500.0), "SUB2", &[], None);
    d.subcircuit((1000.0, 2000.0), (1500.0, 2500.0), "SUB1", &[((1000.0, 2200.0), "IN", PortKind::Input)], Some(child));
    let n = d.netlist();
    let msgs = hierarchy_messages(&n);
    assert_eq!(msgs.len(), 4, "{msgs:?}");
    assert!(msgs.iter().any(|m| m.contains("port EN of SUB1")), "{msgs:?}");
    assert!(msgs.iter().any(|m| m.contains("SUB2") && m.contains("no child sheet")), "{msgs:?}");
    assert!(msgs.iter().any(|m| m.contains("already used")), "{msgs:?}");
    assert!(msgs.iter().any(|m| m.contains("2 subcircuits")), "{msgs:?}");
    assert_eq!(net_nodes(&n, "R1", "2"), ["R1.2", "R2.1"], "the first subcircuit still connects");
    let _ = (root, sub);
}

#[test]
fn a_loop_of_subcircuits_never_hangs() {
    let mut d = Design::new();
    let a = d.sheet;
    let b = d.sheet_named("B");
    d.subcircuit((0.0, 0.0), (500.0, 500.0), "TOB", &[((0.0, 200.0), "X", PortKind::Input)], Some(b));
    d.sheet = b;
    d.subcircuit((0.0, 0.0), (500.0, 500.0), "TOA", &[], Some(a));
    d.terminal((0.0, 0.0), TerminalKind::Input, "X");
    let n = d.netlist();
    assert!(!hierarchy_messages(&n).is_empty());
}
