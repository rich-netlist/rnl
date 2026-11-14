// Build the web viewer: a static site in apps/web/dist/, which git ignores, with the page of
// apps/viewer-ui, its web fonts in fonts/ when python scripts/fonts.py web has written them, the
// fonts' licence in fonts/OFL.txt in any case, and the core as WebAssembly with its JavaScript
// modules in core/:
//
//   node apps/web/build.mjs [--no-core] [--no-solver] [--samples]
//
// --no-core takes the binding as bindings/runtime-wasm/build.mjs last built it instead of building it
// again (the page solves only if that was built with --solver); --no-solver leaves the solver's module out, as the
// local program, whose page solves with the program, does not need it; --samples copies the conformance corpus's cases and equivalence files and the site's
// examples into samples/, to try the viewer with ?url=samples/cases/04-hierarchy.sp and for tests.

import { publishLibrary } from "./library.mjs";
import { buildVersions } from "./build-info.mjs";
import { addPreloadHints } from "./preload-hints.mjs";
import { execFileSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";
import { API_FEATURES, MAX_OPEN_BYTES } from "../viewer-ui/viewer-api-contract.js";
import { writeLocales } from "../../scripts/i18n/viewer-locales.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const usage = "usage: node apps/web/build.mjs [--no-core] [--no-solver] [--samples]";

let buildCore = true;
let samples = false;
let withSolver = true;
for (const arg of process.argv.slice(2)) {
  if (arg === "--no-core") buildCore = false;
  else if (arg === "--samples") samples = true;
  else if (arg === "--no-solver") withSolver = false;
  else {
    console.error(usage);
    process.exit(2);
  }
}

const page = join(root, "apps", "viewer-ui");
const binding = join(root, "bindings", "runtime-wasm");
const core = join(binding, "dist");
const out = join(here, "dist");

// The solver's module is built with the core, for the page to load when the user solves.
if (buildCore) execFileSync(process.execPath, [join(binding, "build.mjs"), ...(withSolver ? ["--solver"] : [])], { stdio: "inherit" });
if (!existsSync(join(core, "rnl_runtime_wasm.wasm"))) {
  console.error(`${core} has no core: build it with node bindings/runtime-wasm/build.mjs, or leave out --no-core`);
  process.exit(1);
}

rmSync(out, { recursive: true, force: true });

// The page, but what only Node and the desktop viewer read: the README, the package.json that makes
// Node read the scripts as modules, and the desktop shell. Its fonts/ comes with it.
const leftOut = new Set(["README.md", "package.json", join("shells", "desktop.js")]);
cpSync(page, out, { recursive: true, filter: (from) => !leftOut.has(relative(page, from)) });
// The languages of the interface and their catalogs, from i18n/ (decision 0015), which the page reads
// from locales/ (i18n.js).
writeLocales(root, join(out, "locales"));
const fonts = existsSync(join(out, "fonts", "fonts.css"));
if (!fonts) {
  console.warn("no web fonts in apps/viewer-ui/fonts/, so texts fall back on sans-serif: write them with python scripts/fonts.py web");
}
// The core lays out texts with the fonts' metrics, which are under the fonts' licence (decision
// 0009), so the site carries it, with their copyright as scripts/fonts.py writes it, even without
// the fonts.
const licence = join(out, "fonts", "OFL.txt");
if (!existsSync(licence)) {
  const copyright = "Copyright 2014-2021 Adobe (http://www.adobe.com/). Noto is a trademark of Google Inc.";
  const ofl = readFileSync(join(root, "LICENSES", "OFL-1.1.txt"), "utf8").replace(/\r\n/g, "\n");
  mkdirSync(dirname(licence), { recursive: true });
  writeFileSync(licence, `${copyright}\n\n${ofl}`);
}

// The core, which the web shell imports from core/client.js, without the package.json for Node.
cpSync(core, join(out, "core"), { recursive: true, filter: (from) => relative(core, from) !== "package.json" && (withSolver || !/^(solver\.(js|d\.ts)|rnl_solver_wasm\.wasm)$/.test(relative(core, from))) });
for (const name of ["embed-demo.html", "embed-demo.js"]) cpSync(join(here, name), join(out, name));
// The solver, when the core's directory has it (core/solver.js and core/rnl_solver_wasm.wasm): its Worker, and
// solver.json, which tells the page it can solve and how large the module it then loads is.
const solverModule = join(out, "core", "rnl_solver_wasm.wasm");
if (existsSync(solverModule) && existsSync(join(out, "core", "solver.js"))) {
  cpSync(join(here, "solver-worker.js"), join(out, "solver-worker.js"));
  writeFileSync(join(out, "solver.json"), JSON.stringify({ worker: "solver-worker.js", bytes: statSync(solverModule).size }) + "\n");
}

// Viewer versions are independent of the core's workspace version.
const { version, coreVersion } = buildVersions(root);
let commit = null;
try {
  commit = execFileSync("git", ["rev-parse", "--short", "HEAD"], { cwd: root, encoding: "utf8", stdio: "pipe" }).trim();
} catch {
  // not a git checkout, or no git: the box names the version only
}
writeFileSync(join(out, "about.json"), `${JSON.stringify({ version, coreVersion, commit }, null, 2)}\n`);
writeFileSync(join(out, "viewer-api.json"), JSON.stringify({ rnl: 1, features: API_FEATURES, maxOpenBytes: MAX_OPEN_BYTES }) + "\n");

if (samples) {
  for (const name of ["cases", "equivalence"]) {
    cpSync(join(root, "tests", "conformance", name), join(out, "samples", name), { recursive: true });
  }
  cpSync(join(root, "site", "examples"), join(out, "samples", "site"), { recursive: true });
  mkdirSync(join(out, "samples", "brand"), { recursive: true });
  // Caption the interactive sample; the canonical source still generates the text-free icons.
  const logo = readFileSync(join(root, "brand", "rnl-logo.sp"), "utf8");
  const caption = '* @RNL {\n*   (4.5, 0.3) @TEXT("RNL", size=11, bold=true, valign="middle");\n*   (28, -4.5) @TEXT("logo by RNL", size=3.3, bold=false)\n* }\n.end';
  writeFileSync(join(out, "samples", "brand", "rnl-logo.sp"), logo.replace(/^\.end[ \t]*\r?$/im, caption));
}

/** The files under a directory, by their paths relative to it, `/`-separated. */
function files(dir, base = dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? files(join(dir, e.name), base) : [relative(base, join(dir, e.name)).split(sep).join("/")],
  );
}

