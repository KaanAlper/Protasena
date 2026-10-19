//! `.kicad_sym` symbol libraries (KiCad documentation: "Symbol Library File Format" and
//! "Schematic and Symbol Library Common Syntax").

mod graphic;
mod pin;
mod resolve;
mod unit;

use crate::common::{At, Effects};
use crate::decode::{flag, int, num1, text, text1, yes_no};
use crate::error::{Error, ErrorKind, Pos, Result};
use crate::report::{Inspect, Report, Unknown};
use crate::sexpr::{self, Sexpr};

pub use graphic::{Graphic, GraphicKind, TextBoxGeometry};
pub use pin::{Alternate, ElectricalType, Pin, PinStyle};
pub use unit::Unit;

/// A symbol library file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SymbolLib {
    /// `(version YYYYMMDD)`.
    pub version: Option<i64>,
    pub generator: Option<String>,
    pub generator_version: Option<String>,
    pub symbols: Vec<Symbol>,
    pub unknown: Vec<Unknown>,
}

/// `(pin_names [(offset OFFSET)] [hide])`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PinNames {
    /// Name offset in mm; KiCad's default is 0.508 when absent.
    pub offset: Option<f64>,
    pub hide: bool,
    pub unknown: Vec<Unknown>,
}

/// A top-level symbol.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Symbol {
    pub name: String,
    /// Where the symbol starts in the file.
    pub pos: Pos,
    /// `(extends "PARENT")`: a derived symbol; see [`SymbolLib::resolve`].
    pub extends: Option<String>,
    /// `(pin_numbers hide)` / `(pin_numbers (hide yes))`; `None` when absent.
    pub pin_numbers_hide: Option<bool>,
    pub pin_names: Option<PinNames>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub exclude_from_sim: Option<bool>,
    /// `(power)` marks a power symbol; KiCad 9 may add a scope word (kept in `power_scope`).
    pub power: bool,
    pub power_scope: Option<String>,
    pub embedded_fonts: Option<bool>,
    pub properties: Vec<Property>,
    /// Units and body styles. Graphics and pins written directly in the symbol (not in a
    /// child unit) go to a unit numbered 0, style 0 (shared by every unit and style).
    pub units: Vec<Unit>,
    pub unknown: Vec<Unknown>,
}

impl Symbol {
    /// The property with this key.
    pub fn property(&self, key: &str) -> Option<&Property> {
        self.properties.iter().find(|p| p.key == key)
    }

    /// The value of the property with this key.
    pub fn value_of(&self, key: &str) -> Option<&str> {
        self.property(key).map(|p| p.value.as_str())
    }

    /// Number of distinct units (not counting the shared unit 0).
    pub fn unit_count(&self) -> usize {
        let mut n: Vec<u32> = self.units.iter().map(|u| u.unit).filter(|&u| u != 0).collect();
        n.sort_unstable();
        n.dedup();
        n.len()
    }

    /// True when some unit has an alternate body style (De Morgan).
    pub fn has_alternate_style(&self) -> bool {
        self.units.iter().any(|u| u.style == 2)
    }
}

/// `(property "KEY" "VALUE" (id N) (at …) (effects …))`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Property {
    /// `(property private "KEY" "VALUE" …)`: marked private (KiCad 9; not in the published
    /// format documentation, read as seen in KiCad 9 libraries).
    pub private: bool,
    pub key: String,
    pub value: String,
    pub id: Option<i64>,
    pub at: Option<At>,
    pub effects: Option<Effects>,
    /// `(hide yes)` on the property itself (KiCad 9); KiCad 6–8 hide via `effects`.
    pub hide: bool,
    pub show_name: Option<bool>,
    pub do_not_autoplace: Option<bool>,
    pub unknown: Vec<Unknown>,
}

impl Property {
    /// Hidden by either form of `hide`.
    pub fn is_hidden(&self) -> bool {
        self.hide || self.effects.as_ref().is_some_and(|e| e.hide)
    }

    pub(crate) fn decode(node: &Sexpr) -> Result<Self> {
        let args = node.args();
        let private = args.first().is_some_and(|a| a.as_atom() == Some("private")) && args.get(2).is_some_and(|a| a.text().is_some());
        let skip = usize::from(private);
        let mut p = Self { private, key: text(node, skip, "property key")?, value: text(node, skip + 1, "property value")?, ..Self::default() };
        for item in node.args().iter().skip(skip + 2) {
            match item.name() {
                "id" => p.id = Some(int(item, 0, "property id")?),
                "at" => p.at = Some(At::decode(item)?),
                "effects" => p.effects = Some(Effects::decode(item)?),
                "hide" => p.hide = flag(item)?,
                "show_name" => p.show_name = Some(flag(item)?),
                "do_not_autoplace" => p.do_not_autoplace = Some(flag(item)?),
                _ => p.unknown.push(Unknown::new("property", item)),
            }
        }
        Ok(p)
    }
}

