//! Sheets and sheet sizes, and the per-kind storage behind the item operations.

use crate::error::DocError;
use crate::graphic::Graphic;
use crate::ids::{ItemId, SheetId};
use crate::item::Item;
use crate::items::{Bus, BusEntry, Component, Junction, Label, Terminal, Text, Wire};
use crate::subcircuit::Subcircuit;

/// Sheet size. Named sizes are landscape; dimensions are in thou.
///
/// ISO sizes are the paper sizes rounded to whole thou; ANSI sizes are exact inches. Proteus's
/// own list of sizes and their dimensions still has to be observed.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "snake_case"))]
pub enum SheetSize {
    /// ISO A4, 297 × 210 mm.
    #[default]
    A4,
    /// ISO A3, 420 × 297 mm.
    A3,
    /// ISO A2, 594 × 420 mm.
    A2,
    /// ISO A1, 841 × 594 mm.
    A1,
    /// ISO A0, 1189 × 841 mm.
    A0,
    /// ANSI A, 11 × 8.5 in.
    A,
    /// ANSI B, 17 × 11 in.
    B,
    /// ANSI C, 22 × 17 in.
    C,
    /// ANSI D, 34 × 22 in.
    D,
    /// ANSI E, 44 × 34 in.
    E,
    /// Any size, in thou.
    Custom {
        /// Width in thou.
        width: f64,
        /// Height in thou.
        height: f64,
    },
}

impl SheetSize {
    /// `(width, height)` in thou.
    pub fn dimensions(self) -> (f64, f64) {
        match self {
            Self::A4 => (11_693.0, 8_268.0),
            Self::A3 => (16_535.0, 11_693.0),
            Self::A2 => (23_386.0, 16_535.0),
            Self::A1 => (33_110.0, 23_386.0),
            Self::A0 => (46_811.0, 33_110.0),
            Self::A => (11_000.0, 8_500.0),
            Self::B => (17_000.0, 11_000.0),
            Self::C => (22_000.0, 17_000.0),
            Self::D => (34_000.0, 22_000.0),
            Self::E => (44_000.0, 34_000.0),
            Self::Custom { width, height } => (width, height),
        }
    }

    /// Checks that a custom size is finite and positive.
    ///
    /// # Errors
    /// [`DocError::NonFinite`] for NaN/∞, [`DocError::Invalid`] for a size ≤ 0.
    pub fn check(self) -> Result<(), DocError> {
        let (w, h) = self.dimensions();
        if !(w.is_finite() && h.is_finite()) {
            Err(DocError::NonFinite("sheet size"))
        } else if w <= 0.0 || h <= 0.0 {
            Err(DocError::Invalid(format!("sheet size {w} × {h} is not positive")))
        } else {
            Ok(())
        }
    }
}

/// One sheet of a design.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct Sheet {
    /// Stable id.
    pub id: SheetId,
    /// Sheet name.
    pub name: String,
    /// Sheet size.
    pub size: SheetSize,
    /// Placed components.
    pub components: Vec<Component>,
    /// Wires.
    pub wires: Vec<Wire>,
    /// Junction dots.
    pub junctions: Vec<Junction>,
    /// Net labels.
    pub labels: Vec<Label>,
    /// Buses.
    pub buses: Vec<Bus>,
    /// Bus entries.
    pub bus_entries: Vec<BusEntry>,
    /// Terminals.
    pub terminals: Vec<Terminal>,
    /// Free texts.
    pub texts: Vec<Text>,
    /// 2D graphics.
    pub graphics: Vec<Graphic>,
    /// Subcircuits.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty"))]
    pub subcircuits: Vec<Subcircuit>,
}

/// Applies `$body` to the vector of the kind of `$id` (an [`ItemId`]), with `$v` bound to the
/// vector; `$wrap` turns an element into an [`Item`].
macro_rules! by_kind {
    ($sheet:expr, $id:expr, |$v:ident, $wrap:ident| $body:expr) => {
        match $id {
            ItemId::Component(_) => {
                let $v = &mut $sheet.components;
                let $wrap = Item::Component;
                $body
            }
            ItemId::Wire(_) => {
                let $v = &mut $sheet.wires;
                let $wrap = Item::Wire;
                $body
            }
            ItemId::Junction(_) => {
                let $v = &mut $sheet.junctions;
                let $wrap = Item::Junction;
                $body
            }
            ItemId::Label(_) => {
                let $v = &mut $sheet.labels;
                let $wrap = Item::Label;
                $body
            }
            ItemId::Terminal(_) => {
                let $v = &mut $sheet.terminals;
                let $wrap = Item::Terminal;
                $body
            }
            ItemId::Bus(_) => {
                let $v = &mut $sheet.buses;
                let $wrap = Item::Bus;
                $body
            }
            ItemId::BusEntry(_) => {
                let $v = &mut $sheet.bus_entries;
                let $wrap = Item::BusEntry;
                $body
            }
            ItemId::Text(_) => {
                let $v = &mut $sheet.texts;
                let $wrap = Item::Text;
                $body
            }
            ItemId::Graphic(_) => {
                let $v = &mut $sheet.graphics;
                let $wrap = Item::Graphic;
                $body
            }
            ItemId::Subcircuit(_) => {
                let $v = &mut $sheet.subcircuits;
                let $wrap = Item::Subcircuit;
                $body
            }
        }
    };
}

