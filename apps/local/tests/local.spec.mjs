// The local program, in a browser: it is started with a netlist, the page opens it, edits are saved where the file is, a change on
// disk is told, Solve runs in the program, the chooser lists the machine's files, and nothing is served to anyone without the
// session. The program is the one built (RNL_LOCAL_BIN, else the debug build), with the web viewer in it or RNL_LOCAL_SITE.

import { test, expect } from "../../../tests/viewer/node_modules/@playwright/test/index.mjs";
import { spawn } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const exe = process.env.RNL_LOCAL_BIN || resolve(here, "../target/debug", process.platform === "win32" ? "rnl-viewer-local.exe" : "rnl-viewer-local");

let work, server, base, address, output;

/** Start the program on a netlist: the address it printed, which works once. */
async function start(...files) {
  const args = ["--no-browser", "--settings", join(work, "settings.json"), ...(process.env.RNL_LOCAL_SITE ? ["--site", process.env.RNL_LOCAL_SITE] : []), ...files];
  const child = spawn(exe, args, { stdio: ["ignore", "pipe", "pipe"], env: { ...process.env, XDG_CONFIG_HOME: join(work, "config"), XDG_CACHE_HOME: join(work, "cache") } });
  output = "";
  const ready = new Promise((ok, fail) => {
    child.stdout.on("data", (chunk) => {
      output += chunk;
      const served = /serving on (http:\/\/127\.0\.0\.1:\d+)\//.exec(output);
      const token = /(http:\/\/127\.0\.0\.1:\d+\/__start\?t=[0-9a-f]{64})/.exec(output);
      if (served && token) ok({ base: served[1], address: token[1] });
    });
    child.stderr.on("data", (chunk) => (output += chunk));
    child.on("exit", (code) => fail(new Error(`the program ended with ${code}: ${output}`)));
  });
  const got = await ready;
  return { child, ...got };
}

test.beforeAll(() => {
  expect(existsSync(exe), `the program is built: ${exe}`).toBe(true);
});

test.beforeEach(async () => {
  work = mkdtempSync(join(tmpdir(), "rnl-local-test-"));
  mkdirSync(join(work, "proj", "sim"), { recursive: true });
  mkdirSync(join(work, "proj", "models"), { recursive: true });
  writeFileSync(join(work, "proj", "models", "m.inc"), "* a model\n");
  writeFileSync(join(work, "proj", "sim", "extra.inc"), "* beside\n");
  writeFileSync(join(work, "outside.inc"), "* far away\n");
  writeFileSync(
    join(work, "proj", "sim", "top.sp"),
    ["* A top level", ".include extra.inc", ".include ../models/m.inc", ".include ../../outside.inc", "R1 a b 1k", "R2 b 0 1k",
      "* @RNL(type=\"header\") { @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\"); @VERSION(\"0.4\") }", "* @RNL { #R1 @PLACE(0, 0); #R2 @PLACE(9, 0); {#R1 #R2} @ALIGN(\"y\") }", ".end", ""].join("\n"),
  );
  writeFileSync(join(work, "proj", "sim", "other.sp"), "* Another\nR9 x 0 9k\n.end\n");
  ({ child: server, base, address } = await start(join(work, "proj", "sim", "top.sp")));
});

test.afterEach(() => {
  server?.kill();
});

const document = (page) => page.evaluate(() => window.viewer.doc);

async function open(page) {
  await page.goto(address);
  await page.waitForFunction(() => window.viewer?.doc && !document.body.classList.contains("busy"), null, { timeout: 90000 });
}

