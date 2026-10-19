//! `extends`: a derived symbol takes its parent's units and settings and overrides
//! properties. Chains are followed (a parent may itself be derived); cycles and missing
//! parents are errors.

use std::collections::{HashMap, HashSet};

use crate::error::{Error, ErrorKind, Pos, Result};

use super::{Symbol, SymbolLib};

impl SymbolLib {
    /// The symbol `name` with its `extends` chain applied. A symbol that extends nothing is
    /// returned as written.
    pub fn resolve(&self, name: &str) -> Result<Symbol> {
        let index = self.index();
        let Some(start) = index.get(name) else {
            return Err(Error::new(ErrorKind::UnknownSymbol(name.to_owned()), Pos { line: 1, col: 1 }));
        };
        resolve_in(&index, start)
    }

    /// Every symbol resolved, in file order; fails on the first broken `extends`.
    pub fn resolve_all(&self) -> Result<Vec<Symbol>> {
        let index = self.index();
        self.symbols.iter().map(|s| resolve_in(&index, s)).collect()
    }

    /// Name → symbol; with duplicate names the first one wins.
    fn index(&self) -> HashMap<&str, &Symbol> {
        let mut index = HashMap::with_capacity(self.symbols.len());
        for s in &self.symbols {
            index.entry(s.name.as_str()).or_insert(s);
        }
        index
    }
}

fn resolve_in(index: &HashMap<&str, &Symbol>, start: &Symbol) -> Result<Symbol> {
    // Leaf first: start, its parent, the parent's parent, …
    let mut chain: Vec<&Symbol> = vec![start];
    let mut seen: HashSet<&str> = HashSet::from([start.name.as_str()]);
    let mut cur = start;
    while let Some(parent_name) = cur.extends.as_deref() {
        if !seen.insert(parent_name) {
            return Err(Error::new(ErrorKind::ExtendsCycle { symbol: start.name.clone() }, start.pos));
        }
        let Some(parent) = index.get(parent_name) else {
            let kind = ErrorKind::MissingParent { symbol: cur.name.clone(), parent: parent_name.to_owned() };
            return Err(Error::new(kind, cur.pos));
        };
        chain.push(parent);
        cur = parent;
    }
    let mut chain = chain.into_iter().rev();
    let Some(root) = chain.next() else { return Ok(start.clone()) };
    Ok(chain.fold(root.clone(), |parent, derived| inherit(&parent, derived)))
}

/// `derived` on top of an already resolved `parent`.
fn inherit(parent: &Symbol, derived: &Symbol) -> Symbol {
    let mut properties = parent.properties.clone();
    for p in &derived.properties {
        match properties.iter_mut().find(|q| q.key == p.key) {
            Some(q) => *q = p.clone(),
            None => properties.push(p.clone()),
        }
    }
    let units = if derived.units.is_empty() { parent.units.iter().map(|u| u.renamed(&derived.name)).collect() } else { derived.units.clone() };
    Symbol {
        name: derived.name.clone(),
        pos: derived.pos,
        extends: derived.extends.clone(),
        pin_numbers_hide: derived.pin_numbers_hide.or(parent.pin_numbers_hide),
        pin_names: derived.pin_names.clone().or_else(|| parent.pin_names.clone()),
        in_bom: derived.in_bom.or(parent.in_bom),
        on_board: derived.on_board.or(parent.on_board),
        exclude_from_sim: derived.exclude_from_sim.or(parent.exclude_from_sim),
        power: derived.power || parent.power,
        power_scope: derived.power_scope.clone().or_else(|| parent.power_scope.clone()),
        embedded_fonts: derived.embedded_fonts.or(parent.embedded_fonts),
        properties,
        units,
        unknown: derived.unknown.clone(),
    }
}
