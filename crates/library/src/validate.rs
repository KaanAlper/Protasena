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
    let mut v = Validator::default();
    v.packages(packages);
    v.devices(devices);
    v.finish()
}

/// Validation in steps, for a loader that sees one file at a time: every package first (only
/// their names and pad names are kept), then every device.
#[derive(Default)]
pub(crate) struct Validator {
    out: Issues,
    package_libs: HashMap<String, String>,
    pads: HashMap<String, HashSet<String>>,
    device_libs: HashMap<String, String>,
    per_lib: HashMap<String, (usize, usize)>,
}

impl Validator {
    /// Checks packages (all of them come before the first device).
    pub(crate) fn packages<'a>(&mut self, packages: impl IntoIterator<Item = &'a Package>) {
        for pkg in packages {
            self.per_lib.entry(pkg.library.clone()).or_default().1 += 1;
            check_package(&mut self.out, pkg);
            match self.package_libs.get(&pkg.name) {
                Some(first) => self.out.push(&pkg.library, &pkg.name, IssueKind::DuplicatePackage { first_library: first.clone() }),
                None => {
                    self.package_libs.insert(pkg.name.clone(), pkg.library.clone());
                    self.pads.insert(pkg.name.clone(), pkg.pads.iter().map(|d| d.name.clone()).collect());
                }
            }
        }
    }

    /// Checks devices against the packages seen so far.
    pub(crate) fn devices(&mut self, devices: &[Device]) {
        for dev in devices {
            self.per_lib.entry(dev.library.clone()).or_default().0 += 1;
            check_device(&mut self.out, dev, &self.pads);
            let key = dev.name.to_uppercase();
            match self.device_libs.get(&key) {
                Some(first) => self.out.push(&dev.library, &dev.name, IssueKind::DuplicateDevice { first_library: first.clone() }),
                None => {
                    self.device_libs.insert(key, dev.library.clone());
                }
            }
        }
    }

    /// Every problem found, with the per-library limits checked last.
    pub(crate) fn finish(mut self) -> Vec<Issue> {
        let mut libs: Vec<_> = self.per_lib.into_iter().collect();
        libs.sort_unstable();
        for (lib, (devs, pkgs)) in libs {
            self.out.limit(&lib, None, "devices", devs, limits::MAX_DEVICES);
            self.out.limit(&lib, None, "packages", pkgs, limits::MAX_PACKAGES);
        }
        self.out.0
    }
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
pub(crate) type PadSets = HashMap<String, HashSet<String>>;

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

/// The problems of one package on its own.
pub(crate) fn package_issues(pkg: &Package) -> Vec<Issue> {
    let mut out = Issues::default();
    check_package(&mut out, pkg);
    out.0
}

/// The problems of one device on its own, with `pads` answering for the packages it uses.
pub(crate) fn device_issues(dev: &Device, pads: &PadSets) -> Vec<Issue> {
    let mut out = Issues::default();
    check_device(&mut out, dev, pads);
    out.0
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
        check_pin_map(out, (lib, item), &pins, pref, pads.get(&pref.name));
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
        for alt in &pin.alternates {
            out.name(lib, item, &alt.name, "pin alternate");
        }
    }
    if unit.graphics.iter().any(|g| !g.shape.is_finite() || !g.width.is_finite()) {
        out.push(lib, item, IssueKind::NonFinite("symbol graphic"));
    }
}

fn check_pin_map(out: &mut Issues, (lib, item): (&str, &str), pins: &HashSet<&str>, pref: &PackageRef, pads: Option<&HashSet<String>>) {
    if pads.is_none() {
        out.push(lib, item, IssueKind::DanglingPackage(pref.name.clone()));
    }
    let mut seen = HashSet::new();
    for (pin, pad) in pref.pin_map.iter() {
        if !seen.insert(pin.as_str()) {
            out.push(lib, item, IssueKind::PinMappedTwice { package: pref.name.clone(), pin: pin.clone() });
        }
        if !pins.contains(pin.as_str()) {
            out.push(lib, item, IssueKind::PinMapMissingPin { package: pref.name.clone(), pin: pin.clone() });
        }
        if let Some(pads) = pads
            && !pads.contains(pad)
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
