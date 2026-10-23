// Manual CI only: observe the unmodified app through native WebKit automation and X11.
// No injected repaint, forced visibility, or renderer flags in the baseline case.
import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { once } from "node:events";
import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

assert.equal(process.platform, "linux");
const exe = path.resolve(process.argv[process.argv.indexOf("--exe") + 1]);
fs.accessSync(exe, fs.constants.X_OK);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const output = path.resolve(process.env.SMOKE_OUTPUT_DIR);
fs.mkdirSync(output, { recursive: true });
const temp = fs.mkdtempSync(path.join(os.tmpdir(), "rnl-linux-diagnostic-"));
const sample = "04-hierarchy.sp";
for (const name of [sample, "04-hierarchy-lib.inc"]) {
  fs.copyFileSync(path.join(root, "tests/conformance/cases", name), path.join(temp, name));
}
// A control without WebDriver, injected JavaScript, rAF observers, or WebKit snapshots.
// This is what an ordinary user launch paints into the X server.
if (process.env.DIAGNOSTIC_NATIVE === '1') {
  const fd = fs.openSync(path.join(output,'process.log'),'w');
  const app = spawn(exe,[path.join(temp,sample)],{detached:true,stdio:['ignore',fd,fd],env:{...process.env,
    XDG_CONFIG_HOME:path.join(temp,'config'),XDG_DATA_HOME:path.join(temp,'data'),XDG_CACHE_HOME:path.join(temp,'cache')}});
  const opened=Date.now();
  try {
    const times=[];
    for(const seconds of [1,3,10]) {
      await new Promise(r=>setTimeout(r,Math.max(0,opened+seconds*1000-Date.now())));
      execFileSync('import',['-window','root',path.join(output,`${seconds}s-x11.png`)]);
      times.push({seconds,elapsedMs:Date.now()-opened,exitCode:app.exitCode});
    }
    fs.writeFileSync(path.join(output,'summary.json'),JSON.stringify({exe,native:true,times},null,2));
  } finally {
    if(app.pid) {try{process.kill(-app.pid,'SIGKILL');}catch(e){if(e.code!=='ESRCH')throw e;}}
    fs.closeSync(fd); fs.rmSync(temp,{recursive:true,force:true});
  }
  process.exit(0);
}
const server = net.createServer().listen(0, "127.0.0.1");
await once(server, "listening");
const port = server.address().port;
await new Promise(resolve => server.close(resolve));
const log = fs.openSync(path.join(output, "process.log"), "w");
const driver = spawn("WebKitWebDriver", [`--port=${port}`, "--host=127.0.0.1"], {
  detached: true, stdio: ["ignore", log, log],
  env: { ...process.env, TAURI_WEBVIEW_AUTOMATION: "true",
    XDG_CONFIG_HOME: path.join(temp, "config"), XDG_DATA_HOME: path.join(temp, "data"),
    XDG_CACHE_HOME: path.join(temp, "cache") },
});
let session;
const results = [];
const began = Date.now();
async function request(method, route, body, timeout = 15_000) {
  const response = await fetch(`http://127.0.0.1:${port}${route}`, {
    method, headers: { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body), signal: AbortSignal.timeout(timeout),
  });
  const result = await response.json();
  assert.ok(response.ok && !result.value?.error, JSON.stringify(result));
  return result.value;
}
async function evaluate(script) {
  return request("POST", `/session/${session}/execute/sync`, { script, args: [] });
}
try {
  for (let i = 0; ; i++) {
    try { await request("GET", "/status"); break; }
    catch (e) { if (i === 100) throw e; await new Promise(r => setTimeout(r, 100)); }
  }
  const opened = Date.now();
  session = (await request("POST", "/session", { capabilities: { alwaysMatch: {
    "webkitgtk:browserOptions": { binary: exe, args: [path.join(temp, sample)] },
  } } })).sessionId;
  await evaluate(`window.__diagnosticFrames = 0;
    requestAnimationFrame(function frame() { window.__diagnosticFrames++; requestAnimationFrame(frame); });
    return true;`);
  for (const seconds of [1, 3, 10]) {
    await new Promise(r => setTimeout(r, Math.max(0, opened + seconds * 1000 - Date.now())));
    const state = await evaluate(`
      const inspect = selector => {
        const elements = [...document.querySelectorAll(selector)];
        return {count:elements.length, elements:elements.slice(0,5).map(el => {
          const s=getComputedStyle(el), r=el.getBoundingClientRect();
          return {tag:el.tagName, text:el.textContent.slice(0,100),
            opacity:s.opacity, visibility:s.visibility, display:s.display,
            color:s.color, background:s.backgroundColor, contentVisibility:s.contentVisibility,
            rect:{x:r.x,y:r.y,width:r.width,height:r.height},
            parents:[...function*(e){while(e=e.parentElement)yield e}(el)].map(p=>{
              const ps=getComputedStyle(p);return {tag:p.tagName,id:p.id,class:p.className?.baseVal??p.className,
                visibility:ps.visibility,opacity:ps.opacity,display:ps.display};})};})};
      };
      return {url:location.href, ready:document.readyState, frames:window.__diagnosticFrames,
        fonts:document.fonts.status, bodyClass:document.body.className,
        level:window.viewer?.level, loaded:!!window.viewer?.doc, busy:document.body.classList.contains('busy'),
        tabs:inspect('.view-tabs [role=tab]'), rows:inspect('#source .ln'),
        svg:inspect('#canvas > svg'), primitives:inspect('#canvas > svg path, #canvas > svg text'),
        startup:inspect('#startup'), loading:inspect('#loading'),
        errors:[...document.querySelectorAll('#diags .error')].map(e=>e.textContent),
        html:document.documentElement.outerHTML};`);
    state.seconds = seconds;
    state.elapsedMs = Date.now() - opened;
    fs.writeFileSync(path.join(output, `${seconds}s.json`), JSON.stringify(state, null, 2));
    // Capture the actual X screen before asking WebKit to snapshot (a snapshot may trigger paint).
    execFileSync("import", ["-window", "root", path.join(output, `${seconds}s-x11.png`)]);
    if (process.env.DIAGNOSTIC_X11_ONLY !== '1') {
      try {
        fs.writeFileSync(path.join(output, `${seconds}s-webkit.png`),
          Buffer.from(await request("GET", `/session/${session}/screenshot`, undefined, 5000), "base64"));
      } catch(e) {
        fs.writeFileSync(path.join(output, `${seconds}s-snapshot-error.txt`), String(e));
      }
    }
    delete state.html;
    results.push(state);
    console.log(`${path.basename(exe)} at ${seconds}s: ${JSON.stringify(state)}`);
  }
} finally {
  fs.writeFileSync(path.join(output, "summary.json"), JSON.stringify({exe, elapsedMs:Date.now()-began,
    environment:Object.fromEntries(Object.entries(process.env).filter(([k]) => /WEBKIT|LIBGL|DISPLAY|APPIMAGE/.test(k))),
    results}, null, 2));
  if (session) await request("DELETE", `/session/${session}`, undefined, 3000).catch(console.error);
  if (driver.pid) { try { process.kill(-driver.pid, "SIGKILL"); } catch (e) { if (e.code !== "ESRCH") throw e; } }
  fs.closeSync(log);
  fs.rmSync(temp, { recursive:true, force:true });
}
