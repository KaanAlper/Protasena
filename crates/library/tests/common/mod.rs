//! Shared helpers: scratch pack directories and a small hand-made library.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use protasena_geom::Pt;
use protasena_library::{Device, Package, PackageRef, Pad, PadShape, Pin, PinDir, PinKind, Symbol, Unit};

/// A fresh empty directory under Cargo's per-target scratch area, removed on drop.
pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new(name: &str) -> Scratch {
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("library-{name}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    pub fn write(&self, file: &str, text: &str) {
        std::fs::write(self.0.join(file), text).unwrap();
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A two-pin device in `library` using package `pkg` (pins "1", "2" → pads "1", "2").
pub fn two_pin(name: &str, library: &str, pkg: &str) -> Device {
    let pins = vec![
        Pin::new("1", "A", PinKind::Passive, Pt::new(-100.0, 0.0), PinDir::Right, 100.0),
        Pin::new("2", "B", PinKind::Passive, Pt::new(100.0, 0.0), PinDir::Left, 100.0),
    ];
    Device {
        name: name.into(),
        library: library.into(),
        prefix: "X".into(),
        description: format!("test part {name}"),
        category: "Tests".into(),
        symbol: Symbol { units: vec![Unit { name: String::new(), body_style: 1, graphics: vec![], pins }], power: false }.into(),
        packages: vec![PackageRef { name: pkg.into(), pin_map: vec![("1".into(), "1".into()), ("2".into(), "2".into())].into() }],
        default_package: Some(pkg.into()),
        ..Device::default()
    }
}

/// A two-pad package.
pub fn two_pad(name: &str, library: &str) -> Package {
    Package {
        name: name.into(),
        library: library.into(),
        description: String::new(),
        pads: vec![
            Pad::through_hole("1", Pt::new(-50.0, 0.0), PadShape::Rect, 60.0, 30.0),
            Pad::through_hole("2", Pt::new(50.0, 0.0), PadShape::Circle, 60.0, 30.0),
        ],
        graphics: vec![],
    }
}

/// `index.json` text for `(name, file, device_count, package_file)` entries.
pub fn index(entries: &[(&str, &str, u64, Option<&str>)]) -> String {
    let libs: Vec<String> = entries
        .iter()
        .map(|(name, file, count, pkg)| {
            let pkg = pkg.map(|p| format!(",\"package_file\":\"{p}\"")).unwrap_or_default();
            format!("{{\"name\":\"{name}\",\"file\":\"{file}\",\"device_count\":{count}{pkg}}}")
        })
        .collect();
    format!("{{\"format\":\"protasena-library\",\"version\":1,\"libraries\":[{}]}}", libs.join(","))
}

/// A device file with these devices.
pub fn device_file(devices: &[Device]) -> String {
    format!("{{\"format\":\"protasena-library-devices\",\"version\":1,\"devices\":{}}}", serde_json::to_string(devices).unwrap())
}

/// A package file with these packages.
pub fn package_file(packages: &[Package]) -> String {
    format!("{{\"format\":\"protasena-library-packages\",\"version\":1,\"packages\":{}}}", serde_json::to_string(packages).unwrap())
}
