// The core for a frontend: started in a module Worker (worker.js) so that building never blocks the
// page, and the files a document needs, collected through the frontend's own reading. In Node the
// Worker is a worker thread.

import { requestText } from "./core.js";

/**
 * The core, running in a Worker.
 *
 * @typedef {object} CoreWorker
 * @property {(request: import("./core.js").Request) => Promise<object>} build Build the Document of a
 *   request; rejects with an Error with the core's message when there is none. The request is sent
 *   as the JSON text `requestText` makes of it, so it is read as loadCore's build reads it.
 * @property {(request: import("./core.js").Request) => Promise<object>} snapshot Build the snapshot of
 *   a request with `parts`, the editor interface's view of the document (snapshot.d.ts).
 * @property {(request: import("./core.js").WriteRequest) => Promise<{ text: string, document: object }>}
 *   write Write the ops of a request into its netlist, and build what was written: the netlist
 *   written and its Document, whose diagnostics say how it reads. Rejects with the core's message
 *   when an op cannot be written.
 * @property {(request: import("./core.js").CatalogRequest) => Promise<object>} catalog List the
 *   symbols, definitions and sets of the packages the core has, the bundled ones and those of the
 *   request's package directories (catalog.d.ts).
 * @property {(request: import("./core.js").DrawingsRequest) => Promise<object[]>} drawings The
 *   drawings, `{ svg?, primitives? }` each, of the objects without a position that the last build
 *   with `lazy_unplaced` listed, as the request names them; rejects when none was built, an object
 *   is not there, or `names` are not those of the build.
 * @property {() => void} terminate Stop the Worker; calls still waiting, and later ones, reject.
 */

/**
 * Start the core in a module Worker.
 *
 * @param {{ worker?: string | URL }} [options] `worker`: where worker.js is, by default next to this
 *   module. The module it loads is next to it.
 * @returns {CoreWorker}
 */
export function startCore(options = {}) {
  const waiting = new Map(); // by request id: the part of the answer wanted, the promise's resolve and reject
  let next = 0;
  let stopped = null; // why the Worker no longer answers

  function stop(error) {
    stopped ??= error;
    for (const { reject } of waiting.values()) reject(stopped);
    waiting.clear();
  }

  function receive(reply) {
    const promise = waiting.get(reply.id);
    if (!promise) return;
    waiting.delete(reply.id);
    if (reply.error !== undefined) promise.reject(new Error(reply.error));
    else if (!(promise.part in reply)) promise.reject(new Error(`The core answered with no ${promise.part}.`));
    else promise.resolve(reply[promise.part]);
  }

  const worker = spawn(options.worker, receive, stop);
  worker.catch(stop);

  /** Send a request, and promise the part of the answer wanted. */
  function send(request, part) {
    if (stopped) return Promise.reject(stopped);
    let text;
    try {
      text = requestText(request);
    } catch (e) {
      return Promise.reject(e);
    }
    const id = next++;
    return new Promise((resolve, reject) => {
      waiting.set(id, { part, resolve, reject });
      worker.then(
        (w) => {
          try {
            w.postMessage({ id, request: text });
          } catch (e) {
            waiting.delete(id);
            reject(e);
          }
        },
        () => {},
      );
    });
  }

  return {
    build: (request) => send(request, "document"),
    snapshot: (request) => send(request, "snapshot"),
    write: (request) => send(request, "written"),
    catalog: (request) => send(request, "catalog"),
    drawings: (request) => send(request, "drawings"),
    terminate() {
      stop(new Error("The core was stopped."));
      worker.then((w) => w.terminate(), () => {});
    },
  };
}

/**
 * A module Worker: the browser's, or a Node worker thread, with the same messages. Without `where`,
 * worker.js next to this module, by a URL written out in full, as bundlers such as Vite need to find
 * the Worker and bundle it.
 */
async function spawn(where, receive, stop) {
  const failed = (message) => stop(new Error(`The core's Worker failed: ${message}`));
  if (typeof Worker === "function") {
    const worker =
      where === undefined
        ? new Worker(new URL("./worker.js", import.meta.url), { type: "module" })
        : new Worker(new URL(where, import.meta.url), { type: "module" });
    worker.addEventListener("message", (e) => receive(e.data));
    worker.addEventListener("error", (e) => failed(e.message || `${where ?? "worker.js"} could not be loaded`));
    return worker;
  }
  const { Worker: NodeWorker } = await import("node:worker_threads");
  const worker = new NodeWorker(new URL(where ?? "./worker.js", import.meta.url));
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
 *   its text, or null when there is none. `decodeText` makes the text of its bytes as rnl does.
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

/**
 * A file's text, from its bytes, as rnl reads a file: UTF-8 without a byte order mark, or Latin-1,
 * each byte the character of that code, when it is not UTF-8. A frontend that reads bytes, from a
 * fetch or a dropped file, decodes them with this, so that its Documents are the CLI's and the
 * desktop viewer's.
 *
 * @param {BufferSource} bytes
 * @returns {string}
 */
export function decodeText(bytes) {
  try {
    const text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
    return text.charCodeAt(0) === 0xfeff ? text.slice(1) : text;
  } catch {
    // not TextDecoder's "latin1", which is windows-1252
    const b = ArrayBuffer.isView(bytes) ? new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength) : new Uint8Array(bytes);
    let text = "";
    for (let k = 0; k < b.length; k += 0x8000) text += String.fromCharCode(...b.subarray(k, k + 0x8000));
    return text;
  }
}
