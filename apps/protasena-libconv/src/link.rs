//! Linking devices to packages.
//!
//! - A `default_package` from the map (`[devices]` entry, [`crate::map::DeviceEntry`]) is
//!   linked first, whatever its pads, with the entry's `pin_map` when it has one.
//! - The symbol's `Footprint` field (`Library:Footprint`) is linked next, whatever its pads.
//! - Its `ki_fp_filters` (space-separated wildcards, optionally `LibraryPattern:NamePattern`,
//!   compared without case) add every footprint whose set of pad numbers equals the symbol's
//!   set of pin numbers, in library/name order, up to [`MAX_FILTER_LINKS`] per device.
//! - Pin maps send pin `N` (and `N#k`, see [`crate::symbol::pad_number`]) to pad `N` when the
//!   package has it.
//!
//! **The default package** (the device's `PACKAGE` property) is, in this order: the map's
//! `default_package`; else the `Footprint` field's package; else the first linked package with
//! a through-hole pad; else the first linked package. Through-hole comes first because a
//! schematic-first, breadboard-to-board workflow starts from leaded parts, and because the
//! filters list SMD footprints first in library order (`R_*` matches LDRs and chip resistors
//! before axial ones).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

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
    /// Whether a named pad is drilled (a through-hole package).
    pub tht: bool,
}

/// The map's default package for a device.
#[derive(Debug, Clone, Copy)]
pub struct Preferred<'a> {
    /// Footprint name, or `Library:Name`.
    pub footprint: &'a str,
    /// Pin → pad, replacing the by-number pin map (e.g. a transistor's `B`, `C`, `E` pins).
    pub pin_map: Option<&'a BTreeMap<String, String>>,
}

/// Footprints by name and by pad-number set.
#[derive(Debug, Default)]
pub struct FootprintIndex {
    entries: Vec<FpEntry>,
    by_key: HashMap<(String, String), usize>,
    by_name: HashMap<String, usize>,
    by_signature: HashMap<Vec<String>, Vec<usize>>,
    by_package: HashMap<String, usize>,
}

