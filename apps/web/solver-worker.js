// The solver of the web viewer, in a module Worker of its own (decision 0010). The page starts it when
// the user first solves, so that no one who only looks at a netlist loads the solver's module, which
// is larger than the core's (core/rnl_solver_wasm.wasm, bindings/solver-wasm). A solve is one
// synchronous call that can take the solver's whole time limit; here it blocks nothing of the page.
//
// It answers each message { id, request } (a SolveRequest of bindings/solver-wasm/js/solver.d.ts)
// with { id, event } for each event of the solve as it happens (docs/solver-progress.md), and
// then { id, answer }, the solver's answer: { document, report, files }, { cancelled: true } or
// { error }; a message { id, table: true } with { id, table }, the table of the solver's settings
// (docs/solver-settings-table.md); or { id, error } when the module could not be loaded or the call threw. The page
// cancels a solve by terminating the Worker, which the solver's module is written for: every
// request carries all it reads, so a new Worker needs nothing of the old one.

import { loadSolver } from "./core/solver.js";

let loading = null;

/** The solver, loaded once: the module is fetched and compiled on the first solve. */
function solver() {
  loading ??= (async () => {
    const response = await fetch(new URL("./core/rnl_solver_wasm.wasm", import.meta.url));
    if (!response.ok) throw new Error(`the solver's module could not be loaded (${response.status})`);
    return loadSolver(await WebAssembly.compile(await response.arrayBuffer()));
  })();
  loading.catch(() => {
    loading = null; // a failed load is tried again by the next solve
  });
  return loading;
}

addEventListener("message", async ({ data: { id, request, table } }) => {
  try {
    const loaded = await solver();
    if (table) return postMessage({ id, table: loaded.settings() });
    const answer = loaded.solve(request, (event) => {
      postMessage({ id, event });
    });
    postMessage({ id, answer });
  } catch (e) {
    postMessage({ id, error: e instanceof Error ? e.message : String(e) });
  }
});
