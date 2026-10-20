//! Drawing 2D graphics with the mouse, and hit testing them.
//!
//! Built the conventional way (Proteus's own clicks were not observable yet, see
//! `docs/gaps.md`): **Line**, **Box** and **Circle** take two clicks (ends; opposite corners;
//! centre then a point on the circle), or a press, drag and release. **Arc** takes three clicks:
//! start, end, then a point on the arc (a drag from start to end gives the first two).
//! **Closed Path** adds a corner per click and closes on a double click or the right button
//! (three corners at least). Escape cancels. A **subcircuit**'s box is drawn like a 2D box.

use protasena_doc::GraphicShape;
use protasena_geom::{Pt, Segment, arc_through};
use serde_json::{Value, json};

use crate::mode::Mode;

/// Most corners of a closed path drawn with the mouse.
pub const MAX_DRAWN_CORNERS: usize = 1000;

/// How many clicked points finish a shape of `mode` (`None`: closed path, finished by a
/// double click or the right button).
pub fn points_needed(mode: Mode) -> Option<usize> {
    match mode {
        Mode::Line2d | Mode::Box2d | Mode::Circle2d | Mode::Subcircuit => Some(2),
        Mode::Arc2d => Some(3),
        _ => None,
    }
}

/// Whether `mode` draws a shape with the mouse.
pub fn draws_shape(mode: Mode) -> bool {
    matches!(mode, Mode::Line2d | Mode::Box2d | Mode::Circle2d | Mode::Arc2d | Mode::ClosedPath2d)
}

/// The shape the clicked `points` and the cursor make now, for the preview (an arc with only
/// its start shows the chord as a line).
pub fn draft(mode: Mode, points: &[Pt], cursor: Pt) -> Option<GraphicShape> {
    let first = *points.first()?;
    Some(match (mode, points) {
        (Mode::Line2d, _) => GraphicShape::Line { a: first, b: cursor },
        (Mode::Box2d | Mode::Subcircuit, _) => GraphicShape::Box { a: first, b: cursor },
        (Mode::Circle2d, _) => GraphicShape::Circle { center: first, radius: first.distance(cursor) },
        (Mode::Arc2d, [_, end, ..]) => GraphicShape::Arc { start: first, mid: cursor, end: *end },
        (Mode::Arc2d, _) => GraphicShape::Line { a: first, b: cursor },
        (Mode::ClosedPath2d, _) => {
            let mut pts = points.to_vec();
            if pts.last() != Some(&cursor) {
                pts.push(cursor);
            }
            GraphicShape::Path { points: pts }
        }
        _ => return None,
    })
}

/// The command and its JSON form that make the shape from all of its clicked `points`.
pub fn command(mode: Mode, points: &[Pt]) -> Option<(&'static str, Value)> {
    let xy = |p: &Pt| json!([p.x, p.y]);
    Some(match (mode, points) {
        (Mode::Line2d, [a, b]) => ("line", json!({ "from": xy(a), "to": xy(b) })),
        (Mode::Box2d, [a, b]) => ("box", json!({ "from": xy(a), "to": xy(b) })),
        (Mode::Subcircuit, [a, b]) => ("subcircuit", json!({ "from": xy(a), "to": xy(b) })),
        (Mode::Circle2d, [c, rim]) => ("circle", json!({ "center": xy(c), "radius": c.distance(*rim) })),
        (Mode::Arc2d, [start, end, mid]) => ("arc", json!({ "start": xy(start), "mid": xy(mid), "end": xy(end) })),
        (Mode::ClosedPath2d, pts) if pts.len() >= 3 => ("closed-path", json!({ "points": pts.iter().map(xy).collect::<Vec<_>>() })),
        _ => return None,
    })
}

/// Distance from `p` to the outline of a shape.
pub fn distance(shape: &GraphicShape, p: Pt) -> f64 {
    let polyline = |pts: &[Pt], closed: bool| {
        let n = pts.len();
        let segs = if closed { n } else { n.saturating_sub(1) };
        (0..segs).filter_map(|i| Some(Segment::new(*pts.get(i)?, *pts.get((i + 1) % n)?).distance_to_pt(p))).fold(f64::INFINITY, f64::min)
    };
    match shape {
        GraphicShape::Line { a, b } => Segment::new(*a, *b).distance_to_pt(p),
        GraphicShape::Box { a, b } => polyline(&[*a, Pt::new(b.x, a.y), *b, Pt::new(a.x, b.y)], true),
        GraphicShape::Circle { center, radius } => (center.distance(p) - radius).abs(),
        GraphicShape::Path { points } => polyline(points, true),
        GraphicShape::Arc { start, mid, end } => match arc_through(*start, *mid, *end) {
            Some(arc) => {
                let tau = std::f64::consts::TAU;
                let ang = (p.y - arc.center.y).atan2(p.x - arc.center.x);
                // How far round from the start, in the arc's own direction.
                let along = if arc.sweep >= 0.0 { (ang - arc.start).rem_euclid(tau) } else { (arc.start - ang).rem_euclid(tau) };
                let within = along <= arc.sweep.abs();
                if within { (arc.center.distance(p) - arc.radius).abs() } else { start.distance(p).min(end.distance(p)) }
            }
            None => Segment::new(*start, *end).distance_to_pt(p),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Pt {
        Pt::new(x, y)
    }

    #[test]
    fn outlines_are_hit_and_insides_are_not() {
        let b = GraphicShape::Box { a: p(0.0, 0.0), b: p(200.0, 100.0) };
        assert!(distance(&b, p(100.0, 0.0)) < 1e-9);
        assert!((distance(&b, p(100.0, 50.0)) - 50.0).abs() < 1e-9);
        let c = GraphicShape::Circle { center: p(0.0, 0.0), radius: 100.0 };
        assert!((distance(&c, p(0.0, 110.0)) - 10.0).abs() < 1e-9);
        let up = GraphicShape::Arc { start: p(100.0, 0.0), mid: p(0.0, 100.0), end: p(-100.0, 0.0) };
        assert!(distance(&up, p(0.0, 100.0)) < 1e-9);
        assert!((distance(&up, p(0.0, -100.0)) - 100.0 * std::f64::consts::SQRT_2).abs() < 1e-6, "the lower half is not part of it");
        let down = GraphicShape::Arc { start: p(100.0, 0.0), mid: p(0.0, -100.0), end: p(-100.0, 0.0) };
        assert!(distance(&down, p(0.0, -100.0)) < 1e-9);
    }

    #[test]
    fn drafts_and_commands_follow_the_clicks() {
        assert_eq!(draft(Mode::Circle2d, &[p(0.0, 0.0)], p(0.0, 50.0)), Some(GraphicShape::Circle { center: p(0.0, 0.0), radius: 50.0 }));
        assert_eq!(
            draft(Mode::Arc2d, &[p(0.0, 0.0), p(200.0, 0.0)], p(100.0, 50.0)),
            Some(GraphicShape::Arc { start: p(0.0, 0.0), mid: p(100.0, 50.0), end: p(200.0, 0.0) })
        );
        assert_eq!(draft(Mode::Line2d, &[], p(1.0, 1.0)), None);
        let (id, v) = command(Mode::Arc2d, &[p(0.0, 0.0), p(200.0, 0.0), p(100.0, 50.0)]).unwrap();
        assert_eq!((id, v["mid"].clone()), ("arc", json!([100.0, 50.0])));
        assert!(command(Mode::ClosedPath2d, &[p(0.0, 0.0), p(1.0, 0.0)]).is_none());
        assert_eq!(points_needed(Mode::ClosedPath2d), None);
    }
}
