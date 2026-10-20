//! Symbol pins.

use crate::common::{At, TextWithEffects};
use crate::decode::{arg, flag, num1, put, required, string, text};
use crate::error::Result;
use crate::report::{Inspect, Report, Unknown, keyword_enum};
use crate::sexpr::Sexpr;

keyword_enum!(
    /// Electrical type of a pin.
    ElectricalType, "pin.type" {
        Input = "input", Output = "output", Bidirectional = "bidirectional", TriState = "tri_state",
        Passive = "passive", Free = "free", Unspecified = "unspecified", PowerIn = "power_in",
        PowerOut = "power_out", OpenCollector = "open_collector", OpenEmitter = "open_emitter",
        NoConnect = "no_connect",
    }
);

keyword_enum!(
    /// Graphic style of a pin.
    PinStyle, "pin.style" {
        Line = "line", Inverted = "inverted", Clock = "clock", InvertedClock = "inverted_clock",
        InputLow = "input_low", ClockLow = "clock_low", OutputLow = "output_low",
        EdgeClockHigh = "edge_clock_high", NonLogic = "non_logic",
    }
);

/// `(alternate "NAME" TYPE STYLE)`: another function the pin can be switched to.
#[derive(Debug, Clone, PartialEq)]
pub struct Alternate {
    pub name: String,
    pub electrical: ElectricalType,
    pub style: PinStyle,
    pub unknown: Vec<Unknown>,
}

/// `(pin TYPE STYLE (at X Y ANGLE) (length L) (name …) (number …))`.
#[derive(Debug, Clone, PartialEq)]
pub struct Pin {
    pub electrical: ElectricalType,
    pub style: PinStyle,
    /// The connection point; the angle (0, 90, 180, 270) points from it into the body.
    pub at: At,
    pub length: f64,
    pub name: TextWithEffects,
    pub number: TextWithEffects,
    /// Bare `hide` (KiCad 6–7) or `(hide yes)` (KiCad 8+).
    pub hide: bool,
    pub alternates: Vec<Alternate>,
    pub unknown: Vec<Unknown>,
}

impl Pin {
    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        let electrical = ElectricalType::from_keyword(&string(arg(node, 0, "electrical type")?, "pin electrical type")?);
        let style = PinStyle::from_keyword(&string(arg(node, 1, "graphic style")?, "pin graphic style")?);
        let (mut at, mut length, mut name, mut number) = (None, None, None, None);
        let mut hide = false;
        let mut alternates = Vec::new();
        let mut unknown = Vec::new();
        for item in node.args().iter().skip(2) {
            match item.name() {
                "at" if item.as_list().is_some() => put(&mut at, At::decode(item)?, item, &mut unknown),
                "length" if item.as_list().is_some() => put(&mut length, num1(item)?, item, &mut unknown),
                "name" if item.as_list().is_some() => put(&mut name, TextWithEffects::decode(item, "pin.name")?, item, &mut unknown),
                "number" if item.as_list().is_some() => put(&mut number, TextWithEffects::decode(item, "pin.number")?, item, &mut unknown),
                "hide" => hide = flag(item)?,
                "alternate" if item.as_list().is_some() => alternates.push(Alternate::decode(item)?),
                _ => unknown.push(Unknown::new("pin", item)),
            }
        }
        Ok(Self {
            electrical,
            style,
            at: required(at, node, "at")?,
            length: required(length, node, "length")?,
            name: required(name, node, "name")?,
            number: required(number, node, "number")?,
            hide,
            alternates,
            unknown,
        })
    }
}

impl Alternate {
    fn decode(node: &Sexpr) -> Result<Self> {
        let mut unknown = Vec::new();
        for item in node.args().iter().skip(3) {
            unknown.push(Unknown::new("alternate", item));
        }
        Ok(Self {
            name: text(node, 0, "alternate name")?,
            electrical: ElectricalType::from_keyword(&text(node, 1, "alternate electrical type")?),
            style: PinStyle::from_keyword(&text(node, 2, "alternate graphic style")?),
            unknown,
        })
    }
}

impl Inspect for Pin {
    fn visit(&self, r: &mut Report) {
        self.electrical.visit(r);
        self.style.visit(r);
        self.name.visit(r);
        self.number.visit(r);
        for a in &self.alternates {
            a.electrical.visit(r);
            a.style.visit(r);
            a.unknown.visit(r);
        }
        self.unknown.visit(r);
    }
}
