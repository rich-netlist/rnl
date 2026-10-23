// Drive the actual Linux binary through WebKitGTK's native WebDriver, as tauri-driver does:
// https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-driver/src/server.rs
// Node 22+, webkit2gtk-driver and xdotool; run under dbus-run-session and xvfb-run.
// No build, plugins or test-only code in the app. --exe accepts an AppImage or an installed binary.
//   dbus-run-session -- xvfb-run -a node apps/desktop/tests/linux-smoke.mjs --exe PATH
import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { once } from "node:events";
import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { screenshotEvidence, assertPainted } from "./visual.mjs";

assert.equal(process.platform, "linux", "The Linux smoke test must run on Linux.");
const exeIndex = process.argv.indexOf("--exe");
assert.ok(exeIndex >= 0 && process.argv[exeIndex + 1], "Pass --exe PATH.");
const exe = path.resolve(process.argv[exeIndex + 1]);
fs.accessSync(exe, fs.constants.X_OK);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const temp = fs.mkdtempSync(path.join(os.tmpdir(), "rnl-linux-smoke-"));
const output = process.env.SMOKE_OUTPUT_DIR || temp;
fs.mkdirSync(output, { recursive: true });
const sample = "04-hierarchy.sp";
for (const name of [sample, "04-hierarchy-lib.inc"]) {
  fs.copyFileSync(path.join(root, "tests/conformance/cases", name), path.join(temp, name));
}
const original = fs.readFileSync(path.join(temp, sample));
const server = net.createServer();
server.listen(0, "127.0.0.1");
await once(server, "listening");
const port = server.address().port;
await new Promise((resolve) => server.close(resolve));
const driver = spawn("WebKitWebDriver", [`--port=${port}`, "--host=127.0.0.1"], {
  detached: true,
  stdio: ["ignore", "inherit", "inherit"],
  env: {
    ...process.env,
    TAURI_WEBVIEW_AUTOMATION: "true",
    XDG_CONFIG_HOME: path.join(temp, "config"),
    XDG_DATA_HOME: path.join(temp, "data"),
    XDG_CACHE_HOME: path.join(temp, "cache"),
    LIBGL_ALWAYS_SOFTWARE: "1",
  },
});
let driverError;
driver.on("error", (e) => { driverError = e; });
let session;
let last;

async function request(method, route, body) {
  const response = await fetch(`http://127.0.0.1:${port}${route}`, {
    method,
    headers: { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(60_000),
  });
  const result = await response.json();
  assert.ok(response.ok && !result.value?.error, `${method} ${route}: ${JSON.stringify(result)}`);
  return result.value;
}

async function until(what, check) {
  const end = Date.now() + 60_000;
  let error;
  while (Date.now() < end) {
    if (driverError) throw driverError;
    assert.equal(driver.exitCode, null, "WebKitWebDriver exited early.");
    try {
      const value = await check();
      if (value) return value;
    } catch (e) { error = e; }
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error(`Timed out waiting for ${what}; last page: ${JSON.stringify(last)}`, { cause: error });
}

try {
  await until("WebKitWebDriver", () => request("GET", "/status"));
  const created = await request("POST", "/session", {
    capabilities: { alwaysMatch: {
      "webkitgtk:browserOptions": { binary: exe, args: [path.join(temp, sample)] },
    } },
  });
  session = created.sessionId;
  assert.ok(session, "WebKitWebDriver returned no session.");
  await until("the sample's drawing", async () => {
    last = await request("POST", `/session/${session}/execute/sync`, { script: `
      const v = window.viewer;
      const svg = document.querySelector("#canvas > svg");
      return {
        loaded: !!v?.doc, level: document.getElementById("level-name")?.textContent,
        drawn: !!svg && svg.getBoundingClientRect().width > 0 && svg.childElementCount > 0,
        levels: v?.doc?.levels.map(l => l.title),
        busy: document.body.classList.contains("busy"),
        errors: [...document.querySelectorAll("#diags .error")].map(e => e.textContent),
        empty: document.getElementById("empty")?.textContent
      };`, args: [] });
    return last.loaded && last.drawn && !last.busy;
  });
  assert.equal(last.level, sample);
  assert.ok(last.levels.includes("amp"), "The sample's subcircuit was not loaded.");
  assert.deepEqual(last.errors, [], "The sample opened with errors.");
  // A valid DOM can still be invisible. Wait for the startup handoff, then require pixels
  // in the tab text, source and schematic. Borders and the grid do not satisfy this check.
  await until("painted tab, source and schematic pixels", async () => {
    const screenshot = await request("GET", `/session/${session}/screenshot`);
    fs.writeFileSync(path.join(output, `${path.basename(exe)}.png`), Buffer.from(screenshot, "base64"));
    const visual = await request("POST", `/session/${session}/execute/sync`, {
      script:`return (${screenshotEvidence.toString()})(${JSON.stringify(screenshot)});`, args:[] });
    fs.writeFileSync(path.join(output, `${path.basename(exe)}-pixels.json`), JSON.stringify(visual,null,2));
    assertPainted(visual);
    return true;
  });
  // An actual visible X11 window, in addition to the WebView's rendered DOM.
  const windows = await until("the visible window's title", () =>
    execFileSync("xdotool", ["search", "--onlyvisible", "--name", `^${sample} - RNL Viewer$`], { encoding: "utf8" }));
  assert.ok(windows.trim(), "There is no visible viewer window.");
  assert.deepEqual(fs.readFileSync(path.join(temp, sample)), original, "Opening the sample wrote to it.");
  console.log(`${path.basename(exe)}: visible window, sample and include loaded, SVG drawn, no errors.`);
} finally {
  if (session) {
    try {
      const screenshot = await request("GET", `/session/${session}/screenshot`);
      fs.writeFileSync(path.join(output, `${path.basename(exe)}.png`), Buffer.from(screenshot, "base64"));
    } catch (e) { console.error("Screenshot:", e.message); }
    await request("DELETE", `/session/${session}`).catch((e) => console.error("Session cleanup:", e.message));
  }
  if (driver.pid) {
    try { process.kill(-driver.pid, "SIGKILL"); } catch (e) { if (e.code !== "ESRCH") throw e; }
  }
  fs.rmSync(temp, { recursive: true, force: true });
}
