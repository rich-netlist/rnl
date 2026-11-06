// Drives a Firefox that a container started with --marionette, over Marionette's own protocol (length-prefixed JSON on
// TCP port 2828), so that no geckodriver is needed: opens the web viewer, waits for a document, solves it through the
// development server's solver service, and writes a screenshot and a JSON summary.
//
//   node scripts/linux-el-probe/drive.mjs URL OUT_DIR [LABEL]

import net from "node:net";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const [, , url, outDir, label = "probe"] = process.argv;
if (!url || !outDir) {
  console.error("usage: drive.mjs URL OUT_DIR [LABEL]");
  process.exit(2);
}
mkdirSync(outDir, { recursive: true });

// (the page's own objects, such as window.viewer, are seen from Marionette's sandbox only through window.wrappedJSObject)
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

class Marionette {
  constructor(socket) {
    this.socket = socket;
    this.buffer = Buffer.alloc(0);
    this.pending = new Map();
    this.next = 1;
    this.hello = new Promise((resolve) => (this.onHello = resolve));
    socket.on("data", (chunk) => {
      this.buffer = Buffer.concat([this.buffer, chunk]);
      for (;;) {
        const colon = this.buffer.indexOf(0x3a);
        if (colon < 0) return;
        const length = Number(this.buffer.subarray(0, colon).toString());
        if (this.buffer.length < colon + 1 + length) return;
        const text = this.buffer.subarray(colon + 1, colon + 1 + length).toString("utf8");
        this.buffer = this.buffer.subarray(colon + 1 + length);
        this.packet(JSON.parse(text));
      }
    });
  }

  packet(message) {
    if (!Array.isArray(message)) return this.onHello(message);
    const [, id, error, result] = message;
    const wait = this.pending.get(id);
    if (!wait) return;
    this.pending.delete(id);
    if (error) wait.reject(new Error(`${wait.name}: ${JSON.stringify(error)}`));
    else wait.resolve(result);
  }

  send(name, params = {}) {
    const id = this.next++;
    const body = JSON.stringify([0, id, name, params]);
    return new Promise((resolve, reject) => {
      this.pending.set(id, { name, resolve, reject });
      this.socket.write(`${Buffer.byteLength(body)}:${body}`);
    });
  }

  script(source, args = []) {
    return this.send("WebDriver:ExecuteScript", { script: source, args, sandbox: "default" }).then((r) => r?.value ?? r);
  }
}

async function connect() {
  for (let attempt = 0; attempt < 120; attempt++) {
    try {
      const socket = await new Promise((resolve, reject) => {
        const s = net.connect({ host: "127.0.0.1", port: 2828 }, () => resolve(s));
        s.once("error", reject);
      });
      const client = new Marionette(socket);
      await client.hello;
      return client;
    } catch {
      await sleep(1000);
    }
  }
  throw new Error("Marionette did not answer on 127.0.0.1:2828");
}

async function until(client, source, seconds) {
  const stop = Date.now() + seconds * 1000;
  let last;
  while (Date.now() < stop) {
    last = await client.script(source).catch((error) => `error: ${error.message}`);
    if (last === true) return true;
    await sleep(1000);
  }
  return last;
}

const summary = { label, url, checks: {} };
const check = (name, ok, detail = "") => {
  summary.checks[name] = { ok: !!ok, detail };
  console.log(`${ok ? "ok  " : "FAIL"} ${name}${detail ? `: ${detail}` : ""}`);
};

const client = await connect();
let failed = false;
try {
  await client.send("WebDriver:NewSession", { capabilities: { alwaysMatch: {} } });
  summary.userAgent = await client.script("return navigator.userAgent");
  console.log(summary.userAgent);
  await client.send("WebDriver:Navigate", { url });
  const loaded = await until(client, "var w = window.wrappedJSObject; return !!(w.viewer && w.viewer.doc && w.viewer.doc.levels.length > 0)", 180);
  check("the viewer opens a document", loaded === true, String(loaded));
  if (loaded === true) {
    check("the drawing is an SVG", await client.script("return !!document.querySelector('#canvas > svg')"));
    summary.levels = await client.script("return window.wrappedJSObject.viewer.doc.levels.length");
    // a solve through the development server's solver service (the page asks its own origin)
    await client.script("window.wrappedJSObject.viewer.command('solve'); return true");
    const solved = await until(
      client,
      "return document.body.classList.contains('preview') && !document.body.classList.contains('solving')",
      240,
    );
    check("a solve shows its preview", solved === true, String(solved));
  } else {
    summary.page = await client.script("return document.body ? document.body.innerText.slice(0, 600) : 'no body'").catch((e) => e.message);
    console.log(summary.page);
  }
  const shot = await client.send("WebDriver:TakeScreenshot", { full: false, hash: false, scroll: true }).catch((e) => ({ error: e.message }));
  const data = shot?.value ?? shot;
  if (typeof data === "string") writeFileSync(join(outDir, `${label}.png`), Buffer.from(data, "base64"));
  else console.log("no screenshot:", JSON.stringify(shot));
} catch (error) {
  check("the run", false, error.message);
}
failed = Object.values(summary.checks).some((c) => !c.ok);
writeFileSync(join(outDir, `${label}.json`), JSON.stringify(summary, null, 2) + "\n");
client.socket.destroy();
process.exit(failed ? 1 : 0);
