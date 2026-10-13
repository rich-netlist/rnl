// A smoke test of the desktop viewer, on Windows: build the app, start it on a netlist of the
// corpus, and drive its page through WebView2's DevTools port. The drawing is shown, the window is
// named after the file, a click selects an instance, Enter opens its subcircuit and Backspace comes
// back, and the native menu has the items of the command table, runs them, and follows the page's
// check marks. The app runs with a WebView2 profile of its own in a temporary directory, so that the
// user's layout stays as it was, and is stopped by its process ID. Node 22 or later, with no
// dependencies.
//
//   node apps/desktop/tests/smoke.mjs [--port PORT] [--no-build]
//
// PORT is the DevTools port, any free one by default; --no-build starts the app as it was built.

import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const exe = path.join(root, "apps", "desktop", "target", "debug", "rnl-viewer.exe");
const netlist = path.join(root, "tests", "conformance", "cases", "04-hierarchy.sp");
const TOP = "04-hierarchy.sp";
/** The command table, which the native menu is built from. */
const table = JSON.parse(fs.readFileSync(path.join(root, "apps", "viewer-ui", "commands.json"), "utf8"));
const TIMEOUT = 60_000; // the first start of WebView2 on a fresh machine can be slow

if (process.platform !== "win32") {
  console.log("The desktop smoke test runs on Windows only.");
  process.exit(0);
}
if (typeof WebSocket !== "function") throw new Error("The smoke test needs Node 22 or later, for its WebSocket.");

/** The value of an option such as --port, or undefined. */
function option(name) {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

/** A TCP port that no one listens on now. */
function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer().once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });
}

/** Wait until `check` gives a value that is not false, and return it. */
async function until(what, check, timeout = TIMEOUT) {
  const t0 = Date.now();
  let last = null;
  for (;;) {
    try {
      const v = await check();
      if (v) return v;
    } catch (e) {
      last = e;
    }
    if (Date.now() - t0 > timeout) throw new Error(`Timed out waiting for ${what}${last ? `: ${last.message}` : ""}.`);
    await new Promise((r) => setTimeout(r, 50));
  }
}

function check(ok, message) {
  if (!ok) throw new Error(message);
}

