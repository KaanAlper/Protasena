//! Annotation: part references `prefix + number` (`R1`, `C12`, `U3`).
//!
//! A reference splits into a prefix and a number: `R12` is (`R`, 12); `R?` and `R` are
//! unannotated parts with prefix `R`; an empty reference (power symbols) takes no part.
//! Annotation never edits the project directly: it returns an [`Op`] (one undo step).

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::ids::ComponentId;
use crate::item::Item;
use crate::items::Component;
use crate::op::Op;
use crate::project::Project;

/// Splits a reference into its prefix and number (`None` when unannotated). `None` overall for a
/// reference without a prefix (empty, or only digits).
pub fn split_reference(reference: &str) -> Option<(&str, Option<u64>)> {
    let r = reference.trim();
    let body = r.strip_suffix('?').unwrap_or(r);
    let prefix = body.trim_end_matches(|c: char| c.is_ascii_digit());
    if prefix.is_empty() {
        return None;
    }
    let digits = body.get(prefix.len()..).unwrap_or("");
    let number = if digits.is_empty() || body.len() != r.len() { None } else { digits.parse().ok() };
    Some((prefix, number))
}

impl Project {
    /// The numbers already used with `prefix` (compared exactly).
    fn used_numbers(&self, prefix: &str) -> HashSet<u64> {
        self.components().filter_map(|(_, c)| split_reference(&c.reference)).filter(|(p, _)| *p == prefix).filter_map(|(_, n)| n).collect()
    }

    /// The next free reference for `prefix`: the smallest number from 1 that no part uses.
    pub fn next_reference(&self, prefix: &str) -> String {
        format!("{prefix}{}", smallest_free(&self.used_numbers(prefix)))
    }

    /// Gives every unannotated part (`R?`) the next free reference of its prefix, in sheet and
    /// list order. Annotated parts keep theirs.
    pub fn annotate(&self) -> Op {
        let mut used: HashMap<String, HashSet<u64>> = HashMap::new();
        let mut ops = Vec::new();
        for (_, c) in self.components() {
            let Some((prefix, None)) = split_reference(&c.reference) else { continue };
            let taken = used.entry(prefix.to_owned()).or_insert_with(|| self.used_numbers(prefix));
            let n = smallest_free(taken);
            taken.insert(n);
            ops.push(renamed(c, format!("{prefix}{n}")));
        }
        Op::batch(ops)
    }

    /// Renumbers every part from 1 per prefix: sheet by sheet, then top to bottom and left to
    /// right by placement origin. Gates sharing an annotated reference keep sharing it.
    pub fn reannotate_all(&self) -> Op {
        // Prefix -> groups in numbering order; a group is the parts sharing one old reference.
        let mut groups: BTreeMap<&str, Vec<Vec<&Component>>> = BTreeMap::new();
        let mut group_of: HashMap<&str, usize> = HashMap::new();
        let mut parts: Vec<(usize, &Component)> =
            self.sheets.iter().enumerate().flat_map(|(i, s)| s.components.iter().map(move |c| (i, c))).collect();
        parts.sort_by(|(sa, a), (sb, b)| {
            let (pa, pb) = (a.origin(), b.origin());
            sa.cmp(sb).then(pb.y.total_cmp(&pa.y)).then(pa.x.total_cmp(&pb.x)).then(a.id.cmp(&b.id))
        });
        for (_, c) in parts {
            let Some((prefix, number)) = split_reference(&c.reference) else { continue };
            let list = groups.entry(prefix).or_default();
            match number.and_then(|_| group_of.get(c.reference.trim())) {
                Some(&g) => {
                    if let Some(group) = list.get_mut(g) {
                        group.push(c);
                    }
                }
                None => {
                    if number.is_some() {
                        group_of.insert(c.reference.trim(), list.len());
                    }
                    list.push(vec![c]);
                }
            }
        }
        let mut ops = Vec::new();
        for (prefix, list) in groups {
            for (n, group) in (1u64..).zip(list) {
                let reference = format!("{prefix}{n}");
                ops.extend(group.into_iter().filter(|c| c.reference != reference).map(|c| renamed(c, reference.clone())));
            }
        }
        sort_by_id(&mut ops);
        Op::batch(ops)
    }
}

fn smallest_free(used: &HashSet<u64>) -> u64 {
    (1u64..).find(|n| !used.contains(n)).unwrap_or(u64::MAX)
}

fn renamed(c: &Component, reference: String) -> Op {
    Op::Modify { item: Item::Component(Component { reference, ..c.clone() }) }
}

fn sort_by_id(ops: &mut [Op]) {
    let key = |op: &Op| match op {
        Op::Modify { item: Item::Component(c) } => c.id,
        _ => ComponentId(u64::MAX),
    };
    ops.sort_by_key(key);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_references() {
        assert_eq!(split_reference("R12"), Some(("R", Some(12))));
        assert_eq!(split_reference("R?"), Some(("R", None)));
        assert_eq!(split_reference("U"), Some(("U", None)));
        assert_eq!(split_reference("R1?"), Some(("R", None)));
        assert_eq!(split_reference(" C3 "), Some(("C", Some(3))));
        assert_eq!(split_reference(""), None);
        assert_eq!(split_reference("123"), None);
        assert_eq!(split_reference("R99999999999999999999999"), Some(("R", None)));
    }
}
