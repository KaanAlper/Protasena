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
`KaanAlper/Protasena-library`; never Proteus's library. See `NOTICE`.

## License

MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). Protasena is not affiliated with Labcenter
Electronics Ltd. Proteus is a trademark of Labcenter Electronics Ltd.
