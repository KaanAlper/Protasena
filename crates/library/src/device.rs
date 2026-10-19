//! Devices: what Pick Devices lists and Edit Component edits.
//!
//! A device joins four kinds of information, like a Proteus device definition:
//! - the **index** fields Pick Devices shows and filters by (category, sub-category,
//!   manufacturer, description, keywords, stock code, library);
//! - the **property definitions** that make up the Edit Component dialog (name, prompt, type,
//!   hidden flag, default value);
//! - the **symbol** and the **packages** it can be laid out with (pin number → pad maps);
//! - an optional **simulation model** link, and where the data came from (**source**).

use serde::{Deserialize, Serialize};

use crate::symbol::Symbol;

/// A library device.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Device {
    /// Device name as typed in Pick Devices (`RES`, `CAP-ELEC`, `74LS00`), unique in a library
    /// regardless of case.
    pub name: String,
    /// Name of the pack library holding the device (filled in by the loader).
    #[serde(default)]
    pub library: String,
    /// Reference prefix for annotation (`R` → R1, R2, …); empty for power symbols.
    #[serde(default)]
    pub prefix: String,
    /// One-line description in our own words.
    #[serde(default)]
    pub description: String,
    /// Pick Devices category.
    #[serde(default)]
    pub category: String,
    /// Pick Devices sub-category.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sub_category: String,
    /// Manufacturer (empty for generic parts).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub manufacturer: String,
    /// Stock (order) code.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stock_code: String,
    /// Extra search words.
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "crate::bounded::strings")]
    pub keywords: Vec<String>,
    /// The schematic symbol.
    #[serde(default)]
    pub symbol: Symbol,
    /// Packages the device can use, each with its pin map.
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "crate::bounded::package_refs")]
    pub packages: Vec<PackageRef>,
    /// The package a new instance gets (one of `packages`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_package: Option<String>,
    /// Property definitions, in Edit Component order.
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "crate::bounded::prop_defs")]
    pub prop_defs: Vec<PropDef>,
    /// Simulation model, if the device simulates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sim_model: Option<SimModelRef>,
    /// Upstream origin of converted data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
}

impl Device {
    /// The property definition with this name (case-insensitive, like Proteus property keys).
    pub fn prop_def(&self, name: &str) -> Option<&PropDef> {
        self.prop_defs.iter().find(|p| p.name.eq_ignore_ascii_case(name))
    }

    /// The package reference with this name.
    pub fn package(&self, name: &str) -> Option<&PackageRef> {
        self.packages.iter().find(|p| p.name == name)
    }

    /// Default values of all properties that have one, in definition order.
    pub fn default_properties(&self) -> Vec<(String, String)> {
        self.prop_defs.iter().filter_map(|p| p.default.as_ref().map(|v| (p.name.clone(), v.clone()))).collect()
    }
}

/// A package a device can use, and which pad each symbol pin lands on.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PackageRef {
    /// Package name (a [`Package`](crate::Package) in the pack).
    pub name: String,
    /// `(pin number, pad name)` pairs.
    #[serde(default, deserialize_with = "crate::bounded::pin_map")]
    pub pin_map: Vec<(String, String)>,
}

impl PackageRef {
    /// The pad a pin lands on.
    pub fn pad_for(&self, pin_number: &str) -> Option<&str> {
        self.pin_map.iter().find(|(pin, _)| pin == pin_number).map(|(_, pad)| pad.as_str())
    }
}

/// One field of the Edit Component dialog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropDef {
    /// Property key (`VALUE`, `PACKAGE`, …).
    pub name: String,
    /// Label shown in Edit Component.
    #[serde(default)]
    pub prompt: String,
    /// Value type.
    pub kind: PropKind,
    /// Not shown on the schematic.
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
    /// Value of a new instance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

impl PropDef {
    /// A property definition.
    pub fn new(name: &str, prompt: &str, kind: PropKind, default: Option<&str>) -> Self {
        Self { name: name.to_owned(), prompt: prompt.to_owned(), kind, hidden: false, default: default.map(str::to_owned) }
    }

    /// The same definition, hidden on the sheet.
    pub fn hidden(self) -> Self {
        Self { hidden: true, ..self }
    }
}

/// Value type of a property.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PropKind {
    /// Free text.
    String,
    /// A number with optional engineering suffix (`10k`, `4.7u`), with an optional unit.
    Float {
        /// Unit symbol shown after the value (`Ω`, `F`, `V`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        unit: Option<String>,
    },
    /// One of a fixed set of keywords.
    Keywords {
        /// Allowed values, in display order.
        #[serde(deserialize_with = "crate::bounded::strings")]
        choices: Vec<String>,
    },
    /// The name of one of the device's packages.
    Package,
}

/// Link to a simulation model.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SimModelRef {
    /// Primitive class (`ANALOG`, `DIGITAL`, `SPICE`, …).
    pub primitive: String,
    /// Model name within that class.
    pub model: String,
    /// Model parameters as `(name, value)`.
    #[serde(default, skip_serializing_if = "Vec::is_empty", deserialize_with = "crate::bounded::pairs")]
    pub params: Vec<(String, String)>,
}

/// Upstream origin of converted library data.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Source {
    /// Upstream project (`kicad`).
    pub upstream: String,
    /// Upstream library name.
    pub lib: String,
    /// Upstream symbol name.
    pub symbol: String,
    /// Licence tag of the data (SPDX, e.g. `CC-BY-SA-4.0`).
    pub licence: String,
}

fn is_false(b: &bool) -> bool {
    !*b
}
