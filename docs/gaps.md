# Where Protasena falls short of Proteus

> **Last reviewed:** 2026-10-20 · **Last updated:** 2026-10-20 · **Change:** minor (S2: device model and pack loader landed) · **Target:** Labcenter Proteus 8 Professional (Windows)

Every known shortfall, one row each, ranked within each section by user impact. This is the work
list: agents pick from here (top of a section first), and remove or update a row in the same PR
that closes it. "Est." means a judgement, not a measurement.

## Schematic capture (milestone 1)

| # | Gap | Why it ranks here | Plan task |
|---|---|---|---|
| S1 | No application window yet: the apps are placeholders | Nothing can be drawn | M1 Task 9 |
| S2 | No parts library: device model, pack loader and 11 built-in generic parts exist; the KiCad reader, converter and published pack do not | Pick Devices needs devices | M1 Tasks 3, 5 |
| S3 | No design model, netlist or project files | Nothing to edit, save or export | M1 Tasks 6–7 |
| S4 | No commands (placing, wiring, editing, undo) | Every tool is a command | M1 Task 8 |
| S5 | No `.pdsprj` reading | Opening Proteus projects | M1 Task 10 |
| S6 | No observed UI inventory in `docs/ui-parity.md` | Rows come from observation | M1 Task 9 |

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