/// The links of one device.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Links {
    /// Package references: the map's default, the `Footprint` field's, then the filters'.
    pub packages: Vec<PackageRef>,
    /// The default package (see the module docs), when the device has any package.
    pub default: Option<String>,
    /// The `Footprint` field's package was found and linked.
    pub field_linked: bool,
    /// The map's default package was found and linked.
    pub map_linked: bool,
    /// What is wrong with the map's default package (missing footprint, pin map that does not
    /// fit), if anything.
    pub map_problem: Option<String>,
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
            index.by_package.entry(e.package.clone()).or_insert(i);
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

    /// Links for a symbol with the map's `preferred` package, the given `Footprint` field,
    /// filter text and pin numbers.
    pub fn link(&self, preferred: Option<Preferred<'_>>, field: &str, filters: &str, pins: &[String]) -> Links {
        let mut links = Links::default();
        let mut used = BTreeSet::new();
        if let Some(p) = preferred {
            self.link_preferred(p, pins, &mut links, &mut used);
        }
        let field = field.trim();
        field.clone_into(&mut links.field);
        if !field.is_empty() {
            match self.find(field) {
                Some(i) => {
                    links.field_linked = true;
                    if used.insert(i) {
                        links.packages.extend(self.package_ref(i, pins));
                    }
                }
                None => links.field_missing = true,
            }
        }
        self.link_filters(filters, pins, &mut links, &mut used);
        links.default = self.default_of(&links, field);
        links
    }

    fn link_preferred(&self, p: Preferred<'_>, pins: &[String], links: &mut Links, used: &mut BTreeSet<usize>) {
        let Some(i) = self.find(p.footprint.trim()) else {
            links.map_problem = Some(format!("default_package {:?} is not a footprint", p.footprint));
            return;
        };
        let Some(mut r) = self.package_ref(i, pins) else { return };
        if let (Some(map), Some(e)) = (p.pin_map, self.entries.get(i)) {
            let bad: Vec<String> = map
                .iter()
                .filter(|(pin, pad)| !pins.contains(pin) || !e.pads.contains(pad.as_str()))
                .map(|(pin, pad)| format!("{pin}→{pad}"))
                .collect();
            if !bad.is_empty() {
                links.map_problem = Some(format!("pin_map entries {} do not fit {}", bad.join(", "), e.package));
                return;
            }
            r.pin_map = pins.iter().filter_map(|pin| map.get(pin).map(|pad| (pin.clone(), pad.clone()))).collect();
        }
        used.insert(i);
        links.map_linked = true;
        links.packages.push(r);
    }

    fn link_filters(&self, filters: &str, pins: &[String], links: &mut Links, used: &mut BTreeSet<usize>) {
        let patterns: Vec<(Option<&str>, &str)> =
            filters.split_whitespace().map(|f| f.split_once(':').map_or((None, f), |(l, n)| (Some(l), n))).collect();
        if patterns.is_empty() || pins.is_empty() {
            return;
        }
        let Some(candidates) = self.by_signature.get(&signature(pins.iter().map(String::as_str))) else { return };
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
    }

    /// The default package: map, `Footprint` field, first through-hole link, first link.
    fn default_of(&self, links: &Links, field: &str) -> Option<String> {
        let first = links.packages.first().map(|r| r.name.clone());
        if links.map_linked {
            return first;
        }
        if links.field_linked {
            return self.find(field).and_then(|i| self.entries.get(i)).map(|e| e.package.clone());
        }
        let tht = links.packages.iter().find(|r| self.by_package.get(&r.name).and_then(|&i| self.entries.get(i)).is_some_and(|e| e.tht));
        tht.map(|r| r.name.clone()).or(first)
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
        let tht = lib.ends_with("_THT") || lib == "Package_TO";
        FpEntry { lib: lib.into(), name: name.into(), package: name.into(), pads: pads.iter().map(|p| (*p).to_owned()).collect(), tht }
    }

    fn index() -> FootprintIndex {
        FootprintIndex::new(vec![
            entry("OptoDevice", "R_LDR", &["1", "2"]),
            entry("Resistor_SMD", "R_0603", &["1", "2"]),
            entry("Resistor_THT", "R_Axial", &["1", "2"]),
            entry("Resistor_SMD", "R_Array_4", &["1", "2", "3", "4"]),
            entry("Capacitor_SMD", "C_0603", &["1", "2"]),
            entry("Package_SO", "SOIC-8", &["1", "2", "3", "4", "5", "6", "7", "8", "9", "9#2"]),
            entry("Package_TO", "TO-92", &["1", "2", "3"]),
        ])
    }

    fn pins(p: &[&str]) -> Vec<String> {
        p.iter().map(|s| (*s).to_owned()).collect()
    }

    fn names(links: &Links) -> Vec<&str> {
        links.packages.iter().map(|p| p.name.as_str()).collect()
    }

    #[test]
    fn field_first_then_filters_with_equal_pad_sets() {
        let links = index().link(None, "Resistor_SMD:R_0603", "R_*", &pins(&["1", "2"]));
        assert_eq!(names(&links), ["R_0603", "R_LDR", "R_Axial"]);
        assert_eq!(links.default.as_deref(), Some("R_0603"), "the field wins over through-hole");
        assert!(links.field_linked && !links.map_linked);
        assert_eq!(links.filter_links, 2);
        assert_eq!(*links.packages[0].pin_map, [("1".into(), "1".into()), ("2".into(), "2".into())]);
    }

    #[test]
    fn without_map_or_field_the_first_through_hole_link_is_the_default() {
        let links = index().link(None, "", "R_*", &pins(&["1", "2"]));
        assert_eq!(names(&links), ["R_LDR", "R_0603", "R_Axial"]);
        assert_eq!(links.default.as_deref(), Some("R_Axial"));
        let links = index().link(None, "", "C_*", &pins(&["1", "2"]));
        assert_eq!(links.default.as_deref(), Some("C_0603"), "no through-hole link: the first one");
        assert_eq!(index().link(None, "", "", &pins(&["1", "2"])).default, None);
    }

    #[test]
    fn the_map_default_comes_first_and_wins() {
        let p = Preferred { footprint: "Resistor_THT:R_Axial", pin_map: None };
        let links = index().link(Some(p), "Resistor_SMD:R_0603", "R_*", &pins(&["1", "2"]));
        assert_eq!(names(&links), ["R_Axial", "R_0603", "R_LDR"], "no repeats");
        assert_eq!(links.default.as_deref(), Some("R_Axial"));
        assert!(links.map_linked && links.field_linked && links.map_problem.is_none());
    }

    #[test]
    fn the_map_pin_map_replaces_numbers() {
        let map: BTreeMap<String, String> = [("E", "1"), ("B", "2"), ("C", "3")].into_iter().map(|(a, b)| (a.into(), b.into())).collect();
        let links = index().link(Some(Preferred { footprint: "TO-92", pin_map: Some(&map) }), "", "", &pins(&["B", "C", "E"]));
        assert_eq!(links.default.as_deref(), Some("TO-92"));
        assert_eq!(*links.packages[0].pin_map, [("B".into(), "2".into()), ("C".into(), "3".into()), ("E".into(), "1".into())]);
    }

    #[test]
    fn map_problems_are_reported_and_linking_goes_on() {
        let links = index().link(Some(Preferred { footprint: "Nope", pin_map: None }), "", "R_*", &pins(&["1", "2"]));
        assert!(links.map_problem.as_deref().is_some_and(|m| m.contains("Nope")));
        assert_eq!(links.default.as_deref(), Some("R_Axial"));
        let map: BTreeMap<String, String> = [("B".to_owned(), "7".to_owned())].into_iter().collect();
        let links = index().link(Some(Preferred { footprint: "TO-92", pin_map: Some(&map) }), "", "", &pins(&["B"]));
        assert!(links.map_problem.as_deref().is_some_and(|m| m.contains("B→7")));
        assert!(links.packages.is_empty() && links.default.is_none());
    }

    #[test]
    fn library_patterns_and_case() {
        let links = index().link(None, "", "resistor_smd:r_*", &pins(&["1", "2"]));
        assert_eq!(names(&links), ["R_0603"]);
        assert_eq!(links.default.as_deref(), Some("R_0603"));
    }

    #[test]
    fn missing_field_and_partial_pin_maps() {
        let links = index().link(None, "Nope:X", "", &pins(&["1"]));
        assert!(links.field_missing && links.packages.is_empty());
        let links = index().link(None, "Package_SO:SOIC-8", "", &pins(&["1", "9", "9#2", "10"]));
        let map = &links.packages[0].pin_map;
        assert_eq!(map.len(), 3, "pin 10 has no pad");
        assert_eq!(map[2], ("9#2".into(), "9".into()));
    }
}
