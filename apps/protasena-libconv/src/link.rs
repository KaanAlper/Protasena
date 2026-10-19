//! Linking devices to packages.
//!
//! - The symbol's `Footprint` field (`Library:Footprint`) is linked first and becomes the
//!   default package, whatever its pads.
//! - Its `ki_fp_filters` (space-separated wildcards, optionally `LibraryPattern:NamePattern`,
//!   compared without case) add every footprint whose set of pad numbers equals the symbol's
//!   set of pin numbers, in library/name order, up to [`MAX_FILTER_LINKS`] per device.
//! - Pin maps send pin `N` (and `N#k`, see [`crate::symbol::pad_number`]) to pad `N` when the
//!   package has it.

use std::collections::{BTreeSet, HashMap, HashSet};

use protasena_library::PackageRef;

use crate::glob;
use crate::symbol::pad_number;

/// Most packages one device gets from its footprint filters.
pub const MAX_FILTER_LINKS: usize = 64;

/// What the linker needs to know about a converted footprint.
#[derive(Debug, Clone)]
pub struct FpEntry {
    /// KiCad footprint library (`.pretty` directory name without the extension).
    pub lib: String,
    /// KiCad footprint name (file name without `.kicad_mod`).
    pub name: String,
    /// The package's name in the pack.
    pub package: String,
    /// Every pad name of the package.
    pub pads: HashSet<String>,
}

/// Footprints by name and by pad-number set.
#[derive(Debug, Default)]
pub struct FootprintIndex {
    entries: Vec<FpEntry>,
    by_key: HashMap<(String, String), usize>,
    by_name: HashMap<String, usize>,
    by_signature: HashMap<Vec<String>, Vec<usize>>,
}

/// The links of one device.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Links {
    /// Package references, `Footprint` field first.
    pub packages: Vec<PackageRef>,
    /// The `Footprint` field's package, when found.
    pub default: Option<String>,
    /// The `Footprint` field named a footprint that does not exist.
    pub field_missing: bool,
    /// The `Footprint` field as written (trimmed).
    pub field: String,
    /// Links that came from the filters.
    pub filter_links: usize,
    /// More filter matches existed than [`MAX_FILTER_LINKS`].
    pub capped: bool,
}

impl FootprintIndex {
    /// Indexes `entries` (kept in the given order, which decides filter-link order).
    pub fn new(entries: Vec<FpEntry>) -> Self {
        let mut index = FootprintIndex::default();
        for (i, e) in entries.iter().enumerate() {
            index.by_key.entry((e.lib.clone(), e.name.clone())).or_insert(i);
            index.by_name.entry(e.name.clone()).or_insert(i);
            index.by_signature.entry(signature(e.pads.iter().map(String::as_str))).or_default().push(i);
        }
        index.entries = entries;
        index
    }

    /// Number of footprints indexed.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is indexed.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Links for a symbol with the given `Footprint` field, filter text and pin numbers.
    pub fn link(&self, field: &str, filters: &str, pins: &[String]) -> Links {
        let mut links = Links::default();
        let field = field.trim();
        field.clone_into(&mut links.field);
        let mut used = BTreeSet::new();
        if !field.is_empty() {
            match self.find(field) {
                Some(i) => {
                    if let Some(r) = self.package_ref(i, pins) {
                        links.default = Some(r.name.clone());
                        used.insert(i);
                        links.packages.push(r);
                    }
                }
                None => links.field_missing = true,
            }
        }
        let patterns: Vec<(Option<&str>, &str)> =
            filters.split_whitespace().map(|f| f.split_once(':').map_or((None, f), |(l, n)| (Some(l), n))).collect();
        if patterns.is_empty() || pins.is_empty() {
            return links;
        }
        let Some(candidates) = self.by_signature.get(&signature(pins.iter().map(String::as_str))) else { return links };
        for &i in candidates {
            let Some(e) = self.entries.get(i) else { continue };
            let hit = patterns.iter().any(|(lp, np)| lp.is_none_or(|lp| glob::matches(lp, &e.lib, true)) && glob::matches(np, &e.name, true));
            if !hit || used.contains(&i) {
                continue;
            }
            if links.filter_links == MAX_FILTER_LINKS {
                links.capped = true;
                break;
            }
            if let Some(r) = self.package_ref(i, pins) {
                used.insert(i);
                links.packages.push(r);
                links.filter_links += 1;
            }
        }
        links
    }

    fn find(&self, field: &str) -> Option<usize> {
        match field.split_once(':') {
            Some((lib, name)) => self.by_key.get(&(lib.to_owned(), name.to_owned())).copied(),
            None => self.by_name.get(field).copied(),
        }
    }

    fn package_ref(&self, i: usize, pins: &[String]) -> Option<PackageRef> {
        let e = self.entries.get(i)?;
        let pin_map = pins.iter().filter(|p| e.pads.contains(pad_number(p))).map(|p| (p.clone(), pad_number(p).to_owned())).collect();
        Some(PackageRef { name: e.package.clone(), pin_map })
    }
}

/// Sorted distinct pad numbers (renamed `N#k` counted as `N`; unnamed pads ignored).
fn signature<'a>(names: impl Iterator<Item = &'a str>) -> Vec<String> {
    let set: BTreeSet<&str> = names.map(pad_number).filter(|n| !n.is_empty()).collect();
    set.into_iter().map(str::to_owned).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(lib: &str, name: &str, pads: &[&str]) -> FpEntry {
        FpEntry { lib: lib.into(), name: name.into(), package: name.into(), pads: pads.iter().map(|p| (*p).to_owned()).collect() }
    }

    fn index() -> FootprintIndex {
        FootprintIndex::new(vec![
            entry("Resistor_SMD", "R_0603", &["1", "2"]),
            entry("Resistor_THT", "R_Axial", &["1", "2"]),
            entry("Resistor_SMD", "R_Array_4", &["1", "2", "3", "4"]),
            entry("Capacitor_SMD", "C_0603", &["1", "2"]),
            entry("Package_SO", "SOIC-8", &["1", "2", "3", "4", "5", "6", "7", "8", "9", "9#2"]),
        ])
    }

    fn pins(p: &[&str]) -> Vec<String> {
        p.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn field_first_then_filters_with_equal_pad_sets() {
        let links = index().link("Resistor_THT:R_Axial", "R_*", &pins(&["1", "2"]));
        let names: Vec<&str> = links.packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["R_Axial", "R_0603"]);
        assert_eq!(links.default.as_deref(), Some("R_Axial"));
        assert_eq!(links.filter_links, 1);
        assert_eq!(links.packages[0].pin_map, vec![("1".into(), "1".into()), ("2".into(), "2".into())]);
    }

    #[test]
    fn library_patterns_and_case() {
        let links = index().link("", "resistor_smd:r_*", &pins(&["1", "2"]));
        let names: Vec<&str> = links.packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["R_0603"]);
        assert_eq!(links.default, None);
    }

    #[test]
    fn missing_field_and_partial_pin_maps() {
        let links = index().link("Nope:X", "", &pins(&["1"]));
        assert!(links.field_missing && links.packages.is_empty());
        let links = index().link("Package_SO:SOIC-8", "", &pins(&["1", "9", "9#2", "10"]));
        let map = &links.packages[0].pin_map;
        assert_eq!(map.len(), 3, "pin 10 has no pad");
        assert_eq!(map[2], ("9#2".into(), "9".into()));
    }
}
