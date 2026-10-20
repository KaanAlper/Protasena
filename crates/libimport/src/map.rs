//! `map.toml`: our own table from KiCad libraries and symbols to Pick Devices names,
//! categories and manufacturers.
//!
//! ```toml
//! [manufacturers]            # a `_`-separated word of a library name → manufacturer
//! ST = "STMicroelectronics"
//!
//! [libraries]                # category per KiCad symbol library
//! Device = { category = "Miscellaneous", sub_category = "Generic devices" }
//!
//! [[rules]]                  # per-symbol overrides inside libraries (first match wins)
//! library = "74xx*"          # wildcard on the library name
//! symbol = "74LS*"           # wildcard on the symbol name
//! category = "TTL 74LS series"
//!
//! [devices]                  # single symbols: Proteus-style name, category, primitive
//! "Device:R" = { name = "RES", category = "Resistors", sub_category = "Generic", primitive = "ANALOG",
//!                default_package = "R_Axial_DIN0207_L6.3mm_D2.5mm_P10.16mm_Horizontal" }
//! "Device:Q_NPN" = { name = "NPN", default_package = "TO-92_Inline", pin_map = { E = "1", B = "2", C = "3" } }
//! ```
//!
//! `default_package` (a footprint name, or `Library:Name`) is linked first and becomes the
//! device's default package; `pin_map` (pin → pad) replaces the by-number pin map for it. The
//! full default-package rule is in [`crate::link`].

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::glob;

/// The parsed map.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Map {
    /// Library-name word → manufacturer.
    #[serde(default)]
    pub manufacturers: BTreeMap<String, String>,
    /// Library → category.
    #[serde(default)]
    pub libraries: BTreeMap<String, LibraryEntry>,
    /// Per-symbol rules, in order.
    #[serde(default)]
    pub rules: Vec<Rule>,
    /// `"Library:Symbol"` → device entry.
    #[serde(default)]
    pub devices: BTreeMap<String, DeviceEntry>,
}

/// Category of a library.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryEntry {
    /// Pick Devices category.
    pub category: String,
    /// Pick Devices sub-category.
    #[serde(default)]
    pub sub_category: String,
    /// Manufacturer of every part of the library (instead of the word lookup).
    #[serde(default)]
    pub manufacturer: Option<String>,
}

/// A per-symbol override.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// Wildcard on the library name.
    pub library: String,
    /// Wildcard on the symbol name.
    pub symbol: String,
    /// Category (kept from the library when absent).
    #[serde(default)]
    pub category: Option<String>,
    /// Sub-category (kept from the library when absent).
    #[serde(default)]
    pub sub_category: Option<String>,
}

/// One mapped symbol.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceEntry {
    /// Device name (Proteus style, e.g. `RES`).
    #[serde(default)]
    pub name: Option<String>,
    /// Category override.
    #[serde(default)]
    pub category: Option<String>,
    /// Sub-category override.
    #[serde(default)]
    pub sub_category: Option<String>,
    /// Value of the `PRIMITIVE` property (e.g. `ANALOG`).
    #[serde(default)]
    pub primitive: Option<String>,
    /// Default package: a footprint name or `Library:Name`.
    #[serde(default)]
    pub default_package: Option<String>,
    /// Pin → pad for the default package (by pin number when absent).
    #[serde(default)]
    pub pin_map: Option<BTreeMap<String, String>>,
}

/// Where a device's category came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// Category.
    pub category: String,
    /// Sub-category.
    pub sub_category: String,
    /// Whether the library has a `[libraries]` entry.
    pub library_mapped: bool,
}

