//! Built-in semiconductors: diodes, bipolar transistors and a generic op-amp.

use super::{analog, device, line, package, package_prop, pin, poly, symbol, text};
use crate::device::{Device, PackageRef};
use crate::symbol::{Fill, Graphic, Pin, PinDir, PinKind};

/// Diode body: triangle pointing right (anode left) and the cathode bar.
fn diode_body() -> Vec<Graphic> {
    vec![poly(&[(-100.0, 80.0), (-100.0, -80.0), (100.0, 0.0), (-100.0, 80.0)]).filled(Fill::Solid), line(100.0, 80.0, 100.0, -80.0)]
}

/// Cathode (pin 1, right) and anode (pin 2, left).
fn diode_pins() -> Vec<Pin> {
    vec![
        pin("1", "K", PinKind::Passive, (200.0, 0.0), PinDir::Left, 100.0).bare(),
        pin("2", "A", PinKind::Passive, (-200.0, 0.0), PinDir::Right, 100.0).bare(),
    ]
}

/// Rectifier diode.
pub(super) fn diode() -> Device {
    Device {
        symbol: symbol(diode_body(), diode_pins()),
        packages: vec![package("DO41", &[("1", "1"), ("2", "2")])],
        default_package: Some("DO41".into()),
        prop_defs: vec![package_prop("DO41")],
        sim_model: analog("DIODE"),
        ..device("DIODE", "D", "Diodes", "Generic", "Generic diode", &["diode", "rectifier"])
    }
}

/// Light-emitting diode: diode body with two arrows pointing away.
pub(super) fn led() -> Device {
    let mut graphics = diode_body();
    for (x, y) in [(0.0, 90.0), (50.0, 70.0)] {
        graphics.push(line(x, y, x + 60.0, y + 60.0));
        graphics.push(poly(&[(x + 35.0, y + 60.0), (x + 60.0, y + 60.0), (x + 60.0, y + 35.0)]));
    }
    Device {
        symbol: symbol(graphics, diode_pins()),
        packages: vec![package("LED5MM", &[("1", "1"), ("2", "2")])],
        default_package: Some("LED5MM".into()),
        prop_defs: vec![package_prop("LED5MM")],
        sim_model: analog("LED"),
        ..device("LED", "D", "Optoelectronics", "LEDs", "Generic light-emitting diode", &["led", "light", "diode"])
    }
}

/// Base (1, left), collector (2) and emitter (3); `collector_up` puts the collector at the top.
fn bjt_pins(collector_up: bool) -> Vec<Pin> {
    let (cy, ey) = if collector_up { (200.0, -200.0) } else { (-200.0, 200.0) };
    let toward = |y: f64| if y > 0.0 { PinDir::Down } else { PinDir::Up };
    vec![
        pin("1", "B", PinKind::Input, (-200.0, 0.0), PinDir::Right, 160.0),
        pin("2", "C", PinKind::Passive, (100.0, cy), toward(cy), 100.0),
        pin("3", "E", PinKind::Passive, (100.0, ey), toward(ey), 100.0),
    ]
}

/// The TO92 pin map for an E-B-C pinout (pad 1 = emitter).
fn to92_ebc() -> PackageRef {
    package("TO92", &[("1", "2"), ("2", "3"), ("3", "1")])
}

/// Base bar, the two legs, the envelope and an emitter arrow.
fn bjt_body(arrow: [(f64, f64); 3]) -> Vec<Graphic> {
    vec![
        line(-40.0, -80.0, -40.0, 80.0),
        line(-40.0, 40.0, 100.0, 100.0),
        line(-40.0, -40.0, 100.0, -100.0),
        super::circle(30.0, 0.0, 150.0),
        poly(&[arrow[0], arrow[1], arrow[2], arrow[0]]).filled(Fill::Solid),
    ]
}

/// NPN bipolar transistor (emitter at the bottom, arrow pointing out).
pub(super) fn npn() -> Device {
    Device {
        symbol: symbol(bjt_body([(65.0, -85.0), (28.0, -44.0), (10.0, -86.0)]), bjt_pins(true)),
        packages: vec![to92_ebc()],
        default_package: Some("TO92".into()),
        prop_defs: vec![package_prop("TO92")],
        sim_model: analog("NPN"),
        ..device("NPN", "Q", "Transistors", "Bipolar", "Generic NPN bipolar transistor", &["transistor", "bjt", "npn"])
    }
}

/// PNP bipolar transistor (emitter at the top, arrow pointing in).
pub(super) fn pnp() -> Device {
    Device {
        symbol: symbol(bjt_body([(9.0, 61.0), (64.0, 60.0), (46.0, 102.0)]), bjt_pins(false)),
        packages: vec![to92_ebc()],
        default_package: Some("TO92".into()),
        prop_defs: vec![package_prop("TO92")],
        sim_model: analog("PNP"),
        ..device("PNP", "Q", "Transistors", "Bipolar", "Generic PNP bipolar transistor", &["transistor", "bjt", "pnp"])
    }
}

/// Generic single op-amp with supply pins, numbered like the common 8-pin DIL pinout.
pub(super) fn opamp() -> Device {
    let graphics = vec![
        poly(&[(-200.0, 200.0), (-200.0, -200.0), (200.0, 0.0), (-200.0, 200.0)]).filled(Fill::Body),
        text(-150.0, 100.0, "-", 60.0),
        text(-150.0, -100.0, "+", 60.0),
    ];
    let hide_name = |p: Pin| Pin { name_visible: false, ..p };
    let pins = vec![
        pin("2", "IN-", PinKind::Input, (-300.0, 100.0), PinDir::Right, 100.0),
        pin("3", "IN+", PinKind::Input, (-300.0, -100.0), PinDir::Right, 100.0),
        pin("6", "OUT", PinKind::Output, (300.0, 0.0), PinDir::Left, 100.0),
        pin("7", "V+", PinKind::PowerIn, (0.0, 300.0), PinDir::Down, 200.0),
        pin("4", "V-", PinKind::PowerIn, (0.0, -300.0), PinDir::Up, 200.0),
    ];
    let map: Vec<(&str, &str)> = ["2", "3", "4", "6", "7"].iter().map(|n| (*n, *n)).collect();
    Device {
        symbol: symbol(graphics, pins.into_iter().map(hide_name).collect()),
        packages: vec![package("DIL08", &map)],
        default_package: Some("DIL08".into()),
        prop_defs: vec![package_prop("DIL08")],
        sim_model: analog("OPAMP"),
        ..device("OPAMP", "U", "Operational Amplifiers", "Generic", "Generic operational amplifier", &["opamp", "op-amp", "amplifier"])
    }
}
