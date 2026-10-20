//! Length-capped list deserializers.
//!
//! A pack file of a few megabytes of tiny JSON items (`[0,0],[0,0],…`) would expand to far
//! more memory than the file size once deserialized. Every list in the model is read through
//! one of these functions, which stop with an error as soon as a list passes its
//! [limit](crate::limits) instead of growing without bound.

use std::fmt;
use std::marker::PhantomData;

use protasena_geom::Pt;
use serde::de::{Deserialize, Deserializer, Error, SeqAccess, Visitor};

use crate::device::{Device, PackageRef, PropDef};
use crate::limits::*;
use crate::pack::IndexEntry;
use crate::package::{LayerGraphic, Package, Pad};
use crate::symbol::{Graphic, Pin, PinAlternate, Unit};

struct Capped<T> {
    limit: usize,
    item: PhantomData<T>,
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for Capped<T> {
    type Value = Vec<T>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a list of at most {} items", self.limit)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<T>, A::Error> {
        // Never trust a size hint further than a small first allocation.
        let mut out = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(self.limit).min(256));
        while let Some(item) = seq.next_element()? {
            if out.len() >= self.limit {
                return Err(A::Error::custom(format_args!("list has more than {} items", self.limit)));
            }
            out.push(item);
        }
        Ok(out)
    }
}

fn capped<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D, limit: usize) -> Result<Vec<T>, D::Error> {
    d.deserialize_seq(Capped { limit, item: PhantomData })
}

macro_rules! capped_lists {
    ($($name:ident: $t:ty = $limit:expr;)*) => {$(
        pub(crate) fn $name<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<$t>, D::Error> {
            capped(d, $limit)
        }
    )*};
}

capped_lists! {
    index_entries: IndexEntry = MAX_LIBRARIES;
    devices: Device = MAX_DEVICES;
    packages: Package = MAX_PACKAGES;
    units: Unit = MAX_UNITS;
    pins: Pin = MAX_PINS;
    alternates: PinAlternate = MAX_ALTERNATES;
    graphics: Graphic = MAX_GRAPHICS;
    layer_graphics: LayerGraphic = MAX_GRAPHICS;
    points: Pt = MAX_POINTS;
    pads: Pad = MAX_PADS;
    prop_defs: PropDef = MAX_PROP_DEFS;
    package_refs: PackageRef = MAX_DEVICE_PACKAGES;
    pin_map: (String, String) = MAX_PIN_MAP;
    pairs: (String, String) = MAX_LIST;
    strings: String = MAX_LIST;
}

/// [`pin_map`], shared.
pub(crate) fn shared_pin_map<'de, D: Deserializer<'de>>(d: D) -> Result<std::sync::Arc<[(String, String)]>, D::Error> {
    pin_map(d).map(Into::into)
}

#[cfg(test)]
mod tests {
    #[derive(Debug, serde::Deserialize)]
    struct Holder {
        #[serde(deserialize_with = "super::strings")]
        list: Vec<String>,
    }

    #[test]
    fn lists_up_to_the_limit_load_and_longer_ones_fail() {
        let make = |n: usize| format!("{{\"list\":[{}]}}", vec!["\"a\""; n].join(","));
        assert_eq!(serde_json::from_str::<Holder>(&make(super::MAX_LIST)).unwrap().list.len(), super::MAX_LIST);
        let err = serde_json::from_str::<Holder>(&make(super::MAX_LIST + 1)).unwrap_err();
        assert!(err.to_string().contains("more than 1024 items"), "{err}");
        assert!(serde_json::from_str::<Holder>("{\"list\":7}").is_err());
    }
}
