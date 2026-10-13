// Build the web viewer: a static site in apps/web/dist/, which git ignores, with the page of
// apps/viewer-ui and the core as WebAssembly with its JavaScript modules in core/:
//
//   node apps/web/build.mjs [--no-core] [--samples]
//
// --no-core takes the binding as bindings/runtime-wasm/build.mjs last built it instead of building it
// again; --samples copies the conformance corpus's cases and equivalence files into samples/, to try
// the viewer with ?url=samples/cases/04-hierarchy.sp and for tests.

import { execFileSync } from "node:child_process";
import { cpSync, existsSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const usage = "usage: node apps/web/build.mjs [--no-core] [--samples]";

let buildCore = true;
let samples = false;
for (const arg of process.argv.slice(2)) {
  if (arg === "--no-core") buildCore = false;
  else if (arg === "--samples") samples = true;
  else {
    console.error(usage);
    process.exit(2);
  }
}

const page = join(root, "apps", "viewer-ui");
const binding = join(root, "bindings", "runtime-wasm");
const core = join(binding, "dist");
const out = join(here, "dist");

if (buildCore) execFileSync(process.execPath, [join(binding, "build.mjs")], { stdio: "inherit" });
if (!existsSync(join(core, "rnl_runtime_wasm.wasm"))) {
  console.error(`${core} has no core: build it with node bindings/runtime-wasm/build.mjs, or leave out --no-core`);
  process.exit(1);
}

rmSync(out, { recursive: true, force: true });

// The page, but what only Node and the desktop viewer read: the README, the package.json that makes
// Node read the scripts as modules, and the desktop shell.
const leftOut = new Set(["README.md", "package.json", join("shells", "desktop.js")]);
cpSync(page, out, { recursive: true, filter: (from) => !leftOut.has(relative(page, from)) });

// The core, which the web shell imports from core/client.js, without the package.json for Node.
cpSync(core, join(out, "core"), { recursive: true, filter: (from) => relative(core, from) !== "package.json" });

// What the About box names: the version of the workspace, and the commit when git knows it.
const manifest = readFileSync(join(root, "Cargo.toml"), "utf8");
const version = /^\[workspace\.package\][^[]*?^version\s*=\s*"([^"]*)"/m.exec(manifest);
let commit = null;
try {
  commit = execFileSync("git", ["rev-parse", "--short", "HEAD"], { cwd: root, encoding: "utf8", stdio: "pipe" }).trim();
} catch {
  // not a git checkout, or no git: the box names the version only
}
writeFileSync(join(out, "about.json"), `${JSON.stringify({ version: version ? version[1] : null, commit }, null, 2)}\n`);

if (samples) {
  for (const name of ["cases", "equivalence"]) {
    cpSync(join(root, "tests", "conformance", name), join(out, "samples", name), { recursive: true });
  }
}

/** The files under a directory, by their paths relative to it, `/`-separated. */
function files(dir, base = dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? files(join(dir, e.name), base) : [relative(base, join(dir, e.name)).split(sep).join("/")],
  );
}

const all = files(out);
const bytes = all.reduce((n, f) => n + statSync(join(out, f)).size, 0);
const wasm = readFileSync(join(out, "core", "rnl_runtime_wasm.wasm"));
const kib = (n) => `${(n / 1024).toFixed(0)} KiB`;
console.log(out);
const packed = gzipSync(wasm, { level: 9 }).length;
console.log(`${all.length} files, ${kib(bytes)}; the core ${kib(wasm.length)}, ${kib(packed)} gzip-compressed`);
