//! Packages, kept in memory or read from their pack file the first time one of them is asked
//! for.
//!
//! A pack's footprints take as much memory as its devices but are only needed for package
//! previews (and, later, PCB layout), so [`crate::Library::load_dir`] checks every package
//! file once, keeps only the package names, and reads a file again when a package of it is
//! first wanted. A file that has become unreadable or invalid by then gives no packages; the
//! reason is kept ([`Packages::failures`]).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::package::Package;

/// Packages by name, grouped in slots (one per library file).
#[derive(Debug, Clone, Default)]
pub(crate) struct Packages {
    slots: Vec<Slot>,
    /// Name → (slot, entry in the slot's `names`).
    by_name: HashMap<String, (usize, usize)>,
}

#[derive(Debug, Clone)]
struct Slot {
    library: String,
    /// The pack file to read on first use; `None` when the packages were given in memory.
    file: Option<PathBuf>,
    /// The names this slot answers for, with their position in the loaded list.
    names: Vec<(String, usize)>,
    loaded: OnceLock<Result<Vec<Package>, String>>,
}

impl Slot {
    fn packages(&self) -> &[Package] {
        let loaded = self.loaded.get_or_init(|| match &self.file {
            Some(file) => crate::pack::load_package_file(file, &self.library),
            None => Ok(Vec::new()),
        });
        loaded.as_deref().unwrap_or_default()
    }
}

impl Packages {
    /// Packages held in memory, one slot per run of the same library.
    pub(crate) fn from_vec(packages: Vec<Package>) -> Self {
        let mut slots: Vec<(String, Vec<Package>)> = Vec::new();
        for p in packages {
            match slots.last_mut() {
                Some((lib, list)) if *lib == p.library => list.push(p),
                _ => slots.push((p.library.clone(), vec![p])),
            }
        }
        let mut out = Packages::default();
        for (library, list) in slots {
            let names = list.iter().enumerate().map(|(i, p)| (p.name.clone(), i)).collect();
            out.slots.push(Slot { library, file: None, names, loaded: OnceLock::from(Ok(list)) });
        }
        out.reindex();
        out
    }

    /// Adds a library file read on first use; `names` are its packages in file order.
    pub(crate) fn add_file(&mut self, library: &str, file: PathBuf, names: Vec<String>) {
        let si = self.slots.len();
        for (ei, n) in names.iter().enumerate() {
            self.by_name.insert(n.clone(), (si, ei));
        }
        let names = names.into_iter().enumerate().map(|(i, n)| (n, i)).collect();
        self.slots.push(Slot { library: library.to_owned(), file: Some(file), names, loaded: OnceLock::new() });
    }

    /// Orders the slots by `rank` of their library (stable).
    pub(crate) fn sort_by_library(&mut self, rank: impl Fn(&str) -> usize) {
        self.slots.sort_by_key(|s| rank(&s.library));
        self.reindex();
    }

    /// The package with this name (reading its file on first use).
    pub(crate) fn get(&self, name: &str) -> Option<&Package> {
        let &(slot, entry) = self.by_name.get(name)?;
        let slot = self.slots.get(slot)?;
        let &(_, pos) = slot.names.get(entry)?;
        let list = slot.packages();
        list.get(pos).filter(|p| p.name == name).or_else(|| list.iter().find(|p| p.name == name))
    }

    /// Every package name with its library, in order.
    pub(crate) fn names(&self) -> impl Iterator<Item = (&str, &str)> {
        self.slots.iter().flat_map(|s| s.names.iter().map(move |(n, _)| (n.as_str(), s.library.as_str())))
    }

    /// Number of packages.
    pub(crate) fn len(&self) -> usize {
        self.slots.iter().map(|s| s.names.len()).sum()
    }

    /// Every package, in order (reading every file not read yet).
    pub(crate) fn all(&self) -> impl Iterator<Item = &Package> {
        self.names().filter_map(|(n, _)| self.get(n))
    }

    /// The same set without the packages named in `drop`.
    pub(crate) fn without(mut self, drop: &HashSet<String>) -> Self {
        for s in &mut self.slots {
            s.names.retain(|(n, _)| !drop.contains(n));
        }
        self.slots.retain(|s| !s.names.is_empty());
        self.reindex();
        self
    }

    /// The same set without the slots of these libraries.
    pub(crate) fn without_libraries(mut self, drop: &HashSet<&str>) -> Self {
        self.slots.retain(|s| !drop.contains(s.library.as_str()));
        self.reindex();
        self
    }

    /// This set followed by `other` (a name in both is answered by `other`).
    pub(crate) fn chain(self, other: Packages) -> Self {
        let shadowed: HashSet<String> = other.by_name.keys().cloned().collect();
        let mut out = self.without(&shadowed);
        out.slots.extend(other.slots);
        out.reindex();
        out
    }

    /// Library files that could not be read again, with the reason.
    pub(crate) fn failures(&self) -> Vec<(String, String)> {
        self.slots
            .iter()
            .filter_map(|s| match s.loaded.get() {
                Some(Err(e)) => Some((s.library.clone(), e.clone())),
                _ => None,
            })
            .collect()
    }

    fn reindex(&mut self) {
        self.by_name.clear();
        for (si, s) in self.slots.iter().enumerate() {
            for (ei, (name, _)) in s.names.iter().enumerate() {
                self.by_name.insert(name.clone(), (si, ei));
            }
        }
    }
}