test("an address that works once opens the page, and nothing is served to anyone without the session", async ({ page, request }) => {
  const without = await request.get(`${base}/api/session`);
  expect(without.status()).toBe(403);
  const file = `${base}/f${join(work, "proj", "sim", "top.sp").replaceAll("\\", "/").replace(/^([^/])/, "/$1")}`;
  expect((await request.get(file)).status()).toBe(403);
  expect((await request.get(`${base}/api/ls?dir=/`)).status()).toBe(403);
  // a wrong Host is no business of the program's
  const foreign = await request.get(`${base}/`, { headers: { Host: "example.com" } });
  expect(foreign.status()).toBe(403);
  // a token that is wrong does not spend the right one (the next request is the user's own address, which still works)
  expect((await request.get(`${base}/__start?t=${"0".repeat(64)}`, { maxRedirects: 0 })).status()).toBe(403);
  await open(page);
  // the token is spent
  const again = await request.get(address, { maxRedirects: 0 });
  expect(again.status()).toBe(403);
  // the page has the key of the session, in its own storage, and the address bar no longer holds the token or the key
  expect(page.url()).not.toContain("__start");
  expect(page.url()).not.toContain("#k=");
  expect(await page.evaluate(() => sessionStorage.getItem("rnl-local-key"))).toMatch(/^[0-9a-f]{64}$/);
  // there is no cookie: a browser sends a cookie to every port of the address, and the key must not go to those
  expect(await page.context().cookies(base)).toEqual([]);
  expect(await page.evaluate(() => document.cookie)).toBe("");
  // nothing frames the page, and what is put in its address by another site does not reach it
  const home = await request.get(`${base}/?shell=local`);
  expect(home.headers()["content-security-policy"]).toContain("frame-ancestors 'none'");
  const stripped = await request.get(`${base}/?shell=local&url=https://evil.example/x.sp`, { maxRedirects: 0 });
  expect([stripped.status(), stripped.headers().location]).toEqual([302, "/?shell=local"]);
});

test("the page opens the netlist named on the command line, and the files it includes beside it and above it, not those beyond", async ({ page }) => {
  await open(page);
  const doc = await document(page);
  // (the Document names the entry by its path, and the files it holds by their paths below its folder)
  expect(doc.path.replaceAll("\\", "/")).toMatch(/proj\/sim\/top\.sp$/);
  await expect(page.locator("#canvas > svg")).toBeVisible();
  const names = doc.files.map((f) => f.name);
  expect(names.join(" ")).toContain("extra.inc");
  expect(names.join(" ")).toContain("m.inc");
  expect(names.join(" ")).not.toContain("outside.inc");
  // the one that was refused is said as a missing include, not served
  expect(JSON.stringify(doc.diagnostics) + JSON.stringify(doc.missing)).toContain("outside.inc");
});

test("File > Save writes the file where it is, and a change on disk is told and asked about", async ({ page }) => {
  await open(page);
  const top = join(work, "proj", "sim", "top.sp");
  await page.evaluate(() => window.viewer.command("toggle-edit"));
  const area = page.locator("#source .editor:not([hidden]) textarea").first();
  await area.fill(`${await area.inputValue()}* saved by the page\n`);
  await page.locator("#apply-edits").click();
  await page.waitForFunction(() => !document.body.classList.contains("busy"));
  await page.evaluate(() => window.viewer.command("save"));
  await expect.poll(() => readFileSync(top, "utf8")).toContain("* saved by the page");
  await expect(page.locator("#note")).toContainText("Saved");
  expect((await page.evaluate(() => window.viewer.edits)).unsaved).toEqual([]);
  // another program changes the file: the page says so, within seconds
  writeFileSync(top, readFileSync(top, "utf8") + "* changed elsewhere\n");
  await expect(page.locator("#note")).toContainText("changed on disk", { timeout: 15000 });
  // a second edit, and Save: the file is not what was read, so it asks, and nothing is written until the user says
  await page.evaluate(() => window.viewer.command("toggle-edit"));
  await area.fill(`${await area.inputValue()}* second\n`);
  await page.locator("#apply-edits").click();
  await page.waitForFunction(() => !document.body.classList.contains("busy"));
  await page.evaluate(() => window.viewer.command("save"));
  await expect(page.locator("dialog.dialog .dialog-title")).toHaveText("File Changed on Disk");
  expect(readFileSync(top, "utf8")).not.toContain("* second");
  await page.getByRole("button", { name: "Overwrite" }).click();
  await expect.poll(() => readFileSync(top, "utf8")).toContain("* second");
});

