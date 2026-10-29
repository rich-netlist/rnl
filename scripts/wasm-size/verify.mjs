// Compare actual UTF-8 ABI response bytes, not parsed/deep-equal JSON alone.
// Only the caller's CI job runs this harness. No compiler or browser is started.
import assert from "node:assert/strict";
import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { resolve, join } from "node:path";
import { pathToFileURL } from "node:url";
import { createHash } from "node:crypto";
import { performance } from "node:perf_hooks";

const [mode, sourceArg, baselineArg, candidateArg, outputArg] = process.argv.slice(2);
const root = resolve(sourceArg);
const baseline = resolve(baselineArg);
const candidate = resolve(candidateArg);
const output = resolve(outputArg);
const encoder = new TextEncoder();
const hash = (data) => createHash("sha256").update(data).digest("hex");
const json = (p) => JSON.parse(readFileSync(p, "utf8"));
const save = (data) => writeFileSync(output, JSON.stringify(data, null, 2) + "\n");
async function instance(path) {
  const bytes = readFileSync(path);
  const { instance: { exports: e } } = await WebAssembly.instantiate(bytes);
  return (request) => {
    const b = typeof request === "string" ? encoder.encode(request) : encoder.encode(JSON.stringify(request));
    const at = e.rnl_request(b.length) >>> 0;
    new Uint8Array(e.memory.buffer, at, b.length).set(b);
    const n = e.rnl_build() >>> 0;
    return Buffer.from(new Uint8Array(e.memory.buffer, e.rnl_response() >>> 0, n));
  };
}
function files(dir) {
  return readdirSync(dir).sort().flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}
if (mode === "corpus") {
  const raw = await instance(baseline);
  const { collect, decodeText } = await import(pathToFileURL(join(root, "bindings/runtime-wasm/js/client.js")));
  const requests = [];
  function call(request, label) {
    const answer = raw(request);
    requests.push({ label, request, sha256: hash(answer), bytes: answer.length });
    const value = JSON.parse(answer);
    if (value.error) throw new Error(value.error);
    return Object.values(value)[0];
  }
  const dirs = ["tests/conformance/cases", "tests/conformance/cases-0.3", "tests/conformance/equivalence", "libraries/standard", "tests/performance/inputs/representative"];
  const entries = dirs.flatMap((p) => files(join(root, p))).filter(p => /\.(sp|scs|inc|rnl)$/.test(p));
  for (const entry of entries) {
    for (const options of [{}, { primitives: true }, { parts: { drawing: false, host: true, model: true } }, { lazy_unplaced: true }]) {
      const label = entry.slice(root.length + 1) + ":" + JSON.stringify(options);
      try {
        const { document, files: collected } = await collect({ build: r => call(r, label) }, { entry, ...(options.parts ? {} : options) }, async p => {
          try { return decodeText(readFileSync(p)); } catch { return null; }
        });
        if (options.parts) call({ entry, files: collected, ...options }, label + ":snapshot");
        if (options.lazy_unplaced) for (let level = 0; level < document.levels.length; level++) {
          const items = document.levels[level].unplaced.map((_, i) => i);
          if (items.length) call({ drawings: { level, items } }, label + ":drawings");
        }
      } catch (error) {
        // Error responses themselves were recorded above. File-loading failures are not hidden.
        if (!requests.length || requests.at(-1).label !== label) throw error;
      }
    }
  }
  const special = [
    { catalog: { all_versions: true } }, { formats: { detect: { "x.sp": "R1 a 0 1k\n" } } },
    { library: { reload: true }, files: {}, pool: [] },
    { write: { dialect: "spice", netlist: "R1 a 0 1k\n.end\n", author: "size-test", ops: [] } },
    { write: { dialect: "spice", netlist: "R1 a 0 1k\n.end\n", author: "size-test", ops: [{ level: "root", op: "set", object: "#R1", at: [0, 0] }] } },
    {}, { entry: "missing.sp" }, { entry: 1 }, { entry: "a.sp", svg: "no" },
    { catalog: { all_versions: "yes" } }, { write: {} }, { drawings: {} }, { formats: { detect: [] } },
    "{", "null", "[]", '{"entry":"a.sp","entry":"b.sp"}',
  ];
  for (const request of special) {
    try { call(request, "boundary:" + JSON.stringify(request)); } catch { /* expected error response */ }
  }
  save({ baseline_sha256: hash(readFileSync(baseline)), requests });
} else if (mode === "check") {
  const corpus = json(join(baseline, "corpus.json"));
  const raw = await instance(candidate);
  const differences = [];
  for (const [index, record] of corpus.requests.entries()) {
    try {
      const answer = raw(record.request);
      if (hash(answer) !== record.sha256) differences.push({ index, label: record.label, expected: record.sha256, actual: hash(answer), prefix: answer.toString("utf8").slice(0, 240) });
    } catch (error) { differences.push({ index, label: record.label, trap: String(error) }); }
  }
  save({ checked: corpus.requests.length, differing: differences.length, differences: differences.slice(0, 30), byte_identical: differences.length === 0 });
} else if (mode === "benchmark") {
  const corpus = json(join(baseline, "corpus.json"));
  const prepared = json(join(baseline, "performance-inputs.json"));
  const requests = [];
  const seen = new Set();
  for (const row of corpus.requests) {
    if (!row.label.includes("tests/performance/inputs/representative/") || !row.label.endsWith(":{}")) continue;
    if (seen.has(row.label)) continue;
    seen.add(row.label);
    requests.push({ name: row.label, request: row.request });
  }
  requests.push(...prepared);
  const raw = [await instance(join(baseline, "baseline/rnl_runtime_wasm.wasm")), await instance(candidate)];
  const results = [];
  for (const { name, request } of requests) {
    // The first call includes lazily initialized word/package data; it is kept separately.
    const cold = raw.map(call => { const t = performance.now(); call(request); return performance.now() - t; });
    for (let n = 0; n < 3; n++) for (const call of raw) call(request);
    const samples = [[], []];
    for (let n = 0; n < 9; n++) for (const k of n % 2 ? [1, 0] : [0, 1]) {
      const t = performance.now();
      const response = raw[k](request);
      samples[k].push(performance.now() - t);
      assert.equal(hash(response), hash(raw[1 - k](request)), "performance input output differs: " + name);
    }
    const median = a => [...a].sort((a, b) => a - b)[Math.floor(a.length / 2)];
    results.push({ name, first_call_ms: cold, baseline_ms: median(samples[0]), candidate_ms: median(samples[1]), samples_ms: samples });
  }
  save({ node: process.version, note: "Same process, baseline/candidate order alternates; 3 warmups, 9 samples. ABI call includes JSON marshalling and response copy. First call is not a fresh instance for each input. No solve call exists in this module.", results });
} else throw new Error("Unknown mode: " + mode);