impl Map {
    /// Parses map text.
    ///
    /// # Errors
    /// The TOML parser's message (with line and column) for invalid text or unknown keys.
    pub fn parse(text: &str) -> Result<Map, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }

    /// The device entry of `lib:symbol`.
    pub fn device(&self, lib: &str, symbol: &str) -> Option<&DeviceEntry> {
        self.devices.get(&format!("{lib}:{symbol}"))
    }

    /// Category and sub-category of a symbol: device entry, then the first matching rule, then
    /// the library entry, then a name derived from the library (`Sensor_Gas` → "Sensor" /
    /// "Gas").
    pub fn place(&self, lib: &str, symbol: &str) -> Placement {
        let entry = self.libraries.get(lib);
        let (mut category, mut sub_category) = match entry {
            Some(e) => (e.category.clone(), e.sub_category.clone()),
            None => derived(lib),
        };
        if let Some(rule) = self.rules.iter().find(|r| glob::matches(&r.library, lib, false) && glob::matches(&r.symbol, symbol, false)) {
            if let Some(c) = &rule.category {
                c.clone_into(&mut category);
            }
            if let Some(s) = &rule.sub_category {
                s.clone_into(&mut sub_category);
            }
        }
        if let Some(dev) = self.device(lib, symbol) {
            if let Some(c) = &dev.category {
                c.clone_into(&mut category);
            }
            if let Some(s) = &dev.sub_category {
                s.clone_into(&mut sub_category);
            }
        }
        Placement { category, sub_category, library_mapped: entry.is_some() }
    }

    /// Manufacturer of a library's parts: the library entry's, else the first word of the
    /// library name found in `[manufacturers]`, else empty.
    pub fn manufacturer(&self, lib: &str) -> String {
        if let Some(m) = self.libraries.get(lib).and_then(|e| e.manufacturer.clone()) {
            return m;
        }
        lib.split('_').find_map(|w| self.manufacturers.get(w)).cloned().unwrap_or_default()
    }
}

/// Category from a library name alone: the first word, and the rest as sub-category.
fn derived(lib: &str) -> (String, String) {
    match lib.split_once('_') {
        Some((head, rest)) => (head.to_owned(), rest.replace('_', " ")),
        None => (lib.to_owned(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: &str = r#"
        [manufacturers]
        ST = "STMicroelectronics"
        [libraries]
        Device = { category = "Miscellaneous", sub_category = "Generic devices" }
        "74xx" = { category = "TTL 74 series" }
        MCU_Odd = { category = "Microprocessor ICs", manufacturer = "Odd Corp" }
        [[rules]]
        library = "74xx*"
        symbol = "74LS*"
        category = "TTL 74LS series"
        [[rules]]
        library = "Device"
        symbol = "R*"
        category = "Resistors"
        sub_category = "Generic"
        [devices]
        "Device:R" = { name = "RES", primitive = "ANALOG", sub_category = "Fixed", default_package = "R_Axial" }
        "Device:Q" = { default_package = "TO-92", pin_map = { E = "1", B = "2" } }
    "#;

    #[test]
    fn places_by_device_rule_library_and_name() {
        let m = Map::parse(MAP).unwrap();
        let p = m.place("Device", "R");
        assert_eq!((p.category.as_str(), p.sub_category.as_str(), p.library_mapped), ("Resistors", "Fixed", true));
        assert_eq!(m.place("Device", "C").category, "Miscellaneous");
        assert_eq!(m.place("74xx", "74LS00").category, "TTL 74LS series");
        assert_eq!(m.place("74xx", "74HC00").category, "TTL 74 series");
        let p = m.place("Sensor_Gas", "X");
        assert_eq!((p.category.as_str(), p.sub_category.as_str(), p.library_mapped), ("Sensor", "Gas", false));
        assert_eq!(m.device("Device", "R").and_then(|d| d.name.as_deref()), Some("RES"));
        assert_eq!(m.device("Device", "R").and_then(|d| d.default_package.as_deref()), Some("R_Axial"));
        let q = m.device("Device", "Q").unwrap();
        assert_eq!(q.pin_map.as_ref().map(|p| p.get("B").map(String::as_str)), Some(Some("2")));
    }

    #[test]
    fn manufacturers() {
        let m = Map::parse(MAP).unwrap();
        assert_eq!(m.manufacturer("MCU_ST_STM32F4"), "STMicroelectronics");
        assert_eq!(m.manufacturer("MCU_Odd"), "Odd Corp");
        assert_eq!(m.manufacturer("Device"), "");
    }

    #[test]
    fn rejects_unknown_keys() {
        assert!(Map::parse("[libraries]\nX = { category = \"A\", colour = \"red\" }").is_err());
        assert!(Map::parse("nonsense = [").is_err());
    }
}
