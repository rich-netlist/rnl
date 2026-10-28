// The manifest reader (js/manifest.js) with a fetch of its own: what it gives a build, its
// checks, and the file each error names. Needs no WebAssembly build.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { test } from "node:test";
import { checkManifest, loadManifest, ManifestError, MANIFEST_LIMITS, readManifest, writeManifest } from "../js/manifest.js";

const BASE = "https://raw.githubusercontent.com/someone/design/0123456789abcdef0123456789abcdef01234567/examples/amp/";
const sha = (text) => createHash("sha256").update(text).digest("hex");
const PACKAGE_URL = "https://rich-netlist.github.io/rnl-packages/eda-style/1.0/eda-style.rnl";
const PACKAGE = "* @RNL(type=\"header\") { @PACKAGE(name=\"eda-style\", version=\"1.0\") }\n";
const FILES = {
  "top.scs": "simulator lang=spectre\ninclude \"cells/amp.scs\"\nX1 (a b) amp\n",
  "cells/amp.scs": "subckt amp (p n)\nends amp\n",
  "lib/pdk.rnl": "* @RNL(type=\"header\") { @PACKAGE(name=\"pdk\", version=\"1.0\") }\n",
  "models/nmos.lib": "* models\n",
};

function manifest(change = {}) {
  return {
    format: 1, id: "amp-demo", title: { en: "Amplifier", zh: "放大器" }, author: "someone",
    source: { repo: "someone/design", commit: "0123456789abcdef0123456789abcdef01234567",
      url: "https://github.com/someone/design/tree/0123456789abcdef0123456789abcdef01234567/examples/amp" },
    license: "MIT", base: BASE, entry: "top.scs", level: "amp",
    files: [
      ...Object.entries(FILES).map(([path, text]) => ({ path, sha256: sha(text), size: Buffer.byteLength(text) })),
      { path: ".packages/eda-style/1.0/eda-style.rnl", sha256: sha(PACKAGE), size: Buffer.byteLength(PACKAGE), url: PACKAGE_URL },
    ],
    pool: ["models/nmos.lib", "lib/pdk.rnl", ".packages/eda-style/1.0/eda-style.rnl"],
    ...change,
  };
}

/** A fetch that serves the manifest at /m.json, the files under BASE and the package; it counts its calls. */
function server(body, override = {}) {
  const calls = [];
  const fetch = async (url) => {
    calls.push(url);
    const text = url === "https://example.test/m.json" ? JSON.stringify(body)
      : url === PACKAGE_URL ? PACKAGE
      : url.startsWith(BASE) ? FILES[url.slice(BASE.length)] : undefined;
    const served = url in override ? override[url] : text;
    if (served == null) return new Response("missing", { status: 404, statusText: "Not Found" });
    return new Response(served);
  };
  return { fetch, calls };
}

test("a manifest gives every file under its id, its pool in order, entry and level", async () => {
  const { fetch, calls } = server(manifest());
  const progress = [];
  const read = await readManifest("https://example.test/m.json", { fetch, onProgress: (done, total) => progress.push([done, total]) });
  assert.equal(read.entry, "/amp-demo/top.scs");
  assert.equal(read.level, "amp");
  assert.deepEqual(read.title, { en: "Amplifier", zh: "放大器" });
  assert.equal(read.files["/amp-demo/cells/amp.scs"], FILES["cells/amp.scs"]);
  assert.equal(read.files["/amp-demo/.packages/eda-style/1.0/eda-style.rnl"], PACKAGE);
  assert.equal(Object.keys(read.files).length, 5);
  assert.deepEqual(read.pool, ["/amp-demo/models/nmos.lib", "/amp-demo/lib/pdk.rnl", "/amp-demo/.packages/eda-style/1.0/eda-style.rnl"]);
  // a file with a url of its own is fetched from there, never from base
  assert.ok(calls.includes(PACKAGE_URL));
  assert.ok(!calls.some((url) => url.startsWith(BASE) && url.includes(".packages")));
  assert.deepEqual(progress[0], [0, 5]);
  assert.deepEqual(progress.at(-1), [5, 5]);
});

test("a file whose bytes do not match its SHA-256 or size stops the reading and is named", async () => {
  const { fetch } = server(manifest(), { [`${BASE}cells/amp.scs`]: "subckt amp (p n)\nends amp\n*changed\n" });
  await assert.rejects(readManifest("https://example.test/m.json", { fetch }), (error) =>
    error instanceof ManifestError && error.file === "cells/amp.scs" && /bytes|SHA-256/.test(error.message));
  const tampered = manifest();
  tampered.files[1] = { ...tampered.files[1], sha256: "0".repeat(64) };
  await assert.rejects(readManifest("https://example.test/m.json", { fetch: server(tampered).fetch }),
    (error) => error.file === "cells/amp.scs" && /SHA-256/.test(error.message));
});

