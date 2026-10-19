//! Stable typed ids.
//!
//! Every sheet and every item on a sheet has an id drawn from one project-wide counter
//! ([`Project::fresh_id`](crate::Project::fresh_id)), so a raw number is never used twice in a
//! project, whatever the kind. Ids survive edits, undo and redo, and saving and loading.

use std::fmt;

/// A typed id: a thin wrapper around the raw project-wide number.
pub trait Id: Copy + Eq + fmt::Debug {
    /// Wraps a raw number.
    fn from_raw(raw: u64) -> Self;
    /// The raw number.
    fn raw(self) -> u64;
}

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident, $tag:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(transparent))]
        pub struct $name(pub u64);

        impl Id for $name {
            fn from_raw(raw: u64) -> Self {
                Self(raw)
            }
            fn raw(self) -> u64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!($tag, "#{}"), self.0)
            }
        }
    };
}

id_type!(/// Id of a [`Sheet`](crate::Sheet).
    SheetId, "sheet");
id_type!(/// Id of a [`Component`](crate::Component).
    ComponentId, "component");
id_type!(/// Id of a [`Wire`](crate::Wire).
    WireId, "wire");
id_type!(/// Id of a [`Junction`](crate::Junction).
    JunctionId, "junction");
id_type!(/// Id of a [`Label`](crate::Label).
    LabelId, "label");
id_type!(/// Id of a [`Terminal`](crate::Terminal).
    TerminalId, "terminal");
id_type!(/// Id of a [`Bus`](crate::Bus).
    BusId, "bus");
id_type!(/// Id of a [`BusEntry`](crate::BusEntry).
    BusEntryId, "bus entry");
id_type!(/// Id of a [`Text`](crate::Text).
    TextId, "text");

/// The id of any item that lives on a sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "snake_case"))]
pub enum ItemId {
    /// A component.
    Component(ComponentId),
    /// A wire.
    Wire(WireId),
    /// A junction dot.
    Junction(JunctionId),
    /// A net label.
    Label(LabelId),
    /// A terminal.
    Terminal(TerminalId),
    /// A bus.
    Bus(BusId),
    /// A bus entry.
    BusEntry(BusEntryId),
    /// A free text.
    Text(TextId),
}

impl ItemId {
    /// The raw project-wide number.
    pub fn raw(self) -> u64 {
        match self {
            Self::Component(i) => i.0,
            Self::Wire(i) => i.0,
            Self::Junction(i) => i.0,
            Self::Label(i) => i.0,
            Self::Terminal(i) => i.0,
            Self::Bus(i) => i.0,
            Self::BusEntry(i) => i.0,
            Self::Text(i) => i.0,
        }
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Component(i) => i.fmt(f),
            Self::Wire(i) => i.fmt(f),
            Self::Junction(i) => i.fmt(f),
            Self::Label(i) => i.fmt(f),
            Self::Terminal(i) => i.fmt(f),
            Self::Bus(i) => i.fmt(f),
            Self::BusEntry(i) => i.fmt(f),
            Self::Text(i) => i.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_display_kind_and_number() {
        assert_eq!(ComponentId(7).to_string(), "component#7");
        assert_eq!(ItemId::BusEntry(BusEntryId(3)).to_string(), "bus entry#3");
        assert_eq!(ItemId::Wire(WireId(9)).raw(), 9);
        assert_eq!(<SheetId as Id>::from_raw(4).raw(), 4);
    }
}
