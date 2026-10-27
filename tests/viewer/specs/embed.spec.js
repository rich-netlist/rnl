// Real iframe, postMessage transport and WebAssembly core; Chromium and WebKit only.
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { createHash } from "node:crypto";
import { expect, test } from "@playwright/test";
import { publicPackagesAvailable, publicPackageText } from "../../../scripts/packages.mjs";
import { zh } from "../zh.js";

test.skip(!process.env.RNL_WEB_URL, "the site is not built: RNL_WEB=0");
const base = process.env.RNL_WEB_URL;
const hierarchy = "samples/cases/04-hierarchy.sp";
const rc = "samples/cases/01-rc-lowpass.sp";
const text = (name) => `* ${name}\nR1 in out 1k\n* @RNL { #R1 @PLACE(0, 0) }\n.end\n`;
const review = process.env.RNL_EMBED_REVIEW_DIR;
const servers = [];

test.afterEach(async () => {
  for (const server of servers.splice(0)) {
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
  }
});

async function start(page, params = {}, { hostOrigin, seed = {} } = {}) {
  const url = new URL("/", base);
  url.search = new URLSearchParams({ embed: "1", ...params });
  const html = `
    <!doctype html><html><head><style>body{margin:0}iframe{width:900px;height:600px;border:0}#outer{height:2400px}</style></head><body>
    <button id="outside">Outer page</button><div id="mount"></div><div id="outer"></div>
    <script>
      window.messages = [];
      window.addEventListener('message', (event) => {
        if (event.data?.rnl === 1) window.messages.push({ ...event.data, origin: event.origin });
      });
      for (const [key, value] of Object.entries(${JSON.stringify(seed)})) localStorage.setItem(key, JSON.stringify(value));
      window.storedBefore = JSON.stringify({...localStorage});
      const frame = document.createElement('iframe'); frame.id = 'viewer'; frame.title = 'Embedded viewer';
      frame.src = ${JSON.stringify(url.href)}; document.getElementById('mount').append(frame);
    </script></body></html>`;
  if (hostOrigin) {
    // Serve the cross-origin parent over real loopback HTTP. A fulfilled main-document route
    // has no IP address, which Chromium can classify as public and block from accessing loopback.
    const server = http.createServer((_, response) => {
      response.writeHead(200, { "Content-Type": "text/html" });
      response.end(html);
    });
    servers.push(server);
    await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
    hostOrigin = `http://127.0.0.1:${server.address().port}`;
  } else await page.route("**/embed-test-host", (route) => route.fulfill({ contentType: "text/html", body: html }));
  await page.addInitScript(() => {
    if (new URLSearchParams(location.search).get("embed") !== "1") return;
    window.storageCalls = [];
    for (const method of ["getItem", "setItem", "removeItem", "clear"]) {
      const original = Storage.prototype[method];
      Storage.prototype[method] = function (...args) {
        window.storageCalls.push([method, ...args]);
        return original.apply(this, args);
      };
    }
  });
  await page.goto(`${hostOrigin || base}/embed-test-host`);
  await waitMessage(page, "ready");
  const frame = page.frames().find((item) => item.url().includes("embed=1"));
  expect(frame).toBeTruthy();
  return frame;
}

async function messages(page, type) {
  return page.evaluate((type) => window.messages.filter((item) => item.type === type), type);
}

async function waitMessage(page, type, count = 1) {
  await expect.poll(async () => (await messages(page, type)).length).toBe(count);
  return (await messages(page, type)).at(-1);
}

async function send(page, message) {
  await page.evaluate((message) => document.getElementById("viewer").contentWindow.postMessage(message, "*"), { rnl: 1, ...message });
}

const viewBox = (frame) => frame.locator("#canvas > svg").getAttribute("viewBox");
test("the shared API advertises optional fields and correlates explicit package imports", async ({ page }) => {
  await start(page);
  expect((await messages(page, "ready"))[0].features).toEqual(expect.arrayContaining(["requestId", "packages"]));
  await send(page, { type: "open", requestId: "canvas", name: "canvas.sp",
    text: fs.readFileSync(new URL("../inputs/missing-canvas-package.sp", import.meta.url), "utf8"),
    packages: [{ name: "analog-canvas-extensions", version: "0.1" }] });
  expect(await waitMessage(page, "loaded")).toMatchObject({ requestId: "canvas", errors: 0 });
});

test("the first open uses the core warmed before ready and keeps only one standby", async ({ page }) => {
  await start(page);
  const warmed = page.workers().find(worker => worker.url().endsWith("/core/worker.js"));
  expect(warmed).toBeTruthy();
  await page.evaluate(text => document.querySelector('iframe').contentWindow.postMessage({ rnl: 1, type: "open", name: "warm.sp", text }, location.origin), text("warm"));
  await waitMessage(page, "loaded");
  expect(await warmed.evaluate(() => location.pathname)).toContain("/core/worker.js");
  await expect.poll(() => page.workers().length).toBe(2);
});

