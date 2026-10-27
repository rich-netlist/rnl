// A test of the desktop viewer's installers, on Windows: install one silently, check what it
// installed, smoke-test the installed app with smoke.mjs, uninstall it silently, and check that
// nothing of it is left: its files, its shortcuts, its entry among the installed apps and, for the
// NSIS installer, its own registry key. Node 22 or later, with no dependencies.
//
//   node apps/desktop/tests/installers.mjs --nsis SETUP [--dir DIR] [--no-smoke] [--packages with|without]
//   node apps/desktop/tests/installers.mjs --msi PACKAGE [--no-smoke] [--packages with|without]
//
// The NSIS installer installs for the current user, into DIR or else a new temporary directory, and
// needs no administrator rights; the test stops when RNL Viewer is installed for the user already,
// so as to leave that installation be. The MSI installs for the machine, into Program Files, and
// needs administrator rights, as GitHub's runners have; the test stops when RNL Viewer is installed
// there already. --no-smoke leaves out the smoke test. --packages says whether the installation
// is made with the public packages, which an installer that carries them offers, as a part of its
// own (the NSIS installer's /PACKAGES=0, the MSI's PUBLICPACKAGES=0): the test checks that the
// packages are there or not, that the smoke test finds them or not, and that the uninstaller takes
// them away.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "../../..");
const config = JSON.parse(fs.readFileSync(path.join(root, "apps", "desktop", "tauri.conf.json"), "utf8"));
const EXTENSIONS = ["sp", "spi", "spice", "cir", "ckt", "net", "netlist", "inc", "lib", "scs", "rnl"];
const PROGID = "RNLViewer.Netlist";
const CAPABILITIES = "Software\\Rich Netlist\\RNL Viewer\\Capabilities";
const NAME = config.productName;
const VERSION = config.version;
const PUBLISHER = config.bundle.publisher;
/** The app's executable, named after the crate. */
const EXE = "rnl-viewer.exe";
/** The NSIS installer's entry among the apps installed for the current user. */
const USER_ENTRY = `HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${NAME}`;
/** Where the installers keep the install location and what they made, for the current user. */
const OWN_KEY = `HKCU\\Software\\${PUBLISHER}\\${NAME}`;
const TIMEOUT = 120_000;

if (process.platform !== "win32") {
  console.log("The installers' test runs on Windows only.");
  process.exit(0);
}