test("Solve runs in the program and shows its preview", async ({ page }) => {
  await open(page);
  await page.evaluate(() => window.viewer.command("solve"));
  await page.waitForFunction(() => document.body.classList.contains("preview") && !document.body.classList.contains("solving"), null, { timeout: 120000 });
  await expect(page.locator("#preview-badge")).toBeVisible();
});

test("Solve tells the page the events of the solve before its result, and solves the circuit shown by its title", async ({ page }) => {
  // (what the page is told is read from a copy of its answer: the page reads the answer itself, and it is gone from the protocol)
  await page.addInitScript(() => {
    const fetched = window.fetch;
    window.fetch = async (input, init) => {
      const response = await fetched(input, init);
      if (String(input).endsWith("/rnl-solver/solve")) {
        window.__solve = { request: JSON.parse(init.body), type: response.headers.get("Content-Type"), text: response.clone().text() };
      }
      return response;
    };
  });
  await open(page);
  await page.evaluate(() => window.viewer.command("solve"));
  await page.waitForFunction(() => document.body.classList.contains("preview") && !document.body.classList.contains("solving"), null, { timeout: 120000 });
  const solve = await page.evaluate(async () => ({ ...window.__solve, text: await window.__solve.text }));
  expect(solve.type).toMatch(/^application\/x-ndjson/);
  const lines = solve.text.trim().split(/\r?\n/).map((line) => JSON.parse(line));
  expect(lines[0].event).toBe("run");
  expect(lines.some((l) => l.event === "level-done")).toBe(true);
  expect(lines.at(-1).result.document.levels.length).toBeGreaterThan(0);
  // the request names the circuit shown, which the program solves alone
  expect(solve.request.stream).toBe(true);
  expect(typeof solve.request.level).toBe("string");
  expect(lines[0].levels).toBe(1);
});

test("Solve > Settings draws its dialog from the program's table of the solver's settings, and the program checks them", async ({ page }) => {
  await open(page);
  await page.evaluate(() => window.viewer.command("solve-settings"));
  const dialog = page.locator("dialog.dialog");
  await expect(dialog.locator(".dialog-title")).toHaveText("Solver Settings");
  await expect(dialog.locator("fieldset")).not.toHaveCount(0);
  await expect(dialog.getByLabel("layout", { exact: true }).locator("option")).toHaveText(["rows", "tree", "lines"]);
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await expect(dialog).toHaveCount(0);
  // the program's own check says which setting it refuses, and does not call a check that found errors a failure
  const checked = await page.evaluate(async () => {
    const response = await fetch("rnl-solver/check", { method: "POST", headers: { "Content-Type": "application/json", "X-RNL": "1" }, body: JSON.stringify({ config: "beam=0" }) });
    return { status: response.status, json: await response.json() };
  });
  expect(checked.status).toBe(200);
  expect(checked.json.ok).toBe(false);
  expect(checked.json.levels[0].errors[0].key).toBe("beam");
});