test("initialization failures still notify an embedded caller before ready", async ({ page }) => {
  const frame = await start(page);
  await page.route("**/about.json", route => route.fulfill({ status: 503, body: "unavailable" }));
  await frame.goto(`${base}/?embed=1`);
  const error = await waitMessage(page, "error");
  expect(error.message).toContain("The viewer version could not be read (503)");
  await expect(frame.locator("#empty")).toHaveText(error.message);
  expect(await messages(page, "ready")).toHaveLength(1);
});
const width = async (frame) => Number((await viewBox(frame)).split(" ")[2]);

async function screenshot(page, name, browserName) {
  if (!review || browserName !== "chromium") return;
  fs.mkdirSync(review, { recursive: true });
  const target = name.startsWith("embed-") ? page.locator("#viewer") : page;
  await target.screenshot({ path: path.join(review, name) });
}

test("URL open fills the iframe, hides all chrome and provides working controls", async ({ page, browserName }) => {
  const frame = await start(page, { url: hierarchy });
  const loaded = await waitMessage(page, "loaded");
  expect(loaded).toMatchObject({ rnl: 1, name: "04-hierarchy.sp", level: "04-hierarchy.sp", errors: 0 });
  expect(loaded.levels).toContain("amp");
  expect(typeof loaded.warnings).toBe("number");
  expect((await messages(page, "ready"))[0].version).toMatch(/^\d+\.\d+\.\d+/);
  for (const selector of ["#menubar", "#sidebar", ".statusbar", ".level-bar"]) await expect(frame.locator(selector)).toBeHidden();
  await expect(frame.locator('input[type="file"]')).toHaveCount(0);
  await expect(frame.locator("#canvas > svg")).toBeVisible();
  await expect(frame.locator("#embed-path")).toBeHidden();
  expect(await frame.locator("#canvas").boundingBox()).toMatchObject({ width: 900, height: 600 });
  await expect(frame.locator(".embed-toolbar button")).toHaveCount(3);
  const fitted = await width(frame);
  await frame.locator("#embed-zoom-in").click();
  expect(await width(frame)).toBeLessThan(fitted);
  await frame.locator("#embed-zoom-out").click();
  expect(await width(frame)).toBeCloseTo(fitted);
  await frame.locator("#embed-zoom-in").click();
  await frame.locator("#embed-fit").click();
  expect(await width(frame)).toBeCloseTo(fitted);
  const button = frame.locator("#embed-fit");
  expect(await button.boundingBox()).toMatchObject({ width: 28, height: 28 });
  await expect(button).toHaveAttribute("title", "Fit to window");
  const link = new URL(await frame.locator("#embed-open").getAttribute("href"));
  expect(link.searchParams.has("embed")).toBe(false);
  expect(link.searchParams.get("url")).toBe(hierarchy);
  await screenshot(page, "embed-light.png", browserName);
});

test("Shift+E opens an embedded subcircuit in a full viewer and keeps the iframe unchanged", async ({ page }) => {
  const frame = await start(page, { url: hierarchy });
  await waitMessage(page, "loaded");
  await frame.evaluate(async () => {
    const { select } = await import('./selection.js');
    select(window.viewer.doc.objects.findIndex((o) => o.name === 'X2'), 'drawing');
  });
  const before = await frame.evaluate(() => ({ path: window.viewer.path, selected: window.viewer.selected }));
  await frame.locator("#canvas").focus();
  const popup = page.context().waitForEvent("page");
  await page.keyboard.press("Shift+e");
  const child = await popup;
  try {
    await expect(child.locator("#level-name")).toHaveText("amp");
    await expect(child.locator("#menubar")).toBeVisible();
    expect(new URL(child.url()).searchParams.has("embed")).toBe(false);
    expect(await frame.evaluate(() => ({ path: window.viewer.path, selected: window.viewer.selected }))).toEqual(before);
  } finally { await child.close(); }
});

test("URL preferences are isolated from existing storage and the OS theme", async ({ page, browserName }) => {
  await page.emulateMedia({ colorScheme: "light" });
  const frame = await start(page, { url: hierarchy, level: "amp", background: "dark", grid: "lines", names: "0" }, {
    seed: { background: "light", "grid-shown": false, "hover-names": true, "sidebar-shown": true },
  });
  expect((await waitMessage(page, "loaded")).level).toBe("amp");
  await expect(frame.locator("body")).toHaveAttribute("data-background", "dark");
  expect(await frame.evaluate(() => window.viewer.grid)).toMatchObject({ shown: true, style: "lines" });
  expect(await frame.evaluate(() => window.viewer.checked("toggle-hover-names"))).toBe(false);
  await expect(frame.locator("#embed-path")).toHaveText("amp");
  expect(await frame.locator("#embed-fit").evaluate((el) => getComputedStyle(el).backgroundColor)).toBe("rgb(35, 35, 39)");
  await screenshot(page, "embed-dark-subcircuit.png", browserName);
  await send(page, { type: "level", name: "04-hierarchy.sp" });
  expect((await waitMessage(page, "level", 2)).name).toBe("04-hierarchy.sp");
  await send(page, { type: "background", value: "light" });
  await expect(frame.locator("body")).toHaveAttribute("data-background", "light");
  expect(await frame.evaluate(() => window.storageCalls)).toEqual([]);
  expect(await page.evaluate(() => JSON.stringify({ ...localStorage }) === window.storedBefore)).toBe(true);
});

