// Actually execute the embedded configuration parser and a tiny 0.3 solve in WebAssembly, then
// compare every logical solution field and the exact structured score with the same native
// harness. This script invokes no cargo; build both harnesses first, in the validation queue.
//
// cargo build -p rnl-runtime-wasm --features test-solver --example solver-smoke
// node bindings/runtime-wasm/build.mjs --features test-solver --out target/wasm-solver-test
// node bindings/runtime-wasm/tests/solver.mjs [--native PATH] [--dist DIR]

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
let dist = resolve(root, "target/wasm-solver-test");
let native = resolve(root, "target/debug/examples", process.platform === "win32" ? "solver-smoke.exe" : "solver-smoke");
const args = process.argv.slice(2);
for (let index = 0; index < args.length; index += 2) {
  const value = args[index + 1];
  assert.ok(value, `missing value for ${args[index]}`);
  if (args[index] === "--native") native = resolve(value);
  else if (args[index] === "--dist") dist = resolve(value);
  else throw new Error(`unknown argument ${args[index]}`);
}
assert.ok(existsSync(native), `${native} is missing: build the feature-gated native solver-smoke example first`);
assert.ok(existsSync(join(dist, "rnl_runtime_wasm.wasm")), `${dist} is missing its test-solver WebAssembly build`);
const { loadCore } = await import(pathToFileURL(join(dist, "core.js")));
const core = await loadCore(pathToFileURL(join(dist, "rnl_runtime_wasm.wasm")));
const expected = JSON.parse(execFileSync(native, [], { encoding: "utf8", maxBuffer: 16 * 1024 * 1024 }));
const response = core.respond({ solver_test: true });
assert.equal(response.error, undefined, response.error);
assert.ok(response.solver_test, "the WebAssembly module must have the test-solver feature");
const actual = response.solver_test;
assert.equal(actual.schema, "rnl-solver-wasm-smoke-v1");
assert.ok(actual.config.length > 0);
assert.equal(actual.levels.length, 1);
for (const level of actual.levels) {
  assert.equal(level.version, "0.3");
  assert.ok(level.solution.placements.length >= 2);
  assert.ok(level.solution.wires.length > 0);
  assert.ok(level.report.scoring.levels.length > 0);
  assert.equal(level.report.scoring.config, actual.config);
  for (const score of level.report.scoring.levels) {
    for (const value of [score.total, score.tolerance]) assert.match(value, /^-?\d+(?:\/\d+)?$/);
    for (const term of score.terms) {
      for (const value of [term.raw, term.weight, term.weighted]) assert.match(value, /^-?\d+(?:\/\d+)?$/);
    }
  }
}
assert.deepEqual(actual, expected, "native and WebAssembly solve geometry, reports and exact scoring must agree");
assert.deepEqual(core.respond({ solver_test: true }), response, "the executed WebAssembly solve must be deterministic");
assert.ok(core.respond({ solver_test: false }).error, "the test request rejects an invalid payload");
console.log("ok: WebAssembly parses the embedded config and executes the same 0.3 solve, routing and exact scoring as native Rust");
