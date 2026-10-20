//! Pick Devices' search state, kept apart from the drawing so it can be tested: keywords and the
//! category, sub-category and manufacturer filters, the lists with their counts and the results.
//!
//! Counts follow the filters to the left of each list: categories count the devices matching the
//! keywords, sub-categories those also in the chosen category, manufacturers those also in the
//! chosen sub-category. Nothing is listed until there are keywords or a category (to compare
//! with Proteus, whose dialog was not observable yet).

use std::collections::{BTreeMap, HashMap};

use protasena_library::{Device, Library, Query};

/// Most results kept (the table scrolls through them).
pub const MAX_RESULTS: usize = 20_000;

/// One filter list: names with device counts, sorted by name.
pub type Counts = Vec<(String, usize)>;

/// The search and what it found.
#[derive(Debug, Clone, Default)]
pub struct PickModel {
    /// Keywords, as typed.
    pub keywords: String,
    /// Chosen category.
    pub category: Option<String>,
    /// Chosen sub-category.
    pub sub_category: Option<String>,
    /// Chosen manufacturer.
    pub manufacturer: Option<String>,
    /// The category list.
    pub categories: Counts,
    /// The sub-category list.
    pub sub_categories: Counts,
    /// The manufacturer list.
    pub manufacturers: Counts,
    /// Matching devices (indices into the library's devices), sorted by name.
    pub results: Vec<usize>,
    /// How many devices matched in all (results may be capped).
    pub total: usize,
    /// The highlighted result (an index into the library's devices).
    pub selected: Option<usize>,
    computed: Option<(String, Option<String>, Option<String>, Option<String>, usize)>,
}

impl PickModel {
    /// Recomputes the lists and results when the query or the library changed.
    pub fn refresh(&mut self, library: &Library) {
        let key = (self.keywords.clone(), self.category.clone(), self.sub_category.clone(), self.manufacturer.clone(), library.len());
        if self.computed.as_ref() == Some(&key) {
            return;
        }
        let index: HashMap<usize, usize> = library.devices().iter().enumerate().map(|(i, d)| (addr(d), i)).collect();
        let words = Query::keywords(self.keywords.trim());
        let by_keywords = library.search(&words);
        self.categories = counts(by_keywords.iter().map(|d| d.category.as_str()));
        let in_category = |d: &&&Device| self.category.as_deref().is_none_or(|c| d.category.eq_ignore_ascii_case(c));
        self.sub_categories = counts(by_keywords.iter().filter(in_category).map(|d| d.sub_category.as_str()));
        let in_sub = |d: &&&Device| self.sub_category.as_deref().is_none_or(|c| d.sub_category.eq_ignore_ascii_case(c));
        self.manufacturers = counts(by_keywords.iter().filter(in_category).filter(in_sub).map(|d| d.manufacturer.as_str()));
        let anything = !self.keywords.trim().is_empty() || self.category.is_some();
        let in_maker = |d: &&&Device| self.manufacturer.as_deref().is_none_or(|m| d.manufacturer.eq_ignore_ascii_case(m));
        let hits: Vec<usize> = if anything {
            by_keywords.iter().filter(in_category).filter(in_sub).filter(in_maker).filter_map(|d| index.get(&addr(d)).copied()).collect()
        } else {
            Vec::new()
        };
        let mut hits = hits;
        let word = self.keywords.trim().to_lowercase();
        if !word.is_empty() {
            // Names equal to the keywords first, then names starting with them, then the rest
            // (each in name order, as the search returned them).
            let rank = |i: &usize| {
                let n = library.devices().get(*i).map(|d| d.name.to_lowercase()).unwrap_or_default();
                if n == word {
                    0
                } else if n.starts_with(&word) {
                    1
                } else {
                    2
                }
            };
            hits.sort_by_key(rank);
        }
        self.total = hits.len();
        self.results = hits.into_iter().take(MAX_RESULTS).collect();
        if self.selected.is_none_or(|s| !self.results.contains(&s)) {
            self.selected = self.results.first().copied();
        }
        self.computed = Some(key);
    }

    /// Chooses a category (`None`: all); the narrower filters start over.
    pub fn set_category(&mut self, c: Option<String>) {
        self.category = c;
        self.sub_category = None;
        self.manufacturer = None;
    }

    /// Chooses a sub-category; the manufacturer starts over.
    pub fn set_sub_category(&mut self, c: Option<String>) {
        self.sub_category = c;
        self.manufacturer = None;
    }

    /// The highlighted device.
    pub fn device<'a>(&self, library: &'a Library) -> Option<&'a Device> {
        self.selected.and_then(|i| library.devices().get(i))
    }

    /// Moves the highlight by `step` rows (arrow keys).
    pub fn step(&mut self, step: isize) {
        let at = self.selected.and_then(|s| self.results.iter().position(|&r| r == s)).unwrap_or(0);
        let next = at.saturating_add_signed(step).min(self.results.len().saturating_sub(1));
        self.selected = self.results.get(next).copied();
    }
}

/// A device's address, to find its index again among the library's devices.
fn addr(d: &Device) -> usize {
    std::ptr::from_ref(d) as usize
}

/// Names with counts, sorted by name; empty names are left out.
fn counts<'a>(names: impl Iterator<Item = &'a str>) -> Counts {
    let mut m: BTreeMap<&str, usize> = BTreeMap::new();
    for n in names.filter(|n| !n.trim().is_empty()) {
        *m.entry(n).or_default() += 1;
    }
    m.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(m: &PickModel, lib: &Library) -> Vec<String> {
        m.results.iter().map(|&i| lib.devices()[i].name.clone()).collect()
    }

    #[test]
    fn keywords_find_devices_and_count_categories() {
        let lib = Library::builtin();
        let mut m = PickModel::default();
        m.refresh(&lib);
        assert!(m.results.is_empty(), "nothing is listed before a query");
        assert!(!m.categories.is_empty(), "all categories are offered");
        m.keywords = "cap".into();
        m.refresh(&lib);
        let found = names(&m, &lib);
        assert!(found.contains(&"CAP".to_string()) && found.contains(&"CAP-ELEC".to_string()), "{found:?}");
        assert_eq!(found[0], "CAP", "an exact name comes first");
        assert_eq!(m.categories.iter().map(|c| c.1).sum::<usize>(), m.total);
        assert_eq!(m.device(&lib).map(|d| d.name.as_str()), Some(found[0].as_str()));
    }

    #[test]
    fn a_category_narrows_results_and_resets_narrower_filters() {
        let lib = Library::builtin();
        let mut m = PickModel::default();
        m.set_category(Some("Capacitors".into()));
        m.refresh(&lib);
        assert!(names(&m, &lib).iter().all(|n| n.starts_with("CAP")));
        let sub = m.sub_categories.first().map(|s| s.0.clone());
        m.set_sub_category(sub);
        m.manufacturer = Some("x".into());
        m.set_category(None);
        assert_eq!((m.sub_category.as_deref(), m.manufacturer.as_deref()), (None, None));
    }

    #[test]
    fn arrow_keys_move_the_highlight_within_the_results() {
        let lib = Library::builtin();
        let mut m = PickModel { keywords: "r".into(), ..PickModel::default() };
        m.refresh(&lib);
        let first = m.selected;
        m.step(-5);
        assert_eq!(m.selected, first);
        m.step(1_000_000);
        assert_eq!(m.selected, m.results.last().copied());
    }
}
