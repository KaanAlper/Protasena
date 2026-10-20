//! Wire paths shared by the `wire` command and the interactive wire tool: where a wire starts
//! and ends ([`Anchor`]), the routed path, and the junction dots a new wire needs.

use protasena_doc::{ComponentId, Sheet};
use protasena_geom::{Pt, Rect, Vec2};
use protasena_library::Library;
use serde_json::{Value, json};

use crate::bodies::{body_bounds, outward, visible_pins};
use crate::hit::closest_on;
use crate::route::{route_through, simplify};

/// Two points closer than this (thou) are the same point.
pub const SAME: f64 = 1e-6;

/// One end of a wire.
#[derive(Debug, Clone, PartialEq)]
pub enum Anchor {
    /// A component's pin end.
    Pin {
        /// The component.
        component: ComponentId,
        /// Pin number.
        pin: String,
        /// The pin end on the sheet.
        at: Pt,
        /// Direction a wire leaves the pin.
        out: Vec2,
    },
    /// A free point (a wire end, a junction, a point on a wire or empty space).
    Point(Pt),
}

impl Anchor {
    /// The point.
    pub fn at(&self) -> Pt {
        match self {
            Anchor::Pin { at, .. } | Anchor::Point(at) => *at,
        }
    }

    /// The direction a wire must leave it, for pins.
    pub fn out(&self) -> Option<Vec2> {
        match self {
            Anchor::Pin { out, .. } => Some(*out),
            Anchor::Point(_) => None,
        }
    }

    /// The JSON form the `wire` command reads.
    pub fn to_json(&self) -> Value {
        match self {
            Anchor::Pin { component, pin, .. } => json!({ "component": component.0, "pin": pin }),
            Anchor::Point(p) => json!([p.x, p.y]),
        }
    }
}

/// The pin `pin` of component `id` on `sheet`, as an anchor.
pub fn pin_anchor(sheet: &Sheet, library: &Library, id: ComponentId, pin: &str) -> Option<Anchor> {
    let c = sheet.components.iter().find(|c| c.id == id)?;
    let p = visible_pins(library, c).into_iter().find(|p| p.number == pin)?;
    Some(Anchor::Pin { component: id, pin: p.number.clone(), at: p.at, out: outward(&p) })
}

/// Half size of the box a connection point is to the router, in thou.
const POINT_HALF: f64 = 0.5;

/// What a new wire between `ends` should not pass through: component bodies, and the
/// connection points (pin ends, wire ends, junctions, terminals) other than its own ends, where
/// it would join nets it does not belong to.
pub fn obstacles(sheet: &Sheet, library: &Library, ends: &[Pt]) -> Vec<Rect> {
    let mut out: Vec<Rect> = sheet.components.iter().map(|c| body_bounds(library, c)).collect();
    let pins = sheet.components.iter().flat_map(|c| visible_pins(library, c)).map(|p| p.at);
    let wire_ends = sheet.wires.iter().flat_map(|w| [w.points.first().copied(), w.points.last().copied()]).flatten();
    let others = sheet.junctions.iter().map(|j| j.at).chain(sheet.terminals.iter().map(|t| t.at));
    let points = pins.chain(wire_ends).chain(others).filter(|p| !ends.iter().any(|e| e.distance(*p) <= SAME));
    out.extend(points.map(|p| Rect::from_point(p).inflate(POINT_HALF)));
    out
}

/// The path of a wire from `from` through `via` to `to`: routed orthogonally when `route`,
/// else straight between the points. Repeated points are dropped; the result has at least two
/// points.
pub fn path(sheet: &Sheet, library: &Library, from: &Anchor, via: &[Pt], to: &Anchor, route: bool, grid: f64) -> Vec<Pt> {
    let mut fixed = Vec::with_capacity(via.len().saturating_add(2));
    fixed.push(from.at());
    fixed.extend_from_slice(via);
    fixed.push(to.at());
    let points =
        if route { route_through(&fixed, from.out(), to.out(), &obstacles(sheet, library, &[from.at(), to.at()]), grid) } else { dedup(fixed) };
    if points.len() < 2 { vec![from.at(), to.at()] } else { points }
}

/// Points without consecutive repeats.
pub fn dedup(points: Vec<Pt>) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(points.len());
    for p in points {
        if !out.last().is_some_and(|l| l.distance(p) <= SAME) {
            out.push(p);
        }
    }
    out
}

/// Where a new wire with `points` needs junction dots on `sheet`: an end of the new wire on
/// another wire's interior (a T), an end where two or more wire ends already meet, and an
/// existing wire end on the new wire's interior. Points with a junction already are skipped.
pub fn junctions_for(sheet: &Sheet, points: &[Pt]) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::new();
    let ends: Vec<Pt> = [points.first(), points.last()].into_iter().flatten().copied().collect();
    for &e in &ends {
        let on_interior = sheet.wires.iter().any(|w| on_interior(&w.points, e));
        let meeting = sheet.wires.iter().flat_map(|w| [w.points.first(), w.points.last()]).flatten().filter(|q| q.distance(e) <= SAME).count();
        if on_interior || meeting >= 2 {
            out.push(e);
        }
    }
    for w in &sheet.wires {
        for q in [w.points.first(), w.points.last()].into_iter().flatten() {
            if on_interior(points, *q) {
                out.push(*q);
            }
        }
    }
    let mut unique: Vec<Pt> = Vec::new();
    for p in out {
        let known = sheet.junctions.iter().any(|j| j.at.distance(p) <= SAME) || unique.iter().any(|u| u.distance(p) <= SAME);
        if !known {
            unique.push(p);
        }
    }
    unique
}

