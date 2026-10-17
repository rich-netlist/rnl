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
import { root } from "../fixtures.js";

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

/** The same Document as `rnl view` builds for `file`, but for the paths that name the files. */
function sameAsRnl(doc, file) {
  expect(unnamed(doc)).toEqual(unnamed(rnlView(file)));
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
  // the problems, the most serious first
  await expect(page.locator("#diags .diag")).toHaveCount(doc.diagnostics.length + (doc.failed ? 1 : 0));
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

test("a folder set on the folder input opens as with Open Folder", async ({ page }) => {
  await start(page);
  const dir = folder({
    "top.sp": "* top\n.include \"lib/parts.inc\"\nX1 a 0 part\n.end\n",
    "lib/parts.inc": ".subckt part p q\nR1 p q 1k\n.ends\n",
    "notes.txt": "notes\n",
  });
  // With a folder, Playwright's setInputFiles waits for the input's own input event, which it starts
  // to listen for while it sets the files; in Firefox and WebKit the event can come first, and then
  // it never returns, though the page has the files and shows the dialog (seen in CI, a trace of
  // run 39023147223). So the dialog does not wait for it: it is started, the dialog checked, and then
  // it is given a few seconds, and left if it hangs; its error, when the page closes, is dropped.
  const setting = page.locator("#folder-input").setInputFiles(dir).catch(() => {});
  const name = path.basename(dir);
  const dialog = page.locator("dialog.dialog");
  await expect(dialog.locator(".dialog-title")).toHaveText("Open Folder");
  await Promise.race([setting, new Promise((resolve) => setTimeout(resolve, 3000))]);
  await expect(dialog.locator(".dialog-body > p").first()).toHaveText(`The netlist to open in ${name}:`);
  // the netlists first, then the files mostly included, then the others; the filter narrows them
  await expect(dialog.locator(".dialog-option")).toHaveText(["top.sp", "lib/parts.inc", "notes.txt"]);
  await dialog.locator("input[type=search]").fill("top");
  await expect(dialog.locator(".dialog-option")).toHaveText(["top.sp"]);
  const doc = await opened(page, () => dialog.locator("input[type=search]").press("Enter"));
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
