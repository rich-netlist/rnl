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
    for (const path of ["0.2/rnl-std.rnl", "0.3/rnl-std.rnl"])
      assert.ok(bundled[`<bundled>/${path}`]);
    assert.ok(Object.keys(bundled).every((path) => /^<bundled>\/0\.[23]\/rnl-/.test(path)));
  } finally { rmSync(out, { recursive: true, force: true }); }
});

test("a series whose standard is not released is published only when RNL_DRAFT_SERIES names it", () => {
  const root = mkdtempSync(join(tmpdir(), "rnl-library-"));
  try {
    for (const series of ["0.3", "0.4"]) {
      const pkg = series === "0.4" ? "rnl-std-all" : "rnl-std";
      mkdirSync(join(root, "libraries", "standard", series), { recursive: true });
      writeFileSync(join(root, "libraries", "standard", series, `${pkg}.rnl`),
        `@RNL(type="header") { @VERSION("${series}"); @PACKAGE(name="${pkg}", version="${series}")}\n`);
    }
    mkdirSync(join(root, "spec", "0.3"), { recursive: true });
    writeFileSync(join(root, "spec", "0.3", "RELEASED"), "released\n");
    const published = (options) => {
      const out = mkdtempSync(join(root, "out-"));
      publishLibrary(join(root, "libraries"), out, options);
      return Object.keys(JSON.parse(readFileSync(join(out, "bundled.json"), "utf8")));
    };
    const both = ["<bundled>/0.3/rnl-std.rnl", "<bundled>/0.4/rnl-std-all.rnl"];
    const before = process.env.RNL_DRAFT_SERIES;
    try {
      delete process.env.RNL_DRAFT_SERIES;
      assert.deepEqual(published(), ["<bundled>/0.3/rnl-std.rnl"]);
      assert.deepEqual(published({ drafts: ["0.4"] }), both);
      process.env.RNL_DRAFT_SERIES = "0.4";
      assert.deepEqual(published(), both);
      assert.deepEqual(published({ drafts: [] }), ["<bundled>/0.3/rnl-std.rnl"]);
    } finally {
      if (before === undefined) delete process.env.RNL_DRAFT_SERIES;
      else process.env.RNL_DRAFT_SERIES = before;
    }
  } finally { rmSync(root, { recursive: true, force: true }); }
});
