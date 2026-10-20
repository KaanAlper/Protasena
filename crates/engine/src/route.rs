//! The wire autorouter: orthogonal paths with few bends that leave pins away from their body
//! and avoid passing through component bodies where a simple path can.
//!
//! Each leg between two fixed points is routed on its own. The candidates are both one-bend L
//! shapes, two-bend Z shapes through the middle, U and Z detours one grid step beyond each
//! obstacle near the leg, and the same with a one-grid stub out of each pin. The best
//! candidate crosses the fewest obstacles (component bodies, and connection points as tiny
//! boxes), then leaves and enters pins along their axis, then has the fewest bends, then is the
//! shortest.

use protasena_geom::{Pt, Rect, Vec2, snap_scalar};

/// One end of a leg: the point and, for a pin end, the direction a wire must leave it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct End {
    /// The point.
    pub at: Pt,
    /// Outward direction (a unit axis vector) for pin ends.
    pub out: Option<Vec2>,
}

impl End {
    /// A free point.
    pub fn free(at: Pt) -> Self {
        Self { at, out: None }
    }
}

/// Routes `from` → `to` orthogonally around `obstacles` (component bodies); `grid` is the
/// snap step used for stubs and middle bends. The result starts at `from.at` and ends at
/// `to.at`, without repeated or collinear points.
pub fn route(from: End, to: End, obstacles: &[Rect], grid: f64) -> Vec<Pt> {
    let (a, b) = (from.at, to.at);
    if !(a.is_finite() && b.is_finite()) {
        return vec![a, b];
    }
    let grid = if grid.is_finite() && grid > 0.0 { grid } else { 100.0 };
    let heads = stub_options(a, from.out, grid);
    let tails = stub_options(b, to.out, grid);
    // Only obstacles near the leg can be crossed by a candidate or shape its detours.
    let near_leg = Rect::new(a, b).inflate(grid * 2.0);
    let nearby: Vec<Rect> = obstacles.iter().filter(|r| r.intersects(near_leg)).copied().collect();
    let mut candidates: Vec<Vec<Pt>> = Vec::new();
    for head in &heads {
        for tail in &tails {
            let (Some(&h), Some(&t)) = (head.last(), tail.first()) else { continue };
            for core in cores(h, t, &nearby, grid) {
                candidates.push(simplify(head.iter().chain(core.iter()).chain(tail.iter()).copied().collect()));
            }
        }
    }
    // Score against every obstacle the candidates can reach (one pass over all obstacles).
    let reach = candidates.iter().flatten().fold(Rect::from_point(a), |r, p| r.include(*p));
    let relevant: Vec<Rect> = obstacles.iter().filter(|r| r.intersects(reach)).copied().collect();
    let mut best: Option<(Score, Vec<Pt>)> = None;
    for path in candidates {
        let s = score(&path, from.out, to.out, &relevant);
        if best.as_ref().is_none_or(|(bs, _)| s < *bs) {
            best = Some((s, path));
        }
    }
    best.map_or_else(|| vec![a, b], |(_, p)| p)
}

/// Routes through fixed points: the first leg leaves `from_out`, the last enters along
/// `to_out`, middle corners are free.
pub fn route_through(points: &[Pt], from_out: Option<Vec2>, to_out: Option<Vec2>, obstacles: &[Rect], grid: f64) -> Vec<Pt> {
    if points.len() > MAX_FIXED {
        return points.to_vec();
    }
    let last = points.len().saturating_sub(1);
    let mut out: Vec<Pt> = Vec::new();
    for (i, pair) in points.windows(2).enumerate() {
        let [a, b] = pair else { continue };
        let from = End { at: *a, out: if i == 0 { from_out } else { None } };
        let to = End { at: *b, out: if i + 1 == last { to_out } else { None } };
        let leg = route(from, to, obstacles, grid);
        let skip = usize::from(!out.is_empty());
        out.extend(leg.into_iter().skip(skip));
    }
    if out.is_empty() { points.to_vec() } else { simplify(out) }
}

/// Drops repeated points and the middle point of three collinear ones (keeps the ends).
pub fn simplify(points: Vec<Pt>) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(points.len());
    for p in points {
        if out.last() == Some(&p) {
            continue;
        }
        if let [.., a, b] = out.as_slice()
            && collinear_through(*a, *b, p)
        {
            out.pop();
        }
        out.push(p);
    }
    out
}

/// `b` lies on the straight axis-aligned run from `a` to `c` (same line, not doubling back).
fn collinear_through(a: Pt, b: Pt, c: Pt) -> bool {
    let between = |p: f64, q: f64, r: f64| (p <= q && q <= r) || (p >= q && q >= r);
    (a.x == b.x && b.x == c.x && between(a.y, b.y, c.y)) || (a.y == b.y && b.y == c.y && between(a.x, b.x, c.x))
}

fn stub_options(at: Pt, out: Option<Vec2>, grid: f64) -> Vec<Vec<Pt>> {
    match out {
        Some(d) if d.is_finite() => vec![vec![at], vec![at, at + d * grid]],
        _ => vec![vec![at]],
    }
}

/// Most fixed points of one routed wire (more are joined straight, unrouted).
pub const MAX_FIXED: usize = 66;

/// Obstacles considered for detours, at most.
const MAX_DETOUR_OBSTACLES: usize = 16;

