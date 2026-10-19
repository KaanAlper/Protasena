use std::path::{Path, PathBuf};

use super::*;
use DepKind::*;

fn c(name: &str, deps: &[(&str, DepKind, bool)]) -> Crate {
    Crate { name: name.into(), deps: deps.iter().map(|(n, k, w)| Dep { name: (*n).into(), kind: *k, workspace: *w }).collect() }
}

#[test]
fn clean_downward_graph_passes() {
    let g = [
        c("protasena-geom", &[("serde", Normal, false)]),
        c("protasena-doc", &[("protasena-geom", Normal, true)]),
        c("protasena-netlist", &[("protasena-doc", Normal, true), ("protasena-library", Normal, true)]),
        c("protasena-io", &[("protasena-netlist", Normal, true), ("protasena-pdsprj", Normal, true)]),
        c("protasena-engine", &[("protasena-io", Normal, true), ("protasena-geom", Dev, true)]),
        c("protasena-ui-egui", &[("protasena-engine", Normal, true), ("egui", Normal, false)]),
        c("protasena", &[("protasena-ui-egui", Normal, true), ("eframe", Normal, false)]),
    ];
    assert!(check(&g).is_empty(), "{:?}", check(&g));
}

#[test]
fn upward_dependency_flagged() {
    let v = check(&[c("protasena-doc", &[("protasena-engine", Normal, true)])]);
    assert!(matches!(v[..], [Violation::Upward { from: 1, to: Some(4), .. }]), "{v:?}");
}

#[test]
fn same_layer_dependency_flagged() {
    let v = check(&[c("protasena-render", &[("protasena-netlist", Normal, true)])]);
    assert!(matches!(v[..], [Violation::Upward { from: 2, to: Some(2), .. }]), "{v:?}");
    let v = check(&[c("protasena-doc", &[("protasena-library", Normal, true)])]);
    assert!(matches!(v[..], [Violation::Upward { from: 1, to: Some(1), .. }]), "{v:?}");
}

#[test]
fn upward_dev_and_build_dependencies_flagged() {
    let v = check(&[c("protasena-geom", &[("protasena-doc", Dev, true)])]);
    assert!(matches!(v[..], [Violation::Upward { kind: Dev, .. }]), "{v:?}");
    let v = check(&[c("protasena-io", &[("protasena-engine", Build, true)])]);
    assert!(matches!(v[..], [Violation::Upward { kind: Build, .. }]), "{v:?}");
}

#[test]
fn library_crate_may_not_depend_on_an_app() {
    let v = check(&[c("protasena-engine", &[("protasena-cli", Normal, true)])]);
    assert!(matches!(v[..], [Violation::Upward { to: None, .. }]), "{v:?}");
    assert!(v[0].to_string().contains("(app)"));
}

#[test]
fn standalone_crates_have_no_workspace_deps() {
    for krate in ["protasena-kicad", "protasena-pdsprj"] {
        let v = check(&[c(krate, &[("protasena-geom", Normal, true)])]);
        assert!(matches!(v[..], [Violation::StandaloneHasWorkspaceDep { .. }]), "{krate}: {v:?}");
    }
}

#[test]
fn self_dev_dependency_ignored() {
    assert!(check(&[c("protasena-doc", &[("protasena-doc", Dev, true)])]).is_empty());
}

#[test]
fn ui_crates_below_l5_flagged() {
    for dep in ["egui", "eframe", "egui-wgpu", "egui_kittest", "wgpu", "winit", "rfd", "bevy_ecs"] {
        let v = check(&[c("protasena-engine", &[(dep, Normal, false)])]);
        assert!(matches!(v[..], [Violation::UiBelowMinLayer { layer: 4, .. }]), "{dep}: {v:?}");
    }
    assert!(check(&[c("protasena-engine", &[("eguilike", Normal, false)])]).is_empty());
    assert!(check(&[c("protasena-ui-egui", &[("winit", Normal, false)])]).is_empty());
}

