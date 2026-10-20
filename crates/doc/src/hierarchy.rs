//! The sheet hierarchy: root sheets and the child sheets of their subcircuits, as a tree in
//! depth-first order (the order of Design ▸ Goto Previous/Next Root or Sub-sheet and of the
//! Design menu's sheet list).
//!
//! A sheet is a **root sheet** unless a subcircuit on another sheet names it as its child. A
//! child sheet belongs to the first subcircuit that reaches it in this order; a second
//! subcircuit naming the same sheet, or a loop of subcircuits, does not make it appear twice
//! (sheets only reachable through a loop are listed as roots after the others). The tree goes
//! at most [`MAX_DEPTH`] levels deep.

use std::collections::HashSet;

use crate::ids::{SheetId, SubcircuitId};
use crate::project::Project;
use crate::sheet::Sheet;
use crate::subcircuit::Subcircuit;

/// Deepest level of child sheets followed.
pub const MAX_DEPTH: usize = 64;

/// One sheet in the hierarchy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetNode {
    /// The sheet.
    pub sheet: SheetId,
    /// 0 for a root sheet, 1 for its children, …
    pub depth: usize,
    /// The sheet and subcircuit this one is the child of.
    pub parent: Option<(SheetId, SubcircuitId)>,
    /// Instance path: the subcircuit names from the root down, joined by `.` (empty for a root
    /// sheet). Local net names of the sheet are prefixed with it.
    pub path: String,
}

/// A subcircuit's instance name: its name, or `SUB<id>` when it has none.
pub fn instance_name(sub: &Subcircuit) -> String {
    match sub.name.trim() {
        "" => format!("SUB{}", sub.id.0),
        n => n.to_owned(),
    }
}

/// `path.name`, or `name` at the root.
pub fn join_path(path: &str, name: &str) -> String {
    if path.is_empty() { name.to_owned() } else { format!("{path}.{name}") }
}

impl Project {
    /// Every sheet once, in hierarchy order (see the module docs).
    pub fn sheet_tree(&self) -> Vec<SheetNode> {
        let children: HashSet<SheetId> = self
            .sheets
            .iter()
            .flat_map(|s| s.subcircuits.iter().filter_map(move |x| x.child.filter(|c| *c != s.id)))
            .filter(|c| self.sheet(*c).is_some())
            .collect();
        let mut out = Vec::with_capacity(self.sheets.len());
        let mut seen = HashSet::new();
        for root in self.sheets.iter().filter(|s| !children.contains(&s.id)) {
            self.visit(root, 0, None, String::new(), &mut seen, &mut out);
        }
        for rest in &self.sheets {
            if !seen.contains(&rest.id) {
                self.visit(rest, 0, None, String::new(), &mut seen, &mut out);
            }
        }
        out
    }

    fn visit(
        &self,
        sheet: &Sheet,
        depth: usize,
        parent: Option<(SheetId, SubcircuitId)>,
        path: String,
        seen: &mut HashSet<SheetId>,
        out: &mut Vec<SheetNode>,
    ) {
        if !seen.insert(sheet.id) {
            return;
        }
        out.push(SheetNode { sheet: sheet.id, depth, parent, path: path.clone() });
        if depth >= MAX_DEPTH {
            return;
        }
        for sub in &sheet.subcircuits {
            if let Some(child) = sub.child.and_then(|c| self.sheet(c)).filter(|c| !seen.contains(&c.id)) {
                self.visit(child, depth + 1, Some((sheet.id, sub.id)), join_path(&path, &instance_name(sub)), seen, out);
            }
        }
    }

    /// The sheet and subcircuit `sheet` is the child of, if it is a child sheet.
    pub fn parent_of(&self, sheet: SheetId) -> Option<(SheetId, SubcircuitId)> {
        self.sheet_tree().into_iter().find(|n| n.sheet == sheet).and_then(|n| n.parent)
    }

    /// A subcircuit with the sheet it is on.
    pub fn subcircuit(&self, id: SubcircuitId) -> Option<(&Sheet, &Subcircuit)> {
        self.sheets.iter().find_map(|s| s.subcircuits.iter().find(|x| x.id == id).map(|x| (s, x)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sheet::SheetSize;
    use protasena_geom::Pt;

    fn with_sub(p: &mut Project, on: usize, id: u64, child: Option<u64>, name: &str) {
        let mut s = Subcircuit::new(SubcircuitId(id), Pt::ORIGIN, Pt::new(100.0, 100.0), name);
        s.child = child.map(SheetId);
        p.sheets[on].subcircuits.push(s);
    }

    fn project(sheets: &[u64]) -> Project {
        let mut p = Project::new("h");
        p.sheets = sheets.iter().map(|&i| Sheet::new(SheetId(i), &format!("S{i}"), SheetSize::A4)).collect();
        p.next_id = 1000;
        p
    }

    fn order(p: &Project) -> Vec<(u64, usize, String)> {
        p.sheet_tree().into_iter().map(|n| (n.sheet.0, n.depth, n.path)).collect()
    }

    #[test]
    fn children_follow_their_parent_with_instance_paths() {
        let mut p = project(&[1, 2, 3, 4]);
        with_sub(&mut p, 0, 10, Some(3), "SUB1");
        with_sub(&mut p, 2, 11, Some(4), " ");
        let tree = order(&p);
        assert_eq!(tree, [(1, 0, String::new()), (3, 1, "SUB1".into()), (4, 2, "SUB1.SUB11".into()), (2, 0, String::new())]);
        assert_eq!(p.parent_of(SheetId(4)), Some((SheetId(3), SubcircuitId(11))));
        assert_eq!(p.parent_of(SheetId(2)), None);
    }

    #[test]
    fn loops_shared_children_and_missing_sheets_list_every_sheet_once() {
        let mut p = project(&[1, 2, 3]);
        with_sub(&mut p, 1, 10, Some(3), "A");
        with_sub(&mut p, 2, 11, Some(2), "B");
        with_sub(&mut p, 0, 12, Some(1), "SELF");
        with_sub(&mut p, 0, 13, Some(99), "GONE");
        let tree = order(&p);
        assert_eq!(tree.len(), 3, "{tree:?}");
        assert_eq!(tree[0], (1, 0, String::new()), "a sheet that names itself stays a root");
        assert_eq!((tree[1].0, tree[1].1), (2, 0), "a loop is entered at its first sheet");
        assert_eq!(tree[2], (3, 1, "A".into()));
        with_sub(&mut p, 0, 14, Some(3), "C");
        assert_eq!(p.sheet_tree().len(), 3, "a shared child is listed once");
    }

    #[test]
    fn deep_chains_stop_at_the_limit() {
        let ids: Vec<u64> = (1..=MAX_DEPTH as u64 + 5).collect();
        let mut p = project(&ids);
        for i in 0..ids.len() - 1 {
            with_sub(&mut p, i, 1000 + i as u64, Some(ids[i + 1]), "S");
        }
        let tree = p.sheet_tree();
        assert_eq!(tree.len(), ids.len());
        assert!(tree.iter().all(|n| n.depth <= MAX_DEPTH));
    }
}
