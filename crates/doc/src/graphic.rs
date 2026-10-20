//! 2D graphics on a sheet: lines, boxes, circles, arcs and closed paths (Proteus's 2D
//! Graphics modes). They are drawing only: no connectivity.

use protasena_geom::{Pt, Rect};

use crate::error::DocError;
use crate::ids::GraphicId;

/// Most vertices of a closed path.
pub const MAX_PATH_POINTS: usize = 4096;

/// A 2D graphic.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Graphic {
    /// Stable id.
    pub id: GraphicId,
    /// What is drawn.
    pub shape: GraphicShape,
}

/// The shape of a 2D graphic, in sheet coordinates (thou, y up).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(tag = "kind", rename_all = "snake_case"))]
pub enum GraphicShape {
    /// A straight line.
    Line {
        /// One end.
        a: Pt,
        /// The other end.
        b: Pt,
    },
    /// An axis-aligned rectangle given by two opposite corners.
    Box {
        /// One corner.
        a: Pt,
        /// The opposite corner.
        b: Pt,
    },
    /// A circle.
    Circle {
        /// Centre.
        center: Pt,
        /// Radius in thou.
        radius: f64,
    },
    /// A circular arc from `start` through `mid` to `end`.
    Arc {
        /// Start point.
        start: Pt,
        /// A point on the arc between the ends.
        mid: Pt,
        /// End point.
        end: Pt,
    },
    /// A closed polygon (the last vertex joins the first).
    Path {
        /// Vertices, three or more.
        points: Vec<Pt>,
    },
}

impl GraphicShape {
    /// The kind's name (`line`, `box`, `circle`, `arc`, `path`).
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Line { .. } => "line",
            Self::Box { .. } => "box",
            Self::Circle { .. } => "circle",
            Self::Arc { .. } => "arc",
            Self::Path { .. } => "path",
        }
    }

    /// The same shape with every point mapped by `f` (radii kept: `f` must be rigid).
    pub fn map(&self, f: impl Fn(Pt) -> Pt) -> GraphicShape {
        match self {
            Self::Line { a, b } => Self::Line { a: f(*a), b: f(*b) },
            Self::Box { a, b } => Self::Box { a: f(*a), b: f(*b) },
            Self::Circle { center, radius } => Self::Circle { center: f(*center), radius: *radius },
            Self::Arc { start, mid, end } => Self::Arc { start: f(*start), mid: f(*mid), end: f(*end) },
            Self::Path { points } => Self::Path { points: points.iter().map(|&p| f(p)).collect() },
        }
    }

    /// The defining points (a circle's centre only).
    pub fn points(&self) -> Vec<Pt> {
        match self {
            Self::Line { a, b } | Self::Box { a, b } => vec![*a, *b],
            Self::Circle { center, .. } => vec![*center],
            Self::Arc { start, mid, end } => vec![*start, *mid, *end],
            Self::Path { points } => points.clone(),
        }
    }

    /// The box that holds the shape: its points, a circle's whole circle and an arc's bulge.
    pub fn bounds(&self) -> Option<Rect> {
        let pts = match self {
            Self::Circle { center, radius } => {
                let r = protasena_geom::Vec2::new(*radius, *radius);
                vec![*center - r, *center + r]
            }
            Self::Arc { start, mid, end } => match protasena_geom::arc_through(*start, *mid, *end) {
                Some(arc) => return Some(arc.bounds()),
                None => self.points(),
            },
            _ => self.points(),
        };
        pts.into_iter().map(Rect::from_point).reduce(Rect::union)
    }

    /// Checks coordinates (finite), the radius (finite, not negative) and the vertex count of
    /// a path (three to [`MAX_PATH_POINTS`]).
    ///
    /// # Errors
    /// [`DocError::NonFinite`] or [`DocError::Invalid`].
    pub fn check(&self) -> Result<(), DocError> {
        if !self.points().iter().all(|p| p.is_finite()) {
            return Err(DocError::NonFinite("graphic point"));
        }
        match self {
            Self::Circle { radius, .. } if !(radius.is_finite() && *radius >= 0.0) => Err(DocError::NonFinite("circle radius")),
            Self::Path { points } if points.len() < 3 || points.len() > MAX_PATH_POINTS => {
                Err(DocError::Invalid(format!("a closed path needs 3 to {MAX_PATH_POINTS} points, not {}", points.len())))
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_points_radius_and_path_length() {
        let p = |x, y| Pt::new(x, y);
        assert!(GraphicShape::Line { a: p(0.0, 0.0), b: p(1.0, 1.0) }.check().is_ok());
        assert_eq!(GraphicShape::Line { a: p(f64::NAN, 0.0), b: p(1.0, 1.0) }.check(), Err(DocError::NonFinite("graphic point")));
        assert_eq!(GraphicShape::Circle { center: p(0.0, 0.0), radius: -1.0 }.check(), Err(DocError::NonFinite("circle radius")));
        assert!(matches!(GraphicShape::Path { points: vec![p(0.0, 0.0), p(1.0, 0.0)] }.check(), Err(DocError::Invalid(_))));
        let c = GraphicShape::Circle { center: p(10.0, 0.0), radius: 5.0 };
        assert_eq!(c.bounds(), Some(Rect::new(p(5.0, -5.0), p(15.0, 5.0))));
        assert_eq!(c.map(|q| p(q.x + 1.0, q.y)), GraphicShape::Circle { center: p(11.0, 0.0), radius: 5.0 });
        // A half circle over the x axis bulges up to y = 100.
        let b = GraphicShape::Arc { start: p(-100.0, 0.0), mid: p(0.0, 100.0), end: p(100.0, 0.0) }.bounds().unwrap();
        assert!((b.max.y - 100.0).abs() < 1e-6 && b.min.y.abs() < 1e-6, "{b:?}");
        let b = GraphicShape::Arc { start: p(-100.0, 0.0), mid: p(0.0, -100.0), end: p(100.0, 0.0) }.bounds().unwrap();
        assert!((b.min.y + 100.0).abs() < 1e-6, "clockwise bulges down: {b:?}");
    }
}
