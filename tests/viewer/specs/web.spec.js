// Smoke tests of the web viewer (apps/web): the site that build.mjs makes with the samples, served by
// serve.mjs, with the web shell and the real core as WebAssembly in its Worker. It opens netlists by
// URL, files set on its file inputs, a folder and dropped files, and each Document is the one
// `rnl view` builds for the same files but for the paths that name them in the workspace.

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import { expect, test } from "@playwright/test";
import { PICK, root, UNCALLED } from "../fixtures.js";

test("New Window hands local files, applied include text, hierarchy and Library to a new tab", async ({ page }, info) => {
  await start(page);
  const top = '* local top\n.include "part.inc"\nX1 a 0 part\n* @RNL { #X1 @PLACE(0, 0) }\n.end\n';
  const part = '.subckt part p q\nR1 p q 1k\n.ends\n* @RNL(scope="part") { #R1 @PLACE(0, 0) }\n';
  await page.locator("#file-input").setInputFiles({ name: "top.sp", mimeType: "text/plain", buffer: Buffer.from(top) });
  await shown(page);
  await opened(page, () => page.locator("#include-input").setInputFiles({ name: "part.inc", mimeType: "text/plain", buffer: Buffer.from(part) }));
  await page.locator("#tab-netlist").click();
  await page.keyboard.press("Control+q");
  await page.locator("#file-tabs button", { hasText: "part.inc" }).click();
  await page.locator("#source textarea:visible").fill(part.replaceAll("R1", "R7"));
  await opened(page, () => page.keyboard.press("Control+Enter"));
  await page.evaluate(async () => {
    const { select } = await import('./selection.js');
    select(window.viewer.doc.objects.findIndex((o) => o.name === 'X1'), 'drawing');
  });
  await page.keyboard.press("e");
  await expect(page.locator("#level-name")).toHaveText("part");
  await page.evaluate(async () => {
    const { select } = await import('./selection.js');
    select(window.viewer.doc.objects.findIndex((o) => o.name === 'R7'), 'drawing');
    (await import('./background.js')).setBackground('dark');
  });
  const before = await page.evaluate(() => ({ level: window.viewer.level, path: window.viewer.path, selected: window.viewer.selected }));
  // Unapplied text must stay in this tab, while its applied predecessor is handed to the child.
  await page.keyboard.press("Control+q");
  await page.locator("#source textarea:visible").fill(part.replaceAll("R1", "R8"));
  await page.getByRole("menuitem", { name: "File", exact: true }).click();
  const popup = page.waitForEvent("popup");
  await page.locator('[data-id="new-window"]').click();
  const child = await popup;
  try {
    await expect(child.locator("#level-name")).toHaveText("part");
    await expect.poll(() => child.evaluate(() => window.viewer.path)).toEqual(before.path);
    expect(await child.evaluate(() => window.viewer.doc.objects[window.viewer.selected].name)).toBe("R7");
    expect(await child.evaluate(() => window.viewer.doc.files.some((f) => f.text.includes("R8")))).toBe(false);
    await expect(child.locator("#pool-list .pool-row")).toHaveCount(2);
    expect(await child.evaluate(async () => (await import('./util.js')).load('background'))).toBe("dark");
    expect(await page.evaluate(() => ({ level: window.viewer.level, path: window.viewer.path, selected: window.viewer.selected }))).toEqual(before);
    if (process.env.RNL_NAV_SCREENSHOTS && info.project.name === "chromium") {
      await child.screenshot({ path: path.join(process.env.RNL_NAV_SCREENSHOTS, "web-new-tab.png") });
      fs.writeFileSync(path.join(process.env.RNL_NAV_SCREENSHOTS, "web-tabs.json"), JSON.stringify({ parent: page.url(), child: child.url(), path: before.path }, null, 2));
    }
  } finally { await child.close(); }
});

test("Shift double click opens a subcircuit in a real new tab and pop-up blocking is explained", async ({ page }, info) => {
  await start(page, "?url=samples/cases/04-hierarchy.sp");
  await shown(page);
  await page.locator("#canvas").focus();
  await page.keyboard.press("f");
  const target = await page.evaluate(async () => {
    const { view } = await import('./drawing.js');
    const { doc, level } = window.viewer;
    const hit = doc.levels[level].hits.find((h) => h.kind === 'box' && h.objects.some((o) => doc.objects[o].name === 'X2'));
    const [a, b] = hit.points;
    const canvas = document.getElementById('canvas'), rect = canvas.getBoundingClientRect();
    return { x: rect.left + ((a[0] + b[0]) / 2 - view.x) * view.k, y: rect.top + ((a[1] + b[1]) / 2 - view.y) * view.k };
  });
  const before = await page.evaluate(() => ({ path: window.viewer.path, selected: window.viewer.selected }));
  const popup = page.waitForEvent("popup");
  await page.keyboard.down("Shift");
  await page.mouse.dblclick(target.x, target.y);
  await page.keyboard.up("Shift");
  const child = await popup;
  try {
    await expect(child.locator("#level-name")).toHaveText("amp");
    expect(await page.evaluate(() => ({ path: window.viewer.path, selected: window.viewer.selected }))).toEqual(before);
    if (process.env.RNL_NAV_SCREENSHOTS && info.project.name === "chromium") await child.screenshot({ path: path.join(process.env.RNL_NAV_SCREENSHOTS, "shift-double-click-new-window.png") });
  } finally { await child.close(); }
  await page.evaluate(() => { window.open = () => null; });
  await page.getByRole("menuitem", { name: "File", exact: true }).click();
  await page.locator('[data-id="new-window"]').click();
  await expect(page.locator("dialog.dialog")).toContainText("The browser blocked the new viewer tab");
});

test("Shift Library opens a previous URL with its includes, and the child retains URL pool sources", async ({ page }) => {
  await start(page, "?url=samples/cases/04-hierarchy.sp");
  const expected = await shown(page);
  await opened(page, () => page.locator("#file-input").setInputFiles({ name: "other.sp", mimeType: "text/plain", buffer: Buffer.from("R1 a 0 1k\n.end\n") }));
  await page.locator("#tab-files").click();
  const before = await page.evaluate(() => ({ path: window.viewer.path, text: window.viewer.doc.files[0].text }));
  const popup = page.waitForEvent("popup");
  await page.locator("#pool-list .pool-open").filter({ hasText: "04-hierarchy.sp" }).click({ modifiers: ["Shift"] });
  const child = await popup;
  try {
    await expect(child.locator("#level-name")).toHaveText("04-hierarchy.sp");
    expect((await shown(child)).files).toEqual(expected.files);
    expect(await page.evaluate(() => ({ path: window.viewer.path, text: window.viewer.doc.files[0].text }))).toEqual(before);
    await expect(page.locator("#pane-files")).toBeVisible();
    // Switch away and reopen the inherited URL pool row: its relative include still resolves.
    await child.locator("#tab-files").click();
    await opened(child, () => child.locator("#pool-list .pool-open").filter({ hasText: /^other.sp$/ }).click());
    await opened(child, () => child.locator("#pool-list .pool-open").filter({ hasText: "04-hierarchy.sp" }).click());
    expect(await child.evaluate(() => window.viewer.doc.files)).toEqual(expected.files);
  } finally { await child.close(); }
});

