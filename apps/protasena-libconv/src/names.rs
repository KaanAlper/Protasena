//! Pack-wide unique device names (compared without case, like Pick Devices).
//!
//! Each device asks for its map name (Proteus style) or else its KiCad symbol name. When
//! several ask for the same name, the one with a map name wins, then the first by (library,
//! symbol); the others become `NAME_LIBRARY` (then `NAME_LIBRARY_2`, … should that be taken
//! too). Names nobody contests are never changed, so the result depends only on the inputs.

use std::collections::{BTreeMap, HashSet};

/// A naming request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request<'a> {
    /// KiCad library.
    pub lib: &'a str,
    /// KiCad symbol.
    pub symbol: &'a str,
    /// Name from the map.
    pub mapped: Option<&'a str>,
}

/// The final name of every request, in request order.
pub fn assign(requests: &[Request<'_>]) -> Vec<String> {
    let wanted: Vec<&str> = requests.iter().map(|r| r.mapped.unwrap_or(r.symbol)).collect();
    // Group by wanted name; the winner of each group is the smallest (no map name?, lib, symbol).
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, w) in wanted.iter().enumerate() {
        groups.entry(w.to_uppercase()).or_default().push(i);
    }
    let mut names: Vec<Option<String>> = vec![None; requests.len()];
    let mut taken: HashSet<String> = HashSet::new();
    let mut losers = Vec::new();
    for members in groups.values() {
        let key = |&i: &usize| requests.get(i).map(|r| (r.mapped.is_none(), r.lib, r.symbol));
        let mut members = members.clone();
        members.sort_by_key(key);
        let mut it = members.into_iter();
        if let Some(first) = it.next()
            && let (Some(slot), Some(w)) = (names.get_mut(first), wanted.get(first))
        {
            taken.insert(w.to_uppercase());
            *slot = Some((*w).to_owned());
        }
        losers.extend(it);
    }
    losers.sort_by_key(|&i| requests.get(i).map(|r| (r.lib, r.symbol)));
    for i in losers {
        let (Some(r), Some(w)) = (requests.get(i), wanted.get(i)) else { continue };
        let base = format!("{w}_{}", r.lib);
        let mut name = base.clone();
        let mut n = 2;
        while !taken.insert(name.to_uppercase()) {
            name = format!("{base}_{n}");
            n += 1;
        }
        if let Some(slot) = names.get_mut(i) {
            *slot = Some(name);
        }
    }
    names.into_iter().zip(wanted).map(|(n, w)| n.unwrap_or_else(|| w.to_owned())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req<'a>(lib: &'a str, symbol: &'a str, mapped: Option<&'a str>) -> Request<'a> {
        Request { lib, symbol, mapped }
    }

    #[test]
    fn map_names_win_then_library_order() {
        let r = [
            req("74xx_IEEE", "74LS00", None),
            req("74xx", "74LS00", None),
            req("Device", "R", Some("RES")),
            req("Other", "res", None),
            req("Solo", "Unique", None),
        ];
        assert_eq!(assign(&r), ["74LS00_74xx_IEEE", "74LS00", "RES", "res_Other", "Unique"]);
    }

    #[test]
    fn suffixes_never_collide_with_real_names() {
        let r = [req("A", "X", None), req("B", "X", None), req("C", "X_B", None)];
        assert_eq!(assign(&r), ["X", "X_B_2", "X_B"]);
    }

    #[test]
    fn same_input_same_names() {
        let r = [req("B", "X", None), req("A", "X", None)];
        let mut swapped = r.clone();
        swapped.reverse();
        let a = assign(&r);
        let mut b = assign(&swapped);
        b.reverse();
        assert_eq!(a, b);
        assert_eq!(a, ["X_B", "X"]);
    }
}
