//! Libraries added on top of a loaded library: the user's imported KiCad libraries and the
//! writable user libraries ([`Library::with_extra`]).

use std::collections::{HashMap, HashSet};

use crate::device::Device;
use crate::error::{Issue, IssueKind};
use crate::library::Library;
use crate::package::Package;
use crate::store::Packages;
use crate::validate::{PadSets, device_issues, package_issues};

impl Library {
    /// This library with `devices` and `packages` of further libraries added after it, which
    /// [`Library::without_extra`] takes off again. Nothing of `self` is replaced: an item of a
    /// library `self` already has is left out, and so is a package whose name is taken or a
    /// device whose name its library already has. A device may repeat the name of a device of
    /// *another* library (Pick Devices lists both, each under its library; a lookup by name
    /// alone finds the first, see [`Library::get`]). Devices may use packages of `self`.
    ///
    /// Everything left out is reported, with every other problem the items have; the result is
    /// consistent apart from the repeated names it allows.
    pub fn with_extra(self, devices: Vec<Device>, packages: Vec<Package>) -> (Library, Vec<Issue>) {
        let mut issues = Vec::new();
        let loaded: HashSet<String> = self.libraries().iter().map(|l| l.name.to_uppercase()).collect();
        let taken_library = |library: &str, item: &str, issues: &mut Vec<Issue>| {
            let taken = loaded.contains(&library.to_uppercase());
            if taken {
                issues.push(Issue { library: library.to_owned(), item: Some(item.to_owned()), kind: IssueKind::LibraryTaken });
            }
            taken
        };
        let packages: Vec<Package> = packages.into_iter().filter(|p| !taken_library(&p.library, &p.name, &mut issues)).collect();
        let devices: Vec<Device> = devices.into_iter().filter(|d| !taken_library(&d.library, &d.name, &mut issues)).collect();
        let taken: HashMap<String, String> = self.package_names().map(|(n, l)| (n.to_owned(), l.to_owned())).collect();
        let mut kept: Vec<Package> = Vec::new();
        let mut seen = HashSet::new();
        for p in packages {
            let first = taken.get(&p.name).cloned().or_else(|| (!seen.insert(p.name.clone())).then(|| p.library.clone()));
            if let Some(first_library) = first {
                issues.push(Issue { library: p.library.clone(), item: Some(p.name.clone()), kind: IssueKind::DuplicatePackage { first_library } });
                continue;
            }
            let problems = package_issues(&p);
            if problems.is_empty() {
                kept.push(p);
            } else {
                issues.extend(problems);
            }
        }
        let pads = self.pad_sets(&devices, &kept);
        let mut names: HashMap<(String, String), String> =
            self.devices().iter().map(|d| ((d.library.to_uppercase(), d.name.to_uppercase()), d.library.clone())).collect();
        let mut accepted = Vec::new();
        for d in devices {
            let problems = device_issues(&d, &pads);
            if !problems.is_empty() {
                issues.extend(problems);
                continue;
            }
            let key = (d.library.to_uppercase(), d.name.to_uppercase());
            if let Some(first_library) = names.get(&key) {
                let kind = IssueKind::DuplicateDevice { first_library: first_library.clone() };
                issues.push(Issue { library: d.library.clone(), item: Some(d.name.clone()), kind });
                continue;
            }
            names.insert(key, d.library.clone());
            accepted.push(d);
        }
        let added: Vec<String> = library_names(&accepted, &kept);
        let Library { devices: mut all, packages: store, extra_libraries: mut extra, .. } = self;
        all.extend(accepted);
        extra.extend(added);
        let mut lib = Library::assemble(all, store.chain(Packages::from_vec(kept)));
        lib.extra_libraries = extra;
        (lib, issues)
    }

    /// The library without what [`Library::with_extra`] added.
    pub fn without_extra(self) -> Library {
        if self.extra_libraries.is_empty() {
            return self;
        }
        let drop: HashSet<&str> = self.extra_libraries.iter().map(String::as_str).collect();
        let devices = self.devices.into_iter().filter(|d| !drop.contains(d.library.as_str())).collect();
        Library::assemble(devices, self.packages.without_libraries(&drop))
    }

    /// The libraries [`Library::with_extra`] added, in order.
    pub fn extra_libraries(&self) -> &[String] {
        &self.extra_libraries
    }

    /// Pad names of every package the devices use, from `extra` or from this library.
    fn pad_sets(&self, devices: &[Device], extra: &[Package]) -> PadSets {
        let by_name: HashMap<&str, &Package> = extra.iter().map(|p| (p.name.as_str(), p)).collect();
        let mut out = PadSets::new();
        for name in devices.iter().flat_map(|d| d.packages.iter().map(|r| r.name.as_str())) {
            if out.contains_key(name) {
                continue;
            }
            if let Some(p) = by_name.get(name).copied().or_else(|| self.package(name)) {
                out.insert(name.to_owned(), p.pads.iter().map(|pad| pad.name.clone()).collect());
            }
        }
        out
    }
}

/// The library names of the items, in order of first appearance.
fn library_names(devices: &[Device], packages: &[Package]) -> Vec<String> {
    let mut seen = HashSet::new();
    devices.iter().map(|d| &d.library).chain(packages.iter().map(|p| &p.library)).filter(|l| seen.insert(l.as_str())).cloned().collect()
}
