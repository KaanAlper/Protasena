//! Pointer events per mode.

use protasena_doc::ItemId;
use protasena_geom::Rect;
use serde_json::{Value, json};

use super::{Feedback, Machine, PointerEvent};
use crate::error::Result;
use crate::hit::{Hit, items_in};
use crate::mode::Mode;
use crate::request::Request;
use crate::session::Session;

/// Most corners of a bus drawn with the mouse.
const MAX_BUS_CORNERS: usize = 1000;

impl Session {
    fn run(&mut self, id: &str, params: Value) -> Result<Feedback> {
        self.execute(id, &params).map(Feedback::result)
    }

    pub(super) fn on_move(&mut self, ev: PointerEvent) -> Feedback {
        let tol = self.view.pick_tolerance();
        if let Machine::Press { raw, dragging, .. } = &mut self.machine
            && !*dragging
            && raw.distance(ev.at) > tol
        {
            *dragging = true;
        }
        Feedback::default()
    }

    pub(super) fn on_press(&mut self, ev: PointerEvent, found: Option<Hit>) -> Result<Feedback> {
        match std::mem::take(&mut self.machine) {
            Machine::Wire(draft) => return self.wire_click(draft, found),
            Machine::Bus(mut points) => {
                if points.len() >= MAX_BUS_CORNERS {
                    self.machine = Machine::Bus(points);
                    return Ok(Feedback::message("A bus takes at most 1000 corners: double-click to finish it"));
                }
                if points.last() != Some(&self.cursor) {
                    points.push(self.cursor);
                }
                self.machine = Machine::Bus(points);
                return Ok(Feedback::handled());
            }
            Machine::Idle | Machine::Press { .. } => {}
        }
        if self.mode.starts_wires()
            && let Some(h) = found.as_ref().filter(|h| h.starts_wire())
            && let Some(fb) = self.start_wire(h)
        {
            return Ok(fb);
        }
        let at = [self.cursor.x, self.cursor.y];
        match self.mode {
            Mode::Selection => Ok(self.select_press(ev, found)),
            Mode::Component => self.component_press(found),
            Mode::JunctionDot => {
                let p = found.as_ref().map_or(self.cursor, |h| self.wire_point(h));
                self.run("junction", json!({ "at": [p.x, p.y] }))
            }
            Mode::WireLabel => Ok(match found {
                Some(Hit::Label { id }) => Feedback::request(Request::EditLabel { id }),
                Some(h @ (Hit::Wire { .. } | Hit::WireEnd { .. })) => Feedback::request(Request::WireLabel { at: self.wire_point(&h) }),
                _ => Feedback::message("Click on a wire to label it"),
            }),
            Mode::Terminals => self.run("terminal", json!({ "at": at })),
            Mode::Buses => {
                self.machine = Machine::Bus(vec![self.cursor]);
                Ok(Feedback::handled())
            }
            Mode::TextScript | Mode::Text2d => Ok(Feedback::request(Request::Text { at: self.cursor })),
            m => Ok(Feedback::message(format!("{} is not available yet", m.label()))),
        }
    }

    /// Where a click on a wire lands: the snapped cursor when it lies on the wire, else the
    /// closest point of the wire.
    fn wire_point(&self, h: &Hit) -> protasena_geom::Pt {
        let (Hit::Wire { wire, at } | Hit::WireEnd { wire, at }) = h else { return self.cursor };
        let on = self
            .sheet()
            .and_then(|sh| sh.wires.iter().find(|w| w.id == *wire))
            .is_some_and(|w| crate::hit::closest_on(&w.points, self.cursor).is_some_and(|(_, d)| d <= crate::wiring::SAME));
        if on { self.cursor } else { *at }
    }

