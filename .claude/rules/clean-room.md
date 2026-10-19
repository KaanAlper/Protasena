# Clean-room

Protasena copies Proteus's **behaviour**, never Labcenter's **material**.

## May be the same as Proteus
- Control labels, menu item names, dialog field names, mode and tool names, keyboard shortcuts
  (they are the interface people work against).
- Layout and structure of the UI: menus, toolbars, the mode toolbar, which tools sit where,
  dialog layouts, status-bar fields, defaults.
- Device naming conventions people type in Pick Devices (`RES`, `CAP`, `7400`, …).
- The `.pdsprj` structure, worked out only from files Proteus saves from our own synthetic
  designs (black-box analysis of data files; Proteus's program is never inspected).

## Never copied, never committed
- Icons, images, splash screens, colours or artwork taken from Proteus screenshots (we pick our
  own colours and draw icons in code; matching *placement* and *meaning* is fine).
- Proteus's libraries: device symbols, packages, simulation models (VSM and SPICE), and its
  sample designs and templates.
- Help text, tooltips' long descriptions, documentation prose. Short labels may be the same;
  any description is written in our own words only.
- Anything inside the Proteus install folder beyond file **names** (no reading, no disassembly).
- Screenshots of Proteus, UI Automation dumps, notes and files it produced: only under
  `plan/proteus/`, which is gitignored. Never paste them into issues, PRs, commit messages or
  docs.

## Privacy
The reference PC is the owner's own desktop. Never commit its AsenaScale device name (it lives in
`plan/proteus/env.sh`), IPs, Windows user paths (`C:\Users\…`), e-mail addresses, or the licence
holder and customer details Proteus shows (title bar, About box, licence manager): notes never
copy them, and screenshots stay under `plan/proteus/`.

## Code and data we may not use
- GPL code — KiCad's application code, gEDA, Fritzing, and any other GPL/LGPL/AGPL EDA code — not
  linked, not copied, not read.
- KiCad's **file format documentation** (dev-docs.kicad.org) may be read; the `kicad` crate is
  written from it.
- KiCad's **libraries** (CC-BY-SA 4.0 with the design exception) are used only as data: converted
  by `protasena-libconv` and published in `KaanAlper/Protasena-library` with KiCad's licence and
  attribution. They are never committed to this repo and never a Cargo dependency.
- Whether ngspice may be used as a library (simulation, later) is checked file by file when that
  sub-project starts.

## Before a commit
If the diff touches assets (images, icons, fonts), UI strings, `docs/`, test fixtures or
`tools/proteus/`, check it against this file: no Labcenter material, no observation data, no
private details, every asset in `ATTRIBUTION.md`.
