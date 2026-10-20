//! The editing modes of Proteus's Mode toolbar, top to bottom.

use serde::Serialize;

/// One button of the Mode toolbar: what a click in the editing window does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// Select, drag and edit objects.
    Selection,
    /// Place the device chosen in the object selector.
    #[default]
    Component,
    /// Place junction dots.
    JunctionDot,
    /// Name wires.
    WireLabel,
    /// Place text scripts.
    TextScript,
    /// Draw buses.
    Buses,
    /// Place subcircuits.
    Subcircuit,
    /// Place terminals of the kind chosen in the object selector.
    Terminals,
    /// Place device pins.
    DevicePins,
    /// Place simulation graphs.
    Graph,
    /// Place active popups.
    ActivePopup,
    /// Place generators.
    Generator,
    /// Place probes.
    Probe,
    /// Place virtual instruments.
    VirtualInstruments,
    /// 2D graphics: lines.
    Line2d,
    /// 2D graphics: boxes.
    Box2d,
    /// 2D graphics: circles.
    Circle2d,
    /// 2D graphics: arcs.
    Arc2d,
    /// 2D graphics: closed paths.
    ClosedPath2d,
    /// 2D graphics: text.
    Text2d,
    /// 2D graphics: symbols.
    Symbols2d,
    /// 2D graphics: markers.
    Markers2d,
}

impl Mode {
    /// Every mode in toolbar order.
    pub const ALL: [Mode; 22] = [
        Mode::Selection,
        Mode::Component,
        Mode::JunctionDot,
        Mode::WireLabel,
        Mode::TextScript,
        Mode::Buses,
        Mode::Subcircuit,
        Mode::Terminals,
        Mode::DevicePins,
        Mode::Graph,
        Mode::ActivePopup,
        Mode::Generator,
        Mode::Probe,
        Mode::VirtualInstruments,
        Mode::Line2d,
        Mode::Box2d,
        Mode::Circle2d,
        Mode::Arc2d,
        Mode::ClosedPath2d,
        Mode::Text2d,
        Mode::Symbols2d,
        Mode::Markers2d,
    ];

    /// Stable id (the `mode` parameter of `set-mode`).
    pub fn id(self) -> &'static str {
        match self {
            Mode::Selection => "selection",
            Mode::Component => "component",
            Mode::JunctionDot => "junction-dot",
            Mode::WireLabel => "wire-label",
            Mode::TextScript => "text-script",
            Mode::Buses => "buses",
            Mode::Subcircuit => "subcircuit",
            Mode::Terminals => "terminals",
            Mode::DevicePins => "device-pins",
            Mode::Graph => "graph",
            Mode::ActivePopup => "active-popup",
            Mode::Generator => "generator",
            Mode::Probe => "probe",
            Mode::VirtualInstruments => "virtual-instruments",
            Mode::Line2d => "line-2d",
            Mode::Box2d => "box-2d",
            Mode::Circle2d => "circle-2d",
            Mode::Arc2d => "arc-2d",
            Mode::ClosedPath2d => "closed-path-2d",
            Mode::Text2d => "text-2d",
            Mode::Symbols2d => "symbols-2d",
            Mode::Markers2d => "markers-2d",
        }
    }

    /// Proteus's name of the mode.
    pub fn label(self) -> &'static str {
        match self {
            Mode::Selection => "Selection Mode",
            Mode::Component => "Component Mode",
            Mode::JunctionDot => "Junction Dot Mode",
            Mode::WireLabel => "Wire Label Mode",
            Mode::TextScript => "Text Script Mode",
            Mode::Buses => "Buses Mode",
            Mode::Subcircuit => "Subcircuit Mode",
            Mode::Terminals => "Terminals Mode",
            Mode::DevicePins => "Device Pins Mode",
            Mode::Graph => "Graph Mode",
            Mode::ActivePopup => "Active Popup Mode",
            Mode::Generator => "Generator Mode",
            Mode::Probe => "Probe Mode",
            Mode::VirtualInstruments => "Virtual Instruments Mode",
            Mode::Line2d => "2D Graphics Line Mode",
            Mode::Box2d => "2D Graphics Box Mode",
            Mode::Circle2d => "2D Graphics Circle Mode",
            Mode::Arc2d => "2D Graphics Arc Mode",
            Mode::ClosedPath2d => "2D Graphics Closed Path Mode",
            Mode::Text2d => "2D Graphics Text Mode",
            Mode::Symbols2d => "2D Graphics Symbols Mode",
            Mode::Markers2d => "2D Graphics Markers Mode",
        }
    }

    /// The mode with this id (ASCII case ignored).
    pub fn from_id(id: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|m| m.id().eq_ignore_ascii_case(id.trim()))
    }

    /// Whether clicks in this mode do something yet. The others are selectable but report
    /// "not available yet".
    pub fn is_built(self) -> bool {
        matches!(
            self,
            Mode::Selection
                | Mode::Component
                | Mode::JunctionDot
                | Mode::WireLabel
                | Mode::TextScript
                | Mode::Buses
                | Mode::Subcircuit
                | Mode::Terminals
                | Mode::Line2d
                | Mode::Box2d
                | Mode::Circle2d
                | Mode::Arc2d
                | Mode::ClosedPath2d
                | Mode::Text2d
        )
    }

    /// Whether clicking a pin end or a wire end starts a wire in this mode (Proteus draws
    /// wires from any placement mode; the modes whose click is about wires themselves do not).
    pub fn starts_wires(self) -> bool {
        !matches!(self, Mode::JunctionDot | Mode::WireLabel | Mode::Buses) && !self.is_2d_graphics()
    }

    /// Whether the mode is one of the 2D graphics modes.
    pub fn is_2d_graphics(self) -> bool {
        matches!(
            self,
            Mode::Line2d | Mode::Box2d | Mode::Circle2d | Mode::Arc2d | Mode::ClosedPath2d | Mode::Text2d | Mode::Symbols2d | Mode::Markers2d
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_round_trip() {
        for m in Mode::ALL {
            assert_eq!(Mode::from_id(m.id()), Some(m));
            assert!(m.label().ends_with("Mode"));
        }
        let mut ids: Vec<_> = Mode::ALL.iter().map(|m| m.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 22);
        assert_eq!(Mode::from_id(" Component "), Some(Mode::Component));
        assert_eq!(Mode::from_id("wire"), None);
    }

    #[test]
    fn wiring_rules() {
        assert!(Mode::Selection.starts_wires());
        assert!(Mode::Component.starts_wires());
        assert!(!Mode::WireLabel.starts_wires());
        assert!(!Mode::Text2d.starts_wires());
    }
}
