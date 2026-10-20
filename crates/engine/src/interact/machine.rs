//! The state of the interactive layer, and the wire tool (shared by every mode that starts
//! wires).

use protasena_geom::Pt;
use serde_json::json;

use super::Feedback;
use crate::error::Result;
use crate::hit::Hit;
use crate::session::Session;
use crate::wiring::Anchor;

/// What is in progress in the editing window.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Machine {
    /// Nothing.
    #[default]
    Idle,
    /// A wire follows the mouse.
    Wire(WireDraft),
    /// A bus is being drawn: its corners so far.
    Bus(Vec<Pt>),
    /// The left button is down in Selection mode.
    Press {
        /// The snapped press point.
        from: Pt,
        /// The press point as given.
        raw: Pt,
        /// The press was on an object (dragging moves the selection) or on empty space
        /// (dragging draws a selection box).
        on_item: bool,
        /// The pointer has moved beyond the pick tolerance since the press.
        dragging: bool,
        /// Ctrl or Shift was held: add to the selection.
        add: bool,
    },
}

/// A wire being drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct WireDraft {
    /// Where it starts.
    pub start: Anchor,
    /// Corners fixed by clicks so far.
    pub corners: Vec<Pt>,
}

impl WireDraft {
    /// The last fixed point.
    pub fn last_fixed(&self) -> Pt {
        self.corners.last().copied().unwrap_or_else(|| self.start.at())
    }
}

/// The anchor a hit offers a wire.
pub(crate) fn anchor_of(h: &Hit) -> Option<Anchor> {
    match h {
        Hit::PinEnd { component, pin, at, out } => Some(Anchor::Pin { component: *component, pin: pin.clone(), at: *at, out: *out }),
        other => other.attach_point().map(Anchor::Point),
    }
}

impl Session {
    /// Starts a wire at a hit.
    pub(crate) fn start_wire(&mut self, h: &Hit) -> Option<Feedback> {
        let start = anchor_of(h)?;
        self.machine = Machine::Wire(WireDraft { start, corners: Vec::new() });
        Some(Feedback::message("Click to fix a corner, on a pin or wire to finish; Escape cancels"))
    }

    /// A left click while a wire follows the mouse.
    pub(crate) fn wire_click(&mut self, mut draft: WireDraft, found: Option<Hit>) -> Result<Feedback> {
        let p = self.cursor;
        let target = found.as_ref().and_then(anchor_of);
        let at_start = draft.corners.is_empty() && target.as_ref().is_some_and(|t| t.at() == draft.start.at());
        if let Some(end) = target.filter(|_| !at_start) {
            return self.finish_wire(draft, end);
        }
        if p == draft.last_fixed() {
            if draft.corners.is_empty() {
                self.machine = Machine::Wire(draft);
                return Ok(Feedback::handled());
            }
            draft.corners.pop();
            return self.finish_wire(draft, Anchor::Point(p));
        }
        if draft.corners.len() >= crate::cmd::MAX_VIA {
            self.machine = Machine::Wire(draft);
            return Ok(Feedback::message("A wire takes at most 64 corners: finish it on a pin or wire"));
        }
        draft.corners.push(p);
        self.machine = Machine::Wire(draft);
        Ok(Feedback::handled())
    }

    /// Ends the wire at `end` and runs the `wire` command.
    pub(crate) fn finish_wire(&mut self, draft: WireDraft, end: Anchor) -> Result<Feedback> {
        self.machine = Machine::Idle;
        if end.at() == draft.start.at() && draft.corners.is_empty() {
            return Ok(Feedback::handled());
        }
        let via: Vec<[f64; 2]> = draft.corners.iter().map(|c| [c.x, c.y]).collect();
        let params = json!({ "from": draft.start.to_json(), "to": end.to_json(), "via": via, "route": self.autorouter });
        self.execute("wire", &params).map(Feedback::result)
    }
}