/// Whether `p` lies on the polyline away from its two ends.
pub fn on_interior(points: &[Pt], p: Pt) -> bool {
    let at_end = [points.first(), points.last()].into_iter().flatten().any(|e| e.distance(p) <= SAME);
    !at_end && closest_on(points, p).is_some_and(|(_, d)| d <= SAME)
}

/// Moves one end of an orthogonal wire to `to`, keeping its segments orthogonal: the
/// neighbouring corner slides along, or an elbow (a Z through the middle) is added.
/// `at_start` picks the first point, else the last.
pub fn stretch(points: &[Pt], at_start: bool, to: Pt, grid: f64) -> Vec<Pt> {
    let mut pts: Vec<Pt> = points.to_vec();
    if !at_start {
        pts.reverse();
    }
    let out = match pts.as_slice() {
        [p0, p1, rest @ ..] => {
            let (p0, p1) = (*p0, *p1);
            let horizontal = p0.y == p1.y;
            let vertical = p0.x == p1.x;
            let next_vertical = rest.first().is_some_and(|p2| p2.x == p1.x);
            let next_horizontal = rest.first().is_some_and(|p2| p2.y == p1.y);
            let mut head: Vec<Pt> = if (horizontal && to.y == p0.y) || (vertical && to.x == p0.x) || !(horizontal || vertical) {
                vec![to, p1]
            } else if horizontal && next_vertical {
                vec![to, Pt::new(p1.x, to.y)]
            } else if vertical && next_horizontal {
                vec![to, Pt::new(to.x, p1.y)]
            } else if horizontal {
                let xm = protasena_geom::snap_scalar((to.x + p1.x) / 2.0, grid);
                vec![to, Pt::new(xm, to.y), Pt::new(xm, p1.y), p1]
            } else {
                let ym = protasena_geom::snap_scalar((to.y + p1.y) / 2.0, grid);
                vec![to, Pt::new(to.x, ym), Pt::new(p1.x, ym), p1]
            };
            head.extend_from_slice(rest);
            head
        }
        _ => vec![to, to],
    };
    let mut out = simplify(out);
    if out.len() < 2 {
        out = vec![to, to];
    }
    if !at_start {
        out.reverse();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Pt {
        Pt::new(x, y)
    }

    #[test]
    fn stretching_keeps_wires_orthogonal() {
        // Straight horizontal wire, end moved up: a Z through the middle.
        let s = stretch(&[p(0.0, 0.0), p(600.0, 0.0)], true, p(0.0, 200.0), 100.0);
        assert_eq!(s, [p(0.0, 200.0), p(300.0, 200.0), p(300.0, 0.0), p(600.0, 0.0)]);
        // Moved along its own axis: just longer.
        assert_eq!(stretch(&[p(0.0, 0.0), p(600.0, 0.0)], false, p(800.0, 0.0), 100.0), [p(0.0, 0.0), p(800.0, 0.0)]);
        // An L: the corner slides.
        let l = stretch(&[p(0.0, 0.0), p(300.0, 0.0), p(300.0, 400.0)], true, p(100.0, 100.0), 100.0);
        assert_eq!(l, [p(100.0, 100.0), p(300.0, 100.0), p(300.0, 400.0)]);
        // The last point of an L.
        let l2 = stretch(&[p(0.0, 0.0), p(300.0, 0.0), p(300.0, 400.0)], false, p(500.0, 500.0), 100.0);
        assert_eq!(l2, [p(0.0, 0.0), p(500.0, 0.0), p(500.0, 500.0)]);
        assert!(s.windows(2).all(|w| w[0].x == w[1].x || w[0].y == w[1].y));
    }

    #[test]
    fn junctions_at_tees() {
        let mut sheet = Sheet::default();
        sheet.wires.push(protasena_doc::Wire { id: protasena_doc::WireId(1), points: vec![p(0.0, 0.0), p(1000.0, 0.0)] });
        assert_eq!(junctions_for(&sheet, &[p(500.0, 0.0), p(500.0, 500.0)]), [p(500.0, 0.0)]);
        assert!(junctions_for(&sheet, &[p(1000.0, 0.0), p(1000.0, 500.0)]).is_empty());
        // An existing end on the new wire's interior.
        assert_eq!(junctions_for(&sheet, &[p(0.0, -500.0), p(0.0, 500.0)]), [p(0.0, 0.0)]);
        assert!(on_interior(&[p(0.0, 0.0), p(10.0, 0.0)], p(5.0, 0.0)));
        assert!(!on_interior(&[p(0.0, 0.0), p(10.0, 0.0)], p(10.0, 0.0)));
    }
}
