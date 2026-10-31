import { test } from "node:test";
import assert from "node:assert/strict";
import { createServer, request } from "node:http";
import { mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createService, insidePath, runVersion } from "./solver-service.mjs";

// A stand-in for the rnl command, run by node: `--version`, and `preview` and `problem` of the entry in the directory it
// runs in, which print what they were given, so that a test sees what the service wrote and passed.
const FAKE = `
const args = process.argv.slice(2);
const fs = require("fs");
if (args[0] === "--version") { console.log("rnl 0.0.0 (fake)"); process.exit(0); }
const entry = args[1] || args[0];
const text = fs.existsSync(entry) ? fs.readFileSync(entry, "utf8") : null;
if (/fail/.test(text || "")) { console.error("the solver is a preview, whose layouts may be poor\\nerror: the netlist fails"); process.exit(1); }
if (/slow/.test(text || "")) { fs.writeFileSync(process.env.FAKE_STARTED, "1"); setTimeout(() => {}, 60000); return; }
const seen = { args: args.slice(1), cwd: process.cwd(), files: fs.readdirSync(".").sort(), text };
console.log(JSON.stringify({ document: { path: entry, seen }, report: { levels: [] } }));
`;

async function setup() {
  const dir = await mkdtemp(join(tmpdir(), "rnl-solver-test-"));
  const script = join(dir, "fake-rnl.cjs");
  await writeFile(script, FAKE);
  return { dir, script };
}

