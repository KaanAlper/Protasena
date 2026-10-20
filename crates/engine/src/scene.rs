//! The sheet as it will be after a move, for checking stretched wires: a stretched wire that
//! would pass through a component body or over another connection point (where it would join
//! a net it does not belong to) is routed again by the autorouter.

use std::collections::{HashMap, HashSet};

use protasena_doc::{Item, ItemId, Sheet};
use protasena_geom::{Pt, Rect, Vec2};
use protasena_library::Library;

use crate::bodies::{body_bounds, outward, visible_pins};
use crate::route::{End, crosses, route};
use crate::wiring::SAME;

/// Half size of the box a connection point is to the router, in thou.
const POINT_HALF: f64 = 0.5;

/// Bodies and connection points after a move.
pub struct Scene {
    bodies: Vec<Rect>,
    /// Pin ends with the direction a wire leaves them.
    pins: Vec<(Pt, Vec2)>,
    /// Junctions, terminals and the ends of the wires that are not re-shaped.
    points: Vec<Pt>,
}

impl Scene {
    /// The sheet with `moves` applied; the wires in `reshaped` are left out (they are what is
    /// being checked).
    pub fn after(sheet: &Sheet, library: &Library, moves: &[(Item, Item)], reshaped: &[ItemId]) -> Scene {
        let by_id: HashMap<ItemId, &Item> = moves.iter().map(|(a, b)| (a.id(), b)).collect();
        let reshaped: HashSet<&ItemId> = reshaped.iter().collect();
        let new_state = |id: ItemId| by_id.get(&id).map(|b| (*b).clone());
        let mut scene = Scene { bodies: Vec::new(), pins: Vec::new(), points: Vec::new() };
        for c in &sheet.components {
            let c = match new_state(ItemId::Component(c.id)) {
                Some(Item::Component(n)) => n,
                _ => c.clone(),
            };
            scene.bodies.push(body_bounds(library, &c));
            scene.pins.extend(visible_pins(library, &c).iter().map(|p| (p.at, outward(p))));
        }
        for j in &sheet.junctions {
            scene.points.push(match new_state(ItemId::Junction(j.id)) {
                Some(Item::Junction(n)) => n.at,
                _ => j.at,
            });
        }
        for t in &sheet.terminals {
            scene.points.push(match new_state(ItemId::Terminal(t.id)) {
                Some(Item::Terminal(n)) => n.at,
                _ => t.at,
            });
        }
        for x in &sheet.subcircuits {
            let x = match new_state(ItemId::Subcircuit(x.id)) {
                Some(Item::Subcircuit(n)) => n,
                _ => x.clone(),
            };
            // Ports sit on the box's edge, so the box is no obstacle (a wire may end on it).
            scene.points.extend(x.ports.iter().map(|p| p.at));
        }
        for w in sheet.wires.iter().filter(|w| !reshaped.contains(&ItemId::Wire(w.id))) {
            let points = match new_state(ItemId::Wire(w.id)) {
                Some(Item::Wire(n)) => n.points,
                _ => w.points.clone(),
            };
            scene.points.extend([points.first(), points.last()].into_iter().flatten().copied());
        }
        scene
    }

    /// The direction a wire leaves a pin end at `p`, if one is there.
    pub fn out_at(&self, p: Pt) -> Option<Vec2> {
        self.pins.iter().find(|(q, _)| q.distance(p) <= SAME).map(|(_, d)| *d)
    }

    fn foreign_points(&self, ends: &[Pt]) -> impl Iterator<Item = Pt> + '_ {
        let ends = ends.to_vec();
        self.pins.iter().map(|(p, _)| *p).chain(self.points.iter().copied()).filter(move |p| !ends.iter().any(|e| e.distance(*p) <= SAME))
    }

    /// Whether a wire with `points` passes through a body or over a connection point other
    /// than its own ends.
    pub fn is_bad(&self, points: &[Pt]) -> bool {
        let ends: Vec<Pt> = [points.first(), points.last()].into_iter().flatten().copied().collect();
        let segments: Vec<(Pt, Pt)> = points.windows(2).filter_map(|w| if let [a, b] = w { Some((*a, *b)) } else { None }).collect();
        let through_body = segments.iter().any(|&(a, b)| self.bodies.iter().any(|r| crosses(a, b, r)));
        through_body
            || self.foreign_points(&ends).any(|p| segments.iter().any(|&(a, b)| protasena_geom::Segment::new(a, b).distance_to_pt(p) <= SAME))
    }

    /// A fresh route between the ends of `points`, avoiding bodies and connection points.
    pub fn reroute(&self, points: &[Pt], grid: f64) -> Option<Vec<Pt>> {
        let (&from, &to) = (points.first()?, points.last()?);
        let mut obstacles = self.bodies.clone();
        obstacles.extend(self.foreign_points(&[from, to]).map(|p| Rect::from_point(p).inflate(POINT_HALF)));
        Some(route(End { at: from, out: self.out_at(from) }, End { at: to, out: self.out_at(to) }, &obstacles, grid))
    }
}