// Playwright builds and serves the site when the run can include these tests (playwright.config.js)
test.skip(!process.env.RNL_WEB_URL, "the site is not built: RNL_WEB=0");

/** The site that the web server serves. */
const site = path.join(root, "apps", "web", "dist");

/** Open the web viewer with `query`, such as "?url=...", and wait until it has started. */
async function start(page, query = "") {
  await page.goto(`${process.env.RNL_WEB_URL}/index.html${query}`);
  await expect(page.locator("#menubar")).toBeVisible();
}

/** Wait until a Document is shown that `seen` does not hold, and nothing is being read; return it. */
async function shown(page) {
  await page.waitForFunction(
    () => {
      const doc = window.viewer.doc;
      return doc && !(window.seen && window.seen.has(doc)) && !document.body.classList.contains("busy");
    },
  );
  return page.evaluate(() => window.viewer.doc);
}

/** Run `action`, which opens or reads a document, and return the Document shown after it. */
async function opened(page, action) {
  await page.evaluate(() => {
    window.seen = new WeakSet(window.viewer.doc ? [window.viewer.doc] : []);
  });
  await action();
  return shown(page);
}

/** The Document that `rnl view` prints for a file, named as given, from the repository's root. */
function rnlView(file) {
  const out = execFileSync(process.env.RNL_BIN, ["view", file], { cwd: root, encoding: "utf8", maxBuffer: 1 << 28 });
  return JSON.parse(out);
}

/**
 * A Document without what names its files in the workspace: the entry's path, and the paths of the
 * files missing, which keep only where they are from the entry's directory, such as ../x.inc.
 */
function unnamed(doc) {
  const names = (p) => p.replaceAll("\\", "/").split("/").filter(Boolean);
  const dir = names(doc.path).slice(0, -1);
  const fromDir = (file) => {
    const to = names(file);
    let k = 0;
    while (k < dir.length && k < to.length - 1 && to[k] === dir[k]) k += 1;
    return [...dir.slice(k).map(() => ".."), ...to.slice(k)].join("/");
  };
  return { ...doc, path: null, missing: doc.missing.map(fromDir) };
}

/**
 * The Document without the drawings of the objects that have no position, as the web viewer asks
 * the core for it (`lazy_unplaced`) and then asks for those of the rows its list shows. The
 * equality test (bindings/runtime-wasm/tests/equality.mjs) compares them with `rnl view`'s.
 */
function lazily(doc) {
  const levels = doc.levels.map((l) => ({ ...l, unplaced: l.unplaced.map(({ svg, primitives, ...u }) => u) }));
  return { ...doc, levels };
}

/** The same Document as `rnl view` builds for `file`, but for the paths that name the files and the drawings of the objects without a position. */
function sameAsRnl(doc, file) {
  expect(unnamed(doc)).toEqual(unnamed(lazily(rnlView(file))));
}

/** The directories the test made, removed after it, and the servers it started, stopped after it. */
const dirs = [];
const servers = [];
test.afterEach(async () => {
  for (const dir of dirs.splice(0)) fs.rmSync(dir, { recursive: true, force: true });
  for (const s of servers.splice(0)) {
    s.closeAllConnections();
    await new Promise((resolve) => s.close(resolve));
  }
});

/**
 * A web server of the test's own on 127.0.0.1, which lets any page read what it serves (CORS): each
 * of `files` by its path, and a redirect for each path of `moved` to the path it gives. Its address.
 */
async function server(files, moved = {}) {
  const s = http.createServer((request, response) => {
    const at = decodeURIComponent(new URL(request.url, "http://host").pathname);
    response.setHeader("Access-Control-Allow-Origin", "*");
    if (at in moved) response.writeHead(302, { Location: moved[at] }).end();
    else if (at in files) response.writeHead(200, { "Content-Type": "text/plain" }).end(files[at]);
    else response.writeHead(404).end();
  });
  servers.push(s);
  await new Promise((resolve) => s.listen(0, "127.0.0.1", resolve));
  return `http://127.0.0.1:${s.address().port}`;
}

/** A new directory under `parent` with files, each a text or bytes by its path in it. */
function folder(files, parent = os.tmpdir()) {
  const dir = fs.mkdtempSync(path.join(parent, "rnl-web-"));
  dirs.push(dir);
  write(dir, files);
  return dir;
}

/** Write files into a directory, each a text or bytes by its path in it. */
function write(dir, files) {
  for (const [name, content] of Object.entries(files)) {
    fs.mkdirSync(path.dirname(path.join(dir, name)), { recursive: true });
    fs.writeFileSync(path.join(dir, name), content);
  }
}

/** What the page shows of the Document: the drawing, the text, the problems and the hierarchy. */
async function showsDocument(page, doc) {
  await expect(page).toHaveTitle(`${doc.name} - RNL Viewer`);
  // the drawing of the first level, the SVG the core wrote
  await expect(page.locator("#level-name")).toHaveText(doc.levels[0].title);
  const groups = doc.levels[0].svg.split("<g data-id=").length - 1;
  await expect(page.locator("#canvas > svg g[data-id]")).toHaveCount(groups);
  // the text: a tab for each file, and the lines of the first
  await expect(page.locator("#file-tabs button")).toHaveText(doc.files.map((f) => f.name.split("/").pop()));
  const lines = doc.files[0].text.replace(/\r?\n$/, "").split(/\r?\n/).length;
  await expect(page.locator("#source .file:not([hidden]) .ln")).toHaveCount(lines);
  // Core diagnostics retain their counts; viewer-only package remedies follow the relevant error.
  await expect(page.locator("#diags .diag:not(.package-hint)")).toHaveCount(doc.diagnostics.length + (doc.failed ? 1 : 0));
  // the hierarchy: the top level, and every subcircuit in the index
  await page.locator("#tab-hierarchy").click();
  await expect(page.locator("#tree .tree-row").first()).toContainText(doc.levels[0].title);
  await expect(page.locator("#index .index-row:not(.top)")).toHaveCount(doc.levels.length - 1);
  await page.locator("#tab-netlist").click();
}

