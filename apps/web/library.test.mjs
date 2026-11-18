import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { publishLibrary } from "./library.mjs";

test("the published library describes every package and separates built-ins from optional packages", () => {
  const out = mkdtempSync(join(tmpdir(), "rnl-library-"));
  try {
    const index = publishLibrary(fileURLToPath(new URL("../../libraries", import.meta.url)), out);
    const optional = index.filter((p) => !p.builtin);
    assert.deepEqual(optional, []);
    assert.ok(index.every((p) => p.path.endsWith(".rnl")));
    assert.ok(index.filter((p) => p.builtin).every((p) => p.path.startsWith("standard/") && (p.name.startsWith("rnl-std") || p.name === "rnl-zh" || p.name === "alias-zh")));
    for (const p of index) assert.ok(readFileSync(join(out, p.path), "utf8").includes(`name="${p.name}"`));
    const bundled = JSON.parse(readFileSync(join(out, "bundled.json"), "utf8"));
    for (const path of ["0.4/rnl-std-all.rnl", "0.4/rnl-std-basic.rnl", "0.4/alias-zh.rnl"])
      assert.ok(bundled[`<bundled>/${path}`], path);
    assert.equal(Object.keys(bundled).length, 16);
    assert.ok(Object.keys(bundled).every((path) => /^<bundled>\/0\.4\/(rnl-|alias-zh)/.test(path)));
  } finally { rmSync(out, { recursive: true, force: true }); }
});

test("every series under libraries/standard is published, whether its standard is released or not", () => {
  const root = mkdtempSync(join(tmpdir(), "rnl-library-"));
  try {
    for (const series of ["0.4", "0.5"]) {
      mkdirSync(join(root, "libraries", "standard", series), { recursive: true });
      writeFileSync(join(root, "libraries", "standard", series, "rnl-std-all.rnl"),
        `@RNL(type="header") { @VERSION("${series}"); @PACKAGE(name="rnl-std-all", version="${series}")}\n`);
    }
    // a released standard of 0.4 and a draft of 0.5: both series are bundled by the core, and both published
    mkdirSync(join(root, "spec", "0.4"), { recursive: true });
    writeFileSync(join(root, "spec", "0.4", "RELEASED"), "released\n");
    const out = mkdtempSync(join(root, "out-"));
    publishLibrary(join(root, "libraries"), out);
    assert.deepEqual(Object.keys(JSON.parse(readFileSync(join(out, "bundled.json"), "utf8"))),
      ["<bundled>/0.4/rnl-std-all.rnl", "<bundled>/0.5/rnl-std-all.rnl"]);
  } finally { rmSync(root, { recursive: true, force: true }); }
});
