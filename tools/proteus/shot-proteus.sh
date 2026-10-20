#!/usr/bin/env bash
# Screenshot of Proteus's main window only. The image is cropped ON THE REFERENCE PC: one
# `asenascale exec` runs PowerShell that finds Proteus's main window, copies exactly its visible
# frame from the screen into a bitmap and prints it as base64 PNG. No other pixel of the owner's
# screens ever leaves the PC, and nothing is written to the PC's disk.
#
#   tools/proteus/shot-proteus.sh plan/proteus/screenshots/NAME-1.png
#
# Env: PROTASENA_REF  the reference PC's AsenaScale device name (required; see plan/proteus/env.sh)
#      AS            the asenascale binary (default: asenascale on PATH; needs `exec`)
# The output is a Proteus screenshot: it must be under plan/proteus/ (gitignored, enforced
# below); never commit it. Read the PNG afterwards: if anything that is not Proteus's is visible,
# delete it.
set -euo pipefail
OUT=${1:?usage: shot-proteus.sh plan/proteus/screenshots/NAME.png}
AS=${AS:-asenascale}
DEV=${PROTASENA_REF:?set PROTASENA_REF to the reference PC\'s AsenaScale device name (plan/proteus/env.sh)}

# OUT must resolve to a .png under <repo>/plan/proteus/.
ROOT=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
ABS=$(realpath -m -- "$OUT")
case "$ABS" in
  "$ROOT"/plan/proteus/*.png) ;;
  *) echo "shot-proteus.sh: OUT must be a .png under plan/proteus/ (got $OUT)" >&2; exit 2 ;;
esac
mkdir -p -- "$(dirname -- "$ABS")"

# Prints "ok WxH+X+Y" on the first line, then the PNG as base64. Errors go to stderr, exit 3.
read -r -d '' PS <<'POWERSHELL' || true
$ErrorActionPreference = 'Stop'
function Fail($m) { [Console]::Error.WriteLine("shot-proteus: $m"); exit 3 }
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class PdsShot {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  // Proteus's visible top-level windows (main window, floating command line, menus, tooltips),
  // top of the z-order first.
  public static System.Collections.Generic.List<IntPtr> Windows(uint pid) {
    var list = new System.Collections.Generic.List<IntPtr>();
    EnumWindows((h, l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid && IsWindowVisible(h) && !IsIconic(h)) list.Add(h);
      return true;
    }, IntPtr.Zero);
    return list;
  }
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out RECT r, int size);
}
"@
[PdsShot]::SetProcessDPIAware() | Out-Null
$p = Get-Process PDS -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { Fail 'no Proteus main window (PDS.EXE not running in the desktop session?)' }
$h = $p.MainWindowHandle
if ([PdsShot]::IsIconic($h)) { Fail 'Proteus is minimized; restore it first' }
if (-not [PdsShot]::IsWindowVisible($h)) { Fail 'Proteus main window is not visible' }
$wr = New-Object PdsShot+RECT
if (-not [PdsShot]::GetWindowRect($h, [ref]$wr)) { Fail 'cannot get the window rectangle' }
$r = New-Object PdsShot+RECT
# DWMWA_EXTENDED_FRAME_BOUNDS = 9: the visible frame, without the invisible resize borders.
if ([PdsShot]::DwmGetWindowAttribute($h, 9, [ref]$r, 16) -ne 0) { $r = $wr }
$w = $r.R - $r.L; $hh = $r.B - $r.T
if ($w -le 0 -or $hh -le 0) { Fail "empty window rectangle ${w}x${hh}" }
$fg = [PdsShot]::GetForegroundWindow(); $fgPid = [uint32]0
[PdsShot]::GetWindowThreadProcessId($fg, [ref]$fgPid) | Out-Null
# PW_RENDERFULLCONTENT = 2: a window renders itself (DirectX content included), so nothing that
# is not Proteus's ends up in the bitmap. Paint Proteus's own windows bottom to top, clipped to
# the main window's frame.
function Paint($win, $g) {
  $q = New-Object PdsShot+RECT
  if (-not [PdsShot]::GetWindowRect($win, [ref]$q)) { return $false }
  $qw = $q.R - $q.L; $qh = $q.B - $q.T
  if ($qw -le 0 -or $qh -le 0 -or $qw -gt 16384 -or $qh -gt 16384) { return $false }
  $one = New-Object System.Drawing.Bitmap $qw, $qh
  $og = [System.Drawing.Graphics]::FromImage($one)
  $hdc = $og.GetHdc()
  $ok = [PdsShot]::PrintWindow($win, $hdc, 2)
  $og.ReleaseHdc($hdc); $og.Dispose()
  if ($ok) { $g.DrawImageUnscaled($one, $q.L - $r.L, $q.T - $r.T) }
  $one.Dispose()
  return $ok
}
$bmp = New-Object System.Drawing.Bitmap $w, $hh
$g = [System.Drawing.Graphics]::FromImage($bmp)
try {
  if (Paint $h $g) {
    $others = [PdsShot]::Windows([uint32]$p.Id)
    $others.Reverse()
    foreach ($o in $others) { if ($o -ne $h) { Paint $o $g | Out-Null } }
  } elseif ($fgPid -eq $p.Id) {
    $g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $hh))
  } else {
    Fail 'PrintWindow failed and Proteus is not the foreground window; refusing to copy the screen'
  }
  $ms = New-Object System.IO.MemoryStream
  $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
  [Console]::Out.WriteLine("ok ${w}x${hh}+$($r.L)+$($r.T)")
  [Console]::Out.Write([Convert]::ToBase64String($ms.ToArray()))
  [Console]::Out.WriteLine()
} finally { $g.Dispose(); $bmp.Dispose() }
POWERSHELL

TMP=$(mktemp)
trap 'rm -f -- "$TMP"' EXIT
if ! "$AS" exec "$DEV" -- "$PS" > "$TMP"; then
  echo "shot-proteus.sh: capture on $DEV failed (see the message above)" >&2
  exit 1
fi
HEADER=$(head -n 1 "$TMP" | tr -d '\r')
case "$HEADER" in
  "ok "*) ;;
  *) echo "shot-proteus.sh: unexpected reply from $DEV: ${HEADER:0:200}" >&2; exit 1 ;;
esac
tail -n +2 "$TMP" | tr -d '\r\n ' | base64 -d > "$ABS" 2>/dev/null || true
if [ "$(head -c 8 "$ABS" | od -An -tx1 | tr -d ' \n')" != "89504e470d0a1a0a" ]; then
  rm -f -- "$ABS"
  echo "shot-proteus.sh: the reply did not decode to a PNG; nothing saved" >&2
  exit 1
fi
echo "$OUT (Proteus window ${HEADER#ok })"