test("omitted preferences use embed defaults, dots and names=1 are supported", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  const frame = await start(page, {}, { seed: { background: "dark", "grid-shown": true, "hover-names": false } });
  await expect(frame.locator("body")).toHaveAttribute("data-background", "light");
  expect(await frame.evaluate(() => window.viewer.grid.shown)).toBe(false);
  expect(await frame.evaluate(() => window.viewer.checked("toggle-hover-names"))).toBe(true);
  expect(await frame.locator("#embed-fit").evaluate((el) => getComputedStyle(el).backgroundColor)).toBe("rgb(251, 251, 250)");
  expect(await frame.evaluate(() => window.storageCalls)).toEqual([]);
  await frame.goto(`${base}/?embed=1&grid=dots&names=1&background=light`);
  await waitMessage(page, "ready", 2);
  expect(await frame.evaluate(() => window.viewer.grid)).toMatchObject({ shown: true, style: "dots" });
  expect(await frame.evaluate(() => window.viewer.checked("toggle-hover-names"))).toBe(true);
});

test("text and URL opens replace the drawing without reloading; fit and level messages work", async ({ page }) => {
  const frame = await start(page);
  await frame.evaluate(() => { window.pageMarker = "still here"; });
  await send(page, { type: "open", name: "inline.sp", text: text("inline") });
  expect(await waitMessage(page, "loaded")).toMatchObject({ name: "inline.sp", levels: ["inline.sp"], errors: 0, warnings: 0 });
  await expect(frame.locator("#embed-open")).toBeHidden();
  const first = await frame.locator("#canvas > svg").innerHTML();
  await send(page, { type: "open", url: hierarchy });
  await waitMessage(page, "loaded", 2);
  expect(await frame.locator("#canvas > svg").innerHTML()).not.toBe(first);
  await expect(frame.locator("#embed-open")).toBeVisible();
  await send(page, { type: "level", name: "amp" });
  await expect(frame.locator("#embed-path")).toHaveText("amp");
  const fitted = await width(frame);
  await frame.locator("#embed-zoom-in").click();
  await send(page, { type: "fit" });
  await expect.poll(() => width(frame)).toBeCloseTo(fitted);
  expect(await frame.evaluate(() => window.pageMarker)).toBe("still here");
  expect((await messages(page, "ready")).length).toBe(1);
  const fields = Object.keys((await messages(page, "loaded"))[1]).sort();
  expect(fields).toEqual(["errors", "level", "levels", "name", "origin", "rnl", "type", "warnings"]);
});

test("double-click descent and Backspace report level changes", async ({ page }) => {
  const frame = await start(page, { url: hierarchy });
  await waitMessage(page, "loaded");
  const point = await frame.evaluate(() => {
    const doc = window.viewer.doc;
    const hit = doc.levels[0].hits.find((hit) => hit.kind === "box" && hit.objects.some((o) => doc.objects[o].name === "X1"));
    const [a, b = a] = hit.points;
    const [x, y, w] = document.querySelector("#canvas > svg").getAttribute("viewBox").split(" ").map(Number);
    const canvas = document.getElementById("canvas");
    const k = canvas.clientWidth / w;
    return { x: ((a[0] + b[0]) / 2 - x) * k, y: ((a[1] + b[1]) / 2 - y) * k };
  });
  await frame.locator("#canvas").dblclick({ position: point });
  expect((await waitMessage(page, "level", 2)).name).toBe("amp");
  await page.keyboard.press("Backspace");
  expect((await waitMessage(page, "level", 3)).name).toBe("04-hierarchy.sp");
});

