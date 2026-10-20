//! Subcircuits as primitives: the box, its name above it, and each port drawn inside the box as
//! a module port (our own shapes: an arrow into the box for inputs, out of it for outputs, a
//! hexagon for bidirectional ports, a plain tab for the others) with its name beside it.

use protasena_doc::{ItemId, PortKind, PortSide, Subcircuit};
use protasena_geom::Pt;

use crate::items::{LABEL_SIZE, REFERENCE_SIZE};
use crate::prim::{HAlign, Ink, Prim, Shape, VAlign};
use crate::symbol::BODY_WIDTH;

/// Length of a port's shape into the box, in thou.
pub const PORT_LENGTH: f64 = 160.0;
/// Half height of a port's shape, in thou.
const PORT_HALF: f64 = 40.0;
/// Gap between a port's shape and its name, in thou.
const PORT_GAP: f64 = 30.0;
/// Gap between the box and the subcircuit's name, in thou.
const NAME_LIFT: f64 = 30.0;
/// Most ports drawn for one subcircuit.
const MAX_DRAWN: usize = 1024;

/// The outline of a port of `kind` from its connection point, along +x (`dx` = 1: into the box
/// from the left edge; -1 from the right edge).
fn port_outline(kind: PortKind, at: Pt, dx: f64) -> Vec<Pt> {
    let (l, h) = (PORT_LENGTH, PORT_HALF);
    let shape: &[(f64, f64)] = match kind {
        // Seen from the box: an input points into it, an output out of it.
        PortKind::Input => &[(0.0, -h), (l - h, -h), (l, 0.0), (l - h, h), (0.0, h)],
        PortKind::Output => &[(0.0, 0.0), (h, -h), (l, -h), (l, h), (h, h)],
        PortKind::Bidir => &[(0.0, 0.0), (h, -h), (l - h, -h), (l, 0.0), (l - h, h), (h, h)],
        PortKind::Default => &[(0.0, -h), (l, -h), (l, h), (0.0, h)],
    };
    shape.iter().map(|&(x, y)| Pt::new(at.x + x * dx, at.y + y)).collect()
}

/// A subcircuit: box, name and ports.
pub fn subcircuit_prims(s: &Subcircuit) -> Vec<Prim> {
    let r = s.rect();
    let mut out =
        vec![Prim::new(Shape::Polyline { points: r.corners().to_vec(), closed: true }, Ink::Subcircuit).width(BODY_WIDTH).fill(Some(Ink::BodyFill))];
    if !s.name.trim().is_empty() {
        let at = Pt::new(r.min.x, r.max.y + NAME_LIFT);
        out.push(Prim::new(
            Shape::Text { at, text: s.name.clone(), size: REFERENCE_SIZE, angle: 0.0, h: HAlign::Left, v: VAlign::Bottom },
            Ink::Subcircuit,
        ));
    }
    for port in s.ports.iter().take(MAX_DRAWN) {
        let (dx, h) = match s.side_of(port.at) {
            PortSide::Left => (1.0, HAlign::Left),
            PortSide::Right => (-1.0, HAlign::Right),
        };
        out.push(Prim::new(Shape::Polyline { points: port_outline(port.kind, port.at, dx), closed: true }, Ink::Port).width(BODY_WIDTH));
        if !port.name.is_empty() {
            let at = Pt::new(port.at.x + (PORT_LENGTH + PORT_GAP) * dx, port.at.y);
            out.push(Prim::new(Shape::Text { at, text: port.name.clone(), size: LABEL_SIZE, angle: 0.0, h, v: VAlign::Middle }, Ink::Port));
        }
    }
    out.into_iter().map(|p| p.owned(Some(ItemId::Subcircuit(s.id)))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use protasena_doc::{Port, SubcircuitId};

    #[test]
    fn box_name_and_ports_point_the_right_way() {
        let mut s = Subcircuit::new(SubcircuitId(3), Pt::new(0.0, 0.0), Pt::new(1000.0, 600.0), "SUB1");
        s.ports.push(Port { at: Pt::new(0.0, 300.0), name: "IN".into(), kind: PortKind::Input });
        s.ports.push(Port { at: Pt::new(1000.0, 300.0), name: "OUT".into(), kind: PortKind::Output });
        let prims = subcircuit_prims(&s);
        assert!(prims.iter().all(|p| p.owner == Some(ItemId::Subcircuit(SubcircuitId(3)))));
        assert_eq!(prims.iter().filter(|p| p.ink == Ink::Port).count(), 4);
        let outlines: Vec<&Vec<Pt>> = prims
            .iter()
            .filter(|p| p.ink == Ink::Port)
            .filter_map(|p| if let Shape::Polyline { points, .. } = &p.shape { Some(points) } else { None })
            .collect();
        assert!(outlines[0].iter().all(|p| p.x >= 0.0 && p.x <= PORT_LENGTH), "the input sits inside the left edge");
        assert!(outlines[1].iter().all(|p| p.x <= 1000.0 && p.x >= 1000.0 - PORT_LENGTH), "the output inside the right edge");
        let name = prims.iter().find(|p| matches!(&p.shape, Shape::Text { text, .. } if text == "SUB1")).unwrap();
        assert!(matches!(name.shape, Shape::Text { at, .. } if at.y > 600.0), "the name is above the box");
    }
}
