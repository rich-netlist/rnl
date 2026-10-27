// Drive the actual Linux binary through WebKitGTK's native WebDriver, as tauri-driver does:
// https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-driver/src/server.rs
// Node 22+, webkit2gtk-driver, xdotool and ImageMagick; run under dbus-run-session and xvfb-run.
// No build, plugins or test-only code in the app. --exe accepts an AppImage or an installed binary.
//   dbus-run-session -- xvfb-run -a node apps/desktop/tests/linux-smoke.mjs --exe PATH [--packages with|without]
// --packages says whether the program bundles the public packages: with, it finds eda-style without a network
// and lists them in the Library; without, it reports the package missing, with the hint to Help > Public Packages.
import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { once } from "node:events";
import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { screenshotEvidence, assertPainted } from "./visual.mjs";
import { cjkEvidence, assertCjk } from "./cjk.mjs";

assert.equal(process.platform, "linux", "The Linux smoke test must run on Linux.");
const exeIndex = process.argv.indexOf("--exe");
assert.ok(exeIndex >= 0 && process.argv[exeIndex + 1], "Pass --exe PATH.");
const exe = path.resolve(process.argv[exeIndex + 1]);
const packagesIndex = process.argv.indexOf("--packages");
const expectPackages = packagesIndex >= 0 ? process.argv[packagesIndex + 1] : undefined;
assert.ok(expectPackages === undefined || expectPackages === "with" || expectPackages === "without", "--packages takes with or without.");
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
        starting: document.body.classList.contains("starting"), startup: !!document.getElementById("startup"),
        errors: [...document.querySelectorAll("#diags .error")].map(e => e.textContent),
        empty: document.getElementById("empty")?.textContent
      };`, args: [] });
    return last.loaded && last.drawn && !last.busy;
  });
  assert.equal(last.level, sample);
  assert.ok(last.levels.includes("amp"), "The sample's subcircuit was not loaded.");
  assert.deepEqual(last.errors, [], "The sample opened with errors.");
  fs.writeFileSync(path.join(output,`${path.basename(exe)}-dom-ready.json`),JSON.stringify(last,null,2));
  const windows = await until("the visible window's title", () =>
    execFileSync("xdotool", ["search", "--onlyvisible", "--name", `^${sample} - RNL Viewer$`], { encoding:"utf8" }));
  const windowId = windows.trim().split(/\s+/)[0];
  assert.ok(windowId, "There is no visible viewer window.");
  let first = true;
  // A valid DOM can still be invisible. Wait for the startup handoff, then require pixels
  // in the tab text, source and schematic. Borders and the grid do not satisfy this check.
  await until("painted tab, source and schematic pixels", async () => {
    // The WebView occupies the client window below GTK's native menu. Crop the real X
    // framebuffer to that viewport; do not let a successful WebKit snapshot mask a blank window.
    const viewport = await request("POST", `/session/${session}/execute/sync`, {
      script:'return {width:innerWidth,height:innerHeight};',args:[] });
    const geometry = Object.fromEntries(execFileSync('xdotool',['getwindowgeometry','--shell',windowId],{encoding:'utf8'})
      .trim().split('\n').map(line=>line.split('=')));
    const width=Number(geometry.WIDTH), height=Math.round(viewport.height*width/viewport.width);
    const x=Number(geometry.X), y=Number(geometry.Y)+Number(geometry.HEIGHT)-height;
    const nativePath=path.join(output,`${path.basename(exe)}-x11.png`);
    const crop=`${width}x${height}${x < 0 ? '' : '+'}${x}${y < 0 ? '' : '+'}${y}`;
    execFileSync('import',['-window','root','-crop',crop,'+repage',nativePath],{timeout:5000});
    const native = fs.readFileSync(nativePath).toString('base64');
    const screenshot = await request("GET", `/session/${session}/screenshot`);
    fs.writeFileSync(path.join(output, `${path.basename(exe)}.png`), Buffer.from(screenshot, "base64"));
    const visual = await request("POST", `/session/${session}/execute/sync`, {
      script:`return (${screenshotEvidence.toString()})(${JSON.stringify(screenshot)});`, args:[] });
    const nativeVisual = await request("POST", `/session/${session}/execute/sync`, {
      script:`return (${screenshotEvidence.toString()})(${JSON.stringify(native)});`,args:[] });
    const evidence={webkit:visual,x11:nativeVisual};
    fs.writeFileSync(path.join(output, `${path.basename(exe)}-pixels.json`), JSON.stringify(evidence,null,2));
    if(first) {
      fs.copyFileSync(nativePath,path.join(output,`${path.basename(exe)}-initial-x11.png`));
      fs.writeFileSync(path.join(output,`${path.basename(exe)}-initial-webkit.png`),Buffer.from(screenshot,'base64'));
      fs.writeFileSync(path.join(output,`${path.basename(exe)}-initial-pixels.json`),JSON.stringify(evidence,null,2));
      first=false;
    }
    assertPainted(visual);
    assertPainted(nativeVisual);
    return true;
  });
  assert.deepEqual(fs.readFileSync(path.join(temp, sample)), original, "Opening the sample wrote to it.");
  // The corpus has Chinese comments. Switch all web UI text to Chinese, verify the actual
  // glyph raster against the shipped font, and retain before/after frames from this binary.
  const execute = script => request('POST', `/session/${session}/execute/sync`, {script, args:[]});
  await execute('window.viewer.command("language-zh");');
  await execute('return document.fonts.ready.then(() => true);');
  const cjk = await execute(`return (${cjkEvidence.toString()})();`);
  fs.writeFileSync(path.join(output, `${path.basename(exe)}-cjk.json`), JSON.stringify(cjk,null,2));
  assertCjk(cjk, process.env.RNL_NO_SYSTEM_CJK === '1');
  // Negative control: remove only the new fallback from the live styles, with the same
  // binary, sample, rendering core and system fonts. This must fail on the fontless runner.
  await execute(`window.cjkStyles = [];
    for (const sheet of document.styleSheets) for (const rule of sheet.cssRules) {
      if (rule.style?.fontFamily.includes('Noto Sans SC')) {
        window.cjkStyles.push([rule,rule.style.fontFamily]);
        rule.style.fontFamily = rule.style.fontFamily.replace(/,\\s*["']?Noto Sans SC["']?/g,'');
      }
    }`);
  const before = await execute(`return (${cjkEvidence.toString()})();`);
  fs.writeFileSync(path.join(output, `${path.basename(exe)}-cjk-before.json`), JSON.stringify(before,null,2));
  fs.writeFileSync(path.join(output, `${path.basename(exe)}-cjk-before.png`), Buffer.from(await request('GET', `/session/${session}/screenshot`),'base64'));
  if (process.env.RNL_NO_SYSTEM_CJK === '1') assert.throws(() => assertCjk(before,true));
  await execute('for (const [rule,font] of window.cjkStyles) rule.style.fontFamily=font;');
  await execute('return new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))));');
  fs.writeFileSync(path.join(output, `${path.basename(exe)}-cjk-after.png`), Buffer.from(await request('GET', `/session/${session}/screenshot`),'base64'));
  if (expectPackages) {
    // the public packages: found without a network when bundled, reported missing with a hint when not
    const top = path.join(temp, "use-eda-style.sp");
    fs.writeFileSync(top, ["* a file that uses a public package", '* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="eda-style", version="0.1") }', "R1 a 0 1k", ".end", ""].join("\n"));
    const info = await execute("return import('./shell.js').then((m) => m.shell.bundledPackages());");
    await execute(`return import('./open.js').then((o) => o.openSource({ path: ${JSON.stringify(top)}, name: 'use-eda-style.sp' }, false));`);
    const missing = await execute("return window.viewer.doc.diagnostics.some((d) => d.message.includes('eda-style') && d.message.includes('cannot be found'));");
    const names = info.packages.map((p) => `${p.name} ${p.version}`);
    if (expectPackages === "with") {
      assert.ok(info.bundled && names.includes("eda-style 0.1"), `bundled: ${JSON.stringify(info)}`);
      assert.equal(missing, false, "eda-style 0.1 is bundled, and was reported missing");
      await execute("document.getElementById('tab-files').click();");
      assert.deepEqual(await execute("const g = document.querySelector('[data-group=\"public\"]'); return g ? [...g.querySelectorAll('.pool-open')].map((b) => b.textContent) : null;"), names);
      console.log(`${path.basename(exe)}: the public packages are bundled and found without a network: ${names.join(", ")}`);
    } else {
      assert.ok(!info.bundled && info.packages.length === 0, `bundled: ${JSON.stringify(info)}`);
      assert.equal(missing, true, "eda-style 0.1 is not bundled, and was not reported missing");
      await execute("const head = document.getElementById('problems-head'); if (head.getAttribute('aria-expanded') === 'false') head.click();");
      assert.equal(await execute("return document.querySelector('.package-hint .msg')?.textContent;"), "Look for it in Help › Public Packages.");
      console.log(`${path.basename(exe)}: the public packages are not bundled; the package is reported missing, with the hint to Help > Public Packages`);
    }
  }
  console.log(`${path.basename(exe)}: visible window, sample and include loaded, SVG drawn, no errors.`);
} finally {
  if (session) {
    try {
      const screenshotPath=path.join(output,`${path.basename(exe)}.png`);
      if(!fs.existsSync(screenshotPath)) {
        const screenshot = await request("GET", `/session/${session}/screenshot`);
        fs.writeFileSync(screenshotPath, Buffer.from(screenshot, "base64"));
      }
    } catch (e) { console.error("Screenshot:", e.message); }
    await request("DELETE", `/session/${session}`).catch((e) => console.error("Session cleanup:", e.message));
  }
  if (driver.pid) {
    try { process.kill(-driver.pid, "SIGKILL"); } catch (e) { if (e.code !== "ESRCH") throw e; }
  }
  fs.rmSync(temp, { recursive: true, force: true });
}
