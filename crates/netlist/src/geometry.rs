//! Point matching for connectivity: exact-ish coincidence of points and points lying on wire
//! segments, with a fixed tolerance in thou.

use std::collections::HashMap;

use protasena_geom::{Pt, Segment};

/// Two points closer than this (in thou, per axis after rounding) are the same point, and a
/// point this close to a segment lies on it.
pub const TOLERANCE: f64 = 1e-3;

/// Coordinates beyond this magnitude (thou) cannot be compared at [`TOLERANCE`] and connect to
/// nothing (about 230 km: far outside any sheet).
pub const MAX_COORD: f64 = 9.0e12;

/// Grid cell of the segment index, in thou.
const CELL: f64 = 1000.0;

/// Segments covering more cells than this are kept in a list searched for every query.
const MAX_CELLS: i64 = 256;

/// A point rounded to the tolerance grid; `None` for a point that cannot be compared.
pub fn key(p: Pt) -> Option<(i64, i64)> {
    Some((quantize(p.x)?, quantize(p.y)?))
}

/// Whether a point can take part in connectivity.
pub fn usable(p: Pt) -> bool {
    key(p).is_some()
}

fn quantize(v: f64) -> Option<i64> {
    if !v.is_finite() || v.abs() > MAX_COORD {
        return None;
    }
    let q = (v / TOLERANCE).round();
    // |q| <= 9e15 < 2^63, so the conversion is exact and cannot saturate.
    Some(q as i64)
}

fn cell(v: f64) -> i64 {
    // Only called with |v| <= MAX_COORD + TOLERANCE, so the quotient fits easily.
    (v / CELL).floor() as i64
}

/// Wire segments indexed for "which segments pass through this point?".
#[derive(Debug, Default)]
pub struct SegmentIndex {
    segments: Vec<(Segment, usize)>,
    cells: HashMap<(i64, i64), Vec<usize>>,
    long: Vec<usize>,
}

impl SegmentIndex {
    /// Adds a segment carrying `tag`; segments with an unusable end are left out (returns
    /// `false`).
    pub fn insert(&mut self, seg: Segment, tag: usize) -> bool {
        if !(usable(seg.a) && usable(seg.b)) {
            return false;
        }
        let i = self.segments.len();
        self.segments.push((seg, tag));
        let b = seg.bounds();
        let (x0, x1) = (cell(b.min.x - TOLERANCE), cell(b.max.x + TOLERANCE));
        let (y0, y1) = (cell(b.min.y - TOLERANCE), cell(b.max.y + TOLERANCE));
        let count = (x1 - x0 + 1).saturating_mul(y1 - y0 + 1);
        if count > MAX_CELLS {
            self.long.push(i);
        } else {
            for cx in x0..=x1 {
                for cy in y0..=y1 {
                    self.cells.entry((cx, cy)).or_default().push(i);
                }
            }
        }
        true
    }

    /// Tags of the segments within [`TOLERANCE`] of `p` (possibly repeated).
    pub fn hits(&self, p: Pt) -> Vec<usize> {
        if !usable(p) {
            return Vec::new();
        }
        let near = self.cells.get(&(cell(p.x), cell(p.y))).into_iter().flatten().chain(&self.long);
        near.filter_map(|&i| self.segments.get(i)).filter(|(s, _)| s.distance_to_pt(p) <= TOLERANCE).map(|&(_, tag)| tag).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_to_the_tolerance_and_refuse_wild_values() {
        assert_eq!(key(Pt::new(100.0, -200.0)), key(Pt::new(100.0001, -200.0002)));
        assert_ne!(key(Pt::new(100.0, 0.0)), key(Pt::new(100.01, 0.0)));
        assert_eq!(key(Pt::new(f64::NAN, 0.0)), None);
        assert_eq!(key(Pt::new(0.0, 1e300)), None);
        assert!(key(Pt::new(-MAX_COORD, MAX_COORD)).is_some());
    }

    #[test]
    fn index_finds_segments_through_a_point() {
        let mut idx = SegmentIndex::default();
        assert!(idx.insert(Segment::new(Pt::new(0.0, 0.0), Pt::new(5000.0, 0.0)), 1));
        assert!(idx.insert(Segment::new(Pt::new(-1e9, 7.0), Pt::new(1e9, 7.0)), 2));
        assert!(idx.insert(Segment::new(Pt::new(1000.0, -500.0), Pt::new(1000.0, 500.0)), 3));
        assert!(!idx.insert(Segment::new(Pt::new(f64::NAN, 0.0), Pt::ORIGIN), 4));
        let mut hits = idx.hits(Pt::new(1000.0, 0.0));
        hits.sort_unstable();
        assert_eq!(hits, [1, 3]);
        assert_eq!(idx.hits(Pt::new(123_456.0, 7.0)), [2]);
        assert!(idx.hits(Pt::new(2500.0, 1.0)).is_empty());
        assert!(idx.hits(Pt::new(f64::INFINITY, 0.0)).is_empty());
    }
}
