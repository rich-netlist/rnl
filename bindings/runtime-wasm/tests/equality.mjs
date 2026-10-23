// Native and WebAssembly builds give identical Documents (decision 0008). For every file of the
// conformance corpus and the packages of libraries/, the Document that `rnl view` prints and the one the
// WebAssembly core builds must be deep-equal, numbers compared exactly. Both get the same entry, an
// absolute path; natively `rnl view` reads the disk, and in WebAssembly client.js's collect gives
// the core the files it asks for, read from the disk as `rnl` reads them. The Documents are built
// as the web viewer builds them, without the drawings of the objects that have no position
// (`lazy_unplaced`), and given them again by asking the core for the objects of each level, so that
// what is compared is what the page ends up with. With --snapshot, the editor interface's
// snapshots are compared instead, with the host circuit and the model: the one `rnl snapshot`
// prints and the one the core gives for the files collect read, revision included.
// Then, in either case, the catalog of every version of the packages the core has: the one
// `rnl catalog --all-versions` prints and the one the core lists for a catalog request.
//
//   node bindings/runtime-wasm/tests/equality.mjs [--snapshot] [--rnl PATH] [--dist DIR] [--view ARG]... [--request JSON]
//
// --rnl: the rnl command, target/debug/rnl by default. --dist: the directory build.mjs wrote, dist/
// by default. --view passes an argument on to `rnl view` and --request adds fields to every request,
// so that both sides can be given the same option, such as
// `--view --no-std --request '{"std": false}'`. The `.rnl` files of the request's package_dirs are
// given with it, as a frontend does.

import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../..");
const usage =
  "usage: node bindings/runtime-wasm/tests/equality.mjs [--snapshot] [--rnl PATH] [--dist DIR] [--view ARG]... [--request JSON]";

let rnl = resolve(root, "target/debug", process.platform === "win32" ? "rnl.exe" : "rnl");
let dist = resolve(here, "../dist");
const view = [];
let extra = {};
let snapshot = false;
const args = process.argv.slice(2);
for (let k = 0; k < args.length; k++) {
  if (args[k] === "--snapshot") {
    snapshot = true;
    continue;
  }
  const value = k + 1 < args.length ? args[k + 1] : null;
  if (value === null) fail(usage);
  if (args[k] === "--rnl") rnl = resolve(value);
  else if (args[k] === "--dist") dist = resolve(value);
  else if (args[k] === "--view") view.push(value);
  else if (args[k] === "--request") extra = { ...extra, ...JSON.parse(value) };
  else fail(usage);
  k++;
}

if (!existsSync(rnl) && existsSync(`${rnl}.exe`)) rnl = `${rnl}.exe`;
if (!existsSync(rnl)) fail(`${rnl} is not there: build it with \`cargo build -p rnl-cli\`, or give --rnl`);
if (!existsSync(join(dist, "rnl_runtime_wasm.wasm"))) fail(`${dist} has no core: run build.mjs, or give --dist`);
const { loadCore } = await import(pathToFileURL(join(dist, "core.js")));
const { collect, decodeText } = await import(pathToFileURL(join(dist, "client.js")));
const core = await loadCore(pathToFileURL(join(dist, "rnl_runtime_wasm.wasm")));

