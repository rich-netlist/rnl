// A smoke test of the desktop viewer, on Windows: build the app, start it on a netlist of the
// corpus, and drive its page through WebView2's DevTools port. The drawing is shown, the window is
// named after the file, a click selects an instance, Enter opens its subcircuit and Backspace comes
// back, the native menu has the items of the command table, those of its submenus too, runs them,
// and follows the page's check marks, and Help > Check for Updates asks GitHub for the releases,
// answered by the test, and offers a newer one. The app runs with a WebView2 profile of its own in
// a temporary directory, so that the user's layout stays as it was, and is stopped by its process
// ID. The test fails at once when the app exits, and a step fails when the page stops answering.
// Node 22 or later, with no dependencies.
//
//   node apps/desktop/tests/smoke.mjs [--port PORT] [--no-build | --exe PATH]
//
// PORT is the DevTools port, any free one by default; --no-build starts the app as it was built;
// --exe starts the app at PATH instead, such as a release build or an installed app, and builds
// nothing.

import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const manifest = path.join(root, "apps", "desktop", "Cargo.toml");
const netlist = path.join(root, "tests", "conformance", "cases", "04-hierarchy.sp");
const TOP = "04-hierarchy.sp";
/** The command table, which the native menu is built from. */
const table = JSON.parse(fs.readFileSync(path.join(root, "apps", "viewer-ui", "commands.json"), "utf8"));
/** The app's version, which Tauri reports. */
const version = JSON.parse(fs.readFileSync(path.join(root, "apps", "desktop", "tauri.conf.json"), "utf8")).version;
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

/**
 * Wait until `check` gives a value that is not false, and return it. A check that throws is tried
 * again, unless its error is final, as when the page's DevTools connection is gone.
 */
async function until(what, check, timeout = TIMEOUT) {
  const t0 = Date.now();
  let last = null;
  for (;;) {
    try {
      const v = await check();
      if (v) return v;
    } catch (e) {
      if (e.final) throw new Error(`Gave up waiting for ${what}: ${e.message}`);
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

/** The app as cargo builds it, in the target directory that CARGO_TARGET_DIR or cargo's configuration may move. */
function appPath() {
  const metadata = execFileSync("cargo", ["metadata", "--manifest-path", manifest, "--format-version", "1", "--no-deps"], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 1 << 26,
  });
  return path.join(JSON.parse(metadata).target_directory, "debug", "rnl-viewer.exe");
}

/**
 * Connect to the page's DevTools target; resolves to send(), evaluate(), on(), which handles an
 * event, and the page's errors. A call rejects when it has no answer within TIMEOUT, and every call
 * rejects, with a final error, once the connection is gone.
 */
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
  const pending = new Map(); // the calls waiting for their answers, by id
  const handlers = new Map(); // the handlers of events, by method
  const errors = [];
  let gone = null; // the error of every call once the connection is gone
  const lose = (why) => {
    if (gone) return;
    gone = Object.assign(new Error(why), { final: true });
    for (const { reject } of pending.values()) reject(gone);
    pending.clear();
  };
  ws.addEventListener("close", () => lose("the DevTools connection closed"));
  ws.addEventListener("error", () => lose("the DevTools connection failed"));
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
    } else if (handlers.has(d.method)) {
      handlers.get(d.method)(d.params);
    }
  });
  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      if (gone) return reject(gone);
      const n = ++id;
      const timer = setTimeout(() => {
        pending.delete(n);
        reject(new Error(`${method} had no answer in ${TIMEOUT / 1000} s`));
      }, TIMEOUT);
      const settle = (f) => (v) => {
        clearTimeout(timer);
        f(v);
      };
      pending.set(n, { resolve: settle(resolve), reject: settle(reject), method });
      ws.send(JSON.stringify({ id: n, method, params }));
    });
  const evaluate = async (expression) => {
    const r = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) throw new Error(`${expression}: ${r.exceptionDetails.exception?.description || r.exceptionDetails.text}`);
    return r.result.value;
  };
  await send("Runtime.enable");
  const on = (method, handler) => handlers.set(method, handler);
  return { ws, send, evaluate, on, errors };
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

/**
 * The native menu bar of the app's main window: per menu its text, and its items' texts, check marks
 * and separators, a submenu's with its own items. A submenu's entry has its flags in the low byte of
 * its state, and the number of its items in the high byte.
 */