function same(got, want, what) {
  check(JSON.stringify(got) === JSON.stringify(want), `${what}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
}

/** Connect to the page's DevTools target; resolves to send(), evaluate() and the page's errors. */
async function connect(port) {
  const target = await until("the page's DevTools target", async () => {
    const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
    return targets.find((t) => t.type === "page");
  });
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.addEventListener("open", resolve);
    ws.addEventListener("error", () => reject(new Error("The DevTools connection failed.")));
  });
  let id = 0;
  const pending = new Map();
  const errors = [];
  ws.addEventListener("message", (m) => {
    const d = JSON.parse(m.data);
    if (d.id && pending.has(d.id)) {
      const { resolve, reject, method } = pending.get(d.id);
      pending.delete(d.id);
      if (d.error) reject(new Error(`${method}: ${d.error.message}`));
      else resolve(d.result);
    } else if (d.method === "Runtime.exceptionThrown") {
      const x = d.params.exceptionDetails;
      errors.push(x.exception ? x.exception.description : x.text);
    } else if (d.method === "Runtime.consoleAPICalled" && d.params.type === "error") {
      errors.push(d.params.args.map((a) => a.value ?? a.description).join(" "));
    }
  });
  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      pending.set(++id, { resolve, reject, method });
      ws.send(JSON.stringify({ id, method, params }));
    });
  const evaluate = async (expression) => {
    const r = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) throw new Error(`${expression}: ${r.exceptionDetails.exception?.description || r.exceptionDetails.text}`);
    return r.result.value;
  };
  await send("Runtime.enable");
  return { ws, send, evaluate, errors };
}

/** Click a page point, in CSS pixels, with the left button. */
async function click(page, { x, y }) {
  await page.send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y, button: "none", buttons: 0 });
  await page.send("Input.dispatchMouseEvent", { type: "mousePressed", x, y, button: "left", buttons: 1, clickCount: 1 });
  await page.send("Input.dispatchMouseEvent", { type: "mouseReleased", x, y, button: "left", buttons: 0, clickCount: 1 });
}

/** Press a key: its `key`, `code` and Windows key code, and the CDP modifiers (2 is Ctrl). */
async function press(page, key, code, vk, modifiers = 0) {
  for (const type of ["rawKeyDown", "keyUp"]) {
    await page.send("Input.dispatchKeyEvent", { type, key, code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers });
  }
}

/** What the page shows: the level's name, the selected object as "kind name", and whether it is busy. */
function shown(page) {
  return page.evaluate(`(() => {
    const v = window.viewer;
    const o = v && v.doc && v.selected !== null ? v.doc.objects[v.selected] : null;
    return {
      doc: !!(v && v.doc),
      drawn: !!document.querySelector("#canvas > svg"),
      busy: document.body.classList.contains("busy"),
      level: document.getElementById("level-name").textContent,
      selected: o ? o.kind + " " + o.name : null,
      zoom: document.getElementById("zoom").textContent,
      sidebar: !document.getElementById("app").classList.contains("sidebar-hidden"),
    };
  })()`);
}

/** The page point in the middle of an instance's symbol on the level shown. */
function symbol(page, name) {
  return page.evaluate(`(() => {
    const v = window.viewer;
    const hit = v.doc.levels[v.level].hits.find((h) => h.kind === "box" && h.objects.some((o) => v.doc.objects[o].name === ${JSON.stringify(name)}));
    const [[x0, y0], [x1, y1]] = hit.points;
    const canvas = document.getElementById("canvas");
    const [x, y, w] = document.querySelector("#canvas > svg").getAttribute("viewBox").split(" ").map(Number);
    const r = canvas.getBoundingClientRect();
    const k = canvas.clientWidth / w;
    return { x: r.left + ((x0 + x1) / 2 - x) * k, y: r.top + ((y0 + y1) / 2 - y) * k };
  })()`);
}

/** Run a PowerShell script and return what it prints. */
function powershell(script) {
  // no progress records, which PowerShell writes as XML to a redirected error stream
  const encoded = Buffer.from(`$ProgressPreference = "SilentlyContinue"\n${script}`, "utf16le").toString("base64");
  return execFileSync("powershell", ["-NoProfile", "-NonInteractive", "-EncodedCommand", encoded], { encoding: "utf8" }).trim();
}

const WIN32 = `Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class Win32Menu {
  [DllImport("user32.dll")] public static extern IntPtr GetMenu(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern int GetMenuItemCount(IntPtr hMenu);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetMenuString(IntPtr hMenu, uint item, StringBuilder text, int max, uint flags);
  [DllImport("user32.dll")] public static extern IntPtr GetSubMenu(IntPtr hMenu, int pos);
  [DllImport("user32.dll")] public static extern uint GetMenuState(IntPtr hMenu, uint item, uint flags);
  [DllImport("user32.dll")] public static extern uint GetMenuItemID(IntPtr hMenu, int pos);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr w, IntPtr l);
}
"@
`;

/** The title of the app's main window. */
function windowTitle(pid) {
  return powershell(`(Get-Process -Id ${pid}).MainWindowTitle`);
}

/** The native menu bar of the app's main window: per menu its text, and its items' texts, check marks and separators. */
function nativeMenu(pid) {
  return JSON.parse(
    powershell(`${WIN32}
$bar = [Win32Menu]::GetMenu((Get-Process -Id ${pid}).MainWindowHandle)
$menus = @()
for ($i = 0; $i -lt [Win32Menu]::GetMenuItemCount($bar); $i++) {
  $text = New-Object System.Text.StringBuilder 256
  [void][Win32Menu]::GetMenuString($bar, $i, $text, 256, 0x400)
  $sub = [Win32Menu]::GetSubMenu($bar, $i)
  $items = @()
  for ($j = 0; $j -lt [Win32Menu]::GetMenuItemCount($sub); $j++) {
    $item = New-Object System.Text.StringBuilder 256
    [void][Win32Menu]::GetMenuString($sub, $j, $item, 256, 0x400)
    $state = [Win32Menu]::GetMenuState($sub, $j, 0x400)
    $items += [pscustomobject]@{ text = $item.ToString(); separator = (($state -band 0x800) -ne 0); checked = (($state -band 0x8) -ne 0) }
  }
  $menus += [pscustomobject]@{ text = $text.ToString(); items = $items }
}
ConvertTo-Json -InputObject $menus -Depth 4 -Compress`),
  );
}

/** Choose a native menu item, as a click on it does: post its id to the window as WM_COMMAND. */
function chooseItem(pid, menu, item) {
  powershell(`${WIN32}
$h = (Get-Process -Id ${pid}).MainWindowHandle
$id = [Win32Menu]::GetMenuItemID([Win32Menu]::GetSubMenu([Win32Menu]::GetMenu($h), ${menu}), ${item})
[void][Win32Menu]::PostMessage($h, 0x0111, [IntPtr]$id, [IntPtr]::Zero)`);
}

/** The native menus the command table gives the desktop on Windows, the keys shown after a tab. */
function tableMenus() {
  const menus = [];
  for (const m of table.menus) {
    const items = [];
    let gap = false;
    for (const item of m.items) {
      if (item === "-") {
        gap = items.length > 0;
        continue;
      }
      if (item.platforms && !item.platforms.includes("desktop")) continue;
      if (gap) items.push({ text: "", separator: true, checked: false });
      gap = false;
      const keys = item.shown || (item.keys ? item.keys[0] : null);
      items.push({ text: keys ? `${item.label}\t${keys}` : item.label, separator: false, checked: !!item.check });
    }
    if (items.length > 0) menus.push({ text: m.label, items });
  }
  return menus;
}

async function main() {
  if (!process.argv.includes("--no-build")) {
    execFileSync("cargo", ["build", "--manifest-path", path.join(root, "apps", "desktop", "Cargo.toml")], { cwd: root, stdio: "inherit" });
  }
  const port = Number(option("--port")) || (await freePort());
  const profile = fs.mkdtempSync(path.join(os.tmpdir(), "rnl-viewer-smoke-"));
  const t0 = Date.now();
  const app = spawn(exe, [netlist], {
    cwd: root,
    env: {
      ...process.env,
      WEBVIEW2_USER_DATA_FOLDER: profile,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
    },
    stdio: ["ignore", "inherit", "inherit"],
  });
  const exited = new Promise((resolve) => app.once("exit", resolve));
  let page = null;
  try {
    page = await connect(port);
    // the page keeps its layout in the profile of the WebView: it must be this one, not the user's
    check(fs.readdirSync(profile).length > 0, `WebView2 did not take ${profile} as its profile.`);

    const s = await until("the drawing", async () => {
      const s = await shown(page);
      return s.doc && s.drawn && !s.busy && s;
    });
    console.log(`drawn ${s.level} in ${Date.now() - t0} ms, at ${s.zoom}`);
    same(s.level, TOP, "the level drawn");
    await until("the window's title", () => windowTitle(app.pid) === `${TOP} - RNL Viewer`, 10_000);

    // a click selects an instance; Enter opens its subcircuit and Backspace comes back
    await click(page, await symbol(page, "X2"));
    await until("X2 to be selected", async () => (await shown(page)).selected === "instance X2", 10_000);
    await press(page, "Enter", "Enter", 13);
    await until("the subcircuit amp", async () => (await shown(page)).level === "amp", 10_000);
    await press(page, "Backspace", "Backspace", 8);
    const back = await until("the top level", async () => {
      const s = await shown(page);
      return s.level === TOP && s;
    }, 10_000);
    same(back.selected, "instance X2", "the selection after going back up");
    console.log("selected X2, opened amp and came back");

    // the native menu: the command table's items, with the page's check marks
    const want = tableMenus();
    same(nativeMenu(app.pid), want, "the native menu");
    // a key in the page clears a check mark, and an item chosen in the menu reaches the page
    const view = want.findIndex((m) => m.text === "&View");
    const item = (id) => {
      const label = table.menus.flatMap((m) => m.items).find((i) => i.id === id).label;
      return want[view].items.findIndex((i) => i.text.split("\t")[0] === label);
    };
    const checked = (id) => nativeMenu(app.pid)[view].items[item(id)].checked;
    await press(page, "b", "KeyB", 66, 2);
    await until("the side panel to be hidden", async () => !(await shown(page)).sidebar, 10_000);
    await until("Netlist Panel's check mark to clear", () => !checked("toggle-netlist"), 10_000);
    chooseItem(app.pid, view, item("toggle-netlist"));
    await until("the side panel to be shown", async () => (await shown(page)).sidebar, 10_000);
    await until("Netlist Panel's check mark", () => checked("toggle-netlist"), 10_000);
    const zoom = (await shown(page)).zoom;
    chooseItem(app.pid, view, item("zoom-in"));
    await until("View > Zoom In", async () => (await shown(page)).zoom !== zoom, 10_000);
    console.log("the native menu has the command table's items and follows the page");

    same(page.errors, [], "the page's errors");
  } finally {
    if (page) page.ws.close();
    // stop the app and its WebView by the app's own process ID
    try {
      execFileSync("taskkill", ["/PID", String(app.pid), "/T", "/F"], { stdio: "ignore" });
    } catch {
      // it has exited already
    }
    await exited;
    // the WebView's own processes let go of the profile a little after the app is gone
    try {
      await until("the WebView to let go of its profile", () => {
        fs.rmSync(profile, { recursive: true, force: true });
        return true;
      }, 30_000);
    } catch (e) {
      console.warn(`The temporary profile ${profile} is left behind: ${e.message}`);
    }
  }
  console.log("the desktop smoke test passed");
}

await main();
