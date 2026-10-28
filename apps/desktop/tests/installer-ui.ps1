# Screenshots of the pages of the installers where the user chooses the public packages, on Windows: the
# NSIS installer's page "Public packages" and the MSI's page of the features (Custom Setup), which has
# the feature "Public packages" in its tree. It starts each installer with its window, clicks Next until
# the page is there, takes a picture of the window, and cancels, so that nothing is installed. For the
# release workflow's rehearsals (workflow_dispatch), on a runner that has no RNL Viewer: the script
# stops when the user's own RNL Viewer is installed, and a developer's machine is not the place to run it.
#
#   pwsh apps/desktop/tests/installer-ui.ps1 -Nsis SETUP -Msi PACKAGE -Out DIR
param(
  [string]$Nsis,
  [string]$Msi,
  [Parameter(Mandatory = $true)][string]$Out
)
$ErrorActionPreference = "Stop"
if (Test-Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\RNL Viewer") {
  throw "RNL Viewer is installed for this user: this script leaves it be, and runs on a machine that has none."
}
New-Item -ItemType Directory -Force $Out | Out-Null
# the paths as Windows writes them: msiexec does not take the forward slashes that a shell may leave in one
if ($Nsis) { $Nsis = (Resolve-Path $Nsis).Path }
if ($Msi) { $Msi = (Resolve-Path $Msi).Path }
Add-Type -ReferencedAssemblies System.Drawing, System.Windows.Forms -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
using System.Text;
public static class InstallerUi {
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [StructLayout(LayoutKind.Sequential)] struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr parent, EnumProc p, IntPtr l);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern IntPtr SendMessageTimeout(IntPtr h, int msg, IntPtr w, StringBuilder l, uint flags, uint timeout, out IntPtr result);
  [DllImport("user32.dll", EntryPoint = "SendMessageTimeoutW")] static extern IntPtr SendMessageTimeoutLong(IntPtr h, int msg, IntPtr w, IntPtr l, uint flags, uint timeout, out IntPtr result);
  [DllImport("user32.dll")] static extern bool PostMessage(IntPtr h, int msg, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] static extern bool IsWindowEnabled(IntPtr h);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern IntPtr SendMessage(IntPtr h, int msg, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr h);

  // WM_GETTEXT with a time limit: a window that is busy, as an installer is while it runs a page's code, must not stop us
  static string Text(IntPtr h) {
    var s = new StringBuilder(1024); IntPtr r;
    return SendMessageTimeout(h, 0x000D, (IntPtr)s.Capacity, s, 2 | 8, 500, out r) == IntPtr.Zero ? "" : s.ToString(); // SMTO_ABORTIFHUNG | SMTO_BLOCK
  }

  /// The visible top-level windows of a process, the newest dialog first as the system lists them.
  public static List<IntPtr> Windows(uint pid) {
    var list = new List<IntPtr>();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == pid && IsWindowVisible(h)) list.Add(h); return true; }, IntPtr.Zero);
    return list;
  }
  /// The texts of a window and of everything in it.
  public static List<string> Texts(IntPtr top) {
    var list = new List<string> { Text(top) };
    EnumChildWindows(top, (h, l) => { var t = Text(h); if (t.Length > 0) list.Add(t); return true; }, IntPtr.Zero);
    return list;
  }
  /// Press the enabled button of a window whose text, without its & and its arrows, is `name`.
  public static bool Press(IntPtr top, string name) {
    bool done = false;
    EnumChildWindows(top, (h, l) => {
      var t = Text(h).Replace("&", "").Replace(">", "").Replace("<", "").Trim();
      if (!done && IsWindowVisible(h) && IsWindowEnabled(h) && string.Equals(t, name, StringComparison.OrdinalIgnoreCase)) {
        PostMessage(h, 0x00F5, IntPtr.Zero, IntPtr.Zero); // BM_CLICK, posted: its page may show a box that waits for an answer
        done = true;
      }
      return !done;
    }, IntPtr.Zero);
    return done;
  }
  /// The whole screen, for when a page does not come.
  public static void Screen(string path) {
    var b = System.Windows.Forms.Screen.PrimaryScreen.Bounds;
    using (var bmp = new Bitmap(b.Width, b.Height))
    using (var g = Graphics.FromImage(bmp)) {
      g.CopyFromScreen(b.Left, b.Top, 0, 0, bmp.Size);
      bmp.Save(path, ImageFormat.Png);
    }
  }
  /// Check the check box whose text, without its &, is `name`, unless it is checked (clicking again would clear it).
  public static void Check(IntPtr top, string name) {
    EnumChildWindows(top, (h, l) => {
      if (IsWindowVisible(h) && string.Equals(Text(h).Replace("&", "").Trim(), name, StringComparison.OrdinalIgnoreCase)) {
        IntPtr state;
        if (SendMessageTimeoutLong(h, 0x00F0, IntPtr.Zero, IntPtr.Zero, 2, 500, out state) != IntPtr.Zero && state == IntPtr.Zero) // BM_GETCHECK
          PostMessage(h, 0x00F5, IntPtr.Zero, IntPtr.Zero);
        return false;
      }
      return true;
    }, IntPtr.Zero);
  }
  public static void Shot(IntPtr top, string path) {
    SetForegroundWindow(top);
    RECT r; GetWindowRect(top, out r);
    using (var bmp = new Bitmap(r.Right - r.Left, r.Bottom - r.Top))
    using (var g = Graphics.FromImage(bmp)) {
      var hdc = g.GetHdc();
      PrintWindow(top, hdc, 2); // PW_RENDERFULLCONTENT
      g.ReleaseHdc(hdc);
      bmp.Save(path, ImageFormat.Png);
    }
  }
}
'@

