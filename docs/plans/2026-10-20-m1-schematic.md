# M1 — schematic capture core: implementation plan

> For agentic workers: implement task by task on branch `m1-schematic` (one commit or more per
> task, conventional commits). Each task ends with `cargo xtask ci` green (from Task 1 on).
> Spec: `docs/specs/2026-10-20-protasena-design.md`.

**Goal:** a Protasena that loads a KiCad-derived parts library, lets you place and wire parts on
a sheet in Proteus's Schematic Capture layout, edit their properties, save/load projects and
export a netlist.

**Architecture:** Rust workspace with AsenaCAD's layering (L0 `geom`/`kicad`/`pdsprj` → L1
`doc`/`library` → L2 `render`/`netlist` → L3 `io` → L4 `engine` → L5 `ui-egui` → apps).

**Tech stack:** Rust stable (edition 2024), egui/eframe 0.36 (as AsenaCAD), serde, egui_kittest.

## Ground rules for every task
- Read `AGENTS.md` (after Task 1) and follow it. Never-crash: no `unwrap`/`expect`/`panic!`/
  indexing that can panic in non-test code; errors via `Result`; parsers treat input as hostile
  (depth and size limits); every crash fix gets a regression test.
- Clean-room: no Proteus assets, no GPL code (KiCad application code included) read or copied.
  KiCad *file format documentation* (dev-docs.kicad.org) may be read.
- Disk is limited: one shared `CARGO_TARGET_DIR=/home/kaanalper/Mimarlık/Protasena/target`,
  `CARGO_PROFILE_DEV_DEBUG=line-tables-only`, `CARGO_INCREMENTAL=0`; after test runs delete
  test executables (`find target/debug/deps -maxdepth 1 -type f -perm -u+x ! -name '*.so'
  -delete`).
- Tests first (red → green), then refactor. Files ≤ ~400 lines; split by responsibility.

## Task 1 — workspace, rules, CI
Files: `Cargo.toml` (workspace, `license = "MIT OR Apache-2.0"`, lints: `unsafe_code = "forbid"`,
clippy `unwrap_used`/`expect_used`/`panic` = deny outside tests), `LICENSE-MIT`, `LICENSE-APACHE`,
`README.md`, `NOTICE` (Protasena is original; Proteus is a trademark of Labcenter Electronics, no
affiliation; KiCad libraries credit), `ATTRIBUTION.md`, `AGENTS.md` + `CLAUDE.md` (`@AGENTS.md`),
`.claude/rules/{clean-room.md,parity-method.md}`, `docs/standards/never-crash.md`,
`docs/ui-parity.md` (empty table skeleton), `docs/gaps.md`, `tools/proteus/README.md` (the two
helpers already in `tools/proteus/`), crates `geom kicad pdsprj doc library render netlist io
engine ui-egui` (empty libs with crate docs), apps `protasena protasena-cli protasena-libconv`,
`xtask` with `ci` (fmt --check, clippy -D warnings, test, layers, assets), `layers` (each crate's
workspace deps allowed only downward per the table in the spec), `assets` (every non-code file
under `crates/`/`apps/` that is an image/font/data has an `ATTRIBUTION.md` row).
CI: `.github/workflows/ci.yml` on ubuntu-latest + windows-latest + macos-latest running
`cargo xtask ci`; third-party actions pinned to commit SHAs.
Model `AGENTS.md`, rules and xtask on `/home/kaanalper/Mimarlık/AsenaCAD` (read-only reference:
same owner, same licence; adapt AutoCAD → Proteus, `plan/autocad` → `plan/proteus`,
`ASENACAD_REF` → `PROTASENA_REF`). Include the owner's rule: Proteus behaviour is the baseline,
improvements are proposed to the owner first and marked ★ when approved.
Done when: `cargo xtask ci` passes; `cargo xtask layers` fails on a deliberately wrong dep
(test in xtask).

## Task 2 — `geom`
`Pt`/`Vec2` (f64), `Rect`, `Orient` (rotation in 90° steps + mirror X/Y, compose, apply,
inverse), `Transform` (orient + offset), grid snapping, segment–point distance, segment
intersection. Property tests for orient group laws (`proptest`).

## Task 3 — `kicad`: S-expression reader and models
- `sexpr`: tokenizer + parser to a tree (`List`, `Atom`, `Str`), positions for errors, limits
  (depth ≤ 256, input ≤ 64 MiB, atom ≤ 1 MiB), never panics (fuzz-style test on random bytes).
