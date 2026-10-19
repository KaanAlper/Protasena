//! Consistency checks for a set of devices and packages. Every problem is collected; nothing
//! here panics, whatever the data.

use std::collections::{HashMap, HashSet};

use crate::device::{Device, PackageRef, PropDef, PropKind};
use crate::error::{Issue, IssueKind};
use crate::limits;
use crate::package::Package;
use crate::symbol::{Symbol, Unit};

/// All problems of a device and package set (empty when it is consistent).
pub(crate) fn validate(devices: &[Device], packages: &[Package]) -> Vec<Issue> {
    let mut out = Issues::default();
    let mut package_libs: HashMap<&str, &str> = HashMap::new();
    let mut per_lib: HashMap<&str, (usize, usize)> = HashMap::new();
    for pkg in packages {
        per_lib.entry(pkg.library.as_str()).or_default().1 += 1;
        check_package(&mut out, pkg);
        match package_libs.get(pkg.name.as_str()) {
            Some(first) => out.push(&pkg.library, &pkg.name, IssueKind::DuplicatePackage { first_library: (*first).to_owned() }),
            None => {
                package_libs.insert(&pkg.name, &pkg.library);
            }
        }
    }
    let pads: PadSets = packages.iter().map(|p| (p.name.as_str(), p.pads.iter().map(|d| d.name.as_str()).collect())).collect();
    let mut device_libs: HashMap<String, &str> = HashMap::new();
    for dev in devices {
        per_lib.entry(dev.library.as_str()).or_default().0 += 1;
        check_device(&mut out, dev, &pads);
        let key = dev.name.to_uppercase();
        match device_libs.get(&key) {
            Some(first) => out.push(&dev.library, &dev.name, IssueKind::DuplicateDevice { first_library: (*first).to_owned() }),
            None => {
                device_libs.insert(key, &dev.library);
            }
        }
    }
    let mut libs: Vec<_> = per_lib.into_iter().collect();
    libs.sort_unstable();
    for (lib, (devs, pkgs)) in libs {
        out.limit(lib, None, "devices", devs, limits::MAX_DEVICES);
        out.limit(lib, None, "packages", pkgs, limits::MAX_PACKAGES);
    }
    out.0
}

/// The problem with a name, if any.
pub(crate) fn name_problem(name: &str, what: &'static str) -> Option<IssueKind> {
    if name.is_empty() {
        Some(IssueKind::EmptyName(what))
    } else if name.len() > limits::MAX_NAME_BYTES || name.chars().any(char::is_control) {
        Some(IssueKind::BadName(what))
    } else {
        None
    }
}

/// Pad names per package name.
type PadSets<'a> = HashMap<&'a str, HashSet<&'a str>>;

#[derive(Default)]
struct Issues(Vec<Issue>);

impl Issues {
    fn push(&mut self, library: &str, item: &str, kind: IssueKind) {
        let item = (!item.is_empty()).then(|| item.to_owned());
        self.0.push(Issue { library: library.to_owned(), item, kind });
    }

    fn limit(&mut self, library: &str, item: Option<&str>, what: &'static str, count: usize, limit: usize) {
        if count > limit {
            self.push(library, item.unwrap_or_default(), IssueKind::TooMany { what, count, limit });
        }
    }

    fn name(&mut self, library: &str, item: &str, name: &str, what: &'static str) {
        if let Some(kind) = name_problem(name, what) {
            self.push(library, item, kind);
        }
    }
}

fn check_package(out: &mut Issues, pkg: &Package) {
    let (lib, item) = (pkg.library.as_str(), pkg.name.as_str());
    out.name(lib, item, &pkg.library, "library");
    out.name(lib, item, &pkg.name, "package");
    out.limit(lib, Some(item), "pads", pkg.pads.len(), limits::MAX_PADS);
    out.limit(lib, Some(item), "graphics", pkg.graphics.len(), limits::MAX_GRAPHICS);
    let mut seen = HashSet::new();
    for pad in &pkg.pads {
        if !seen.insert(pad.name.as_str()) {
            out.push(lib, item, IssueKind::DuplicatePad(pad.name.clone()));
        }
        if !pad.is_sane() {
            out.push(lib, item, IssueKind::NonFinite("pad"));
        }
    }
    if pkg.graphics.iter().any(|g| !g.graphic.shape.is_finite() || !g.graphic.width.is_finite()) {
        out.push(lib, item, IssueKind::NonFinite("package graphic"));
    }
}