/// Orthogonal paths `h` → `t`: L shapes, Z shapes through the snapped middle, and detours
/// (U and Z shapes) one grid step beyond each body near the leg.
fn cores(h: Pt, t: Pt, obstacles: &[Rect], grid: f64) -> Vec<Vec<Pt>> {
    let mut xs = vec![snap_scalar((h.x + t.x) / 2.0, grid)];
    let mut ys = vec![snap_scalar((h.y + t.y) / 2.0, grid)];
    let span = Rect::new(h, t).inflate(grid);
    for r in obstacles.iter().filter(|r| r.intersects(span)).take(MAX_DETOUR_OBSTACLES) {
        let up = |v: f64| ((v + grid) / grid).ceil() * grid;
        let down = |v: f64| ((v - grid) / grid).floor() * grid;
        xs.extend([up(r.max.x), down(r.min.x)]);
        ys.extend([up(r.max.y), down(r.min.y)]);
    }
    let mut out = vec![vec![h, Pt::new(t.x, h.y), t], vec![h, Pt::new(h.x, t.y), t]];
    out.extend(xs.into_iter().filter(|x| x.is_finite()).map(|x| vec![h, Pt::new(x, h.y), Pt::new(x, t.y), t]));
    out.extend(ys.into_iter().filter(|y| y.is_finite()).map(|y| vec![h, Pt::new(h.x, y), Pt::new(t.x, y), t]));
    out
}

/// Lower is better: (body crossings, pin-axis violations, bends, length).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct Score(usize, usize, usize, f64);

fn score(path: &[Pt], from_out: Option<Vec2>, to_out: Option<Vec2>, obstacles: &[Rect]) -> Score {
    let segments: Vec<(Pt, Pt)> = path.windows(2).filter_map(|w| if let [a, b] = w { Some((*a, *b)) } else { None }).collect();
    let crossings = segments.iter().map(|&(a, b)| obstacles.iter().filter(|r| crosses(a, b, r)).count()).sum();
    let along = |seg: Option<&(Pt, Pt)>, dir: Option<Vec2>| match (seg, dir) {
        (Some(&(a, b)), Some(d)) => usize::from((b - a).dot(d) <= 0.0),
        _ => 0,
    };
    let violations = along(segments.first(), from_out) + along(segments.last(), to_out.map(|d| -d));
    let length = segments.iter().map(|&(a, b)| (b - a).manhattan_length()).sum();
    Score(crossings, violations, path.len().saturating_sub(2), length)
}

/// Whether segment `a`–`b` passes through the open interior of `r`.
pub(crate) fn crosses(a: Pt, b: Pt, r: &Rect) -> bool {
    let (lo_x, hi_x) = (a.x.min(b.x), a.x.max(b.x));
    let (lo_y, hi_y) = (a.y.min(b.y), a.y.max(b.y));
    let overlap = |lo: f64, hi: f64, min: f64, max: f64| if lo == hi { lo > min && lo < max } else { lo < max && hi > min };
    overlap(lo_x, hi_x, r.min.x, r.max.x) && overlap(lo_y, hi_y, r.min.y, r.max.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Pt {
        Pt::new(x, y)
    }

    #[test]
    fn aligned_points_get_a_straight_wire() {
        let r = route(End::free(p(0.0, 0.0)), End::free(p(500.0, 0.0)), &[], 100.0);
        assert_eq!(r, [p(0.0, 0.0), p(500.0, 0.0)]);
    }

    #[test]
    fn pins_facing_each_other_get_a_z() {
        let from = End { at: p(200.0, 0.0), out: Some(Vec2::new(1.0, 0.0)) };
        let to = End { at: p(800.0, 500.0), out: Some(Vec2::new(-1.0, 0.0)) };
        let r = route(from, to, &[], 100.0);
        assert_eq!(r, [p(200.0, 0.0), p(500.0, 0.0), p(500.0, 500.0), p(800.0, 500.0)]);
    }

    #[test]
    fn free_end_gets_an_l_leaving_the_pin_along_its_axis() {
        let from = End { at: p(200.0, 0.0), out: Some(Vec2::new(1.0, 0.0)) };
        let r = route(from, End::free(p(600.0, 400.0)), &[], 100.0);
        assert_eq!(r, [p(200.0, 0.0), p(600.0, 0.0), p(600.0, 400.0)]);
    }

    #[test]
    fn avoids_a_body_in_the_way() {
        // From the left pin of a body going right to a point behind the body.
        let body = Rect::new(p(-100.0, -40.0), p(100.0, 40.0));
        let from = End { at: p(-200.0, 0.0), out: Some(Vec2::new(-1.0, 0.0)) };
        let r = route(from, End::free(p(300.0, 0.0)), &[body], 100.0);
        let segs: Vec<_> = r.windows(2).map(|w| (w[0], w[1])).collect();
        assert!(segs.iter().all(|&(a, b)| !crosses(a, b, &body)), "{r:?}");
        assert_eq!(r.first(), Some(&p(-200.0, 0.0)));
        assert_eq!(r.last(), Some(&p(300.0, 0.0)));
    }

    #[test]
    fn route_through_fixed_corners_and_simplify() {
        let r = route_through(&[p(0.0, 0.0), p(300.0, 300.0), p(600.0, 300.0)], None, None, &[], 100.0);
        assert_eq!(r.first(), Some(&p(0.0, 0.0)));
        assert!(r.contains(&p(300.0, 300.0)) || r.windows(2).any(|w| w[0].y == 300.0 && w[1].y == 300.0));
        assert!(r.windows(2).all(|w| w[0].x == w[1].x || w[0].y == w[1].y));
        assert_eq!(simplify(vec![p(0.0, 0.0), p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0), p(2.0, 5.0)]), [p(0.0, 0.0), p(2.0, 0.0), p(2.0, 5.0)]);
        assert_eq!(route(End::free(p(f64::NAN, 0.0)), End::free(p(1.0, 0.0)), &[], 100.0).len(), 2);
    }
}
