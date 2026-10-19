//! The netlist: nets, parts, unconnected pins and the electrical rules check report.

use protasena_doc::DeviceRef;
use serde::{Deserialize, Serialize};

/// A pin of a part: `(reference, pin number)`, e.g. `R1.2`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Node {
    /// Part reference.
    pub reference: String,
    /// Pin number.
    pub pin: String,
}

impl Node {
    /// The node `reference.pin`.
    pub fn new(reference: &str, pin: &str) -> Self {
        Self { reference: reference.to_owned(), pin: pin.to_owned() }
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.reference, self.pin)
    }
}

/// One net.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Net {
    /// Net name: a label, terminal or power name when it has one, else `#NETnnnn`.
    pub name: String,
    /// The pins on the net, in natural order (`R2` before `R10`).
    pub nodes: Vec<Node>,
    /// Further names the net carries (when several labels or terminals meet), sorted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

/// One pin of a part and the net it is on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartPin {
    /// Pin number.
    pub number: String,
    /// Pin name.
    pub name: String,
    /// The net (`None` for an unconnected pin).
    pub net: Option<String>,
}

/// A physical part (all placed gates of one reference).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    /// Reference.
    pub reference: String,
    /// The library device.
    pub device: DeviceRef,
    /// Value.
    pub value: String,
    /// Simulation primitive class (`ANALOG`), when the device has a model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primitive: Option<String>,
    /// Simulation model name (`RESISTOR`), when the device has a model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Pins in symbol order.
    pub pins: Vec<PartPin>,
}

impl Part {
    /// The net of the pin with this name (without case), if any.
    pub fn net_of_pin_named(&self, name: &str) -> Option<&str> {
        self.pins.iter().find(|p| p.name.eq_ignore_ascii_case(name)).and_then(|p| p.net.as_deref())
    }
}

/// How serious a rules-check finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The design is wrong or could not be read completely.
    Error,
    /// Probably a mistake.
    Warning,
}

/// The kinds of rules-check finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErcKind {
    /// A pin connected to nothing.
    UnconnectedPin,
    /// A net with only one pin.
    SingleNodeNet,
    /// Two or more outputs drive one net.
    OutputConflict,
    /// A net carries more than one name.
    MultipleNames,
    /// A component's device (or its gate) is not in the library.
    UnknownDevice,
    /// Two parts share a reference.
    DuplicateReference,
    /// A coordinate is not finite or too large to compare; that point connects to nothing.
    InvalidGeometry,
}

/// One rules-check finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErcItem {
    /// Severity.
    pub severity: Severity,
    /// Kind.
    pub kind: ErcKind,
    /// The net concerned, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub net: Option<String>,
    /// The pins concerned.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<Node>,
    /// Description in plain words.
    pub message: String,
}

/// The connectivity of a project.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Netlist {
    /// Nets with at least one pin, sorted by name in natural order.
    pub nets: Vec<Net>,
    /// Pins connected to nothing, in natural order (they are on no net).
    pub unconnected_pins: Vec<Node>,
    /// Parts in natural reference order (power symbols are not parts).
    pub parts: Vec<Part>,
    /// Rules-check findings, errors first.
    pub erc: Vec<ErcItem>,
}

impl Netlist {
    /// The net with this name.
    pub fn net(&self, name: &str) -> Option<&Net> {
        self.nets.iter().find(|n| n.name == name)
    }

    /// The net a pin is on.
    pub fn net_of(&self, reference: &str, pin: &str) -> Option<&Net> {
        self.nets.iter().find(|n| n.nodes.iter().any(|x| x.reference == reference && x.pin == pin))
    }

    /// The part with this reference.
    pub fn part(&self, reference: &str) -> Option<&Part> {
        self.parts.iter().find(|p| p.reference == reference)
    }

    /// Findings of one kind.
    pub fn erc_of(&self, kind: ErcKind) -> impl Iterator<Item = &ErcItem> {
        self.erc.iter().filter(move |e| e.kind == kind)
    }
}
