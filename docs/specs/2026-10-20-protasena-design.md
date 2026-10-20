# Protasena design

> **Status:** approved by the owner (2026-10-20) · **Target:** Labcenter Proteus 8 Professional (Windows)

## Goal
Protasena is an open-source, clean-room, pure-Rust electronics design suite whose tools, settings
and workflow match **Proteus 8 Professional**: schematic capture (ISIS), circuit simulation, and
PCB layout (ARES). The end goal is every Proteus tool and setting. Proteus's behaviour is the
baseline; where Protasena can do something better or easier, the change is proposed to the owner
first and, once approved, recorded as an intentional difference. The look (colours, icons,
fonts) is our own and modern; layout, order and behaviour come from Proteus.

Runs on Linux, Windows and macOS. Code: MIT OR Apache-2.0. Repository: `KaanAlper/Protasena`
(public).

## Sub-projects (each with its own plan and PRs)
1. **Schematic capture** (this spec's first milestone): library, sheets, placing and wiring,
   properties, netlist, project files.
2. **Simulation**: SPICE (analog and mixed) through the netlist; probes, graphs, generators and
   virtual instruments; later microcontroller simulation (Proteus VSM's models are closed: ours
   are written from datasheets).
3. **PCB layout**: footprints, placement, routing, design rules, Gerber/drill output.

## Clean-room
Proteus runs on the owner's Windows PC and is only **observed** from outside: screenshots of its
own window (`tools/proteus/shot-proteus.sh`), UI Automation dumps (`tools/proteus/uia-dump.ps1`),
behaviour on synthetic designs. Everything observed lives in `plan/proteus/` (gitignored, never
committed).
- Never read, disassemble or copy anything inside the Proteus installation (file names only).
  Never copy its icons, colours, artwork, help text, libraries (symbols, packages, models) or
  sample designs.
- Control labels, menu item names, dialog field names and keyboard shortcuts may be the same
  (they are the interface). Descriptions and tooltips' long text are written in our own words.
- Privacy: never commit the PC's device name, IPs, Windows user paths, e-mail addresses, licence
  holder or customer details Proteus shows.
- Never copy or link GPL code (KiCad's application code, gEDA, Fritzing, …); don't read their
  sources. Whether ngspice (mostly BSD-licensed, with parts under other licences) can be used as
  a library in sub-project 2 is checked file by file when that sub-project starts.

## Parts library
Built from **KiCad's official libraries** (symbols and footprints; CC-BY-SA 4.0 with KiCad's
design exception), never Proteus's library.
- The `kicad` crate reads `.kicad_sym` and `.kicad_mod`, written from KiCad's published file
  format documentation (not from KiCad's code).
- `protasena-libconv` (in this repo, MIT) converts the KiCad libraries into Protasena's own
  library format once, adding what Proteus's device model needs: device = symbol + package +
  simulation model link, category and sub-category, Proteus-style names and keywords (`RES`,
  `CAP`, `7400`, …).
- The converted pack lives in a separate repository, `KaanAlper/Protasena-library`, under
  CC-BY-SA 4.0 with KiCad's attribution and licence files. It is an optional build/runtime input
  (`PROTASENA_LIBRARY_DIR`), never a Cargo dependency; code and tests work without it (a small
  built-in set of our own generic parts covers tests).
- Users' own KiCad libraries are imported at run time through the same conversion code as
  `protasena-libconv` (the `libimport` crate) and the same `kicad` crate. Eagle
  `.lbr` import comes later.
- **No-loss check:** a corpus test converts every symbol and footprint of a pinned KiCad library
  release and fails on any parse error or dropped element.

## Project files
- Protasena's own project format (versioned, text-based, diff-friendly).
- **`.pdsprj`** (Proteus 8 project) read and later write. There is no public specification: the
  format is worked out only from files Proteus saves from our own synthetic designs (black-box
  analysis of data files; Proteus's program is never inspected). Sample files stay in
  `plan/proteus/samples/`; tests use files we write ourselves. Writing counts as done only when
  Proteus on the reference PC opens what we wrote and a round trip
  (Protasena → Proteus → Protasena) keeps every property.

## Architecture
Same layering discipline as AsenaCAD (checked by `cargo xtask layers`):

| Layer | Crates | Role |
|---|---|---|
| L0 | `geom`, `kicad`, `pdsprj` | 2D geometry; KiCad file reader; Proteus project container reader/writer (standalone, no workspace deps) |
| L1 | `doc`, `library` | Design model (projects, sheets, components, pins, wires, junctions, labels, buses, terminals, properties); device model and library loader |
| L2 | `render`, `netlist`, `libimport` | Drawing; connectivity and netlists; KiCad library conversion (shared by `protasena-libconv` and the run-time import of the user's own KiCad libraries) |
| L3 | `io` | Project files (own format, `.pdsprj`), netlist export |
| L4 | `engine` | Everything is a command; undo; Proteus's tools and modes |
| L5 | `ui-egui` | Proteus's window layout with our theme; control channel |
| apps | `protasena`, `protasena-cli`, `protasena-libconv`, `xtask` | |

Rules carried over from AsenaCAD: everything is a command (`CommandSpec`), programmatic calls never
open dialogs, the UI is thin, colours from theme tokens, i18n from the start, never crash (no
panics in non-test code, errors through `Result`, hostile input), a JSON-lines control channel
for driving and screenshotting the app.

## Parity loop
1. **Observe** Proteus (script/keystrokes on the open design → screenshots, UIA dumps, notes).
2. **Build** it as commands and UI.
3. **Compare** side by side.
4. **Record** in `docs/*-parity.md` (✅ matches, ◐ partial with differences listed, ✗ missing,
   ★ intentional improvement approved by the owner).

The inventory of every menu item, toolbar button, mode, dialog, setting and shortcut
(`plan/proteus/notes/ui-inventory.md`) becomes the rows of `docs/ui-parity.md`.

## Milestone 1: schematic capture core
- Workspace, rules (`AGENTS.md`, clean-room, parity method), CI (fmt, clippy, tests, layers,
  assets).
- `kicad` reader + `protasena-libconv` + corpus test; `Protasena-library` repo.
- Design model, netlist, own project format.
- Engine: Selection, Component (Pick Devices, place), wire (pin to pin, auto routing), junction
  dot, wire label, terminals (power, ground, input, output, bidirectional, default), bus, text
  script; rotate/mirror, move, drag, delete, copy, block operations, undo/redo; zoom, pan, grid,
  snap; Edit Component properties; netlist export.
- UI: Proteus's Schematic Capture layout: menu bar, top toolbars, mode toolbar on the left,
  overview window, object selector (P/L, DEVICES), editing window, status bar with coordinates.
- `.pdsprj`: feasibility from samples, then read.

Out of milestone 1: simulation, PCB, `.pdsprj` write, Eagle import, 3D models.

## Testing
Unit and integration tests per crate; engine commands tested through JSON and interactive forms;
hostile inputs never panic; UI tests with egui_kittest; library corpus test (opt-in when the
KiCad library is present); `.pdsprj` tests on our own files.
