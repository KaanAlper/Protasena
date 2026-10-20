# Where Protasena falls short of Proteus

> **Last reviewed:** 2026-10-20 · **Last updated:** 2026-10-20 · **Change:** minor (S5: `.pdsprj` reader and import landed, remaining records and sample plan listed) · **Target:** Labcenter Proteus 8 Professional (Windows)

Every known shortfall, one row each, ranked within each section by user impact. This is the work
list: agents pick from here (top of a section first), and remove or update a row in the same PR
that closes it. "Est." means a judgement, not a measurement.

## Schematic capture (milestone 1)

| # | Gap | Why it ranks here | Plan task |
|---|---|---|---|
| S1 | No application window yet: the apps are placeholders | Nothing can be drawn | M1 Task 9 |
| S2 | Parts library not wired into the app: the KiCad pack exists (22 583 devices, 15 415 packages, measured; `KaanAlper/Protasena-library`) `io::load_library` reads `PROTASENA_LIBRARY_DIR`, but no application uses it yet. Converter approximations: pin graphic styles are drawn as body graphics (the device model has no pin style), pin alternate functions are dropped, footprint zones and arcs inside footprint polygons are skipped | Pick Devices needs the pack loaded | M1 Task 9 |
| S3 | No Save/Open/Export Netlist commands: the design model, the netlist and our own project files (`.psproj`: atomic save with a `.bak` backup, checked loading, netlist export files) exist in the crates, but no command reaches them yet; Proteus's backup and auto-save behaviour not yet observed | Nothing can be saved from the application | M1 Task 8 |
| S4 | No commands (placing, wiring, editing, undo) | Every tool is a command | M1 Task 8 |
| S5 | `.pdsprj` reading is partial (`docs/pdsprj.md`). **Reads:** the container, the manifest, the `ROOT.DSN` header (dates, release, times), style names, the embedded library (symbol names; device definition texts by section), circuits with wires, buses, nodes (junctions), components (reference, value, device, origin, label positions, properties, pin links), terminals (symbol, label), generators and probes (read, not imported). **Does not read:** component and terminal orientation (imported unrotated), the y-axis direction (taken as stored, unconfirmed), the header's design-property texts, wire labels, bus entries, sub-circuits and further sheets, 2D graphics and free text, junction dots versus plain nodes (inferred: three or more links), simulation graphs and the simulation settings trailer, `ROOT.CDB`, script members, symbol graphics. Embedded device definitions are reported, not converted; projects that use the owner's own Proteus libraries need their own handling later. No `.pdsprj` writing (L3). **Sample plan** (Proteus saves of our own synthetic designs, once it runs non-elevated so it can be driven; one change per save, diffed against the previous): empty sheet; one resistor; the same resistor rotated 90°, 180°, 270° and mirrored X/Y; two resistors and one wire; a changed value; a wire label; each terminal kind (default, input, output, bidirectional, power, ground, bus); a junction dot; a bus with an entry; 2D line, box and text; a second sheet; design properties filled in | Opening Proteus projects; orientation is the largest visible error | M1 Task 10 follow-up (observation) |
| S6 | No observed UI inventory in `docs/ui-parity.md` | Rows come from observation | M1 Task 9 |
| S7 | Connectivity, naming and annotation rules not yet compared with Proteus: wires crossing or touching at bends, pin ends on wire interiors, label case and scope, terminal scope and unnamed power/ground terminals, hidden power pins, net name priority and `#NET` numbering, annotation order, default sheet names and sizes | Netlists must match Proteus for the same design | M1 Task 6 follow-up (observation) |

## Later sub-projects

| # | Gap | Notes |
|---|---|---|
| L1 | No simulation (SPICE, probes, graphs, instruments, microcontrollers) | Sub-project 2 |
| L2 | No PCB layout (footprints, routing, design rules, Gerber/drill) | Sub-project 3 |
| L3 | No `.pdsprj` writing, Eagle `.lbr` import, 3D models | After milestone 1 |

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-20 | major | First gaps list: milestone 1 tasks and later sub-projects |
| 2026-10-20 | minor | S2 narrowed: device model, pack format and built-in parts (M1 Task 4) |
| 2026-10-20 | minor | S3 narrowed to project files (M1 Task 6 landed the design model and netlist); S7 lists the rules still to observe |
| 2026-10-20 | minor | S3 narrowed to the commands: M1 Task 7 landed `.psproj` save/load and netlist export files |
| 2026-10-20 | minor | S2 narrowed: `protasena-libconv` and the published KiCad pack (M1 Task 5); approximations listed |
| 2026-10-20 | minor | S5 narrowed: `.pdsprj` reader and import (M1 Task 10); what is not decoded and the sample plan listed |
