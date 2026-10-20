//! Joining by name (labels, terminals, power symbols, hidden power pins), net names, gates and
//! determinism.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::*;
use protasena_doc::{Component, ComponentId, DeviceRef, Item, Op, TerminalKind};
use protasena_geom::{Orient, Pt, Transform, Vec2};
use protasena_library::{Device, Library, Pin, PinDir, PinKind, Symbol, Unit};
use protasena_netlist::{ErcKind, Node, build};

#[test]
fn labels_with_the_same_text_join_across_sheets() {
    let mut d = Design::new();
    d.part("RES", "R1", (0.0, 0.0));
    d.wire(&[(200.0, 0.0), (600.0, 0.0)]);
    d.label((400.0, 0.0), "SDA");
    d.new_sheet();
    d.part("RES", "R2", (0.0, 0.0));
    d.wire(&[(-200.0, 0.0), (-600.0, 0.0)]);
    d.label((-600.0, 0.0), " SDA ");
    d.label((-400.0, 0.0), "sda"); // case-sensitive: another net name on the same wire
    let n = d.netlist();
    let net = n.net("SDA").unwrap();
    assert_eq!(net.nodes, [Node::new("R1", "2"), Node::new("R2", "1")]);
    assert_eq!(net.aliases, ["sda"]);
    assert_eq!(n.erc_of(ErcKind::MultipleNames).count(), 1);
}

#[test]
fn terminals_join_by_name_and_power_terminals_default_to_rails() {
    let mut d = Design::new();
    d.part("RES", "R1", (0.0, 0.0));
    d.terminal((-200.0, 0.0), TerminalKind::Power, "");
    d.terminal((200.0, 0.0), TerminalKind::Input, "IN");
    d.part("RES", "R2", (0.0, 1000.0));
    d.terminal((-200.0, 1000.0), TerminalKind::Ground, "");
    d.terminal((200.0, 1000.0), TerminalKind::Default, "IN");
    d.part("RES", "R3", (0.0, 2000.0));
    d.terminal((-200.0, 2000.0), TerminalKind::Power, "+12V");
    d.terminal((200.0, 2000.0), TerminalKind::Bus, "IN");
    let n = d.netlist();
    assert_eq!(net_name(&n, "R1", "1").as_deref(), Some("VCC"));
    assert_eq!(net_name(&n, "R2", "1").as_deref(), Some("GND"));
    assert_eq!(net_name(&n, "R3", "1").as_deref(), Some("+12V"));
    assert_eq!(net_nodes(&n, "R1", "2"), ["R1.2", "R2.2"]);
    assert_eq!(n.unconnected_pins, [Node::new("R3", "2")]);
}

#[test]
fn power_symbols_name_nets_by_value_and_power_names_win() {
    let mut d = Design::new();
    d.part("RES", "R1", (0.0, 0.0));
    d.part_at("POWER", "", "+5V", (-200.0, 0.0), Orient::IDENTITY);
    d.part_at("GROUND", "", "", (200.0, 0.0), Orient::IDENTITY);
    d.part("RES", "R2", (0.0, -1000.0));
    d.part_at("GROUND", "", "", (5000.0, 5000.0), Orient::IDENTITY);
    d.wire(&[(5000.0, 5000.0), (5000.0, -1000.0), (200.0, -1000.0)]);
    d.label((1000.0, -1000.0), "RETURN");
    let n = d.netlist();
    assert_eq!(net_name(&n, "R1", "1").as_deref(), Some("+5V"));
    let gnd = n.net("GND").unwrap();
    assert_eq!(gnd.nodes, [Node::new("R1", "2"), Node::new("R2", "2")]);
    assert_eq!(gnd.aliases, ["RETURN"]);
    assert!(n.part("").is_none(), "power symbols are not parts");
    assert_eq!(n.parts.len(), 2);
}

/// A two-gate part: gates A and B with an output each and a shared hidden supply pin 8.
fn dual_library() -> Library {
    let pin = |n: &str, name: &str, kind, x: f64, hidden| Pin { hidden, ..Pin::new(n, name, kind, Pt::new(x, 0.0), PinDir::Left, 100.0) };
    let unit = |name: &str, out: &str| Unit {
        name: name.into(),
        body_style: 1,
        graphics: Vec::new(),
        pins: vec![pin(out, "Y", PinKind::Output, 200.0, false), pin("8", "VCC", PinKind::PowerIn, 0.0, true)],
    };
    let dual = Device {
        name: "DUAL".into(),
        library: "TEST".into(),
        prefix: "U".into(),
        symbol: Symbol { units: vec![unit("A", "1"), unit("B", "7")], power: false }.into(),
        ..Device::default()
    };
    let builtin = Library::builtin();
    let mut devices = builtin.devices().to_vec();
    devices.push(dual);
    Library::from_parts(devices, builtin.packages().cloned().collect()).unwrap()
}

