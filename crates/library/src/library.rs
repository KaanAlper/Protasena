//! The in-memory library: lookup by name, Pick Devices search and the category tree.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::device::Device;
use crate::error::{Issue, LibraryError};
use crate::package::Package;
use crate::store::Packages;
use crate::validate::{Validator, validate};

/// A validated set of devices and packages from one or more pack libraries.
///
/// Packages of a loaded pack are read from their files on first use ([`Library::package`]).
#[derive(Debug, Clone, Default)]
pub struct Library {
    pub(crate) devices: Vec<Device>,
    pub(crate) packages: Packages,
    /// Upper-case device name → index into `devices`.
    by_name: HashMap<String, usize>,
    /// Lower-case name, description and keywords per device, joined by newlines.
    haystacks: Vec<String>,
    libraries: Vec<LibraryInfo>,
    /// The libraries [`Library::with_extra`] added.
    pub(crate) extra_libraries: Vec<String>,
}

/// A pack library and how much it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryInfo {
    /// Library name.
    pub name: String,
    /// Devices in it.
    pub device_count: usize,
    /// Packages in it.
    pub package_count: usize,
}

/// A Pick Devices query. Empty fields match everything; text compares without case.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    /// Whitespace-separated keywords; a device matches when every keyword occurs in its name,
    /// description or keywords.
    pub keywords: String,
    /// Category filter (whole value).
    pub category: Option<String>,
    /// Sub-category filter (whole value).
    pub sub_category: Option<String>,
    /// Manufacturer filter (whole value).
    pub manufacturer: Option<String>,
}

impl Query {
    /// A keyword query.
    pub fn keywords(keywords: &str) -> Self {
        Self { keywords: keywords.to_owned(), ..Self::default() }
    }

    /// The same query restricted to a category.
    pub fn category(self, category: &str) -> Self {
        Self { category: Some(category.to_owned()), ..self }
    }

    /// The same query restricted to a sub-category.
    pub fn sub_category(self, sub_category: &str) -> Self {
        Self { sub_category: Some(sub_category.to_owned()), ..self }
    }

    /// The same query restricted to a manufacturer.
    pub fn manufacturer(self, manufacturer: &str) -> Self {
        Self { manufacturer: Some(manufacturer.to_owned()), ..self }
    }
}

/// A category with its device count and sub-categories (both sorted by name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    /// Category name.
    pub name: String,
    /// Devices in the category.
    pub count: usize,
    /// Non-empty sub-categories with their device counts.
    pub sub_categories: Vec<(String, usize)>,
}

impl Library {
    /// Builds a library after checking the data.
    ///
    /// Library order is the order in which library names first appear in `devices`, then in
    /// `packages`; devices and packages are grouped by library in that order (keeping their
    /// order within a library), which is also the order a saved pack loads back in.
    ///
    /// # Errors
    /// [`LibraryError::Invalid`] listing every problem found.
    pub fn from_parts(devices: Vec<Device>, packages: Vec<Package>) -> Result<Library, LibraryError> {
        let issues = validate(&devices, &packages);
        if issues.is_empty() { Ok(Self::index(devices, packages)) } else { Err(LibraryError::Invalid(issues)) }
    }

    /// Builds the lookup tables without validating (the caller has checked the data).
    pub(crate) fn index(devices: Vec<Device>, mut packages: Vec<Package>) -> Library {
        let libraries = library_infos(&devices, packages.iter().map(|p| p.library.as_str()));
        let order = rank(&libraries);
        packages.sort_by_key(|p| order(&p.library));
        Self::assemble(devices, Packages::from_vec(packages))
    }

    /// Builds the lookup tables over checked devices and a package store.
    pub(crate) fn assemble(mut devices: Vec<Device>, mut packages: Packages) -> Library {
        let libraries = library_infos(&devices, packages.names().map(|(_, lib)| lib));
        // Group by library (stable within a library), as the pack stores them.
        let order = rank(&libraries);
        devices.sort_by_key(|d| order(&d.library));
        packages.sort_by_library(&order);
        // A name repeated in another library (see `with_extra`) is found in the first one.
        let mut by_name = HashMap::with_capacity(devices.len());
        for (i, d) in devices.iter().enumerate() {
            by_name.entry(d.name.to_uppercase()).or_insert(i);
        }
        let haystacks = devices.iter().map(haystack).collect();
        Library { devices, packages, by_name, haystacks, libraries, extra_libraries: Vec::new() }
    }

    /// This library with `top` laid over it: `top` wins name clashes. A device of `self` is
    /// left out when `top` has a device of the same name (without case), and a package when
    /// `top` has one of the same name, together with the devices of `self` that use it (their
    /// pin maps were made for the replaced package). Libraries of `self` come first.
    ///
    /// Both libraries being consistent, so is the result.
    pub fn overlay(self, top: Library) -> Library {
        let top_devices: HashSet<String> = top.devices.iter().map(|d| d.name.to_uppercase()).collect();
        let top_packages: HashSet<String> = top.packages.names().map(|(n, _)| n.to_owned()).collect();
        let kept = self.devices.into_iter().filter(|d| {
            let uses_replaced =
                d.packages.iter().any(|r| top_packages.contains(&r.name)) || d.default_package.as_ref().is_some_and(|p| top_packages.contains(p));
            !top_devices.contains(&d.name.to_uppercase()) && !uses_replaced
        });
        let devices = kept.chain(top.devices).collect();
        Self::assemble(devices, self.packages.chain(top.packages))
    }