/** A server of the service on a free port, with the fake command run by this node. */
async function serve(options = "") {
  const { dir, script } = await setup();
  // node runs the script: a .cmd file cannot be run without a shell
  const rnl = process.execPath;
  const service = createService({ rnl, prefix: [script], version: await runVersion(rnl, [script]), options, work: join(dir, "work") });
  const server = createServer((req, res) => {
    const { pathname } = new URL(req.url, "http://localhost");
    service.handle(req, res, pathname, server.address().port).then((done) => {
      if (!done) { res.writeHead(404); res.end(); }
    });
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return { dir, service, port: server.address().port, close: async () => { server.close(); await rm(dir, { recursive: true, force: true }); } };
}

/** One request: the status and the JSON; `headers` over the defaults. */
function call(port, method, path, body, headers = {}) {
  return new Promise((resolve, reject) => {
    const req = request({ host: "127.0.0.1", port, method, path, headers: { "Content-Type": "application/json", ...headers } }, (res) => {
      const chunks = [];
      res.on("data", (c) => chunks.push(c));
      res.on("end", () => { const text = Buffer.concat(chunks).toString("utf8"); resolve({ status: res.statusCode, json: text ? JSON.parse(text) : null }); });
    });
    req.on("error", reject);
    req.end(body === undefined ? undefined : JSON.stringify(body));
  });
}

const SOLVE = "/rnl-solver/solve";

test("the service says what it runs", async () => {
  const s = await serve("time_limit=none");
  try {
    const { status, json } = await call(s.port, "GET", "/rnl-solver");
    assert.equal(status, 200);
    assert.deepEqual({ service: json.service, version: json.version, options: json.options }, { service: "rnl-solver", version: "rnl 0.0.0 (fake)", options: "time_limit=none" });
  } finally { await s.close(); }
});

test("a solve writes the files where the page has them, runs preview on the entry there, and gives its document and report", async () => {
  const s = await serve("time_limit=none");
  try {
    const files = { "/top.sp": "* top\n.include cell.inc\n", "/cell.inc": "* cell\n", "/<pool>/packages/p.rnl": "pkg", "/<pool>/lib/x.inc": "inc" };
    const { status, json } = await call(s.port, "POST", SOLVE, { entry: "/top.sp", files, pool: ["/<pool>/packages/p.rnl", "/<pool>/lib/x.inc"], options: "beam=8" });
    assert.equal(status, 200);
    const seen = json.document.seen;
    // the entry is named relative to where it runs, with the other files beside it
    assert.equal(json.document.path, "top.sp");
    assert.deepEqual(seen.files, ["_pool_", "cell.inc", "top.sp"]);
    assert.equal(seen.text, files["/top.sp"]);
    // the pool's directories are searched, the packages' as packages; the options are the server's then the page's
    assert.ok(seen.args.some((a, i) => a === "--package-dir" && /packages$/.test(seen.args[i + 1])));
    assert.ok(seen.args.some((a, i) => a === "--path" && /lib$/.test(seen.args[i + 1])));
    assert.equal(seen.args[seen.args.indexOf("--solver-options") + 1], "time_limit=none;beam=8");
    assert.deepEqual(json.report, { levels: [] });
    // nothing is left behind
    assert.deepEqual(await readdir(join(s.dir, "work")), []);
  } finally { await s.close(); }
});

test("a netlist the solver fails on gives its message, and nothing is left behind", async () => {
  const s = await serve();
  try {
    const { status, json } = await call(s.port, "POST", SOLVE, { entry: "/top.sp", files: { "/top.sp": "fail" } });
    assert.equal(status, 400);
    assert.equal(json.error, "error: the netlist fails");
    assert.deepEqual(await readdir(join(s.dir, "work")), []);
  } finally { await s.close(); }
});

test("a request with no entry, or a path that goes up, is refused", async () => {
  const s = await serve();
  try {
    assert.equal((await call(s.port, "POST", SOLVE, { files: {} })).status, 400);
    assert.equal((await call(s.port, "POST", SOLVE, { entry: "/top.sp", files: {} })).status, 400);
    assert.equal((await call(s.port, "POST", SOLVE, { entry: "/../top.sp", files: { "/../top.sp": "x" } })).status, 400);
    assert.deepEqual(await readdir(s.dir).then((l) => l.filter((n) => n === "top.sp")), []);
  } finally { await s.close(); }
});

test("only the page of this server may use it: another address, another origin, a form body and a wrong method are refused", async () => {
  const s = await serve();
  try {
    const ok = { entry: "/top.sp", files: { "/top.sp": "* x" } };
    assert.equal((await call(s.port, "GET", "/rnl-solver", undefined, { Host: "evil.example" })).status, 403);
    assert.equal((await call(s.port, "POST", SOLVE, ok, { Origin: "http://evil.example" })).status, 403);
    assert.equal((await call(s.port, "POST", SOLVE, ok, { "Sec-Fetch-Site": "cross-site" })).status, 403);
    assert.equal((await call(s.port, "POST", SOLVE, ok, { "Content-Type": "text/plain" })).status, 415);
    assert.equal((await call(s.port, "GET", SOLVE)).status, 405);
    assert.equal((await call(s.port, "POST", "/rnl-solver", ok)).status, 405);
    // the page's own origin and a same-origin fetch do
    assert.equal((await call(s.port, "POST", SOLVE, ok, { Origin: `http://127.0.0.1:${s.port}`, "Sec-Fetch-Site": "same-origin" })).status, 200);
    // a request for something else is not the service's
    assert.equal((await call(s.port, "GET", "/index.html")).status, 404);
  } finally { await s.close(); }
});

test("a request that is dropped stops the solver and removes the directory", async () => {
  const s = await serve();
  const started = join(s.dir, "started");
  process.env.FAKE_STARTED = started;
  try {
    await new Promise((resolve, reject) => {
      const req = request({ host: "127.0.0.1", port: s.port, method: "POST", path: SOLVE, headers: { "Content-Type": "application/json" } });
      req.on("error", () => {});
      req.end(JSON.stringify({ entry: "/top.sp", files: { "/top.sp": "slow" } }));
      const wait = async () => {
        for (let k = 0; k < 100; k++) {
          if ((await readdir(s.dir)).includes("started")) { req.destroy(); return resolve(); }
          await new Promise((r) => setTimeout(r, 50));
        }
        reject(new Error("the solver did not start"));
      };
      wait();
    });
    for (let k = 0; k < 100 && (await readdir(join(s.dir, "work"))).length > 0; k++) await new Promise((r) => setTimeout(r, 50));
    assert.deepEqual(await readdir(join(s.dir, "work")), []);
  } finally { delete process.env.FAKE_STARTED; await s.close(); }
});

test("the paths of the page's workspace become paths under the solve's directory, or nothing", () => {
  assert.equal(insidePath("/top.sp"), "top.sp");
  assert.equal(insidePath("/a/b.sp"), "a/b.sp");
  assert.equal(insidePath("/<pool>/lib/x.inc"), "_pool_/lib/x.inc");
  for (const bad of ["top.sp", "/", "/a/../b", "/../b", "/a//b", "/a/./b", "/a\0b", 3, null]) assert.equal(insidePath(bad), null, String(bad));
});

test("a path that is not the rnl command is not served", async () => {
  await assert.rejects(runVersion(process.execPath.replace(/node(\.exe)?$/, "no-such-command")), /cannot be run/);
});