#[test]
fn gates_of_one_part_share_pins_and_hidden_power_pins_join_rails() {
    let mut d = Design::new();
    d.lib = dual_library();
    let sheet = d.sheet;
    for (unit, x) in [(0u32, 0.0), (1, 1000.0)] {
        let id: ComponentId = d.p.fresh_id().unwrap();
        let c = Component { unit, ..Component::new(id, DeviceRef::new("TEST", "DUAL"), "U1", "") }
            .placed(Transform::new(Orient::IDENTITY, Vec2::new(x, 0.0)));
        d.p.apply(Op::Add { sheet, item: Item::Component(c), index: None }).unwrap();
    }
    d.part_at("POWER", "", "", (5000.0, 0.0), Orient::IDENTITY);
    d.wire(&[(200.0, 0.0), (1200.0, 0.0)]);
    let n = d.netlist();
    assert_eq!(n.net("VCC").unwrap().nodes, [Node::new("U1", "8")]);
    assert_eq!(net_nodes(&n, "U1", "1"), ["U1.1", "U1.7"]);
    assert_eq!(n.erc_of(ErcKind::OutputConflict).count(), 1);
    let part = n.part("U1").unwrap();
    assert_eq!(part.pins.iter().map(|p| p.number.as_str()).collect::<Vec<_>>(), ["1", "8", "7"]);
    assert_eq!(n.erc_of(ErcKind::DuplicateReference).count(), 0);
}

#[test]
fn repeated_references_are_reported_not_merged() {
    let mut d = Design::new();
    d.part("RES", "R1", (0.0, 0.0));
    d.part("RES", "R1", (0.0, 1000.0));
    d.part("RES", "R?", (0.0, 2000.0));
    d.part("RES", "R?", (0.0, 3000.0));
    let n = d.netlist();
    assert_eq!(n.erc_of(ErcKind::DuplicateReference).count(), 2);
    assert_eq!(n.parts.len(), 4);
    assert!(n.nets.is_empty());
}

#[test]
fn unnamed_nets_are_numbered_by_their_lowest_pin_whatever_the_order() {
    let build_with = |order: &[usize]| {
        let mut d = Design::new();
        let parts = [("R10", 0.0), ("R2", 1000.0), ("R1", 2000.0)];
        for &i in order {
            let (r, y) = parts[i];
            d.part("RES", r, (0.0, y));
            d.wire(&[(200.0, y), (600.0, y)]);
            d.wire(&[(-200.0, y), (-600.0, y)]);
        }
        d.label((600.0, 0.0), "#NET0002"); // an explicit name takes its number
        d.netlist()
    };
    let n = build_with(&[0, 1, 2]);
    let names: Vec<(&str, String)> = n.nets.iter().map(|net| (net.name.as_str(), net.nodes[0].to_string())).collect();
    assert_eq!(
        names,
        [
            ("#NET0001", "R1.1".into()),
            ("#NET0002", "R10.2".into()),
            ("#NET0003", "R1.2".into()),
            ("#NET0004", "R2.1".into()),
            ("#NET0005", "R2.2".into()),
            ("#NET0006", "R10.1".into())
        ]
    );
    assert_eq!(build_with(&[2, 0, 1]), n);
    assert_eq!(build_with(&[1, 2, 0]), n);
}

#[test]
fn unknown_devices_are_reported_and_skipped() {
    let mut d = Design::new();
    d.part("RES", "R1", (0.0, 0.0));
    d.part("NOSUCH", "X1", (200.0, 0.0));
    let n = build(&d.p, &Library::default());
    assert_eq!(n.erc_of(ErcKind::UnknownDevice).count(), 2);
    assert!(n.parts.is_empty() && n.nets.is_empty());
    let n = d.netlist();
    assert_eq!(n.erc_of(ErcKind::UnknownDevice).count(), 1);
    assert_eq!(n.parts.len(), 1);
}