test("the chooser lists the machine's files, goes up and in, and opens the one chosen", async ({ page }) => {
  await open(page);
  await page.evaluate(() => window.viewer.command("open"));
  const dialog = page.locator("dialog.dialog");
  await expect(dialog.locator(".dialog-title")).toHaveText("Open Netlist");
  await expect(dialog).toContainText("sim");
  // the folder shown, its files and the way up; not the browser's own chooser, as on the desktop: the files are the machine's
  const options = dialog.locator(".dialog-option");
  await expect(options.filter({ hasText: "other.sp" })).toHaveCount(1);
  await expect(options.filter({ hasText: "Parent folder" })).toHaveCount(1);
  await expect(options.filter({ hasText: "From the computer of the browser" })).toHaveCount(0);
  // up, and down into models
  await options.filter({ hasText: "Parent folder" }).click();
  await dialog.getByRole("button", { name: "Open" }).click();
  await expect(options.filter({ hasText: "models/" })).toHaveCount(1);
  await options.filter({ hasText: "models/" }).dblclick();
  await expect(options.filter({ hasText: "m.inc" })).toHaveCount(1);
  await options.filter({ hasText: "Parent folder" }).click();
  await dialog.getByRole("button", { name: "Open" }).click();
  await options.filter({ hasText: "sim/" }).dblclick();
  await options.filter({ hasText: "other.sp" }).dblclick();
  await page.waitForFunction(() => window.viewer.doc?.path.endsWith("other.sp"));
  expect((await document(page)).path.replaceAll("\\", "/")).toMatch(/proj\/sim\/other\.sp$/);
});

test("Add Folder to PATH (with Sub-folders) chooses a folder of the machine, whose files the Library lists and opens, and no other folder is listed", async ({ page }) => {
  mkdirSync(join(work, "lib", "cells"), { recursive: true });
  mkdirSync(join(work, "lib", ".hidden"), { recursive: true });
  writeFileSync(join(work, "lib", "amp.sp"), "* An amplifier\nR5 in out 5k\n.end\n");
  writeFileSync(join(work, "lib", "cells", "inv.inc"), "* An inverter\n");
  writeFileSync(join(work, "lib", ".hidden", "secret.inc"), "* hidden\n");
  writeFileSync(join(work, "lib", "notes.txt"), "not a netlist\n");
  await open(page);
  const lib = join(work, "lib").replaceAll("\\", "/");
  const pagePath = (p) => (process.platform === "win32" ? `/${p}` : p);
  const listed = (dir, subfolders) => page.evaluate(async ({ dir, subfolders }) => {
    const response = await fetch(`api/path?${new URLSearchParams({ dir, ...(subfolders ? { subfolders: "1" } : {}) })}`, { headers: { "X-RNL": "1" } });
    return response.status;
  }, { dir, subfolders });
  // before it is PATH, the program lists it to no one
  expect(await listed(pagePath(lib), false)).toBe(403);
  // the chooser starts in the folder of the netlist: up twice, into lib, and that folder chosen
  await page.evaluate(() => window.viewer.command("add-path-subfolders"));
  const dialog = page.locator("dialog.dialog");
  const options = dialog.locator(".dialog-option");
  await expect(dialog.locator(".dialog-title")).toHaveText("Choose Folder");
  for (let i = 0; i < 2; i++) {
    await options.filter({ hasText: "Parent folder" }).click();
    await dialog.getByRole("button", { name: "Choose" }).click();
  }
  await options.filter({ hasText: "lib/" }).dblclick();
  await options.filter({ hasText: "Choose this folder" }).click();
  await dialog.getByRole("button", { name: "Choose" }).click();
  await expect.poll(() => JSON.parse(readFileSync(join(work, "settings.json"), "utf8"))["search-paths-subfolders"]).toEqual([pagePath(lib)]);
  expect(JSON.parse(readFileSync(join(work, "settings.json"), "utf8"))["search-paths"]).toEqual([pagePath(lib)]);
  // the Library lists its netlists, those of its sub-folders too, nothing hidden and nothing else
  await page.locator("#tab-files").click();
  const group = page.locator(`.lib-group[data-group=${JSON.stringify(`path:${pagePath(lib)}`)}]`);
  await expect(group.locator(".lib-deep")).toHaveText("with sub-folders");
  // (showing the Library scans PATH again: its rows are made again when that ends)
  await expect(page.locator("#library-groups")).not.toHaveAttribute("aria-busy", "true");
  await group.locator(".lib-toggle").click();
  await expect(group.locator(".pool-open")).toHaveText(["amp.sp", "cells/inv.inc"]);
  // a file of it opens, read from the machine
  await group.locator(".pool-open", { hasText: "amp.sp" }).click();
  await page.waitForFunction(() => window.viewer.doc?.path.endsWith("amp.sp"));
  expect((await document(page)).files[0].text).toContain("R5 in out 5k");
  // the program lists that folder, and no other: not the one above it, not one below it, not the same written otherwise
  expect(await listed(pagePath(lib), true)).toBe(200);
  expect(await listed(pagePath(join(work, "proj").replaceAll("\\", "/")), false)).toBe(403);
  expect(await listed(pagePath(work.replaceAll("\\", "/")), false)).toBe(403);
  expect(await listed(`${pagePath(lib)}/cells`, false)).toBe(403);
  expect(await listed(`${pagePath(lib)}/../proj`, false)).toBe(403);
  // and it is still PATH after the program is started again
  server.kill();
  ({ child: server, base, address } = await start(join(work, "proj", "sim", "top.sp")));
  await open(page);
  await page.locator("#tab-files").click();
  await expect(group.locator(".lib-deep")).toHaveText("with sub-folders");
  await expect(page.locator("#library-groups")).not.toHaveAttribute("aria-busy", "true");
  await group.locator(".lib-toggle").click();
  await expect(group.locator(".pool-open")).toHaveText(["amp.sp", "cells/inv.inc"]);
});

