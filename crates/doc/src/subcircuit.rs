//! Subcircuits: a box on a sheet standing for a child sheet (Proteus's Subcircuit mode), with
//! ports on its left and right edges that connect to the child sheet's input, output and
//! bidirectional terminals of the same name.

use protasena_geom::{Pt, Rect};

use crate::error::DocError;
use crate::ids::{SheetId, SubcircuitId};

/// Most ports on one subcircuit.
pub const MAX_PORTS: usize = 1024;

/// The kinds of subcircuit port (the PORTS list of Subcircuit mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "snake_case"))]
pub enum PortKind {
    /// Plain port.
    #[default]
    Default,
    /// Input port.
    Input,
    /// Output port.
    Output,
    /// Bidirectional port.
    Bidir,
}

/// One port of a subcircuit: a connection point on the box's left or right edge.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Port {
    /// Connection point (on a vertical edge of the box).
    pub at: Pt,
    /// Name: joins the child sheet's terminal of the same name.
    pub name: String,
    /// Kind.
    #[cfg_attr(feature = "serde", serde(default))]
    pub kind: PortKind,
}

/// Which edge of the box a port sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortSide {
    /// The left edge (the port draws towards the right, into the box).
    Left,
    /// The right edge.
    Right,
}

/// A subcircuit block.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Subcircuit {
    /// Stable id.
    pub id: SubcircuitId,
    /// One corner of the box.
    pub a: Pt,
    /// The opposite corner.
    pub b: Pt,
    /// Instance name (`SUB1`): the prefix of the child sheet's local net names.
    #[cfg_attr(feature = "serde", serde(default))]
    pub name: String,
    /// The child sheet, once one is made (Goto Child Sheet creates it).
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub child: Option<SheetId>,
    /// Ports, in the order they were added.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Vec::is_empty"))]
    pub ports: Vec<Port>,
}

impl Subcircuit {
    /// A subcircuit with no ports and no child sheet.
    pub fn new(id: SubcircuitId, a: Pt, b: Pt, name: &str) -> Self {
        Self { id, a, b, name: name.to_owned(), child: None, ports: Vec::new() }
    }

    /// The box, normalized.
    pub fn rect(&self) -> Rect {
        Rect::new(self.a, self.b).normalized()
    }

    /// Every point: the corners and the ports.
    pub fn points(&self) -> impl Iterator<Item = Pt> + '_ {
        [self.a, self.b].into_iter().chain(self.ports.iter().map(|p| p.at))
    }

    /// The same subcircuit with every point mapped by `f` (a rigid map: the box stays a box).
    pub fn map(&self, f: impl Fn(Pt) -> Pt) -> Subcircuit {
        let ports = self.ports.iter().map(|p| Port { at: f(p.at), ..p.clone() }).collect();
        Subcircuit { a: f(self.a), b: f(self.b), ports, ..self.clone() }
    }

    /// The edge a port at `at` belongs to: the nearer of the two vertical edges.
    pub fn side_of(&self, at: Pt) -> PortSide {
        let r = self.rect();
        if (at.x - r.min.x).abs() <= (at.x - r.max.x).abs() { PortSide::Left } else { PortSide::Right }
    }

    /// The point on a vertical edge nearest to `at`, if `at` lies within `reach` of one and
    /// strictly between the top and bottom corners.
    pub fn edge_point(&self, at: Pt, reach: f64) -> Option<Pt> {
        let r = self.rect();
        if !(at.is_finite() && at.y > r.min.y && at.y < r.max.y) {
            return None;
        }
        let x = match self.side_of(at) {
            PortSide::Left => r.min.x,
            PortSide::Right => r.max.x,
        };
        ((at.x - x).abs() <= reach).then_some(Pt::new(x, at.y))
    }

    /// Checks coordinates (finite), the box (a width and a height) and the port count.
    ///
    /// # Errors
    /// [`DocError::NonFinite`] or [`DocError::Invalid`].
    pub fn check(&self) -> Result<(), DocError> {
        if !self.points().all(|p| p.is_finite()) {
            return Err(DocError::NonFinite("subcircuit point"));
        }
        let r = self.rect();
        if !(r.width() > 0.0 && r.height() > 0.0) {
            return Err(DocError::Invalid(format!("{}: a subcircuit needs a width and a height", self.id)));
        }
        if self.ports.len() > MAX_PORTS {
            return Err(DocError::Invalid(format!("{}: more than {MAX_PORTS} ports", self.id)));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub() -> Subcircuit {
        Subcircuit::new(SubcircuitId(1), Pt::new(400.0, 300.0), Pt::new(-400.0, -300.0), "SUB1")
    }

    #[test]
    fn ports_go_on_the_nearer_vertical_edge() {
        let s = sub();
        assert_eq!(s.edge_point(Pt::new(-390.0, 100.0), 20.0), Some(Pt::new(-400.0, 100.0)));
        assert_eq!(s.side_of(Pt::new(-390.0, 100.0)), PortSide::Left);
        assert_eq!(s.edge_point(Pt::new(410.0, 0.0), 20.0), Some(Pt::new(400.0, 0.0)));
        assert_eq!(s.edge_point(Pt::new(0.0, 0.0), 20.0), None, "the middle is no edge");
        assert_eq!(s.edge_point(Pt::new(400.0, 300.0), 20.0), None, "nor a corner");
        assert_eq!(s.edge_point(Pt::new(f64::NAN, 0.0), 20.0), None);
    }

    #[test]
    fn checks_the_box_and_the_points() {
        assert!(sub().check().is_ok());
        let flat = Subcircuit::new(SubcircuitId(1), Pt::ORIGIN, Pt::new(100.0, 0.0), "S");
        assert!(matches!(flat.check(), Err(DocError::Invalid(_))));
        let mut bad = sub();
        bad.ports.push(Port { at: Pt::new(f64::INFINITY, 0.0), name: "A".into(), kind: PortKind::Input });
        assert_eq!(bad.check(), Err(DocError::NonFinite("subcircuit point")));
        let moved = sub().map(|p| Pt::new(p.x + 10.0, p.y));
        assert_eq!(moved.rect().min, Pt::new(-390.0, -300.0));
    }
}