- `symbol`: `.kicad_sym` → `SymbolLib { symbols }`; `Symbol { name, extends, properties (name,
  value, position, effects/hide), pin_numbers_hide, pin_names (offset, hide), in_bom, on_board,
  power, units: Vec<Unit { unit, style (body style / De Morgan), graphics, pins }> }`; graphics:
  arc, circle, rectangle, polyline, bezier, text, text_box with stroke/fill; pins: electrical
  type, graphic style, at (x, y, angle), length, name/number with effects, alternates, hide.
  `extends` resolution (derived symbols inherit the parent's units).
- `footprint`: `.kicad_mod` → `Footprint { name, layer, descr, tags, attr, properties, pads
  (number, type, shape, at, size, drill, layers, roundrect ratio, …), graphics (fp_line, fp_arc,
  fp_circle, fp_rect, fp_poly, fp_text), model refs (path only) }`.
- Unknown tokens are kept as `Unknown(tree)` (lossless), counted by a `report()`.
- Fixtures: small hand-written files in `crates/kicad/tests/data/` (our own, CC0 in
  ATTRIBUTION.md). Opt-in corpus test: `KICAD_SYMBOL_DIR`/`KICAD_FOOTPRINT_DIR` → parse every
  file; zero errors; print counts of unknown tokens.

## Task 4 — `library`: device model and pack format
`Device { name (Proteus-style, e.g. RES), description, category, sub_category, manufacturer,
keywords, symbol (ours: units, pins, graphics in Protasena units), packages: Vec<PackageRef>,
pin_map (symbol pin ↔ pad), sim_model: Option<SimModelRef>, properties (defaults, e.g.
VALUE=10k, PRIMITIVE=ANALOG), source (KiCad lib/symbol + licence tag) }`, `Package { name,
pads, graphics }`. Pack on disk: one directory with `index.json` (versioned) + per-library
JSON files; loader with validation and limits; search by keyword/category (Pick Devices). A
built-in mini set of our own generic parts (RES, CAP, CAP-ELEC, DIODE, LED, NPN, PNP, GROUND,
POWER, generic op-amp) for tests and for running without the pack.

## Task 5 — `protasena-libconv` and `Protasena-library`
App: `protasena-libconv --symbols DIR --footprints DIR --map map.toml --out DIR`. Converts
KiCad symbols → devices (units, pins, graphics; KiCad mm → Protasena units), links footprints
from the symbol's `Footprint`/`ki_fp_filters` fields, assigns category from the KiCad library
name via `map.toml` (our own mapping: e.g. `Device:R` → name RES, category "Resistors",
`74xx:74LS00` → 74LS00, category "TTL 74LS series"), writes the pack and a report (counts,
unmapped, failures — must be zero failures). Pin a KiCad library release (latest stable 9.x tag)
in the README. Create `KaanAlper/Protasena-library` (public, CC-BY-SA 4.0, KiCad's LICENSE and
attribution copied, README explains the conversion and the pinned upstream tag) and push the
generated pack. Do not vendor KiCad files into the Protasena repo.

## Task 6 — `doc` and `netlist`
`Project { sheets, design_properties }`, `Sheet { name, size, components, wires, junctions,
labels, buses, terminals, texts }`, `Component { id, device ref, reference (R1…), value,
transform, properties, unit }`, wires as polylines between endpoints, junctions, labels
(net names), terminals (power/ground/input/output/bidir/default with net names). Edits as
reversible operations (for undo). Auto-annotation (R1, R2…), pin positions in sheet space.
`netlist`: connectivity (pin/wire endpoint/junction/label/terminal unification, power nets by
name across sheets), stable net names, ERC basics (unconnected pins report), export formats:
our JSON and a SPICE-style netlist.

## Task 7 — `io`
Own project format (`.psproj`, JSON with a version field, sorted keys, stable ids), save/load
round trip tests, hostile file tests, netlist export files.

## Task 8 — `engine`
`CommandSpec` registry (id, label, menu path, shortcut, params doc, enabled, JSON run,
interactive prompt/mode machine), `Session` with project, selection, undo/redo stack.
Commands: place component, wire (pin to pin with Proteus-style orthogonal auto routing —
follow-mouse with elbows; manual corners on click), junction, wire label, terminal, bus, text,
rotate (+/-90), mirror X/Y, move, drag, delete, copy/paste, block copy/move/rotate/delete,
undo/redo, edit component properties, annotate, netlist export, save/open/new.

## Task 9 — `ui-egui` and the app
Proteus Schematic Capture layout (from `plan/proteus/notes/ui-inventory.md` when available):
menu bar with Proteus's menus (unbuilt items shown disabled, listed in docs/ui-parity.md), top
toolbars, left mode toolbar with rotation/mirror controls, overview window, object selector
(P opens Pick Devices, L library manager, DEVICES list), editing window (grid dots/lines, sheet
border, zoom F6/F7/F8, wheel zoom, middle-drag pan, G grid toggle, snap), status bar (messages,
sheet name, x/y coordinates). Our own theme (`theme::Tokens`, light and dark), icons drawn in
code. Pick Devices dialog: keywords, category/sub-category/manufacturer lists, results table,
symbol and package previews. Edit Component dialog. Control channel (JSON lines:
`ui.screenshot`, `ui.click`, `cmdline`/`engine.execute`, `ui.elements`).

## Task 10 — `.pdsprj` feasibility and reader
From `plan/proteus/notes/pdsprj-format.md`: container reader in `pdsprj` (L0), mapping into
`doc` in `io`, tests on files we write ourselves in the observed structure. Proteus sample files
never enter the repo.

## Finish
PR `m1-schematic` → main with a summary, parity doc rows, known gaps in `docs/gaps.md`.
