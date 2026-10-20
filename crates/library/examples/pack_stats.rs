//! Loads a pack and prints load time, Pick Devices search times and (on Linux) the peak and
//! current resident memory:
//!
//! ```text
//! cargo run --release -p protasena-library --example pack_stats -- PACK_DIR [QUERY…]
//! ```

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::PathBuf;
use std::time::Instant;

use protasena_library::{Library, Query};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(dir) = args.next().map(PathBuf::from) else {
        eprintln!("usage: pack_stats PACK_DIR [QUERY…]");
        std::process::exit(2);
    };
    let t = Instant::now();
    // As the application does: the built-in parts with the pack laid over them.
    let lib = match Library::load_dir(&dir) {
        Ok(pack) => Library::builtin().overlay(pack),
        Err(e) => {
            eprintln!("load failed: {e}");
            std::process::exit(1);
        }
    };
    println!("loaded {} devices, {} libraries in {:.2?}", lib.len(), lib.libraries().len(), t.elapsed());
    memory("after load");
    let queries: Vec<String> = args.collect();
    let queries = if queries.is_empty() { vec!["res".to_owned(), "op amp".to_owned(), "stm32".to_owned(), "74ls".to_owned()] } else { queries };
    for q in &queries {
        let t = Instant::now();
        let hits = lib.search(&Query::keywords(q)).len();
        println!("search {q:?}: {hits} hits in {:.2?}", t.elapsed());
    }
    let t = Instant::now();
    let cats = lib.categories().len();
    println!("categories: {cats} in {:.2?}", t.elapsed());
    for name in ["RES", "STM32F407VGTx", "74LS00"] {
        let t = Instant::now();
        let found = lib.get(name).map(|d| (d.symbol.pins().count(), d.default_package.clone()));
        let pkg = found.as_ref().and_then(|(_, p)| p.as_deref()).and_then(|p| lib.package(p)).map(|p| p.pads.len());
        println!("get {name}: {found:?} package pads {pkg:?} in {:.2?}", t.elapsed());
    }
    memory("at the end");
}

/// Peak (`VmHWM`) and current (`VmRSS`) resident memory from `/proc/self/status`.
fn memory(when: &str) {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else { return };
    let field = |key: &str| status.lines().find(|l| l.starts_with(key)).map(|l| l.trim_start_matches(key).trim().to_owned()).unwrap_or_default();
    println!("memory {when}: peak {}, current {}", field("VmHWM:"), field("VmRSS:"));
}
