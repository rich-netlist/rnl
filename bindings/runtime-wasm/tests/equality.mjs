// Native and WebAssembly builds give identical Documents (decision 0006). For every file of the
// conformance corpus and the standard packages, the Document that `rnl view` prints and the one the
// WebAssembly core builds must be deep-equal, numbers compared exactly. Both get the same entry, an
// absolute path; natively `rnl view` reads the disk, and in WebAssembly client.js's collect gives
// the core the files it asks for, read from the disk as `rnl` reads them.
//
//   node bindings/runtime-wasm/tests/equality.mjs [--rnl PATH] [--dist DIR] [--view ARG]... [--request JSON]
//
// --rnl: the rnl command, target/debug/rnl by default. --dist: the directory build.mjs wrote, dist/
// by default. --view passes an argument on to `rnl view` and --request adds fields to every request,
// so that both sides can be given the same option, such as
// `--view --no-std --request '{"std": false}'`. The `.rnl` files of the request's package_dirs are
// given with it, as a frontend does.

import { execFileSync } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../..");
const usage =
  "usage: node bindings/runtime-wasm/tests/equality.mjs [--rnl PATH] [--dist DIR] [--view ARG]... [--request JSON]";

let rnl = resolve(root, "target/debug", process.platform === "win32" ? "rnl.exe" : "rnl");
let dist = resolve(here, "../dist");
const view = [];
let extra = {};
const args = process.argv.slice(2);
for (let k = 0; k < args.length; k++) {
  const value = k + 1 < args.length ? args[k + 1] : null;
  if (value === null) fail(usage);
  if (args[k] === "--rnl") rnl = resolve(value);
  else if (args[k] === "--dist") dist = resolve(value);
  else if (args[k] === "--view") view.push(value);
  else if (args[k] === "--request") extra = { ...extra, ...JSON.parse(value) };
  else fail(usage);
  k++;
}

if (!existsSync(rnl)) fail(`${rnl} is not there: build it with \`cargo build -p rnl-cli\`, or give --rnl`);
if (!existsSync(join(dist, "rnl_runtime_wasm.wasm"))) fail(`${dist} has no core: run build.mjs, or give --dist`);
const { loadCore } = await import(pathToFileURL(join(dist, "core.js")));
const { collect } = await import(pathToFileURL(join(dist, "client.js")));
const core = await loadCore(pathToFileURL(join(dist, "rnl_runtime_wasm.wasm")));

const entries = [];
for (const dir of ["tests/conformance/cases", "tests/conformance/equivalence", "libraries/standard"]) {
  const names = readdirSync(join(root, dir)).filter((name) => /\.(sp|scs|inc|rnl)$/.test(name));
  entries.push(...names.sort().map((name) => join(root, dir, name)));
}
if (entries.length < 20) fail(`only ${entries.length} files found under ${root}`);

const packages = {};
for (const dir of extra.package_dirs ?? []) {
  for (const name of readdirSync(dir).filter((name) => name.endsWith(".rnl"))) {
    packages[`${dir}/${name}`] = await readText(join(dir, name));
  }
}

let differ = 0;
let builds = 0;
for (const entry of entries) {
  const want = native(entry);
  const [got, read] = await webAssembly(entry);
  const name = relative(root, entry).replaceAll("\\", "/");
  const d = difference(want, got, "");
  if (d) {
    differ++;
    console.log(`DIFF ${name}: ${d}`);
  } else {
    const count = (n, what) => `${n} ${what}${n === 1 ? "" : "s"}`;
    const what = want.error ? `the same error: ${want.error}` : `${count(read, "file")}, ${count(got.document.levels.length, "level")}`;
    console.log(`ok   ${name} (${what})`);
  }
}
const all = `${entries.length} files, ${builds} WebAssembly builds`;
if (differ) fail(`${all}: ${differ} Documents differ between the native and the WebAssembly build`);
console.log(`${all}: the native and the WebAssembly Documents are identical`);

/** The Document `rnl view` prints, or its error. */
function native(entry) {
  try {
    const out = execFileSync(rnl, ["view", entry, ...view], { encoding: "utf8", maxBuffer: 1 << 30, stdio: "pipe" });
    return { document: JSON.parse(out) };
  } catch (e) {
    if (typeof e.status !== "number") throw e;
    return { error: e.stderr.trim().replace(/^rnl: /, "") };
  }
}

/**
 * The Document the WebAssembly core builds, with the files collected from the disk, or its error;
 * and how many files were read.
 */
async function webAssembly(entry) {
  const counted = { build: (request) => (builds++, core.build(request)) };
  try {
    const { document, files } = await collect(counted, { ...extra, entry, files: packages }, readText);
    return [{ document }, Object.keys(files).length - Object.keys(packages).length];
  } catch (e) {
    return [{ error: e.message }, 0];
  }
}

/**
 * A file's text as `rnl` reads it (FsProvider): UTF-8 without a byte order mark, or Latin-1 when it
 * is not UTF-8; null when it cannot be read.
 */
async function readText(path) {
  let bytes;
  try {
    bytes = await readFile(path);
  } catch {
    return null;
  }
  try {
    const text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
    return text.charCodeAt(0) === 0xfeff ? text.slice(1) : text;
  } catch {
    return bytes.toString("latin1");
  }
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