test("missing files, a missing manifest and a package that does not match are told", async () => {
  await assert.rejects(readManifest("https://example.test/m.json", { fetch: server(manifest(), { [`${BASE}models/nmos.lib`]: null }).fetch }),
    (error) => error.file === "models/nmos.lib" && /could not be fetched: 404/.test(error.message));
  await assert.rejects(readManifest("https://example.test/none.json", { fetch: server(manifest()).fetch }), /manifest could not be fetched: 404/);
  // the first file that fails stops the rest: nothing more is fetched after it
  const failing = server(manifest(), { [`${BASE}top.scs`]: null });
  await assert.rejects(readManifest("https://example.test/m.json", { fetch: failing.fetch, concurrency: 1 }), (error) => error.file === "top.scs");
  assert.equal(failing.calls.length, 2);
  // the caller's signal stops it too, with its reason
  const stopping = new AbortController();
  const slow = { fetch: async (url, options) => url.endsWith("m.json") ? server(manifest()).fetch(url)
    : new Promise((_, reject) => options.signal.addEventListener("abort", () => reject(options.signal.reason))) };
  const reading = readManifest("https://example.test/m.json", { ...slow, signal: stopping.signal });
  setTimeout(() => stopping.abort(new Error("stopped")), 10);
  await assert.rejects(reading, /stopped/);
  await assert.rejects(readManifest("https://example.test/m.json", { fetch: server(manifest(), { [PACKAGE_URL]: "tampered" }).fetch }),
    (error) => error.file === ".packages/eda-style/1.0/eda-style.rnl");
});