fn check_device(out: &mut Issues, dev: &Device, pads: &PadSets) {
    let (lib, item) = (dev.library.as_str(), dev.name.as_str());
    out.name(lib, item, &dev.library, "library");
    out.name(lib, item, &dev.name, "device");
    check_symbol(out, lib, item, &dev.symbol);
    out.limit(lib, Some(item), "packages", dev.packages.len(), limits::MAX_DEVICE_PACKAGES);
    let pins: HashSet<&str> = dev.symbol.pins().map(|p| p.number.as_str()).collect();
    let mut seen = HashSet::new();
    for pref in &dev.packages {
        if !seen.insert(pref.name.as_str()) {
            out.push(lib, item, IssueKind::PackageListedTwice(pref.name.clone()));
        }
        check_pin_map(out, (lib, item), &pins, pref, pads.get(pref.name.as_str()));
    }
    if let Some(def) = &dev.default_package
        && dev.package(def).is_none()
    {
        out.push(lib, item, IssueKind::BadDefaultPackage(def.clone()));
    }
    check_props(out, lib, item, &dev.prop_defs);
}

fn check_symbol(out: &mut Issues, lib: &str, item: &str, symbol: &Symbol) {
    if symbol.units.is_empty() {
        out.push(lib, item, IssueKind::NoUnits);
    }
    out.limit(lib, Some(item), "units", symbol.units.len(), limits::MAX_UNITS);
    for unit in &symbol.units {
        check_unit(out, lib, item, unit);
    }
}

fn check_unit(out: &mut Issues, lib: &str, item: &str, unit: &Unit) {
    out.limit(lib, Some(item), "pins in a unit", unit.pins.len(), limits::MAX_PINS);
    out.limit(lib, Some(item), "graphics in a unit", unit.graphics.len(), limits::MAX_GRAPHICS);
    let mut seen = HashSet::new();
    for pin in &unit.pins {
        if pin.number.is_empty() {
            out.push(lib, item, IssueKind::EmptyName("pin number"));
        } else if !seen.insert(pin.number.as_str()) {
            out.push(lib, item, IssueKind::DuplicatePin(pin.number.clone()));
        }
        if !pin.at.is_finite() || !pin.length.is_finite() {
            out.push(lib, item, IssueKind::NonFinite("pin"));
        }
    }
    if unit.graphics.iter().any(|g| !g.shape.is_finite() || !g.width.is_finite()) {
        out.push(lib, item, IssueKind::NonFinite("symbol graphic"));
    }
}

fn check_pin_map(out: &mut Issues, (lib, item): (&str, &str), pins: &HashSet<&str>, pref: &PackageRef, pads: Option<&HashSet<&str>>) {
    if pads.is_none() {
        out.push(lib, item, IssueKind::DanglingPackage(pref.name.clone()));
    }
    let mut seen = HashSet::new();
    for (pin, pad) in &pref.pin_map {
        if !seen.insert(pin.as_str()) {
            out.push(lib, item, IssueKind::PinMappedTwice { package: pref.name.clone(), pin: pin.clone() });
        }
        if !pins.contains(pin.as_str()) {
            out.push(lib, item, IssueKind::PinMapMissingPin { package: pref.name.clone(), pin: pin.clone() });
        }
        if let Some(pads) = pads
            && !pads.contains(pad.as_str())
        {
            out.push(lib, item, IssueKind::PinMapMissingPad { package: pref.name.clone(), pad: pad.clone() });
        }
    }
}

fn check_props(out: &mut Issues, lib: &str, item: &str, props: &[PropDef]) {
    out.limit(lib, Some(item), "property definitions", props.len(), limits::MAX_PROP_DEFS);
    let mut seen = HashSet::new();
    for prop in props {
        out.name(lib, item, &prop.name, "property");
        if !seen.insert(prop.name.to_uppercase()) {
            out.push(lib, item, IssueKind::DuplicatePropDef(prop.name.clone()));
        }
        if let (PropKind::Keywords { choices }, Some(value)) = (&prop.kind, &prop.default)
            && !choices.iter().any(|c| c.eq_ignore_ascii_case(value))
        {
            out.push(lib, item, IssueKind::BadKeywordDefault { prop: prop.name.clone(), value: value.clone() });
        }
    }
}
