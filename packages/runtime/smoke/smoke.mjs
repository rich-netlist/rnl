// Smoke test of the runtime package in a frontend built with Vite, installed as npm installs it:
//
//   node packages/runtime/smoke/smoke.mjs SPEC
//
// SPEC is what npm installs: the tarball that publish-runtime.yml packed, or a version on npm, such
// as @rich-netlist/runtime@0.1.0-preview.9. A version npm does not serve yet is tried again for some
// minutes, as a new one takes them to appear. The page, index.html and main.js, builds a netlist
// with the core in its Worker and writes an edit (vite.config.js has the settings the package's
// README asks for). The test builds the page with Vite, checks that the build has the Worker and the
// module, opens the build and then the dev server in Chromium, and requires the page's result: no
// error, a level drawn with its SVG, and the edit written.

import { execFileSync, execSync, spawn } from "node:child_process";
import { existsSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { fileURLToPath } from "node:url";

const { publicPackagesAvailable } = await import("../../../scripts/packages.mjs");
if (!publicPackagesAvailable(["analog-canvas-extensions"])) process.exit(0);

const here = dirname(fileURLToPath(import.meta.url));
const spec = process.argv[2];
if (!spec) {
  console.error("usage: node packages/runtime/smoke/smoke.mjs SPEC");
  process.exit(2);
}
const fail = (message) => {
  console.error(`FAIL ${message}`);
  process.exit(1);
};

// npm is a command script on Windows, which only a shell runs
const npm = (args) =>
  process.platform === "win32"
    ? execSync(["npm", ...args.map((a) => JSON.stringify(a))].join(" "), { cwd: here, stdio: "pipe", encoding: "utf8" })
    : execFileSync("npm", args, { cwd: here, stdio: "pipe", encoding: "utf8" });

// the package, as a frontend installs it
const local = existsSync(spec);
const what = local ? resolve(spec).split(String.fromCharCode(92)).join("/") : spec;
for (let tried = 0; ; tried++) {
  try {
    npm(["install", "--no-save", "--no-audit", "--no-fund", what]);
    break;
  } catch (e) {
    const text = `${e.stdout ?? ""}${e.stderr ?? ""}`;
    if (local || tried >= 30 || !/E404|ETARGET|No matching version/.test(text)) fail(`npm install ${what}: ${text}`);
    console.log(`${what} is not on npm yet; trying again in 20 s`);
    await sleep(20_000);
  }
}
const installed = JSON.parse(readFileSync(join(here, "node_modules", "@rich-netlist", "runtime", "package.json"), "utf8")).version;
console.log(`installed @rich-netlist/runtime ${installed}`);

// the build: the Worker and the module among its files
const vite = join(here, "node_modules", "vite", "bin", "vite.js");
rmSync(join(here, "dist"), { recursive: true, force: true });
execFileSync(process.execPath, [vite, "build", "--logLevel", "warn"], { cwd: here, stdio: "inherit" });
const assets = readdirSync(join(here, "dist", "assets"));
if (!assets.some((f) => f.endsWith(".wasm"))) fail(`the build has no .wasm: ${assets.join(", ")}`);
if (!assets.some((f) => /worker/i.test(f) && f.endsWith(".js"))) fail(`the build has no Worker: ${assets.join(", ")}`);
console.log(`built: ${assets.join(", ")}`);

const { chromium } = await import("playwright");
const browser = await chromium.launch();
let server;
try {
  for (const [mode, args, port] of [
    ["the build", ["preview", "--host", "127.0.0.1", "--port", "4174", "--strictPort"], 4174],
    ["the dev server", ["--host", "127.0.0.1", "--port", "5174", "--strictPort"], 5174],
  ]) {
    server = spawn(process.execPath, [vite, ...args], { cwd: here, stdio: "ignore" });
    const url = `http://127.0.0.1:${port}/`;
    let up = false;
    for (let k = 0; k < 60 && !up; k++) {
      up = await fetch(url).then((r) => r.ok, () => false);
      if (!up) await sleep(500);
    }
    if (!up) fail(`${mode} does not answer at ${url}`);
    const page = await browser.newPage();
    const errors = [];
    page.on("pageerror", (e) => errors.push(String(e)));
    page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
    await page.goto(url);
    await page.waitForSelector("#result[data-state]", { timeout: 60_000 });
    const state = await page.getAttribute("#result", "data-state");
    const text = await page.textContent("#result");
    if (state !== "done") fail(`${mode}: ${text} ${errors.join("; ")}`);
    const r = JSON.parse(text);
    if (r.errors.length) fail(`${mode}: the netlist reads with errors: ${r.errors.join("; ")}`);
    if (r.levels < 1 || r.svg === 0) fail(`${mode}: no level drawn: ${text}`);
    if (!r.written.includes("#R1 @PLACE(2, 0)")) fail(`${mode}: the edit is not written: ${r.written}`);
    console.log(`ok   ${mode}: ${r.levels} level, ${r.svg} characters of SVG, the edit written`);
    await page.close();
    server.kill();
    server = undefined;
  }
} finally {
  server?.kill();
  await browser.close();
}
