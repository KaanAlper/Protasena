//! SPICE-style text netlist: one element line per part (`R1 N1 N2 10k`).
//!
//! Parts whose simulation model is a known primitive become SPICE elements (R, C, L, D, Q, V)
//! with their pins in SPICE order; every other part becomes a subcircuit call (`X…`) with a
//! comment saying the subcircuit must be supplied. A net named `GND` is SPICE node `0`; an
//! unconnected pin gets a node of its own (`NC_R1_2`).

use std::collections::BTreeSet;
use std::fmt::Write as _;

use crate::model::{Netlist, Part};

/// A SPICE element for a simulation model: its letter, the pin names in SPICE order (empty: the
/// symbol's order) and how the value is written.
struct Primitive {
    letter: char,
    pins: &'static [&'static str],
    value: Value,
}

enum Value {
    /// The part's value (`10k`).
    Plain,
    /// A model card name, declared once with `.model NAME TYPE`.
    Model(&'static str),
    /// `DC <value>`.
    Dc,
}

fn primitive(part: &Part) -> Option<Primitive> {
    let model = part.model.as_deref()?.to_ascii_uppercase();
    let p = |letter, pins, value| Some(Primitive { letter, pins, value });
    match model.as_str() {
        "RESISTOR" => p('R', &[], Value::Plain),
        "CAPACITOR" => p('C', &[], Value::Plain),
        "INDUCTOR" => p('L', &[], Value::Plain),
        "DIODE" | "LED" => p('D', &["A", "K"], Value::Model("D")),
        "NPN" => p('Q', &["C", "B", "E"], Value::Model("NPN")),
        "PNP" => p('Q', &["C", "B", "E"], Value::Model("PNP")),
        "BATTERY" | "DC" | "VSOURCE" => p('V', &["+", "-"], Value::Dc),
        _ => None,
    }
}

/// The netlist as SPICE text, titled with the design name.
pub fn to_spice(netlist: &Netlist, design: &str) -> String {
    let mut out = String::new();
    let mut models: BTreeSet<(String, &'static str)> = BTreeSet::new();
    let _ = writeln!(out, "* Protasena netlist of {}", one_line(design));
    let _ = writeln!(out, "* {} nets, {} parts", netlist.nets.len(), netlist.parts.len());
    for part in &netlist.parts {
        line(&mut out, part, &mut models);
    }
    for (name, kind) in models {
        let _ = writeln!(out, ".model {name} {kind}");
    }
    out.push_str(".end\n");
    out
}

fn line(out: &mut String, part: &Part, models: &mut BTreeSet<(String, &'static str)>) {
    let reference = token(&part.reference);
    let node = |number: &str, net: Option<&str>| match net {
        Some(n) if n.eq_ignore_ascii_case("GND") => "0".to_owned(),
        Some(n) => token(n),
        None => token(&format!("NC_{}_{number}", part.reference)),
    };
    let symbol_order = || part.pins.iter().map(|p| node(&p.number, p.net.as_deref())).collect::<Vec<_>>();
    if let Some(prim) = primitive(part) {
        let nodes = if prim.pins.is_empty() {
            Some(symbol_order())
        } else {
            let pick = |name: &str| part.pins.iter().find(|p| p.name.eq_ignore_ascii_case(name)).map(|p| node(&p.number, p.net.as_deref()));
            prim.pins.iter().map(|n| pick(n)).collect::<Option<Vec<_>>>()
        };
        if let Some(nodes) = nodes.filter(|n| !n.is_empty()) {
            let name = element_name(prim.letter, &reference);
            let value = match prim.value {
                Value::Plain => token(part.value.trim()),
                Value::Dc => format!("DC {}", token(part.value.trim())),
                Value::Model(kind) => {
                    let model = token(part.model.as_deref().unwrap_or(&part.device.name));
                    models.insert((model.clone(), kind));
                    model
                }
            };
            if value.is_empty() || value == "DC " {
                let _ = writeln!(out, "* {reference}: no value");
            }
            let _ = writeln!(out, "{name} {} {value}", nodes.join(" "));
            return;
        }
        let _ = writeln!(out, "* {reference}: pins do not match the {} primitive", prim.letter);
    }
    let subckt = token(part.model.as_deref().unwrap_or(&part.device.name));
    let why = match (&part.primitive, &part.model) {
        (Some(p), Some(m)) => format!("no SPICE primitive for {}/{}", one_line(p), one_line(m)),
        _ => "no simulation model".to_owned(),
    };
    let _ = writeln!(out, "* {reference} ({}): {why}; called as subcircuit {subckt}, which must be supplied", one_line(&part.device.name));
    let _ = writeln!(out, "{} {} {subckt}", element_name('X', &reference), symbol_order().join(" "));
}

/// `reference` when it already starts with the element letter, else the letter in front.
fn element_name(letter: char, reference: &str) -> String {
    if reference.chars().next().is_some_and(|c| c.eq_ignore_ascii_case(&letter)) { reference.to_owned() } else { format!("{letter}{reference}") }
}

/// A SPICE token: whitespace, control characters and `( ) , = ;` become `_`.
fn token(s: &str) -> String {
    s.chars().map(|c| if c.is_whitespace() || c.is_control() || "(),=;".contains(c) { '_' } else { c }).collect()
}

/// Text for a comment: line breaks become spaces.
fn one_line(s: &str) -> String {
    s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_tokens_are_spice_safe() {
        assert_eq!(element_name('V', "B1"), "VB1");
        assert_eq!(element_name('R', "r3"), "r3");
        assert_eq!(element_name('X', ""), "X");
        assert_eq!(token("a b(c)=d;\n"), "a_b_c__d__");
        assert_eq!(one_line("x\ny"), "x y");
    }
}