const { publicPackageDir, publicPackagesAvailable } = await import("../../../scripts/packages.mjs");
const hasPublicPackages = publicPackagesAvailable();
const entries = [];
for (const dir of ["tests/conformance/cases", "tests/conformance/equivalence", "tests/conformance/cases-0.3", "libraries/standard"]) {
  entries.push(...files(join(root, dir)).filter((path) => /\.(sp|scs|inc|rnl)$/.test(path)));
}
if (entries.length < 20) fail(`only ${entries.length} files found under ${root}`);
// Explicit/default zero widths, double hairlines and the viewer's dark-colour boundary cases.
entries.push(join(root, "tests/viewer/inputs/view-improvements.sp"));
entries.push(join(root, "tests/viewer/inputs/wire-labels.sp"));
entries.push(join(root, "tests/viewer/inputs/usage-inspector.sp"));
const defaultHairlines = join(root, "tests/viewer/inputs/default-hairlines.sp");
entries.push(defaultHairlines);
// Independent public geometry and style packages exercise custom fonts and generated terminal markers.
const edaDirs = hasPublicPackages ? ["eda-symbol", "eda-style"].map(name => publicPackageDir(name).replaceAll("\\", "/")) : [];
const edaEntries = new Set(edaDirs.flatMap((dir) => files(dir).filter((path) => /\.(rnl|scs|sp)$/.test(path))));
entries.push(...edaEntries);
if (hasPublicPackages) entries.push(join(publicPackageDir("analog-canvas-extensions"), "analog-canvas-extensions.rnl"));
const edaFiles = Object.fromEntries(await Promise.all(edaDirs.map(async (dir) => {
  const name = dir.split("/").at(-2);
  const path = `${dir}/${name}.rnl`;
  return [path, await readText(path)];
})));
// and a file that is not UTF-8, which rnl reads as Latin-1
const temp = mkdtempSync(join(tmpdir(), "rnl-equality-"));
process.on("exit", () => rmSync(temp, { recursive: true, force: true }));
const [micro, degree, c1] = [0xb5, 0xb0, 0x80].map((c) => String.fromCharCode(c));
entries.push(join(temp, "latin1.sp"));
writeFileSync(entries.at(-1), Buffer.from(`* 10${micro}s at 25${degree}C ${c1}\nR1 a 0 1k\n.end\n`, "latin1"));

const packages = {};
for (const dir of extra.package_dirs ?? []) {
  for (const name of readdirSync(dir).filter((name) => name.endsWith(".rnl"))) {
    packages[`${dir}/${name}`] = await readText(join(dir, name));
  }
}

const count = (n, what) => `${n} ${what}${n === 1 ? "" : "s"}`;
let differ = 0;
let builds = 0;
for (const entry of entries) {
  const want = native(entry);
  const [got, read] = await webAssembly(entry);
  const name = (entry.startsWith(temp) ? relative(temp, entry) : relative(root, entry)).replaceAll("\\", "/");
  const d = difference(want, got, "");
  if (d) {
    differ++;
    console.log(`DIFF ${name}: ${d}`);
  } else {
    const what = want.error ? `the same error: ${want.error}` : `${count(read, "file")}, ${count(got.value.levels.length, "level")}`;
    console.log(`ok   ${name} (${what})`);
  }
}
// and the catalog, with the same packages and options on both sides
const catalogs = (() => {
  const want = nativeCatalog();
  let got;
  try {
    got = { value: core.catalog({ ...extra, catalog: { all_versions: true }, files: packages }) };
  } catch (e) {
    got = { error: e.message };
  }
  const d = difference(want, got, "");
  if (d) {
    console.log(`DIFF rnl catalog --all-versions: ${d}`);
  } else {
    const c = got.value;
    const what = want.error ? `the same error: ${want.error}` : `${count(c.packages.length, "package")}, ${count(c.symbols.length, "symbol")}`;
    console.log(`ok   rnl catalog --all-versions (${what})`);
  }
  return d === null;
})();
const all = `${entries.length} files and the catalog, ${builds} WebAssembly builds`;
const outputs = snapshot ? "snapshots" : "Documents";
const differing = [differ && `${differ} ${outputs}`, !catalogs && "the catalogs"].filter(Boolean);
if (differing.length) fail(`${all}: ${differing.join(" and ")} differ between the native and the WebAssembly build`);
console.log(`${all}: the native and the WebAssembly ${outputs} and catalogs are identical`);

/**
 * The files in a directory and in its subdirectories, such as the version directories of the
 * standard packages, by name.
 */
function files(dir) {
  const out = [];
  for (const name of readdirSync(dir).sort()) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) out.push(...files(path));
    else out.push(path);
  }
  return out;
}

/** The Document `rnl view` prints, or with --snapshot the snapshot `rnl snapshot` prints; or its error. */
function native(entry) {
  const command = snapshot ? ["snapshot", entry, "--parts", "host,model"] : ["view", entry];
  if (entry === defaultHairlines) command.push("--no-std");
  if (edaEntries.has(entry)) for (const dir of edaDirs) command.push("--package-dir", dir);
  try {
    const out = execFileSync(rnl, [...command, ...view], { encoding: "utf8", maxBuffer: 1 << 30, stdio: "pipe" });
    return { value: JSON.parse(out) };
  } catch (e) {
    if (typeof e.status !== "number") throw e;
    return { error: e.stderr.trim().replace(/^rnl: /, "") };
  }
}

