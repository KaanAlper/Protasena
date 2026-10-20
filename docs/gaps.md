# Where Protasena falls short of Proteus

> **Last reviewed:** 2026-10-20 · **Last updated:** 2026-10-20 · **Change:** minor (S2: KiCad converter and published pack landed; S3: project files landed) · **Target:** Labcenter Proteus 8 Professional (Windows)

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
| S5 | No `.pdsprj` reading | Opening Proteus projects | M1 Task 10 |
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