/** Netlists of the samples, by their paths under samples/ and tests/conformance/. */
const SAMPLES = ["cases/04-hierarchy.sp", "cases/01-rc-lowpass.sp", "cases/12-diagnostics.sp", "equivalence/inverter-chain.scs"];

for (const file of SAMPLES) {
  test(`?url=samples/${file} opens it as rnl view does`, async ({ page }) => {
    await start(page, `?url=samples/${file}`);
    const doc = await shown(page);
    expect(doc.path).toBe(`/samples/${file}`);
    sameAsRnl(doc, `tests/conformance/${file}`);
    await showsDocument(page, doc);
    await page.locator("#tab-files").click();
    await expect(page.locator("#pool-list .pool-row.current")).toHaveAttribute("data-path", `/samples/${file}`);
    await expect(page.locator("#pool-list .pool-row.current")).toHaveAttribute("aria-current", "true");
    // the core builds in a Worker
    expect(page.workers().map((w) => new URL(w.url()).pathname)).toContain("/core/worker.js");
  });
}

test("a URL's include is read from its server, and the levels, files and problems are shown", async ({ page }) => {
  await start(page, "?url=samples/cases/04-hierarchy.sp");
  const doc = await shown(page);
  expect(doc.files.map((f) => f.path)).toEqual(["04-hierarchy.sp", "04-hierarchy-lib.inc"]);
  expect(doc.missing).toEqual([]);
  await page.locator("#tab-hierarchy").click();
  await expect(page.locator("#tree .tree-row")).toHaveText(["04-hierarchy.sptop", "X1amp", "X2amp", "X3rdiv"]);
  // the include's subcircuit opens, and its file is shown
  await page.locator("#index .index-row", { hasText: "rdiv" }).click();
  await expect(page.locator("#level-name")).toHaveText("rdiv");
  await page.locator("#tab-netlist").click();
  await page.locator("#file-tabs button", { hasText: "04-hierarchy-lib.inc" }).click();
  const first = doc.files[1].text.split(/\r?\n/)[0];
  await expect(page.locator("#source .file:not([hidden]) .ln").first()).toContainText(first);

  // the problems of 12-diagnostics, with their files and lines
  await start(page, "?url=samples/cases/12-diagnostics.sp");
  const diagnostics = await shown(page);
  const errors = diagnostics.diagnostics.filter((d) => d.severity === "error");
  expect(errors.length).toBeGreaterThan(0);
  await expect(page.locator("#counts .n-error")).toHaveText(`${errors.length} ${errors.length === 1 ? "error" : "errors"}`);
  const [e] = errors;
  await expect(page.locator("#diags .diag.error .loc").first()).toHaveText(`${e.file}:${e.line}${e.col ? `:${e.col}` : ""}`);
  expect(diagnostics.diagnostics.filter((d) => d.line > 0).every((d) => d.file_index === 0)).toBe(true);
});

test("a URL that redirects opens as the one it redirects to, with its includes above it", async ({ page }) => {
  const base = await server(
    {
      "/a/b/top.sp": "* top\n.include \"x.inc\"\n.include \"../lib.inc\"\nX1 a 0 inner\nX2 a 0 outer\n.end\n",
      "/a/b/x.inc": "* x\n.subckt inner p q\nR1 p q 1k\n.ends\n",
      "/a/lib.inc": "* lib\n.subckt outer p q\nR2 p q 1k\n.ends\n",
    },
    { "/short": "/a/b/top.sp", "/top.sp": "/a/b/top.sp" },
  );
  await start(page, `?url=${base}/a/b/top.sp`);
  const doc = await shown(page);
  expect(doc.path).toBe("/a/b/top.sp");
  expect(doc.files.map((f) => f.path)).toEqual(["top.sp", "x.inc", "/a/lib.inc"]);
  expect(doc.missing).toEqual([]);
  expect(doc.diagnostics.filter((d) => d.severity === "error")).toEqual([]);
  // a short address, and a netlist at the server's root, that the server sends on to it
  for (const from of ["/short", "/top.sp"]) {
    await start(page, `?url=${base}${from}`);
    expect(await shown(page)).toEqual(doc);
  }
  await showsDocument(page, doc);
});

test("a netlist served at the root of its server is named after the server", async ({ page }) => {
  // named as the page names it, for rnl view
  const files = {
    "127.0.0.1": "* top\n.include \"x.inc\"\nX1 a 0 inner\n.end\n",
    "x.inc": "* x\n.subckt inner p q\nR1 p q 1k\n.ends\n",
  };
  const base = await server({ "/": files["127.0.0.1"], "/x.inc": files["x.inc"] });
  await start(page, `?url=${base}/`);
  const doc = await shown(page);
  expect([doc.path, doc.name]).toEqual(["/127.0.0.1", "127.0.0.1"]);
  expect(doc.files.map((f) => f.path)).toEqual(["127.0.0.1", "x.inc"]);
  // every problem is at a line of one of the files
  expect(doc.diagnostics.length).toBeGreaterThan(0);
  for (const d of doc.diagnostics) expect(doc.files[d.file_index]?.name).toBe(d.file);
  sameAsRnl(doc, path.join(folder(files), "127.0.0.1"));
  await showsDocument(page, doc);
});

test("files set on the file input open as with Open, and the page asks which is the netlist", async ({ page }) => {
  await start(page);
  const cases = (...names) => names.map((name) => path.join(root, "tests", "conformance", "cases", name));
  // a netlist and its include: the include is mostly included, so the netlist opens
  let doc = await opened(page, () =>
    page.locator("#file-input").setInputFiles(cases("04-hierarchy.sp", "04-hierarchy-lib.inc")),
  );
  expect(doc.path).toBe("/04-hierarchy.sp");
  expect(doc.files.map((f) => f.path)).toEqual(["04-hierarchy.sp", "04-hierarchy-lib.inc"]);
  sameAsRnl(doc, "tests/conformance/cases/04-hierarchy.sp");
  await showsDocument(page, doc);

  // two netlists: the page asks, the netlists first, and opens the one chosen
  await page.locator("#file-input").setInputFiles(cases("04-hierarchy-lib.inc", "12-diagnostics.sp", "01-rc-lowpass.sp"));
  const dialog = page.locator("dialog.dialog");
  await expect(dialog.locator(".dialog-title")).toHaveText("Open");
  await expect(dialog.locator(".dialog-option")).toHaveText(["01-rc-lowpass.sp", "12-diagnostics.sp", "04-hierarchy-lib.inc"]);
  doc = await opened(page, async () => {
    await dialog.locator(".dialog-option", { hasText: "12-diagnostics.sp" }).click();
    await dialog.getByRole("button", { name: "Open" }).click();
  });
  await expect(dialog).toHaveCount(0);
  expect(doc.path).toBe("/12-diagnostics.sp");
  sameAsRnl(doc, "tests/conformance/cases/12-diagnostics.sp");

  // cancelled, the dialog opens nothing
  await page.locator("#file-input").setInputFiles(cases("12-diagnostics.sp", "01-rc-lowpass.sp"));
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await expect(dialog).toHaveCount(0);
  expect(await page.evaluate(() => window.viewer.doc.path)).toBe("/12-diagnostics.sp");
});

