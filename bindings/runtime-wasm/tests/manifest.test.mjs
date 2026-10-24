// The gallery manifest reader (js/manifest.js) with a fetch of its own: what it gives a build, its
// checks, and the file each error names. Needs no WebAssembly build.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { test } from "node:test";
import { checkManifest, ManifestError, MANIFEST_LIMITS, readManifest } from "../js/manifest.js";

const BASE = "https://raw.githubusercontent.com/someone/design/0123456789abcdef0123456789abcdef01234567/";
const sha = (text) => createHash("sha256").update(text).digest("hex");
const FILES = {
  "top.scs": "simulator lang=spectre\ninclude \"cells/amp.scs\"\nX1 (a b) amp\n",
  "cells/amp.scs": "subckt amp (p n)\nends amp\n",
  "cells/0.3/old.inc": "* versioned\n",
  "cells/deep/skip.inc": "* not in a search path\n",
  "lib/pdk.rnl": "* @RNL(type=\"header\") { @PACKAGE(name=\"pdk\", version=\"1.0\") }\n",
  "models/nmos.lib": "* models\n",
};
const PACKAGE_URL = "https://rich-netlist.github.io/rnl-packages/eda-style/1.0/eda-style.rnl";
const PACKAGE = "* @RNL(type=\"header\") { @PACKAGE(name=\"eda-style\", version=\"1.0\") }\n";

function manifest(change = {}) {
  return {
    format: 1, id: "amp-demo", title: { en: "Amplifier", zh: "放大器" }, author: "someone",
    source: { repo: "someone/design", commit: "0123456789abcdef0123456789abcdef01234567", url: "https://github.com/someone/design" },
    license: "MIT", base: BASE, entry: "top.scs", paths: ["models", "cells"], level: "amp",
    files: Object.entries(FILES).map(([path, text]) => ({ path, sha256: sha(text), size: Buffer.byteLength(text) })),
    packages: [{ path: "lib/pdk.rnl" }, { name: "eda-style", version: "1.0", url: PACKAGE_URL, sha256: sha(PACKAGE) }],
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

test("a manifest gives every file and package under its id, the pool in search order, entry and level", async () => {
  const { fetch } = server(manifest());
  const progress = [];
  const read = await readManifest("https://example.test/m.json", { fetch, onProgress: (done, total) => progress.push([done, total]) });
  assert.equal(read.entry, "/amp-demo/top.scs");
  assert.equal(read.level, "amp");
  assert.deepEqual(read.title, { en: "Amplifier", zh: "放大器" });
  assert.equal(read.files["/amp-demo/cells/amp.scs"], FILES["cells/amp.scs"]);
  assert.equal(Object.keys(read.files).length, Object.keys(FILES).length + 1);
  // models first, then cells and its version folder, never a deeper folder; then the packages
  assert.deepEqual(read.pool, [
    "/amp-demo/models/nmos.lib",
    "/amp-demo/cells/amp.scs",
    "/amp-demo/cells/0.3/old.inc",
    "/amp-demo/lib/pdk.rnl",
    "/amp-demo/.packages/eda-style/1.0/eda-style.rnl",
  ]);
  assert.deepEqual(read.packages, ["/amp-demo/lib/pdk.rnl", "/amp-demo/.packages/eda-style/1.0/eda-style.rnl"]);
  assert.deepEqual(progress[0], [0, 7]);
  assert.deepEqual(progress.at(-1), [7, 7]);
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
  await assert.rejects(readManifest("https://example.test/m.json", { fetch: server(manifest(), { [PACKAGE_URL]: "tampered" }).fetch }),
    (error) => error.file === "eda-style 1.0");
});

test("the checks refuse what the gallery's CI never writes", () => {
  const refused = (change, pattern) => assert.throws(() => checkManifest(manifest(change)), pattern);
  refused({ format: 2 }, /update the viewer/);
  refused({ base: "https://example.test/design/" }, /raw\.githubusercontent\.com/);
  refused({ id: "../x" }, /id must be/);
  refused({ entry: "../top.scs" }, /leaves the commit/);
  refused({ entry: "/top.scs" }, /not a relative path/);
  refused({ entry: "missing.scs" }, /not among the manifest's files/);
  refused({ paths: ["cells/../.."] }, /leaves the commit/);
  refused({ files: [...manifest().files, manifest().files[0]] }, /twice/);
  refused({ files: Array.from({ length: MANIFEST_LIMITS.files + 1 }, (_, i) => ({ path: `f${i}.sp`, sha256: "a".repeat(64), size: 1 })) }, /more than 300/);
  refused({ files: [{ path: "top.scs", sha256: "a".repeat(64), size: MANIFEST_LIMITS.bytes + 1 }] }, /bytes, more than/);
  refused({ packages: [{ name: "x", version: "1", url: "https://example.test/x.rnl", sha256: "a".repeat(64) }] }, /not on raw GitHub/);
  refused({ packages: [{ path: "lib/none.rnl" }] }, /not among the manifest's files/);
  // a trailing slash on a search path is accepted
  assert.deepEqual(checkManifest(manifest({ paths: ["cells/", ""] })).paths, ["cells", ""]);
});
