// Build the solver_parity example for native and wasm32-unknown-unknown, then pass both paths.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';

const [native, wasm] = process.argv.slice(2);
assert.ok(native && wasm, 'usage: node solver-equality.mjs NATIVE WASM');
const run = spawnSync(native, [], { encoding: 'utf8', timeout: 30000 });
assert.equal(run.status, 0, run.stderr || String(run.error));
const { instance } = await WebAssembly.instantiate(readFileSync(wasm), {});
const api = instance.exports;
let count = 0;
for (const line of run.stdout.trim().split(/\r?\n/)) {
  const [example, work, expected] = JSON.parse(line);
  const length = api.parity_run(example, work);
  const actual = new TextDecoder().decode(new Uint8Array(api.memory.buffer, api.parity_ptr(), length));
  assert.equal(actual, expected, `example ${example}, work ${work}`);
  count++;
}
assert.equal(count, 42);
console.log(`${count} native/WASM solver records are byte-identical`);
