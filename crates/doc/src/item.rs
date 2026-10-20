//! [`Item`]: any one thing on a sheet, as operations carry it.

use protasena_geom::{Pt, Vec2};

use crate::error::DocError;
use crate::graphic::Graphic;
use crate::ids::ItemId;
use crate::items::{Bus, BusEntry, Component, Junction, Label, Terminal, Text, Wire};

/// One item of any kind, owned (what [`Op::Add`](crate::Op::Add) and
/// [`Op::Modify`](crate::Op::Modify) carry).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "snake_case"))]
pub enum Item {
    /// A component.
    Component(Component),
    /// A wire.
    Wire(Wire),
    /// A junction dot.
    Junction(Junction),
    /// A net label.
    Label(Label),
    /// A terminal.
    Terminal(Terminal),
    /// A bus.
    Bus(Bus),
    /// A bus entry.
    BusEntry(BusEntry),
    /// A free text.
    Text(Text),
    /// A 2D graphic.
    Graphic(Graphic),
}

impl Item {
    /// The item's id.
    pub fn id(&self) -> ItemId {
        match self {
            Self::Component(c) => ItemId::Component(c.id),
            Self::Wire(w) => ItemId::Wire(w.id),
            Self::Junction(j) => ItemId::Junction(j.id),
            Self::Label(l) => ItemId::Label(l.id),
            Self::Terminal(t) => ItemId::Terminal(t.id),
            Self::Bus(b) => ItemId::Bus(b.id),
            Self::BusEntry(e) => ItemId::BusEntry(e.id),
            Self::Text(t) => ItemId::Text(t.id),
            Self::Graphic(g) => ItemId::Graphic(g.id),
        }
    }

    /// Checks that every coordinate is finite and polylines have at least two points.
    ///
    /// # Errors
    /// [`DocError::NonFinite`] or [`DocError::Invalid`] naming the first problem.
    pub fn check(&self) -> Result<(), DocError> {
        match self {
            Self::Component(c) => {
                finite_vec(c.transform.offset, "component position")?;
                finite_vec(c.reference_label.offset, "reference label offset")?;
                finite_vec(c.value_label.offset, "value label offset")?;
                match c.properties.duplicate_name() {
                    Some(name) => Err(DocError::Invalid(format!("{}: property {name} is given twice", c.id))),
                    None => Ok(()),
                }
            }
            Self::Wire(w) => polyline(&w.points, "wire"),
            Self::Bus(b) => polyline(&b.points, "bus"),
            Self::Junction(j) => finite(j.at, "junction position"),
            Self::Label(l) => finite(l.at, "label position"),
            Self::Terminal(t) => finite(t.at, "terminal position"),
            Self::BusEntry(e) => finite(e.bus_end, "bus entry end").and(finite(e.wire_end, "bus entry end")),
            Self::Text(t) => {
                finite(t.at, "text position")?;
                if t.size.is_finite() && t.size >= 0.0 { Ok(()) } else { Err(DocError::NonFinite("text size")) }
            }
            Self::Graphic(g) => g.shape.check(),
        }
    }

    /// The same item moved by `by`; the result is [checked](Item::check).
    ///
    /// # Errors
    /// [`DocError::NonFinite`] when `by` or a moved coordinate is not finite.
    pub fn translated(&self, by: Vec2) -> Result<Item, DocError> {
        finite_vec(by, "move offset")?;
        let shift = |pts: &[Pt]| pts.iter().map(|&p| p + by).collect::<Vec<_>>();
        let moved = match self {
            Self::Component(c) => {
                let mut transform = c.transform;
                transform.offset += by;
                Self::Component(Component { transform, ..c.clone() })
            }
            Self::Wire(w) => Self::Wire(Wire { id: w.id, points: shift(&w.points) }),
            Self::Bus(b) => Self::Bus(Bus { id: b.id, points: shift(&b.points) }),
            Self::Junction(j) => Self::Junction(Junction { at: j.at + by, ..*j }),
            Self::Label(l) => Self::Label(Label { at: l.at + by, ..l.clone() }),
            Self::Terminal(t) => Self::Terminal(Terminal { at: t.at + by, ..t.clone() }),
            Self::BusEntry(e) => Self::BusEntry(BusEntry { bus_end: e.bus_end + by, wire_end: e.wire_end + by, ..*e }),
            Self::Text(t) => Self::Text(Text { at: t.at + by, ..t.clone() }),
            Self::Graphic(g) => Self::Graphic(Graphic { id: g.id, shape: g.shape.map(|p| p + by) }),
        };
        moved.check()?;
        Ok(moved)
    }
}

fn finite(p: Pt, what: &'static str) -> Result<(), DocError> {
    if p.is_finite() { Ok(()) } else { Err(DocError::NonFinite(what)) }
}

fn finite_vec(v: Vec2, what: &'static str) -> Result<(), DocError> {
    finite(v.to_pt(), what)
}

fn polyline(points: &[Pt], what: &'static str) -> Result<(), DocError> {
    if points.len() < 2 {
        return Err(DocError::Invalid(format!("a {what} needs at least two points, not {}", points.len())));
    }
    points.iter().try_for_each(|&p| finite(p, if what == "bus" { "bus point" } else { "wire point" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{JunctionId, WireId};

    fn wire(points: &[(f64, f64)]) -> Item {
        Item::Wire(Wire { id: WireId(1), points: points.iter().map(|&(x, y)| Pt::new(x, y)).collect() })
    }

    #[test]
    fn check_rejects_short_and_non_finite_polylines() {
        assert!(wire(&[(0.0, 0.0), (100.0, 0.0)]).check().is_ok());
        assert!(matches!(wire(&[(0.0, 0.0)]).check(), Err(DocError::Invalid(_))));
        assert_eq!(wire(&[(0.0, 0.0), (f64::NAN, 0.0)]).check(), Err(DocError::NonFinite("wire point")));
    }

    #[test]
    fn translate_moves_every_point_and_refuses_overflow() {
        let moved = wire(&[(0.0, 0.0), (100.0, 0.0)]).translated(Vec2::new(10.0, -5.0)).unwrap();
        assert_eq!(moved, wire(&[(10.0, -5.0), (110.0, -5.0)]));
        let j = Item::Junction(Junction { id: JunctionId(2), at: Pt::new(f64::MAX, 0.0) });
        assert_eq!(j.translated(Vec2::new(f64::MAX, 0.0)), Err(DocError::NonFinite("junction position")));
        assert_eq!(j.translated(Vec2::new(f64::NAN, 0.0)), Err(DocError::NonFinite("move offset")));
    }
}
