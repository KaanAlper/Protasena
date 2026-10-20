//! Rigid placement transforms: an [`Orient`] followed by a translation.

use crate::orient::Orient;
use crate::point::{Pt, Vec2};
use crate::rect::Rect;

/// Maps local coordinates (for example a symbol's own) to world coordinates:
/// `p ↦ orient(p) + offset`.
///
/// `a.then(b)` applies `a` first and `b` second. With integer-valued offsets and coordinates
/// every operation is exact.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Transform {
    /// Orientation applied first, about the local origin.
    pub orient: Orient,
    /// Translation applied after the orientation.
    pub offset: Vec2,
}

impl Transform {
    /// The transform that changes nothing.
    pub const IDENTITY: Transform = Transform { orient: Orient::IDENTITY, offset: Vec2::ZERO };

    /// `p ↦ orient(p) + offset`.
    pub const fn new(orient: Orient, offset: Vec2) -> Self {
        Self { orient, offset }
    }

    /// A pure translation by `offset`.
    pub const fn translation(offset: Vec2) -> Self {
        Self { orient: Orient::IDENTITY, offset }
    }

    /// A pure orientation about the origin.
    pub const fn from_orient(orient: Orient) -> Self {
        Self { orient, offset: Vec2::ZERO }
    }

    /// Applies `orient` about `pivot` instead of the origin (`pivot` stays where it is).
    pub fn about(pivot: Pt, orient: Orient) -> Self {
        let v = pivot.to_vec();
        Self { orient, offset: v - orient.apply_vec(v) }
    }

    /// Maps a point.
    pub fn apply_pt(self, p: Pt) -> Pt {
        self.orient.apply_pt(p) + self.offset
    }

    /// Maps a vector (direction or size): orientation only, no translation.
    pub fn apply_vec(self, v: Vec2) -> Vec2 {
        self.orient.apply_vec(v)
    }

    /// Maps a rectangle; the result is the normalized rectangle through the mapped corners.
    pub fn apply_rect(self, r: Rect) -> Rect {
        Rect::new(self.apply_pt(r.min), self.apply_pt(r.max))
    }

    /// The transform that applies `self` first and `next` second.
    pub fn then(self, next: Transform) -> Transform {
        Transform { orient: self.orient.then(next.orient), offset: next.orient.apply_vec(self.offset) + next.offset }
    }

    /// The transform that undoes `self`.
    pub fn inverse(self) -> Transform {
        let inv = self.orient.inverse();
        Transform { orient: inv, offset: -inv.apply_vec(self.offset) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_a_symbol_pin() {
        // A pin at local (10, 0), symbol rotated 90° CCW and placed at (100, 50).
        let t = Transform::new(Orient::rotation(1), Vec2::new(100.0, 50.0));
        assert_eq!(t.apply_pt(Pt::new(10.0, 0.0)), Pt::new(100.0, 60.0));
        assert_eq!(t.apply_vec(Vec2::new(10.0, 0.0)), Vec2::new(0.0, 10.0));
        assert_eq!(t.inverse().apply_pt(Pt::new(100.0, 60.0)), Pt::new(10.0, 0.0));
    }

    #[test]
    fn about_rotates_around_the_pivot() {
        let t = Transform::about(Pt::new(1.0, 1.0), Orient::rotation(2));
        assert_eq!(t.apply_pt(Pt::new(2.0, 1.0)), Pt::new(0.0, 1.0));
        assert_eq!(t.apply_pt(Pt::new(1.0, 1.0)), Pt::new(1.0, 1.0));
    }

    #[test]
    fn then_translates_after_orienting() {
        let a = Transform::translation(Vec2::new(5.0, 0.0));
        let b = Transform::from_orient(Orient::MIRROR_X);
        assert_eq!(a.then(b).apply_pt(Pt::ORIGIN), Pt::new(-5.0, 0.0));
        assert_eq!(b.then(a).apply_pt(Pt::ORIGIN), Pt::new(5.0, 0.0));
        assert_eq!(Transform::IDENTITY.then(a), a);
    }

    #[test]
    fn apply_rect_normalizes() {
        let r = Rect::new(Pt::new(0.0, 0.0), Pt::new(4.0, 2.0));
        let t = Transform::from_orient(Orient::rotation(1));
        assert_eq!(t.apply_rect(r), Rect::new(Pt::new(-2.0, 0.0), Pt::new(0.0, 4.0)));
    }
}