test("Include Folder adds netlists and packages, and clicking a pooled file opens it", async ({ page }) => {
  await start(page);
  const dir = folder({
    "top.sp": "* top\n.include \"lib/parts.inc\"\nX1 a 0 part\n.end\n",
    "lib/parts.inc": ".subckt part p q\nR1 p q 1k\n.ends\n",
    "notes.txt": "notes\n",
  });
  const setting = page.locator("#folder-input").setInputFiles(dir).catch(() => {});
  const name = path.basename(dir);
  const row = page.locator("#pool-list .pool-row", { has: page.getByRole("button", { name: `Open ${name}/top.sp`, exact: true }) });
  await expect(row).toBeVisible();
  await Promise.race([setting, new Promise((resolve) => setTimeout(resolve, 3000))]);
  expect(await page.evaluate(() => window.viewer.doc)).toBeNull();
  await expect(page.locator("#pool-list")).not.toContainText("notes.txt");
  const doc = await opened(page, () => row.locator(".pool-open").click());
  expect(doc.path).toBe(`/${name}/top.sp`);
  expect(doc.files.map((f) => f.path)).toEqual(["top.sp", "lib/parts.inc"]);
  sameAsRnl(doc, path.join(dir, "top.sp"));
  await showsDocument(page, doc);
});

test("files dropped on the page open as with Open", async ({ browserName, page }) => {
  // A drop made in the page: Chromium gives its items no entries, so the page opens the files as
  // with Open, and Firefox gives them entries that read the files, as a drop from the computer does.
  // WebKit gives them entries whose files cannot be read (NotFoundError), which a drop from the
  // computer does not.
  test.skip(browserName === "webkit", "WebKit's entries of a drop made in the page cannot read their files");
  await start(page);
  const files = {
    "top.sp": "* top\n.include \"parts.inc\"\nX1 a 0 part\n.end\n",
    "parts.inc": ".subckt part p q\nR1 p q 1k\n.ends\n",
  };
  const dataTransfer = await page.evaluateHandle((files) => {
    const transfer = new DataTransfer();
    for (const [name, text] of Object.entries(files)) transfer.items.add(new File([text], name));
    return transfer;
  }, files);
  const doc = await opened(page, () => page.dispatchEvent("#app", "drop", { dataTransfer }));
  await expect(page.locator("#pool-list .pool-row.current")).toHaveCount(1);
  expect(doc.path).toBe("/top.sp");
  expect(doc.files.map((f) => f.path)).toEqual(["top.sp", "parts.inc"]);
  sameAsRnl(doc, path.join(folder(files), "top.sp"));
  await showsDocument(page, doc);
});

test("a Latin-1 file is read as rnl reads it", async ({ page }) => {
  await start(page);
  const [micro, degree] = [0xb5, 0xb0].map((c) => String.fromCharCode(c));
  const bytes = Buffer.from(`* 10${micro}s at 25${degree}C\nR1 a 0 1k\n.end\n`, "latin1");
  const doc = await opened(page, () =>
    page.locator("#file-input").setInputFiles({ name: "latin1.sp", mimeType: "text/plain", buffer: bytes }),
  );
  expect(doc.files[0].text.split("\n")[0]).toBe(`* 10${micro}s at 25${degree}C`);
  sameAsRnl(doc, path.join(folder({ "latin1.sp": bytes }), "latin1.sp"));
  await expect(page.locator("#source .ln").first()).toContainText(`10${micro}s at 25${degree}C`);
});

test("a netlist of subcircuits alone opens on the one nothing calls, as the core reads it", async ({ page }) => {
  await start(page);
  const open = (name, lines) => {
    const text = lines.concat("").join(String.fromCharCode(10));
    return { text, file: { name, mimeType: "text/plain", buffer: Buffer.from(text) } };
  };
  const [name, lines] = UNCALLED.uncalled;
  const { text, file } = open(name, lines);
  const doc = await opened(page, () => page.locator("#file-input").setInputFiles(file));
  sameAsRnl(doc, path.join(folder({ [name]: text }), name));
  expect(doc.levels.map((l) => l.title)).toEqual(["uncalled.sp", "ota_5t", "Testbench"]);
  // the top level has no objects: the testbench opens, drawn, and the status bar says why
  await expect(page.locator("#level-name")).toHaveText("Testbench");
  await expect(page.locator("#notice")).toHaveText("The top level is empty; showing Testbench.");
  await expect(page.locator("#canvas > svg g[data-id]")).not.toHaveCount(0);
  await page.locator("#tab-hierarchy").click();
  await expect(page.locator("#tree .tree-row")).toHaveText(["uncalled.sptop", "Testbenchnot called", "XDUTota_5t"]);
  await page.locator('#tree .tree-row[data-path="r2/0"]').click();
  await expect(page.locator("#level-name")).toHaveText("ota_5t");
  await expect(page.locator("#notice")).toBeHidden();

  // a file with no object at all says so, and has no hierarchy to point to
  const empty = open(...UNCALLED.nothing);
  await opened(page, () => page.locator("#file-input").setInputFiles(empty.file));
  await expect(page.locator("#level-name")).toHaveText("nothing.sp");
  await expect(page.locator("#empty")).toHaveText("This level has no objects, so nothing is drawn.");
});

test("an include that is not there is missing, and a problem at its line", async ({ page }) => {
  await start(page);
  const text = "* top\n.include \"gone.inc\"\nR1 a 0 1k\n.end\n";
  const doc = await opened(page, () =>
    page.locator("#file-input").setInputFiles({ name: "top.sp", mimeType: "text/plain", buffer: Buffer.from(text) }),
  );
  expect(doc.missing).toEqual(["/gone.inc"]);
  const problem = doc.diagnostics.find((d) => d.message.includes("gone.inc cannot be found"));
  expect(problem).toMatchObject({ severity: "error", file: "top.sp", file_index: 0, line: 2 });
  sameAsRnl(doc, path.join(folder({ "top.sp": text }), "top.sp"));
  await expect(page.locator("#diags .diag.error .msg")).toContainText(["gone.inc cannot be found"]);
  await expect(page.locator("#source .ln").nth(1)).toHaveClass(/\bd-error\b/);
});

