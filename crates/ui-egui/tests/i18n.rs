//! Every interface string is in the English catalog (`i18n/en.tsv`), and the catalog holds no
//! string the code no longer uses.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::collections::BTreeSet;
use std::path::Path;

use protasena_engine::Mode;
use protasena_ui_egui::i18n::catalog;
use protasena_ui_egui::menus::model::{Entry, MENUS};
use protasena_ui_egui::toolbars::{TOOLBARS, Tool};

/// The literals of `tl!("…")` in the sources under `dir`.
fn literals(dir: &Path, out: &mut BTreeSet<String>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            literals(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            let src = std::fs::read_to_string(&p).unwrap();
            for (i, _) in src.match_indices("tl!(\"") {
                let rest = &src[i + 5..];
                let end = rest.find("\")").unwrap();
                out.insert(rest[..end].replace("\\\"", "\""));
            }
        }
    }
}

fn menu_labels(entries: &[Entry], out: &mut BTreeSet<String>) {
    for e in entries {
        match e {
            Entry::Item { label, .. } => {
                out.insert((*label).to_owned());
            }
            Entry::Sub { label, entries } => {
                out.insert((*label).to_owned());
                menu_labels(entries, out);
            }
            Entry::Sep | Entry::Sheets => {}
        }
    }
}

fn used() -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    literals(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut keys);
    for m in MENUS {
        keys.insert(m.label.to_owned());
        menu_labels(m.entries, &mut keys);
    }
    for (_, tools) in TOOLBARS {
        for t in tools {
            if let Tool::Button { label, .. } = t {
                keys.insert((*label).to_owned());
            }
        }
    }
    for m in Mode::ALL {
        keys.insert(m.label().to_owned());
        if let Some(c) = protasena_ui_egui::sidepane::caption(m) {
            keys.insert(c.to_owned());
        }
    }
    keys
}

#[test]
fn every_string_shown_is_in_the_catalog() {
    let missing: Vec<String> = used().into_iter().filter(|k| !catalog().contains_key(k.as_str())).collect();
    assert!(missing.is_empty(), "add to i18n/en.tsv:\n{}", missing.join("\n"));
}

#[test]
fn the_catalog_has_no_unused_strings() {
    let used = used();
    let stale: Vec<&&str> = catalog().keys().filter(|k| !used.contains(**k)).collect();
    assert!(stale.is_empty(), "no longer used: {stale:?}");
}
