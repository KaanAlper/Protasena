//! The conversion report: counts, what could not be mapped or linked, what was approximated,
//! and failures (which must be zero for a usable pack).

use std::collections::BTreeMap;

use serde::Serialize;

/// Named counters of things the conversion approximated or dropped, merged across threads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Notes(BTreeMap<String, u64>);

impl Notes {
    /// Adds `n` to the counter `key`.
    pub fn add(&mut self, key: &str, n: u64) {
        if n > 0 {
            *self.0.entry(key.to_owned()).or_default() += n;
        }
    }

    /// Adds one to the counter `key`.
    pub fn note(&mut self, key: &str) {
        self.add(key, 1);
    }

    /// Adds every counter of `other`.
    pub fn merge(&mut self, other: &Notes) {
        for (k, v) in &other.0 {
            self.add(k, *v);
        }
    }

    /// The value of a counter (0 when never noted).
    pub fn get(&self, key: &str) -> u64 {
        self.0.get(key).copied().unwrap_or(0)
    }
}

/// Totals of the run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    /// `.kicad_sym` files read.
    pub symbol_libraries: usize,
    /// Symbols in them (derived ones included).
    pub symbols: usize,
    /// Symbols written with `extends` (resolved against their parent).
    pub derived_symbols: usize,
    /// Devices written.
    pub devices: usize,
    /// Of which power symbols.
    pub power_devices: usize,
    /// Devices whose name comes from the map (Proteus-style names).
    pub mapped_names: usize,
    /// Devices renamed to keep names unique.
    pub disambiguated: usize,
    /// Units written (unit × body style).
    pub units: usize,
    /// Pins written.
    pub pins: usize,
    /// `.pretty` directories read.
    pub footprint_libraries: usize,
    /// Footprints in them.
    pub footprints: usize,
    /// Packages written.
    pub packages: usize,
    /// Pads written.
    pub pads: usize,
    /// Devices with at least one package.
    pub devices_with_packages: usize,
    /// Package links (device × package).
    pub package_links: usize,
    /// Links from the symbol's `Footprint` field.
    pub footprint_field_links: usize,
    /// Default packages named by the map (`default_package`).
    pub map_default_links: usize,
    /// Links found through `ki_fp_filters` (pad set equal to the pin set).
    pub filter_links: usize,
    /// Devices whose filter matches were cut at the per-device limit.
    pub filter_capped_devices: usize,
    /// Pin alternates (alternate pin functions) carried over.
    pub pin_alternates: u64,
    /// KiCad tokens the reader kept as unknown.
    pub kicad_unknown_tokens: u64,
}

/// A device whose name differs from its KiCad symbol name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Renamed {
    /// KiCad library.
    pub lib: String,
    /// KiCad symbol.
    pub symbol: String,
    /// Device name in the pack.
    pub name: String,
}

/// A `Footprint` field that names a footprint the footprint libraries do not have.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct MissingFootprint {
    /// KiCad library of the symbol.
    pub lib: String,
    /// KiCad symbol.
    pub symbol: String,
    /// The field's value.
    pub footprint: String,
}

/// Something that could not be converted at all.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Failure {
    /// File (relative to its input directory) or `library/item`.
    pub what: String,
    /// Why.
    pub error: String,
}

/// The whole report, written as JSON by `--report`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Report {
    /// Upstream description given on the command line (`--upstream`), e.g. the KiCad tag.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub upstream: String,
    /// Totals.
    pub counts: Counts,
    /// Symbol libraries without a `[libraries]` entry in the map (category derived from the
    /// library name).
    pub unmapped_libraries: Vec<String>,
    /// Map entries (`[devices]`) that name a symbol the libraries do not have.
    pub unused_map_devices: Vec<String>,
    /// Devices with a Proteus-style name from the map, or renamed to stay unique.
    pub renamed: Vec<Renamed>,
    /// `Footprint` fields naming missing footprints.
    pub footprint_links_not_found: Vec<MissingFootprint>,
    /// Approximations and dropped details, by kind.
    pub notes: Notes,
    /// KiCad tokens the reader did not know, by token.
    pub kicad_unknown: BTreeMap<String, usize>,
    /// Conversion failures (must be empty).
    pub failures: Vec<Failure>,
}

impl Report {
    /// The report as pretty JSON.
    ///
    /// # Errors
    /// Only if serialisation fails (it does not for this type).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_merge_and_skip_zero() {
        let mut a = Notes::default();
        a.note("x");
        a.add("y", 0);
        let mut b = Notes::default();
        b.add("x", 2);
        a.merge(&b);
        assert_eq!(a.get("x"), 3);
        assert_eq!(serde_json::to_string(&a).unwrap(), r#"{"x":3}"#);
    }
}
