//! The whole run: footprints → packages, symbols → devices, unique names, validation, pack
//! writing and a final load of the written pack.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use protasena_library::{Device, Library, LibraryError, Package};

use crate::device::Draft;
use crate::inputs::{self, FpResult};
use crate::link::{FootprintIndex, FpEntry};
use crate::map::Map;
use crate::names::{self, Request};
use crate::pack::{self, Lib};
use crate::report::{Failure, MissingFootprint, Renamed, Report};

/// Suffix of the pack libraries that hold a footprint library's packages.
pub const FOOTPRINT_LIB_SUFFIX: &str = ".pretty";

/// What to convert and where to.
#[derive(Debug, Clone)]
pub struct Options {
    /// Directory of `.kicad_sym` files.
    pub symbols: PathBuf,
    /// Directory of `.pretty` directories.
    pub footprints: PathBuf,
    /// The map.
    pub map: Map,
    /// Output pack directory.
    pub out: PathBuf,
    /// Worker threads.
    pub jobs: usize,
    /// Free text recorded in the report (e.g. the upstream tag).
    pub upstream: String,
}

/// Runs the conversion. The report lists failures; the caller decides what they mean.
///
/// # Errors
/// Only for problems that stop the whole run (unreadable input directory, unwritable output).
pub fn run(opts: &Options) -> Result<Report, String> {
    let mut report = Report { upstream: opts.upstream.clone(), ..Report::default() };
    let packages = footprints(opts, &mut report)?;
    let index = FootprintIndex::new(packages.iter().map(|(entry, _)| entry.clone()).collect());
    let mut drafts = Vec::new();
    for result in inputs::convert_symbols(&opts.symbols, &opts.map, &index, opts.jobs)? {
        match result {
            Ok(r) => {
                report.counts.symbol_libraries += 1;
                report.counts.symbols += r.symbols;
                report.counts.derived_symbols += r.derived;
                report.notes.merge(&r.notes);
                merge_unknown(&mut report.kicad_unknown, &r.unknown);
                report.failures.extend(r.failures);
                if !opts.map.libraries.contains_key(&r.lib) {
                    report.unmapped_libraries.push(r.lib.clone());
                }
                drafts.extend(r.drafts);
            }
            Err(f) => report.failures.push(f),
        }
    }
    report.unused_map_devices = unused_map_devices(&opts.map, &drafts);
    let devices = finish_devices(drafts, &mut report);
    let packages: Vec<Package> = packages.into_iter().map(|(_, p)| p).collect();
    let library = validated(devices, packages, &mut report);
    count(&library, &mut report);
    report.counts.alternates_dropped = report.notes.get("pin alternates dropped (primary function kept)");
    report.counts.kicad_unknown_tokens = report.kicad_unknown.values().map(|&v| v as u64).sum();
    pack::prepare(&opts.out)?;
    pack::write(&opts.out, &libs(&library))?;
    if let Err(e) = Library::load_dir(&opts.out) {
        report.failures.push(Failure { what: "pack".into(), error: format!("the written pack does not load: {e}") });
    }
    report.failures.sort();
    Ok(report)
}

/// Converts the footprints; package names are the footprint names, or `Library:Name` when a
/// name occurs in more than one library.
fn footprints(opts: &Options, report: &mut Report) -> Result<Vec<(FpEntry, Package)>, String> {
    let (lib_count, files) = inputs::footprint_files(&opts.footprints)?;
    report.counts.footprint_libraries = lib_count;
    report.counts.footprints = files.len();
    let mut ok: Vec<FpResult> = Vec::new();
    for r in inputs::convert_footprints(files, opts.jobs)? {
        match r {
            Ok(r) => ok.push(r),
            Err(f) => report.failures.push(f),
        }
    }
    let mut uses: HashMap<String, usize> = HashMap::new();
    for r in &ok {
        *uses.entry(r.file.name.clone()).or_default() += 1;
    }
    let mut out = Vec::with_capacity(ok.len());
    for mut r in ok {
        report.notes.merge(&r.notes);
        merge_unknown(&mut report.kicad_unknown, &r.unknown);
        if uses.get(&r.file.name).is_some_and(|&n| n > 1) {
            r.package.name = format!("{}:{}", r.file.lib, r.file.name);
            report.notes.note("footprint names used in several libraries (package named Library:Name)");
        }
        r.package.library = format!("{}{FOOTPRINT_LIB_SUFFIX}", r.file.lib);
        let pads: HashSet<String> = r.package.pads.iter().map(|p| p.name.clone()).collect();
        let tht = r.package.pads.iter().any(|p| !p.name.is_empty() && p.drill.is_some());
        out.push((FpEntry { lib: r.file.lib, name: r.file.name, package: r.package.name.clone(), pads, tht }, r.package));
    }
    Ok(out)
}