/** The value of an option such as --dir, or undefined. */
function option(name) {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

function check(ok, message) {
  if (!ok) throw new Error(message);
}

function same(got, want, what) {
  check(JSON.stringify(got) === JSON.stringify(want), `${what}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
}

/** Run a PowerShell script and return what it prints, in UTF-8 whatever the console's code page. */
function powershell(script) {
  const full = `$ProgressPreference = "SilentlyContinue"\n[Console]::OutputEncoding = [Text.Encoding]::UTF8\n${script}`;
  const encoded = Buffer.from(full, "utf16le").toString("base64");
  const r = spawnSync("powershell", ["-NoProfile", "-NonInteractive", "-EncodedCommand", encoded], { encoding: "utf8" });
  if (r.status !== 0) throw new Error(`PowerShell failed: ${r.stderr.trim()}`);
  return r.stdout.trim();
}

/** A PowerShell string literal. */
const quote = (text) => `'${text.replace(/'/g, "''")}'`;

/**
 * Run a program and wait for it and the processes it starts, as an NSIS uninstaller starts a copy of
 * itself; returns its exit code. The arguments are passed as they are written, since an NSIS
 * installer takes /D= last and unquoted, spaces and all.
 */
function run(file, args) {
  return Number(powershell(`(Start-Process -FilePath ${quote(file)} -ArgumentList ${quote(args)} -WindowStyle Hidden -Wait -PassThru).ExitCode`));
}

/** The values of a registry key by name, or null when there is no such key. */
function registry(key) {
  const r = spawnSync("reg", ["query", key], { encoding: "utf8" });
  if (r.status !== 0) return null;
  const values = {};
  for (const line of r.stdout.split(/\r?\n/)) {
    const m = /^ {4}(.+?) {4}REG_\w+ {4}(.*)$/.exec(line);
    if (m) values[m[1]] = m[2];
  }
  return values;
}

/** Existing defaults must survive installation, updates and removal. */
function associationDefaults(hive) {
  return EXTENSIONS.map((ext) => [ext,
    registry(`${hive}\\Software\\Classes\\.${ext}`) || {},
    registry(`HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts\\.${ext}\\UserChoice`),
  ]);
}

function checkAssociations(hive, exe, before) {
  same(registry(`${hive}\\Software\\RegisteredApplications`)?.[NAME], CAPABILITIES, "the Default Apps registration");
  same(registry(`${hive}\\${CAPABILITIES}`)?.ApplicationName, NAME, "the application name");
  const types = registry(`${hive}\\${CAPABILITIES}\\FileAssociations`);
  for (const ext of EXTENSIONS) {
    same(types?.[`.${ext}`], PROGID, `the .${ext} default-app capability`);
    check(Object.hasOwn(registry(`${hive}\\Software\\Classes\\.${ext}\\OpenWithProgids`) || {}, PROGID), `.${ext} does not offer RNL Viewer in Open With`);
  }
  same(Object.values(registry(`${hive}\\Software\\Classes\\${PROGID}\\shell\\open\\command`) || {}),
    [`"${exe}" "%1"`], "the quoted file-open command");
  same(associationDefaults(hive), before, "the user's previous defaults after installation");
}

function checkAssociationsRemoved(hive, before) {
  check(!Object.hasOwn(registry(`${hive}\\Software\\RegisteredApplications`) || {}, NAME), "Default Apps registration was left behind");
  same(registry(`${hive}\\${CAPABILITIES}`), null, "the capabilities left behind");
  same(registry(`${hive}\\Software\\Classes\\${PROGID}`), null, "the ProgID left behind");
  for (const ext of EXTENSIONS) {
    check(!Object.hasOwn(registry(`${hive}\\Software\\Classes\\.${ext}\\OpenWithProgids`) || {}, PROGID), `.${ext} retained the Open With entry`);
  }
  same(associationDefaults(hive), before, "the user's previous defaults after uninstallation");
}

/** Ask Explorer's association machinery to use our ProgID, without changing UserChoice. */
function openAssociatedFile(exe) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "rnl-association-"));
  const file = path.join(dir, "中文 网表.NETLIST");
  fs.writeFileSync(file, "* file association path quoting\nR1 a 0 1k\n.end\n");
  try {
    powershell(`Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class RnlAssociation {
  [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)]
  public struct Info {
    public uint cbSize, fMask;
    public IntPtr hwnd;
    public string lpVerb, lpFile, lpParameters, lpDirectory;
    public int nShow;
    public IntPtr hInstApp, lpIDList;
    public string lpClass;
    public IntPtr hkeyClass;
    public uint dwHotKey;
    public IntPtr hIcon, hProcess;
  }
  [DllImport("shell32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
  public static extern bool ShellExecuteEx(ref Info info);
  [DllImport("kernel32.dll")] public static extern uint GetProcessId(IntPtr process);
  [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr handle);
  public static int Open(string file) {
    var info = new Info { cbSize=(uint)Marshal.SizeOf(typeof(Info)), fMask=0x541,
      lpVerb="open", lpFile=file, lpClass="RNLViewer.Netlist", nShow=1 };
    if (!ShellExecuteEx(ref info)) throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
    var pid=GetProcessId(info.hProcess);
    CloseHandle(info.hProcess);
    return (int)pid;
  }
}
'@
$env:WEBVIEW2_USER_DATA_FOLDER = ${quote(path.join(dir, "profile"))}
$env:RNL_VIEWER_TEST_CONFIG_DIR = ${quote(path.join(dir, "settings"))}
$viewerProcess = Get-Process -Id ([RnlAssociation]::Open(${quote(file)}))
try {
  $deadline = [DateTime]::UtcNow.AddSeconds(30)
  do {
    $viewerProcess.Refresh()
    if ($viewerProcess.HasExited) { throw 'The associated Viewer exited before opening the file.' }
    if ($viewerProcess.MainWindowTitle -eq ${quote(`${path.basename(file)} - RNL Viewer`)}) { break }
    Start-Sleep -Milliseconds 100
  } while ([DateTime]::UtcNow -lt $deadline)
  if ($viewerProcess.MainWindowTitle -ne ${quote(`${path.basename(file)} - RNL Viewer`)}) { throw 'The associated Viewer did not open the Unicode filename.' }
  if ($viewerProcess.Path -ne ${quote(exe)}) { throw 'The association started a different executable.' }
} finally {
  [void]$viewerProcess.CloseMainWindow()
  if (!$viewerProcess.WaitForExit(5000)) { $viewerProcess.Kill(); $viewerProcess.WaitForExit() }
  $testProfile = ${quote(path.join(dir, "profile"))}
  Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" |
    Where-Object { $_.CommandLine -and $_.CommandLine.IndexOf($testProfile, [StringComparison]::OrdinalIgnoreCase) -ge 0 } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
}`);
    console.log("the Windows file association opened a Unicode/space .NETLIST path with the installed Viewer");
  } finally {
    try { fs.rmSync(dir, { recursive: true, force: true, maxRetries: 20, retryDelay: 100 }); }
    catch (error) { console.warn(`The temporary association-test profile remains at ${dir}: ${error.message}`); }
  }
}

