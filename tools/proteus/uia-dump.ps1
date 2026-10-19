param([int]$MaxDepth = 12)
# UI Automation tree of Proteus's main window, one element per line, indented by depth:
#   ControlType<TAB>Name<TAB>AutomationId<TAB>ClassName<TAB>x,y,w,h
# Tabs and line breaks inside Name/AutomationId/ClassName are replaced by spaces, so every element
# is exactly one line with five tab-separated fields.
# Reads only structure (type, name, id, class, rectangle); never HelpText or other help prose.
# Speed: every element is a cross-process UI Automation call. Depth 12 (the default) walks the
# whole window and can take a minute or more; -MaxDepth 4..6 is enough for the top-level layout.
# Run on the reference PC through AsenaScale (see tools/proteus/README.md), e.g.
#   "$AS" exec "$PROTASENA_REF" -- "& { $(cat tools/proteus/uia-dump.ps1) } -MaxDepth 6"
# The output is a Proteus observation: keep it under plan/proteus/uia/, never commit it.
[Console]::OutputEncoding = [Text.Encoding]::UTF8
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$p = Get-Process PDS -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) {
  [Console]::Error.WriteLine('uia-dump: no Proteus main window (PDS.EXE not running in the desktop session?)')
  exit 3
}
$root = [System.Windows.Automation.AutomationElement]::FromHandle($p.MainWindowHandle)
$walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
function Clean([string]$s) { if ($null -eq $s) { '' } else { $s -replace '[\t\r\n]+', ' ' } }
function Dump($el, [int]$depth, [int]$max) {
  if ($depth -gt $max) { return }
  try {
    $c = $el.Current; $r = $c.BoundingRectangle
    $t = $c.ControlType.ProgrammaticName -replace '^ControlType\.',''
    $rect = if ($r.IsEmpty) { '-' } else { '{0},{1},{2},{3}' -f [int]$r.X,[int]$r.Y,[int]$r.Width,[int]$r.Height }
    "{0}{1}`t{2}`t{3}`t{4}`t{5}" -f ('  ' * $depth), $t, (Clean $c.Name), (Clean $c.AutomationId), (Clean $c.ClassName), $rect
  } catch { "{0}<error>" -f ('  ' * $depth); return }
  $ch = $walker.GetFirstChild($el)
  while ($ch -ne $null) { Dump $ch ($depth + 1) $max; $ch = $walker.GetNextSibling($ch) }
}
Dump $root 0 $MaxDepth
