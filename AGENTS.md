# Protasena — instructions for agents

Protasena is an open-source, clean-room, pure-Rust electronics design suite whose tools, settings
and workflow follow **Labcenter Proteus 8 Professional (Windows)**: schematic capture first, then
simulation and PCB layout. It runs on Linux, Windows and macOS. Code: MIT OR Apache-2.0.

**Proteus's behaviour is the baseline.** Build what Proteus does, the way it does it. Where we
can do something better or easier, propose it to the owner first and build it only once
approved; approved differences are marked ★ in the parity docs. The look (colours, icons, fonts)
is our own; layout, order and behaviour come from Proteus.

This file is the single source of agent instructions. `CLAUDE.md` contains only `@AGENTS.md`
(Claude Code imports this file through it); edit `AGENTS.md`, never `CLAUDE.md`.

## Start every session here
`plan/` is local and gitignored (layout in `.claude/rules/parity-method.md`); never commit it,
never quote its contents in commits, PRs or docs.
1. Read `plan/STATUS.md`. If it is missing, create it with these lines (ask the owner for values
   you don't know):
   - `Milestone:` the current milestone (see the design spec below).
   - `Next task:` what to do next.
   - `Reference PC:` a note about the reference PC. Only the exact line `Reference PC: disposable`
     allows force-closing Proteus there.
   - `Proteus exe:` path and version of the reference Proteus.
2. Check `plan/proteus/env.sh`. Every shell block that talks to the reference PC starts with
   `. plan/proteus/env.sh; AS=${AS:-asenascale}`. If it is missing, create it (ask the owner):
   ```sh
   export PROTASENA_REF=<the reference PC's AsenaScale device name>
   export AS=/path/to/asenascale   # optional; default: asenascale on PATH (needs `exec`)
   ```
3. Goals and milestones: `docs/specs/2026-10-20-protasena-design.md`. Current plan:
   `docs/plans/2026-10-20-m1-schematic.md`. Work list: `docs/gaps.md`.
4. Licence limits and credits: `NOTICE`, `ATTRIBUTION.md`.

## Never crash
Outranks feature work. Standard: [`docs/standards/never-crash.md`](docs/standards/never-crash.md).
No panics in non-test code, no `unsafe`, errors through `Result`, input is hostile (files,
libraries, command and control parameters), every crash fix lands with a regression test.

## Clean-room (the most serious rule)
Proteus runs on the owner's Windows PC and may only be **observed** from the outside: run it, use
it with synthetic designs, take screenshots of its window and UI Automation dumps. Details:
[`.claude/rules/clean-room.md`](.claude/rules/clean-room.md).
- **Never** read, disassemble or copy anything inside the Proteus installation (file names only).
  Never copy its icons, colours, artwork, help text, libraries (symbols, packages, simulation
  models) or sample designs.
- **Never commit** Proteus screenshots, dumps, notes or files Proteus produced. They live only
  under `plan/proteus/`. This repo is public.
- **Privacy:** never commit the reference PC's device name, IPs, Windows user paths
  (`C:\Users\…`), e-mail addresses, licence holder or customer details Proteus shows.
- **GPL code is banned:** never copy, link or read KiCad's application code, gEDA or Fritzing.
  KiCad's *file format documentation* (dev-docs.kicad.org) may be read. KiCad's libraries enter
  only as data, through the separate `KaanAlper/Protasena-library` repo (CC-BY-SA 4.0), never as
  files in this repo or a Cargo dependency.
- `.pdsprj` is worked out only from files Proteus saves from our own synthetic designs; samples
  stay in `plan/proteus/samples/`, tests use files we write ourselves.
- Every bundled asset is original or openly licensed and has a row in `ATTRIBUTION.md`
  (`cargo xtask assets` enforces it).

## Architecture rules
- **Layering** (`cargo xtask layers`, table in `xtask/src/layers.rs`): a crate may depend only on
  crates in strictly lower layers; L0 crates on no workspace crate.

  | Layer | Crates | Role |
  |---|---|---|
  | L0 | `geom`, `kicad`, `pdsprj` | 2D geometry; KiCad file reader; `.pdsprj` container (no workspace deps) |
  | L1 | `doc`, `library` | Design model; device model and library loader |
  | L2 | `render`, `netlist` | Drawing; connectivity and netlists |
  | L3 | `io` | Project files (own format, `.pdsprj`), netlist export |
  | L4 | `engine` | Commands, session, undo; Proteus's tools and modes |
  | L5 | `ui-egui` | Proteus's window layout with our theme; control channel |
  | apps | `protasena`, `protasena-cli`, `protasena-libconv`, `xtask` | exempt |

  Nothing below L5 depends on egui/eframe/winit/wgpu/rfd. A new crate is registered in the
  table (and here) before it lands. Changing the table is an architecture decision: ask the owner.
- **Everything is a command.** User-visible behaviour is a `CommandSpec` in `crates/engine`
  (id, label, menu path, shortcut, params doc, `enabled`, JSON `run`, optional interactive
  machine) plus tests. Menus, toolbars, keys, CLI and the control channel reach the same commands.
- **Programmatic calls never open dialogs.** The JSON form runs with defaults.
- **The UI is thin**: panels read engine state and act through commands. Colours come from theme
  tokens; strings go through i18n from the start.
- **Rust only.** Library crates are `protasena-*` (directories under `crates/` without the
  prefix); environment variables are `PROTASENA_*`.

## The parity loop
Every feature is done the same way (details in
[`.claude/rules/parity-method.md`](.claude/rules/parity-method.md)):
1. **Observe** Proteus on the reference PC (synthetic design, keystrokes → screenshots via
   `tools/proteus/shot-proteus.sh`, UIA dumps, notes in `plan/proteus/notes/`).
2. **Build** it in Protasena as commands and UI.
3. **Compare** side by side.
4. **Record** in the matching `docs/*-parity.md` row: ✅ matches, ◐ partial (differences
   listed), ✗ missing, ★ intentional improvement approved by the owner.
A feature is not done until step 3 shows no differences, or the remaining ones are listed in
`docs/gaps.md`. Never mark ★ without the owner's approval.

## Running the app
The application does not exist yet (M1 Task 9 builds it). Today the apps are placeholders that
answer `--help`/`--version`: `cargo run -p protasena-cli -- --help`. When the UI lands, this
section documents `--control PORT` (JSON-lines control channel: `ui.screenshot`, `ui.click`,
`engine.execute`, `ui.elements`); look at screenshots of every UI change.

**Parts library**: the converted KiCad pack lives outside this repo
(`KaanAlper/Protasena-library`, CC-BY-SA 4.0). Point `PROTASENA_LIBRARY_DIR` at its `pack/`
directory (`index.json` inside); without it only the built-in generic parts exist, and tests
never need it. Regenerate it with `protasena-libconv` (command and pinned KiCad tag in that
repo's README; mapping table `apps/protasena-libconv/data/map.toml`); never copy pack or KiCad
files into this repo.

**Proteus helpers** in `tools/proteus/` (see its `README.md`): `shot-proteus.sh` is the only way
to screenshot Proteus (it crops to Proteus's window on the PC; never call `asenascale shot`
yourself); `uia-dump.ps1` dumps the UI Automation tree of Proteus's main window. Their output
goes only to `plan/proteus/`.

## Quality gate
`cargo xtask ci` (fmt --check, clippy -D warnings, tests, layers, assets) passes before every
commit. Tests first (red → green), then refactor. Files stay under ~400 lines; split by
responsibility. Before committing anything that touches assets, UI strings, `docs/`, test
fixtures or `tools/proteus/`, check the diff against the clean-room rules above.

## Pull requests
Work on a branch, conventional commits (`feat:`, `fix:`, `docs:`, `chore:`, `test:`,
`refactor:`, `ci:`), small logical commits. Open a PR to `KaanAlper/Protasena` `main`; never push
to `main`, never force-push. With `gh`, always pass `-R KaanAlper/Protasena`.

## Disk discipline
Disk is limited. Use one shared target dir and slim debug info:
```sh
export CARGO_TARGET_DIR=<repo>/target CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0
```
No other target dirs (parallel agents included: share it, edit only the crates you own). After
test or CI runs delete the test executables:
`find target/debug/deps -maxdepth 1 -type f -perm -u+x ! -name '*.so' -delete`.

## Progress docs
`docs/*-parity.md` and `docs/gaps.md` follow
[`docs/standards/progress-docs.md`](docs/standards/progress-docs.md). Update the affected rows,
status line and revision history in the same commit as the work.