/** The app's entries among the apps installed for the machine: their names, versions and publishers. */
function machineEntries() {
  return JSON.parse(
    powershell(`$entries = @(Get-ItemProperty 'HKLM:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*' -ErrorAction SilentlyContinue |
  Where-Object { $_.DisplayName -eq ${quote(NAME)} } |
  ForEach-Object { [pscustomobject]@{ name = $_.DisplayName; version = $_.DisplayVersion; publisher = $_.Publisher } })
ConvertTo-Json -InputObject $entries -Compress`),
  );
}

/** The folders of the shell that the installers put shortcuts in, and Program Files. */
function folders() {
  const names = ["Programs", "Desktop", "CommonPrograms", "CommonDesktopDirectory", "ProgramFiles"];
  const paths = powershell(names.map((n) => `[Environment]::GetFolderPath(${quote(n)})`).join("\n")).split(/\r?\n/);
  return Object.fromEntries(names.map((n, k) => [n, paths[k]]));
}

/** Wait until `done()` holds; throw after TIMEOUT. */
async function until(what, done) {
  const t0 = Date.now();
  while (!done()) {
    if (Date.now() - t0 > TIMEOUT) throw new Error(`Timed out waiting for ${what}.`);
    await new Promise((r) => setTimeout(r, 250));
  }
}

/** The paths among these that are there. */
const present = (paths) => paths.filter((p) => fs.existsSync(p));

/** Whether the installation is to have the public packages, "with" or "without"; unchecked when not given. */
const packages = (() => {
  const value = option("--packages");
  if (value !== undefined && value !== "with" && value !== "without") throw new Error("--packages takes with or without.");
  return value;
})();

/** The installed public packages are there, or not, as asked. */
function checkPackages(dir) {
  if (!packages) return;
  const folder = path.join(dir, "public-packages");
  if (packages === "without") return same(fs.existsSync(folder), false, "the public packages in an installation made without them");
  for (const file of ["public-packages.json", "eda-style/0.1/eda-style.rnl", "LICENSES/MIT.txt"]) {
    check(fs.existsSync(path.join(folder, file)), `${file} is not among the installed public packages.`);
  }
}

/** Smoke-test the installed app, unless --no-smoke. */
function tryApp(exe) {
  if (process.argv.includes("--no-smoke")) return;
  const r = spawnSync(process.execPath, [path.join(here, "smoke.mjs"), "--exe", exe, ...(packages ? ["--packages", packages] : [])], { cwd: root, stdio: "inherit" });
  check(r.status === 0, "The smoke test of the installed app failed.");
}

/**
 * Check an installation with `inspect`, then uninstall it with `uninstall` in any case; the error of
 * the checks, if any, is the one thrown.
 */
async function installed(inspect, uninstall) {
  let failed = null;
  try {
    inspect();
  } catch (e) {
    failed = e;
  }
  console.log("uninstalling");
  try {
    await uninstall();
  } catch (e) {
    failed ??= e;
  }
  if (failed) throw failed;
}

