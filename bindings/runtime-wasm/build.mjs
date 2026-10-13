// Build the core for WebAssembly and put it with the JavaScript modules into dist/, which the web
// viewer copies:
//
//   node bindings/runtime-wasm/build.mjs [--features LIST] [--out DIR]
//
// --features passes cargo features on, such as test-panic for the tests; --out puts the result into
// DIR instead. The module is built with the workspace's `wasm` profile.

import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

const here = dirname(fileURLToPath(import.meta.url));
const usage = "usage: node bindings/runtime-wasm/build.mjs [--features LIST] [--out DIR]";

let out = join(here, "dist");
const features = [];
const args = process.argv.slice(2);
for (let k = 0; k < args.length; k++) {
  if (args[k] === "--out" && k + 1 < args.length) out = resolve(args[++k]);
  else if (args[k] === "--features" && k + 1 < args.length) features.push("--features", args[++k]);
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

const bytes = readFileSync(wasm);
const size = (n) => `${n} bytes (${(n / 1024).toFixed(0)} KiB)`;
console.log(`${out}`);
console.log(`rnl_runtime_wasm.wasm: ${size(bytes.length)}, ${size(gzipSync(bytes, { level: 9 }).length)} gzip-compressed`);
