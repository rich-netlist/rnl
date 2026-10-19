import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { publishLibrary } from "./library.mjs";

test("the published library describes every package and separates built-ins from optional packages", () => {
  const out = mkdtempSync(join(tmpdir(), "rnl-library-"));
  try {
    const index = publishLibrary(fileURLToPath(new URL("../../libraries", import.meta.url)), out);
    const optional = index.filter((p) => !p.builtin);
    assert.ok(optional.some((p) => p.name === "pdk-sky130" && p.version === "0.1"));
    assert.ok(optional.some((p) => p.name === "analog-canvas-extensions" && p.version === "0.1"));
    for (const name of ["eda-symbol", "eda-style"]) {
      assert.deepEqual(optional.filter((p) => p.name === name).map((p) => p.path), [`${name}/${name}.rnl`]);
    }
    assert.ok(index.every((p) => p.path.endsWith(".rnl")));
    assert.ok(index.filter((p) => p.builtin).every((p) => p.path.startsWith("standard/") && (p.name.startsWith("rnl-std") || p.name === "rnl-zh")));
    for (const p of index) assert.ok(readFileSync(join(out, p.path), "utf8").includes(`name="${p.name}"`));
    const bundled = JSON.parse(readFileSync(join(out, "bundled.json"), "utf8"));
    for (const path of ["0.2/rnl-std.rnl", "0.3/rnl-std.rnl"])
      assert.ok(bundled[`<bundled>/${path}`]);
    assert.ok(Object.keys(bundled).every((path) => /^<bundled>\/0\.[23]\/rnl-/.test(path)));
  } finally { rmSync(out, { recursive: true, force: true }); }
});
