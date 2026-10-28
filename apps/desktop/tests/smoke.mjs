// A smoke test of the desktop viewer, on Windows: build the app, start it on a netlist of the
// corpus, and drive its page through the DevTools port that the app's --remote-debugging-port=PORT
// opens in its WebView. The drawing is shown, the window is named after the file, a click selects
// an instance, Enter opens its subcircuit and Backspace comes back, the native menu has the items
// of the command table, those of its submenus too, runs them, and follows the page's check marks,
// Help > Check for Updates asks GitHub for the releases, answered by the test, and offers a newer
// one, Solve > Solve draws the netlist solved as a preview and leaves the file on disk as it was,
// Edit > Edit Text and Apply Changes build the text edited natively and leave the file as it was, a
// window asked to close with the change not saved asks, File > Save writes the one file, and File >
// Load from Clipboard draws the text the test gives the page as the clipboard's. The netlist is
// a copy of one of the corpus, in a directory of the test's own, which it may save over. The
// app runs with a WebView2 profile of its own in a temporary directory, so that the user's layout
// stays as it was, and with settings of its own: a debug build keeps them where
// RNL_VIEWER_TEST_CONFIG_DIR says, and for a release build, which keeps them where Tauri puts them
// whatever the environment says, the user's settings file is set aside for the run and put back
// after, so that each run starts from the defaults. The app is stopped by its process ID. The test fails at once when the app exits, and
// a step fails when the page stops answering. Node 22 or later, with no dependencies.
//
//   node apps/desktop/tests/smoke.mjs [--port PORT] [--no-build | --exe PATH] [--packages with|without]
//
// PORT is the DevTools port, any free one by default; --no-build starts the app as it was built;
// --exe starts the app at PATH instead, such as a release build or an installed app, and builds
// nothing. --packages says whether the app bundles the public packages: with, it finds eda-style
// without a network and lists the bundled ones in the Library and in Help > Public Packages; without,
// it reports the package missing and points to Help > Public Packages.

import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { runtimeVersion } from "./webview2.mjs";
import { uiCatalog, uiLanguages } from "../../../scripts/i18n/viewer-locales.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const manifest = path.join(root, "apps", "desktop", "Cargo.toml");
const cases = path.join(root, "tests", "conformance", "cases");
/** Copies of the netlist and its include, in a directory of the test's own so that it may save over them; made in main. */
let netlist = null;
let netlistInclude = null;
const TOP = "04-hierarchy.sp";
/** The command table, which the native menu is built from, with the languages of i18n/ in its Language
 * menu after English, as the app is built with them (build.rs). */
const table = JSON.parse(fs.readFileSync(path.join(root, "apps", "viewer-ui", "commands.json"), "utf8"));
for (const menu of table.menus) (function addLanguages(items) {
  for (const item of items.filter((i) => i.items)) {
    if (item.languages) item.items.push(...uiLanguages(root).slice(1).map((l) => ({ id: `language-${l.lang}`, label: l.name, check: true, radio: true })));
    addLanguages(item.items);
  }
})(menu.items);
const tauriConf = JSON.parse(fs.readFileSync(path.join(root, "apps", "desktop", "tauri.conf.json"), "utf8"));
/** The app's version, which Tauri reports. */
const version = tauriConf.version;
/** The settings file of a release build on Windows: in Tauri's app_config_dir, which no variable of the environment moves. */
const releaseSettings = path.join(process.env.APPDATA || path.join(os.homedir(), "AppData", "Roaming"), tauriConf.identifier, "settings.json");

/**
 * Set the release build's settings file aside, so that the run starts from the defaults and leaves
 * the user's settings as they were; return what puts it back. A file set aside before and not put
 * back, by a run that was killed, stops the test rather than be written over.
 */
function setSettingsAside() {
  const kept = `${releaseSettings}.smoke-kept`;
  check(!fs.existsSync(kept), `${kept} is left from an earlier run: put it back as ${releaseSettings}, or delete it.`);
  if (fs.existsSync(releaseSettings)) fs.renameSync(releaseSettings, kept);
  return () => {
    fs.rmSync(releaseSettings, { force: true });
    if (fs.existsSync(kept)) fs.renameSync(kept, releaseSettings);
  };
}
/** What the app is expected to bundle of the public packages, "with" or "without"; unchecked when not given. */
const expectPackages = (() => {
  const i = process.argv.indexOf("--packages");
  const value = i >= 0 ? process.argv[i + 1] : undefined;
  if (value !== undefined && value !== "with" && value !== "without") throw new Error("--packages takes with or without.");
  return value;
})();
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
async function hover(page, { x, y }) {
  await page.send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y, button: "none", buttons: 0 });
}

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