test("origin restricts actual cross-origin senders, and missing rnl markers are ignored", async ({ page }) => {
  const frame = await start(page, { url: hierarchy, origin: new URL(base).origin });
  await waitMessage(page, "loaded");
  const otherOrigin = base.replace("127.0.0.1", "localhost");
  await page.route("**/embed-intruder", (route) => route.fulfill({ contentType: "text/html", body: `
    <script>parent.frames[0].postMessage({rnl:1,type:'background',value:'dark'}, '*'); parent.postMessage('intruder-sent', '*');</script>` }));
  await page.evaluate((origin) => {
    window.intruderSent = false;
    window.addEventListener("message", (e) => { if (e.data === "intruder-sent") window.intruderSent = true; });
    const intruder = document.createElement("iframe");
    intruder.src = `${origin}/embed-intruder`;
    document.body.append(intruder);
  }, otherOrigin);
  await page.waitForFunction(() => window.intruderSent);
  await page.evaluate(() => document.getElementById("viewer").contentWindow.postMessage({ type: "background", value: "dark" }, "*"));
  await send(page, { type: "level", name: "amp" });
  await waitMessage(page, "level", 2);
  await expect(frame.locator("body")).toHaveAttribute("data-background", "light");
  await send(page, { type: "background", value: "dark" });
  await expect(frame.locator("body")).toHaveAttribute("data-background", "dark");
});

test("without origin, a parent on a different origin can open text", async ({ page }) => {
  const other = new URL(base);
  other.port = String(Number(other.port) + 1);
  const frame = await start(page, {}, { hostOrigin: other.origin });
  await send(page, { type: "open", name: "cross-origin.sp", text: text("cross origin") });
  expect((await waitMessage(page, "loaded")).name).toBe("cross-origin.sp");
  await expect(frame.locator("#canvas > svg")).toBeVisible();
});

test("bad netlists and failed URLs report errors on the canvas without dialogs, then recover", async ({ page }) => {
  const frame = await start(page);
  await send(page, { type: "open", name: "bad.sp", text: '* Bad\n* @RNL { @THIS-DOES-NOT-EXIST }\n.end\n' });
  const failure = await waitMessage(page, "error");
  expect(failure.message).toContain("Could not open the netlist:");
  await expect(frame.locator("#empty")).toHaveText(failure.message);
  await expect(frame.locator("dialog")).toHaveCount(0);
  await send(page, { type: "open", url: "samples/missing.sp" });
  expect((await waitMessage(page, "error", 2)).message).toContain("404");
  await send(page, { type: "open", url: rc });
  await waitMessage(page, "loaded");
  await expect(frame.locator("#empty")).toBeHidden();
});

for (const fails of [false, true]) {
  test(`the latest open wins over a late ${fails ? "failure" : "success"}`, async ({ page }) => {
    const frame = await start(page);
    await frame.evaluate(async () => {
      const { shell } = await import("./shell.js");
      const build = shell.build;
      window.finishedBuilds = 0;
      shell.build = async (...args) => { try { return await build(...args); } finally { window.finishedBuilds++; } };
    });
    let release;
    let started;
    const requested = new Promise((resolve) => { started = resolve; });
    const hold = new Promise((resolve) => { release = resolve; });
    await page.route("**/slow.sp", async (route) => {
      started();
      await hold;
      await route.fulfill(fails ? { status: 404, body: "missing" } : { body: text("old") });
    });
    await send(page, { type: "open", url: "slow.sp" });
    await requested;
    await send(page, { type: "open", name: "latest.sp", text: text("latest") });
    expect((await waitMessage(page, "loaded")).name).toBe("latest.sp");
    release();
    await frame.waitForFunction(() => window.finishedBuilds === 2);
    expect(await frame.evaluate(() => window.viewer.doc.name)).toBe("latest.sp");
    expect((await messages(page, "loaded")).length).toBe(1);
    expect(await messages(page, "error")).toEqual([]);
  });
}

test('a later embed open cancels an in-flight cross-file descent', async ({page}) => {
  const frame = await start(page, {url:hierarchy});
  await waitMessage(page,'loaded');
  let release, started;
  const hold=new Promise(resolve=>{release=resolve;});
  const requested=new Promise(resolve=>{started=resolve;});
  await page.route('**/04-hierarchy-lib.inc', async route=>{
    started(); await hold; await route.continue();
  });
  await frame.evaluate(async()=>{
    const {descend}=await import('./subcircuits.js');
    window.navigationFinished=false;
    void descend(window.viewer.doc.objects.findIndex(o=>o.name==='X3')).finally(()=>{window.navigationFinished=true;});
  });
  await requested;
  await send(page,{type:'open',name:'latest.sp',text:text('latest')});
  expect((await waitMessage(page,'loaded',2)).name).toBe('latest.sp');
  release();
  await frame.waitForFunction(()=>window.navigationFinished);
  expect(await frame.evaluate(()=>window.viewer.doc.name)).toBe('latest.sp');
  expect(await messages(page,'error')).toEqual([]);
});

