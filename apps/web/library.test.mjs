import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { publishLibrary } from "./library.mjs";

/** The package files of a series folder by their `<bundled>/` paths, as the core lists a series it bundles. */
function series(libraries, name) {
  const dir = join(libraries, "standard", name);
  return Object.fromEntries(readdirSync(dir).filter((f) => f.endsWith(".rnl"))
    .map((f) => [`<bundled>/${name}/${f}`, readFileSync(join(dir, f), "utf8").replace(/\r\n/g, "\n")]));
}

test("the published library describes every package and separates built-ins from optional packages", () => {
  const out = mkdtempSync(join(tmpdir(), "rnl-library-"));
  try {
    const libraries = fileURLToPath(new URL("../../libraries", import.meta.url));
    const index = publishLibrary(libraries, out, series(libraries, "0.4"));
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

test("every series under libraries/standard is published, and the built-in ones are those the core lists", () => {
  const root = mkdtempSync(join(tmpdir(), "rnl-library-"));
  try {
    for (const name of ["0.4", "0.5"]) {
      mkdirSync(join(root, "libraries", "standard", name), { recursive: true });
      writeFileSync(join(root, "libraries", "standard", name, "rnl-std-all.rnl"),
        `@RNL(type="header") { @VERSION("${name}"); @PACKAGE(name="rnl-std-all", version="${name}")}\n`);
    }
    // a draft series 0.5, which the core bundles and reads but does not list: published, not built in
    const out = mkdtempSync(join(root, "out-"));
    const index = publishLibrary(join(root, "libraries"), out, series(join(root, "libraries"), "0.4"));
    assert.deepEqual(index.map((p) => [p.path, p.builtin]), [["standard/0.4/rnl-std-all.rnl", true], ["standard/0.5/rnl-std-all.rnl", false]]);
    assert.ok(readFileSync(join(out, "standard", "0.5", "rnl-std-all.rnl"), "utf8").includes('version="0.5"'));
    assert.deepEqual(Object.keys(JSON.parse(readFileSync(join(out, "bundled.json"), "utf8"))), ["<bundled>/0.4/rnl-std-all.rnl"]);
    // a package the core lists must be among the files
    assert.throws(() => publishLibrary(join(root, "libraries"), out, { "<bundled>/0.6/rnl-std-all.rnl": "" }), /does not have: <bundled>\/0\.6/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});
