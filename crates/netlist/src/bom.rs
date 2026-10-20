//! The bill of materials: the design's parts grouped by value, device and package, each group
//! with its references in natural order (`R2` before `R10`) and its quantity.
//!
//! Built the conventional way (Proteus's report could not be observed): every sheet counts,
//! power symbols are left out, the gates of one part are one part (they share the reference),
//! and parts that are not annotated yet (`R?`) are counted one by one. The package is the
//! part's own, else its device's default. Groups are listed in the natural order of their first
//! reference.

use std::collections::{BTreeMap, BTreeSet};

use protasena_doc::{Project, split_reference};
use serde::{Deserialize, Serialize};

use crate::order::natural_cmp;
use crate::source::DeviceSource;

/// One line of the bill of materials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BomLine {
    /// How many parts.
    pub quantity: usize,
    /// Their references, in natural order.
    pub references: Vec<String>,
    /// Value.
    pub value: String,
    /// Device name.
    pub device: String,
    /// PCB package (empty: none).
    pub package: String,
    /// The device's description.
    pub description: String,
}

/// The bill of materials of a design.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Bom {
    /// The design's name.
    pub design: String,
    /// The lines.
    pub lines: Vec<BomLine>,
}

impl Bom {
    /// How many parts in all.
    pub fn total(&self) -> usize {
        self.lines.iter().map(|l| l.quantity).sum()
    }
}

/// The bill of materials of `project` (see the module docs).
pub fn bill_of_materials<S: DeviceSource + ?Sized>(project: &Project, devices: &S) -> Bom {
    // (value, device, package) -> references, and the description of the group.
    let mut groups: BTreeMap<(String, String, String), (BTreeSet<String>, Vec<String>, String)> = BTreeMap::new();
    for (_, c) in project.components() {
        let device = devices.device(&c.device);
        if device.is_some_and(|d| d.symbol.power) {
            continue;
        }
        let package = c.package.clone().or_else(|| device.and_then(|d| d.default_package.clone())).unwrap_or_default();
        let key = (c.value.trim().to_owned(), c.device.name.trim().to_owned(), package);
        let entry = groups.entry(key).or_default();
        let reference = c.reference.trim().to_owned();
        let annotated = matches!(split_reference(&reference), Some((_, Some(_))));
        if annotated {
            entry.0.insert(reference);
        } else {
            entry.1.push(reference);
        }
        if entry.2.is_empty() {
            entry.2 = device.map(|d| d.description.clone()).unwrap_or_default();
        }
    }
    let mut lines: Vec<BomLine> = groups
        .into_iter()
        .map(|((value, device, package), (annotated, mut loose, description))| {
            let mut references: Vec<String> = annotated.into_iter().collect();
            references.sort_by(|a, b| natural_cmp(a, b));
            loose.sort_by(|a, b| natural_cmp(a, b));
            references.extend(loose);
            BomLine { quantity: references.len(), references, value, device, package, description }
        })
        .collect();
    lines.sort_by(|a, b| match (a.references.first(), b.references.first()) {
        (Some(x), Some(y)) => natural_cmp(x, y).then_with(|| natural_cmp(&a.value, &b.value)),
        _ => std::cmp::Ordering::Equal,
    });
    Bom { design: project.name.clone(), lines }
}
