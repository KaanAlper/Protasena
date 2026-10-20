//! Atomic writes: a failed write leaves the old file (and its backup) as they were.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use std::io::{self, Write};

use common::{sample_project, scratch};
use protasena_io::{SaveError, atomic_write, backup_path, save};

fn names(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

#[test]
fn a_failing_writer_leaves_the_old_file_and_no_temp_files() {
    let dir = scratch("failing-writer");
    let path = dir.join("design.psproj");
    std::fs::write(&path, b"old contents").unwrap();
    let err = atomic_write(&path, true, |out: &mut dyn Write| {
        out.write_all(b"half of the new")?;
        Err(io::Error::other("disk full"))
    })
    .unwrap_err();
    assert!(err.to_string().contains("disk full"), "{err}");
    assert_eq!(std::fs::read(&path).unwrap(), b"old contents");
    assert_eq!(names(&dir), ["design.psproj"], "no backup and no temp file after a failed write");
}

#[test]
fn a_successful_write_replaces_the_file_and_keeps_a_backup_on_request() {
    let dir = scratch("successful-writer");
    let path = dir.join("out.txt");
    atomic_write(&path, true, |out: &mut dyn Write| out.write_all(b"one")).unwrap();
    assert_eq!(names(&dir), ["out.txt"]);
    atomic_write(&path, true, |out: &mut dyn Write| out.write_all(b"two")).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"two");
    assert_eq!(std::fs::read(backup_path(&path)).unwrap(), b"one");
    atomic_write(&path, false, |out: &mut dyn Write| out.write_all(b"three")).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"three");
    assert_eq!(std::fs::read(backup_path(&path)).unwrap(), b"one", "no new backup when not asked for");
}

#[test]
fn saving_into_a_missing_directory_fails_cleanly() {
    let dir = scratch("missing-dir");
    let path = dir.join("no").join("such").join("dir.psproj");
    match save(&sample_project(), &path) {
        Err(SaveError::Io { path: p, .. }) => assert_eq!(p, path),
        other => panic!("{other:?}"),
    }
    assert!(names(&dir).is_empty());
}

#[test]
fn saving_onto_a_directory_fails_and_leaves_it() {
    let dir = scratch("onto-dir");
    let target = dir.join("taken.psproj");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("inside"), b"x").unwrap();
    assert!(save(&sample_project(), &target).is_err());
    assert_eq!(std::fs::read(target.join("inside")).unwrap(), b"x");
    assert_eq!(names(&dir), ["taken.psproj"]);
}

#[cfg(unix)]
#[test]
fn saving_into_a_read_only_directory_leaves_the_old_project() {
    use protasena_io::load;
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("read-only");
    let path = dir.join("design.psproj");
    let first = sample_project();
    save(&first, &path).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    // Root ignores directory permissions; the test only means something for other users.
    let probe = std::fs::File::create(dir.join("probe")).is_err();
    let mut second = first.clone();
    second.name = "second".into();
    let result = save(&second, &path);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    if probe {
        assert!(result.is_err());
        assert_eq!(load(&path).unwrap().0, first);
        assert_eq!(names(&dir), ["design.psproj"]);
    }
}
