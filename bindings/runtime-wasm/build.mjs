// Build the core for WebAssembly and put it with the JavaScript modules into dist/, which the web
// viewer copies:
//
//   node bindings/runtime-wasm/build.mjs [--features LIST] [--out DIR] [--packages DIR]
//                                        [--name NAME] [--version VERSION]
//
// --features passes cargo features on, such as test-panic for the tests; --out puts the result into
// DIR instead. The module is built with the workspace's `wasm` profile. --packages brings the .rnl
// files of DIR with the build: packages.js lists them, and the Worker gives them to every request.
// --name and --version go into the package.json, for a frontend that links the result as a package.

import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

const here = dirname(fileURLToPath(import.meta.url));
const usage =
  "usage: node bindings/runtime-wasm/build.mjs [--features LIST] [--out DIR] [--packages DIR] [--name NAME] [--version VERSION]";

let out = join(here, "dist");
const features = [];
let packages = null;
const manifest = {};
const args = process.argv.slice(2);
for (let k = 0; k < args.length; k++) {
  if (args[k] === "--out" && k + 1 < args.length) out = resolve(args[++k]);
  else if (args[k] === "--features" && k + 1 < args.length) features.push("--features", args[++k]);
  else if (args[k] === "--packages" && k + 1 < args.length) packages = resolve(args[++k]);
  else if ((args[k] === "--name" || args[k] === "--version") && k + 1 < args.length) manifest[args[k].slice(2)] = args[++k];
  else {
    console.error(usage);
    process.exit(2);
  }
}

const cargo = (cargoArgs, options) => execFileSync("cargo", cargoArgs, { cwd: here, ...options });
const metadata = JSON.parse(cargo(["metadata", "--format-version", "1", "--no-deps"], { encoding: "utf8" }));
const build = ["build", "--profile", "wasm", "--target", "wasm32-unknown-unknown", "-p", "rnl-runtime-wasm"];
cargo([...build, ...features], { stdio: "inherit" });

const wasm = join(metadata.target_directory, "wasm32-unknown-unknown", "wasm", "rnl_runtime_wasm.wasm");
mkdirSync(out, { recursive: true });
copyFileSync(wasm, join(out, "rnl_runtime_wasm.wasm"));
for (const name of readdirSync(join(here, "js"))) copyFileSync(join(here, "js", name), join(out, name));
// the declarations of the Document and of the snapshot, which core.d.ts and client.d.ts refer to
for (const name of ["document.d.ts", "snapshot.d.ts"]) copyFileSync(join(here, "..", "..", "crates", "rnl-view", "types", name), join(out, name));
const lines = (...l) => l.join("\n") + "\n";
if (Object.keys(manifest).length) {
  const pkg = JSON.parse(readFileSync(join(here, "js", "package.json"), "utf8"));
  writeFileSync(join(out, "package.json"), lines(JSON.stringify({ ...pkg, ...manifest }, null, 2)));
}
// the packages the build brings, under a directory of their own, with LF line endings
if (packages) {
  const dir = "/rnl-bundled/packages";
  const names = readdirSync(packages)
    .filter((n) => n.endsWith(".rnl"))
    .sort();
  const text = (n) => readFileSync(join(packages, n), "utf8").split("\r\n").join("\n");
  const files = Object.fromEntries(names.map((n) => [`${dir}/${n}`, text(n)]));
  writeFileSync(
    join(out, "packages.js"),
    lines(
      `// Brought by build.mjs --packages: ${names.join(", ")}. See packages.js in bindings/runtime-wasm/js.`,
      "",
      "/** @type {Record<string, string>} */",
      `export const files = ${JSON.stringify(files, null, 1)};`,
      "",
      "/** @type {string[]} */",
      `export const packageDirs = ${JSON.stringify(names.length ? [dir] : [])};`,
    ),
  );
  console.log(`packages.js: ${names.join(", ")}`);
}

const bytes = readFileSync(wasm);
const size = (n) => `${n} bytes (${(n / 1024).toFixed(0)} KiB)`;
console.log(`${out}`);
console.log(`rnl_runtime_wasm.wasm: ${size(bytes.length)}, ${size(gzipSync(bytes, { level: 9 }).length)} gzip-compressed`);
