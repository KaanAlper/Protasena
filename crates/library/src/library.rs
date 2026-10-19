//! The in-memory library: lookup by name, Pick Devices search and the category tree.

use std::collections::{BTreeMap, HashMap};

use crate::device::Device;
use crate::error::{Issue, LibraryError};
use crate::package::Package;
use crate::validate::validate;

/// A validated set of devices and packages from one or more pack libraries.
#[derive(Debug, Clone, Default)]
pub struct Library {
    devices: Vec<Device>,
    packages: Vec<Package>,
    /// Upper-case device name → index into `devices`.
    by_name: HashMap<String, usize>,
    package_by_name: HashMap<String, usize>,
    /// Lower-case name, description and keywords per device, joined by newlines.
    haystacks: Vec<String>,
    libraries: Vec<LibraryInfo>,
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
    pub(crate) fn index(mut devices: Vec<Device>, mut packages: Vec<Package>) -> Library {
        let libraries = library_infos(&devices, &packages);
        // Group by library (stable within a library), as the pack stores them.
        let slot: HashMap<&str, usize> = libraries.iter().enumerate().map(|(i, l)| (l.name.as_str(), i)).collect();
        let order = |lib: &str| slot.get(lib).copied().unwrap_or(usize::MAX);
        devices.sort_by_key(|d| order(&d.library));
        packages.sort_by_key(|p| order(&p.library));
        let by_name = devices.iter().enumerate().map(|(i, d)| (d.name.to_uppercase(), i)).collect();
        let package_by_name = packages.iter().enumerate().map(|(i, p)| (p.name.clone(), i)).collect();
        let haystacks = devices.iter().map(haystack).collect();
        Library { devices, packages, by_name, package_by_name, haystacks, libraries }
    }

    /// Re-runs validation (empty when the library is consistent).
    pub fn issues(&self) -> Vec<Issue> {
        validate(&self.devices, &self.packages)
    }

    /// The device with this name, ignoring case.
    pub fn get(&self, name: &str) -> Option<&Device> {
        self.by_name.get(&name.to_uppercase()).and_then(|&i| self.devices.get(i))
    }

    /// The package with this name.
    pub fn package(&self, name: &str) -> Option<&Package> {
        self.package_by_name.get(name).and_then(|&i| self.packages.get(i))
    }

    /// All devices, in pack order.
    pub fn devices(&self) -> &[Device] {
        &self.devices
    }

    /// All packages, in pack order.
    pub fn packages(&self) -> &[Package] {
        &self.packages
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

/// The libraries in order of first appearance (devices, then packages) with their counts.
fn library_infos(devices: &[Device], packages: &[Package]) -> Vec<LibraryInfo> {
    let mut libraries: Vec<LibraryInfo> = Vec::new();
    let mut slot: HashMap<&str, usize> = HashMap::new();
    let names = devices.iter().map(|d| (d.library.as_str(), true)).chain(packages.iter().map(|p| (p.library.as_str(), false)));
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
