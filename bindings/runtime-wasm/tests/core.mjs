// The JavaScript modules on their own: core.js builds from memory, lists what is missing, throws the
// core's errors and survives a panic or running out of memory; client.js collects the files a
// document needs, decodes them as rnl does, and runs the core in a Worker, here a Node worker thread.
//
//   node bindings/runtime-wasm/tests/core.mjs [DIR]
//
// DIR is what build.mjs wrote, dist/ by default. The tests that stop the core need a module built
// with the test-panic feature, and are skipped without it:
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
const { collect, decodeText, startCore } = await import(new URL("client.js", dist));
const wasm = new URL("rnl_runtime_wasm.wasm", dist);
const char = (...codes) => String.fromCharCode(...codes);

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
  assert.equal(d.levels[0].empty, false);
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

test("a lone surrogate becomes U+FFFD, as TextEncoder makes it", () => {
  const [high, low, replacement, backslash] = [char(0xd800), char(0xdc00), char(0xfffd), char(0x5c)];
  const d = core.build({ entry: `/w/${low}.sp`, files: { [`/w/${low}.sp`]: `* ${high}\n.end\n` } });
  assert.equal(d.path, `/w/${replacement}.sp`);
  assert.equal(d.files[0].text, `* ${replacement}\n.end\n`);
  // a backslash before ud800 is text, and a pair of surrogates is a character
  const kept = `* ${backslash}ud800 ${backslash}${backslash}ud800 ${high}${low}\n.end\n`;
  assert.equal(core.build({ entry: "a.sp", files: { "a.sp": kept } }).files[0].text, kept);
});

test("the same request gives the same Document every time", async () => {
  const path = resolve(here, "../../../tests/conformance/cases/16-ties.sp");
  const request = { entry: "/w/16-ties.sp", files: { "/w/16-ties.sp": await readFile(path, "utf8") } };
  const first = core.build(request);
  for (let k = 0; k < 6; k++) assert.deepEqual(core.build(request), first);
  assert.ok(first.diagnostics.some((d) => d.message.includes("did you mean $ABCV?")));
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

test("running out of memory is thrown as such, and a new instance answers", { skip: !panics && "no test-panic feature" }, () => {
  assert.throws(() => core.build({ ...untitled, out_of_memory: true }), (e) => {
    assert.match(e.message, /^The core stopped: it ran out of memory, with \d+ MB in use, when it asked for 1073741824 bytes more$/);
    assert.ok(e.cause instanceof WebAssembly.RuntimeError);
    return true;
  });
  assert.equal(core.build(untitled).path, "untitled-1");
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
    // the request is sent as JSON text, so what loadCore's build takes the Worker takes
    const proxy = { ...untitled, files: new Proxy(untitled.files, {}) };
    assert.deepEqual(await worker.build(proxy), core.build(proxy));
    assert.equal((await worker.build({ ...untitled, done: () => {} })).path, "untitled-1");
    await assert.rejects(worker.build({ ...untitled, size: 1n }), TypeError);
    assert.equal((await worker.build(untitled)).path, "untitled-1");
  } finally {
    worker.terminate();
  }
  await assert.rejects(worker.build(untitled), { message: "The core was stopped." });
});

test("decodeText reads bytes as rnl reads a file", () => {
  const encoded = (text) => new TextEncoder().encode(text);
  // UTF-8, without a byte order mark at the start
  assert.equal(decodeText(encoded(`${char(0xfeff)}* ${char(0xb5)}\n`)), `* ${char(0xb5)}\n`);
  assert.equal(decodeText(encoded(`* ${char(0xfeff)}\n`).buffer), `* ${char(0xfeff)}\n`);
  // not UTF-8: Latin-1, each byte the character of that code, also from 0x80 to 0x9f
  const bytes = new Uint8Array([0x0a, 0x2a, 0x20, 0xb5, 0xb0, 0x80, 0x9f, 0x0a]);
  assert.equal(decodeText(bytes.subarray(1)), `* ${char(0xb5, 0xb0, 0x80, 0x9f)}\n`);
  assert.equal(decodeText(bytes.buffer), `\n* ${char(0xb5, 0xb0, 0x80, 0x9f)}\n`);
  assert.equal(decodeText(new Uint8Array(70000).fill(0xe9)), char(0xe9).repeat(70000));
});
