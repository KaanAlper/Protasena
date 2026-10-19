# Parity method

Goal: Protasena behaves like **Labcenter Proteus 8 Professional (Windows)**. Proteus's behaviour
is the baseline; every claim of parity is backed by a side-by-side observation.

## The loop
1. **Observe**: build a small synthetic design for exactly one behaviour in Proteus on the
   reference PC, drive it (keystrokes, menu picks), save screenshots
   (`tools/proteus/shot-proteus.sh`), UI Automation dumps (`tools/proteus/uia-dump.ps1`) and
   notes under `plan/proteus/`.
2. **Build** it in Protasena as commands and UI.
3. **Compare** the same steps in Protasena side by side.
4. **Record** in `docs/*-parity.md`: ✅ matches, ◐ partial (list what differs), ✗ missing,
   ★ intentional improvement approved by the owner.

## Improvements (★)
Where Protasena can do something better or easier than Proteus, propose it to the owner first
(what Proteus does, what we would do, why). Build it only once approved, and mark the row ★ with
a one-line note of the difference and the date of the approval. Without approval, match Proteus.

## What "matches" means
| Area         | Must be equal |
|--------------|---------------|
| Tool / mode  | Name, where it sits, shortcut, what a click/drag/double-click does in it, defaults, the resulting design (coordinates on the same grid), undo granularity |
| UI element   | Position and order relative to its neighbours, label text, control type (button, toggle, drop-down, list), enabled/disabled rules, keyboard access |
| Dialog       | Field set and order, defaults, OK/Cancel behaviour, what is remembered next time |
| Netlist      | The same nets and connections for the same design |
| Not required | Pixel-identical rendering, Labcenter's icons/colours/fonts (forbidden anyway, see clean-room) |

## Observations
- One behaviour per observation, named `<area>-<what>` (e.g. `wire-autoroute-elbow`).
- Use only synthetic designs made for the observation; never open real customer designs.
- Notes describe in our own words what was seen (labels, order, defaults, differences), never
  copy help text and never contain private details (see clean-room).

## plan/ layout (local, gitignored)
```
plan/
  STATUS.md        Milestone:, Next task:, Reference PC: (note; `disposable` allows force-closing
                   Proteus), Proteus exe:   (see AGENTS.md "Start every session here")
  proteus/
    env.sh         export PROTASENA_REF=<device>  [AS=<asenascale>]
                   sourced first: `. plan/proteus/env.sh; AS=${AS:-asenascale}`
    screenshots/   <name>-<n>.png             Proteus, only via tools/proteus/shot-proteus.sh
                   <name>.protasena.png       Protasena, same steps
    uia/           <name>.tsv                 UI Automation dumps (tools/proteus/uia-dump.ps1)
    notes/         <area>.md                  what was observed; ui-inventory.md (every menu item,
                                              toolbar button, mode, dialog, setting, shortcut:
                                              the rows of docs/ui-parity.md); pdsprj-format.md
    samples/       <name>.pdsprj              files Proteus saved from our synthetic designs
    logs/          <name>.log                 logs of observation runs
```
