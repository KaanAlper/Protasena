# Protasena

Protasena is an open-source, clean-room electronics design suite written in pure Rust whose
tools, settings and workflow follow Labcenter Proteus 8 Professional: schematic capture first,
then circuit simulation and PCB layout. It is meant to run on Linux, Windows and macOS.

## Status

Pre-alpha: the workspace, rules and CI are in place; the application itself is not built yet.
Goals and milestones: [docs/specs/2026-10-20-protasena-design.md](docs/specs/2026-10-20-protasena-design.md);
current plan: [docs/plans/2026-10-20-m1-schematic.md](docs/plans/2026-10-20-m1-schematic.md).
Parity with Proteus is tracked in `docs/*-parity.md`, known shortfalls in `docs/gaps.md`.

## Build

    cargo build
    cargo xtask ci        # fmt, clippy, tests, layering and asset checks; run before every commit

Contributors and agents: read [AGENTS.md](AGENTS.md) first.

## Parts library

Built from KiCad's official libraries (CC-BY-SA 4.0) and distributed separately in
[KaanAlper/Protasena-library](https://github.com/KaanAlper/Protasena-library); never Proteus's
library. See `NOTICE`. To use it, clone that repository and point `PROTASENA_LIBRARY_DIR` at its
`pack/` directory:

    git clone https://github.com/KaanAlper/Protasena-library
    export PROTASENA_LIBRARY_DIR=$PWD/Protasena-library/pack

Without it Protasena runs with its small built-in set of generic parts. The pack is produced by
`apps/protasena-libconv` (`cargo run --release -p protasena-libconv -- --help`; the conversion
code is the `crates/libimport` crate) using the category and naming table in
`apps/protasena-libconv/data/map.toml`.

## License

MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). Protasena is not affiliated with Labcenter
Electronics Ltd. Proteus is a trademark of Labcenter Electronics Ltd.
