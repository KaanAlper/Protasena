//! One resolved KiCad symbol → one Protasena device (name assigned later, see
//! [`crate::names`]).

use std::collections::HashSet;

use protasena_kicad::symbol::Symbol as KSymbol;
use protasena_library::{Device, PropDef, PropKind, Source};

use crate::link::{FootprintIndex, Links};
use crate::map::Map;
use crate::report::Notes;
use crate::symbol;

/// Most keywords a device keeps.
pub const MAX_KEYWORDS: usize = 64;

/// SPDX tag of KiCad's library licence.
pub const LICENCE: &str = "CC-BY-SA-4.0";

/// A converted device before its pack-wide unique name is known.
#[derive(Debug, Clone)]
pub struct Draft {
    /// KiCad library.
    pub lib: String,
    /// KiCad symbol name.
    pub symbol: String,
    /// Name from the map, if any.
    pub mapped_name: Option<String>,
    /// The device (its `name` is still the KiCad symbol name).
    pub device: Device,
    /// How the packages were found (its `packages` list is moved into the device).
    pub links: Links,
}

/// Converts a resolved symbol of library `lib`.
pub fn build(lib: &str, s: &KSymbol, map: &Map, fps: &FootprintIndex, notes: &mut Notes) -> Draft {
    let entry = map.device(lib, &s.name);
    let placement = map.place(lib, &s.name);
    let value = s.value_of("Value").unwrap_or_default();
    let symbol = symbol::convert(s, notes);
    let pins = pin_numbers(&symbol);
    let mut links = fps.link(s.value_of("Footprint").unwrap_or_default(), s.value_of("ki_fp_filters").unwrap_or_default(), &pins);
    let mut prop_defs = vec![PropDef::new("VALUE", if s.power { "Net" } else { "Value" }, PropKind::String, Some(value))];
    let packages = std::mem::take(&mut links.packages);
    let default_package = links.default.clone().or_else(|| packages.first().map(|p| p.name.clone()));
    if let Some(def) = &default_package {
        prop_defs.push(PropDef::new("PACKAGE", "PCB Package", PropKind::Package, Some(def)).hidden());
    }
    if let Some(primitive) = entry.and_then(|e| e.primitive.as_deref()) {
        prop_defs.push(PropDef::new("PRIMITIVE", "Primitive", PropKind::String, Some(primitive)).hidden());
    }
    let device = Device {
        name: s.name.clone(),
        library: lib.to_owned(),
        prefix: prefix(s),
        description: description(s),
        category: placement.category,
        sub_category: placement.sub_category,
        manufacturer: map.manufacturer(lib),
        stock_code: String::new(),
        keywords: keywords(s, notes),
        symbol,
        packages,
        default_package,
        prop_defs,
        sim_model: None,
        source: Some(Source { upstream: "kicad".into(), lib: lib.to_owned(), symbol: s.name.clone(), licence: LICENCE.into() }),
    };
    Draft { lib: lib.to_owned(), symbol: s.name.clone(), mapped_name: entry.and_then(|e| e.name.clone()), device, links }
}

/// The annotation prefix from `Reference`; empty for power symbols and other parts KiCad does
/// not annotate (references starting with `#`).
pub fn prefix(s: &KSymbol) -> String {
    let reference = s.value_of("Reference").unwrap_or_default().trim();
    if s.power || reference.starts_with('#') { String::new() } else { reference.to_owned() }
}

/// `Description` (KiCad 8+), else `ki_description` (KiCad 6–7).
pub fn description(s: &KSymbol) -> String {
    [s.value_of("Description"), s.value_of("ki_description")]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|d| !d.is_empty())
        .unwrap_or_default()
        .to_owned()
}

/// `ki_keywords` (or `Keywords`) split at white space, without repeats, at most
/// [`MAX_KEYWORDS`].
pub fn keywords(s: &KSymbol, notes: &mut Notes) -> Vec<String> {
    let text = s.value_of("ki_keywords").or_else(|| s.value_of("Keywords")).unwrap_or_default();
    let mut seen = HashSet::new();
    let words: Vec<String> = text.split_whitespace().filter(|w| seen.insert(w.to_lowercase())).map(str::to_owned).collect();
    if words.len() > MAX_KEYWORDS {
        notes.note("keyword lists cut to 64 words");
    }
    words.into_iter().take(MAX_KEYWORDS).collect()
}

/// Distinct pin numbers in order of first appearance.
fn pin_numbers(s: &protasena_library::Symbol) -> Vec<String> {
    let mut seen = HashSet::new();
    s.pins().filter(|p| seen.insert(p.number.as_str())).map(|p| p.number.clone()).collect()
}
