//! The commands, by area. Ids are stable and lower-case; labels, menu paths and shortcuts are
//! Proteus's.

mod block;
mod clip;
mod design;
mod edit;
mod file;
mod items;
mod place;
mod sheets;
mod view;
mod wire;

pub(crate) use block::drag_ops;
pub(crate) use place::new_component;
pub(crate) use wire::MAX_VIA;

use protasena_doc::{Id, Item, ItemId};
use serde_json::Value;

use crate::args::id_json;
use crate::registry::CommandSpec;

/// Every command, menu area by menu area.
pub(crate) fn all_specs() -> Vec<CommandSpec> {
    let mut v = Vec::new();
    v.extend(file::specs());
    v.extend(edit::specs());
    v.extend(clip::specs());
    v.extend(block::specs());
    v.extend(view::specs());
    v.extend(place::specs());
    v.extend(items::specs());
    v.extend(wire::specs());
    v.extend(design::specs());
    v.extend(sheets::specs());
    v
}

/// JSON list of item ids (raw numbers).
pub(crate) fn ids_json(ids: &[ItemId]) -> Value {
    Value::Array(ids.iter().map(|&id| id_json(id)).collect())
}

/// `item` with the raw id `raw` (of its own kind).
pub(crate) fn with_id(item: &Item, raw: u64) -> Item {
    match item {
        Item::Component(c) => Item::Component(protasena_doc::Component { id: Id::from_raw(raw), ..c.clone() }),
        Item::Wire(w) => Item::Wire(protasena_doc::Wire { id: Id::from_raw(raw), ..w.clone() }),
        Item::Junction(j) => Item::Junction(protasena_doc::Junction { id: Id::from_raw(raw), ..*j }),
        Item::Label(l) => Item::Label(protasena_doc::Label { id: Id::from_raw(raw), ..l.clone() }),
        Item::Terminal(t) => Item::Terminal(protasena_doc::Terminal { id: Id::from_raw(raw), ..t.clone() }),
        Item::Bus(b) => Item::Bus(protasena_doc::Bus { id: Id::from_raw(raw), ..b.clone() }),
        Item::BusEntry(e) => Item::BusEntry(protasena_doc::BusEntry { id: Id::from_raw(raw), ..*e }),
        Item::Text(t) => Item::Text(protasena_doc::Text { id: Id::from_raw(raw), ..t.clone() }),
    }
}
