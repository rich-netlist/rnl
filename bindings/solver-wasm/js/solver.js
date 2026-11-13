// The solver as WebAssembly: the module bindings/solver-wasm builds (rnl_solver_wasm.wasm), behind
// one call that takes a solve request and gives the Document drawn as solved, the report and the
// texts written, as `rnl preview` does. The call is synchronous and long, so a page runs it in a
// Worker of its own, and terminates that Worker to cancel: every request carries all it reads,
// and the core's Worker does not depend on it. The tests run it in Node.

/**
 * What to solve. Fields the solver does not know are ignored.
 *
 * @typedef {object} SolveRequest
 * @property {string} entry The path of the file to solve, as the frontend names it.
 * @property {Record<string, string>} [files] The texts of the workspace by path, the entry's among them.
 * @property {string[]} [package_dirs] The directories, among the paths of `files`, whose packages
 *   are found by name.
 * @property {"spice" | "spectre"} [dialect] The format of the entry, instead of guessing it.
 * @property {Record<string, string>} [formats] The format set for files, by path as in `files`.
 * @property {boolean} [std] Use the standard package at the root; true when not given.
 * @property {boolean} [include_std] Add the standard package to draft 0.4 files.
 * @property {string} [solver_config] The solver settings as RNL named parameters, as
 *   `rnl solve --solver-config` takes them, such as `limit="time",seconds=10`.
 * @property {string} [level] Only the level with this title.
 * @property {boolean} [respace] Space each level again from its record of the solver, nothing
 *   searched (docs/solver-respace.md).
 */

/**
 * The answer: the Document drawn as solved, the report and the texts written by path; or
 * `{ cancelled: true }`; or `{ error }`.
 *
 * @typedef {{ document: object, report: object, files: Record<string, string> } | { cancelled: true } | { error: string }} SolveAnswer
 */

/**
 * Instantiate the solver from the module's bytes or a compiled `WebAssembly.Module`.
 *
 * @param {BufferSource | WebAssembly.Module} source
 * @param {{ now?: () => number }} [options] `now` gives milliseconds, which the solver's time
 *   limit is measured on; `performance.now()` when not given.
 * @returns {Promise<{ solve(request: SolveRequest, onProgress?: (event: object) => void): SolveAnswer }>}
 *   `onProgress` gets each progress event as docs/solver-progress.md describes it.
 */
export async function loadSolver(source, { now = () => performance.now() } = {}) {
  const decoder = new TextDecoder();
  const encoder = new TextEncoder();
  let memory = null;
  let onProgress = () => {};
  const text = (ptr, len) => decoder.decode(new Uint8Array(memory.buffer, ptr >>> 0, len >>> 0));
  const imports = {
    env: {
      rnl_now: () => now(),
      rnl_progress: (ptr, len) => onProgress(JSON.parse(text(ptr, len))),
      // cancelling is terminating the Worker
      rnl_cancelled: () => 0,
    },
  };
  const made = await WebAssembly.instantiate(source, imports);
  const api = (made.instance ?? made).exports;
  memory = api.memory;
  return {
    solve(request, progress) {
      onProgress = progress ?? (() => {});
      const bytes = encoder.encode(JSON.stringify(request));
      const at = api.rnl_request(bytes.length) >>> 0;
      new Uint8Array(memory.buffer, at, bytes.length).set(bytes);
      let length;
      try {
        length = api.rnl_solve() >>> 0;
      } catch (e) {
        const panic = api.rnl_panic_len() >>> 0;
        const failed = api.rnl_alloc_failed() >>> 0;
        const why = panic ? text(api.rnl_panic_message(), panic) : failed ? `out of memory allocating ${failed} bytes` : String(e);
        throw new Error(`the solver stopped: ${why}`);
      }
      return JSON.parse(text(api.rnl_response(), length));
    },
  };
}