function Wait-Until($what, [scriptblock]$done, $seconds = 90) {
  $t0 = Get-Date
  while ($true) {
    $v = & $done
    if ($v) { return $v }
    if (((Get-Date) - $t0).TotalSeconds -gt $seconds) { throw "Timed out waiting for $what." }
    Start-Sleep -Milliseconds 300
  }
}

# Click through the installer's pages until one has the text `$page`, shoot it, and cancel the installer.
function Shoot-Page($process, $page, $file, [string[]]$forward) {
  $log = @()
  $shot = $false
  $deadline = (Get-Date).AddMinutes(4)
  while ((Get-Date) -lt $deadline -and -not $process.HasExited) {
    $windows = [InstallerUi]::Windows([uint32]$process.Id)
    foreach ($w in $windows) {
      $texts = [InstallerUi]::Texts($w)
      if ($texts | Where-Object { $_ -like "*$page*" }) {
        Start-Sleep -Milliseconds 700
        [InstallerUi]::Shot($w, $file)
        Write-Host "took $file at the page with '$page'"
        $shot = $true
        break
      }
      $log += ($texts -join " | ")
      # a license page's check box comes before its Next
      foreach ($agree in @("I accept the terms in the License Agreement", "I accept the terms")) {
        $box = $texts | Where-Object { $_.Replace("&", "") -like "*$agree*" }
        if ($box) { [InstallerUi]::Check($w, $box.Replace("&", "")) }
      }
      foreach ($name in $forward) { if ([InstallerUi]::Press($w, $name)) { break } }
    }
    if ($shot) { break }
    # a page takes a moment to come, and a second click on the old one would pass over the page we want
    Start-Sleep -Milliseconds 1500
  }
  if (-not $shot) {
    Write-Host ("pages seen:`n" + ($log | Select-Object -Last 12 | ForEach-Object { "  " + $_ }) -join "`n")
    $log | Set-Content (Join-Path $Out "pages-seen.txt")
    try { [InstallerUi]::Screen((Join-Path $Out "screen-when-the-page-did-not-come.png")) } catch { Write-Host "no screen picture: $_" }
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
    throw "The page with '$page' did not come."
  }
  # cancel: the button, then the question's Yes
  foreach ($w in [InstallerUi]::Windows([uint32]$process.Id)) { [InstallerUi]::Press($w, "Cancel") | Out-Null }
  $end = (Get-Date).AddSeconds(30)
  while (-not $process.HasExited -and (Get-Date) -lt $end) {
    foreach ($w in [InstallerUi]::Windows([uint32]$process.Id)) {
      foreach ($yes in @("Yes", "OK")) { if ([InstallerUi]::Press($w, $yes)) { break } }
    }
    Start-Sleep -Milliseconds 500
  }
  if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
}

if ($Nsis) {
  Write-Host "the NSIS installer's page of the public packages"
  $p = Start-Process -FilePath $Nsis -PassThru
  Shoot-Page $p "Install the public packages" (Join-Path $Out "nsis-public-packages-page.png") @("Next", "I Agree", "Agree")
}
if ($Msi) {
  Write-Host "the MSI's page of the features"
  $p = Start-Process -FilePath "msiexec.exe" -ArgumentList "/i `"$Msi`"" -PassThru
  Shoot-Page $p "Custom Setup" (Join-Path $Out "msi-custom-setup-page.png") @("Next")
}
if (Test-Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\RNL Viewer") { throw "The installer was not cancelled: RNL Viewer is installed." }
Write-Host "nothing was installed"
