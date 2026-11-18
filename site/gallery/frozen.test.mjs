// Tests of the frozen groups of the gallery of the packages: node --test site/gallery/frozen.test.mjs
// (needs the core in bindings/runtime-wasm/dist/, as build.mjs does)

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const parse = (text) => JSON.parse(text.slice(text.indexOf("(") + 1, text.lastIndexOf(")")));

/** A frozen group with the id and series of a standard series, and nothing in it. */
const empty = (series, packages = []) => ({
  id: `standard-${series}`, dir: `libraries/standard/${series}`, kind: "standard", series, packages,
  counts: { packages: packages.length, symbols: 0 },
});

/** Runs build.mjs over a tree with the package rnl-std-style in standard/0.3 (when `with03`) and a package outside the
 *  standard, with the given frozen groups; gives the files it wrote. Both packages are the package file of the series
 *  0.4, in the language 0.4 that the core reads, the one in standard/0.3 with the version 0.3. */
function built(frozen, { with03 }) {
  const dir = mkdtempSync(join(tmpdir(), "rnl-gallery-frozen-"));
  try {
    const style = readFileSync(join(root, "libraries", "standard", "0.4", "rnl-std-style.rnl"), "utf8");
    mkdirSync(join(dir, "libraries", "extension"), { recursive: true });
    writeFileSync(join(dir, "libraries", "extension", "rnl-x.rnl"), style.replaceAll("rnl-std-style", "rnl-x"));
    if (with03) {
      mkdirSync(join(dir, "libraries", "standard", "0.3"), { recursive: true });
      writeFileSync(join(dir, "libraries", "standard", "0.3", "rnl-std-style.rnl"), style.replace('version="0.4"', 'version="0.3"'));
    }
    mkdirSync(join(dir, "spec", "0.3"), { recursive: true });
    writeFileSync(join(dir, "spec", "0.3", "RELEASED"), "released\n");
    mkdirSync(join(dir, "frozen"));
    for (const g of frozen) writeFileSync(join(dir, "frozen", `${g.id}.js`), `RNLGallery.group(${JSON.stringify(g)});\n`);
    const out = join(dir, "out");
    mkdirSync(out);
    execFileSync(process.execPath, [join(here, "build.mjs"), "--out", out, "--libraries", join(dir, "libraries"), "--frozen", join(dir, "frozen")], { stdio: "pipe" });
    const files = Object.fromEntries(readdirSync(join(out, "gallery")).map((name) => [name, readFileSync(join(out, "gallery", name), "utf8")]));
    return { index: parse(files["index.js"]).groups, file: (name) => files[name] };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

test("a frozen group is shown, as it is, when libraries/ no longer holds its package files", () => {
  const group = empty("0.3", [{ name: "rnl-std", version: "0.3" }]);
  const { index, file } = built([group], { with03: false });
  assert.deepEqual(index.map((g) => g.id), ["standard-0.3", "extension"]);
  assert.equal(file("standard-0.3.js"), `RNLGallery.group(${JSON.stringify(group)});\n`);
  assert.deepEqual(index[0].packages, [{ name: "rnl-std", version: "0.3" }]);
});

test("a frozen group replaces the group read from the package files of its directory", () => {
  const group = empty("0.3", [{ name: "from-the-snapshot", version: "0.3" }]);
  const { index, file } = built([group], { with03: true });
  assert.deepEqual(index.map((g) => g.id), ["standard-0.3", "extension"]);
  assert.deepEqual(index[0].packages, [{ name: "from-the-snapshot", version: "0.3" }]);
  assert.equal(parse(file("standard-0.3.js")).packages[0].name, "from-the-snapshot");
});

test("a frozen group sorts among the series by its number", () => {
  const { index } = built([empty("0.2"), empty("0.3")], { with03: false });
  assert.deepEqual(index.map((g) => g.id), ["standard-0.3", "standard-0.2", "extension"]);
});

test("the frozen groups of the repository are whole: each is named by its id, with its packages and drawn symbols", () => {
  const dir = join(here, "frozen");
  const files = readdirSync(dir).filter((name) => name.endsWith(".js")).sort();
  assert.deepEqual(files, ["standard-0.2.js", "standard-0.3.js"]);
  for (const name of files) {
    const text = readFileSync(join(dir, name), "utf8");
    assert.match(text, /^RNLGallery\.group\(.*\);\n$/s, name);
    const data = parse(text);
    assert.equal(`${data.id}.js`, name);
    assert.equal(data.kind, "standard");
    assert.equal(data.dir, `libraries/standard/${data.series}`);
    assert.equal(data.id, `standard-${data.series}`);
    assert.ok(data.counts.packages > 0 && data.counts.symbols > 0, name);
    assert.equal(data.packages.length, data.counts.packages, name);
    const symbols = data.packages.flatMap((p) => p.statements).filter((s) => s.kind === "symbol");
    assert.equal(symbols.filter((s) => s.svg).length, data.counts.symbols, name);
    for (const p of data.packages) assert.ok(p.path.startsWith(`${data.dir}/`), `${name}: ${p.path}`);
  }
});
