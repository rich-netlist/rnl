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
import { copyFile, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
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
  assert.deepEqual(
    first.diagnostics.map(({ severity, file, line, col, message }) => [severity, file, line, col, message]),
    [["error", "16-ties.sp", 32, 21, "$ABCZ cannot be found, did you mean $ABCV? (6.6)"]],
  );
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

const RC = "\n.subckt rc VSS\nR1 in VSS 1k\n.ends rc\n";
const rcWrite = (ops) => ({ write: { dialect: "spice", netlist: RC, author: "test", ops }, svg: false });
const placeR1 = { level: "subckt:rc", op: "set", object: "#R1", at: [2, 0] };

test("a write gives the netlist written and its Document", () => {
  const { text, document } = core.write(rcWrite([placeR1]));
  assert.equal(
    text,
    '\n* @RNL(type="header") { @VERSION("0.3")}\n.subckt rc VSS\nR1 in VSS 1k\n* @RNL(author="test") {\n*   #R1 @PLACE(2, 0)\n* }\n.ends rc\n',
  );
  assert.equal(document.path, "/rnl-edit/netlist.sp");
  assert.equal(document.files[0].text, text);
  assert.deepEqual(document.diagnostics.filter((d) => d.severity === "error"), []);
  // an op that cannot be written is the error; a build and a write each want their own answer
  assert.throws(() => core.write(rcWrite([{ ...placeR1, level: "subckt:none" }])), { message: "op 0 (set): the netlist has no subcircuit none" });
  assert.throws(() => core.build(rcWrite([placeR1])), /the request is not a build/);
  assert.throws(() => core.write(untitled), /the request is not a write/);
  assert.ok("written" in core.respond(rcWrite([placeR1])));
});

test("parts ask for a snapshot, through the core and through the Worker", async () => {
  const request = { ...untitled, parts: { drawing: false, host: true, model: true } };
  const s = core.snapshot(request);
  assert.equal(s.schema, "rnl-snapshot/1");
  assert.equal(s.drawing, undefined);
  assert.deepEqual(s.levels[0].host.instances.map((i) => i.id), ["#R1", "#C1"]);
  const instances = s.levels[0].model.objects.filter((o) => o.kind === "instance");
  assert.deepEqual(instances.map((o) => [o.id, o.at.value]), [["#R1", [0, 0]], ["#C1", [4, 0]]]);
  // the standard package's ground stubs for the unwired pins on 0, by where its rule is written
  assert.ok(s.levels[0].model.objects.some((o) => o.definition === "$GND_STUB" && o.id.startsWith("@")));
  assert.throws(() => core.build(request), /the request is not a build without parts/);
  assert.throws(() => core.snapshot(untitled), /the request is not a build with parts/);
  const worker = startCore();
  try {
    assert.deepEqual(await worker.snapshot(request), s);
  } finally {
    worker.terminate();
  }
});

test("the drawings of the objects without a position are kept for a request, through the core and through the Worker", async () => {
  const text = "* some left\n.model nmos nmos\nR1 a b 1k\nC1 b 0 1p\nM1 d g s 0 nmos\n* @RNL { #R1 @PLACE(0, 0) }\n.end\n";
  const request = { entry: "/w/a.sp", files: { "/w/a.sp": text }, primitives: true };
  const carried = core.build(request);
  const listed = carried.levels[0].unplaced;
  assert.ok(listed.length >= 2 && listed.every((u) => u.svg && u.primitives));
  const asked = (items, names) => ({ drawings: { level: 0, items, names } });
  const names = listed.map((u) => u.name);
  const all = listed.map((_, k) => k);
  // nothing is kept by a build that does not ask for it
  assert.throws(() => core.drawings(asked(all, names)), /lazy_unplaced/);
  // the Document without the drawings of the objects, and those on request, are the Document with them
  const lazy = core.build({ ...request, lazy_unplaced: true });
  assert.ok(lazy.levels[0].unplaced.every((u) => u.svg === undefined && u.primitives === undefined));
  assert.deepEqual(lazy.levels[0].unplaced.map((u, k) => ({ ...u, svg: listed[k].svg, primitives: listed[k].primitives })), listed);
  const drawn = core.drawings(asked(all, names));
  assert.deepEqual(drawn, listed.map((u) => ({ svg: u.svg, primitives: u.primitives })));
  assert.deepEqual(core.drawings(asked([1, 0])), [drawn[1], drawn[0]]);
  // what is not there, or is of another build, is refused with the core's message
  assert.throws(() => core.drawings(asked([99], [])), /99/);
  assert.throws(() => core.drawings(asked([0], ["R9"])), { message: /built again/ });
  assert.throws(() => core.build(asked(all, names)), /the request is not a build without parts/);
  assert.ok("drawings" in core.respond(asked(all, names)));
  const worker = startCore();
  try {
    await assert.rejects(worker.drawings(asked(all, names)), /lazy_unplaced/);
    const built = await worker.build({ ...request, lazy_unplaced: true });
    assert.deepEqual(built, lazy);
    assert.deepEqual(await worker.drawings(asked(all, names)), drawn);
    await assert.rejects(worker.drawings(asked([0], ["R9"])), { message: /built again/ });
  } finally {
    worker.terminate();
  }
});

test("the Worker writes, and gives every request the packages its build brings", async () => {
  const plain = startCore();
  try {
    assert.equal((await plain.write(rcWrite([placeR1]))).text, core.write(rcWrite([placeR1])).text);
    await assert.rejects(plain.write(rcWrite([{ ...placeR1, op: "turn" }])), { message: 'op 0: unknown op "turn"' });
  } finally {
    plain.terminate();
  }
  // a copy of the build that brings one package
  const to = await mkdtemp(join(tmpdir(), "rnl-core-"));
  try {
    for (const name of ["core.js", "worker.js", "rnl_runtime_wasm.wasm", "package.json"]) await copyFile(new URL(name, dist), join(to, name));
    const acme = '@RNL(type="header") { @VERSION("0.2"); @PACKAGE(name="acme", version="1.0")}';
    const files = JSON.stringify({ "/rnl-bundled/packages/acme.rnl": acme });
    await writeFile(join(to, "packages.js"), `export const files = ${files};\nexport const packageDirs = ["/rnl-bundled/packages"];\n`);
    const header = '* t\n* @RNL(type="header") { @VERSION("0.2"); @USE-PACKAGE(name="acme", version="1.0")}\nR1 a 0 1k\n';
    const request = { entry: "/w/t.sp", files: { "/w/t.sp": header } };
    const errors = (d) => d.diagnostics.filter((x) => x.severity === "error").map((x) => x.message);
    assert.notDeepEqual(errors(core.build(request)), []);
    const worker = startCore({ worker: pathToFileURL(join(to, "worker.js")) });
    try {
      assert.deepEqual(errors(await worker.build(request)), []);
      const use = { level: "subckt:rc", op: "use-package", name: "acme", version: "1.0" };
      assert.deepEqual(errors((await worker.write(rcWrite([use, placeR1]))).document), []);
    } finally {
      worker.terminate();
    }
  } finally {
    await rm(to, { recursive: true, force: true });
  }
});

test("a catalog request lists the packages, through the core and through the Worker", async () => {
  const c = core.catalog({ catalog: {} });
  assert.deepEqual(Object.keys(c), ["packages", "symbols", "definitions", "sets"]);
  const resistor = c.symbols.find((s) => s.symbol === "$RESISTOR");
  assert.deepEqual([resistor.package, resistor.version, resistor.file], ["rnl-std-basic", "0.3", "rnl-std-basic.rnl"]);
  assert.deepEqual(resistor.pins, [
    { terminal: 1, at: [0, 2], direction: "up" },
    { terminal: 2, at: [0, -2], direction: "down" },
  ]);
  assert.deepEqual(resistor.box, [-1, -2, 1, 2]);
  assert.deepEqual(resistor.labels.map((l) => l.key), ["name", "value"]);
  assert.equal(c.symbols.find((s) => s.symbol === "$GROUND_SYMBOL").definition, "$GROUND");
  assert.equal(c.definitions.find((d) => d.definition === "$GROUND").symbol, "$GROUND_SYMBOL");
  // of each name the newest version: rnl-std 0.3, not 0.2
  const versions = (catalog, name) => catalog.packages.filter((p) => p.name === name).map((p) => p.version);
  assert.deepEqual(versions(c, "rnl-std"), ["0.3"]);
  assert.deepEqual(versions(core.catalog({ catalog: { all_versions: true } }), "rnl-std"), ["0.2", "0.3"]);
  // a request's own package, by name
  const acme = '@RNL(type="header") { @VERSION("0.3"); @PACKAGE(name="acme", version="1.0")}\n@RNL { $ACME = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1, direction="up") } }\n';
  const own = { catalog: { packages: ["acme"] }, files: { "/p/acme.rnl": acme }, package_dirs: ["/p"] };
  assert.deepEqual(core.catalog(own).symbols.map((s) => [s.symbol, s.package, s.box]), [["$ACME", "acme", [0, 0, 0, 0]]]);
  // a catalog request and a build each want their own answer
  assert.throws(() => core.build({ catalog: {} }), /the request is not a build without parts/);
  assert.throws(() => core.catalog(untitled), /the request is not a catalog request/);
  assert.ok("catalog" in core.respond({ catalog: {} }));
  // in a Node worker thread, as loadCore gives it
  const worker = startCore();
  try {
    assert.deepEqual(await worker.catalog({ catalog: {} }), c);
    assert.deepEqual(await worker.catalog(own), core.catalog(own));
    await assert.rejects(worker.catalog({ catalog: { all_versions: "yes" } }), /^Error: the request is not valid: /);
  } finally {
    worker.terminate();
  }
});

test("a formats request lists the formats and detects files, through the core and through the Worker", async () => {
  const request = { formats: { detect: { "/w/a.inc": "* a\n", "/w/b.lib": "simulator lang=spectre\n" } } };
  const f = core.formats(request);
  assert.deepEqual(f.list.map((x) => [x.name, x.title, x.fallback]), [["spice", "SPICE", true], ["spectre", "Spectre", false]]);
  assert.deepEqual(f.detected, { "/w/a.inc": "spice", "/w/b.lib": "spectre" });
  // a build that sets a file's format is a build, not a formats request
  const files = {
    "/w/top.sp": '* top\n.include "cells.scs"\nX1 a 0 amp\n.end\n',
    "/w/cells.scs": "simulator lang=spectre\nsubckt amp (p n)\nR1 (p n) resistor r=1k\nends amp\n",
  };
  const d = core.build({ entry: "/w/top.sp", files, formats: { "/w/cells.scs": "spice" } });
  assert.deepEqual(d.files.map((x) => [x.format, x.default_format]), [["spice", "spice"], ["spice", "spectre"]]);
  assert.deepEqual(d.formats, f.list);
  assert.throws(() => core.formats(untitled), /the request is not a formats request/);
  const worker = startCore();
  try {
    assert.deepEqual(await worker.formats(request), f);
  } finally {
    worker.terminate();
  }
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

const canvasPath = "/library/analog-canvas-extensions.rnl";
const { publicPackageText, publicPackagesAvailable } = await import("../../../scripts/packages.mjs");
const hasPublicPackages = publicPackagesAvailable(["analog-canvas-extensions"]);
const canvasText = hasPublicPackages ? publicPackageText("analog-canvas-extensions") : "";
const canvasNetlist = `* optional canvas
* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="analog-canvas-extensions", version="0.1") }
R1 a 0 1k
* @RNL { #R1 @SET-SYMBOL $CANVAS_VCVS_DIAMOND; #R1 @PLACE(0, 0) }
.end
`;
const canvasRequest = { entry: "/canvas.sp", files: { "/canvas.sp": canvasNetlist, [canvasPath]: canvasText } };
test("the downloaded Canvas package is not built-in or searched merely because it is in files", { skip: !hasPublicPackages }, () => {
  const doc = core.build(canvasRequest);
  assert.ok(doc.diagnostics.some(d => d.severity === "error" && d.message.includes("package analog-canvas-extensions 0.1 cannot be found")));
  assert.ok(core.catalog({ catalog: {} }).packages.every(p => p.name.startsWith("rnl-std") || p.name === "rnl-zh"));
});
test("the downloaded Canvas package resolves when explicitly placed in the pool", { skip: !hasPublicPackages }, () => {
  const doc = core.build({ ...canvasRequest, pool: [canvasPath] });
  assert.deepEqual(doc.diagnostics.filter(d => d.severity === "error"), []);
  assert.ok(doc.levels[0].svg.length > 0);
});