#[test]
fn unregistered_crate_is_error() {
    let v = check(&[c("protasena-mystery", &[])]);
    assert!(matches!(&v[..], [Violation::Unregistered { krate }] if krate == "protasena-mystery"), "{v:?}");
    assert!(v[0].to_string().contains("register"));
}

#[test]
fn apps_and_xtask_exempt() {
    for app in ["protasena", "protasena-cli", "protasena-libconv", "xtask"] {
        assert!(check(&[c(app, &[("egui", Normal, false), ("protasena-ui-egui", Normal, true)])]).is_empty(), "{app}");
    }
}

#[test]
fn table_matches_the_spec() {
    let layer = |n: &str| classify(&format!("{PREFIX}{n}")).and_then(Class::layer);
    for (n, l) in [("geom", 0), ("kicad", 0), ("pdsprj", 0), ("doc", 1), ("library", 1), ("render", 2), ("netlist", 2)] {
        assert_eq!(layer(n), Some(l), "{n}");
    }
    for (n, l) in [("io", 3), ("engine", 4), ("ui-egui", 5)] {
        assert_eq!(layer(n), Some(l), "{n}");
    }
}

#[test]
fn metadata_parsing() {
    let meta: Value = serde_json::from_str(
        r#"{"packages":[
            {"name":"protasena-doc","dependencies":[
                {"name":"protasena-geom","kind":null,"path":"/x/crates/geom"},
                {"name":"serde","kind":null},
                {"name":"proptest","kind":"dev"}]},
            {"name":"protasena-geom","dependencies":[]}
        ]}"#,
    )
    .unwrap();
    let g = from_metadata(&meta).unwrap();
    assert_eq!(g.len(), 2);
    let doc = g.iter().find(|c| c.name == "protasena-doc").unwrap();
    assert!(doc.deps[0].workspace && !doc.deps[1].workspace);
    assert_eq!(doc.deps[2].kind, Dev);
    assert!(check(&g).is_empty());
    assert!(from_metadata(&serde_json::json!({})).is_err());
}

/// A throwaway workspace with `protasena-geom`, `protasena-doc` and `protasena-engine`, where
/// `doc` depends on `doc_dep`. Removed on drop.
struct TempWorkspace(PathBuf);

impl TempWorkspace {
    fn new(tag: &str, doc_dep: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("protasena-xtask-layers-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let write = |rel: &str, text: &str| {
            let p = dir.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        write("Cargo.toml", "[workspace]\nresolver = \"3\"\nmembers = [\"crates/*\"]\n");
        for (name, deps) in
            [("geom", String::new()), ("doc", format!("protasena-{doc_dep} = {{ path = \"../{doc_dep}\" }}\n")), ("engine", String::new())]
        {
            write(
                &format!("crates/{name}/Cargo.toml"),
                &format!("[package]\nname = \"protasena-{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\n{deps}"),
            );
            write(&format!("crates/{name}/src/lib.rs"), "");
        }
        TempWorkspace(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn layers_run_rejects_a_workspace_with_an_upward_dependency() {
    let bad = TempWorkspace::new("bad", "engine");
    let err = run(bad.path()).unwrap_err();
    assert!(err.contains("1 layering violation"), "{err}");
    let crates = from_metadata(&metadata(bad.path()).unwrap()).unwrap();
    let v = check(&crates);
    assert!(
        matches!(&v[..], [Violation::Upward { krate, dep, from: 1, to: Some(4), kind: Normal }]
            if krate == "protasena-doc" && dep == "protasena-engine"),
        "{v:?}"
    );
}

#[test]
fn layers_run_accepts_a_workspace_with_a_downward_dependency() {
    let good = TempWorkspace::new("good", "geom");
    run(good.path()).unwrap();
}

#[test]
fn this_workspace_passes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    run(root).unwrap();
}