test("an include above the files opened is missing, not a file of that name opened with them", async ({ page }) => {
  await start(page);
  // top.sp and x.inc chosen in sub/, whose top.sp includes the x.inc above it, which is not there
  const dir = folder({
    "sub/top.sp": "* top\n.include \"../x.inc\"\nX1 a 0 inner\n.end\n",
    "sub/x.inc": "* sub's own x.inc\n.subckt inner p q\nR1 p q 1k\n.ends\n",
  });
  const doc = await opened(page, () =>
    page.locator("#file-input").setInputFiles(["top.sp", "x.inc"].map((name) => path.join(dir, "sub", name))),
  );
  expect(doc.path).toBe("/top.sp");
  expect(doc.files.map((f) => f.path)).toEqual(["top.sp"]);
  expect(doc.missing).toEqual(["/../x.inc"]);
  const problem = doc.diagnostics.find((d) => d.message === "the included file ../x.inc cannot be found");
  expect(problem).toMatchObject({ severity: "error", file: "top.sp", file_index: 0, line: 2 });
  sameAsRnl(doc, path.join(dir, "sub", "top.sp"));
});

test("Reload reads a served netlist again after it changed, with an include that now is there", async ({ page }) => {
  // a directory of the site, served like the samples
  const dir = folder({ "top.sp": "* top\n.include \"parts.inc\"\nR1 a 0 1k\n* @RNL {#R1 @PLACE(0, 0)}\n.end\n" }, site);
  const name = path.basename(dir);
  await start(page, `?url=${name}/top.sp`);
  let doc = await shown(page);
  expect(doc.missing).toEqual([`/${name}/parts.inc`]);
  expect(doc.diagnostics.some((d) => d.message.includes("parts.inc cannot be found"))).toBe(true);
  sameAsRnl(doc, path.join(dir, "top.sp"));

  write(dir, {
    "top.sp": "* top, changed\n.include \"parts.inc\"\nR1 a 0 2k\nX1 a 0 part\n* @RNL {#R1 @PLACE(0, 0)}\n.end\n",
    "parts.inc": ".subckt part p q\nR2 p q 1k\n.ends\n",
  });
  doc = await opened(page, () => page.keyboard.press("F5"));
  expect(doc.missing).toEqual([]);
  expect(doc.files.map((f) => f.path)).toEqual(["top.sp", "parts.inc"]);
  expect(doc.files[0].text).toContain("R1 a 0 2k");
  sameAsRnl(doc, path.join(dir, "top.sp"));
  await showsDocument(page, doc);
  // the page was not loaded again
  expect(await page.evaluate(() => !!window.seen)).toBe(true);
});

test("Help > About names the version and the commit the site was built from", async ({ page }) => {
  await start(page, "?url=samples/cases/01-rc-lowpass.sp");
  await shown(page);
  // what build.mjs wrote: the workspace's version, and the commit when git knows it
  const about = JSON.parse(fs.readFileSync(path.join(site, "about.json"), "utf8"));
  expect(about.version).toMatch(/^\d+\.\d+\.\d+/);
  await page.locator("#menubar .menu-title", { hasText: "Help" }).click();
  await page.locator('#menubar .menu-item[data-id="about"]').click();
  const dialog = page.locator("dialog.dialog");
  await expect(dialog.locator(".dialog-title")).toHaveText("About RNL Viewer");
  await expect(dialog.locator(".dialog-name")).toHaveText(`RNL Viewer ${about.version}${about.commit ? ` (${about.commit})` : ""}`);
  await dialog.getByRole("button", { name: "Close" }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.locator("#level-name")).toHaveText("01-rc-lowpass.sp");
});

test("Help > Desktop Viewer opens the site's page of the tools in a new tab", async ({ context, page }) => {
  // the tab is given a page of the test's own instead of the site's
  await context.route("https://rich-netlist.github.io/**", (route) =>
    route.fulfill({ contentType: "text/html", body: "<!doctype html><title>RNL tools</title>" }),
  );
  await start(page, "?url=samples/cases/01-rc-lowpass.sp");
  await shown(page);
  await page.locator("#menubar .menu-title", { hasText: "Help" }).click();
  const [tab] = await Promise.all([
    context.waitForEvent("page"),
    page.locator('#menubar .menu-item[data-id="desktop-viewer"]').click(),
  ]);
  // Playwright's browsers prefer en-US, so the English page
  await expect(tab).toHaveURL("https://rich-netlist.github.io/rnl/en/tools.html");
  expect(await tab.evaluate(() => window.opener)).toBe(null);
  await tab.close();
  await expect(page.locator("#level-name")).toHaveText("01-rc-lowpass.sp");
});

test("File > Load from Clipboard opens the text alone, as rnl view reads it in a file of its own", async ({ page }) => {
  const text = fs.readFileSync(path.join(root, "tests", "conformance", "cases", "04-hierarchy.sp"), "utf8");
  await start(page);
  // the test gives the page the clipboard's text, as a browser would once it lets the page read it
  await page.evaluate((t) => {
    navigator.clipboard.readText = async () => t;
  }, text);
  const doc = await opened(page, async () => {
    await page.locator("#menubar .menu-title", { hasText: "File" }).click();
    await page.locator('#menubar .menu-item[data-id="load-clipboard"]').click();
    await page.locator("dialog.dialog").getByRole("button", { name: "Load" }).click();
  });
  expect([doc.name, doc.files.length, doc.missing]).toEqual(["clipboard.sp", 1, ["/04-hierarchy-lib.inc"]]);
  sameAsRnl(doc, path.join(folder({ "clipboard.sp": text }), "clipboard.sp"));
  await showsDocument(page, doc);
});

