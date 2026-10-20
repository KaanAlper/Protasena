//! Small helpers that read typed values out of [`Sexpr`] nodes, with positioned errors.

use crate::common::Pt;
use crate::error::{Error, ErrorKind, Result};
use crate::report::Unknown;
use crate::sexpr::Sexpr;

pub(crate) fn invalid(node: &Sexpr, msg: impl Into<String>) -> Error {
    Error::new(ErrorKind::Invalid(msg.into()), node.pos())
}

/// Argument `i` (after the token name) of a list.
pub(crate) fn arg<'a>(node: &'a Sexpr, i: usize, what: &str) -> Result<&'a Sexpr> {
    node.args().get(i).ok_or_else(|| invalid(node, format!("({}) is missing {what}", node.name())))
}

/// A finite number written as a bare atom.
pub(crate) fn number(item: &Sexpr, what: &str) -> Result<f64> {
    item.as_atom()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .ok_or_else(|| invalid(item, format!("{what}: expected a number, found '{}'", item.name())))
}

pub(crate) fn num(node: &Sexpr, i: usize, what: &str) -> Result<f64> {
    number(arg(node, i, what)?, what)
}

/// Argument `i` as a number when present.
pub(crate) fn opt_num(node: &Sexpr, i: usize, what: &str) -> Result<Option<f64>> {
    node.args().get(i).map(|a| number(a, what)).transpose()
}

pub(crate) fn integer(item: &Sexpr, what: &str) -> Result<i64> {
    item.as_atom().and_then(|s| s.parse::<i64>().ok()).ok_or_else(|| invalid(item, format!("{what}: expected an integer, found '{}'", item.name())))
}

pub(crate) fn int(node: &Sexpr, i: usize, what: &str) -> Result<i64> {
    integer(arg(node, i, what)?, what)
}

/// An atom or a quoted string.
pub(crate) fn string(item: &Sexpr, what: &str) -> Result<String> {
    item.text().map(str::to_owned).ok_or_else(|| invalid(item, format!("{what}: expected text, found a list")))
}

pub(crate) fn text(node: &Sexpr, i: usize, what: &str) -> Result<String> {
    string(arg(node, i, what)?, what)
}

/// Every argument as text, e.g. `(layers "F.Cu" "F.Mask")`.
pub(crate) fn texts(node: &Sexpr, what: &str) -> Result<Vec<String>> {
    node.args().iter().map(|a| string(a, what)).collect()
}

/// `(NAME X Y)`.
pub(crate) fn pt(node: &Sexpr) -> Result<Pt> {
    let what = node.name().to_owned();
    Ok(Pt { x: num(node, 0, &what)?, y: num(node, 1, &what)? })
}

/// `yes`/`no` (also `true`/`false`).
pub(crate) fn bool_word(item: &Sexpr) -> Option<bool> {
    match item.as_atom()? {
        "yes" | "true" => Some(true),
        "no" | "false" => Some(false),
        _ => None,
    }
}

/// A flag: bare `NAME` or `(NAME)` is set, `(NAME yes|no)` says so explicitly (KiCad 6 writes
/// bare flags, later versions the explicit form).
pub(crate) fn flag(node: &Sexpr) -> Result<bool> {
    match node {
        Sexpr::Atom { .. } => Ok(true),
        Sexpr::List { .. } => match node.args() {
            [] => Ok(true),
            [v] => bool_word(v).ok_or_else(|| invalid(v, format!("({}): expected yes or no", node.name()))),
            _ => Err(invalid(node, format!("({}): expected one yes/no value", node.name()))),
        },
        Sexpr::Str { .. } => Err(invalid(node, "expected a flag")),
    }
}

/// `(NAME yes|no)` with the value required.
pub(crate) fn yes_no(node: &Sexpr) -> Result<bool> {
    let v = arg(node, 0, "yes or no")?;
    bool_word(v).ok_or_else(|| invalid(v, format!("({}): expected yes or no", node.name())))
}

/// The number of `(NAME VALUE)`.
pub(crate) fn num1(node: &Sexpr) -> Result<f64> {
    let what = node.name().to_owned();
    num(node, 0, &what)
}

/// The text of `(NAME "VALUE")`.
pub(crate) fn text1(node: &Sexpr) -> Result<String> {
    let what = node.name().to_owned();
    text(node, 0, &what)
}

/// True for a bare atom equal to `word`.
pub(crate) fn is_word(item: &Sexpr, word: &str) -> bool {
    item.as_atom() == Some(word)
}

/// Fails when a required field was not found.
pub(crate) fn required<T>(value: Option<T>, node: &Sexpr, what: &str) -> Result<T> {
    value.ok_or_else(|| invalid(node, format!("({}) is missing ({what})", node.name())))
}

/// Stores `value` in an empty slot; a repeated token is kept as unknown instead of
/// overwriting the first one.
pub(crate) fn put<T>(slot: &mut Option<T>, value: T, item: &Sexpr, unknown: &mut Vec<Unknown>) {
    if slot.is_some() {
        unknown.push(Unknown::new("duplicate", item));
    } else {
        *slot = Some(value);
    }
}