test("the settings are kept in a file of the user's own, and survive the program", async ({ page }) => {
  await open(page);
  await page.evaluate(() => window.viewer.command("toggle-currents"));
  await expect.poll(() => (existsSync(join(work, "settings.json")) ? JSON.parse(readFileSync(join(work, "settings.json"), "utf8"))["currents-shown"] : null)).toBe(true);
  server.kill();
  ({ child: server, base, address } = await start(join(work, "proj", "sim", "top.sp")));
  await open(page);
  expect(await page.evaluate(() => window.viewer.checked("toggle-currents"))).toBe(true);
});

test("a page of another origin cannot read or change anything, though it knows the key it is not given", async ({ page, request }) => {
  await open(page);
  const key = await page.evaluate(() => sessionStorage.getItem("rnl-local-key"));
  const header = { "X-RNL-Key": key, "Content-Type": "application/json" };
  const body = JSON.stringify({ key: "x", value: 1 });
  // a cookie, or a wrong key, is nothing: what a page of another local port has is a cookie at most
  expect((await request.get(`${base}/api/session`, { headers: { Cookie: `rnl_session=${key}` } })).status()).toBe(403);
  expect((await request.get(`${base}/api/session`, { headers: { "X-RNL-Key": "0".repeat(64) } })).status()).toBe(403);
  // the files are served with the key too, and not without it
  const file = `${base}/f${join(work, "proj", "sim", "other.sp").replaceAll("\\", "/").replace(/^([^/])/, "/$1")}`;
  expect((await request.get(file)).status()).toBe(403);
  expect((await request.get(file, { headers: { "X-RNL-Key": key } })).status()).toBe(200);
  // no X-RNL header: a change is refused though the key is right
  expect((await request.put(`${base}/api/settings`, { headers: header, data: body })).status()).toBe(403);
  // the header, but another origin
  expect((await request.put(`${base}/api/settings`, { headers: { ...header, "X-RNL": "1", Origin: "http://127.0.0.1:1" }, data: body })).status()).toBe(403);
  expect((await request.put(`${base}/api/settings`, { headers: { ...header, "X-RNL": "1" }, data: body })).status()).toBe(200);
});

