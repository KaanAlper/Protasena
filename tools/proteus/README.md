# Proteus observation helpers

Small scripts for looking at the reference Labcenter Proteus 8 Professional (Windows) from the
outside, through AsenaScale. They contain no Labcenter material: only our own shell and
PowerShell code that talks to the Windows desktop.

Both need `PROTASENA_REF` (the reference PC's AsenaScale device name) and, optionally, `AS` (an
`asenascale` with the PC-to-PC command `exec`). Both are set in the local, gitignored
`plan/proteus/env.sh`: `. plan/proteus/env.sh; AS=${AS:-asenascale}` before calling a helper.

| Helper | What it does |
|--------|--------------|
| `shot-proteus.sh plan/proteus/screenshots/NAME.png` | One `asenascale exec` runs PowerShell on the PC that finds Proteus's main window (`PDS.EXE`) and has it, then each of Proteus's other visible windows (dialogs, open menus), paint itself into a bitmap of the main window's visible frame (`PrintWindow` with `PW_RENDERFULLCONTENT`, DWM extended frame bounds, DPI-aware), and prints it as base64 PNG; the script decodes it locally and checks it is a PNG. Windows of other programs never end up in the image, even when they are on top of Proteus, and nothing is written to the PC's disk. Only if `PrintWindow` fails does it copy the frame from the screen, and then only while Proteus is the foreground window (otherwise it refuses). Fails if Proteus is not running or minimized. OUT must be a `.png` under `plan/proteus/`. **The only way to screenshot Proteus**: never call `asenascale shot` directly. Read the PNG afterwards; if anything that isn't Proteus's is visible, delete it. |
| `uia-dump.ps1` | Runs on the PC (`"$AS" exec "$PROTASENA_REF" -- "& { $(cat tools/proteus/uia-dump.ps1) } -MaxDepth 6" > plan/proteus/uia/NAME.tsv`) and prints the UI Automation tree of Proteus's main window: control type, name, AutomationId, class and rectangle per element, one tab-separated line each, indented by depth. It never reads HelpText. Depth 12 (default) can take a minute or more; 4–6 is quick. |

## Where the output goes
Everything these helpers produce is an observation of Proteus: screenshots go to
`plan/proteus/screenshots/`, dumps to `plan/proteus/uia/`. `plan/` is gitignored; **never commit
their output** and never paste it into issues, PRs, commit messages or docs. Proteus's title bar
and About box can show the licence holder and file paths: notes never copy them.

## Clean-room
Observe Proteus's behaviour, never copy Labcenter's material (icons, artwork, help text,
libraries, files from the install folder). Build from your own notes, not from screenshot
pixels. Full rules: [`.claude/rules/clean-room.md`](../../.claude/rules/clean-room.md).
