// The WebView2 Runtime, which the desktop viewer runs on, on Windows: print the version installed,
// and with --install, install the runtime first when there is none, with Microsoft's bootstrapper,
// which the app's installers download too. Windows 10 and 11 come with the runtime; GitHub's Windows
// Server runners need not. Node 22 or later, with no dependencies.
//
//   node apps/desktop/tests/webview2.mjs [--install]
//
// The smoke test imports runtimeVersion() to say what it runs on.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

/** The runtime's client ID with EdgeUpdate, which keeps its version, "pv", under it. */
const CLIENT = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
/** Where the version is kept: installed for the machine, on 64-bit and 32-bit Windows, or for the user. */
const KEYS = [
  `HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\${CLIENT}`,
  `HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\${CLIENT}`,
  `HKCU\\Software\\Microsoft\\EdgeUpdate\\Clients\\${CLIENT}`,
];
/** Microsoft's Evergreen bootstrapper. */
const BOOTSTRAPPER = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";

/** The version of the WebView2 Runtime installed, or null when there is none. */
export function runtimeVersion() {
  for (const key of KEYS) {
    const r = spawnSync("reg", ["query", key, "/v", "pv"], { encoding: "utf8" });
    const m = r.status === 0 ? /\bpv\s+REG_SZ\s+(\S+)/.exec(r.stdout) : null;
    if (m && m[1] !== "0.0.0.0") return m[1];
  }
  return null;
}

/** Download the bootstrapper and run it silently; it installs the runtime for the machine. */
async function install() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "webview2-"));
  try {
    const setup = path.join(dir, "MicrosoftEdgeWebview2Setup.exe");
    const response = await fetch(BOOTSTRAPPER);
    if (!response.ok) throw new Error(`The bootstrapper could not be downloaded: ${response.status} ${response.statusText}.`);
    fs.writeFileSync(setup, Buffer.from(await response.arrayBuffer()));
    const r = spawnSync(setup, ["/silent", "/install"], { stdio: "inherit", timeout: 600_000 });
    if (r.status !== 0) throw new Error(`The bootstrapper failed: ${r.error ? r.error.message : `exit code ${r.status}`}.`);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

async function main() {
  if (process.platform !== "win32") {
    console.log("The WebView2 Runtime is for Windows only.");
    return;
  }
  let version = runtimeVersion();
  if (!version && process.argv.includes("--install")) {
    console.log("There is no WebView2 Runtime: installing it with Microsoft's bootstrapper.");
    await install();
    version = runtimeVersion();
  }
  if (!version) throw new Error("There is no WebView2 Runtime.");
  console.log(`WebView2 Runtime ${version}`);
}

// run as a script, not when imported; Windows' paths differ in case only as the same path
if (process.argv[1] && path.resolve(process.argv[1]).toLowerCase() === fileURLToPath(import.meta.url).toLowerCase()) {
  try {
    await main();
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
}