impl Inspect for Property {
    fn visit(&self, r: &mut Report) {
        self.effects.visit(r);
        self.unknown.visit(r);
    }
}

/// Parses a `.kicad_sym` file.
pub fn parse_symbol_lib(input: &[u8]) -> Result<SymbolLib> {
    let root = sexpr::parse(input)?;
    SymbolLib::from_sexpr(&root)
}

impl SymbolLib {
    /// Reads a library from an already parsed `(kicad_symbol_lib …)` tree.
    pub fn from_sexpr(root: &Sexpr) -> Result<Self> {
        if root.head() != Some("kicad_symbol_lib") {
            let found = root.name().to_owned();
            return Err(Error::new(ErrorKind::WrongRoot { expected: "kicad_symbol_lib", found }, root.pos()));
        }
        let mut lib = Self::default();
        for item in root.args() {
            match item.head() {
                Some("version") => lib.version = Some(int(item, 0, "version")?),
                Some("generator") => lib.generator = Some(text1(item)?),
                Some("generator_version") => lib.generator_version = Some(text1(item)?),
                Some("symbol") => lib.symbols.push(decode_symbol(item)?),
                _ => lib.unknown.push(Unknown::new("kicad_symbol_lib", item)),
            }
        }
        Ok(lib)
    }

    /// The symbol with this name, as written (not resolved).
    pub fn get(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.name == name)
    }

    /// Everything kept unread in the library.
    pub fn report(&self) -> Report {
        Inspect::report(self)
    }
}

impl Inspect for SymbolLib {
    fn visit(&self, r: &mut Report) {
        self.symbols.visit(r);
        self.unknown.visit(r);
    }
}

impl Inspect for Symbol {
    fn visit(&self, r: &mut Report) {
        self.pin_names.as_ref().map(|p| &p.unknown).visit(r);
        self.properties.visit(r);
        self.units.visit(r);
        self.unknown.visit(r);
    }
}

fn decode_symbol(node: &Sexpr) -> Result<Symbol> {
    let mut s = Symbol { name: text(node, 0, "symbol name")?, pos: node.pos(), ..Symbol::default() };
    let mut shared = Unit::shared(&s.name);
    for item in node.args().iter().skip(1) {
        match item.name() {
            "extends" => s.extends = Some(text1(item)?),
            "pin_numbers" => s.pin_numbers_hide = Some(decode_hide_holder(item, &mut s.unknown)?),
            "pin_names" => s.pin_names = Some(decode_pin_names(item)?),
            "in_bom" => s.in_bom = Some(yes_no(item)?),
            "on_board" => s.on_board = Some(yes_no(item)?),
            "exclude_from_sim" => s.exclude_from_sim = Some(yes_no(item)?),
            "embedded_fonts" => s.embedded_fonts = Some(yes_no(item)?),
            "power" => {
                s.power = true;
                for (i, a) in item.args().iter().enumerate() {
                    match a.text() {
                        Some(scope) if i == 0 => s.power_scope = Some(scope.to_owned()),
                        _ => s.unknown.push(Unknown::new("power", a)),
                    }
                }
            }
            "property" if item.as_list().is_some() => s.properties.push(Property::decode(item)?),
            "symbol" if item.as_list().is_some() => s.units.push(Unit::decode(item)?),
            _ => {
                if !shared.take_item(item)? {
                    s.unknown.push(Unknown::new("symbol", item));
                }
            }
        }
    }
    if !shared.is_empty() {
        s.units.insert(0, shared);
    }
    Ok(s)
}

/// `(pin_numbers hide)`, `(pin_numbers (hide yes))`, `(pin_numbers)`: the hide setting.
fn decode_hide_holder(node: &Sexpr, unknown: &mut Vec<Unknown>) -> Result<bool> {
    let mut hide = false;
    for item in node.args() {
        match item.name() {
            "hide" => hide = flag(item)?,
            _ => unknown.push(Unknown::new("pin_numbers", item)),
        }
    }
    Ok(hide)
}

fn decode_pin_names(node: &Sexpr) -> Result<PinNames> {
    let mut p = PinNames::default();
    for item in node.args() {
        match item.name() {
            "offset" if item.as_list().is_some() => p.offset = Some(num1(item)?),
            "hide" => p.hide = flag(item)?,
            _ => p.unknown.push(Unknown::new("pin_names", item)),
        }
    }
    Ok(p)
}

#[cfg(test)]
mod tests;
