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
 *   limit is measured on; `performance.now()` when not given. It has to go on: on a clock that
 *   stands still a time limit never ends, so give a limit of work with such a clock.
 * @returns {Promise<{ solve(request: SolveRequest, onProgress?: (event: object) => boolean | void): SolveAnswer }>}
 *   `onProgress` gets each progress event as docs/solver-progress.md describes it. When it
 *   returns `false` the solve is cancelled, and the answer is `{ cancelled: true }`. When it
 *   throws, the solve is cancelled too, and `solve` throws what it threw.
 */
export async function loadSolver(source, { now = () => performance.now() } = {}) {
  const decoder = new TextDecoder();
  const encoder = new TextEncoder();
  let onProgress = () => {};
  // set by an event's callback: asked by the solver after each event
  let cancelled = false;
  let thrown = null;
  let instance = null;
  const text = (ptr, len) => decoder.decode(new Uint8Array(instance.exports.memory.buffer, ptr >>> 0, len >>> 0));
  const imports = {
    env: {
      rnl_now: () => now(),
      // nothing is thrown through the module, which would not release what the call holds
      rnl_progress: (ptr, len) => {
        if (cancelled) return;
        try {
          if (onProgress(JSON.parse(text(ptr, len))) === false) cancelled = true;
        } catch (e) {
          thrown = { error: e };
          cancelled = true;
        }
      },
      rnl_cancelled: () => (cancelled ? 1 : 0),
    },
  };
  const module = source instanceof WebAssembly.Module ? source : await WebAssembly.compile(source);
  instance = await WebAssembly.instantiate(module, imports);

  /**
   * After a trap the instance must not answer again: make a new one. A page's main thread may
   * refuse to make one synchronously; it is then made asynchronously, and solves fail until it is
   * there. A Worker, where the solver belongs, makes it at once.
   */
  function restart() {
    try {
      instance = new WebAssembly.Instance(module, imports);
    } catch {
      instance = null;
      WebAssembly.instantiate(module, imports).then((i) => (instance = i));
    }
  }

  return {
    solve(request, progress) {
      if (!instance) throw new Error("The solver is starting again after a failure; solve again in a moment.");
      onProgress = progress ?? (() => {});
      cancelled = false;
      thrown = null;
      const api = instance.exports;
      const bytes = encoder.encode(JSON.stringify(request));
      let answer;
      try {
        const at = api.rnl_request(bytes.length) >>> 0;
        new Uint8Array(api.memory.buffer, at, bytes.length).set(bytes);
        const length = api.rnl_solve() >>> 0;
        answer = text(api.rnl_response(), length);
      } catch (e) {
        // a panic, or a trap such as running out of memory
        let why = "";
        try {
          const panic = api.rnl_panic_len() >>> 0;
          const failed = api.rnl_alloc_failed() >>> 0;
          why = panic ? text(api.rnl_panic_message(), panic) : failed ? `out of memory allocating ${failed} bytes` : "";
        } catch {
          // the instance cannot say
        }
        restart();
        throw new Error(`The solver stopped: ${why || e.message}`, { cause: e });
      }
      if (thrown) throw thrown.error;
      return JSON.parse(answer);
    },
  };
}
