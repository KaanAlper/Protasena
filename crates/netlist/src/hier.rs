//! Hierarchical design: the scope of names on a sheet instance, and the checks of subcircuits,
//! ports and child sheets.
//!
//! A child sheet is connected once, as the instance of the subcircuit that reaches it first in
//! [`Project::sheet_tree`] order; its local names are prefixed by that instance's path
//! (`SUB1.DATA`). Global names are never prefixed: power and ground terminals, power symbols,
//! hidden power pins and labels marked global.

use std::collections::{BTreeMap, HashMap, HashSet};

use protasena_doc::{Project, SheetNode, TerminalKind, instance_name, join_path};

use crate::model::{ErcItem, ErcKind, Severity};

/// How names on one sheet instance are scoped.
#[derive(Debug, Clone, Default)]
pub(crate) struct Scope {
    /// The instance path (empty on a root sheet).
    pub path: String,
    /// The parent subcircuit's ports by name, with their elements (child sheets only).
    pub ports: HashMap<String, usize>,
}

impl Scope {
    /// The net name `name` stands for here: global names as they are, local names with the
    /// instance path in front.
    pub fn name(&self, name: &str, global: bool) -> String {
        let name = name.trim();
        if global || self.path.is_empty() || name.is_empty() { name.to_owned() } else { join_path(&self.path, name) }
    }

    /// The parent's port a terminal of `kind` named `name` joins, if any (input, output and
    /// bidirectional terminals join the port of their name).
    pub fn port(&self, kind: TerminalKind, name: &str) -> Option<usize> {
        matches!(kind, TerminalKind::Input | TerminalKind::Output | TerminalKind::Bidir).then(|| self.ports.get(name.trim()).copied()).flatten()
    }
}

/// Whether a terminal kind connects to subcircuit ports.
fn is_port_terminal(kind: TerminalKind) -> bool {
    matches!(kind, TerminalKind::Input | TerminalKind::Output | TerminalKind::Bidir)
}

fn warn(message: String) -> ErcItem {
    ErcItem { severity: Severity::Warning, kind: ErcKind::Hierarchy, net: None, nodes: Vec::new(), message }
}

/// Rules-check findings about the hierarchy: subcircuits without a child sheet or with one that
/// another subcircuit already uses, ports without a terminal on the child sheet, repeated
/// subcircuit and port names.
pub(crate) fn findings(project: &Project, tree: &[SheetNode]) -> Vec<ErcItem> {
    let mut out = Vec::new();
    let owner: HashMap<_, _> = tree.iter().filter_map(|n| n.parent.map(|(_, sub)| (n.sheet, sub))).collect();
    for sheet in &project.sheets {
        let mut names: BTreeMap<String, usize> = BTreeMap::new();
        for sub in &sheet.subcircuits {
            let name = instance_name(sub);
            *names.entry(name.clone()).or_default() += 1;
            let child = sub.child.and_then(|c| project.sheet(c));
            match child {
                None => out.push(warn(format!("subcircuit {name} on sheet {} has no child sheet", sheet.name))),
                Some(c) if owner.get(&c.id) != Some(&sub.id) => {
                    out.push(warn(format!(
                        "subcircuit {name}: sheet {} is already used by another subcircuit; {name} is not connected to it",
                        c.name
                    )));
                }
                Some(c) => {
                    let terminals: HashSet<&str> = c.terminals.iter().filter(|t| is_port_terminal(t.kind)).map(|t| t.name.trim()).collect();
                    for port in sub.ports.iter().filter(|p| !terminals.contains(p.name.trim())) {
                        let p = port.name.trim();
                        out.push(warn(format!("port {p} of {name} has no input, output or bidirectional terminal {p} on sheet {}", c.name)));
                    }
                }
            }
            let mut seen = HashSet::new();
            for port in &sub.ports {
                if !seen.insert(port.name.trim()) {
                    out.push(warn(format!("{name} has two ports called {}", port.name.trim())));
                }
            }
        }
        for (name, n) in names.into_iter().filter(|(_, n)| *n > 1) {
            out.push(warn(format!("{n} subcircuits on sheet {} are called {name}", sheet.name)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_names_take_the_path_and_global_ones_do_not() {
        let s = Scope { path: "SUB1.SUB2".into(), ports: HashMap::from([("IN".to_owned(), 7)]) };
        assert_eq!(s.name(" DATA ", false), "SUB1.SUB2.DATA");
        assert_eq!(s.name("VCC", true), "VCC");
        assert_eq!(s.name("", false), "");
        assert_eq!(Scope::default().name("DATA", false), "DATA");
        assert_eq!(s.port(TerminalKind::Input, " IN"), Some(7));
        assert_eq!(s.port(TerminalKind::Default, "IN"), None, "default terminals are local names");
        assert_eq!(s.port(TerminalKind::Output, "OUT"), None);
    }
}
