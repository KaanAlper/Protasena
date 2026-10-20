//! `PROJECT.XML`, the project manifest: a small XML file with a timestamp element (release,
//! file version, creation and modification times) and the list of script members.
//!
//! Only the attributes named here are read; every other attribute (one of them is
//! licence-related) is skipped without being stored, so it can never leak into a report.

use crate::error::Error;
use crate::limits::{MAX_MANIFEST_BYTES, MAX_MANIFEST_ENTRIES};

/// Name of the manifest member.
pub const MANIFEST: &str = "PROJECT.XML";

/// What the manifest says about the project.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Manifest {
    /// Program release that saved the project (the release number × 100).
    pub release: Option<u32>,
    /// File format version.
    pub file_version: Option<u32>,
    /// Creation time, Unix seconds.
    pub created: Option<i64>,
    /// Last modification time, Unix seconds.
    pub modified: Option<i64>,
    /// Members listed under the scripts element (relative to `SCRIPTS/`).
    pub scripts: Vec<String>,
}

/// Parses a manifest.
///
/// # Errors
/// [`Error::TooLarge`] above [`MAX_MANIFEST_BYTES`]; [`Error::Format`] when the text has no
/// project element.
pub fn parse_manifest(bytes: &[u8]) -> Result<Manifest, Error> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        return Err(Error::TooLarge { what: MANIFEST.into(), size, limit: MAX_MANIFEST_BYTES as u64 });
    }
    let text = String::from_utf8_lossy(bytes);
    if !text.contains("<PROJECT") {
        return Err(Error::Format { name: MANIFEST.into(), message: "no project element".into() });
    }
    let mut manifest = Manifest::default();
    if let Some(attrs) = element_attributes(&text, "TIMESTAMP") {
        for (name, value) in attributes(attrs) {
            match name {
                "RELEASE" => manifest.release = value.trim().parse().ok(),
                "FILEVER" => manifest.file_version = value.trim().parse().ok(),
                "CREATED" => manifest.created = value.trim().parse().ok(),
                "MODIFIED" => manifest.modified = value.trim().parse().ok(),
                _ => {}
            }
        }
    }
    if let Some(scripts) = between(&text, "<SCRIPTS>", "</SCRIPTS>") {
        manifest.scripts = elements(scripts, "<STRING>", "</STRING>").take(MAX_MANIFEST_ENTRIES).map(unescape).collect();
    }
    Ok(manifest)
}

/// The attribute text of the first `<NAME …>` element.
fn element_attributes<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}");
    let mut from = 0;
    while let Some(at) = text.get(from..)?.find(&open) {
        let start = from.checked_add(at)?.checked_add(open.len())?;
        let rest = text.get(start..)?;
        if rest.starts_with(|c: char| c.is_whitespace() || c == '/' || c == '>') {
            let end = rest.find('>')?;
            return rest.get(..end).map(|a| a.trim_end_matches('/'));
        }
        from = start;
    }
    None
}

/// `name="value"` pairs (either quote), in order; malformed text ends the list.
fn attributes(mut s: &str) -> impl Iterator<Item = (&str, &str)> {
    std::iter::from_fn(move || {
        s = s.trim_start();
        let eq = s.find('=')?;
        let name = s.get(..eq)?.trim();
        let rest = s.get(eq.checked_add(1)?..)?.trim_start();
        let quote = rest.chars().next().filter(|&q| q == '"' || q == '\'')?;
        let body = rest.get(1..)?;
        let close = body.find(quote)?;
        let value = body.get(..close)?;
        s = body.get(close.checked_add(1)?..)?;
        Some((name, value))
    })
}

fn between<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)?.checked_add(open.len())?;
    let rest = text.get(start..)?;
    rest.get(..rest.find(close)?)
}

fn elements<'a>(mut s: &'a str, open: &'a str, close: &'a str) -> impl Iterator<Item = &'a str> {
    std::iter::from_fn(move || {
        let start = s.find(open)?.checked_add(open.len())?;
        let rest = s.get(start..)?;
        let end = rest.find(close)?;
        let value = rest.get(..end)?;
        s = rest.get(end.checked_add(close.len())?..)?;
        Some(value)
    })
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_named_attributes_and_skips_the_rest() {
        let xml = "<?xml version='1.0'?>\n<PROJECT>\n <TIMESTAMP RELEASE=\"900\" OTHER='secret-123' FILEVER=\"901\" \
                   CREATED=\"100\" MODIFIED=\"200\"/>\n <SCRIPTS><STRING>A&amp;B.DAT</STRING><STRING>C.DAT</STRING></SCRIPTS>\n</PROJECT>";
        let m = parse_manifest(xml.as_bytes()).unwrap();
        assert_eq!((m.release, m.file_version, m.created, m.modified), (Some(900), Some(901), Some(100), Some(200)));
        assert_eq!(m.scripts, ["A&B.DAT", "C.DAT"]);
        assert!(!format!("{m:?}").contains("secret"));
    }

    #[test]
    fn tolerates_missing_and_malformed_parts() {
        let m = parse_manifest(b"<PROJECT><TIMESTAMPX/><TIMESTAMP RELEASE=x FILEVER=\"7\"/></PROJECT>").unwrap();
        assert_eq!(m, Manifest::default());
        assert!(parse_manifest(b"<OTHER/>").is_err());
        assert!(parse_manifest(&vec![b' '; MAX_MANIFEST_BYTES + 1]).is_err());
        assert_eq!(parse_manifest(b"<PROJECT><SCRIPTS><STRING>x").unwrap().scripts, Vec::<String>::new());
    }
}
