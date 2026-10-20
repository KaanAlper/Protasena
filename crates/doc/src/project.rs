//! The project: sheets, design properties and the id counter.

use std::collections::HashSet;

use crate::error::DocError;
use crate::ids::{ComponentId, Id, ItemId, SheetId};
use crate::item::Item;
use crate::items::Component;
use crate::sheet::{Sheet, SheetSize};

/// Name a new project's first sheet gets (Proteus's default still has to be observed).
pub const FIRST_SHEET_NAME: &str = "Root sheet 1";

/// The fields of Proteus's design properties.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct DesignProperties {
    /// Design title.
    pub title: String,
    /// Document number.
    pub doc_number: String,
    /// Revision.
    pub revision: String,
    /// Author.
    pub author: String,
    /// Creation date and time (as written by the application).
    pub created: String,
    /// Last modification date and time.
    pub modified: String,
}

/// A design: one or more sheets plus project-wide properties.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct Project {
    /// Project name.
    pub name: String,
    /// Design properties.
    pub design: DesignProperties,
    /// The sheets in order.
    pub sheets: Vec<Sheet>,
    /// The next raw id [`Project::fresh_id`] tries. Ids are never reused within a project.
    pub next_id: u64,
}

impl Project {
    /// A project with one empty A4 sheet.
    pub fn new(name: &str) -> Self {
        let sheet = Sheet::new(SheetId(1), FIRST_SHEET_NAME, SheetSize::A4);
        Self { name: name.to_owned(), design: DesignProperties::default(), sheets: vec![sheet], next_id: 2 }
    }

    /// A new id of any kind, never used before in this project.
    ///
    /// # Errors
    /// [`DocError::IdsExhausted`] when the counter has reached `u64::MAX`.
    pub fn fresh_id<I: Id>(&mut self) -> Result<I, DocError> {
        let used = self.used_ids();
        let mut raw = self.next_id.max(1);
        while used.contains(&raw) {
            raw = raw.checked_add(1).ok_or(DocError::IdsExhausted)?;
        }
        self.next_id = raw.checked_add(1).ok_or(DocError::IdsExhausted)?;
        Ok(I::from_raw(raw))
    }

    /// Makes sure [`Project::fresh_id`] never hands out `raw` (called when an id is added).
    pub(crate) fn reserve(&mut self, raw: u64) {
        self.next_id = self.next_id.max(raw.saturating_add(1));
    }

    /// Every raw id in use (sheets and items).
    pub(crate) fn used_ids(&self) -> HashSet<u64> {
        self.sheets.iter().flat_map(|s| std::iter::once(s.id.0).chain(s.item_ids().map(ItemId::raw))).collect()
    }

    /// The sheet with this id.
    pub fn sheet(&self, id: SheetId) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.id == id)
    }

    pub(crate) fn sheet_index(&self, id: SheetId) -> Option<usize> {
        self.sheets.iter().position(|s| s.id == id)
    }

    /// The sheet holding an item.
    pub fn sheet_of(&self, id: ItemId) -> Option<SheetId> {
        self.sheets.iter().find(|s| s.index_of(id).is_some()).map(|s| s.id)
    }

    /// A copy of an item.
    pub fn item(&self, id: ItemId) -> Option<Item> {
        self.sheets.iter().find_map(|s| s.item(id))
    }

    /// A component.
    pub fn component(&self, id: ComponentId) -> Option<&Component> {
        self.sheets.iter().flat_map(|s| &s.components).find(|c| c.id == id)
    }

    /// Every component with the sheet it is on, in sheet order.
    pub fn components(&self) -> impl Iterator<Item = (&Sheet, &Component)> {
        self.sheets.iter().flat_map(|s| s.components.iter().map(move |c| (s, c)))
    }

    /// Problems of a project that was not built through operations (for example one read from a
    /// file): duplicate ids, non-finite coordinates, malformed items, an id counter that would
    /// hand out a used id. Empty for a consistent project.
    pub fn issues(&self) -> Vec<DocError> {
        let mut issues = Vec::new();
        let mut seen = HashSet::new();
        let mut max_raw = 0;
        for sheet in &self.sheets {
            max_raw = max_raw.max(sheet.id.0);
            if !seen.insert(sheet.id.0) {
                issues.push(DocError::DuplicateSheet(sheet.id));
            }
            if let Err(e) = sheet.size.check() {
                issues.push(e);
            }
            for id in sheet.item_ids() {
                max_raw = max_raw.max(id.raw());
                if !seen.insert(id.raw()) {
                    issues.push(DocError::DuplicateItem(id));
                }
            }
            let checks = sheet.item_ids().filter_map(|id| sheet.item(id)).filter_map(|item| item.check().err());
            issues.extend(checks);
        }
        if !self.sheets.is_empty() && self.next_id <= max_raw {
            issues.push(DocError::Invalid(format!("next id {} is not above the largest id {max_raw}", self.next_id)));
        }
        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::WireId;
    use crate::items::{DeviceRef, Wire};
    use protasena_geom::Pt;

    #[test]
    fn new_project_has_one_sheet_and_fresh_ids_skip_used_ones() {
        let mut p = Project::new("demo");
        assert_eq!(p.sheets.len(), 1);
        assert!(p.issues().is_empty());
        let a: ComponentId = p.fresh_id().unwrap();
        let b: WireId = p.fresh_id().unwrap();
        assert_eq!((a.0, b.0), (2, 3));
        // A hand-built project with a stale counter still gets unused ids.
        p.sheets[0].components.push(Component::new(ComponentId(4), DeviceRef::new("", "RES"), "R1", ""));
        p.next_id = 1;
        assert_eq!(p.issues().len(), 1);
        let c: ComponentId = p.fresh_id().unwrap();
        assert!(c.0 != 1 && c.0 != 4);
    }

    #[test]
    fn exhausted_counter_is_an_error() {
        let mut p = Project::new("x");
        p.next_id = u64::MAX;
        assert_eq!(p.fresh_id::<WireId>(), Err(DocError::IdsExhausted));
    }

    #[test]
    fn issues_report_duplicates_and_bad_geometry() {
        let mut p = Project::new("x");
        let w = Wire { id: WireId(5), points: vec![Pt::ORIGIN, Pt::new(f64::INFINITY, 0.0)] };
        p.sheets[0].wires.push(w.clone());
        p.sheets[0].wires.push(w);
        p.sheets[0].size = SheetSize::Custom { width: -1.0, height: 10.0 };
        p.next_id = 6;
        let issues = p.issues();
        assert!(issues.contains(&DocError::DuplicateItem(ItemId::Wire(WireId(5)))));
        assert!(issues.contains(&DocError::NonFinite("wire point")));
        assert!(issues.iter().any(|e| matches!(e, DocError::Invalid(m) if m.contains("sheet size"))));
    }
}
