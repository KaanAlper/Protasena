//! Built-in passives, sources and power symbols.

use super::{analog, arc, device, line, package, package_prop, pin, rect, symbol, text, value};
use crate::device::{Device, PropDef, PropKind};
use crate::symbol::{PinDir, PinKind, Symbol};

/// Two passive pins at x = ±200 running inwards by `length`, names and numbers hidden.
fn two_pins(length: f64, names: (&str, &str)) -> Vec<crate::symbol::Pin> {
    vec![
        pin("1", names.0, PinKind::Passive, (-200.0, 0.0), PinDir::Right, length).bare(),
        pin("2", names.1, PinKind::Passive, (200.0, 0.0), PinDir::Left, length).bare(),
    ]
}

/// Resistor.
pub(super) fn res() -> Device {
    Device {
        symbol: symbol(vec![rect(-100.0, -40.0, 100.0, 40.0)], two_pins(100.0, ("1", "2"))),
        packages: vec![package("RES40", &[("1", "1"), ("2", "2")])],
        default_package: Some("RES40".into()),
        prop_defs: vec![value("Resistance", "Ω", "10k"), package_prop("RES40")],
        sim_model: analog("RESISTOR"),
        ..device("RES", "R", "Resistors", "Generic", "Generic resistor", &["resistor"])
    }
}

/// Non-polarised capacitor.
pub(super) fn cap() -> Device {
    let plates = vec![line(-30.0, -100.0, -30.0, 100.0), line(30.0, -100.0, 30.0, 100.0)];
    Device {
        symbol: symbol(plates, two_pins(170.0, ("1", "2"))),
        packages: vec![package("CAP10", &[("1", "1"), ("2", "2")])],
        default_package: Some("CAP10".into()),
        prop_defs: vec![value("Capacitance", "F", "1nF"), package_prop("CAP10")],
        sim_model: analog("CAPACITOR"),
        ..device("CAP", "C", "Capacitors", "Generic", "Generic non-polarised capacitor", &["capacitor"])
    }
}

/// Polarised (electrolytic) capacitor: straight positive plate, curved negative plate.
pub(super) fn cap_elec() -> Device {
    let graphics = vec![line(-30.0, -100.0, -30.0, 100.0), arc((60.0, 100.0), (30.0, 0.0), (60.0, -100.0)), text(-100.0, 80.0, "+", 60.0)];
    Device {
        symbol: symbol(graphics, two_pins(170.0, ("+", "-"))),
        packages: vec![package("ELEC-RAD10", &[("1", "1"), ("2", "2")])],
        default_package: Some("ELEC-RAD10".into()),
        prop_defs: vec![value("Capacitance", "F", "1uF"), package_prop("ELEC-RAD10")],
        sim_model: analog("CAPACITOR"),
        ..device(
            "CAP-ELEC",
            "C",
            "Capacitors",
            "Electrolytic",
            "Generic electrolytic capacitor, polarised",
            &["capacitor", "electrolytic", "polarised"],
        )
    }
}

/// Single-cell battery: long plate positive (pin 1, left), short plate negative.
pub(super) fn battery() -> Device {
    let graphics = vec![line(-30.0, -100.0, -30.0, 100.0), line(30.0, -50.0, 30.0, 50.0), text(-100.0, 80.0, "+", 60.0)];
    Device {
        symbol: symbol(graphics, two_pins(170.0, ("+", "-"))),
        prop_defs: vec![PropDef::new("VALUE", "Voltage", PropKind::Float { unit: Some("V".into()) }, Some("9V"))],
        sim_model: analog("BATTERY"),
        ..device("BATTERY", "B", "Sources", "Batteries", "DC voltage source drawn as a battery", &["battery", "cell", "source"])
    }
}

/// Power symbol with one pin at the origin; the pin name is the net name.
fn power_symbol(net: &str, orient: PinDir, graphics: Vec<crate::symbol::Graphic>) -> std::sync::Arc<Symbol> {
    let one = symbol(graphics, vec![pin("1", net, PinKind::PowerIn, (0.0, 0.0), orient, 0.0).bare()]);
    std::sync::Arc::new(Symbol { power: true, ..(*one).clone() })
}

fn net_prop(default: &str) -> PropDef {
    PropDef::new("VALUE", "Net", PropKind::String, Some(default))
}

/// Ground symbol: stem down to three bars.
pub(super) fn ground() -> Device {
    let graphics =
        vec![line(0.0, 0.0, 0.0, -50.0), line(-50.0, -50.0, 50.0, -50.0), line(-30.0, -75.0, 30.0, -75.0), line(-10.0, -100.0, 10.0, -100.0)];
    Device {
        symbol: power_symbol("GND", PinDir::Down, graphics),
        prop_defs: vec![net_prop("GND")],
        ..device("GROUND", "", "Power", "Ground", "Ground (0 V) connection", &["ground", "gnd", "earth", "power"])
    }
}

/// Supply rail symbol: stem up to a bar.
pub(super) fn power() -> Device {
    let graphics = vec![line(0.0, 0.0, 0.0, 50.0), line(-50.0, 50.0, 50.0, 50.0)];
    Device {
        symbol: power_symbol("VCC", PinDir::Up, graphics),
        prop_defs: vec![net_prop("VCC")],
        ..device("POWER", "", "Power", "Supply", "Supply rail connection", &["power", "supply", "rail", "vcc"])
    }
}