test("the Not drawn list asks the core for the drawings of its rows, which the Document does not carry", async ({ page }) => {
  const text = "* none placed\nR1 a b 1k\nC1 b 0 1p\nM1 d g s 0 nmos\n.model nmos nmos\n.end\n";
  await start(page);
  await page.evaluate((t) => {
    navigator.clipboard.readText = async () => t;
  }, text);
  const doc = await opened(page, async () => {
    await page.locator("#menubar .menu-title", { hasText: "File" }).click();
    await page.locator('#menubar .menu-item[data-id="load-clipboard"]').click();
    await page.locator("dialog.dialog").getByRole("button", { name: "Load" }).click();
  });
  const listed = doc.levels[0].unplaced;
  expect(listed.length).toBeGreaterThanOrEqual(3);
  expect(listed.some((u) => u.svg || u.primitives)).toBe(false);
  await page.locator("#tab-unplaced").click();
  const rows = page.locator("#unplaced-list .unplaced-row");
  await expect(rows).toHaveCount(listed.length);
  await expect(rows.locator(".thumb svg")).toHaveCount(listed.length);
  // each is drawn with its own symbol and the nets at its pins
  const nets = rows.filter({ has: page.locator(".name", { hasText: /^M1$/ }) }).locator(".thumb svg text");
  expect((await nets.allTextContents()).sort()).toEqual(["0", "d", "g", "s"]);
});

test("the core's labels are clicked and marked, the pointer shows names, and a click on two objects lists them", async ({ page }) => {
  await start(page);
  await page.evaluate((t) => {
    navigator.clipboard.readText = async () => t;
  }, PICK.concat("").join("\n"));
  const doc = await opened(page, async () => {
    await page.locator("#menubar .menu-title", { hasText: "File" }).click();
    await page.locator('#menubar .menu-item[data-id="load-clipboard"]').click();
    await page.locator("dialog.dialog").getByRole("button", { name: "Load" }).click();
  });
  const labels = doc.levels[0].hits.filter((h) => h.kind === "label");
  expect(labels.map((h) => h.id).sort()).toEqual(["#C1::name", "#C1::value", "#R1::name", "#R1::value", "#R2::name", "#R2::value", "a text"]);
  expect(labels.filter((h) => h.objects.length === 0).map((h) => h.id)).toEqual(["a text"]);
  /** The page point of a point of the drawing, in its pixels. */
  const point = ([u, v]) =>
    page.evaluate(
      ([u, v]) => {
        const canvas = document.getElementById("canvas");
        const [x, y, w] = document.querySelector("#canvas > svg").getAttribute("viewBox").split(" ").map(Number);
        const r = canvas.getBoundingClientRect();
        const k = canvas.clientWidth / w;
        return { x: r.left + (u - x) * k, y: r.top + (v - y) * k };
      },
      [u, v],
    );
  const middle = (h) => [(h.points[0][0] + h.points[1][0]) / 2, (h.points[0][1] + h.points[1][1]) / 2];
  const selected = () => page.evaluate(() => (window.viewer.selected === null ? null : window.viewer.doc.objects[window.viewer.selected].name));
  // a click on C1's value selects C1, and its two labels are marked
  const value = await point(middle(labels.find((h) => h.id === "#C1::value")));
  await page.mouse.click(value.x, value.y);
  await expect.poll(selected).toBe("C1");
  await expect(page.locator("#canvas svg .overlay rect.label")).toHaveCount(2);
  // the pointer at rest on C1's symbol shows its name
  const c1 = await point(middle(doc.levels[0].hits.find((h) => h.kind === "box" && h.objects.some((o) => doc.objects[o].name === "C1"))));
  await page.mouse.move(c1.x, c1.y);
  await expect(page.locator("#hover-tip")).toHaveText("C1");
  // a click where R1's box and R2's overlap lists both
  const box = (name) => doc.levels[0].hits.find((h) => h.kind === "box" && h.objects.some((o) => doc.objects[o].name === name)).points;
  const [[a0, b0], [a1, b1]] = [box("R1"), box("R2")];
  const both = await point([(Math.max(a0[0], a1[0]) + Math.min(b0[0], b1[0])) / 2, (Math.max(a0[1], a1[1]) + Math.min(b0[1], b1[1])) / 2]);
  await page.mouse.click(both.x, both.y);
  await expect(page.locator("#pick-list .pick-row")).toHaveText(["instanceR1", "instanceR2"]);
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect.poll(selected).toBe("R2");
});

test("the text edited and applied is built by the core as rnl view builds the same files, and Save downloads the file shown", async ({ page }) => {
  const top = "* top\n.include \"parts.inc\"\nR1 a 0 1k\nX1 a 0 part\n* @RNL {#R1 @PLACE(0, 0)}\n.end\n";
  const parts = ".subckt part p q\nR2 p q 1k\n.ends\n";
  const dir = folder({ "top.sp": top, "parts.inc": parts }, site);
  await start(page, `?url=${path.basename(dir)}/top.sp`);
  await shown(page);
  // edit the netlist and its include, each in the text box of its own tab
  await page.keyboard.press("Control+q");
  const area = page.locator("#source .editor:not([hidden]) textarea");
  const editedTop = top.replace("R1 a 0 1k", "R1 a 0 2k");
  const editedParts = parts.replace("R2", "R7");
  await area.fill(editedTop);
  await page.locator("#file-tabs button", { hasText: "parts.inc" }).click();
  await area.fill(editedParts);
  await expect(page.locator("#edit-state")).toHaveText("Changes not applied yet");
  // applied, the core builds the files with the texts in place of what is read, as it builds the same texts from disk
  const doc = await opened(page, () => page.locator("#apply-edits").click());
  expect(doc.files.map((f) => [f.path, f.text])).toEqual([["top.sp", editedTop], ["parts.inc", editedParts]]);
  expect(doc.objects.some((o) => o.kind === "instance" && o.name === "R7")).toBe(true);
  sameAsRnl(doc, path.join(folder({ "top.sp": editedTop, "parts.inc": editedParts }), "top.sp"));
  await expect(page.locator("#source .editor")).toHaveCount(0);
  // the server's files are as they were: nothing was written
  expect(fs.readFileSync(path.join(dir, "parts.inc"), "utf8")).toBe(parts);
  // Save downloads the file shown, as the text box holds it
  const [download] = await Promise.all([page.waitForEvent("download"), page.keyboard.press("Control+s")]);
  expect(download.suggestedFilename()).toBe("parts.inc");
  expect(fs.readFileSync(await download.path(), "utf8")).toBe(editedParts);
  await expect(page.locator("#note")).toHaveText("Downloaded parts.inc.");
  // the file downloaded is what was read from now on; the netlist's change is still not saved
  await expect(page.locator("#file-tabs button[data-unsaved='true']")).toHaveText(["top.sp"]);
});


