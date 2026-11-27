// The solver of the web viewer, in a module Worker of its own (decision 0010). The page starts it when
// the user first solves, so that no one who only looks at a netlist loads the solver's module, which
// is larger than the core's (core/rnl_solver_wasm.wasm, bindings/solver-wasm). A solve is one
// synchronous call that can take the solver's whole time limit; here it blocks nothing of the page.
//
// It answers each message { id, request } (a SolveRequest of bindings/solver-wasm/js/solver.d.ts)
// with { id, event } for each event of the solve as it happens (docs/solver-progress.md), and
// then { id, answer }, the solver's answer: { document, report, files }, { cancelled: true } or
// { error }; a message { id, table: true } with { id, table }, the table of the solver's settings
// (docs/solver-settings-table.md); a message { id, check: request } with { id, table: answer }, what the module's
// check(request) gives (`rnl solve --check --json`; with no entry, only its solver_config); or { id, error } when the module could not be loaded or the call threw. The page
// cancels a solve by terminating the Worker, which the solver's module is written for: every
// request carries all it reads, so a new Worker needs nothing of the old one, and a download under way
// stops with it.
//
// While the module is loaded, the first time a message needs it, the Worker tells the page how it goes, in
// messages of no call: { load: { phase: "download", done, total } } as its bytes come (`done` of `total`, the
// size a message gives as `size`, which solver.json says; null when none was given), then
// { load: { phase: "compile" } }, then { load: { phase: "ready" } }. A module loaded already is not told of.
// The bytes counted are those the body gives, uncompressed as the browser gives them, as solver.json's size is.

import { loadSolver } from "./core/solver.js";

let loading = null;

/** How often, at most, the bytes come are told, in milliseconds. */
const TELL_EVERY = 100;

const tell = (load) => postMessage({ load });
const reason = (e) => (e instanceof Error ? e.message : String(e));

/** The bytes of the solver's module, told as they come; `total` is the size expected, or null. */
async function download(total) {
  tell({ phase: "download", done: 0, total });
  let response;
  try {
    response = await fetch(new URL("./core/rnl_solver_wasm.wasm", import.meta.url));
  } catch (e) {
    throw new Error(`the solver could not be downloaded: ${reason(e)}`);
  }
  if (!response.ok) throw new Error(`the solver could not be downloaded (HTTP ${response.status})`);
  try {
    if (!response.body) return new Uint8Array(await response.arrayBuffer());
    const reader = response.body.getReader();
    const chunks = [];
    let done = 0;
    let told = performance.now();
    for (;;) {
      const { done: end, value } = await reader.read();
      if (end) break;
      chunks.push(value);
      done += value.length;
      if (performance.now() - told >= TELL_EVERY) {
        told = performance.now();
        tell({ phase: "download", done, total });
      }
    }
    tell({ phase: "download", done, total });
    const bytes = new Uint8Array(done);
    let at = 0;
    for (const chunk of chunks) {
      bytes.set(chunk, at);
      at += chunk.length;
    }
    return bytes;
  } catch (e) {
    throw new Error(`the solver could not be downloaded: ${reason(e)}`);
  }
}

/** The solver, loaded once: the module is downloaded and compiled when a message first needs it. */
function solver(size) {
  loading ??= (async () => {
    const bytes = await download(Number.isFinite(size) && size > 0 ? size : null);
    tell({ phase: "compile" });
    let loaded;
    try {
      loaded = await loadSolver(await WebAssembly.compile(bytes));
    } catch (e) {
      throw new Error(`the solver could not be prepared: ${reason(e)}`);
    }
    tell({ phase: "ready" });
    return loaded;
  })();
  loading.catch(() => {
    loading = null; // a failed load is tried again by the next solve
  });
  return loading;
}

addEventListener("message", async ({ data: { id, request, table, check, size } }) => {
  try {
    const loaded = await solver(size);
    if (table) return postMessage({ id, table: loaded.settings() });
    if (check) {
      const checked = loaded.check(check);
      return postMessage(checked.error !== undefined ? { id, error: checked.error } : { id, table: checked });
    }
    const answer = loaded.solve(request, (event) => {
      postMessage({ id, event });
    });
    postMessage({ id, answer });
  } catch (e) {
    postMessage({ id, error: e instanceof Error ? e.message : String(e) });
  }
});