for (const identified of [false, true]) test(`cross-file navigation supersedes a ${identified ? 'correlated' : 'legacy'} embed API import`, async ({ page }) => {
  const frame = await start(page, { url: hierarchy });
  await waitMessage(page, 'loaded');
  let release, started;
  const hold = new Promise(resolve => { release = resolve; });
  const requested = new Promise(resolve => { started = resolve; });
  await page.route('**/delayed-import.sp', async route => {
    started(); await hold; await route.fulfill({ body: text('obsolete') });
  });
  await frame.evaluate(async () => {
    const { shell } = await import('./shell.js');
    const build = shell.build;
    shell.build = async (...args) => {
      try { return await build(...args); }
      finally { if (args[0].name.endsWith('/delayed-import.sp')) window.importFinished = true; }
    };
  });
  await send(page, { type: 'open', ...(identified ? { requestId: 'obsolete' } : {}), url: 'delayed-import.sp' });
  await requested;
  await frame.evaluate(async () => {
    const { descend } = await import('./subcircuits.js');
    await descend(window.viewer.doc.objects.findIndex(o => o.name === 'X3'));
  });
  if (identified) expect(await waitMessage(page, 'error')).toMatchObject({ requestId: 'obsolete', reason: 'superseded' });
  release();
  await frame.waitForFunction(() => window.importFinished);
  expect(await frame.evaluate(() => window.viewer.doc.name)).toBe('04-hierarchy-lib.inc');
  expect(await messages(page, 'loaded')).toHaveLength(1);
  if (!identified) expect(await messages(page, 'error')).toEqual([]);
  await expect(frame.locator('#embed-path')).toContainText('rdiv');
  expect((await messages(page, 'level')).at(-1).name).toBe('rdiv');
});

test('initial cross-file level selection belongs to the embed import, in the netlist with all it includes', async ({ page }) => {
  const frame = await start(page, { url: hierarchy, level: 'rdiv' });
  expect(await waitMessage(page, 'loaded')).toMatchObject({ name: '04-hierarchy.sp', level: 'rdiv' });
  // rdiv is defined in the included file, and shown in the netlist that includes it, not in that file alone
  expect(await frame.evaluate(() => window.viewer.doc.name)).toBe('04-hierarchy.sp');
  expect(await frame.evaluate(() => window.viewer.doc.levels[window.viewer.level].title)).toBe('rdiv');
  expect(await messages(page, 'error')).toEqual([]);
  await expect(frame.locator('body')).not.toHaveClass(/busy/);
});

test('unknown levels report protocol errors without unhandled rejections', async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await start(page, { url: hierarchy });
  await waitMessage(page, 'loaded');
  await send(page, { type: 'level', name: 'missing-level' });
  expect(await waitMessage(page, 'error')).toMatchObject({ message: 'Unknown level: missing-level' });
  expect(errors).toEqual([]);
});

test("iframe resizing preserves the drawing centre and outer arrow keys scroll the parent", async ({ page }) => {
  const frame = await start(page, { url: rc });
  await waitMessage(page, "loaded");
  const centre = async () => frame.evaluate(() => {
    const [x, y, w, h] = document.querySelector("#canvas > svg").getAttribute("viewBox").split(" ").map(Number);
    return [x + w / 2, y + h / 2];
  });
  const before = await centre();
  const oldWidth = await width(frame);
  await page.locator("#viewer").evaluate((el) => { el.style.width = "650px"; el.style.height = "450px"; });
  await expect.poll(() => width(frame)).not.toBe(oldWidth);
  const after = await centre();
  expect(after[0]).toBeCloseTo(before[0], 8);
  expect(after[1]).toBeCloseTo(before[1], 8);
  const box = await viewBox(frame);
  await page.locator("#outside").focus();
  await page.keyboard.press("ArrowDown");
  await expect.poll(() => page.evaluate(() => scrollY)).toBeGreaterThan(0);
  expect(await viewBox(frame)).toBe(box);
  await frame.locator("#canvas").focus();
  await page.keyboard.press("ArrowRight");
  await expect.poll(() => viewBox(frame)).not.toBe(box);
});

test("Chinese browser language labels controls", async ({ browser }) => {
  const context = await browser.newContext({ locale: "zh-CN" });
  const page = await context.newPage();
  const frame = await start(page);
  await expect(frame.locator("#embed-fit")).toHaveAttribute("aria-label", zh("Fit to window"));
  await expect(frame.locator("#embed-zoom-in")).toHaveAttribute("title", zh("Zoom in"));
  await expect(frame.locator("#embed-zoom-out")).toHaveAttribute("title", zh("Zoom out"));
  await context.close();
});

test("ready waits for the core and embed stylesheet", async ({ page }) => {
  let release;
  let requested;
  const hold = new Promise((resolve) => { release = resolve; });
  const waiting = new Promise((resolve) => { requested = resolve; });
  await page.route("**/embed.css", async (route) => {
    requested();
    await hold;
    await route.continue();
  });
  const starting = start(page);
  await waiting;
  expect(await messages(page, "ready")).toEqual([]);
  release();
  const frame = await starting;
  await send(page, { type: "open", name: "ready.sp", text: text("ready") });
  await waitMessage(page, "loaded");
  await expect(frame.locator("#canvas > svg")).toBeVisible();
});

