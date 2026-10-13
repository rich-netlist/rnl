// The core for a frontend: started in a module Worker (worker.js) so that building never blocks the
// page, and the files a document needs, collected through the frontend's own reading. In Node the
// Worker is a worker thread.

/**
 * The core, running in a Worker.
 *
 * @typedef {object} CoreWorker
 * @property {(request: import("./core.js").Request) => Promise<object>} build Build the Document of a
 *   request; rejects with an Error with the core's message when there is none.
 * @property {() => void} terminate Stop the Worker; builds still waiting, and later ones, reject.
 */

/**
 * Start the core in a module Worker.
 *
 * @param {{ worker?: string | URL }} [options] `worker`: where worker.js is, by default next to this
 *   module. The module it loads is next to it.
 * @returns {CoreWorker}
 */
export function startCore(options = {}) {
  const url = new URL(options.worker ?? "./worker.js", import.meta.url);
  const waiting = new Map(); // by request id: the promise's resolve and reject
  let next = 0;
  let stopped = null; // why the Worker no longer answers

  function stop(error) {
    stopped ??= error;
    for (const { reject } of waiting.values()) reject(stopped);
    waiting.clear();
  }

  function receive({ id, document, error }) {
    const promise = waiting.get(id);
    if (!promise) return;
    waiting.delete(id);
    if (error !== undefined) promise.reject(new Error(error));
    else promise.resolve(document);
  }

  const worker = spawn(url, receive, stop);
  worker.catch(stop);

  return {
    build(request) {
      if (stopped) return Promise.reject(stopped);
      const id = next++;
      return new Promise((resolve, reject) => {
        waiting.set(id, { resolve, reject });
        worker.then((w) => w.postMessage({ id, request }), () => {});
      });
    },
    terminate() {
      stop(new Error("The core was stopped."));
      worker.then((w) => w.terminate(), () => {});
    },
  };
}

/** A module Worker: the browser's, or a Node worker thread, with the same messages. */
async function spawn(url, receive, stop) {
  const failed = (message) => stop(new Error(`The core's Worker failed: ${message}`));
  if (typeof Worker === "function") {
    const worker = new Worker(url, { type: "module" });
    worker.addEventListener("message", (e) => receive(e.data));
    worker.addEventListener("error", (e) => failed(e.message || `${url} could not be loaded`));
    return worker;
  }
  const { Worker: NodeWorker } = await import("node:worker_threads");
  const worker = new NodeWorker(url);
  worker.on("message", receive);
  worker.on("error", (e) => failed(e.message));
  worker.on("exit", (code) => failed(`it exited with code ${code}`));
  return worker;
}

/**
 * Build a document with the files it needs. Build, read what `Document.missing` lists that was not
 * tried yet, and build again, until nothing new can be read. The entry is read first when it is not
 * among the files. Packages found by name are never missing: give the files of the package
 * directories in the request.
 *
 * @param {{ build(request: import("./core.js").Request): object | Promise<object> }} core The core:
 *   from startCore, or from core.js's loadCore.
 * @param {import("./core.js").Request} request What to build; its `files` are the files known so far.
 * @param {(path: string) => Promise<string | null>} read Read a file by the path the core asked for:
 *   its text, or null when there is none.
 * @returns {Promise<{ document: object, files: Record<string, string> }>} The last Document, and the
 *   files it was built from.
 */
export async function collect(core, request, read) {
  const files = { ...request.files };
  const tried = new Set(Object.keys(files));
  let wanted = request.entry in files ? [] : [request.entry];
  let document = null;
  for (;;) {
    const texts = await Promise.all(wanted.map((path) => read(path)));
    let found = false;
    wanted.forEach((path, k) => {
      tried.add(path);
      if (typeof texts[k] === "string") {
        files[path] = texts[k];
        found = true;
      }
    });
    if (document && !found) return { document, files };
    document = await core.build({ ...request, files });
    wanted = document.missing.filter((path) => !tried.has(path));
    if (wanted.length === 0) return { document, files };
  }
}