test("a request is refused from its head, without waiting for the body it announces", async ({ request }) => {
  // 400 MB announced, none sent: refused at once, as nothing of the user's is served to one with no key
  const net = await import("node:net");
  const port = Number(new URL(base).port);
  const answer = await new Promise((ok, fail) => {
    const socket = net.connect(port, "127.0.0.1", () => {
      socket.write(`POST /api/save HTTP/1.1\r\nHost: 127.0.0.1:${port}\r\nX-RNL: 1\r\nContent-Type: application/json\r\nContent-Length: 400000000\r\n\r\n`);
    });
    let text = "";
    socket.on("data", (chunk) => {
      text += chunk;
      if (text.includes("\r\n\r\n")) {
        socket.destroy();
        ok(text.split("\r\n")[0]);
      }
    });
    socket.on("error", fail);
    setTimeout(() => fail(new Error("no answer: the program waited for the body")), 5000);
  });
  expect(answer).toContain("403");
  // and the program goes on serving
  expect((await request.get(`${base}/`, { maxRedirects: 0 })).status()).toBe(302);
});

test("Save As asks before it replaces a file, and nothing is written until the user says", async ({ page }) => {
  await open(page);
  const other = join(work, "proj", "sim", "other.sp");
  const before = readFileSync(other, "utf8");
  const dialog = page.locator("dialog.dialog");
  const saveAs = async () => {
    await page.evaluate(() => window.viewer.command("save-as"));
    await dialog.locator(".dialog-option", { hasText: "Save in this folder" }).click();
    await dialog.getByRole("button", { name: "Save" }).click();
    await dialog.locator("input").fill("other.sp");
    await dialog.getByRole("button", { name: "Save" }).click();
  };
  await saveAs();
  await expect(dialog).toContainText("other.sp exists already");
  expect(readFileSync(other, "utf8")).toBe(before);
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await expect(dialog).toHaveCount(0);
  expect(readFileSync(other, "utf8")).toBe(before);
  // again, and this time the user says to replace it
  await saveAs();
  await dialog.getByRole("button", { name: "Replace" }).click();
  await expect.poll(() => readFileSync(other, "utf8")).toContain("A top level");
});

test("the page is the desktop viewer: none of the web's own commands, and a file dropped is not opened", async ({ page }) => {
  await open(page);
  const offered = (id) => page.locator(`#menubar .menu-item[data-id="${id}"]`);
  for (const id of ["open-url", "load-manifest", "download-manifest", "include-library", "desktop-viewer"]) await expect(offered(id)).toHaveCount(0);
  for (const id of ["save", "save-as", "save-project", "add-path-subfolders"]) await expect(offered(id)).not.toHaveCount(0);
  await expect(page.locator("#include-library")).toBeHidden();
  const before = (await document(page)).path;
  // a browser gives a dropped file without its path: the page says so, and the document shown stays
  const files = await page.evaluateHandle(() => {
    const data = new DataTransfer();
    data.items.add(new File(["* dropped\nR1 a 0 1k\n.end\n"], "dropped.sp"));
    return data;
  });
  await page.dispatchEvent("body", "drop", { dataTransfer: files });
  await expect(page.locator("dialog.dialog")).toContainText("A browser does not tell the page where a dropped file is.");
  expect((await document(page)).path).toBe(before);
});

test("New Window opens the file of the machine in a new tab, with the changes applied and not saved", async ({ page, context }) => {
  await open(page);
  await page.evaluate(() => window.viewer.command("toggle-edit"));
  const area = page.locator("#source .editor:not([hidden]) textarea").first();
  await area.fill(`${await area.inputValue()}* applied here\n`);
  await page.locator("#apply-edits").click();
  await page.waitForFunction(() => !document.body.classList.contains("busy"));
  const opened = context.waitForEvent("page");
  await page.evaluate(() => { void window.viewer.command("new-window"); });
  const other = await opened;
  await other.waitForFunction(() => window.viewer?.doc && !document.body.classList.contains("busy") && window.viewer.edits.unsaved.length > 0, null, { timeout: 90000 });
  expect((await document(other)).path).toBe((await document(page)).path);
  expect(await other.evaluate(() => window.viewer.edits)).toEqual(await page.evaluate(() => window.viewer.edits));
  expect(await other.evaluate(() => window.viewer.doc.files[0].text)).toContain("* applied here");
  // the file on disk is as it was: nothing was saved
  expect(readFileSync(join(work, "proj", "sim", "top.sp"), "utf8")).not.toContain("* applied here");
});

