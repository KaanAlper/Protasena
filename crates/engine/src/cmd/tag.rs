//! Tool ▸ Search & Tag (T): tags (selects) the components on the current sheet whose property
//! matches. Built the conventional way (Proteus's dialog was not observable):
//! - the property is a name: `REF`, `VALUE`, `DEVICE`, `PACKAGE` or any other property of the
//!   part (compared without case); a part without it has the empty value;
//! - operators: equals, not-equals, begins, not-begins, ends, contains, not-contains, and
//!   less/greater, which compare engineering values (`4k7` < `10k`, `100nF` < `1u`);
//! - the search starts a new set of tagged parts (`new`), keeps only the tagged ones that
//!   match (`and`), or adds the matches to them (`or`).

use protasena_doc::{Component, ItemId};
use serde_json::{Value, json};

use super::ids_json;
use crate::args::Args;
use crate::error::Result;
use crate::registry::CommandSpec;
use crate::request::Request;
use crate::session::Session;

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("search-tag", "Search & Tag", search_tag)
            .menu(&["Tool", "Search & Tag"])
            .key("T")
            .params("{property?: string (default REF), value?: string, op?: \"equals\"|\"not-equals\"|\"begins\"|\"not-begins\"|\"ends\"|\"contains\"|\"not-contains\"|\"less\"|\"greater\", case?: bool, mode?: \"new\"|\"and\"|\"or\"} — tags the matching components of the current sheet")
            .dialog(|_| Some(Request::SearchTag)),
    ]
}

/// A comparison of Search & Tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Equals,
    Begins,
    Ends,
    Contains,
    Less,
    Greater,
}

/// The value of a component's property (the empty string when it has none): `REF`, `VALUE`,
/// `DEVICE` and `PACKAGE` are the part's own fields.
pub(super) fn property_of<'a>(c: &'a Component, name: &str) -> &'a str {
    let n = name.trim();
    if n.eq_ignore_ascii_case("REF") || n.eq_ignore_ascii_case("REFERENCE") {
        &c.reference
    } else if n.eq_ignore_ascii_case("VALUE") {
        &c.value
    } else if n.eq_ignore_ascii_case("DEVICE") {
        &c.device.name
    } else if n.eq_ignore_ascii_case("PACKAGE") {
        c.package.as_deref().unwrap_or("")
    } else {
        c.properties.get(n).unwrap_or("")
    }
}

/// A number written the engineering way: `10k`, `4k7`, `100n`, `2.2uF`, `1MEG`, `4R7`, `0.5`.
pub fn engineering_value(s: &str) -> Option<f64> {
    let t = s.trim();
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    let rest = t.get(digits.len()..)?;
    let (scale, len) = prefix(rest);
    let after = rest.get(len..)?;
    let more: String = if len > 0 { after.chars().take_while(char::is_ascii_digit).collect() } else { String::new() };
    if digits.is_empty() || (!more.is_empty() && digits.contains('.')) {
        return None;
    }
    let number = if more.is_empty() { digits } else { format!("{digits}.{more}") };
    number.parse::<f64>().ok().map(|v| v * scale).filter(|v| v.is_finite())
}

/// The multiplier an SI prefix letter stands for, and its length (`MEG` before `M` and `m`).
fn prefix(s: &str) -> (f64, usize) {
    if s.len() >= 3 && s.get(..3).is_some_and(|p| p.eq_ignore_ascii_case("meg")) {
        return (1e6, 3);
    }
    let Some(c) = s.chars().next() else { return (1.0, 0) };
    let scale = match c {
        'p' => 1e-12,
        'n' => 1e-9,
        'u' | 'µ' => 1e-6,
        'm' => 1e-3,
        'k' | 'K' => 1e3,
        'M' => 1e6,
        'G' => 1e9,
        'T' => 1e12,
        'R' | 'r' => 1.0,
        _ => return (1.0, 0),
    };
    (scale, c.len_utf8())
}

fn matches(value: &str, wanted: &str, op: Op, case: bool) -> bool {
    let (v, w) = if case { (value.to_owned(), wanted.to_owned()) } else { (value.to_lowercase(), wanted.to_lowercase()) };
    match op {
        Op::Equals => v == w,
        Op::Begins => v.starts_with(&w),
        Op::Ends => v.ends_with(&w),
        Op::Contains => v.contains(&w),
        Op::Less | Op::Greater => match (engineering_value(value), engineering_value(wanted)) {
            (Some(a), Some(b)) => (op == Op::Less && a < b) || (op == Op::Greater && a > b),
            _ => false,
        },
    }
}

/// The operator and whether it is negated.
fn op_param(a: &Args) -> Result<(Op, bool)> {
    let name = a.str("op")?.unwrap_or("equals").trim().to_ascii_lowercase();
    let (negated, base) = match name.strip_prefix("not-") {
        Some(b) => (true, b),
        None => (false, name.as_str()),
    };
    let op = match base {
        "equals" => Op::Equals,
        "begins" => Op::Begins,
        "ends" => Op::Ends,
        "contains" => Op::Contains,
        "less" if !negated => Op::Less,
        "greater" if !negated => Op::Greater,
        _ => {
            return Err(a.bad(format!(
                "unknown `op` {name:?}: equals, not-equals, begins, not-begins, ends, not-ends, contains, not-contains, less or greater"
            )));
        }
    };
    Ok((op, negated))
}

fn search_tag(s: &mut Session, a: &Args) -> Result<Value> {
    let property = a.str("property")?.unwrap_or("REF").trim();
    if property.is_empty() {
        return Err(a.bad("`property` is empty"));
    }
    let wanted = a.str("value")?.unwrap_or("");
    let (op, negated) = op_param(a)?;
    let case = a.bool("case")?.unwrap_or(false);
    let mode = a.str("mode")?.unwrap_or("new").trim().to_ascii_lowercase();
    let found: Vec<ItemId> = s
        .current_sheet()?
        .components
        .iter()
        .filter(|c| matches(property_of(c, property), wanted, op, case) != negated)
        .map(|c| ItemId::Component(c.id))
        .collect();
    let tagged = match mode.as_str() {
        "new" => found,
        "and" => s.selection.iter().copied().filter(|id| found.contains(id)).collect(),
        "or" => s.selection.iter().copied().chain(found).collect(),
        _ => return Err(a.bad(format!("unknown `mode` {mode:?}: new, and or or"))),
    };
    s.set_selection(tagged);
    Ok(json!({ "ids": ids_json(&s.selection), "count": s.selection.len() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engineering_values_read_like_a_parts_list() {
        let close = |s: &str, v: f64| engineering_value(s).is_some_and(|x| (x - v).abs() <= v.abs() * 1e-12);
        assert!(close("10k", 10e3) && close("4k7", 4.7e3) && close("100n", 100e-9) && close("2.2uF", 2.2e-6));
        assert!(close("1MEG", 1e6) && close("1M", 1e6) && close("1m", 1e-3) && close("4R7", 4.7) && close(" 0.5 ", 0.5));
        assert!(close("10µF", 10e-6) && close("47", 47.0));
        for bad in ["", "k", "x10", "1.5k3", "."] {
            assert_eq!(engineering_value(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn comparisons_ignore_case_unless_asked() {
        assert!(matches("BC547", "bc5", Op::Begins, false));
        assert!(!matches("BC547", "bc5", Op::Begins, true));
        assert!(matches("4k7", "10k", Op::Less, false) && matches("1u", "100n", Op::Greater, false));
        assert!(!matches("abc", "10k", Op::Less, false), "not a number: no match");
    }
}
