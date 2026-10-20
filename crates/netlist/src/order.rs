//! Natural ordering (`R2` before `R10`) and a small union–find.

use std::cmp::Ordering;

use crate::model::Node;

/// Compares strings with digit runs taken as numbers (`R2 < R10`, `U1A < U1B`); equal
/// numbers with different leading zeros, and otherwise equal strings, fall back to plain order.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a, b);
    loop {
        match (x.chars().next(), y.chars().next()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(cx), Some(cy)) if cx.is_ascii_digit() && cy.is_ascii_digit() => {
                let (nx, rx) = split_digits(x);
                let (ny, ry) = split_digits(y);
                let (tx, ty) = (nx.trim_start_matches('0'), ny.trim_start_matches('0'));
                let by_value = tx.len().cmp(&ty.len()).then_with(|| tx.cmp(ty));
                if by_value != Ordering::Equal {
                    return by_value;
                }
                (x, y) = (rx, ry);
            }
            (Some(cx), Some(cy)) => {
                if cx != cy {
                    return cx.cmp(&cy);
                }
                (x, y) = (x.get(cx.len_utf8()..).unwrap_or(""), y.get(cy.len_utf8()..).unwrap_or(""));
            }
        }
    }
}

fn split_digits(s: &str) -> (&str, &str) {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    (s.get(..end).unwrap_or(""), s.get(end..).unwrap_or(""))
}

/// Natural order of nodes: by reference, then pin number.
pub fn node_cmp(a: &Node, b: &Node) -> Ordering {
    natural_cmp(&a.reference, &b.reference).then_with(|| natural_cmp(&a.pin, &b.pin))
}

/// Disjoint sets over `0..n` (union by size, path halving; never recursive).
#[derive(Debug, Clone, Default)]
pub struct UnionFind {
    parent: Vec<usize>,
    size: Vec<usize>,
}

impl UnionFind {
    /// Adds a singleton set and returns its element.
    pub fn add(&mut self) -> usize {
        let e = self.parent.len();
        self.parent.push(e);
        self.size.push(1);
        e
    }

    /// The representative of `e`'s set (`e` itself when out of range).
    pub fn find(&mut self, mut e: usize) -> usize {
        while let Some(&p) = self.parent.get(e) {
            if p == e {
                break;
            }
            let grand = self.parent.get(p).copied().unwrap_or(p);
            if let Some(slot) = self.parent.get_mut(e) {
                *slot = grand;
            }
            e = grand;
        }
        e
    }

    /// Joins the sets of `a` and `b`.
    pub fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return;
        }
        let (sa, sb) = (self.size.get(ra).copied().unwrap_or(1), self.size.get(rb).copied().unwrap_or(1));
        let (big, small) = if sa >= sb { (ra, rb) } else { (rb, ra) };
        if let Some(slot) = self.parent.get_mut(small) {
            *slot = big;
        }
        if let Some(slot) = self.size.get_mut(big) {
            *slot = sa + sb;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order_compares_numbers_by_value() {
        let mut v = vec!["R10", "R2", "C1", "R1", "#NET0002", "R02", "U1B", "U1A", "R", "Ω3", "R99999999999999999999999"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, ["#NET0002", "C1", "R", "R1", "R02", "R2", "R10", "R99999999999999999999999", "U1A", "U1B", "Ω3"]);
        assert_eq!(natural_cmp("a", "a"), Ordering::Equal);
    }

    #[test]
    fn union_find_joins_sets() {
        let mut uf = UnionFind::default();
        let e: Vec<usize> = (0..5).map(|_| uf.add()).collect();
        uf.union(e[0], e[1]);
        uf.union(e[3], e[4]);
        uf.union(e[1], e[4]);
        assert_eq!(uf.find(e[0]), uf.find(e[3]));
        assert_ne!(uf.find(e[2]), uf.find(e[0]));
        assert_eq!(uf.find(99), 99);
    }
}
