//! The two text structures inside `ROOT.DSN`: brace-delimited property blocks
//! (`{KEY=VALUE}` lines) and the sectioned text of embedded device definitions.

use crate::limits::MAX_TEXT_LINES;

/// One `{KEY=VALUE}` line (or a bare line inside a section).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRecord {
    /// Text before the first `=` (the whole content when there is none), trimmed.
    pub key: String,
    /// Text after the first `=`, trimmed; raw (quotes and commas kept).
    pub value: String,
    /// Whether the line was enclosed in braces.
    pub braced: bool,
}

/// One section of a device definition: a `{*NAME}` line (or a `*NAME args` line) and the
/// records up to the next section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSection {
    /// Section name without the star (`DEVICE`, `PROPDEFS`, `INDEX`, `COMPONENT`, …).
    pub name: String,
    /// Words after the name on a `*NAME args` line; empty for `{*NAME}`.
    pub args: String,
    /// The records in order.
    pub records: Vec<TextRecord>,
}

impl TextSection {
    /// The value of the first record with this key (compared without case).
    pub fn get(&self, key: &str) -> Option<&str> {
        self.records.iter().find(|r| r.key.eq_ignore_ascii_case(key)).map(|r| r.value.as_str())
    }
}

/// Parses a `{KEY=VALUE}` block into pairs, in order. Lines without braces or without a key
/// are skipped.
pub fn parse_property_block(text: &str) -> Vec<(String, String)> {
    lines(text).filter_map(record).filter(|r| r.braced && !r.key.is_empty()).map(|r| (r.key, r.value)).collect()
}

/// Parses the sectioned text of a device definition. Records before the first section go into
/// a section with an empty name.
pub fn parse_sections(text: &str) -> Vec<TextSection> {
    let mut sections: Vec<TextSection> = Vec::new();
    for line in lines(text) {
        if let Some((name, args)) = section_header(line) {
            sections.push(TextSection { name, args, records: Vec::new() });
            continue;
        }
        let Some(rec) = record(line) else { continue };
        match sections.last_mut() {
            Some(section) => section.records.push(rec),
            None => sections.push(TextSection { name: String::new(), args: String::new(), records: vec![rec] }),
        }
    }
    sections
}

fn lines(text: &str) -> impl Iterator<Item = &str> {
    text.split('\n').take(MAX_TEXT_LINES).map(|l| l.trim_end_matches('\r').trim())
}

fn section_header(line: &str) -> Option<(String, String)> {
    if let Some(name) = line.strip_prefix("{*").and_then(|l| l.strip_suffix('}')) {
        return Some((name.trim().to_owned(), String::new()));
    }
    let rest = line.strip_prefix('*')?;
    let (name, args) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    (!name.is_empty()).then(|| (name.to_owned(), args.trim().to_owned()))
}

fn record(line: &str) -> Option<TextRecord> {
    if line.is_empty() {
        return None;
    }
    let (inner, braced) = match line.strip_prefix('{').and_then(|l| l.strip_suffix('}')) {
        Some(inner) => (inner, true),
        None => (line, false),
    };
    let (key, value) = inner.split_once('=').unwrap_or((inner, ""));
    Some(TextRecord { key: key.trim().to_owned(), value: value.trim().to_owned(), braced })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_blocks_keep_order_and_raw_values() {
        let pairs = parse_property_block("{A=1}\n{B = x,y}\r\nloose\n{=skip}\n{C=}\n");
        assert_eq!(pairs, [("A".into(), "1".into()), ("B".into(), "x,y".into()), ("C".into(), String::new())]);
        assert!(parse_property_block("").is_empty());
    }

    #[test]
    fn sections_collect_their_records() {
        let text = "{X=0}\n{*DEVICE}\n{PREFIX=X}\n\n{*PROPDEFS}\n{VALUE=\"Value\",STRING}\n*PINOUT PKG1\n{PIN \"1\" = 1}\nloose=1\n";
        let s = parse_sections(text);
        let names: Vec<_> = s.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["", "DEVICE", "PROPDEFS", "PINOUT"]);
        assert_eq!(s[1].get("prefix"), Some("X"));
        assert_eq!(s[2].records[0].value, "\"Value\",STRING");
        assert_eq!(s[3].args, "PKG1");
        assert_eq!(s[3].records[0].key, "PIN \"1\"");
        assert!(!s[3].records[1].braced);
        assert_eq!(parse_sections("*")[0].records[0].key, "*");
    }
}
