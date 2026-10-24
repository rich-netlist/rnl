// The manifest reader (js/manifest.js) with a fetch of its own: what it gives a build, its
// checks, and the file each error names. Needs no WebAssembly build.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { test } from "node:test";
import { checkManifest, ManifestError, MANIFEST_LIMITS, readManifest } from "../js/manifest.js";

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
  refused({ base: "https://example.test/design/" }, /raw\.githubusercontent\.com/);
  refused({ id: "../x" }, /id must be/);
  refused({ entry: "../top.scs" }, /leaves the commit/);
  refused({ entry: "/top.scs" }, /not a relative path/);
  refused({ entry: "missing.scs" }, /not among the manifest's files/);
  refused({ files: [...manifest().files, manifest().files[0]] }, /twice/);
  refused({ files: Array.from({ length: MANIFEST_LIMITS.files + 1 }, (_, i) => file(`f${i}.sp`)) }, /more than 300/);
  refused({ files: [file("top.scs", MANIFEST_LIMITS.fileBytes + 1)] }, /bytes, more than/);
  refused({ files: Array.from({ length: 11 }, (_, i) => file(i ? `f${i}.sp` : "top.scs", MANIFEST_LIMITS.fileBytes)), pool: [] }, /files hold .* bytes, more than/);
  refused({ files: [file("top.scs"), { ...file(".packages/x/1/x.rnl"), url: "https://example.test/x.rnl" }], pool: [] }, /not on raw GitHub/);
  refused({ pool: ["lib/none.rnl"] }, /not among the manifest's files/);
  refused({ pool: ["lib/../top.scs"] }, /leaves the commit/);
  refused({ pool: ["lib/pdk.rnl", "lib/pdk.rnl"] }, /pool lists a file twice/);
  // no pool is an empty one
  assert.deepEqual(checkManifest(manifest({ pool: undefined })).pool, []);
});
