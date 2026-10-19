//! The things placed on a sheet. Coordinates are sheet coordinates in thou, y up (see
//! `protasena-geom`).

use protasena_geom::{Orient, Pt, Transform, Vec2};

use crate::ids::{BusEntryId, BusId, ComponentId, JunctionId, LabelId, TerminalId, TextId, WireId};
use crate::properties::Properties;

/// Names a library device. The design model does not depend on the library (both are layer
/// L1); `protasena-netlist` resolves the reference against a library.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeviceRef {
    /// Pack library name (`GENERIC`); empty matches any library.
    #[cfg_attr(feature = "serde", serde(default))]
    pub library: String,
    /// Device name (`RES`, `74LS00`), compared without case.
    pub name: String,
}

impl DeviceRef {
    /// A reference to `name` in `library`.
    pub fn new(library: &str, name: &str) -> Self {
        Self { library: library.to_owned(), name: name.to_owned() }
    }
}

/// Where a component's reference or value text sits, and whether it is shown.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FieldLabel {
    /// Anchor relative to the component's origin, in sheet axes (it does not turn with the part).
    pub offset: Vec2,
    /// Whether the text is drawn.
    #[cfg_attr(feature = "serde", serde(default = "yes"))]
    pub visible: bool,
}

impl Default for FieldLabel {
    fn default() -> Self {
        Self { offset: Vec2::ZERO, visible: true }
    }
}

/// A placed device (one unit, i.e. gate, of it).
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Component {
    /// Stable id.
    pub id: ComponentId,
    /// The library device.
    pub device: DeviceRef,
    /// Which gate of a multi-gate device: 0 is the first distinct unit name of the symbol.
    #[cfg_attr(feature = "serde", serde(default))]
    pub unit: u32,
    /// Part reference (`R1`, `U3`); `R?` before annotation, empty for power symbols. Gates of one
    /// physical part share the reference.
    #[cfg_attr(feature = "serde", serde(default))]
    pub reference: String,
    /// Value (`10k`, `BC547`); for a power symbol, the net it joins.
    #[cfg_attr(feature = "serde", serde(default))]
    pub value: String,
    /// Placement: symbol coordinates to sheet coordinates.
    #[cfg_attr(feature = "serde", serde(default, with = "crate::serde_impl::transform"))]
    pub transform: Transform,
    /// Chosen PCB package (one of the device's packages), if any.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub package: Option<String>,
    /// Further property values, in Edit Component order.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Properties::is_empty"))]
    pub properties: Properties,
    /// The reference text.
    #[cfg_attr(feature = "serde", serde(default))]
    pub reference_label: FieldLabel,
    /// The value text.
    #[cfg_attr(feature = "serde", serde(default))]
    pub value_label: FieldLabel,
}

impl Component {
    /// A component of `device` with a reference and value, at the identity placement.
    pub fn new(id: ComponentId, device: DeviceRef, reference: &str, value: &str) -> Self {
        Self { id, device, reference: reference.to_owned(), value: value.to_owned(), ..Self::default() }
    }

    /// The same component with another placement.
    pub fn placed(self, transform: Transform) -> Self {
        Self { transform, ..self }
    }

    /// The component's origin on the sheet.
    pub fn origin(&self) -> Pt {
        self.transform.offset.to_pt()
    }
}

/// A wire: a polyline of two or more points (segments need not be orthogonal). All its points
/// belong to one conductor.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Wire {
    /// Stable id.
    pub id: WireId,
    /// Vertices, from one end to the other.
    pub points: Vec<Pt>,
}

/// A junction dot: joins every wire passing through it.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Junction {
    /// Stable id.
    pub id: JunctionId,
    /// Position.
    pub at: Pt,
}

/// A net label: names the net of the wire it sits on. Labels with the same text are one net.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Label {
    /// Stable id.
    pub id: LabelId,
    /// Attachment point (on a wire).
    pub at: Pt,
    /// Net name.
    pub text: String,
    /// Text orientation.
    #[cfg_attr(feature = "serde", serde(default, with = "crate::serde_impl::orient"))]
    pub orient: Orient,
}

/// The kinds of terminal (Proteus's Terminals mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "snake_case"))]
pub enum TerminalKind {
    /// Plain terminal: its name joins nets like a label.
    #[default]
    Default,
    /// Input port.
    Input,
    /// Output port.
    Output,
    /// Bidirectional port.
    Bidir,
    /// Supply rail; joins the net of its name (`VCC` when unnamed).
    Power,
    /// Ground; joins the net of its name (`GND` when unnamed).
    Ground,
    /// Bus port.
    Bus,
}

/// A terminal: a named connection point.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Terminal {
    /// Stable id.
    pub id: TerminalId,
    /// Connection point.
    pub at: Pt,
    /// Kind.
    #[cfg_attr(feature = "serde", serde(default))]
    pub kind: TerminalKind,
    /// Net name.
    #[cfg_attr(feature = "serde", serde(default))]
    pub name: String,
    /// Orientation of the terminal symbol.
    #[cfg_attr(feature = "serde", serde(default, with = "crate::serde_impl::orient"))]
    pub orient: Orient,
}

/// A bus: a polyline drawn thick. Buses do not join nets by themselves.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bus {
    /// Stable id.
    pub id: BusId,
    /// Vertices.
    pub points: Vec<Pt>,
}

/// A bus entry: a short segment from a bus to a wire.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BusEntry {
    /// Stable id.
    pub id: BusEntryId,
    /// End on the bus.
    pub bus_end: Pt,
    /// End on the wire.
    pub wire_end: Pt,
}

/// Free text on the sheet.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Text {
    /// Stable id.
    pub id: TextId,
    /// Anchor.
    pub at: Pt,
    /// The text.
    pub text: String,
    /// Character height in thou; 0 = the default size.
    #[cfg_attr(feature = "serde", serde(default))]
    pub size: f64,
    /// Orientation.
    #[cfg_attr(feature = "serde", serde(default, with = "crate::serde_impl::orient"))]
    pub orient: Orient,
}

#[cfg(feature = "serde")]
fn yes() -> bool {
    true
}