/** The page point in the middle of the label with the group id `id`, such as `#X3::name`, on the level shown. */
function labelPoint(page, id) {
  return page.evaluate(`(() => {
    const v = window.viewer;
    const hit = v.doc.levels[v.level].hits.find((h) => h.kind === "label" && h.id === ${JSON.stringify(id)});
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

function itemEnabled(pid, at) {
  return powershell(`${WIN32}
$menu = [Win32Menu]::GetMenu((Get-Process -Id ${pid}).MainWindowHandle)
foreach ($k in @(${at.slice(0, -1).join(", ")})) { $menu = [Win32Menu]::GetSubMenu($menu, $k) }
([Win32Menu]::GetMenuState($menu, ${at.at(-1)}, 0x400) -band 3) -eq 0`) === "True";
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
  await page.evaluate(`window.viewer.command('language-en')`);
  same(s.level, TOP, "the level drawn");
  await until("the window's title", () => windowTitle(app.pid) === `${TOP} - RNL Viewer`, 10_000);
  await until("painted tab, source and schematic pixels", async () => {
    const {screenshotEvidence, assertPainted} = await import('./visual.mjs');
    const {data} = await page.send("Page.captureScreenshot", {format:"png"});
    const visual = await page.evaluate(`(${screenshotEvidence.toString()})(${JSON.stringify(data)})`);
    if (process.env.SMOKE_OUTPUT_DIR) {
      fs.writeFileSync(path.join(process.env.SMOKE_OUTPUT_DIR,"native-startup.png"),Buffer.from(data,"base64"));
      fs.writeFileSync(path.join(process.env.SMOKE_OUTPUT_DIR,"native-startup-pixels.json"),JSON.stringify(visual,null,2));
    }
    assertPainted(visual);
    return true;
  });
  same(await page.evaluate("window.__TAURI__.app.getVersion()"), version, "the running Viewer version");
  await page.evaluate(`(async () => {
    (await import('./panels.js')).setPane('netlist');
    (await import('./text.js')).revealLine(0, 8, false, true);
  })()`);
  check(await page.evaluate("document.querySelector('#source .file:not([hidden]) [data-line=\"8\"] .fold-toggle')?.getAttribute('aria-expanded') === 'true'"), "the subcircuit fold control is missing");
  if (process.env.SMOKE_OUTPUT_DIR) {
    const capture = await page.send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(path.join(process.env.SMOKE_OUTPUT_DIR, "native-source-expanded.png"), Buffer.from(capture.data, "base64"));
  }
  await page.evaluate("document.querySelector('#source .file:not([hidden]) [data-line=\"8\"] .fold-toggle').click()");
  check(await page.evaluate("!document.querySelector('#source .file:not([hidden]) [data-line=\"9\"]')"), "folding kept the subcircuit body visible");
  if (process.env.SMOKE_OUTPUT_DIR) {
    const capture = await page.send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(path.join(process.env.SMOKE_OUTPUT_DIR, "native-source-folded.png"), Buffer.from(capture.data, "base64"));
  }
  await page.evaluate("document.querySelector('#source .file:not([hidden]) [data-line=\"8\"] .fold-toggle').click()");

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

  // a click on an instance's label selects the instance, which the native Document gives the label's hit for
  await click(page, await labelPoint(page, "#X3::name"));
  await until("X3 to be selected by its label", async () => (await shown(page)).selected === "instance X3", 10_000);
  console.log("a click on X3's label selected X3");

  // the native menu: the command table's items, with the page's check marks
  const ids = commands(table.menus.flatMap((m) => m.items)).filter((c) => c.check).map((c) => c.id);
  const marks = await page.evaluate(`Object.fromEntries(${JSON.stringify(ids)}.map((id) => [id, window.viewer.checked(id)]))`);
  same(marks["grid-dots"] && !marks["grid-lines"], true, "the grid's marks at start");
  const want = tableMenus(marks);
  same(nativeMenu(app.pid), want, "the native menu");
  // a key in the page clears a check mark, and an item chosen in the menu reaches the page
  const at = (id) => find(want, id).at;
  // Use the real native command, then compare every label and preserve checks/shortcuts.
  await page.evaluate(`window.localeDocument = window.viewer.doc; window.localeSvg = document.querySelector('#canvas > svg')`);
  chooseItem(app.pid, at("language-zh"));
  await until("Chinese UI", () => page.evaluate(`document.documentElement.lang === 'zh-CN' && window.viewer.checked('language-zh')`));
  const catalog = uiCatalog(root, "zh-CN");
  // the View menu and its Language submenu have their English word after the translation (`english`)
  const english = new Map();
  const words = (items) => { for (const item of items) if (item !== "-" && item.items) { if (item.english) english.set(item.label, item.english); words(item.items); } };
  words(table.menus);
  const translatedMenu = items => items.map(item => {
    const [label, keys] = item.text.split('\t');
    return { ...item, text: (catalog[label] || label) + (english.has(label) ? ` (${english.get(label)})` : '') + (keys ? '\t' + keys : ''),
      ...(item.items ? {items: translatedMenu(item.items)} : {}) };
  });
  same(nativeMenu(app.pid), translatedMenu(tableMenus({...marks, "language-en":false, "language-zh":true})), "Chinese native menu");
  same(await page.evaluate(`window.localeDocument === window.viewer.doc && window.localeSvg === document.querySelector('#canvas > svg')`), true, "language change preserves document and drawing");
  chooseItem(app.pid, at("about"));
  await until("Chinese About", () => page.evaluate(`document.querySelector('dialog[open] .dialog-title')?.textContent === '关于 RNL Viewer'`));
  await page.evaluate(`document.querySelector('dialog[open] button').click()`);
  chooseItem(app.pid, at("language-en"));
  await until("English UI", () => page.evaluate(`document.documentElement.lang === 'en' && window.viewer.checked('language-en')`));
  same(nativeMenu(app.pid), want, "restored English native menu");
  chooseItem(app.pid, at("file-associations"));
  await until("the File Associations dialog", () => page.evaluate(`document.querySelector('dialog[open] .dialog-title')?.textContent === 'File Associations'`));
  check(await page.evaluate(`document.querySelector('dialog[open]').textContent.includes('.netlist')`), "file-association help omits .netlist");
  await page.evaluate(`Array.from(document.querySelectorAll('dialog[open] button')).find(b => b.textContent === 'Cancel').click()`);
  const checked = (id) => find(nativeMenu(app.pid), id).item.checked;
  await press(page, "b", "KeyB", 66, 2);
  await until("the side panel to be hidden", async () => !(await shown(page)).sidebar, 10_000);
  await until("Netlist pane's check mark to clear", () => !checked("panel-netlist"), 10_000);
  chooseItem(app.pid, at("panel-netlist"));
  await until("the side panel to be shown", async () => (await shown(page)).sidebar, 10_000);
  await until("Netlist pane's check mark", () => checked("panel-netlist"), 10_000);
  chooseItem(app.pid, at("panel-symbol"));
  await until("Symbol check mark", () => checked("panel-symbol") && !checked("panel-netlist"));
  chooseItem(app.pid, at("panel-symbol"));
  await until("Symbol stays checked when selected again", () => checked("panel-symbol"));
  chooseItem(app.pid, at("panel-hide"));
  await until("Hide disabled and every pane unchecked", () => {
    const menu = nativeMenu(app.pid);
    return !itemEnabled(app.pid, at("panel-hide")) &&
      ["files", "netlist", "hierarchy", "symbol", "unplaced", "solve"].every(name => !find(menu, `panel-${name}`).item.checked);
  });
  chooseItem(app.pid, at("panel-netlist"));
  await until("Hide enabled again", () => checked("panel-netlist") && itemEnabled(app.pid, at("panel-hide")));
  await newWindowSmoke(app, page, at);
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

  // the pointer at rest on an instance shows its name beside it; View > Show Names on Hover, on to
  // begin with, takes it away, and brings it back
  const tipText = () => page.evaluate(`(() => { const t = document.getElementById("hover-tip"); return t.hidden ? null : t.textContent; })()`);
  await hover(page, await symbol(page, "X3"));
  await until("X3's name beside the pointer", async () => (await tipText()) === "X3", 10_000);
  chooseItem(app.pid, at("toggle-hover-names"));
  await until("Show Names on Hover's check mark to clear", () => !checked("toggle-hover-names"), 10_000);
  await until("the name to go", async () => (await tipText()) === null, 10_000);
  const x2 = await symbol(page, "X2");
  await hover(page, x2);
  await hover(page, { x: x2.x + 3, y: x2.y + 3 });
  await new Promise((resolve) => setTimeout(resolve, 300));
  same(await tipText(), null, "the name beside the pointer with Show Names on Hover off");
  chooseItem(app.pid, at("toggle-hover-names"));
  await until("Show Names on Hover's check mark", () => checked("toggle-hover-names"), 10_000);
  await hover(page, x2);
  await until("X2's name beside the pointer", async () => (await tipText()) === "X2", 10_000);
  console.log("the pointer at rest showed X3's and X2's names, and View > Show Names on Hover took them away and brought them back");

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
  chooseItem(app.pid, at("check-updates"));
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

  // Solve > Solve: the app solves the netlist natively, on a thread of its own, and the page draws
  // it as a preview, with the report; the file on disk is as it was; Close Preview reads it again
  const onDisk = fs.readFileSync(netlist);
  const includeOnDisk = fs.readFileSync(netlistInclude);
  chooseItem(app.pid, at("solve"));
  await until("the solve's preview", () =>
    page.evaluate(`document.body.classList.contains("preview") && !document.body.classList.contains("solving")`),
  );
  const solved = await page.evaluate(`({
    record: window.viewer.doc.files[0].text.includes('@RNL(type="generated"'),
    levels: document.querySelectorAll("#solve-report .solve-level").length,
    badge: document.getElementById("preview-badge").hidden === false,
  })`);
  same(solved.record && solved.badge && solved.levels > 0, true, "the solve drawn as a preview, with its report");
  same(fs.readFileSync(netlist).equals(onDisk), true, "the netlist on disk after the solve");
  chooseItem(app.pid, at("solve-close"));
  await until("the netlist as written", () =>
    page.evaluate(`!document.body.classList.contains("preview") && !document.body.classList.contains("busy")`),
  );
  console.log("Solve > Solve drew the netlist solved as a preview, and the file on disk is as it was");

  // Solve > Settings: the dialog shows the solver's own defaults, which the app gives it, and what
  // the solver does not take is refused in its words (the app checks the options with the solver);
  // Cancel keeps nothing
  chooseItem(app.pid, at("solve-settings"));
  const beam = await until("the settings dialog with the solver's beam", () =>
    page.evaluate(`(() => {
      const label = [...document.querySelectorAll("dialog.dialog .settings-label")].find((l) => l.textContent.startsWith("Beam"));
      return label ? label.querySelector("input").value : null;
    })()`),
  );
  same(/^[1-9][0-9]*$/.test(beam), true, "the beam of the solver's defaults in the dialog");
  await page.evaluate(`(() => {
    const label = [...document.querySelectorAll("dialog.dialog .settings-label")].find((l) => l.textContent.startsWith("Beam"));
    const input = label.querySelector("input");
    input.value = "0";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    [...document.querySelectorAll("dialog.dialog button")].find((b) => b.textContent === "OK").click();
  })()`);
  const refused = await until("the solver's refusal in the dialog", () =>
    page.evaluate(`(() => {
      const error = document.querySelector("dialog.dialog .dialog-error");
      return error && !error.hidden ? error.textContent : null;
    })()`),
  );
  same(refused, "the beam must be at least 1", "what the solver says of a beam of 0");
  await page.evaluate(`[...document.querySelectorAll("dialog.dialog button")].find((b) => b.textContent === "Cancel").click()`);
  await until("the dialog to close", async () => !(await page.evaluate(`!!document.querySelector("dialog.dialog")`)), 10_000);
  same(await page.evaluate(`localStorage.getItem("solve-options")`), null, "the settings kept after Cancel");
  console.log("Solve > Settings showed the solver's defaults, and the solver refused a beam of 0 in its own words");
  // Edit > Edit Text, Apply Changes and File > Save (decision 0011), on a copy of the netlist: the
  // text boxes appear and the menu's check mark follows; a change is not applied until Apply Changes
  // builds the netlist natively from the text in memory, over the files, and the file on disk is as
  // it was; a window asked to close with the change not saved stays, and the page asks; Save writes
  // the file, in the line ends it had, and nothing else
  chooseItem(app.pid, at("toggle-edit"));
  await until("the text boxes of the two files", () => page.evaluate(`document.querySelectorAll("#source .editor").length === 2`));
  await until("Edit Text's check mark", () => checked("toggle-edit"), 10_000);
  const edit = `(() => {
    const area = document.querySelector("#source .editor:not([hidden]) textarea");
    area.value = area.value.replace(/\\bX3\\b/g, "X9");
    area.dispatchEvent(new Event("input", { bubbles: true }));
  })()`;
  await page.evaluate(edit);
  const named = (name) => page.evaluate(`window.viewer.doc.objects.some((o) => o.kind === "instance" && o.name === ${JSON.stringify(name)})`);
  same(await named("X3"), true, "the instance before the change is applied");
  chooseItem(app.pid, at("apply-edits"));
  await until("the netlist built from the text held", async () => (await named("X9")) && !(await named("X3")), 20_000);
  same(fs.readFileSync(netlist).equals(onDisk), true, "the netlist on disk after the change was applied");
  // closing the window asks, as the text has changes not saved: WM_CLOSE, as taskkill sends it
  execFileSync("taskkill", ["/PID", String(app.pid)], { stdio: "ignore" });
  const asked = await until("the question of the window that is asked to close", () =>
    page.evaluate(`(() => {
      const d = document.querySelector("dialog.dialog .dialog-title");
      return d && d.textContent;
    })()`),
  );
  same(asked, "Changes Not Saved", "the dialog of the window asked to close");
  await page.evaluate(`[...document.querySelectorAll("dialog.dialog button")].find((b) => b.textContent === "Cancel").click()`);
  await until("the dialog to close", async () => !(await page.evaluate(`!!document.querySelector("dialog.dialog")`)), 10_000);
  same(app.exitCode === null && app.signalCode === null, true, "the app still running after the window was asked to close");
  // Save writes the file, as the text box holds it, with the line ends it had
  chooseItem(app.pid, at("save"));
  const saved = onDisk.toString("utf8").replace(/\bX3\b/g, "X9");
  await until("the file saved", () => fs.readFileSync(netlist, "utf8") === saved, 20_000);
  await until("no file with changes not saved", () =>
    page.evaluate(`document.querySelectorAll("#netlist-file[data-unsaved='true']").length === 0`),
  );
  same(fs.readFileSync(netlistInclude).equals(includeOnDisk), true, "the include after the netlist was saved");
  // Saving before applying is different from downloading on the web: the disk source really
  // changes. Verify the same newly typed USE-PACKAGE sequence against the native reader/cache.
  const withPackage = saved.replace('@VERSION("0.2")', '@VERSION("0.2"); @USE-PACKAGE(name="rnl-std", version="0.2")');
  const setEditor = text => page.evaluate(`(() => {
    const area = document.querySelector('#source .editor:not([hidden]) textarea');
    area.value = ${JSON.stringify(text.replace(/\r\n/g,'\n'))};
    area.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
  chooseItem(app.pid, at("toggle-edit"));
  await until("the editor before Save then Apply", () => page.evaluate(`!!document.querySelector('#source textarea')`));
  await setEditor(withPackage);
  chooseItem(app.pid, at("save"));
  await until("the unapplied package saved to disk", () => fs.readFileSync(netlist,'utf8') === withPackage);
  chooseItem(app.pid, at("apply-edits"));
  await until("the saved package applied natively", () => page.evaluate(`!document.querySelector('#source textarea') &&
    window.viewer.doc.files[0].text.replace(/\\r\\n/g,'\\n') === ${JSON.stringify(withPackage.replace(/\r\n/g,'\n'))}`));
  chooseItem(app.pid, at("toggle-edit"));
  await until("the saved package retained on re-entering editing", () => page.evaluate(`document.querySelector('#source textarea')?.value === ${JSON.stringify(withPackage.replace(/\r\n/g,'\n'))}`));
  await setEditor(saved);
  chooseItem(app.pid, at("save"));
  await until("the original text restored on disk", () => fs.readFileSync(netlist,'utf8') === saved);
  chooseItem(app.pid, at("apply-edits"));
  await until("the original text applied again", () => page.evaluate(`!document.querySelector('#source textarea') &&
    window.viewer.doc.files[0].text.replace(/\\r\\n/g,'\\n') === ${JSON.stringify(saved.replace(/\r\n/g,'\n'))}`));
  console.log("Save before Apply retained a newly typed USE-PACKAGE with the real native disk reader");
  // another program changes the file after the viewer read it: Save does not write over it, and asks;
  // Overwrite writes the text of the viewer
  chooseItem(app.pid, at("toggle-edit"));
  await until("editing again", () => page.evaluate(`!!document.querySelector("#source textarea")`));
  const eol = saved.includes("\r\n") ? "\r\n" : "\n";
  fs.appendFileSync(netlist, `* changed by another program${eol}`);
  await page.evaluate(`(() => {
    const area = document.querySelector("#source .editor:not([hidden]) textarea");
    area.value += "* and in the viewer\\n";
    area.dispatchEvent(new Event("input", { bubbles: true }));
  })()`);
  chooseItem(app.pid, at("save"));
  const changedOnDisk = await until("the question about the file changed on disk", () =>
    page.evaluate(`(() => {
      const d = document.querySelector("dialog.dialog .dialog-title");
      return d && d.textContent;
    })()`),
  );
  same(changedOnDisk, "File Changed on Disk", "the dialog of the file changed by another program");
  same(fs.readFileSync(netlist, "utf8"), `${saved}* changed by another program${eol}`, "the file on disk while the question is asked");
  await page.evaluate(`[...document.querySelectorAll("dialog.dialog button")].find((b) => b.textContent === "Overwrite").click()`);
  await until("the file overwritten", () => fs.readFileSync(netlist, "utf8") === `${saved}* and in the viewer${eol}`, 20_000);
  // what was saved is not applied yet, and leaving the text boxes with a line not applied asks about it
  chooseItem(app.pid, at("apply-edits"));
  await until("applied and exited", () => page.evaluate(`document.querySelectorAll("#source .editor").length === 0`), 20_000);
  await until("the text read again", () => page.evaluate(`document.querySelectorAll("#source .editor").length === 0`));
  console.log("Edit > Edit Text, Apply Changes and File > Save built the edited text natively and saved the one file, as it was written");

  // Disable Auto Include to check normal clipboard include resolution independently of Library.
  await page.evaluate(`import('./pool.js').then(p => p.toggleAutoInclude(false))`);
  // File > Load from Clipboard: the page reads the clipboard, whose text the test gives in the page,
  // shows it, and on Load the app draws it natively as clipboard.sp, alone, its include missing
  const text = fs.readFileSync(netlist, "utf8");
  await page.evaluate(`navigator.clipboard.readText = async () => ${JSON.stringify(text)}`);
  chooseItem(app.pid, at("load-clipboard"));
  const preview = await until("the clipboard's text in its preview", () =>
    page.evaluate(`(() => {
      const box = document.querySelector("dialog.dialog .clip-preview");
      return box && box.textContent;
    })()`),
  );
  same(preview, text.replace(/\r?\n$/, "").replace(/\r\n/g, "\n"), "the clipboard's text in its preview");
  await page.evaluate(`[...document.querySelectorAll("dialog.dialog button")].find((b) => b.textContent === "Load").click()`);
  await until("the clipboard's netlist", async () => {
    const s = await shown(page);
    return s.level === "clipboard.sp" && s.drawn && !s.busy;
  });
  await until("the window's title", () => windowTitle(app.pid) === "clipboard.sp - RNL Viewer", 10_000);
  const pasted = await page.evaluate(`({ files: window.viewer.doc.files.length, missing: window.viewer.doc.missing })`);
  same(pasted, { files: 1, missing: ["/04-hierarchy-lib.inc"] }, "the clipboard's netlist, alone");
  console.log("File > Load from Clipboard drew the netlist that the clipboard held, alone");
  await page.evaluate(`import('./pool.js').then(p => p.toggleAutoInclude(true))`);
  check(await page.evaluate(`window.viewer.doc.files.length === 2 && window.viewer.doc.files.some(f => f.name.endsWith('04-hierarchy-lib.inc')) &&
    window.viewer.doc.diagnostics.some(d => d.severity === 'warning' && d.message.includes('added by Auto Include'))`),
  "clipboard Auto Include did not resolve the library subcircuit");

  // a netlist with nothing placed: its Document carries no drawing of the objects that have no
  // position, the app keeps them from the build, and the Not drawn list asks for those of its rows
  const bare = "* none placed\nR1 a b 1k\nC1 b 0 1p\n.end\n";
  await page.evaluate(`navigator.clipboard.readText = async () => ${JSON.stringify(bare)}`);
  chooseItem(app.pid, at("load-clipboard"));
  await until("the second clipboard text in its preview", () =>
    page.evaluate(`(() => {
      const box = document.querySelector("dialog.dialog .clip-preview");
      return box !== null && box.textContent.startsWith("* none placed");
    })()`),
  );
  await page.evaluate(`[...document.querySelectorAll("dialog.dialog button")].find((b) => b.textContent === "Load").click()`);
  await until("the netlist with nothing placed", () =>
    page.evaluate(`window.viewer.doc.files[0].text.startsWith("* none placed") && !document.body.classList.contains("busy")`),
  );
  same(await page.evaluate(`window.viewer.doc.levels[0].unplaced.some((u) => u.svg || u.primitives)`), false, "drawings in the Document");
  await page.evaluate(`document.getElementById("tab-unplaced").click()`);
  const thumbs = await until("the drawings of the objects not drawn", () =>
    page.evaluate(`(() => {
      const rows = [...document.querySelectorAll("#unplaced-list .unplaced-row")];
      return rows.length >= 2 && rows.every((r) => r.querySelector(".thumb svg")) ? rows.length : 0;
    })()`),
  );
  console.log(`The Not drawn list asked the app for the drawings of its ${thumbs} rows, which the Document did not carry`);

  await filePoolSmoke(page);
  await nativeDropSmoke(page, app.pid);
  await independentNavigationSmoke(page);
  // last: it opens a file and a dialog of its own
  if (expectPackages) await publicPackagesSmoke(page, expectPackages);
  same(page.errors, [], "the page's errors");
}

/** Exercise the actual native event bridge and backend, preserving the normal opening flow. */
async function nativeDropSmoke(page, pid) {
  const dropped = path.join(path.dirname(netlist), "拖入 电路.NETLIST");
  fs.copyFileSync(netlist, dropped);
  const emit = (type, paths = []) => page.evaluate(`window.__TAURI__.event.emitTo({ kind: 'Webview', label: 'main' }, 'tauri://drag-${type}', ${JSON.stringify({ paths, position: { x: 100, y: 100 } })})`);
  await emit("enter", [dropped]);
  await until("native drag feedback", () => page.evaluate(`document.body.classList.contains('dropping')`));
  await emit("leave");
  await until("native drag feedback to clear", () => page.evaluate(`!document.body.classList.contains('dropping')`));
  await emit("drop", [dropped]);
  await until("the dropped netlist", () => page.evaluate(`window.viewer.doc?.name === ${JSON.stringify(path.basename(dropped))} && !document.body.classList.contains('busy')`));
  await until("the dropped Unicode filename in the title", () => windowTitle(pid) === `${path.basename(dropped)} - RNL Viewer`);
  await emit("drop", [netlist]);
  await until("the original netlist after a second drop", () => page.evaluate(`window.viewer.doc?.name === ${JSON.stringify(TOP)} && !document.body.classList.contains('busy')`));
  console.log("native drop events opened a Unicode/space .NETLIST path, preserved includes and cleared drag feedback");
}

/** File > New Window launches this executable again, restores the session and leaves its parent. */
async function newWindowSmoke(app, page, at) {
  const port = await freePort();
  const before = await page.evaluate(`({ path: window.viewer.path, level: window.viewer.level, selected: window.viewer.selected })`);
  await page.evaluate(`(async () => {
    const shell = (await import('./shells/desktop.js')).default;
    const original = shell.newWindow;
    shell.newWindow = async (session, doc) => {
      session.testPort = ${port};
      const pid = await original(session, doc);
      window.spawnedViewer = pid;
      return pid;
    };
  })()`);
  let child = null;
  let pid = null;
  try {
    chooseItem(app.pid, at("new-window"));
    pid = await until("the second viewer process", () => page.evaluate("window.spawnedViewer || null"));
    check(pid !== app.pid, "New Window reused the original process");
    const childInfo = JSON.parse(powershell(`Get-CimInstance Win32_Process -Filter "ProcessId = ${pid}" | Select-Object ParentProcessId,CommandLine | ConvertTo-Json -Compress`));
    same(childInfo.ParentProcessId, app.pid, "the new process's parent");
    check(childInfo.CommandLine.includes("--session"), "the child has no session file argument");
    child = await connect(port);
    await until("the child document and level", async () => {
      const shown = await child.evaluate(`window.viewer?.doc && !document.body.classList.contains('busy') && ({ path: window.viewer.path, level: window.viewer.level, selected: window.viewer.selected })`);
      return shown && JSON.stringify(shown) === JSON.stringify(before);
    });
    same(await child.evaluate(`document.querySelectorAll('#pool-list .pool-row').length`), 1, "the child's Library");
    same(await page.evaluate(`({ path: window.viewer.path, level: window.viewer.level, selected: window.viewer.selected })`), before, "the parent after New Window");
    // Both processes start from the same preferences, then save distinct keys without losing either.
    await page.evaluate(`(async () => { const u = await import('./util.js'); await u.save('background', 'dark'); })()`);
    await child.evaluate(`(async () => { const u = await import('./util.js'); await u.save('grid-major', 7); })()`);
    const settings = await child.evaluate(`(async () => (await import('./util.js')).settingsPath)()`);
    const values = JSON.parse(fs.readFileSync(settings, "utf8"));
    same([values.background, values["grid-major"]], ["dark", 7], "independent settings updates");
    if (process.env.SMOKE_OUTPUT_DIR) {
      fs.mkdirSync(process.env.SMOKE_OUTPUT_DIR, { recursive: true });
      for (const [name, target] of [["desktop-parent.png", page], ["desktop-new-window.png", child]]) {
        const screenshot = await target.send("Page.captureScreenshot", { format: "png" });
        fs.writeFileSync(path.join(process.env.SMOKE_OUTPUT_DIR, name), Buffer.from(screenshot.data, "base64"));
      }
      fs.writeFileSync(path.join(process.env.SMOKE_OUTPUT_DIR, "desktop-two-processes.json"), JSON.stringify({ parent: app.pid, child: pid, session: before, process: childInfo }, null, 2));
    }
    console.log(`File > New Window started independent process ${pid} from ${app.pid}, restored its level/selection/Library and preserved concurrent settings`);
  } finally {
    child?.ws.close();
    if (pid) {
      try { execFileSync("taskkill", ["/PID", String(pid), "/T", "/F"], { stdio: "ignore" }); } catch { /* already closed */ }
    }
    await page.evaluate(`(async () => { const shell = (await import('./shells/desktop.js')).default; delete window.spawnedViewer; })()`);
  }
}

/** Exercise the real desktop shell and native core; replace only the OS chooser's selection. */
/**
 * The public packages: the app that bundles them finds eda-style, a file that uses it, without a network,
 * and shows them in the Library and in Help > Public Packages; the app that does not reports the package
 * missing with a hint to Help > Public Packages, and bundles nothing. Neither downloads anything.
 */
async function publicPackagesSmoke(page, mode) {
  const work = path.dirname(netlist);
  const top = path.join(work, "use-eda-style.sp");
  fs.writeFileSync(top, ["* a file that uses a public package", '* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="eda-style", version="0.1") }', "R1 a 0 1k", ".end", ""].join("\n"));
  const info = await page.evaluate(`(async () => {
    const { shell } = await import('./shell.js');
    window.fetched = [];
    const original = window.fetch;
    window.fetch = (...args) => { window.fetched.push(String(args[0]?.url ?? args[0])); return original.apply(window, args); };
    return await shell.bundledPackages();
  })()`);
  await page.evaluate(`(async () => {
    const { openSource } = await import('./open.js');
    await openSource({ path: ${JSON.stringify(top)}, name: 'use-eda-style.sp' }, false);
    const head = document.getElementById('problems-head');
    if (head.getAttribute('aria-expanded') === 'false') head.click();
  })()`);
  const missing = await page.evaluate(`window.viewer.doc.diagnostics.some(d => d.message.includes('eda-style') && d.message.includes('cannot be found'))`);
  await page.evaluate("document.getElementById('tab-files').click()");
  const group = await page.evaluate(`(() => {
    const g = document.querySelector('[data-group="public"]');
    return g ? [...g.querySelectorAll('.pool-open')].map((b) => b.textContent) : null;
  })()`);
  await page.evaluate("window.viewer.command('public-packages')");
  const dialog = await until("the Help > Public Packages dialog", () => page.evaluate(`document.querySelector('dialog.dialog[open], dialog.dialog')?.innerText || ''`));
  await page.evaluate("document.querySelector('dialog.dialog button')?.click()");
  if (mode === "with") {
    check(info.bundled, `the build with the public packages bundles none: ${JSON.stringify(info)}`);
    const names = info.packages.map((p) => `${p.name} ${p.version}`);
    check(names.includes("eda-style 0.1"), `the bundled packages are ${names.join(", ")}`);
    check(!missing, "eda-style 0.1 is bundled, and was reported missing");
    same(group, names, "the Library's group of the bundled packages");
    check(dialog.includes(`This build bundles ${names.join(", ")}.`), `Help > Public Packages: ${dialog}`);
    console.log(`The public packages are bundled and found without a network: ${names.join(", ")}`);
  } else {
    check(!info.bundled && info.packages.length === 0, `the build without the public packages bundles ${JSON.stringify(info)}`);
    check(missing, "eda-style 0.1 is not bundled, and was not reported missing");
    same(group, null, "the Library's group of the bundled packages");
    const hint = await page.evaluate(`document.querySelector('.package-hint .msg')?.textContent`);
    same(hint, "Look for it in Help › Public Packages.", "the hint of the missing package");
    check(dialog.includes("This build does not bundle the public packages."), `Help > Public Packages: ${dialog}`);
    console.log("The public packages are not bundled: the package is reported missing, with a hint to Help > Public Packages");
  }
  same(await page.evaluate("window.fetched.filter((u) => !/^(blob|data):/.test(u) && !u.startsWith(location.origin) && !u.startsWith('http://ipc.localhost'))"), [], "what the app fetched from elsewhere");
}

async function filePoolSmoke(page) {
  const work = path.dirname(netlist);
  const parts = path.join(work, "pool");
  fs.mkdirSync(parts);
  const top = path.join(work, "pool-top.sp");
  const pkg = path.join(parts, "renamed.rnl");
  const inc = path.join(parts, "part.inc");
  fs.writeFileSync(top, '* top\n* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="acme", version="1.0") }\n.include "part.inc"\n.end\n');
  fs.writeFileSync(pkg, '@RNL(type="header") { @VERSION("0.3"); @PACKAGE(name="acme", version="1.0") }\n');
  fs.writeFileSync(inc, "Rpool a 0 1k\n");
  const slashed = (p) => p.replaceAll("\\", "/");
  await page.evaluate(`(async () => {
    const { openSource } = await import('./open.js');
    await openSource({ path: ${JSON.stringify(top)}, name: 'pool-top.sp' }, false);
  })()`);
  check(await page.evaluate(`window.viewer.doc.diagnostics.some(d => d.message.includes('acme 1.0 cannot be found'))`), "Open File unexpectedly added package directories");
  const selected = [pkg, inc].map((p) => ({ path: slashed(p), name: path.basename(p), source: { path: slashed(p), name: path.basename(p) } }));
  await page.evaluate(`(async () => {
    const shell = (await import('./shells/desktop.js')).default;
    shell.include = async () => ${JSON.stringify(selected)};
    await (await import('./pool.js')).includeFiles('folder');
  })()`);
  same(await page.evaluate(`window.viewer.doc.diagnostics.filter(d => d.severity === 'error')`), [], "pool search diagnostics");
  check(await page.evaluate(`window.viewer.doc.files.some(f => f.text.includes('Rpool'))`), "the pooled include was not read");
  const pasted = await page.evaluate(`(async () => {
    const shell = (await import('./shells/desktop.js')).default;
    const doc = await shell.build({ name: 'clipboard.sp', text: '* clipboard\\n.include "part.inc"\\n.end\\n' },
      { [${JSON.stringify(slashed(inc))}]: 'Redited a 0 2k\\n' });
    return doc.files.map(f => f.text);
  })()`);
  check(pasted.some(t => t.startsWith('* clipboard')) && pasted.some(t => t.includes('Redited')),
    "an include edit replaced the clipboard entry");
  await page.evaluate(`document.querySelector('[aria-label="Open renamed.rnl"]').click()`);
  await until("a pool package opened", () => page.evaluate(`window.viewer.doc.name === 'renamed.rnl' && !document.body.classList.contains('busy')`));
  await page.evaluate(`(async () => {
    await (await import('./open.js')).openSource({ path: ${JSON.stringify(top)}, name: 'pool-top.sp' }, false);
    document.getElementById('tab-files').click();
    document.querySelector('[aria-label="Exclude part.inc"]').click();
  })()`);
  // The document is installed before its first frame is painted. Let this reload finish before
  // the synthetic chooser starts another rebuild; a synchronous click can beat that final paint.
  await until("the excluded include missing", () => page.evaluate(`window.viewer.doc.diagnostics.some(d => d.message.includes('part.inc cannot be found')) && !document.body.classList.contains('busy')`));
  await page.evaluate(`(async () => {
    const shell = (await import('./shells/desktop.js')).default;
    shell.pickPath = async () => ${JSON.stringify(slashed(parts))};
    document.getElementById('add-path').click();
  })()`);
  await until("PATH resolving the include", () => page.evaluate(`window.viewer.doc.files.some(f => f.text.includes('Rpool')) && !document.body.classList.contains('busy')`));
  const settings = await page.evaluate(`(async () => {
    const util = await import('./util.js'); await util.flushSettings(); return util.settingsPath;
  })()`);
  same(JSON.parse(fs.readFileSync(settings, "utf8"))["search-paths"], [slashed(parts)], "native settings file PATH");
  await autoIncludeSmoke(page, work, parts, pkg);
  await memoryCacheSmoke(page, work);
  await preloadSmoke(page, work);
  check(await page.evaluate(`Array.from(document.querySelectorAll('[data-group="builtin"] .pool-row')).every(r => !r.textContent.includes('Exclude'))`), "a built-in can be excluded");
  await page.evaluate(`document.querySelector('[data-group="builtin"] .lib-toggle').click()`);
  await page.evaluate(`document.querySelector('[aria-label="Open 0.3/rnl-std.rnl"]').click()`);
  await until("a built-in package opened", () => page.evaluate(`window.viewer.doc.path.includes('<bundled>') && !document.body.classList.contains('busy')`));
  check(await page.evaluate(`(async () => (await import('./open.js')).currentSource().readonly)()`), "built-in package is not read-only");
  await page.send("Page.reload");
  await until("settings after reload", () => page.evaluate(`window.viewer && window.viewer.doc && document.querySelectorAll('.lib-path').length === 1 && !document.body.classList.contains('busy')`));
  same(await page.evaluate(`document.querySelectorAll('#pool-list .pool-row').length`), 1, "only the initially opened file joins the new session");
  console.log("The native file pool resolves packages/includes, opens and excludes files; PATH persists and built-ins are read-only");
  // Exercise the new diagnostic in the real native shell, including its actual built-in catalog.
  const optional = fs.readFileSync(path.join(root, "tests/viewer/inputs/missing-canvas-package.sp"), "utf8");
  await page.evaluate(`(async () => {
    await (await import('./open.js')).openSource({ name: 'canvas.sp', text: ${JSON.stringify(optional)} }, false);
    const head = document.getElementById('problems-head');
    if (head.getAttribute('aria-expanded') === 'false') head.click();
  })()`);
  await until("the native missing-package hint", () => page.evaluate(`
    document.querySelector('.package-hint .msg')?.textContent === 'Look for it in Help › Public Packages.'
      && window.viewer.doc.diagnostics.some(d => d.message.includes('package analog-canvas-extensions 0.1 cannot be found'))`));
  same(await page.evaluate(`document.querySelectorAll('[data-group="builtin"] .pool-open').length`), 25,
    "the native built-in library contains both standard series");
  check(await page.evaluate(`Array.from(document.querySelectorAll('[data-group="builtin"] .pool-open')).every(b => /^0\\.[23]\\/rnl-(std|zh)/.test(b.textContent))`),
    "a nonstandard package remains built in");
  if (process.env.SMOKE_OUTPUT_DIR) {
    fs.mkdirSync(process.env.SMOKE_OUTPUT_DIR, { recursive: true });
    const image = await page.send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(path.join(process.env.SMOKE_OUTPUT_DIR, "desktop-native-missing-canvas-hint.png"), Buffer.from(image.data, "base64"));
  }
  console.log("The native core bundles only standard packages and offers the public-package website on missing USE-PACKAGE");
}

/** File transitions exercise the real disk reader and build cache, not a fixture shell. */
async function independentNavigationSmoke(page) {
  const work = path.dirname(netlist);
  const top = path.join(work, 'visit-top.sp'), cell = path.join(work, 'visit-cell.inc'), leaf = path.join(work, 'visit-leaf.inc');
  fs.writeFileSync(top, '* visits\n.include "visit-cell.inc"\n.include "visit-leaf.inc"\nXcell a 0 visit_cell\n* @RNL { #Xcell @PLACE(0, 0) }\n.end\n');
  fs.writeFileSync(cell, '* cell\n.subckt visit_cell p n\nXleaf p n visit_leaf\n* @RNL { #Xleaf @PLACE(0, 0) }\n.ends\n');
  fs.writeFileSync(leaf, '* leaf\n.subckt visit_leaf p n\nRleaf p n 1k\n* @RNL { #Rleaf @PLACE(0, 0) }\n.ends\n');
  const result = await page.evaluate(`(async () => {
    const {openSource} = await import('./open.js');
    const {descend, ascend, namedPath, backTo} = await import('./subcircuits.js');
    const {toggleAutoInclude, poolSnapshot} = await import('./pool.js');
    await toggleAutoInclude(false);
    await openSource({path:${JSON.stringify(top)},name:'visit-top.sp'},false);
    await descend(window.viewer.doc.objects.findIndex(o => o.name === 'Xcell'));
    const v=window.viewer;
    const isolated={entry:v.doc.path, files:v.doc.files.length, opens:v.doc.objects.find(o => o.name === 'Xleaf').opens,
      path:namedPath().map(s=>s.title), listed:poolSnapshot().some(i=>i.path.endsWith('/visit-cell.inc'))};
    await toggleAutoInclude(true);
    const resolved=v.doc.objects.find(o=>o.name==='Xleaf').opens !== null;
    await descend(v.doc.objects.findIndex(o=>o.name==='Xleaf'));
    const deep=namedPath().map(s=>s.title);
    await backTo(0);
    const back={path:namedPath().map(s=>s.title),selected:v.doc.objects[v.selected]?.name};
    await descend(v.doc.objects.findIndex(o=>o.name==='Xcell'));
    await ascend();
    return {isolated,resolved,deep,back,entry:v.doc.path};
  })()`);
  same(result.isolated.files,1,'child independently reads one disk file');
  same(result.isolated.opens,null,'parent include definitions are absent with Auto Include off');
  check(result.isolated.entry.replaceAll('\\','/').endsWith('/visit-cell.inc'),'child did not become the entry');
  same(result.isolated.listed,true,'visited disk file joins Files');
  same(result.isolated.path,['visit-top.sp','visit_cell'],'cross-file descent history');
  same(result.resolved,true,'Auto Include resolves the leaf using the Library index');
  same(result.deep,['visit-top.sp','visit_cell','visit_leaf'],'deep cross-file history');
  same(result.back,{path:['visit-top.sp'],selected:'Xcell'},'cross-level breadcrumb jump restores prefix and selection');
  check(result.entry.replaceAll('\\','/').endsWith('/visit-top.sp'),'ascend did not reopen the top entry');
  console.log('Cross-file disk navigation isolates includes, adds Library files, and restores visit history');
}

async function memoryCacheSmoke(page, work) {
  const file = path.join(work, 'cache-a.sp');
  fs.writeFileSync(file, '* a\nRcache a 0 1k\n.end\n');
  const first = await page.evaluate(`(async () => {
    const { shell } = await import('./shell.js');
    await shell.memoryCache(true);
    const before = await shell.memoryCache();
    const a = { path: ${JSON.stringify(file)}, name: 'cache-a.sp' };
    const b = { name: 'cache-b.sp', text: '* b\\nCcache b 0 1p\\n.end\\n' };
    const cold = await shell.build(a);
    await shell.build(b);
    const warm = await shell.build(a);
    const stats = await shell.memoryCache();
    if (JSON.stringify(cold) !== JSON.stringify(warm)) throw Error('cache changed the document');
    const drawings = await shell.unplacedDrawings(0, [0], ['Rcache']);
    return { before, stats, hasSvg: !!drawings[0].svg };
  })()`);
  same(first.stats.hits - first.before.hits, 1, 'native cached switch hit');
  same(first.stats.misses - first.before.misses, 2, 'native cold builds');
  check(first.hasSvg && first.stats.used <= first.stats.limit, 'cached drawings or budget are invalid');
  fs.writeFileSync(file, '* a\nRcache a 0 9k\n.end\n');
  const changed = await page.evaluate(`(async () => {
    const { shell } = await import('./shell.js');
    const a = { path: ${JSON.stringify(file)}, name: 'cache-a.sp' };
    const doc = await shell.build(a);
    const changed = await shell.memoryCache();
    await shell.build(a, undefined, undefined, undefined, undefined, { fresh: true });
    const fresh = await shell.memoryCache();
    await shell.memoryCache(true);
    const after = await shell.memoryCache();
    const drawings = await shell.unplacedDrawings(0, [0], ['Rcache']);
    const { save } = await import('./util.js');
    await save('render-cache-mib', 0);
    await shell.build(a);
    const disabled = await shell.memoryCache();
    await save('render-cache-mib', 256);
    await shell.memoryCache();
    return { text: doc.files[0].text, changed, fresh, after, disabled, hasSvg: !!drawings[0].svg };
  })()`);
  check(changed.text.includes('9k'), 'same-size source edit reused old text');
  same(changed.changed.misses, first.stats.misses + 1, 'source edit did not invalidate the cache');
  same(changed.fresh.misses, changed.changed.misses + 1, 'forced reload reused the cache');
  same(changed.after.entries, 0, 'Clear Cache kept entries');
  check(changed.hasSvg, 'Clear Cache lost current unplaced drawings');
  same(changed.disabled.used, 0, 'disabled cache retained a result');
  console.log('Native cache reuses JSON/SVG, restores lazy drawings, detects same-size edits, forces reload and respects clear/disable');
}

/** The native preloader never activates a document, inherits parent includes, or promotes Files. */
async function preloadSmoke(page, work) {
  const child = path.join(work, 'preload-cell.inc'), leaf = path.join(work, 'preload-leaf.inc');
  fs.writeFileSync(child, '* cell\n.subckt preload_cell p n\nXleaf p n preload_leaf\n.ends\n');
  fs.writeFileSync(leaf, '* leaf\n.subckt preload_leaf p n\nRleaf p n 1k\n.ends\n');
  const result = await page.evaluate(`(async () => {
    const {shell} = await import('./shell.js');
    const {save} = await import('./util.js');
    const {poolSnapshot} = await import('./pool.js');
    const shown = window.viewer.doc;
    const listed = poolSnapshot().map(i=>i.path);
    await save('background-preload', true);
    await save('auto-include', false);
    await shell.memoryCache(true);
    await shell.setPool([{path:${JSON.stringify(leaf)}, source:{path:${JSON.stringify(leaf)}}}]);
    await shell.build({name:'active.sp',text:'* active\\nRactive a 0 1k\\n.end\\n'});
    const before = await shell.memoryCache();
    const source = {path:${JSON.stringify(child)}, name:'preload-cell.inc'};
    const ticket = await window.__TAURI__.core.invoke('preload_ticket');
    await shell.cancelPreload();
    const late = await window.__TAURI__.core.invoke('preload_document', {
      path:source.path, pool:[], autoInclude:false, ticket });
    let status;
    for(let i=0;i<30;i++) {
      status = await shell.preload(source);
      if(status !== 'busy' && status !== 'cancelled') break;
      await new Promise(r=>setTimeout(r,100));
    }
    const prepared = await shell.memoryCache();
    const drawn = await shell.unplacedDrawings(0,[0],['Ractive']);
    const isolated = await shell.build(source);
    const after = await shell.memoryCache();
    const noLeaf = isolated.objects.find(o=>o.name==='Xleaf').opens === null;
    await save('auto-include', true);
    const automatic = await shell.preload(source);
    const resolved = await shell.build(source);
    await save('background-preload', false);
    const disabled = await shell.preload(source);
    await save('background-preload', true);
    return {status,automatic,disabled,late,noLeaf,resolved:resolved.objects.find(o=>o.name==='Xleaf').opens !== null,
      retained:prepared.prefetched, hit:after.hits-before.hits, misses:after.misses-before.misses,
      hasSvg:!!drawn[0].svg, sameView:shown===window.viewer.doc,
      sameLibrary:JSON.stringify(listed)===JSON.stringify(poolSnapshot().map(i=>i.path))};
  })()`);
  same(result.status,'ready','native preload completes');
  same(result.late,'cancelled','a late asynchronous IPC cannot adopt the post-cancel generation');
  same(result.retained,1,'native cache retains a speculative document');
  same(result.hit,1,'first independent open uses the preloaded document');
  same(result.misses,0,'preloaded open did not compile again');
  check(result.hasSvg && result.sameView && result.sameLibrary,'preloading changed active drawings, view or Files');
  check(result.noLeaf && result.resolved,'preloading did not respect independent Auto Include scopes');
  same(result.automatic,'ready','Auto Include uses its own cache key');
  same(result.disabled,'disabled','preload preference is respected');
  console.log('Background native compile/SVG reuses its first open, preserves active drawings/Files and isolates Auto Include scopes');
}

async function autoIncludeSmoke(page, work, parts, pkg) {
  const top = path.join(work, "auto-top.sp");
  const cell = path.join(parts, "cell.NETLIST");
  const duplicate = path.join(parts, "duplicate.inc");
  const body = '* auto top\n* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="acme", version="1.0") }\nX1 a 0 cell\n.end\n';
  fs.writeFileSync(top, body);
  fs.writeFileSync(cell, ".subckt cell p n\nR1 p n 1k\n.ends\n");
  await page.evaluate(`(async () => {
    await (await import('./open.js')).openSource({ path: ${JSON.stringify(top)}, name: 'auto-top.sp' }, false);
    await window.viewer.command('reload-library');
  })()`);
  check(await page.evaluate(`Array.from(document.querySelectorAll('[data-group^="path:"] .pool-row')).some(r => r.dataset.path.endsWith('cell.NETLIST'))`), "PATH did not list netlists alongside packages");
  same(await page.evaluate("window.viewer.doc.files.length"), 2, "Auto Include defaults on");
  check(await page.evaluate(`window.viewer.doc.diagnostics.some(d => d.severity === 'warning' && d.message.includes('cell.NETLIST') && d.message.includes('added by Auto Include'))`), "the automatic include warning is missing");
  same(await page.evaluate("window.viewer.doc.files[0].text"), body, "Auto Include altered the source");
  check(await page.evaluate("document.getElementById('auto-include').checked && window.viewer.checked('toggle-auto-include')"), "Auto Include controls disagree");
  const solved = await page.evaluate(`(async () => {
    const shell = (await import('./shells/desktop.js')).default;
    const result = await shell.solve({ path: ${JSON.stringify(top)}, name: 'auto-top.sp' }, {});
    return { files: result.document.files.map(f => f.name), levels: result.report.levels.map(l => l.title) };
  })()`);
  check(solved.levels.some(name => name === 'cell'), "Solve did not inherit Auto Include");
  fs.writeFileSync(duplicate, ".subckt cell p n\n.ends\n");
  await page.evaluate("window.viewer.command('reload-library')");
  check(await page.evaluate(`window.viewer.doc.diagnostics.some(d => d.message.includes('conflicting definitions'))`), "the reloaded collision was ignored");
  same(await page.evaluate("window.viewer.doc.files.length"), 1, "an ambiguous definition was imported");
  fs.unlinkSync(duplicate);
  fs.writeFileSync(cell, ".subckt replacement p n\nR2 p n 2k\n.ends\n");
  fs.writeFileSync(top, body.replace("X1 a 0 cell", "X1 a 0 replacement"));
  await page.evaluate("window.viewer.command('reload-library')");
  check(await page.evaluate(`window.viewer.doc.diagnostics.some(d => d.message.includes('replacement') && d.message.includes('added by Auto Include'))`), "Reload Library retained stale definitions");
  fs.writeFileSync(pkg, fs.readFileSync(pkg, "utf8").replace('version="1.0"', 'version="2.0"'));
  await page.evaluate("window.viewer.command('reload-library')");
  check(await page.evaluate(`window.viewer.doc.diagnostics.some(d => d.message.includes('acme 1.0 cannot be found'))`), "Reload Library did not reread packages");
  await page.evaluate("window.viewer.command('toggle-auto-include')");
  same(await page.evaluate("window.viewer.doc.files.length"), 1, "disabling Auto Include kept the library file");
  console.log("PATH lists netlists; Auto Include imports unique definitions without changing text, reports collisions, and Reload Library refreshes netlists/packages and rebuilds the index");
}

/**
 * What there is of the app's WebView, for when its DevTools port does not answer: whether the app's
 * window answers, the WebView2 browser processes that the app started, with their arguments, and
 * what is in the profile.
 */
function webViewState(pid, profile) {
  const lines = [];
  try {
    lines.push(
      powershell(`$app = Get-Process -Id ${pid}
"the app's window: '$($app.MainWindowTitle)', responding: $($app.Responding)"
$all = @(Get-CimInstance Win32_Process -Filter "Name = 'msedgewebview2.exe'")
"msedgewebview2.exe processes: $($all.Count)"
$all | Where-Object { $_.ParentProcessId -eq ${pid} } | ForEach-Object { "the app's: $($_.CommandLine)" }`),
    );
  } catch (e) {
    lines.push(`The app's processes could not be listed: ${e.message}`);
  }
  let files = [];
  try {
    files = fs.readdirSync(profile, { recursive: true });
  } catch {
    // it is gone, or was never made
  }
  lines.push(`the profile ${profile}: ${files.length} entries${files.length ? `, such as ${files.slice(0, 12).join(", ")}` : ""}`);
  return lines.join("\n");
}

async function main() {
  if (process.env.SMOKE_OUTPUT_DIR) fs.mkdirSync(process.env.SMOKE_OUTPUT_DIR, { recursive: true });
  const given = option("--exe");
  if (!given && !process.argv.includes("--no-build")) {
    execFileSync("cargo", ["build", "--manifest-path", manifest], { cwd: root, stdio: "inherit" });
  }
  const exe = given ? path.resolve(given) : appPath();
  check(fs.existsSync(exe), `There is no ${exe} to start${given ? "" : ": build the app, or leave out --no-build"}.`);
  const pe = fs.readFileSync(exe);
  same(pe.readUInt16LE(pe.readUInt32LE(0x3c) + 24 + 68), 2, "Viewer uses the Windows GUI subsystem, without a console");
  console.log(`starting ${exe}, on the WebView2 Runtime ${runtimeVersion() || "that the registry does not name"}`);
  const port = Number(option("--port")) || (await freePort());
  const profile = fs.mkdtempSync(path.join(os.tmpdir(), "rnl-viewer-smoke-"));
  const work = fs.mkdtempSync(path.join(os.tmpdir(), "rnl-viewer-smoke-files-"));
  for (const name of ["04-hierarchy.sp", "04-hierarchy-lib.inc"]) fs.copyFileSync(path.join(cases, name), path.join(work, name));
  netlist = path.join(work, "04-hierarchy.sp");
  netlistInclude = path.join(work, "04-hierarchy-lib.inc");
  const putSettingsBack = setSettingsAside();
  const t0 = Date.now();
  // the app gives the port to WebView2 itself: WebView2 ignores WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
  // where the app runs elevated, as on GitHub's runners, but takes the profile from the environment
  const app = spawn(exe, [`--remote-debugging-port=${port}`, netlist], {
    cwd: root,
    env: { ...process.env, WEBVIEW2_USER_DATA_FOLDER: profile, RNL_VIEWER_TEST_CONFIG_DIR: path.join(profile, "settings") },
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
  } catch (e) {
    // with no page to drive, say what there is of the WebView, while the app runs
    if (!session.page && app.exitCode === null && app.signalCode === null) console.error(webViewState(app.pid, profile));
    throw e;
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
    putSettingsBack();
    fs.rmSync(work, { recursive: true, force: true });
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
