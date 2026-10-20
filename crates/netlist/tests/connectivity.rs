//! Connection rules: pins, wire ends, T joints, junctions, crossings, transforms.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::*;
use protasena_geom::Orient;
use protasena_netlist::{ErcKind, Node};

#[test]
fn a_wire_joins_two_pins_and_leaves_the_others_unconnected() {
    let mut d = Design::new();
    d.part("RES", "R1", (0.0, 0.0));
    d.part("RES", "R2", (1000.0, 0.0));
    d.wire(&[(200.0, 0.0), (500.0, 0.0), (500.0, 300.0), (800.0, 300.0), (800.0, 0.0)]);
    let n = d.netlist();
    assert_eq!(net_nodes(&n, "R1", "2"), ["R1.2", "R2.1"]);
    assert_eq!(n.nets.len(), 1);
    assert_eq!(n.unconnected_pins, [Node::new("R1", "1"), Node::new("R2", "2")]);
    assert_eq!(n.erc_of(ErcKind::UnconnectedPin).count(), 2);
    assert_eq!(n.part("R1").unwrap().pins[1].net.as_deref(), Some("#NET0001"));
    assert_eq!(n.part("R1").unwrap().pins[0].net, None);
}

#[test]
fn touching_pin_ends_connect_without_a_wire() {
    let mut d = Design::new();
    d.part("RES", "R1", (0.0, 0.0));
    d.part("RES", "R2", (400.0, 0.0));
    assert_eq!(net_nodes(&d.netlist(), "R1", "2"), ["R1.2", "R2.1"]);
}

#[test]
fn a_wire_ending_on_another_wire_makes_a_t() {
    let mut d = Design::new();
    d.part("RES", "R1", (0.0, 0.0));
    d.part("RES", "R2", (1200.0, 0.0));
    d.part_at("RES", "R3", "", (600.0, -700.0), Orient::rotation(1)); // pin 2 at (600, -500)
    d.wire(&[(200.0, 0.0), (1000.0, 0.0)]);
    d.wire(&[(600.0, -500.0), (600.0, 0.0)]);
    assert_eq!(net_nodes(&d.netlist(), "R1", "2"), ["R1.2", "R2.1", "R3.2"]);
}

#[test]
fn crossing_wires_connect_only_through_a_junction() {
    let mut d = Design::new();
    d.part("RES", "R1", (0.0, 0.0));
    d.part("RES", "R2", (1200.0, 0.0));
    d.part_at("RES", "R3", "", (600.0, -700.0), Orient::rotation(1));
    d.part_at("RES", "R4", "", (600.0, 700.0), Orient::rotation(1)); // pin 1 at (600, 500)
    d.wire(&[(200.0, 0.0), (1000.0, 0.0)]);
    d.wire(&[(600.0, -500.0), (600.0, 500.0)]);
    let n = d.netlist();
    assert_eq!(net_nodes(&n, "R1", "2"), ["R1.2", "R2.1"]);
    assert_eq!(net_nodes(&n, "R3", "2"), ["R3.2", "R4.1"]);
    d.junction((600.0, 0.0));
    assert_eq!(net_nodes(&d.netlist(), "R1", "2"), ["R1.2", "R2.1", "R3.2", "R4.1"]);
}

#[test]
fn bends_meeting_at_one_point_do_not_connect() {
    let mut d = Design::new();
    d.part("RES", "R1", (0.0, 0.0));
    d.part("RES", "R2", (0.0, 1000.0));
    // Both wires bend at (500, 500) but neither ends there.
    d.wire(&[(200.0, 0.0), (500.0, 0.0), (500.0, 500.0), (1000.0, 500.0)]);
    d.wire(&[(200.0, 1000.0), (500.0, 1000.0), (500.0, 500.0), (0.0, 500.0)]);
    let n = d.netlist();
    assert_eq!(net_nodes(&n, "R1", "2"), ["R1.2"]);
    assert_eq!(net_nodes(&n, "R2", "2"), ["R2.2"]);
    assert_eq!(n.erc_of(ErcKind::SingleNodeNet).count(), 2);
}

#[test]
fn a_pin_end_on_a_wire_interior_connects() {
    let mut d = Design::new();
    d.part("RES", "R1", (0.0, 0.0));
    d.part("RES", "R2", (1600.0, 0.0));
    d.part_at("RES", "R3", "", (800.0, 200.0), Orient::rotation(1)); // pin 1 at (800, 0)
    d.wire(&[(200.0, 0.0), (1400.0, 0.0)]);
    assert_eq!(net_nodes(&d.netlist(), "R3", "1"), ["R1.2", "R2.1", "R3.1"]);
}

#[test]
fn rotation_and_mirror_move_the_pins() {
    for (orient, ends) in [
        (Orient::rotation(1), [(1000.0, 800.0), (1000.0, 1200.0)]),
        (Orient::rotation(2), [(1200.0, 1000.0), (800.0, 1000.0)]),
        (Orient::MIRROR_X, [(1200.0, 1000.0), (800.0, 1000.0)]),
        (Orient::rotation(3).mirror_y(), [(1000.0, 800.0), (1000.0, 1200.0)]),
    ] {
        let mut d = Design::new();
        d.part_at("RES", "R1", "", (1000.0, 1000.0), orient);
        d.part("RES", "R2", (3000.0, 0.0)); // pin 1 at (2800, 0)
        d.wire(&[ends[1], (ends[1].0, 3000.0), (2800.0, 3000.0), (2800.0, 0.0)]);
        let n = d.netlist();
        assert_eq!(net_nodes(&n, "R1", "2"), ["R1.2", "R2.1"], "{orient:?}");
        assert_eq!(n.unconnected_pins[0], Node::new("R1", "1"), "{orient:?}");
        let placed = protasena_netlist::placed_pins(d.p.components().next().unwrap().1, d.lib.get("RES").unwrap());
        assert_eq!(placed.iter().map(|p| (p.at.x, p.at.y)).collect::<Vec<_>>(), ends, "{orient:?}");
    }
}

#[test]
fn a_dangling_wire_is_a_single_node_net_not_an_unconnected_pin() {
    let mut d = Design::new();
    d.part("RES", "R1", (0.0, 0.0));
    d.wire(&[(-200.0, 0.0), (-500.0, 0.0)]);
    let n = d.netlist();
    assert_eq!(n.unconnected_pins, [Node::new("R1", "2")]);
    let single: Vec<_> = n.erc_of(ErcKind::SingleNodeNet).collect();
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].nodes, [Node::new("R1", "1")]);
}

#[test]
fn opamp_outputs_on_one_net_conflict() {
    let mut d = Design::new();
    d.part("OPAMP", "U1", (0.0, 0.0)); // OUT at (300, 0)
    d.part_at("OPAMP", "U2", "", (1000.0, 0.0), Orient::MIRROR_X); // OUT at (700, 0)
    d.wire(&[(300.0, 0.0), (700.0, 0.0)]);
    let n = d.netlist();
    let conflict: Vec<_> = n.erc_of(ErcKind::OutputConflict).collect();
    assert_eq!(conflict.len(), 1);
    assert_eq!(conflict[0].nodes, [Node::new("U1", "6"), Node::new("U2", "6")]);
    assert_eq!(conflict[0].net.as_deref(), Some("#NET0001"));
}