impl Sheet {
    /// An empty sheet.
    pub fn new(id: SheetId, name: &str, size: SheetSize) -> Self {
        Self { id, name: name.to_owned(), size, ..Self::default() }
    }

    /// The position of an item in its kind's list.
    pub fn index_of(&self, id: ItemId) -> Option<usize> {
        let raw = id.raw();
        fn pos(mut ids: impl Iterator<Item = u64>, raw: u64) -> Option<usize> {
            ids.position(|r| r == raw)
        }
        match id {
            ItemId::Component(_) => pos(self.components.iter().map(|c| c.id.0), raw),
            ItemId::Wire(_) => pos(self.wires.iter().map(|w| w.id.0), raw),
            ItemId::Junction(_) => pos(self.junctions.iter().map(|j| j.id.0), raw),
            ItemId::Label(_) => pos(self.labels.iter().map(|l| l.id.0), raw),
            ItemId::Terminal(_) => pos(self.terminals.iter().map(|t| t.id.0), raw),
            ItemId::Bus(_) => pos(self.buses.iter().map(|b| b.id.0), raw),
            ItemId::BusEntry(_) => pos(self.bus_entries.iter().map(|e| e.id.0), raw),
            ItemId::Text(_) => pos(self.texts.iter().map(|t| t.id.0), raw),
            ItemId::Graphic(_) => pos(self.graphics.iter().map(|g| g.id.0), raw),
            ItemId::Subcircuit(_) => pos(self.subcircuits.iter().map(|x| x.id.0), raw),
        }
    }

    /// A copy of the item with this id.
    pub fn item(&self, id: ItemId) -> Option<Item> {
        let i = self.index_of(id)?;
        match id {
            ItemId::Component(_) => self.components.get(i).cloned().map(Item::Component),
            ItemId::Wire(_) => self.wires.get(i).cloned().map(Item::Wire),
            ItemId::Junction(_) => self.junctions.get(i).copied().map(Item::Junction),
            ItemId::Label(_) => self.labels.get(i).cloned().map(Item::Label),
            ItemId::Terminal(_) => self.terminals.get(i).cloned().map(Item::Terminal),
            ItemId::Bus(_) => self.buses.get(i).cloned().map(Item::Bus),
            ItemId::BusEntry(_) => self.bus_entries.get(i).copied().map(Item::BusEntry),
            ItemId::Text(_) => self.texts.get(i).cloned().map(Item::Text),
            ItemId::Graphic(_) => self.graphics.get(i).cloned().map(Item::Graphic),
            ItemId::Subcircuit(_) => self.subcircuits.get(i).cloned().map(Item::Subcircuit),
        }
    }

    /// Every item id on the sheet, kind by kind in list order.
    pub fn item_ids(&self) -> impl Iterator<Item = ItemId> + '_ {
        let c = self.components.iter().map(|x| ItemId::Component(x.id));
        let w = self.wires.iter().map(|x| ItemId::Wire(x.id));
        let j = self.junctions.iter().map(|x| ItemId::Junction(x.id));
        let l = self.labels.iter().map(|x| ItemId::Label(x.id));
        let t = self.terminals.iter().map(|x| ItemId::Terminal(x.id));
        let b = self.buses.iter().map(|x| ItemId::Bus(x.id));
        let e = self.bus_entries.iter().map(|x| ItemId::BusEntry(x.id));
        let x = self.texts.iter().map(|x| ItemId::Text(x.id));
        let g = self.graphics.iter().map(|x| ItemId::Graphic(x.id));
        let s = self.subcircuits.iter().map(|x| ItemId::Subcircuit(x.id));
        c.chain(w).chain(j).chain(l).chain(t).chain(b).chain(e).chain(x).chain(g).chain(s)
    }

    /// Inserts an item at `index` in its kind's list (clamped; `None` appends).
    pub(crate) fn insert(&mut self, item: Item, index: Option<usize>) {
        fn put<T>(v: &mut Vec<T>, x: T, index: Option<usize>) {
            let at = index.map_or(v.len(), |i| i.min(v.len()));
            v.insert(at, x);
        }
        match item {
            Item::Component(x) => put(&mut self.components, x, index),
            Item::Wire(x) => put(&mut self.wires, x, index),
            Item::Junction(x) => put(&mut self.junctions, x, index),
            Item::Label(x) => put(&mut self.labels, x, index),
            Item::Terminal(x) => put(&mut self.terminals, x, index),
            Item::Bus(x) => put(&mut self.buses, x, index),
            Item::BusEntry(x) => put(&mut self.bus_entries, x, index),
            Item::Text(x) => put(&mut self.texts, x, index),
            Item::Graphic(x) => put(&mut self.graphics, x, index),
            Item::Subcircuit(x) => put(&mut self.subcircuits, x, index),
        }
    }

    /// Removes an item; returns it with the index it had.
    pub(crate) fn remove(&mut self, id: ItemId) -> Option<(Item, usize)> {
        let i = self.index_of(id)?;
        by_kind!(self, id, |v, wrap| (i < v.len()).then(|| (wrap(v.remove(i)), i)))
    }

    /// Replaces the item with the same id; returns the old one.
    pub(crate) fn replace(&mut self, item: Item) -> Option<Item> {
        let (old, i) = self.remove(item.id())?;
        self.insert(item, Some(i));
        Some(old)
    }
}
