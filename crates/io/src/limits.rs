//! What the project loader accepts: file size, nesting depth, sheet and item counts.

use protasena_doc::Project;

use crate::error::LoadError;

/// Largest project file read: 256 MiB.
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

/// Deepest nesting of JSON arrays and objects. A project file needs about ten levels.
pub const MAX_DEPTH: usize = 32;

/// Most sheets in a project.
pub const MAX_SHEETS: usize = 4096;

/// Most items (components, wires, …) in a project, all sheets together.
pub const MAX_ITEMS: usize = 4_000_000;

/// Checks the nesting depth of JSON text without parsing it (brackets inside strings do not
/// count), so a hostile file cannot exhaust the parser's stack or our patience.
///
/// # Errors
/// [`LoadError::TooDeep`].
pub(crate) fn check_depth(bytes: &[u8]) -> Result<(), LoadError> {
    let (mut depth, mut in_string, mut escaped) = (0usize, false, false);
    for &b in bytes {
        if in_string {
            match (escaped, b) {
                (true, _) => escaped = false,
                (false, b'\\') => escaped = true,
                (false, b'"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'[' | b'{' => {
                depth += 1;
                if depth > MAX_DEPTH {
                    return Err(LoadError::TooDeep { limit: MAX_DEPTH });
                }
            }
            b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

/// Checks the sheet and item counts of a parsed project (the file size limit bounds the memory
/// parsing takes; these bound the work later stages such as connectivity do).
///
/// # Errors
/// [`LoadError::TooMany`].
pub(crate) fn check_counts(project: &Project) -> Result<(), LoadError> {
    check_counts_within(project, MAX_SHEETS, MAX_ITEMS)
}

fn check_counts_within(project: &Project, max_sheets: usize, max_items: usize) -> Result<(), LoadError> {
    let sheets = project.sheets.len();
    if sheets > max_sheets {
        return Err(LoadError::TooMany { what: "sheets", count: sheets, limit: max_sheets });
    }
    let items = project.sheets.iter().fold(0usize, |n, s| n.saturating_add(s.item_ids().count()));
    if items > max_items {
        return Err(LoadError::TooMany { what: "items", count: items, limit: max_items });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_counts_brackets_outside_strings() {
        assert!(check_depth(b"[[[]]]").is_ok());
        let deep = "[".repeat(MAX_DEPTH + 1);
        assert_eq!(check_depth(deep.as_bytes()), Err(LoadError::TooDeep { limit: MAX_DEPTH }));
        assert!(check_depth("[".repeat(MAX_DEPTH).as_bytes()).is_ok());
        let quoted = format!("[\"{}\\\"{}\"]", "[".repeat(100), "{".repeat(100));
        assert!(check_depth(quoted.as_bytes()).is_ok());
        // Unbalanced closers never underflow.
        assert!(check_depth(b"]]]]}}[").is_ok());
    }

    #[test]
    fn counts_are_checked() {
        let mut p = Project::new("x");
        assert!(check_counts(&p).is_ok());
        let junction = |id| protasena_doc::Junction { id: protasena_doc::JunctionId(id), at: protasena_geom::Pt::ORIGIN };
        p.sheets[0].junctions = (10..13).map(junction).collect();
        assert!(check_counts_within(&p, 1, 3).is_ok());
        assert_eq!(check_counts_within(&p, 1, 2), Err(LoadError::TooMany { what: "items", count: 3, limit: 2 }));
        p.sheets = vec![protasena_doc::Sheet::default(); MAX_SHEETS + 1];
        assert!(matches!(check_counts(&p), Err(LoadError::TooMany { what: "sheets", .. })));
    }
}