function nativeMenu(pid) {
  return JSON.parse(
    powershell(`${WIN32}
function Read-Menu([IntPtr]$menu) {
  $items = @()
  for ($j = 0; $j -lt [Win32Menu]::GetMenuItemCount($menu); $j++) {
    $text = New-Object System.Text.StringBuilder 256
    [void][Win32Menu]::GetMenuString($menu, $j, $text, 256, 0x400)
    $state = [Win32Menu]::GetMenuState($menu, $j, 0x400)
    $sub = [Win32Menu]::GetSubMenu($menu, $j)
    if ($sub -ne [IntPtr]::Zero) { $state = $state -band 0xFF }
    $item = [ordered]@{ text = $text.ToString(); separator = (($state -band 0x800) -ne 0); checked = (($state -band 0x8) -ne 0) }
    if ($sub -ne [IntPtr]::Zero) { $item.items = Read-Menu $sub }
    $items += [pscustomobject]$item
  }
  return ,$items
}
$bar = [Win32Menu]::GetMenu((Get-Process -Id ${pid}).MainWindowHandle)
$menus = @()
for ($i = 0; $i -lt [Win32Menu]::GetMenuItemCount($bar); $i++) {
  $text = New-Object System.Text.StringBuilder 256
  [void][Win32Menu]::GetMenuString($bar, $i, $text, 256, 0x400)
  $menus += [pscustomobject]@{ text = $text.ToString(); items = (Read-Menu ([Win32Menu]::GetSubMenu($bar, $i))) }
}
ConvertTo-Json -InputObject $menus -Depth 10 -Compress`),
  );
}

/**
 * Choose a native menu item, as a click on it does: post its id to the window as WM_COMMAND. `at` is
 * where it is: the menu's index, the indexes of the submenus' entries down to it, and its own.
 */
function chooseItem(pid, at) {
  powershell(`${WIN32}
$h = (Get-Process -Id ${pid}).MainWindowHandle
$menu = [Win32Menu]::GetMenu($h)
foreach ($k in @(${at.slice(0, -1).join(", ")})) { $menu = [Win32Menu]::GetSubMenu($menu, $k) }
$id = [Win32Menu]::GetMenuItemID($menu, ${at[at.length - 1]})
[void][Win32Menu]::PostMessage($h, 0x0111, [IntPtr]$id, [IntPtr]::Zero)`);
}

/** The commands of the table, those in its submenus too. */
const commands = (items) => items.flatMap((item) => (item === "-" ? [] : item.items ? commands(item.items) : [item]));

/**
 * The native menus the command table gives the desktop on Windows, the keys shown after a tab, and
 * the check marks the page has set: `marks` by command id.
 */
function tableMenus(marks) {
  const entries = (items) => {
    const shown = [];
    let gap = false;
    for (const item of items) {
      if (item === "-") {
        gap = shown.length > 0;
        continue;
      }
      let next;
      if (item.items) {
        const inner = entries(item.items);
        if (inner.length === 0) continue;
        next = { text: item.label, separator: false, checked: false, items: inner };
      } else {
        if (item.platforms && !item.platforms.includes("desktop")) continue;
        const keys = item.shown || (item.keys ? item.keys[0] : null);
        next = { text: keys ? `${item.label}\t${keys}` : item.label, separator: false, checked: !!item.check && marks[item.id] };
      }
      if (gap) shown.push({ text: "", separator: true, checked: false });
      gap = false;
      shown.push(next);
    }
    return shown;
  };
  return table.menus.map((m) => ({ text: m.label, items: entries(m.items) })).filter((m) => m.items.length > 0);
}

/** Where a command is in the native menus, as chooseItem takes it; and its item there. */
function find(menus, id) {
  const label = commands(table.menus.flatMap((m) => m.items)).find((c) => c.id === id).label;
  const search = (items, at) => {
    for (let k = 0; k < items.length; k++) {
      const found = items[k].items ? search(items[k].items, [...at, k]) : items[k].text.split("\t")[0] === label && [...at, k];
      if (found) return found;
    }
    return null;
  };
  for (let m = 0; m < menus.length; m++) {
    const at = search(menus[m].items, [m]);
    if (at) return { at, item: at.slice(1).reduce((entry, k) => entry.items[k], menus[m]) };
  }
  throw new Error(`The native menu has no item for ${id}.`);
}

