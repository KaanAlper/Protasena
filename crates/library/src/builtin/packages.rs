//! Built-in through-hole packages (top view, thou, origin at the package centre).

use super::{BUILTIN_LIBRARY, arc, circle, line, p, rect};
use crate::package::{LayerGraphic, Package, Pad, PadShape, layer};
use crate::symbol::Graphic;

/// Standard round pad: 70 th copper, 35 th drill.
const PAD: f64 = 70.0;
const DRILL: f64 = 35.0;

pub(super) fn all() -> Vec<Package> {
    vec![res40(), cap10(), elec_rad10(), do41(), led5mm(), to92(), dil08()]
}

fn package(name: &str, description: &str, pads: Vec<Pad>, silk: Vec<Graphic>) -> Package {
    Package {
        name: name.to_owned(),
        library: BUILTIN_LIBRARY.to_owned(),
        description: description.to_owned(),
        pads,
        graphics: silk.into_iter().map(|graphic| LayerGraphic { layer: layer::TOP_SILK.to_owned(), graphic }).collect(),
    }
}

fn pad(name: &str, x: f64, y: f64, shape: PadShape) -> Pad {
    Pad::through_hole(name, p(x, y), shape, PAD, DRILL)
}

/// Two round pads at ±`half_pitch` on the x axis (pad 1 square when `square_one`).
fn two_pads(half_pitch: f64, square_one: bool) -> Vec<Pad> {
    let first = if square_one { PadShape::Rect } else { PadShape::Circle };
    vec![pad("1", -half_pitch, 0.0, first), pad("2", half_pitch, 0.0, PadShape::Circle)]
}

/// Axial resistor, 400 th pitch.
fn res40() -> Package {
    let silk = vec![rect(-120.0, -45.0, 120.0, 45.0), line(-200.0, 0.0, -120.0, 0.0), line(120.0, 0.0, 200.0, 0.0)];
    package("RES40", "Axial lead, 400 th pitch", two_pads(200.0, false), silk)
}

/// Ceramic disc capacitor, 100 th pitch.
fn cap10() -> Package {
    package("CAP10", "Radial disc, 100 th pitch", two_pads(50.0, false), vec![rect(-110.0, -40.0, 110.0, 40.0)])
}

/// Radial electrolytic capacitor, 100 th pitch, 200 th body.
fn elec_rad10() -> Package {
    let plus = vec![circle(0.0, 0.0, 100.0), line(-150.0, 60.0, -110.0, 60.0), line(-130.0, 40.0, -130.0, 80.0)];
    package("ELEC-RAD10", "Radial electrolytic, 100 th pitch, pad 1 positive", two_pads(50.0, true), plus)
}

/// DO-41 axial diode, 400 th pitch, pad 1 cathode (band side).
fn do41() -> Package {
    let silk = vec![rect(-100.0, -50.0, 100.0, 50.0), line(-70.0, -50.0, -70.0, 50.0)];
    package("DO41", "Axial diode, 400 th pitch, pad 1 cathode", two_pads(200.0, true), silk)
}

/// 5 mm round LED, 100 th pitch, pad 1 cathode (flat side).
fn led5mm() -> Package {
    let silk = vec![arc((-50.0, -87.0), (100.0, 0.0), (-50.0, 87.0)), line(-50.0, -87.0, -50.0, 87.0)];
    package("LED5MM", "5 mm round LED, 100 th pitch, pad 1 cathode", two_pads(50.0, true), silk)
}

/// TO-92 in line, 100 th pitch, flat side towards -y.
fn to92() -> Package {
    let pads = vec![pad("1", -100.0, 0.0, PadShape::Rect), pad("2", 0.0, 0.0, PadShape::Circle), pad("3", 100.0, 0.0, PadShape::Circle)];
    let silk = vec![arc((-90.0, -50.0), (0.0, 100.0), (90.0, -50.0)), line(-90.0, -50.0, 90.0, -50.0)];
    package("TO92", "TO-92, three leads in line, 100 th pitch", pads, silk)
}

/// 8-pin DIL, 300 th row spacing, 100 th pitch, pin 1 top left, counter-clockwise numbering.
fn dil08() -> Package {
    let mut pads = Vec::with_capacity(8);
    for i in 0..4u8 {
        let y = 150.0 - 100.0 * f64::from(i);
        let shape = if i == 0 { PadShape::Rect } else { PadShape::Oval };
        pads.push(pad(&(i + 1).to_string(), -150.0, y, shape));
        pads.push(pad(&(8 - i).to_string(), 150.0, y, PadShape::Oval));
    }
    pads.sort_by_key(|p| p.name.parse::<u8>().unwrap_or(u8::MAX));
    let silk = vec![rect(-100.0, -200.0, 100.0, 200.0), arc((-30.0, 200.0), (0.0, 170.0), (30.0, 200.0))];
    package("DIL08", "Dual in line, 8 pins, 300 th rows", pads, silk)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dil08_numbers_counter_clockwise() {
        let pkg = dil08();
        let names: Vec<&str> = pkg.pads.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["1", "2", "3", "4", "5", "6", "7", "8"]);
        assert_eq!(pkg.pad("1").unwrap().at, p(-150.0, 150.0));
        assert_eq!(pkg.pad("4").unwrap().at, p(-150.0, -150.0));
        assert_eq!(pkg.pad("5").unwrap().at, p(150.0, -150.0));
        assert_eq!(pkg.pad("8").unwrap().at, p(150.0, 150.0));
    }
}
