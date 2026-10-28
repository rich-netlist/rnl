// The viewer's manifest module is a copy of the runtime package's (bindings/runtime-wasm/js), which
// the desktop viewer cannot reach: the source is the package's, and the two must stay the same.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const NOTE = "// A copy of bindings/runtime-wasm/js/manifest.js: do not change it here; change that file and copy it here.\n";

test("apps/viewer-ui/manifest.js is bindings/runtime-wasm/js/manifest.js, with the line that says so", () => {
  const read = (path) => readFileSync(new URL(path, import.meta.url), "utf8").replace(/\r\n/g, "\n");
  assert.equal(read("../viewer-ui/manifest.js"), NOTE + read("../../bindings/runtime-wasm/js/manifest.js"),
    "apps/viewer-ui/manifest.js is a copy: change bindings/runtime-wasm/js/manifest.js and copy it to apps/viewer-ui/manifest.js, after the line that says it is a copy");
});