/** Drive the app's page through the DevTools port and check what it shows; `session` keeps the connection. */
async function drive(app, port, profile, t0, session) {
  const page = await connect(port);
  session.page = page;
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
  const ids = commands(table.menus.flatMap((m) => m.items)).filter((c) => c.check).map((c) => c.id);
  const marks = await page.evaluate(`Object.fromEntries(${JSON.stringify(ids)}.map((id) => [id, window.viewer.checked(id)]))`);
  same(marks["grid-dots"] && !marks["grid-lines"], true, "the grid's marks at start");
  const want = tableMenus(marks);
  same(nativeMenu(app.pid), want, "the native menu");
  // a key in the page clears a check mark, and an item chosen in the menu reaches the page
  const at = (id) => find(want, id).at;
  const checked = (id) => find(nativeMenu(app.pid), id).item.checked;
  await press(page, "b", "KeyB", 66, 2);
  await until("the side panel to be hidden", async () => !(await shown(page)).sidebar, 10_000);
  await until("Netlist Panel's check mark to clear", () => !checked("toggle-netlist"), 10_000);
  chooseItem(app.pid, at("toggle-netlist"));
  await until("the side panel to be shown", async () => (await shown(page)).sidebar, 10_000);
  await until("Netlist Panel's check mark", () => checked("toggle-netlist"), 10_000);
  const zoom = (await shown(page)).zoom;
  chooseItem(app.pid, at("zoom-in"));
  await until("View > Zoom In", async () => (await shown(page)).zoom !== zoom, 10_000);
  // an item of a submenu reaches the page too, and the page sets the marks of its group
  const grid = () => page.evaluate("window.viewer.grid");
  chooseItem(app.pid, at("grid-lines"));
  await until("View > Grid > Lines", async () => (await grid()).style === "lines", 10_000);
  await until("the marks of Lines and Dots", () => checked("grid-lines") && !checked("grid-dots"), 10_000);
  // Lines chosen again stays checked, though the native menu clears the mark of the item chosen;
  // Axes chosen after it tells when both have reached the page
  chooseItem(app.pid, at("grid-lines"));
  chooseItem(app.pid, at("grid-axes"));
  await until("View > Grid > Axes", async () => (await grid()).axes, 10_000);
  await until("the marks of Lines and Axes", () => checked("grid-lines") && checked("grid-axes"), 10_000);
  same((await grid()).style, "lines", "the grid's style");
  console.log("the native menu has the command table's items, its submenus too, and follows the page");

  // Help > Check for Updates: the app's version, from Tauri; the page asks GitHub's API, which the
  // content security policy lets it reach, for the releases, answered here with a newer one, and
  // offers it; Cancel closes the dialog, which opens no browser
  same(await page.evaluate("window.__TAURI__.app.getVersion()"), version, "the app's version");
  page.on("Fetch.requestPaused", ({ requestId }) => {
    const releases = [{ tag_name: "viewer-v99.0.0", draft: false, prerelease: false }];
    const headers = { "Content-Type": "application/json", "Access-Control-Allow-Origin": "*" };
    page
      .send("Fetch.fulfillRequest", {
        requestId,
        responseCode: 200,
        responseHeaders: Object.entries(headers).map(([name, value]) => ({ name, value })),
        body: Buffer.from(JSON.stringify(releases)).toString("base64"),
      })
      .catch(() => {});
  });
  await page.send("Fetch.enable", { patterns: [{ urlPattern: "https://api.github.com/*" }] });
  const help = want.findIndex((m) => m.text === "&Help");
  chooseItem(app.pid, help, want[help].items.findIndex((i) => i.text === "Check for &Updates..."));
  const said = await until("the dialog of Check for Updates", () =>
    page.evaluate(`(() => {
      const p = document.querySelector("dialog.dialog .dialog-body p");
      return p && p.textContent;
    })()`),
  );
  same(said, "RNL Viewer 99.0.0 is available.", "what Check for Updates found");
  await page.evaluate(`[...document.querySelectorAll("dialog.dialog button")].find((b) => b.textContent === "Cancel").click()`);
  await until("the dialog to close", async () => !(await page.evaluate(`!!document.querySelector("dialog.dialog")`)), 10_000);
  await page.send("Fetch.disable");
  console.log(`Help > Check for Updates offered the newer release GitHub was made to list, against ${version}`);

  same(page.errors, [], "the page's errors");
}

async function main() {
  const given = option("--exe");
  if (!given && !process.argv.includes("--no-build")) {
    execFileSync("cargo", ["build", "--manifest-path", manifest], { cwd: root, stdio: "inherit" });
  }
  const exe = given ? path.resolve(given) : appPath();
  check(fs.existsSync(exe), `There is no ${exe} to start${given ? "" : ": build the app, or leave out --no-build"}.`);
  console.log(`starting ${exe}`);
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
  let end = null; // how the app ended, or why it did not start
  const exited = new Promise((resolve) => {
    app.once("exit", (code, signal) => {
      end = signal || `code ${code}`;
      resolve();
    });
    app.once("error", (e) => {
      end = e.message;
      resolve();
    });
  });
  // the test fails at once when the app ends before it is stopped
  const early = exited.then(() => {
    throw new Error(`The app ended (${end}) before the test was done.`);
  });
  early.catch(() => {}); // the race below reports it; the end of the app after the test is expected
  const session = { page: null };
  try {
    await Promise.race([drive(app, port, profile, t0, session), early]);
  } finally {
    if (session.page) session.page.ws.close();
    // stop the app and its WebView by the app's own process ID while it is still the app's: Node
    // holds the process until it has seen it exit, and after that Windows may give the ID to another
    if (app.pid !== undefined && app.exitCode === null && app.signalCode === null) {
      try {
        execFileSync("taskkill", ["/PID", String(app.pid), "/T", "/F"], { stdio: "ignore" });
      } catch {
        // it has exited already
      }
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

try {
  await main();
} catch (e) {
  console.error(`The desktop smoke test failed: ${e.message}`);
  process.exit(1);
}
