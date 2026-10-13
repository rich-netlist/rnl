// The JavaScript modules on their own: core.js builds from memory, lists what is missing, throws the
// core's errors and survives a panic; client.js collects the files a document needs and runs the
// core in a Worker, here a Node worker thread.
//
//   node bindings/runtime-wasm/tests/core.mjs [DIR]
//
// DIR is what build.mjs wrote, dist/ by default. The panic tests need a module built with the
// test-panic feature, and are skipped without it:
//
//   node bindings/runtime-wasm/build.mjs --features test-panic --out target/wasm-test
//   node bindings/runtime-wasm/tests/core.mjs target/wasm-test

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const dist = pathToFileURL(`${resolve(process.argv[2] ?? resolve(here, "../dist"))}/`);
const { loadCore } = await import(new URL("core.js", dist));
const { collect, startCore } = await import(new URL("client.js", dist));
const wasm = new URL("rnl_runtime_wasm.wasm", dist);

const NETLIST = `* untitled
R1 a 0 1k
C1 a 0 1p
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   #R1 @PLACE(0, 0);
*   #C1 @PLACE(4, 0)
* }
.end
`;
const untitled = { entry: "untitled-1", files: { "untitled-1": NETLIST } };

const core = await loadCore(wasm);
/** Whether the module panics on request: built with the test-panic feature. */
const panics = (() => {
  try {
    core.build({ ...untitled, panic: "probe" });
    return false;
  } catch {
    return true;
  }
})();

test("an untitled text is built from memory", () => {
  const d = core.build(untitled);
  assert.equal(d.path, "untitled-1");
  assert.equal(d.dialect, "spice");
  assert.equal(d.files[0].text, NETLIST);
  assert.equal(d.levels.length, 1);
  assert.ok(d.levels[0].svg.startsWith("<svg"));
  assert.equal(d.levels[0].hits.filter((h) => h.kind === "box").length, 2);
  assert.deepEqual(d.missing, []);
});

test("the module can be given by URL, as a response, as bytes or compiled", async () => {
  const bytes = await readFile(wasm);
  const response = new Response(bytes, { headers: { "content-type": "application/wasm" } });
  const buffer = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
  const forms = [wasm, wasm.href, response, new Response(bytes), bytes, buffer, await WebAssembly.compile(bytes)];
  for (const form of forms) assert.equal((await loadCore(form)).build(untitled).path, "untitled-1");
  await assert.rejects(loadCore(new Response("", { status: 404 })), /could not be loaded/);
});

test("a missing include is listed in Document.missing", () => {
  const top = `* top\n.include "lib/parts.inc"\nX1 a b part\n.end\n`;
  const d = core.build({ entry: "/w/top.sp", files: { "/w/top.sp": top } });
  assert.deepEqual(d.missing, ["/w/lib/parts.inc"]);
  assert.ok(d.diagnostics.some((x) => x.severity === "error" && x.message.includes("lib/parts.inc cannot be found")));
});

test("an error is thrown with the core's message", () => {
  assert.throws(() => core.build({ entry: "b.sp", files: { "a.sp": NETLIST } }), { message: "cannot read b.sp" });
  assert.throws(() => core.build({ files: {} }), /^Error: the request is not valid: missing field `entry`/);
  // and the core still answers
  assert.equal(core.build(untitled).path, "untitled-1");
});

test("a panic is thrown with its message, and a new instance answers", { skip: !panics && "no test-panic feature" }, () => {
  const Instance = WebAssembly.Instance;
  let made = 0;
  WebAssembly.Instance = function (...args) {
    made++;
    return new Instance(...args);
  };
  try {
    assert.throws(() => core.build({ ...untitled, panic: "boom" }), (e) => {
      assert.match(e.message, /^The core stopped: panicked at .*lib\.rs:\d+:\d+:\nboom$/);
      assert.ok(e.cause instanceof WebAssembly.RuntimeError);
      return true;
    });
    assert.equal(made, 1);
    assert.equal(core.build(untitled).path, "untitled-1");
    assert.equal(made, 1);
  } finally {
    WebAssembly.Instance = Instance;
  }
});

test("collect reads what is missing until nothing new can be read", async () => {
  const disk = new Map([
    ["/w/top.sp", `* top\n.include "a.inc"\nX1 p q a\n.end\n`],
    ["/w/a.inc", `* a\n.include "b.inc"\n.include "none.inc"\n.subckt a p q\nX1 p q b\n.ends\n`],
    ["/w/b.inc", `* b\n.subckt b p q\nR1 p q 1k\n.ends\n`],
  ]);
  const asked = [];
  const read = async (path) => (asked.push(path), disk.get(path) ?? null);
  let builds = 0;
  const counted = { build: (request) => (builds++, core.build(request)) };

  const { document, files } = await collect(counted, { entry: "/w/top.sp" }, read);
  assert.deepEqual(Object.keys(files), ["/w/top.sp", "/w/a.inc", "/w/b.inc"]);
  assert.deepEqual(document.missing, ["/w/none.inc"]);
  assert.deepEqual(document.files.map((f) => f.path), ["top.sp", "a.inc", "b.inc"]);
  assert.deepEqual(asked.sort(), ["/w/a.inc", "/w/b.inc", "/w/none.inc", "/w/top.sp"]);
  assert.equal(builds, 3);

  // the files known so far are not read again
  asked.length = 0;
  const again = await collect(core, { entry: "/w/top.sp", files }, read);
  assert.deepEqual(asked, ["/w/none.inc"]);
  assert.deepEqual(again.document, document);

  await assert.rejects(collect(core, { entry: "/w/none.sp" }, read), { message: "cannot read /w/none.sp" });
});

test("the core runs in a Worker", async () => {
  const worker = startCore();
  try {
    const entries = ["/a/one.sp", "/b/two.sp", "/c/three.sp"];
    const built = await Promise.all(entries.map((entry) => worker.build({ entry, files: { [entry]: NETLIST } })));
    assert.deepEqual(built.map((d) => d.path), entries);
    assert.deepEqual(built[0].levels, core.build({ entry: "/a/one.sp", files: { "/a/one.sp": NETLIST } }).levels);
    await assert.rejects(worker.build({ entry: "b.sp", files: {} }), { message: "cannot read b.sp" });
    if (panics) {
      await assert.rejects(worker.build({ ...untitled, panic: "boom" }), /^Error: The core stopped: panicked at/);
    }
    const { document } = await collect(worker, { entry: "/w/top.sp" }, async (path) => (path === "/w/top.sp" ? NETLIST : null));
    assert.equal(document.path, "/w/top.sp");
  } finally {
    worker.terminate();
  }
  await assert.rejects(worker.build(untitled), { message: "The core was stopped." });
});