test("the normal web viewer keeps its UI and stored preferences with embed=0", async ({ page }) => {
  await page.goto(base);
  await expect(page.locator("#menubar")).toBeVisible();
  await page.evaluate(() => {
    for (const [key, value] of Object.entries({ background: "dark", "grid-shown": true, "grid-style": "lines", "hover-names": true })) {
      localStorage.setItem(key, JSON.stringify(value));
    }
  });
  await page.goto(`${base}/?embed=0&url=${rc}&background=light&grid=off&names=0`);
  await expect(page.locator("#canvas > svg")).toBeVisible();
  for (const selector of ["#menubar", "#sidebar", ".statusbar"]) await expect(page.locator(selector)).toBeVisible();
  await expect(page.locator(".embed-toolbar")).toHaveCount(0);
  await expect(page.locator("body")).toHaveAttribute("data-background", "dark");
  expect(await page.evaluate(() => window.viewer.grid)).toMatchObject({ shown: true, style: "lines" });
  expect(await page.evaluate(() => window.viewer.checked("toggle-hover-names"))).toBe(true);
});

test("demo switches all four published examples in its existing iframe", async ({ page, browserName }) => {
  await page.goto(`${base}/embed-demo.html`);
  await expect(page.locator("#events")).toContainText("01-rc-lowpass.sp");
  const frame = page.frames().find((item) => item.url().includes("embed=1"));
  await frame.evaluate(() => { window.pageMarker = 42; });
  await screenshot(page, "demo-before.png", browserName);
  for (const name of ["Two-stage buffer", "Line styles", "Five-transistor OTA"]) {
    await page.getByRole("button", { name, exact: true }).click();
    const sample = name === "Two-stage buffer" ? "04-hierarchy.sp" : name === "Line styles" ? "17-line-styles.sp" : "ota5t.sp";
    await expect(page.locator("#events")).toContainText(sample);
    await expect(frame.locator("#canvas > svg")).toBeVisible();
  }
  expect(await frame.evaluate(() => window.pageMarker)).toBe(42);
  await screenshot(page, "demo-after.png", browserName);
});

test("public packages: every picker record loads through the offline catalog", async ({ page }) => {
  test.skip(!publicPackagesAvailable(), "Run python scripts/packages.py download");
  await page.route("https://rich-netlist.github.io/rnl-packages/**", route => route.abort());
  await page.goto(base);
  await expect(page.locator("#menubar")).toBeVisible();
  const results = await page.evaluate(async () => {
    const { default: shell } = await import('./shells/web.js');
    const packages = await shell.publicPackages();
    const results = [];
    for (const pkg of packages) {
      const items = await shell.includePackage(pkg);
      shell.setPool(items);
      const source = { name: 'public.sp', text: `* public\n* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="${pkg.name}", version="${pkg.version}") }\nR1 a 0 1k\n.end\n` };
      const doc = await shell.build(source);
      results.push({ name: pkg.name, errors: doc.diagnostics.filter(d => d.severity === 'error') });
    }
    return results;
  });
  expect(results.map(p => p.name).sort()).toEqual(['analog-canvas-extensions', 'eda-style', 'eda-symbol', 'pdk-sky130']);
  for (const result of results) expect(result.errors, result.name).toEqual([]);
});

test("public packages: failed catalogs preserve built-ins and altered package bytes fail", async ({ page }) => {
  test.skip(!publicPackagesAvailable(), "Run python scripts/packages.py download");
  await page.route('**/public-packages/catalog.json', route => route.fulfill({ status: 503, body: 'unavailable' }));
  await page.goto(`${base}/?url=samples/cases/01-rc-lowpass.sp`);
  await page.locator('#tab-files').click();
  await expect(page.locator('#public-package-status')).toContainText('Built-in standard packages remain available');
  await page.locator('[data-group="builtin"] .lib-toggle').click();
  await expect(page.locator('[data-group="builtin"] .pool-row')).toHaveCount(25);
  await page.unroute('**/public-packages/catalog.json');
  await page.route('**/public-packages/packages/analog-canvas-extensions/**/analog-canvas-extensions.rnl', route => route.fulfill({ body: 'altered package' }));
  const error = await page.evaluate(async () => {
    const { default: shell } = await import('./shells/web.js');
    const pkg = (await shell.publicPackages()).find(p => p.name === 'analog-canvas-extensions');
    try { await shell.includePackage(pkg); } catch (error) { return String(error); }
  });
  expect(error).toContain('SHA-256 mismatch');
});

