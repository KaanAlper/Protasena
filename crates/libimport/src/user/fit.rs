//! Fitting imported libraries in beside the loaded ones: names that would clash are changed.

use std::collections::{HashMap, HashSet};

use protasena_library::{Library, PropKind};

use super::Source;
use crate::convert::FOOTPRINT_LIB_SUFFIX;
use protasena_library::{Device, Package};

/// Added to the name of an imported library that a loaded library already has.
pub const LIBRARY_SUFFIX: &str = "_KICAD";

/// The devices and packages of `sources`, renamed where they would clash with `base` or with
/// an earlier source, and a note for every rename:
/// - a library whose name `base` (or an earlier source) already uses gets [`LIBRARY_SUFFIX`]
///   (then `_2`, `_3`, …);
/// - a package whose name is taken becomes `Library:Name` (then `Library:Name_2`, …), and the
///   devices of the same source follow it.
///
/// Device names are left alone: the same name may be listed in several libraries.
pub fn fit(sources: &[Source], base: &Library) -> (Vec<Device>, Vec<Package>, Vec<String>) {
    let mut libs: HashSet<String> = base.libraries().iter().map(|l| l.name.to_uppercase()).collect();
    let mut pkgs: HashSet<String> = base.package_names().map(|(n, _)| n.to_owned()).collect();
    let (mut devices, mut packages, mut notes) = (Vec::new(), Vec::new(), Vec::new());
    for src in sources {
        let mut lib_names: HashMap<String, String> = HashMap::new();
        let names = src.devices.iter().map(|d| d.library.as_str()).chain(src.packages.iter().map(|p| p.library.as_str()));
        for name in names {
            if lib_names.contains_key(name) {
                continue;
            }
            let new = free(name, &mut libs, |n| format!("{name}{LIBRARY_SUFFIX}{}", suffix(n)), str::to_uppercase);
            if new != name {
                notes.push(format!("{}: library {name} is imported as {new} (the name is taken)", src.report.path));
            }
            lib_names.insert(name.to_owned(), new);
        }
        let mut renamed: HashMap<String, String> = HashMap::new();
        for p in &src.packages {
            let stem = p.library.strip_suffix(FOOTPRINT_LIB_SUFFIX).unwrap_or(&p.library);
            let new = if pkgs.insert(p.name.clone()) {
                p.name.clone()
            } else {
                let base_name = format!("{stem}:{}", p.name.rsplit(':').next().unwrap_or(&p.name));
                free(&base_name, &mut pkgs, |n| format!("{base_name}{}", suffix(n)), str::to_owned)
            };
            if new != p.name {
                notes.push(format!("{}: package {} is imported as {new} (the name is taken)", src.report.path, p.name));
                renamed.insert(p.name.clone(), new.clone());
            }
            let library = lib_names.get(&p.library).cloned().unwrap_or_else(|| p.library.clone());
            packages.push(Package { name: new, library, ..p.clone() });
        }
        for d in &src.devices {
            devices.push(refit(d, &lib_names, &renamed));
        }
    }
    (devices, packages, notes)
}

/// `""`, then `_2`, `_3`, … for the `n`-th try.
fn suffix(n: usize) -> String {
    if n == 0 { String::new() } else { format!("_{}", n + 1) }
}

/// `want` when it is free, else the first free `make(n)`; the name is taken in `used` (keys
/// made with `key`).
fn free(want: &str, used: &mut HashSet<String>, make: impl Fn(usize) -> String, key: fn(&str) -> String) -> String {
    if used.insert(key(want)) {
        return want.to_owned();
    }
    let mut n = 0;
    loop {
        let name = make(n);
        if used.insert(key(&name)) {
            return name;
        }
        n += 1;
    }
}

/// The device in its renamed library, with its renamed packages.
fn refit(d: &Device, libs: &HashMap<String, String>, pkgs: &HashMap<String, String>) -> Device {
    let pkg = |n: &String| pkgs.get(n).cloned().unwrap_or_else(|| n.clone());
    let mut d = d.clone();
    if let Some(l) = libs.get(&d.library) {
        d.library.clone_from(l);
    }
    for r in &mut d.packages {
        r.name = pkg(&r.name);
    }
    d.default_package = d.default_package.as_ref().map(pkg);
    for p in &mut d.prop_defs {
        if p.kind == PropKind::Package {
            p.default = p.default.as_ref().map(pkg);
        }
    }
    d
}