    /// Re-runs validation (empty when the library is consistent).
    pub fn issues(&self) -> Vec<Issue> {
        let mut v = Validator::default();
        v.packages(self.packages.all());
        v.devices(&self.devices);
        v.finish()
    }

    /// The device with this name, ignoring case.
    pub fn get(&self, name: &str) -> Option<&Device> {
        self.by_name.get(&name.to_uppercase()).and_then(|&i| self.devices.get(i))
    }

    /// The package with this name. A loaded pack reads the package's file on first use; `None`
    /// also when that file can no longer be read ([`Library::package_failures`]).
    pub fn package(&self, name: &str) -> Option<&Package> {
        self.packages.get(name)
    }

    /// Package names with their library, in pack order (nothing is read).
    pub fn package_names(&self) -> impl Iterator<Item = (&str, &str)> {
        self.packages.names()
    }

    /// Number of packages.
    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    /// Package libraries whose file could not be read again, with the reason.
    pub fn package_failures(&self) -> Vec<(String, String)> {
        self.packages.failures()
    }

    /// All devices, in pack order.
    pub fn devices(&self) -> &[Device] {
        &self.devices
    }

    /// All packages, in pack order (reading every package file not read yet).
    pub fn packages(&self) -> impl Iterator<Item = &Package> {
        self.packages.all()
    }

    /// The pack libraries.
    pub fn libraries(&self) -> &[LibraryInfo] {
        &self.libraries
    }

    /// Number of devices.
    pub fn len(&self) -> usize {
        self.devices.len()
    }

    /// Whether there are no devices.
    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }

    /// Devices matching a Pick Devices query, sorted by name (then library).
    pub fn search(&self, query: &Query) -> Vec<&Device> {
        let words: Vec<String> = query.keywords.split_whitespace().map(str::to_lowercase).collect();
        let lower = |f: &Option<String>| f.as_deref().map(str::to_lowercase);
        let (category, sub_category, manufacturer) = (lower(&query.category), lower(&query.sub_category), lower(&query.manufacturer));
        let field = |filter: &Option<String>, value: &str| filter.as_deref().is_none_or(|f| f == value.to_lowercase());
        let mut hits: Vec<&Device> = self
            .devices
            .iter()
            .zip(&self.haystacks)
            .filter(|(d, hay)| {
                field(&category, &d.category)
                    && field(&sub_category, &d.sub_category)
                    && field(&manufacturer, &d.manufacturer)
                    && words.iter().all(|w| hay.contains(w.as_str()))
            })
            .map(|(d, _)| d)
            .collect();
        hits.sort_by_cached_key(|d| (d.name.to_lowercase(), d.library.clone()));
        hits
    }

    /// The category tree with device counts, sorted by name.
    pub fn categories(&self) -> Vec<Category> {
        let mut tree: BTreeMap<&str, (usize, BTreeMap<&str, usize>)> = BTreeMap::new();
        for d in &self.devices {
            let (count, subs) = tree.entry(d.category.as_str()).or_default();
            *count += 1;
            if !d.sub_category.is_empty() {
                *subs.entry(d.sub_category.as_str()).or_default() += 1;
            }
        }
        tree.into_iter()
            .map(|(name, (count, subs))| Category {
                name: name.to_owned(),
                count,
                sub_categories: subs.into_iter().map(|(s, n)| (s.to_owned(), n)).collect(),
            })
            .collect()
    }
}

/// Position of a library name in `libraries` (unknown names last).
fn rank(libraries: &[LibraryInfo]) -> impl Fn(&str) -> usize + use<> {
    let slot: HashMap<String, usize> = libraries.iter().enumerate().map(|(i, l)| (l.name.clone(), i)).collect();
    move |lib| slot.get(lib).copied().unwrap_or(usize::MAX)
}

/// The libraries in order of first appearance (devices, then packages) with their counts.
fn library_infos<'a>(devices: &'a [Device], package_libs: impl Iterator<Item = &'a str>) -> Vec<LibraryInfo> {
    let mut libraries: Vec<LibraryInfo> = Vec::new();
    let mut slot: HashMap<&str, usize> = HashMap::new();
    let names = devices.iter().map(|d| (d.library.as_str(), true)).chain(package_libs.map(|l| (l, false)));
    for (name, is_device) in names {
        let i = *slot.entry(name).or_insert_with(|| {
            libraries.push(LibraryInfo { name: name.to_owned(), device_count: 0, package_count: 0 });
            libraries.len().saturating_sub(1)
        });
        if let Some(info) = libraries.get_mut(i) {
            if is_device {
                info.device_count += 1;
            } else {
                info.package_count += 1;
            }
        }
    }
    libraries
}

fn haystack(d: &Device) -> String {
    let mut hay = format!("{}\n{}", d.name, d.description);
    for k in &d.keywords {
        hay.push('\n');
        hay.push_str(k);
    }
    hay.to_lowercase()
}