test("public packages: legacy URLs and embedded USE-PACKAGE resolve through the catalog", async ({ page }) => {
  test.skip(!publicPackagesAvailable(), "Run python scripts/packages.py download");
  await page.route("https://rich-netlist.github.io/rnl-packages/**", route => route.abort());
  await page.goto(`${base}/?url=libraries/eda-symbol/eda-symbol.rnl`);
  await page.waitForFunction(() => window.viewer?.doc && !document.body.classList.contains('busy'));
  const doc = await page.evaluate(() => window.viewer.doc);
  expect(doc.diagnostics.filter(d => d.severity === 'error')).toEqual([]);
  expect(doc.files[0].text).toContain('@PACKAGE(name="eda-symbol", version="0.1")');
  const frame = await start(page);
  const source = fs.readFileSync(new URL('../inputs/missing-canvas-package.sp', import.meta.url), 'utf8');
  await send(page, { type: 'open', text: source, name: 'canvas.sp' });
  const loaded = await waitMessage(page, 'loaded');
  expect(loaded.errors).toBe(0);
  await expect(frame.locator('#canvas > svg')).toBeVisible();
});

test("public packages: foreign library URLs and includes bypass the catalog", async ({ page }) => {
  let catalogs = 0;
  await page.route('**/public-packages/catalog.json', route => {
    catalogs++;
    return route.fulfill({ json: { schemaVersion: 1, packages: [] } });
  });
  const include = 'Rexternal a 0 2k\n';
  const requests = [];
  const server = http.createServer((request, response) => {
    requests.push(request.url);
    response.writeHead(200, { 'Content-Type': 'text/plain', 'Access-Control-Allow-Origin': '*' });
    response.end(request.url === '/libraries/x.rnl' ? include : '* external includes\n.include "../libraries/x.rnl"\nR1 a 0 1k\n.end\n');
  });
  servers.push(server);
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const foreign = `http://127.0.0.1:${server.address().port}`;
  const frame = await start(page);
  const result = await frame.evaluate(async foreign => {
    const { default: shell, sourceFromUrl } = await import('./shells/web.js');
    const opened = await sourceFromUrl(`${foreign}/libraries/x.rnl`).open(false);
    const doc = await shell.build(sourceFromUrl(`${foreign}/proj/top.sp`));
    return { opened, files: doc.files, errors: doc.diagnostics.filter(d => d.severity === 'error') };
  }, foreign);
  expect(result.opened.text).toBe(include);
  expect(result.errors).toEqual([]);
  expect(result.files).toEqual(expect.arrayContaining([expect.objectContaining({ text: expect.stringContaining('Rexternal') })]));
  expect(requests.filter(url => url === '/libraries/x.rnl')).toHaveLength(2);
  expect(catalogs).toBe(0);
});

test("public packages: only the local library root maps, and missing includes return null", async ({ page }) => {
  let catalogs = 0;
  await page.route('**/public-packages/catalog.json', route => {
    catalogs++;
    return route.fulfill({ json: { schemaVersion: 1, packages: [] } });
  });
  await page.route('**/other/libraries/x.rnl', route => route.fulfill({ body: 'Rother a 0 3k\n' }));
  const frame = await start(page);
  const ordinary = await frame.evaluate(async () => {
    const { sourceFromUrl } = await import('./shells/web.js');
    return (await sourceFromUrl('other/libraries/x.rnl').open(false)).text;
  });
  expect(ordinary).toBe('Rother a 0 3k\n');
  expect(catalogs).toBe(0);
  const missing = await frame.evaluate(async () => {
    const { sourceFromUrl } = await import('./shells/web.js');
    const source = sourceFromUrl('samples/cases/01-rc-lowpass.sp');
    await source.open(false);
    const included = await source.read('/libraries/not-published.rnl', false);
    let error;
    try { await sourceFromUrl('libraries/not-published.rnl').open(false); }
    catch (failure) { error = failure.message; }
    return { included, error };
  });
  expect(missing.included).toBeNull();
  expect(missing.error).toContain('could not be fetched: No public package matches the old address:');
  expect(catalogs).toBe(1);
});

test("public packages: versions coexist at their canonical catalog paths", async ({ page }) => {
  test.skip(!publicPackagesAvailable(), "Run python scripts/packages.py download");
  const catalog = JSON.parse(fs.readFileSync(path.join(process.env.RNL_PACKAGE_FIXTURE, 'catalog.json'), 'utf8'));
  const previous = catalog.packages.find(pkg => pkg.name === 'eda-symbol');
  const bytes = publicPackageText('eda-symbol').replace('@PACKAGE(name="eda-symbol", version="0.1")', '@PACKAGE(name="eda-symbol", version="0.2")');
  const next = { ...previous, version: '0.2', path: 'eda-symbol/0.2/eda-symbol.rnl',
    url: 'packages/eda-symbol/0.2/eda-symbol.rnl', sha256: createHash('sha256').update(bytes).digest('hex') };
  catalog.packages.push(next);
  await page.route('**/public-packages/catalog.json', route => route.fulfill({ json: catalog }));
  await page.route('**/public-packages/packages/eda-symbol/0.2/eda-symbol.rnl', route => route.fulfill({ body: bytes }));
  const frame = await start(page);
  const result = await frame.evaluate(async () => {
    const { default: shell } = await import('./shells/web.js');
    const packages = (await shell.publicPackages()).filter(pkg => pkg.name === 'eda-symbol');
    const items = [];
    for (const pkg of packages) {
      // A restored descriptor must not override the canonical path from the catalog.
      items.push(...await shell.includePackage({ ...pkg, path: 'eda-symbol/eda-symbol.rnl' }));
    }
    shell.setPool(items);
    const doc = await shell.build({ name: 'versions.sp', text: '* versions\n* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="eda-symbol", version="0.1"); @USE-PACKAGE(name="eda-symbol", version="0.2") }\nR1 a 0 1k\n.end\n' });
    return { paths: items.map(item => item.path), errors: doc.diagnostics.filter(d => d.severity === 'error') };
  });
  expect(result.paths).toEqual(['/library/eda-symbol/eda-symbol.rnl', '/library/eda-symbol/0.2/eda-symbol.rnl']);
  expect(result.errors).toEqual([]);
});