test("Include File resolves a package with a different filename and Exclude removes it from search", async ({ page }) => {
  await start(page);
  await page.locator("#file-input").setInputFiles({ name: "top.sp", mimeType: "text/plain", buffer: Buffer.from(
    '* top\n* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="acme", version="1.0") }\nR1 a 0 1k\n.end\n') });
  await shown(page);
  await page.locator("#tab-files").click();
  expect(await page.locator("#pool-list .pool-row").count()).toBe(1);
  const d = await opened(page, () => page.locator("#include-input").setInputFiles({ name: "custom.rnl", mimeType: "text/plain", buffer: Buffer.from(
    '@RNL(type="header") { @VERSION("0.3"); @PACKAGE(name="acme", version="1.0") }\n') }));
  expect(d.diagnostics.some((d) => d.message.includes("acme 1.0 cannot be found"))).toBe(false);
  const excluded = await opened(page, () => page.getByRole("button", { name: "Exclude custom.rnl", exact: true }).click());
  expect(excluded.diagnostics.some((d) => d.message.includes("acme 1.0 cannot be found"))).toBe(true);
});

test("the web library adds SKY130 without duplicating built-ins, which open read-only", async ({ page }) => {
  await start(page);
  await page.locator("#tab-files").click();
  await page.locator('[data-group="builtin"] .lib-toggle').click();
  await expect(page.getByRole("button", { name: "Open 0.2/rnl-std.rnl", exact: true })).toBeVisible();
  await expect(page.locator('[data-group="builtin"]').getByRole("button", { name: /Exclude/ })).toHaveCount(0);
  await page.locator("#include-library").click();
  const dialog = page.locator("dialog.dialog");
  await expect(dialog.locator(".dialog-option")).toContainText(["pdk-sky130"]);
  await expect(dialog.locator(".dialog-option")).not.toContainText(["rnl-std-basic"]);
  await dialog.locator("input[type=search]").fill("sky130");
  await dialog.locator("input[type=search]").press("Enter");
  await expect(page.getByRole("button", { name: "Exclude pdk-sky130 0.1", exact: true })).toBeVisible();
  await page.locator("#file-input").setInputFiles({ name: "sky.sp", mimeType: "text/plain", buffer: Buffer.from(
    '* sky\n* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="pdk-sky130", version="0.1") }\nXM1 d g 0 0 sky130_fd_pr__nfet_01v8\n.end\n') });
  const doc = await shown(page);
  expect(doc.diagnostics.filter((d) => d.severity === "error")).toEqual([]);
  await page.locator("#tab-files").click();
  const builtin = await opened(page, () => page.getByRole("button", { name: "Open 0.3/rnl-std.rnl", exact: true }).click());
  expect(builtin.path).toContain("<bundled>");
  await page.evaluate(() => window.viewer.command("toggle-edit"));
  expect(await page.evaluate(() => window.viewer.checked("toggle-edit"))).toBe(false);
  await expect(page.locator("#source")).toContainText("@PACKAGE");
});

test("a missing published Canvas package offers Include and rebuilds; other versions offer the File menu hint", async ({ page }, info) => {
  await start(page);
  const source = fs.readFileSync(path.join(root, "tests/viewer/inputs/missing-canvas-package.sp"));
  const before = await opened(page, () => page.locator("#file-input").setInputFiles({ name: "canvas.sp", mimeType: "text/plain", buffer: source }));
  expect(before.diagnostics.some(d => d.message.includes("package analog-canvas-extensions 0.1 cannot be found"))).toBe(true);
  await expect(page.locator(".package-hint")).toContainText("This package is published.");
  if (process.env.RNL_PACKAGE_SCREENSHOTS && info.project.name === "chromium") {
    await page.screenshot({ path: path.join(process.env.RNL_PACKAGE_SCREENSHOTS, "web-missing-canvas-include.png") });
  }
  const after = await opened(page, () => page.locator(".package-hint").getByRole("button", { name: "Include", exact: true }).click());
  expect(after.diagnostics.filter(d => d.severity === "error")).toEqual([]);
  await expect(page.locator(".package-hint")).toHaveCount(0);
  await page.locator("#tab-files").click();
  await expect(page.getByRole("button", { name: "Exclude analog-canvas-extensions 0.1", exact: true })).toBeVisible();
  const builtin = page.locator('[data-group="builtin"]');
  await builtin.locator(".lib-toggle").click();
  await expect(builtin.locator(".pool-row")).toHaveCount(25);
  expect(await builtin.locator(".pool-open").allTextContents()).toEqual(expect.arrayContaining(["0.3/rnl-std.rnl", "0.2/rnl-std.rnl"]));
  for (const name of await builtin.locator(".pool-open").allTextContents()) expect(name).toMatch(/^0\.[23]\/rnl-(std|zh)/);
  if (process.env.RNL_PACKAGE_SCREENSHOTS && info.project.name === "chromium") {
    await page.screenshot({ path: path.join(process.env.RNL_PACKAGE_SCREENSHOTS, "web-library-standard-builtins.png") });
  }
  await opened(page, () => page.locator("#file-input").setInputFiles({ name: "canvas-newer.sp", mimeType: "text/plain", buffer: Buffer.from(source.toString().replace('version="0.1"', 'version="99.0"')) }));
  await expect(page.locator(".package-hint")).toContainText("Look for it in File › Include Public Package…");
  await expect(page.locator(".package-hint button")).toHaveCount(0);
});


test("independent public EDA packages draw their gallery and custom terminal markers", async ({ page }) => {
  await start(page);
  const index = await (await page.request.get(`${process.env.RNL_WEB_URL}/libraries/index.json`)).json();
  for (const name of ["eda-style", "eda-symbol"]) {
    expect(index.filter((p) => p.name === name)).toMatchObject([
      { version: "0.1", path: `${name}/${name}.rnl`, builtin: false },
    ]);
  }
  expect(index.every((p) => p.path.endsWith(".rnl"))).toBe(true);
  const bundled = await (await page.request.get(`${process.env.RNL_WEB_URL}/libraries/bundled.json`)).json();
  expect(Object.keys(bundled).some((p) => /eda-(style|symbol)/.test(p))).toBe(false);
  for (const name of ["eda-style", "eda-symbol"]) {
    await page.locator("#tab-files").click();
    await page.locator("#include-library").click();
    const dialog = page.locator("dialog.dialog");
    await dialog.locator("input[type=search]").fill(name);
    await dialog.getByRole("option", { name: new RegExp(`^${name} 0\\.1 —`) }).dblclick();
    await expect(page.getByRole("button", { name: `Exclude ${name} 0.1`, exact: true })).toBeVisible();
    if (name === "eda-style") {
      await expect(page.getByRole("button", { name: "Exclude eda-symbol 0.1", exact: true })).toHaveCount(0);
    }
  }
  const doc = await opened(page, () => page.locator("#file-input").setInputFiles(path.join(root, "libraries/eda-style/eda-style-gallery.scs")));
  expect(doc.diagnostics.filter((d) => ["error", "warning"].includes(d.severity))).toEqual([]);
  expect(doc.levels).toHaveLength(7);
  const svg = doc.levels[0].svg;
  expect(svg).toContain("#00cc66");
  expect(svg).toContain("#ff0000");
  expect(svg).toContain('font-family="Lucida Console, Monaco, DejaVu Sans Mono, Consolas, monospace"');
  const custom = await opened(page, () => page.locator("#file-input").setInputFiles(path.join(root, "libraries/eda-style/examples/custom-symbol.sp")));
  expect(custom.diagnostics.filter((d) => ["error", "warning"].includes(d.severity))).toEqual([]);
  expect(await page.evaluate((svg) => new DOMParser().parseFromString(svg, "image/svg+xml").querySelectorAll('rect[fill="#ff0000"]').length, custom.levels[0].svg)).toBe(6);
});