publishLibrary(join(root, "libraries"), join(out, "libraries"));
// Tests use a verified offline fixture, never the public service.
const fixture = process.env.RNL_PACKAGE_FIXTURE;
if (fixture) cpSync(fixture, join(out, "public-packages"), { recursive: true });
writeFileSync(join(out, "public-packages.json"), JSON.stringify({
  url: fixture ? "public-packages/catalog.json" : process.env.RNL_PACKAGE_CATALOG || "https://rich-netlist.github.io/rnl-packages/catalog.json",
}) + "\n");

// What the page puts before the names of the values it keeps in the browser's storage: none, unless RNL_STORAGE_PREFIX says,
// as a frozen older build beside the current one needs (apps/viewer-ui/storage-prefix.js).
const prefix = process.env.RNL_STORAGE_PREFIX ?? "";
if (prefix !== "") {
  if (!/^[A-Za-z0-9._-]{1,32}$/.test(prefix)) {
    console.error("RNL_STORAGE_PREFIX is 1 to 32 letters, digits, dots, hyphens or underscores");
    process.exit(2);
  }
  writeFileSync(join(out, "storage-prefix.js"), `// written by apps/web/build.mjs from RNL_STORAGE_PREFIX
export const STORAGE_PREFIX = ${JSON.stringify(prefix)};
`);
}

// the page asks for its scripts, the core and the files it reads at start all at once (preload-hints.mjs)
const hints = addPreloadHints(out);
const all = files(out);
const bytes = all.reduce((n, f) => n + statSync(join(out, f)).size, 0);
const wasm = readFileSync(join(out, "core", "rnl_runtime_wasm.wasm"));
const kib = (n) => `${(n / 1024).toFixed(0)} KiB`;
console.log(out);
const packed = gzipSync(wasm, { level: 9 }).length;
console.log(`${all.length} files, ${kib(bytes)}; the core ${kib(wasm.length)}, ${kib(packed)} gzip-compressed; ${hints} preload hints`);
if (existsSync(join(out, "solver.json"))) {
  const solver = readFileSync(solverModule);
  console.log(`the solver, loaded when the user solves: ${kib(solver.length)}, ${kib(gzipSync(solver, { level: 9 }).length)} gzip-compressed`);
}
if (fonts) {
  const font = all.filter((f) => f.startsWith("fonts/"));
  console.log(`the web fonts: ${font.length} files, ${kib(font.reduce((n, f) => n + statSync(join(out, f)).size, 0))}`);
}