test("PATH is searched as the desktop searches it: folder by folder, a bare include in each folder itself, not in its version folders", async ({ page }) => {
  server.kill();
  // the first PATH folder has part.inc only in a version folder; the second has it itself
  mkdirSync(join(work, "pa", "1.0"), { recursive: true });
  mkdirSync(join(work, "pb"), { recursive: true });
  writeFileSync(join(work, "pa", "1.0", "part.inc"), "* in a version folder\nR1 a 0 1k\n");
  writeFileSync(join(work, "pb", "part.inc"), "* in the folder\nR2 a 0 2k\n");
  writeFileSync(join(work, "proj", "sim", "uses.sp"), "* uses\n.include part.inc\n.end\n");
  const pagePath = (p) => (process.platform === "win32" ? `/${p.replaceAll("\\", "/")}` : p);
  writeFileSync(join(work, "settings.json"), JSON.stringify({ "search-paths": [pagePath(join(work, "pa")), pagePath(join(work, "pb"))] }));
  ({ child: server, base, address } = await start(join(work, "proj", "sim", "uses.sp")));
  await open(page);
  // (the netlist is read again once PATH is listed)
  await page.locator("#tab-files").click();
  await expect(page.locator("#library-groups")).not.toHaveAttribute("aria-busy", "true");
  await page.waitForFunction(() => window.viewer.doc?.files.length > 1 && !document.body.classList.contains("busy"));
  const read = (await document(page)).files.map((f) => f.path);
  expect(read.some((p) => p.endsWith("pb/part.inc"))).toBe(true);
  expect(read.some((p) => p.endsWith("1.0/part.inc"))).toBe(false);
});

test("Save Project holds the files of a PATH folder, read from the machine", async ({ page }) => {
  server.kill();
  mkdirSync(join(work, "lib"), { recursive: true });
  writeFileSync(join(work, "lib", "amp.sp"), "* An amplifier\nR5 in out 5k\n.end\n");
  const lib = join(work, "lib").replaceAll("\\", "/");
  const pagePath = (p) => (process.platform === "win32" ? `/${p}` : p);
  writeFileSync(join(work, "settings.json"), JSON.stringify({ "search-paths": [pagePath(lib)] }));
  ({ child: server, base, address } = await start(join(work, "proj", "sim", "top.sp")));
  await open(page);
  await page.locator("#tab-files").click();
  await expect(page.locator("#library-groups")).not.toHaveAttribute("aria-busy", "true");
  // (the archive is still saved by the browser here: its own save dialog, where it has one, is not one a test can answer)
  await page.evaluate(() => { window.showSaveFilePicker = undefined; });
  const download = page.waitForEvent("download");
  // the command waits for the dialog of what the archive holds, which is answered as the desktop's is
  await page.evaluate(() => { void window.viewer.command("save-project"); });
  await page.locator("dialog.dialog").getByRole("button", { name: "Save", exact: true }).click();
  const zip = readFileSync(await (await download).path(), "latin1");
  expect(zip.startsWith("PK")).toBe(true);
  expect(zip).toContain("PATH/1-lib/amp.sp");
  expect(zip).toContain("R5 in out 5k");
});

test("the format chosen for a file of the machine is kept for the next start", async ({ page }) => {
  await open(page);
  const top = (await document(page)).path;
  await page.evaluate((path) => import("./formats.js").then((f) => f.chooseFormat(path, "spectre")), top);
  await expect.poll(() => JSON.parse(readFileSync(join(work, "settings.json"), "utf8"))["file-formats"] ?? {}).toEqual({ [top]: "spectre" });
});
