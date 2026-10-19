//! Footprint text: `fp_text` and positioned `property` tokens.

use crate::common::{At, Effects};
use crate::decode::{flag, is_word, put, text, text1};
use crate::error::Result;
use crate::report::{Inspect, Report, Unknown, keyword_enum};
use crate::sexpr::Sexpr;

keyword_enum!(
    /// What an `fp_text` is.
    FpTextType, "fp_text.type" { Reference = "reference", Value = "value", User = "user" }
);

/// Placement and look shared by footprint texts and properties.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextAttrs {
    pub at: Option<At>,
    pub layer: Option<String>,
    /// `(layer "…" knockout)`.
    pub knockout: bool,
    /// The text may be rotated away from upright.
    pub unlocked: bool,
    pub hide: bool,
    pub effects: Option<Effects>,
    pub uuid: Option<String>,
}

impl TextAttrs {
    /// Takes `item` when it is one of the shared text tokens.
    pub(crate) fn take(&mut self, item: &Sexpr, unknown: &mut Vec<Unknown>) -> Result<bool> {
        match item.name() {
            "unlocked" => self.unlocked = flag(item)?,
            "hide" => self.hide = flag(item)?,
            _ if item.as_list().is_none() => return Ok(false),
            "at" => put(&mut self.at, At::decode(item)?, item, unknown),
            "layer" => {
                put(&mut self.layer, text1(item)?, item, unknown);
                for extra in item.args().iter().skip(1) {
                    if is_word(extra, "knockout") {
                        self.knockout = true;
                    } else {
                        unknown.push(Unknown::new("layer", extra));
                    }
                }
            }
            "effects" => put(&mut self.effects, Effects::decode(item)?, item, unknown),
            "uuid" | "tstamp" => put(&mut self.uuid, text1(item)?, item, unknown),
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Hidden by `hide` on the text or in its effects.
    pub fn is_hidden(&self) -> bool {
        self.hide || self.effects.as_ref().is_some_and(|e| e.hide)
    }
}

/// `(fp_text TYPE "TEXT" (at …) (layer …) …)`.
#[derive(Debug, Clone, PartialEq)]
pub struct FpText {
    pub kind: FpTextType,
    pub text: String,
    pub attrs: TextAttrs,
    pub unknown: Vec<Unknown>,
}

impl FpText {
    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        let mut t = Self {
            kind: FpTextType::from_keyword(&text(node, 0, "text type")?),
            text: text(node, 1, "text")?,
            attrs: TextAttrs::default(),
            unknown: Vec::new(),
        };
        for item in node.args().iter().skip(2) {
            if !t.attrs.take(item, &mut t.unknown)? {
                t.unknown.push(Unknown::new("fp_text", item));
            }
        }
        Ok(t)
    }
}

impl Inspect for FpText {
    fn visit(&self, r: &mut Report) {
        self.kind.visit(r);
        self.attrs.effects.visit(r);
        self.unknown.visit(r);
    }
}

/// `(property "KEY" "VALUE" …)`: from KiCad 8 `Reference`, `Value` and other fields carry
/// their own position, layer and effects.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FpProperty {
    pub key: String,
    pub value: String,
    pub attrs: TextAttrs,
    pub unknown: Vec<Unknown>,
}

impl FpProperty {
    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        let mut p = Self { key: text(node, 0, "property key")?, value: text(node, 1, "property value")?, ..Self::default() };
        for item in node.args().iter().skip(2) {
            if !p.attrs.take(item, &mut p.unknown)? {
                p.unknown.push(Unknown::new("property", item));
            }
        }
        Ok(p)
    }
}

impl Inspect for FpProperty {
    fn visit(&self, r: &mut Report) {
        self.attrs.effects.visit(r);
        self.unknown.visit(r);
    }
}