/** The catalog `rnl catalog --all-versions` prints, with the arguments of --view; or its error. */
function nativeCatalog() {
  try {
    const out = execFileSync(rnl, ["catalog", "--all-versions", ...view], { encoding: "utf8", maxBuffer: 1 << 30, stdio: "pipe" });
    return { value: JSON.parse(out) };
  } catch (e) {
    if (typeof e.status !== "number") throw e;
    return { error: e.stderr.trim().replace(/^rnl: /, "") };
  }
}

/**
 * The Document the WebAssembly core builds, with the files collected from the disk, or with
 * --snapshot its snapshot of those files; or its error; and how many files were read.
 */
async function webAssembly(entry) {
  const counted = { build: (request) => (builds++, core.build(request)) };
  try {
    const lazy = snapshot ? {} : { lazy_unplaced: true };
    const defaults = entry === defaultHairlines ? { std: false } : {};
    const external = edaEntries.has(entry)
      ? { package_dirs: [...(extra.package_dirs ?? []), ...edaDirs], files: { ...packages, ...edaFiles } }
      : { files: packages };
    const request = { ...extra, ...lazy, ...defaults, ...external, entry };
    const { document, files } = await collect(counted, request, readText);
    const read = Object.keys(files).length - Object.keys(packages).length;
    if (!snapshot) return [{ value: withDrawings(document) }, read];
    builds++;
    const parts = { drawing: false, host: true, model: true };
    return [{ value: core.snapshot({ ...request, files, parts }) }, read];
  } catch (e) {
    return [{ error: e.message }, 0];
  }
}

/**
 * The Document of the last build with the drawings of its unplaced objects, as the core gives them
 * on request, put where a Document that carries them has them.
 */
function withDrawings(document) {
  document.levels.forEach((level, k) => {
    if (level.unplaced.length === 0) return;
    const items = level.unplaced.map((_, i) => i);
    const names = level.unplaced.map((u) => u.name);
    const drawn = core.drawings({ drawings: { level: k, items, names } });
    level.unplaced.forEach((u, i) => Object.assign(u, drawn[i]));
  });
  return document;
}

/** A file's text as `rnl` reads it, decoded by client.js's decodeText; null when it cannot be read. */
async function readText(path) {
  let bytes;
  try {
    bytes = await readFile(path);
  } catch {
    return null;
  }
  return decodeText(bytes);
}

/**
 * Where two values read from JSON first differ, or null when they are the same. Numbers compare
 * with Object.is, so even 0 and -0 differ; fields compare in order.
 */
function difference(a, b, at) {
  const shown = (v) => {
    const s = Object.is(v, -0) ? "-0" : JSON.stringify(v);
    return s === undefined || s.length <= 80 ? String(s) : `${s.slice(0, 77)}...`;
  };
  const kind = (v) => (v === null ? "null" : Array.isArray(v) ? "array" : typeof v);
  if (kind(a) !== kind(b) || (kind(a) !== "array" && kind(a) !== "object")) {
    return kind(a) === kind(b) && Object.is(a, b) ? null : `${at || "."}: ${shown(a)} natively, ${shown(b)} in WebAssembly`;
  }
  if (Array.isArray(a)) {
    if (a.length !== b.length) return `${at || "."}: ${a.length} items natively, ${b.length} in WebAssembly`;
    for (let k = 0; k < a.length; k++) {
      const d = difference(a[k], b[k], `${at}[${k}]`);
      if (d) return d;
    }
    return null;
  }
  const keys = [Object.keys(a).join(", "), Object.keys(b).join(", ")];
  if (keys[0] !== keys[1]) return `${at || "."}: fields ${keys[0]} natively, ${keys[1]} in WebAssembly`;
  for (const key of Object.keys(a)) {
    const d = difference(a[key], b[key], `${at}.${key}`);
    if (d) return d;
  }
  return null;
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
