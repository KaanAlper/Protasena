//! What the editing window draws on top of the design while a tool is in use.

use protasena_doc::{Component, ComponentId, Item, Op};
use protasena_geom::{Pt, Transform};
use protasena_netlist::DeviceSource;
use serde::Serialize;

use super::Machine;
use crate::hit::Hit;
use crate::mode::Mode;
use crate::session::Session;
use crate::wiring::{Anchor, path};

/// Rubber-band geometry for the UI.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Preview {
    /// The wire being drawn, as it would be made now.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub wire: Vec<Pt>,
    /// The bus being drawn, up to the cursor.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bus: Vec<Pt>,
    /// The component Component mode would place at the cursor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ghost: Option<Component>,
    /// Dragged items in their new state (wires that follow included).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub moved: Vec<Item>,
    /// The selection box, two corners.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rect: Option<[Pt; 2]>,
    /// What is under the pointer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hover: Option<Hit>,
}

impl Session {
    /// The preview for the current pointer position.
    pub fn preview(&self) -> Preview {
        let mut p = Preview { hover: self.hover.clone(), ..Preview::default() };
        let Some(sheet) = self.sheet() else { return p };
        match &self.machine {
            Machine::Wire(draft) => {
                let end = match &self.hover {
                    Some(h @ Hit::PinEnd { at, .. }) if *at != draft.start.at() => super::machine::anchor_of(h).unwrap_or(Anchor::Point(self.cursor)),
                    _ => Anchor::Point(self.cursor),
                };
                p.wire = path(sheet, &self.library, &draft.start, &draft.corners, &end, self.autorouter, self.view.snap.step());
            }
            Machine::Bus(points) => {
                p.bus = points.clone();
                if p.bus.last() != Some(&self.cursor) {
                    p.bus.push(self.cursor);
                }
            }
            Machine::Press { from, raw, on_item, dragging: true, .. } => {
                if *on_item {
                    let by = self.snap(self.raw_cursor) - *from;
                    let ops = crate::cmd::drag_ops(self, &self.selection, by).unwrap_or_default();
                    p.moved = ops.into_iter().filter_map(|op| if let Op::Modify { item } = op { Some(item) } else { None }).collect();
                } else {
                    p.rect = Some([*raw, self.raw_cursor]);
                }
            }
            Machine::Press { .. } => {}
            Machine::Idle => {
                if self.mode == Mode::Component
                    && let Some(d) = self.device.as_ref().and_then(|r| DeviceSource::device(&self.library, r))
                {
                    let t = Transform::new(self.placement, self.cursor.to_vec());
                    p.ghost = Some(crate::cmd::new_component(self, d, ComponentId(0), self.cursor, t));
                }
            }
        }
        p
    }
}
