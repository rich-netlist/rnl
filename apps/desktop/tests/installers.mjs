// A test of the desktop viewer's installers, on Windows: install one silently, check what it
// installed, smoke-test the installed app with smoke.mjs, uninstall it silently, and check that
// nothing of it is left: its files, its shortcuts, its entry among the installed apps and, for the
// NSIS installer, its own registry key. Node 22 or later, with no dependencies.
//
//   node apps/desktop/tests/installers.mjs --nsis SETUP [--dir DIR] [--no-smoke]
//   node apps/desktop/tests/installers.mjs --msi PACKAGE [--no-smoke]
//
// The NSIS installer installs for the current user, into DIR or else a new temporary directory, and
// needs no administrator rights; the test stops when RNL Viewer is installed for the user already,
// so as to leave that installation be. The MSI installs for the machine, into Program Files, and
// needs administrator rights, as GitHub's runners have; the test stops when RNL Viewer is installed
// there already. --no-smoke leaves out the smoke test.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "../../..");
const config = JSON.parse(fs.readFileSync(path.join(root, "apps", "desktop", "tauri.conf.json"), "utf8"));
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
  return Number(powershell(`(Start-Process -FilePath ${quote(file)} -ArgumentList ${quote(args)} -Wait -PassThru).ExitCode`));
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

/** Smoke-test the installed app, unless --no-smoke. */
function tryApp(exe) {
  if (process.argv.includes("--no-smoke")) return;
  const r = spawnSync(process.execPath, [path.join(here, "smoke.mjs"), "--exe", exe], { cwd: root, stdio: "inherit" });
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

  console.log(`installing ${setup} into ${dir}`);
  same(run(setup, `/S /D=${dir}`), 0, "the installer's exit code");
  await installed(
    () => {
      console.log(`installed: ${fs.readdirSync(dir).join(", ")}`);
      for (const file of [EXE, "uninstall.exe", "LICENSE.txt", "rnl-solver.toml"]) check(fs.existsSync(path.join(dir, file)), `${file} was not installed.`);
      const entry = registry(USER_ENTRY);
      check(entry !== null, `There is no ${USER_ENTRY}.`);
      same([entry.DisplayName, entry.DisplayVersion, entry.Publisher], [NAME, VERSION, PUBLISHER], "the entry among the installed apps");
      same(present(shortcuts), shortcuts, "the shortcuts in the Start menu and on the desktop");
      tryApp(path.join(dir, EXE));
    },
    async () => {
      same(run(path.join(dir, "uninstall.exe"), "/S"), 0, "the uninstaller's exit code");
      await until("the uninstaller to remove the app", () => !fs.existsSync(dir) && registry(USER_ENTRY) === null);
    },
  );
  same(present(shortcuts), [], "the shortcuts left");
  same(registry(OWN_KEY), own, `what is left in ${OWN_KEY}`);
  if (base) fs.rmSync(base, { recursive: true, force: true });
  console.log("the NSIS installer installed the app for the user, and its uninstaller left nothing of it");
}

/** The MSI: for the machine, into Program Files; it needs administrator rights. */
async function msi(pkg) {
  const shell = folders();
  same(machineEntries(), [], `the entries of ${NAME} among the apps installed for the machine before the test`);
  const dir = path.join(shell.ProgramFiles, NAME);
  check(!fs.existsSync(dir), `${dir} is there already.`);
  const shortcuts = [path.join(shell.CommonPrograms, NAME, `${NAME}.lnk`), path.join(shell.CommonDesktopDirectory, `${NAME}.lnk`)];
  const log = path.join(os.tmpdir(), "rnl-viewer-msi.log");

  console.log(`installing ${pkg}, logging to ${log}`);
  same(run("msiexec", `/i "${pkg}" /qn /norestart /l*v "${log}"`), 0, "msiexec's exit code");
  await installed(
    () => {
      console.log(`installed: ${fs.readdirSync(dir).join(", ")}`);
      for (const file of [EXE, "LICENSE.txt", "rnl-solver.toml"]) check(fs.existsSync(path.join(dir, file)), `${file} was not installed.`);
      same(machineEntries(), [{ name: NAME, version: VERSION, publisher: PUBLISHER }], "the entry among the installed apps");
      same(present(shortcuts), shortcuts, "the shortcuts in the Start menu and on the desktop");
      tryApp(path.join(dir, EXE));
    },
    () => same(run("msiexec", `/x "${pkg}" /qn /norestart`), 0, "msiexec's exit code when uninstalling"),
  );
  same(present([dir, ...shortcuts]), [], "the files and shortcuts left");
  same(machineEntries(), [], "the entries left among the installed apps");
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
