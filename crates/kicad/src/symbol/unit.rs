//! Symbol units: the child `(symbol "NAME_UNIT_STYLE" …)` tokens.

use crate::decode::{invalid, text, text1};
use crate::error::Result;
use crate::report::{Inspect, Report, Unknown};
use crate::sexpr::Sexpr;

use super::{Graphic, Pin};

/// One unit / body style of a symbol, with its drawing and pins.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Unit {
    /// The unit identifier as written, `"NAME_UNIT_STYLE"`.
    pub name: String,
    /// Unit number; 0 means shared by every unit.
    pub unit: u32,
    /// Body style: 1 normal, 2 alternate (De Morgan); 0 means shared by every style.
    pub style: u32,
    /// `(unit_name "…")`: display name of the unit.
    pub unit_name: Option<String>,
    pub graphics: Vec<Graphic>,
    pub pins: Vec<Pin>,
    pub unknown: Vec<Unknown>,
}

impl Unit {
    /// The unit that holds graphics and pins written directly in the parent symbol.
    pub(crate) fn shared(symbol: &str) -> Self {
        Self { name: format!("{symbol}_0_0"), ..Self::default() }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.graphics.is_empty() && self.pins.is_empty() && self.unknown.is_empty() && self.unit_name.is_none()
    }

    /// Takes `item` when it is a graphic item or a pin; false when it is neither.
    pub(crate) fn take_item(&mut self, item: &Sexpr) -> Result<bool> {
        if item.head() == Some("pin") {
            self.pins.push(Pin::decode(item)?);
            return Ok(true);
        }
        match Graphic::decode(item)? {
            Some(g) => {
                self.graphics.push(g);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        let name = text(node, 0, "unit identifier")?;
        let (unit, style) = split_unit_id(&name).ok_or_else(|| invalid(node, format!("'{name}' is not a NAME_UNIT_STYLE unit identifier")))?;
        let mut u = Self { name, unit, style, ..Self::default() };
        for item in node.args().iter().skip(1) {
            if item.head() == Some("unit_name") {
                u.unit_name = Some(text1(item)?);
            } else if !u.take_item(item)? {
                u.unknown.push(Unknown::new("unit", item));
            }
        }
        Ok(u)
    }

    /// The same unit renamed for a derived symbol (`PARENT_1_1` → `CHILD_1_1`).
    pub(crate) fn renamed(&self, symbol: &str) -> Self {
        Self { name: format!("{symbol}_{}_{}", self.unit, self.style), ..self.clone() }
    }
}

/// `"NAME_UNIT_STYLE"` → (UNIT, STYLE); NAME may itself contain underscores.
fn split_unit_id(id: &str) -> Option<(u32, u32)> {
    let mut parts = id.rsplitn(3, '_');
    let style = parts.next()?.parse().ok()?;
    let unit = parts.next()?.parse().ok()?;
    parts.next()?;
    Some((unit, style))
}

impl Inspect for Unit {
    fn visit(&self, r: &mut Report) {
        self.graphics.visit(r);
        self.pins.visit(r);
        self.unknown.visit(r);
    }
}

#[cfg(test)]
mod tests {
    use super::split_unit_id;

    #[test]
    fn splits_unit_identifiers() {
        assert_eq!(split_unit_id("R_0_1"), Some((0, 1)));
        assert_eq!(split_unit_id("74LS00_A_B_3_2"), Some((3, 2)));
        assert_eq!(split_unit_id("R_x_1"), None);
        assert_eq!(split_unit_id("1_1"), None);
        assert_eq!(split_unit_id("R_1"), None);
    }
}