test("relative files win over pooled files with the same name, including deferred URL collection", async ({ page }) => {
  await start(page);
  await page.locator("#include-input").setInputFiles({ name: "part.inc", mimeType: "text/plain", buffer: Buffer.from("Rpool a 0 1k\n") });
  await page.locator("#tab-files").click();
  await expect(page.getByRole("button", { name: "Exclude part.inc", exact: true })).toBeVisible();
  const local = await opened(page, () => page.locator("#file-input").setInputFiles([
    { name: "top.sp", mimeType: "text/plain", buffer: Buffer.from('* top\n.include "part.inc"\n.end\n') },
    { name: "part.inc", mimeType: "text/plain", buffer: Buffer.from("Rrelative a 0 2k\n") },
  ]));
  expect(local.files[1].text).toContain("Rrelative");
  await page.route("**/priority/top.sp", (route) => route.fulfill({ body: '* top\n.include "part.inc"\n.end\n', contentType: "text/plain" }));
  await page.route("**/priority/part.inc", (route) => route.fulfill({ body: "Rremote a 0 3k\n", contentType: "text/plain" }));
  await page.evaluate(() => window.viewer.command("open-url"));
  const dialog = page.locator("dialog.dialog");
  await dialog.locator("input").fill(`${process.env.RNL_WEB_URL}/priority/top.sp`);
  const remote = await opened(page, () => dialog.locator("input").press("Enter"));
  expect(remote.files[1].text).toContain("Rremote");
  expect(remote.diagnostics.some((d) => d.message.includes("resolved through"))).toBe(false);
});

/** Load actual clipboard content through the shared preview and the WebAssembly shell. */
async function pasteText(page, text) {
  await page.evaluate((text) => {
    navigator.clipboard.readText = async () => text;
    window.viewer.command("load-clipboard");
  }, text);
  await expect(page.locator("dialog.dialog .dialog-title")).toHaveText("Load from Clipboard");
  return opened(page, () => page.locator("dialog.dialog").getByRole("button", { name: "Load", exact: true }).click());
}

test("opened web files join Files once; Exclude keeps the root open and removes package lookup", async ({ page }) => {
  await start(page);
  const pkg = { name: "custom.rnl", mimeType: "text/plain", buffer: Buffer.from(
    '@RNL(type="header") { @VERSION("0.3"); @PACKAGE(name="acme-open", version="1.0") }\n') };
  const top = { name: "top.sp", mimeType: "text/plain", buffer: Buffer.from(
    '* top\n* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="acme-open", version="1.0") }\nR1 a 0 1k\n.end\n') };
  await opened(page, () => page.locator("#file-input").setInputFiles(pkg));
  await opened(page, () => page.locator("#file-input").setInputFiles(pkg));
  await expect(page.locator("#pool-list .pool-row")).toHaveCount(1);
  const loaded = await opened(page, () => page.locator("#file-input").setInputFiles(top));
  expect(loaded.diagnostics.some((d) => d.message.includes("acme-open 1.0 cannot be found"))).toBe(false);
  await page.locator("#tab-files").click();
  await opened(page, () => page.getByRole("button", { name: "Open custom.rnl", exact: true }).click());
  await page.locator("#tab-files").click();
  const excluded = await opened(page, () => page.getByRole("button", { name: "Exclude custom.rnl", exact: true }).click());
  expect(excluded.name).toBe("custom.rnl");
  await expect(page.getByRole("button", { name: "Open custom.rnl", exact: true })).toHaveCount(0);
  const missing = await opened(page, () => page.getByRole("button", { name: "Open top.sp", exact: true }).click());
  expect(missing.diagnostics.some((d) => d.message.includes("acme-open 1.0 cannot be found"))).toBe(true);
});

test("temporary package is absent from include and USE-PACKAGE lookup; applied text survives switching", async ({ page }) => {
  await start(page);
  const pkg = '@RNL(type="header") { @VERSION("0.3"); @PACKAGE(name="acme-temp", version="1.0") }\n';
  await pasteText(page, pkg);
  await expect(page.locator(".pool-temporary.current")).toHaveAttribute("aria-current", "true");
  await expect(page.locator("#pool-list .pool-row")).toHaveCount(0);
  const text = '* top\n* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="acme-temp", version="1.0") }\n.include "clipboard.rnl"\n.end\n';
  const top = { name: "other.sp", mimeType: "text/plain", buffer: Buffer.from(text) };
  const missing = await opened(page, () => page.locator("#file-input").setInputFiles(top));
  expect(missing.missing).toContain("/clipboard.rnl");
  expect(missing.diagnostics.some((d) => d.message.includes("acme-temp 1.0 cannot be found"))).toBe(true);
  await page.locator("#tab-files").click();
  await opened(page, () => page.locator(".pool-temporary .pool-open").click());
  await page.locator("#tab-netlist").click();
  await page.evaluate(() => window.viewer.command("toggle-edit"));
  const edited = pkg.replace("1.0", "2.0");
  await page.locator("#source textarea").fill(edited);
  await opened(page, () => page.locator("#apply-edits").click());
  await page.locator("#tab-files").click();
  await page.getByRole("button", { name: "Open other.sp", exact: true }).click();
  await opened(page, () => page.locator("dialog.dialog").getByRole("button", { name: "Discard Changes", exact: true }).click());
  await page.locator("#tab-files").click();
  const back = await opened(page, () => page.locator(".pool-temporary .pool-open").click());
  expect(back.files[0].text).toBe(edited);
  await pasteText(page, "* next paste\nR2 b 0 2k\n.end\n");
  await expect(page.locator(".pool-temporary")).toHaveCount(1);
  expect((await page.evaluate(() => window.viewer.doc)).files[0].text).toContain("R2 b 0 2k");
});
