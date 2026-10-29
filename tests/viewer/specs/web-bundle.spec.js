// The web viewer's site as it is published, with its scripts bundled (apps/web/bundle.mjs): built here into a
// directory of its own and served, since web.spec.js and the other specs read the unbundled site and import single
// modules of the page, which a bundle has not. It needs the core built and esbuild installed (npm ci --prefix apps/web).

import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { expect, test } from "@playwright/test";
import { root } from "../fixtures.js";

test.skip(!process.env.RNL_WEB_URL, "the site is not built: RNL_WEB=0");
const hasEsbuild = fs.existsSync(path.join(root, "apps/web/node_modules/esbuild"));
test.skip(!hasEsbuild && !process.env.CI, "esbuild is not installed: npm ci --prefix apps/web");
test.describe.configure({ mode: "serial" });

let dir;
let server;
let base;

test.beforeAll(async () => {
  expect(hasEsbuild, "esbuild is not installed: npm ci --prefix apps/web").toBe(true);
  dir = fs.mkdtempSync(path.join(os.tmpdir(), "rnl-web-bundle-"));
  execFileSync(process.execPath, ["apps/web/build.mjs", "--no-core", "--samples", "--bundle", "--out", dir], { cwd: root, stdio: "pipe" });
  server = spawn(process.execPath, ["apps/web/serve.mjs", "--port", "0", "--dir", dir], { cwd: root, stdio: ["ignore", "pipe", "inherit"] });
  base = await new Promise((resolve, reject) => {
    let out = "";
    server.stdout.on("data", chunk => {
      out += chunk;
      const m = /http:\/\/127\.0\.0\.1:\d+\//.exec(out);
      if (m) resolve(m[0]);
    });
    server.on("exit", code => reject(new Error(`serve.mjs exited with ${code}`)));
  });
});

test.afterAll(() => {
  server?.kill();
  if (dir) fs.rmSync(dir, { recursive: true, force: true });
});

test.beforeEach(async ({ page }) => {
  await page.route("https://rich-netlist.github.io/rnl-packages/**", route => route.abort());
});

test("the bundled site is a few scripts, none of the page's modules", async () => {
  const names = fs.readdirSync(dir, { recursive: true }).map(String).map(n => n.split(path.sep).join("/"));
  const scripts = names.filter(n => n.endsWith(".js") && !n.startsWith("fonts/") && !n.startsWith("samples/"));
  expect(scripts).toEqual(expect.arrayContaining(["startup.js", "main.js", "shells/web.js", "core/client.js", "core/worker.js", "embed.js", "full-viewer-api.js"]));
  expect(scripts.length).toBeLessThan(25);
  // the sources are not carried beside their bundle
  for (const gone of ["panels.js", "selection.js", "i18n.js", "core/core.js"]) expect(scripts).not.toContain(gone);
  expect(names).not.toContain("shells/desktop.js");
  // the Worker still finds the core where it was
  expect(names).toContain("core/rnl_runtime_wasm.wasm");
  expect(fs.readFileSync(path.join(dir, "core/worker.js"), "utf8")).toContain('new URL("./rnl_runtime_wasm.wasm",import.meta.url)');
  // and so does the client that starts the Worker: its code is in a chunk of core/, not in a script of another directory
  const chunks = names.filter(n => /^core\/chunk-.*\.js$/.test(n)).map(n => fs.readFileSync(path.join(dir, n), "utf8"));
  expect(chunks.some(text => text.includes('new URL("./worker.js",import.meta.url)'))).toBe(true);
  expect(fs.readFileSync(path.join(dir, "shells/web.js"), "utf8")).not.toContain("worker.js");
});

test("the page starts, builds its document in the core's Worker and reads what its hints name", async ({ page }) => {
  const errors = [];
  const requested = new Map();
  page.on("pageerror", error => errors.push(String(error)));
  page.on("console", message => { if (message.type() === "error") errors.push(message.text()); });
  page.on("response", response => {
    const url = new URL(response.url());
    if (url.origin === new URL(base).origin) requested.set(url.pathname.replace(/^\//, ""), response.status());
  });
  await page.goto(`${base}?url=samples/cases/04-hierarchy.sp`);
  await expect(page).toHaveTitle(/^04-hierarchy\.sp/);
  await expect(page.locator("#canvas > svg")).toBeVisible();
  expect(await page.evaluate(() => window.viewer.doc.objects.length)).toBeGreaterThan(3);
  const hints = await page.evaluate(() => ({
    modules: [...document.querySelectorAll('link[rel="modulepreload"]')].map(l => l.getAttribute("href")),
    fetched: [...document.querySelectorAll('link[rel="preload"][as="fetch"]')].map(l => l.getAttribute("href")),
  }));
  expect(hints.modules.length).toBeGreaterThan(5);
  for (const href of [...hints.modules, ...hints.fetched]) expect(requested.get(href), href).toBe(200);
  expect(hints.fetched).toEqual(expect.arrayContaining(["core/rnl_runtime_wasm.wasm", "locales/keys.json"].filter(f => fs.existsSync(path.join(dir, f)))));
  // every script the page runs to start is hinted, but those of the modes this page is not in
  const scripts = [...requested].filter(([p, status]) => p.endsWith(".js") && status === 200 && !["embed.js", "full-viewer-api.js"].includes(p)).map(([p]) => p);
  expect(scripts.filter(p => !hints.modules.includes(p) && p !== "startup.js" && p !== "main.js")).toEqual([]);
  expect([...requested].filter(([, status]) => status >= 400)).toEqual([]);
  expect(errors).toEqual([]);
});

test("a language chosen and the text applied work from the bundle", async ({ page }) => {
  const errors = [];
  page.on("pageerror", error => errors.push(String(error)));
  await page.goto(`${base}?url=samples/cases/04-hierarchy.sp`);
  await expect(page).toHaveTitle(/^04-hierarchy\.sp/);
  await page.evaluate(() => window.viewer.command("language-zh"));
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
  await page.evaluate(() => window.viewer.command("language-en"));
  await page.keyboard.press("Control+q");
  const area = page.locator("#source .editor:not([hidden]) textarea");
  const text = await area.inputValue();
  await area.fill(`* a comment, applied\n${text}`);
  await page.locator("#apply-edits").click();
  await expect.poll(() => page.evaluate(() => window.viewer.doc.files[0].text.includes("a comment, applied"))).toBe(true);
  expect(errors).toEqual([]);
});

test("the embedded mode starts from its own script and says it is ready", async ({ page }) => {
  await page.goto(`${base}about.json`);
  const ready = await page.evaluate(async base => {
    const frame = document.createElement("iframe");
    frame.src = `${base}?embed=1&origin=${encodeURIComponent(location.origin)}`;
    const got = new Promise(resolve => addEventListener("message", e => { if (e.data?.rnl === 1 && e.data.type === "ready") resolve(e.data); }));
    document.body.append(frame);
    return Promise.race([got, new Promise(resolve => setTimeout(() => resolve(null), 20000))]);
  }, base);
  expect(ready).not.toBeNull();
});