/** The NSIS installer: for the current user, into a directory of the test's own. */
async function nsis(setup) {
  const shell = folders();
  check(registry(USER_ENTRY) === null, `${NAME} is installed for this user already: the test leaves it be.`);
  // the temporary directory's path in its long form: the uninstaller removes a shortcut only when its
  // target, which Windows keeps long, is written as the installation directory was given, and TEMP
  // may give it short, with RUNNER~1 for the user's directory on GitHub's runners
  const temp = fs.realpathSync.native(os.tmpdir());
  const base = option("--dir") ? null : fs.mkdtempSync(path.join(temp, "rnl-viewer-nsis-"));
  const dir = path.resolve(option("--dir") || path.join(base, NAME));
  check(!fs.existsSync(dir) || fs.readdirSync(dir).length === 0, `${dir} is not empty.`);
  const shortcuts = [path.join(shell.Programs, `${NAME}.lnk`), path.join(shell.Desktop, `${NAME}.lnk`)];
  same(present(shortcuts), [], "the shortcuts there before the test");
  const own = registry(OWN_KEY);
  check(registry(`HKCU\\Software\\Classes\\${PROGID}`) === null, "RNL Viewer file associations already exist; leave them intact.");
  const defaults = associationDefaults("HKCU");

  console.log(`installing ${setup} into ${dir}`);
  // /D= goes last; /PACKAGES=0 leaves the public packages out, and without it they are installed
  same(run(setup, `/S ${packages === "without" ? "/PACKAGES=0 " : ""}/D=${dir}`), 0, "the installer's exit code");
  await installed(
    () => {
      console.log(`installed: ${fs.readdirSync(dir).join(", ")}`);
      for (const file of [EXE, "uninstall.exe", "LICENSE.txt"]) check(fs.existsSync(path.join(dir, file)), `${file} was not installed.`);
      checkPackages(dir);
      const entry = registry(USER_ENTRY);
      check(entry !== null, `There is no ${USER_ENTRY}.`);
      same([entry.DisplayName, entry.DisplayVersion, entry.Publisher], [NAME, VERSION, PUBLISHER], "the entry among the installed apps");
      same(present(shortcuts), shortcuts, "the shortcuts in the Start menu and on the desktop");
      checkAssociations("HKCU", path.join(dir, EXE), defaults);
      openAssociatedFile(path.join(dir, EXE));
      tryApp(path.join(dir, EXE));
    },
    async () => {
      same(run(path.join(dir, "uninstall.exe"), "/S"), 0, "the uninstaller's exit code");
      await until("the uninstaller to remove the app", () => !fs.existsSync(dir) && registry(USER_ENTRY) === null);
    },
  );
  same(present(shortcuts), [], "the shortcuts left");
  same(registry(OWN_KEY), own, `what is left in ${OWN_KEY}`);
  checkAssociationsRemoved("HKCU", defaults);
  if (base) fs.rmSync(base, { recursive: true, force: true });
  console.log("the NSIS installer installed the app for the user, and its uninstaller left nothing of it");
}

/** The MSI: for the machine, into Program Files; it needs administrator rights. */
async function msi(pkg) {
  const shell = folders();
  same(machineEntries(), [], `the entries of ${NAME} among the apps installed for the machine before the test`);
  const dir = path.join(shell.ProgramFiles, NAME);
  check(!fs.existsSync(dir), `${dir} is there already.`);
  check(registry(`HKLM\\Software\\Classes\\${PROGID}`) === null, "RNL Viewer file associations already exist; leave them intact.");
  const defaults = associationDefaults("HKLM");
  const shortcuts = [path.join(shell.CommonPrograms, NAME, `${NAME}.lnk`), path.join(shell.CommonDesktopDirectory, `${NAME}.lnk`)];
  const log = path.join(os.tmpdir(), "rnl-viewer-msi.log");

  console.log(`installing ${pkg}, logging to ${log}`);
  // the feature Public packages is installed unless PUBLICPACKAGES=0 leaves it out
  same(run("msiexec", `/i "${pkg}" /qn /norestart${packages === "without" ? " PUBLICPACKAGES=0" : ""} /l*v "${log}"`), 0, "msiexec's exit code");
  await installed(
    () => {
      console.log(`installed: ${fs.readdirSync(dir).join(", ")}`);
      for (const file of [EXE, "LICENSE.txt"]) check(fs.existsSync(path.join(dir, file)), `${file} was not installed.`);
      checkPackages(dir);
      same(machineEntries(), [{ name: NAME, version: VERSION, publisher: PUBLISHER }], "the entry among the installed apps");
      same(present(shortcuts), shortcuts, "the shortcuts in the Start menu and on the desktop");
      checkAssociations("HKLM", path.join(dir, EXE), defaults);
      openAssociatedFile(path.join(dir, EXE));
      tryApp(path.join(dir, EXE));
    },
    () => same(run("msiexec", `/x "${pkg}" /qn /norestart`), 0, "msiexec's exit code when uninstalling"),
  );
  same(present([dir, ...shortcuts]), [], "the files and shortcuts left");
  same(machineEntries(), [], "the entries left among the installed apps");
  checkAssociationsRemoved("HKLM", defaults);
  console.log("the MSI installed the app for the machine, and uninstalling it left nothing of it");
}

try {
  const [setup, pkg] = [option("--nsis"), option("--msi")];
  check(!setup !== !pkg, "Give one installer to test: --nsis SETUP or --msi PACKAGE.");
  if (setup) await nsis(path.resolve(setup));
  else await msi(path.resolve(pkg));
} catch (e) {
  console.error(`The installers' test failed: ${e.message}`);
  process.exit(1);
}
