//! Interface strings. Every string the window shows goes through [`t`] (literals through the
//! [`tl!`](crate::tl) macro), keyed by its English text. The catalog is
//! `crates/ui-egui/i18n/en.tsv` (`key<TAB>text` lines); only English exists so far, and a test
//! checks that every string the code uses is in it.
//!
//! Command ids, mode ids and design data never pass through here.

use std::collections::HashMap;
use std::sync::OnceLock;

/// The English catalog.
const EN: &str = include_str!("../i18n/en.tsv");

/// Most catalog lines read (the file is ours, but nothing here may be unbounded).
const MAX_LINES: usize = 20_000;

/// A literal interface string, translated: `tl!("Pick Devices")`.
#[macro_export]
macro_rules! tl {
    ($s:literal) => {
        $crate::i18n::t($s)
    };
}

/// The parsed catalog: key → text.
pub fn catalog() -> &'static HashMap<&'static str, &'static str> {
    static CATALOG: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    CATALOG.get_or_init(|| parse(EN))
}

/// Parses `key<TAB>text` lines; blank lines and lines starting with `#` are skipped, a line
/// without a tab maps the key to itself.
pub fn parse(src: &'static str) -> HashMap<&'static str, &'static str> {
    src.lines()
        .take(MAX_LINES)
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| match l.split_once('\t') {
            Some((k, v)) => (k, v),
            None => (l, l),
        })
        .collect()
}

/// The interface text for `key` (the key itself when the catalog has no entry).
pub fn t(key: &'static str) -> &'static str {
    catalog().get(key).copied().unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_map_keys_to_texts() {
        let c = parse("# comment\nOK\nCancel\tAbbrechen\n\n");
        assert_eq!(c.get("OK"), Some(&"OK"));
        assert_eq!(c.get("Cancel"), Some(&"Abbrechen"));
        assert_eq!(c.len(), 2);
        assert_eq!(t("a string nobody wrote"), "a string nobody wrote");
    }
}