const usageCurrent = fs.readFileSync(new URL("../inputs/usage-current.sp", import.meta.url), "utf8");
const frames = (frame) => frame.locator("#canvas .usage-frame");
const arrows = (frame) => frame.locator("#canvas .current-arrow");
async function openUsageCurrent(page, count = 1) {
  await send(page, { type: "open", name: "usage-current.sp", text: usageCurrent });
  await waitMessage(page, "loaded", count);
}

test("an embedded drawing starts without the usage frames and with the current directions; ready tells they are known", async ({ page }) => {
  // the user's own settings of the complete viewer are neither read nor written
  const frame = await start(page, {}, { seed: { "usages-shown": true, "currents-shown": false } });
  expect((await messages(page, "ready"))[0].features).toEqual(expect.arrayContaining(["usages", "currents"]));
  await openUsageCurrent(page);
  await expect(arrows(frame)).toHaveCount(7);
  await expect(frames(frame)).toHaveCount(0);
  expect(await frame.evaluate(() => window.storageCalls)).toEqual([]);
  expect(await page.evaluate(() => JSON.stringify({ ...localStorage }) === window.storedBefore)).toBe(true);
});

test("usages=1 and currents=0 turn the defaults over, for this page only", async ({ page, browserName }) => {
  const frame = await start(page, { usages: "1", currents: "0" }, { seed: { "usages-shown": false, "currents-shown": true } });
  await openUsageCurrent(page);
  await expect(frames(frame).first()).toBeVisible();
  await expect(arrows(frame)).toHaveCount(0);
  expect(await frame.evaluate(() => window.storageCalls)).toEqual([]);
  expect(await page.evaluate(() => JSON.stringify({ ...localStorage }) === window.storedBefore)).toBe(true);
  // another open keeps them
  await openUsageCurrent(page, 2);
  await expect(frames(frame).first()).toBeVisible();
  await expect(arrows(frame)).toHaveCount(0);
  await screenshot(page, "embed-usages-1-currents-0.png", browserName);
});

test("usages=0 and currents=1 are the defaults; other values are no choice", async ({ page }) => {
  const frame = await start(page, { usages: "0", currents: "1" });
  await openUsageCurrent(page);
  await expect(frames(frame)).toHaveCount(0);
  await expect(arrows(frame)).toHaveCount(7);
  await frame.goto(`${base}/?embed=1&usages=yes&currents=no`);
  await waitMessage(page, "ready", 2);
  await send(page, { type: "open", name: "usage-current.sp", text: usageCurrent });
  await waitMessage(page, "loaded", 2);
  await expect(frames(frame)).toHaveCount(0);
  await expect(arrows(frame)).toHaveCount(7);
});

test("usages and currents messages switch the usage frames and the current directions, on and off, without a reload", async ({ page }) => {
  const frame = await start(page, {}, { seed: { "usages-shown": false, "currents-shown": true } });
  await openUsageCurrent(page);
  await expect(arrows(frame)).toHaveCount(7);
  await expect(frames(frame)).toHaveCount(0);
  await send(page, { type: "usages", value: true });
  await expect(frames(frame).first()).toBeVisible();
  await send(page, { type: "currents", value: false });
  await expect(arrows(frame)).toHaveCount(0);
  await send(page, { type: "usages", value: false });
  await expect(frames(frame)).toHaveCount(0);
  await send(page, { type: "currents", value: true });
  await expect(arrows(frame)).toHaveCount(7);
  // a value that is no boolean changes nothing
  await send(page, { type: "usages", value: "1" });
  await send(page, { type: "currents", value: 0 });
  await send(page, { type: "fit" });
  await expect(frames(frame)).toHaveCount(0);
  await expect(arrows(frame)).toHaveCount(7);
  expect(await messages(page, "error")).toEqual([]);
  expect(await frame.evaluate(() => window.storageCalls)).toEqual([]);
  expect(await page.evaluate(() => JSON.stringify({ ...localStorage }) === window.storedBefore)).toBe(true);
});
