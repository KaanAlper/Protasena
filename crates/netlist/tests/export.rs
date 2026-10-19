//! JSON and SPICE exports.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::*;
use protasena_geom::Orient;
use protasena_netlist::{NetlistError, export};

/// Battery, resistor and LED in a loop, a transistor and an op-amp with a capacitor.
fn circuit() -> Design {
    let mut d = Design::new();
    d.part_at("BATTERY", "B1", "9V", (0.0, 0.0), Orient::IDENTITY); // + (-200,0), - (200,0)
    d.part_at("RES", "R1", "470", (0.0, 1000.0), Orient::IDENTITY); // (-200,1000) (200,1000)
    d.part_at("LED", "D1", "", (600.0, 1000.0), Orient::IDENTITY); // A (400,1000), K (800,1000)
    d.part_at("GROUND", "", "", (200.0, -500.0), Orient::IDENTITY);
    d.wire(&[(-200.0, 0.0), (-200.0, 1000.0)]);
    d.wire(&[(200.0, 1000.0), (400.0, 1000.0)]);
    d.wire(&[(800.0, 1000.0), (800.0, 0.0), (200.0, 0.0)]);
    d.wire(&[(200.0, 0.0), (200.0, -500.0)]);
    d.part_at("NPN", "Q1", "", (3000.0, 0.0), Orient::IDENTITY);
    d.part_at("OPAMP", "U1", "", (6000.0, 0.0), Orient::IDENTITY);
    d.part_at("CAP", "C1", "100n", (6500.0, 0.0), Orient::IDENTITY); // pin 1 at (6300, 0) = U1 OUT
    d
}

#[test]
fn spice_lines_use_primitives_values_and_ground() {
    let d = circuit();
    let n = d.netlist();
    let spice = export::to_spice(&n, "loop");
    let lines: Vec<&str> = spice.lines().collect();
    assert_eq!(lines[0], "* Protasena netlist of loop");
    let vcc = n.part("B1").unwrap().net_of_pin_named("+").unwrap().to_owned();
    let mid = n.part("D1").unwrap().net_of_pin_named("A").unwrap().to_owned();
    assert!(lines.contains(&format!("VB1 {vcc} 0 DC 9V").as_str()), "{spice}");
    assert!(lines.contains(&format!("R1 {vcc} {mid} 470").as_str()), "{spice}");
    assert!(lines.contains(&format!("D1 {mid} 0 LED").as_str()), "{spice}");
    assert!(lines.contains(&"Q1 NC_Q1_2 NC_Q1_1 NC_Q1_3 NPN"), "{spice}");
    let out = n.part("U1").unwrap().net_of_pin_named("OUT").unwrap().to_owned();
    assert!(lines.contains(&format!("C1 {out} NC_C1_2 100n").as_str()), "{spice}");
    assert!(lines.contains(&format!("XU1 NC_U1_2 NC_U1_3 {out} NC_U1_7 NC_U1_4 OPAMP").as_str()), "{spice}");
    assert!(lines.iter().any(|l| l.starts_with("* U1 (OPAMP): no SPICE primitive for ANALOG/OPAMP")), "{spice}");
    assert!(lines.contains(&".model LED D") && lines.contains(&".model NPN NPN"), "{spice}");
    assert_eq!(lines.last(), Some(&".end"));
    assert_eq!(export::to_spice(&circuit().netlist(), "loop"), spice, "deterministic");
}

#[test]
fn json_round_trips_and_checks_its_format() {
    let n = circuit().netlist();
    let json = export::to_json(&n, "loop").unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["format"], export::JSON_FORMAT);
    assert_eq!(value["version"], export::JSON_VERSION);
    assert_eq!(value["design"], "loop");
    assert!(value["nets"].as_array().unwrap().iter().any(|net| net["name"] == "GND"));
    assert_eq!(export::from_json(&json).unwrap(), (n, "loop".to_owned()));
    assert!(matches!(export::from_json(r#"{"format":"other","version":1}"#), Err(NetlistError::Format(_))));
    assert!(matches!(export::from_json(r#"{"format":"protasena-netlist","version":99}"#), Err(NetlistError::Format(_))));
    for bad in ["", "[]", "{", r#"{"format":"protasena-netlist","version":-1}"#, r#"{"format":"protasena-netlist","version":1,"nets":[{"name":1}]}"#]
    {
        assert!(matches!(export::from_json(bad), Err(NetlistError::Json(_))), "{bad}");
    }
}
