//! Sharing equal symbols and pin maps between devices while a pack loads.
//!
//! KiCad's derived symbols (`extends`) repeat their parent's drawing, so about two thirds of
//! the symbols in KiCad's libraries are copies; and a device's packages found through its
//! footprint filters all have the same pin map (pin `N` → pad `N`). Keeping one copy of each
//! is what makes a full pack fit in a few hundred megabytes.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::device::Device;
use crate::symbol::Symbol;

/// Equal values seen so far.
#[derive(Default)]
pub(crate) struct Interner {
    symbols: HashMap<u64, Vec<Arc<Symbol>>>,
    pin_maps: HashSet<Arc<[(String, String)]>>,
}

impl Interner {
    /// The device with its symbol and pin maps replaced by earlier equal ones.
    pub(crate) fn device(&mut self, mut d: Device) -> Device {
        d.symbol = self.symbol(std::mem::take(&mut d.symbol));
        for p in &mut d.packages {
            p.pin_map = self.pin_map(std::mem::take(&mut p.pin_map));
        }
        d
    }

    fn symbol(&mut self, s: Arc<Symbol>) -> Arc<Symbol> {
        let bucket = self.symbols.entry(fingerprint(&s)).or_default();
        if let Some(same) = bucket.iter().find(|b| ***b == *s) {
            return Arc::clone(same);
        }
        bucket.push(Arc::clone(&s));
        s
    }

    fn pin_map(&mut self, m: Arc<[(String, String)]>) -> Arc<[(String, String)]> {
        if let Some(same) = self.pin_maps.get(&*m) {
            return Arc::clone(same);
        }
        self.pin_maps.insert(Arc::clone(&m));
        m
    }
}

/// A hash of what usually tells symbols apart (units, pin numbers, names and positions); equal
/// symbols always have equal fingerprints.
fn fingerprint(s: &Symbol) -> u64 {
    let mut h = DefaultHasher::new();
    s.power.hash(&mut h);
    s.units.len().hash(&mut h);
    for u in &s.units {
        (&u.name, u.body_style, u.graphics.len(), u.pins.len()).hash(&mut h);
        for p in &u.pins {
            (&p.number, &p.name, p.at.x.to_bits(), p.at.y.to_bits()).hash(&mut h);
        }
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Library;

    #[test]
    fn equal_symbols_and_pin_maps_are_shared() {
        let lib = Library::builtin();
        let res = lib.get("RES").unwrap().clone();
        let mut copy = res.clone();
        copy.name = "RES2".into();
        copy.symbol = Arc::new((*res.symbol).clone());
        copy.packages[0].pin_map = res.packages[0].pin_map.to_vec().into();
        let mut i = Interner::default();
        let a = i.device(res);
        let b = i.device(copy);
        assert!(Arc::ptr_eq(&a.symbol, &b.symbol));
        assert!(Arc::ptr_eq(&a.packages[0].pin_map, &b.packages[0].pin_map));
        let cap = i.device(lib.get("CAP").unwrap().clone());
        assert!(!Arc::ptr_eq(&a.symbol, &cap.symbol), "different symbols stay apart");
    }
}
