# Where Protasena falls short of Proteus

> **Last reviewed:** 2026-10-20 · **Last updated:** 2026-10-20 · **Change:** major (S3/S4: the engine's commands and modes landed; S4 lists what is still to compare) · **Target:** Labcenter Proteus 8 Professional (Windows)

Every known shortfall, one row each, ranked within each section by user impact. This is the work
list: agents pick from here (top of a section first), and remove or update a row in the same PR
that closes it. "Est." means a judgement, not a measurement.

## Schematic capture (milestone 1)

| # | Gap | Why it ranks here | Plan task |
|---|---|---|---|
| S1 | No application window yet: the apps are placeholders | Nothing can be drawn | M1 Task 9 |
| S2 | Parts library not wired into the app: the KiCad pack exists (22 583 devices, 15 415 packages, measured; `KaanAlper/Protasena-library`) `io::load_library` reads `PROTASENA_LIBRARY_DIR`, but no application uses it yet. Converter approximations: pin graphic styles are drawn as body graphics (the device model has no pin style), pin alternate functions are dropped, footprint zones and arcs inside footprint polygons are skipped | Pick Devices needs the pack loaded | M1 Task 9 |
| S3 | Save/Open/Export Netlist exist only as engine commands (`new`, `open`, `save`, `save-as`, `export-netlist`): no window reaches them yet. Proteus's backup and auto-save behaviour not yet observed; New refuses unsaved changes unless told to discard them (the UI asks), to compare with Proteus's prompt | Nothing can be saved from the application | M1 Task 9; observation |
| S4 | The engine (`crates/engine`) has 63 commands (measured) and interactive machines for 8 of the 22 modes (Selection, Component, Junction Dot, Wire Label, Text Script, Buses, Terminals, 2D Graphics Text). **Missing:** Subcircuit, Device Pins, Graph, Active Popup, Generator, Probe, Virtual Instruments and the other 2D graphics modes (they report "not available yet"); Text Script places plain text (no script semantics); Search & Tag, Property Assignment Tool, Align, Send To Back/Bring To Front, Tidy Design, Make Device/Symbol, Packaging Tool, Decompose; bus entries cannot be placed and buses carry no nets; multi-gate parts are not given the next free gate when placed. **Built the conventional way, to compare with Proteus** (its clicks and dialogs were not observable yet): Component mode shows a ghost at the cursor and places on one click (Proteus may pick up and then place), a click on a placed part selects it; real-time annotation on placement and the reference/value text positions; the wire autorouter's choice of path (L, Z through the middle, detours one grid step beyond bodies and connection points, a one-grid stub out of pins), straight (possibly diagonal) segments between clicked corners when the autorouter is off, how a wire ends (click on a pin, wire, junction or terminal; a second click on the same point; double click) and is cancelled (Escape, right button), automatic junction dots at T's; dragging stretches attached wires with an elbow or routes them again, Block Move keeps wires attached too, rotate/mirror of a selection turns each part about its origin with wires following, Block Rotate turns about the snapped centre of the selection, which axis X-Mirror and Y-Mirror flip; Delete keeps the wires of a deleted part, the Delete key itself (no menu item carries it); paste and Block Copy give colliding references the next free number and default to the cursor position; box selection takes only fully enclosed objects, Ctrl/Shift add and toggle; Zoom In/Out by a factor of 2 keeping the centre (Proteus may centre on the cursor), Toggle False Origin at the cursor, a 6-pixel pick tolerance; Global Annotator defaults to incremental; new sheets are named `Root sheet N`, a new project `UNTITLED`. Undo keeps 1000 steps and the design stays marked changed after undoing back to the saved state (Proteus's undo depth and title-bar marker to compare). Every dialog (Pick Devices, Edit Component, Edit Wire Label, Edit Design/Sheet Properties, Global Annotator, file choosers) is a request to the UI; their fields are not observed | Commands are the whole editing surface | M1 Task 9; observation |
| S5 | No `.pdsprj` reading | Opening Proteus projects | M1 Task 10 |
| S6 | No observed UI inventory in `docs/ui-parity.md` | Rows come from observation | M1 Task 9 |
| S7 | Connectivity, naming and annotation rules not yet compared with Proteus: wires crossing or touching at bends, pin ends on wire interiors, label case and scope, terminal scope and unnamed power/ground terminals, hidden power pins, net name priority and `#NET` numbering, annotation order, default sheet names and sizes; the engine adds a junction dot where a new wire ends on another wire's interior or where three wire ends meet, and a wire label must sit on a wire (within 1 th) — to compare | Netlists must match Proteus for the same design | M1 Task 6 follow-up (observation) |

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
| 2026-10-20 | major | S3 narrowed and S4 rewritten: M1 Task 8 landed the engine (commands, session, undo, modes); missing modes and tools and the behaviour still to compare listed; S7 notes the engine's junction and label rules |
