//! 2D graphics as primitives: lines, boxes, circles, arcs and closed paths.

use protasena_doc::{Graphic, GraphicShape, ItemId};
use protasena_geom::{Pt, arc_through};

use crate::prim::{Ink, Prim, Shape};

/// Line width of 2D graphics, in thou.
pub const GRAPHIC_WIDTH: f64 = 8.0;

/// The primitive of a 2D graphic.
pub fn graphic_prims(g: &Graphic) -> Vec<Prim> {
    vec![Prim::new(graphic_shape(&g.shape), Ink::Graphic).width(GRAPHIC_WIDTH).owned(Some(ItemId::Graphic(g.id)))]
}

/// The drawn shape of a 2D graphic (an arc through three collinear points is a line).
pub fn graphic_shape(s: &GraphicShape) -> Shape {
    match s {
        GraphicShape::Line { a, b } => Shape::Line { a: *a, b: *b },
        GraphicShape::Box { a, b } => Shape::Polyline { points: vec![*a, Pt::new(b.x, a.y), *b, Pt::new(a.x, b.y)], closed: true },
        GraphicShape::Circle { center, radius } => Shape::Circle { center: *center, radius: *radius },
        GraphicShape::Arc { start, mid, end } => match arc_through(*start, *mid, *end) {
            Some(arc) => Shape::Arc { center: arc.center, radius: arc.radius, start: arc.start, sweep: arc.sweep },
            None => Shape::Line { a: *start, b: *end },
        },
        GraphicShape::Path { points } => Shape::Polyline { points: points.clone(), closed: true },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protasena_doc::GraphicId;

    #[test]
    fn every_kind_has_its_shape_and_owner() {
        let p = |x, y| Pt::new(x, y);
        let g = |shape| Graphic { id: GraphicId(4), shape };
        let b = graphic_prims(&g(GraphicShape::Box { a: p(0.0, 0.0), b: p(200.0, 100.0) }));
        assert_eq!(b[0].owner, Some(ItemId::Graphic(GraphicId(4))));
        assert!(matches!(&b[0].shape, Shape::Polyline { points, closed: true } if points.len() == 4 && points[1] == p(200.0, 0.0)));
        let a = graphic_shape(&GraphicShape::Arc { start: p(100.0, 0.0), mid: p(0.0, 100.0), end: p(-100.0, 0.0) });
        assert!(matches!(a, Shape::Arc { radius, sweep, .. } if (radius - 100.0).abs() < 1e-9 && sweep > 0.0));
        let flat = graphic_shape(&GraphicShape::Arc { start: p(0.0, 0.0), mid: p(1.0, 0.0), end: p(2.0, 0.0) });
        assert_eq!(flat, Shape::Line { a: p(0.0, 0.0), b: p(2.0, 0.0) });
        assert!(matches!(graphic_shape(&GraphicShape::Circle { center: p(0.0, 0.0), radius: 50.0 }), Shape::Circle { .. }));
    }
}