/// Assigns unique names, records renames and link statistics, and sorts the devices by
/// library, then name.
fn finish_devices(drafts: Vec<Draft>, report: &mut Report) -> Vec<Device> {
    let requests: Vec<Request<'_>> = drafts.iter().map(|d| Request { lib: &d.lib, symbol: &d.symbol, mapped: d.mapped_name.as_deref() }).collect();
    let names = names::assign(&requests);
    let mut devices = Vec::with_capacity(drafts.len());
    for (mut d, name) in drafts.into_iter().zip(names) {
        if d.mapped_name.is_some() {
            report.counts.mapped_names += 1;
        } else if name != d.symbol {
            report.counts.disambiguated += 1;
        }
        if name != d.symbol {
            report.renamed.push(Renamed { lib: d.lib.clone(), symbol: d.symbol.clone(), name: name.clone() });
        }
        if d.links.field_missing {
            report.footprint_links_not_found.push(MissingFootprint {
                lib: d.lib.clone(),
                symbol: d.symbol.clone(),
                footprint: d.links.field.clone(),
            });
        }
        if let Some(problem) = &d.links.map_problem {
            report.failures.push(Failure { what: format!("map.toml/{}:{}", d.lib, d.symbol), error: problem.clone() });
        }
        report.counts.footprint_field_links += usize::from(d.links.field_linked);
        report.counts.map_default_links += usize::from(d.links.map_linked);
        report.counts.filter_links += d.links.filter_links;
        report.counts.filter_capped_devices += usize::from(d.links.capped);
        d.device.name = name;
        devices.push(d.device);
    }
    report.renamed.sort();
    report.footprint_links_not_found.sort();
    devices.sort_by(|a, b| {
        (a.library.as_str(), a.name.to_lowercase(), a.name.as_str()).cmp(&(b.library.as_str(), b.name.to_lowercase(), b.name.as_str()))
    });
    devices
}

/// A validated library. Items the validator rejects are dropped and reported as failures
/// (repeated, since dropping a package can leave a device pointing at nothing).
fn validated(mut devices: Vec<Device>, mut packages: Vec<Package>, report: &mut Report) -> Library {
    packages.sort_by(|a, b| (a.library.as_str(), a.name.as_str()).cmp(&(b.library.as_str(), b.name.as_str())));
    for _ in 0..4 {
        match Library::from_parts(devices.clone(), packages.clone()) {
            Ok(lib) => return lib,
            Err(LibraryError::Invalid(issues)) => {
                let bad: HashSet<(String, String)> = issues.iter().map(|i| (i.library.clone(), i.item.clone().unwrap_or_default())).collect();
                for i in &issues {
                    report.failures.push(Failure { what: format!("{}/{}", i.library, i.item.as_deref().unwrap_or("")), error: i.kind.to_string() });
                }
                devices.retain(|d| !bad.contains(&(d.library.clone(), d.name.clone())));
                packages.retain(|p| !bad.contains(&(p.library.clone(), p.name.clone())));
            }
            Err(e) => {
                report.failures.push(Failure { what: "library".into(), error: e.to_string() });
                break;
            }
        }
    }
    Library::default()
}

fn count(lib: &Library, report: &mut Report) {
    let c = &mut report.counts;
    c.devices = lib.devices().len();
    c.power_devices = lib.devices().iter().filter(|d| d.symbol.power).count();
    c.units = lib.devices().iter().map(|d| d.symbol.units.len()).sum();
    c.pins = lib.devices().iter().map(|d| d.symbol.pins().count()).sum();
    c.devices_with_packages = lib.devices().iter().filter(|d| !d.packages.is_empty()).count();
    c.package_links = lib.devices().iter().map(|d| d.packages.len()).sum();
    c.packages = lib.package_count();
    c.pads = lib.packages().map(|p| p.pads.len()).sum();
}

/// The libraries in pack order, each with its items.
fn libs(lib: &Library) -> Vec<Lib<'_>> {
    let mut devices: BTreeMap<&str, Vec<&Device>> = BTreeMap::new();
    for d in lib.devices() {
        devices.entry(d.library.as_str()).or_default().push(d);
    }
    let mut packages: BTreeMap<&str, Vec<&Package>> = BTreeMap::new();
    for p in lib.packages() {
        packages.entry(p.library.as_str()).or_default().push(p);
    }
    let symbol_libs = devices.into_iter().map(|(name, items)| Lib::Devices(name, items));
    symbol_libs.chain(packages.into_iter().map(|(name, items)| Lib::Packages(name, items))).collect()
}

fn unused_map_devices(map: &Map, drafts: &[Draft]) -> Vec<String> {
    let present: HashSet<String> = drafts.iter().map(|d| format!("{}:{}", d.lib, d.symbol)).collect();
    map.devices.keys().filter(|k| !present.contains(*k)).cloned().collect()
}

fn merge_unknown(into: &mut BTreeMap<String, usize>, from: &BTreeMap<String, usize>) {
    for (k, v) in from {
        *into.entry(k.clone()).or_default() += v;
    }
}