    fn select_press(&mut self, ev: PointerEvent, found: Option<Hit>) -> Feedback {
        let add = ev.modifiers.ctrl || ev.modifiers.shift;
        let item = found.map(|h| h.item());
        if let Some(id) = item {
            let mut selection = self.selection.clone();
            match (selection.iter().position(|&x| x == id), add) {
                (None, false) => selection = vec![id],
                (None, true) => selection.push(id),
                (Some(i), true) => {
                    selection.remove(i);
                }
                (Some(_), false) => {}
            }
            self.set_selection(selection);
        }
        // A Ctrl/Shift click that took the item out of the selection does not drag.
        let on_item = item.is_some_and(|id| self.selection.contains(&id));
        self.machine = Machine::Press { from: self.snap(ev.at), raw: ev.at, on_item, dragging: false, add };
        Feedback::handled()
    }

    fn component_press(&mut self, found: Option<Hit>) -> Result<Feedback> {
        if let Some(Hit::Component { id }) = found {
            self.set_selection(vec![ItemId::Component(id)]);
            return Ok(Feedback::handled());
        }
        if self.device.is_none() {
            return Ok(Feedback::message("No device selected: pick one with P"));
        }
        let o = self.placement;
        self.run(
            "place-component",
            json!({ "at": [self.cursor.x, self.cursor.y], "orient": { "angle": o.angle_degrees(), "mirror": o.is_mirrored() } }),
        )
    }

    pub(super) fn on_right(&mut self) -> Result<Feedback> {
        match std::mem::take(&mut self.machine) {
            Machine::Bus(points) if points.len() >= 2 => self.finish_bus(points),
            Machine::Idle => Ok(Feedback::default()),
            _ => Ok(Feedback::handled()),
        }
    }

    fn finish_bus(&mut self, points: Vec<protasena_geom::Pt>) -> Result<Feedback> {
        self.machine = Machine::Idle;
        let pts: Vec<[f64; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
        self.run("bus", json!({ "points": pts }))
    }

    pub(super) fn on_release(&mut self, ev: PointerEvent) -> Result<Feedback> {
        let Machine::Press { from, raw, on_item, dragging, add } = self.machine else { return Ok(Feedback::default()) };
        self.machine = Machine::Idle;
        match (on_item, dragging) {
            (true, true) => {
                let by = self.snap(ev.at) - from;
                if by.length() == 0.0 || self.selection.is_empty() {
                    return Ok(Feedback::handled());
                }
                let ids: Vec<u64> = self.selection.iter().map(|id| id.raw()).collect();
                self.run("drag", json!({ "ids": ids, "by": [by.x, by.y] }))
            }
            (false, true) => {
                let Some(sheet) = self.sheet() else { return Ok(Feedback::handled()) };
                let inside = items_in(sheet, &self.library, Rect::new(raw, ev.at));
                let selection = if add { self.selection.iter().copied().chain(inside).collect() } else { inside };
                self.set_selection(selection);
                Ok(Feedback::handled())
            }
            (false, false) => {
                if !add {
                    self.set_selection(Vec::new());
                }
                Ok(Feedback::handled())
            }
            (true, false) => Ok(Feedback::handled()),
        }
    }

    pub(super) fn on_double(&mut self, found: Option<Hit>) -> Result<Feedback> {
        match std::mem::take(&mut self.machine) {
            Machine::Wire(draft) => {
                let end = crate::wiring::Anchor::Point(self.cursor);
                return self.finish_wire(draft, end);
            }
            Machine::Bus(points) if points.len() >= 2 => return self.finish_bus(points),
            Machine::Bus(_) | Machine::Idle | Machine::Press { .. } => {}
        }
        Ok(match found {
            Some(Hit::Component { id } | Hit::PinEnd { component: id, .. }) => Feedback::request(Request::EditComponent { id }),
            Some(Hit::Label { id }) => Feedback::request(Request::EditLabel { id }),
            Some(Hit::Terminal { id, .. }) => Feedback::request(Request::EditTerminal { id }),
            Some(Hit::Text { id }) => Feedback::request(Request::EditText { id }),
            _ => Feedback::default(),
        })
    }
}
