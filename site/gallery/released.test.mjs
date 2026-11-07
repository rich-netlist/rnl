// Tests of which standard series build.mjs shows: node --test site/gallery/released.test.mjs
// (needs the core in bindings/runtime-wasm/dist/, as build.mjs does)

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");

/** A tree with the package rnl-std-style in the standard series 0.3 and a draft of 0.4, and the groups build.mjs shows of it. */
function shown(released, flags) {
  const dir = mkdtempSync(join(tmpdir(), "rnl-gallery-released-"));
  try {
    const style = readFileSync(join(root, "libraries", "standard", "0.3", "rnl-std-style.rnl"), "utf8");
    for (const v of ["0.3", "0.4"]) {
      mkdirSync(join(dir, "libraries", "standard", v), { recursive: true });
      writeFileSync(join(dir, "libraries", "standard", v, "rnl-std-style.rnl"), style.replaceAll("0.3", v));
    }
    mkdirSync(join(dir, "libraries", "extension"), { recursive: true });
    writeFileSync(join(dir, "libraries", "extension", "rnl-x.rnl"), style.replaceAll("0.3", "0.4").replaceAll("rnl-std-style", "rnl-x"));
    for (const v of released) {
      mkdirSync(join(dir, "spec", v), { recursive: true });
      writeFileSync(join(dir, "spec", v, "RELEASED"), "released\n");
    }
    const out = join(dir, "out");
    mkdirSync(out);
    execFileSync(process.execPath, [join(root, "site", "gallery", "build.mjs"), "--out", out, "--libraries", join(dir, "libraries"), ...flags(dir)], { stdio: "pipe" });
    const text = readFileSync(join(out, "gallery", "index.js"), "utf8");
    return JSON.parse(text.slice(text.indexOf("(") + 1, text.lastIndexOf(")"))).groups.map((g) => g.id);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

test("a standard series is shown once the standard of its number is released", () => {
  assert.deepEqual(shown(["0.3"], () => []), ["standard-0.3", "extension"]);
  assert.deepEqual(shown(["0.3", "0.4"], () => []), ["standard-0.4", "standard-0.3", "extension"]);
});

test("a series without its standard released is shown only with --drafts, and a package outside the series always", () => {
  assert.deepEqual(shown([], () => []), ["extension"]);
  assert.deepEqual(shown(["0.3"], () => ["--drafts"]), ["standard-0.4", "standard-0.3", "extension"]);
});

test("--spec names the spec/ directory, instead of the one beside the libraries", () => {
  // the repository's own spec/ has 0.3 released and no 0.4
  assert.deepEqual(shown([], () => ["--spec", join(root, "spec")]), ["standard-0.3", "extension"]);
});
