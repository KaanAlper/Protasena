//! The `.psproj` text: encoding a [`Project`] and decoding it with checks.

use protasena_doc::{DocError, Project};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::error::Category;

use crate::error::{LoadError, SaveError};
use crate::{limits, pretty};

/// Format tag of a project file.
pub const FORMAT: &str = "protasena-project";
/// Version of the project file format this build writes, and the newest it reads.
pub const VERSION: u32 = 1;
/// File name extension of project files (without the dot).
pub const EXTENSION: &str = "psproj";

/// What loading found that did not stop it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LoadReport {
    /// The file's format version (older versions are read and saved back as [`VERSION`]).
    pub version: u32,
    /// Problems that leave the project usable: malformed items, a stale id counter, no sheets.
    pub warnings: Vec<DocError>,
}

#[derive(Serialize)]
struct FileOut<'a> {
    format: &'a str,
    version: u32,
    project: &'a Project,
}

/// The tag fields alone, read before the project so a foreign file gets a clear error.
#[derive(Deserialize)]
struct Header {
    format: Option<Value>,
    version: Option<Value>,
}

#[derive(Deserialize)]
struct Body {
    project: Option<Project>,
}

/// The project as `.psproj` text: pretty JSON (two-space indent, arrays of plain values such as
/// points on one line, fields in a fixed order, sheets and items in the model's list order,
/// numbers written so they read back exactly) with a final newline. The same project always gives the same text.
///
/// # Errors
/// [`SaveError::NonFinite`] when a coordinate or size is NaN or infinite (JSON cannot hold it);
/// [`SaveError::Json`] if serialization fails otherwise.
pub fn encode(project: &Project) -> Result<String, SaveError> {
    if let Some(what) = project.issues().into_iter().find_map(|e| if let DocError::NonFinite(what) = e { Some(what) } else { None }) {
        return Err(SaveError::NonFinite(what));
    }
    let file = FileOut { format: FORMAT, version: VERSION, project };
    let mut text = pretty::to_string(&file).map_err(SaveError::Json)?;
    text.push('\n');
    Ok(text)
}

/// Reads `.psproj` bytes. Unknown fields are ignored (a newer minor revision may add them);
/// a newer format version is refused.
///
/// # Errors
/// See [`LoadError`]: size, depth and count limits, malformed JSON, a foreign or newer file,
/// and duplicate ids ([`LoadError::Invalid`]). Other inconsistencies are
/// [`LoadReport::warnings`].
pub fn decode(bytes: &[u8]) -> Result<(Project, LoadReport), LoadError> {
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if size > limits::MAX_FILE_BYTES {
        return Err(LoadError::TooLarge { size, limit: limits::MAX_FILE_BYTES });
    }
    limits::check_depth(bytes)?;
    let version = check_header(bytes)?;
    let body: Body = serde_json::from_slice(bytes).map_err(|e| LoadError::Json(e.to_string()))?;
    let project = body.project.ok_or_else(|| LoadError::NotAProject("there is no project".to_owned()))?;
    limits::check_counts(&project)?;
    let (errors, mut warnings): (Vec<_>, Vec<_>) = project.issues().into_iter().partition(is_fatal);
    if !errors.is_empty() {
        return Err(LoadError::Invalid(errors));
    }
    if project.sheets.is_empty() {
        warnings.push(DocError::Invalid("the project has no sheets".to_owned()));
    }
    Ok((project, LoadReport { version, warnings }))
}

/// Checks the format tag and version; returns the version.
fn check_header(bytes: &[u8]) -> Result<u32, LoadError> {
    // A struct also deserializes from an array; only an object is a project file.
    if bytes.iter().find(|b| !b" \t\r\n".contains(b)) != Some(&b'{') {
        return Err(LoadError::NotAProject("the file is not a JSON object".to_owned()));
    }
    let header: Header = serde_json::from_slice(bytes).map_err(|e| match e.classify() {
        Category::Data => LoadError::NotAProject("the file is not a JSON object".to_owned()),
        Category::Io | Category::Syntax | Category::Eof => LoadError::Json(e.to_string()),
    })?;
    match header.format {
        Some(Value::String(tag)) if tag == FORMAT => {}
        Some(Value::String(tag)) => return Err(LoadError::NotAProject(format!("format is \"{tag}\", not \"{FORMAT}\""))),
        Some(_) => return Err(LoadError::NotAProject("format is not a string".to_owned())),
        None => return Err(LoadError::NotAProject("there is no format tag".to_owned())),
    }
    let found = match header.version {
        Some(v) => v.as_u64().ok_or_else(|| LoadError::NotAProject(format!("version {v} is not a whole number")))?,
        None => return Err(LoadError::NotAProject("there is no version".to_owned())),
    };
    match u32::try_from(found) {
        Ok(v) if (1..=VERSION).contains(&v) => Ok(v),
        _ => Err(LoadError::UnsupportedVersion { found, supported: VERSION }),
    }
}

/// Problems that make a loaded project unusable: ids must be unique for operations to address
/// items, and every number must be finite.
fn is_fatal(e: &DocError) -> bool {
    matches!(e, DocError::DuplicateSheet(_) | DocError::DuplicateItem(_) | DocError::NonFinite(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_project_round_trips() {
        let p = Project::new("empty");
        let text = encode(&p).unwrap();
        let (back, report) = decode(text.as_bytes()).unwrap();
        assert_eq!(back, p);
        assert_eq!(report, LoadReport { version: VERSION, warnings: vec![] });
    }

    #[test]
    fn the_header_is_checked_before_the_project() {
        let wrong = br#"{"format": "other", "version": 1, "project": {"sheets": 5}}"#;
        assert!(matches!(decode(wrong), Err(LoadError::NotAProject(m)) if m.contains("other")));
        let newer = br#"{"format": "protasena-project", "version": 99, "project": {"sheets": 5}}"#;
        assert_eq!(decode(newer), Err(LoadError::UnsupportedVersion { found: 99, supported: VERSION }));
        let huge = br#"{"format": "protasena-project", "version": 18446744073709551615}"#;
        assert!(matches!(decode(huge), Err(LoadError::UnsupportedVersion { .. })));
        let bad_project = br#"{"format": "protasena-project", "version": 1, "project": {"sheets": 5}}"#;
        assert!(matches!(decode(bad_project), Err(LoadError::Json(_))));
    }
}