test("the checks refuse what a manifest must not hold", () => {
  const refused = (change, pattern) => assert.throws(() => checkManifest(manifest(change)), pattern);
  const file = (path, size = 1) => ({ path, sha256: "a".repeat(64), size });
  refused({ format: 2 }, /update the viewer/);
  refused({ base: "ftp://example.test/design/" }, /base must be the https address of a directory/);
  refused({ base: "https://example.test/design" }, /base must be the https address of a directory/);
  refused({ base: "http://example.test/design/" }, /base must be the https address of a directory/);
  // http only on this computer, as a page in development has
  assert.equal(checkManifest(manifest({ base: "http://localhost:8080/design/" })).base, "http://localhost:8080/design/");
  assert.equal(checkManifest(manifest({ base: "http://127.0.0.1/design/" })).base, "http://127.0.0.1/design/");
  refused({ id: "../x" }, /id must be/);
  refused({ entry: "../top.scs" }, /must be a relative path within the manifest/);
  refused({ entry: "/top.scs" }, /not a relative path/);
  refused({ entry: "missing.scs" }, /not among the manifest's files/);
  refused({ files: [...manifest().files, manifest().files[0]] }, /twice/);
  refused({ files: Array.from({ length: MANIFEST_LIMITS.files + 1 }, (_, i) => file(`f${i}.sp`)) }, /more than 300/);
  refused({ files: [file("top.scs", MANIFEST_LIMITS.fileBytes + 1)] }, /bytes, more than/);
  refused({ files: Array.from({ length: 11 }, (_, i) => file(i ? `f${i}.sp` : "top.scs", MANIFEST_LIMITS.fileBytes)), pool: [] }, /files hold .* bytes, more than/);
  refused({ files: [file("top.scs"), { ...file(".packages/x/1/x.rnl"), url: "file:///x.rnl" }], pool: [] }, /not an https address/);
  refused({ formats: { "missing.scs": "spectre" } }, /not among the manifest's files/);
  refused({ formats: { "top.scs": 3 } }, /not a format's name/);
  refused({ paths: [{ path: "../lib" }] }, /must be a relative path within the manifest/);
  refused({ settings: { autoInclude: "yes" } }, /autoInclude/);
  // without a base, every file needs a url, unless the manifest is in an archive
  const unaddressed = manifest({ base: undefined });
  assert.throws(() => checkManifest(unaddressed), /has no "url" field and the manifest has no "base" URL/);
  assert.equal(checkManifest(unaddressed, { archive: true }).files.length, 5);
  // a caller that holds less gives smaller limits: a web page's project
  assert.throws(() => checkManifest(unaddressed, { archive: true, limits: { projectBytes: 10 } }), /bytes, more than 10\./);
  // an id is not needed: the entry's name is it then
  assert.equal(checkManifest(manifest({ id: undefined, entry: "top.scs" })).id, "top");
  refused({ pool: ["lib/none.rnl"] }, /not among the manifest's files/);
  refused({ pool: ["lib/../top.scs"] }, /must be a relative path within the manifest/);
  refused({ pool: ["lib/pdk.rnl", "lib/pdk.rnl"] }, /pool lists a file twice/);
  // no pool is an empty one
  assert.deepEqual(checkManifest(manifest({ pool: undefined })).pool, []);
});

test("a project's manifest reads its files from the archive, with their formats, paths and settings", async () => {
  const archive = Object.fromEntries(Object.entries(FILES).map(([path, text]) => [path, new TextEncoder().encode(text)]));
  archive[".packages/eda-style/1.0/eda-style.rnl"] = new TextEncoder().encode(PACKAGE);
  const json = manifest({ base: undefined, id: undefined, formats: { "cells/amp.scs": "spectre" }, paths: [{ path: "models", subfolders: true }], settings: { autoInclude: false } });
  const fetches = [];
  const read = await loadManifest(json, { readFile: async (path) => archive[path] ?? null, root: "D:/work/amp/", fetch: async (url) => { fetches.push(url); } });
  assert.equal(fetches.length, 0, "nothing is fetched, not even a file with a url");
  assert.equal(read.entry, "D:/work/amp/top.scs");
  assert.deepEqual(read.formats, { "D:/work/amp/cells/amp.scs": "spectre" });
  assert.deepEqual(read.paths, [{ path: "D:/work/amp/models", subfolders: true }]);
  assert.deepEqual(read.settings, { autoInclude: false });
  assert.deepEqual(read.bytes["D:/work/amp/models/nmos.lib"], archive["models/nmos.lib"]);
  delete archive["lib/pdk.rnl"];
  await assert.rejects(loadManifest(json, { readFile: async (path) => archive[path] ?? null }), (error) => error.file === "lib/pdk.rnl" && /not in the archive/.test(error.message));
});

test("writeManifest gives a manifest that reads back to the same files", async () => {
  const encode = (text) => new TextEncoder().encode(text);
  const written = await writeManifest({ archive: true, entry: "top.scs", level: "(top)", pool: ["models/nmos.lib"], formats: { "top.scs": "spectre" },
    settings: { autoInclude: true }, files: Object.entries(FILES).map(([path, text]) => ({ path, bytes: encode(text) })) });
  assert.equal(written.format, 1);
  assert.equal(written.files[0].sha256, sha(FILES["top.scs"]));
  assert.equal("paths" in written, false);
  const read = await loadManifest(JSON.parse(JSON.stringify(written)), { readFile: async (path) => encode(FILES[path]) });
  assert.equal(read.files["/top/cells/amp.scs"], FILES["cells/amp.scs"]);
  assert.deepEqual(read.formats, { "/top/top.scs": "spectre" });
  // a manifest of links needs a url for every file
  await assert.rejects(writeManifest({ entry: "top.scs", files: [{ path: "top.scs", bytes: encode("x") }] }), /has no "url" field/);
  const links = await writeManifest({ entry: "top.scs", files: [{ path: "top.scs", bytes: encode("x"), url: "https://example.test/top.scs" }] });
  assert.equal(links.files[0].url, "https://example.test/top.scs");
});

test("a file is decoded as client.js decodeText decodes it: UTF-8 without a BOM, else Latin-1", async () => {
  const { decodeText } = await import("../js/client.js");
  const samples = [
    [0xef, 0xbb, 0xbf, 0x2a, 0x20, 0x41], // UTF-8 with a BOM
    [0xef, 0xbb, 0xbf, 0xef, 0xbb, 0xbf, 0x41], // a BOM, then U+FEFF as text
    [0xe7, 0x94, 0xb5, 0xe8, 0xb7, 0xaf], // UTF-8: 电路
    [0x2a, 0x20, 0xb5, 0xe7, 0xc2, 0xb7], // GBK / GB18030: * 电路, read as Latin-1
    [0x63, 0x61, 0x66, 0xe9], // Latin-1: café
    [0x41, 0xc3, 0x28, 0x42], // a byte UTF-8 does not allow there
    [0xff, 0xfe, 0x41, 0x00], // a UTF-16 BOM, which is not read as UTF-16
    [],
  ].map((b) => new Uint8Array(b));
  for (const bytes of samples) {
    const json = { format: 1, id: "d", entry: "a.sp", files: [{ path: "a.sp", sha256: sha(bytes), size: bytes.length }] };
    const read = await loadManifest(json, { readFile: async () => bytes });
    assert.equal(read.files["/d/a.sp"], decodeText(bytes));
  }
});

test("display settings are given as proposed, and those of another shape are left out and named", async () => {
  const display = {
    view: { center: [12.5, -4], scale: 2 }, background: "dark", colors: { component: "#2275e2", net: "red" },
    usages: false, currents: true, grid: "dots", zoomToFit: true,
  };
  const checked = checkManifest(manifest({ display }));
  assert.deepEqual(checked.display, { view: { center: [12.5, -4], scale: 2 }, background: "dark", colors: { component: "#2275e2" },
    usages: false, currents: true, grid: "dots", zoomToFit: true });
  // a colour is kept as written; a setting this module does not know is kept, for a newer viewer,
  // and named with those of another shape
  assert.deepEqual(checked.displayIgnored, ["colors.net", "zoomToFit"]);
  assert.deepEqual(checkManifest(manifest({ display: { view: { fit: true }, background: "sepia" } })).display, { view: { fit: true } });
  assert.deepEqual(checkManifest(manifest()).display, {});
  // a manifest written with them reads them back
  const encode = (text) => new TextEncoder().encode(text);
  const written = await writeManifest({ archive: true, entry: "top.scs", display: { background: "light" },
    files: Object.entries(FILES).map(([path, text]) => ({ path, bytes: encode(text) })) });
  const read = await loadManifest(written, { readFile: async (path) => encode(FILES[path]) });
  assert.deepEqual(read.display, { background: "light" });
});
