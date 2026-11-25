// The core for a frontend: started in a module Worker (worker.js) so that building never blocks the
// page, and the files a document needs, collected through the frontend's own reading. In Node the
// Worker is a worker thread.

import { requestText } from "./core.js";

export { coreVersion, fileResultFormat } from "./version.js";

/**
 * The core, running in a Worker.
 *
 * @typedef {object} CoreWorker
 * @property {(request: import("./core.js").Request) => Promise<object>} build Build the Document of a
 *   request; rejects with an Error with the core's message when there is none. The request is sent
 *   as the JSON text `requestText` makes of it, so it is read as loadCore's build reads it.
 * @property {(request: import("./core.js").Request) => Promise<object>} snapshot Build the snapshot of
 *   a request with `parts`, the editor interface's view of the document (snapshot.d.ts).
 * @property {(request: import("./core.js").Request) => Promise<object>} outline The outline of a
 *   request's entry: its files, levels and the subcircuits each calls, the language version and
 *   packages of each file, and the diagnostics of reading the netlist, with no RNL interpreted
 *   (`Outline` of document.d.ts).
 * @property {(request: import("./core.js").Request) => Promise<object>} buildFile The reading of the
 *   request's `file`, one file of the document, in the context of its entry (`FileResult` of
 *   document.d.ts). Packages the Worker classified for an earlier request are read again; keep one
 *   Worker for a run of them all the same, as later versions keep them.
 * @property {(request: import("./core.js").Request) => Promise<object>} hintsOf The geometry hints
 *   of the request's `hints_of`, a file of the document, from the reading of it that the last file
 *   request with `cache` and without `hints` kept (`FileHints` of document.d.ts). Rejects with the
 *   message "not kept" when the Worker keeps no such reading; it gives them once.
 * @property {(request: import("./core.js").Request) => Promise<object>} restOf What the answer of
 *   the request's `rest_of`, a file of the document, was left without: its spans, which the last
 *   file request with `cache`, `"spans": false` and without `hints` kept, and its geometry hints
 *   (`FileRest` of document.d.ts). Rejects with the message "not kept" when the Worker keeps no
 *   such reading; it gives them once.
 * @property {(request: import("./core.js").Request) => Promise<object>} symbols The symbols the
 *   subcircuits of the request's `symbols`, a file of the document, give themselves (`FileSymbols`
 *   of document.d.ts).
 * @property {(request: import("./core.js").WriteRequest) => Promise<{ text: string, document: object }>}
 *   write Write the ops of a request into its netlist, and build what was written: the netlist
 *   written and its Document, whose diagnostics say how it reads. Rejects with the core's message
 *   when an op cannot be written.
 * @property {(request: import("./core.js").ReplaceRecordsRequest) => Promise<import("./core.js").Replaced>}
 *   replaceRecords Replace the records of a level of a netlist that has records already, in the file
 *   that holds the level: the path of that file and its new text. Rejects with the core's message, which
 *   begins with the kind of the refusal, when the level is not found, has no place for a record, or the
 *   records given do not read.
 * @property {(request: import("./core.js").CatalogRequest) => Promise<object>} catalog List the
 *   symbols, definitions and sets of the packages the core has, the bundled ones and those of the
 *   request's package directories (catalog.d.ts).
 * @property {(request: import("./core.js").DrawingsRequest) => Promise<object[]>} drawings The
 *   drawings, `{ svg?, primitives? }` each, of the objects without a position that the last build
 *   with `lazy_unplaced` listed, as the request names them; rejects when none was built, an object
 *   is not there, or `names` are not those of the build.
 * @property {(request: {library: {reload?: boolean, refresh?: string[]}, pool: string[], files: Record<string, string>, formats?: Record<string, string>}) => Promise<string[]>} library Update the viewer library index, reading its files with the formats set for them; returns read errors.
 * @property {(request: import("./core.js").FormatsRequest) => Promise<{ list: object[], detected: Record<string, string> }>} formats
 *   The formats a file can be read with, and the format each file of `detect` has when none is set.
 * @property {() => Promise<Record<string, string>>} bundled The bundled packages a frontend lists among
 *   its built-in ones, by path with their texts: all the core has but those of a draft series.
 * @property {(paths: string[]) => Promise<{ files: string[], truncated: string[], limit: number }>} folders The files of
 *   folders as a browser gives them (each path's first part the folder included, or a file given alone) that the Library
 *   lists, as every viewer lists a folder included; the folders that had more files than are listed, by name; and how
 *   many files a folder brings in at most.
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
  // the texts of the files the Worker holds, by path, as the last request with files sent them (`message`)
  let held = new Map();

  function stop(error) {
    stopped ??= error;
    for (const { reject } of waiting.values()) reject(stopped);
    waiting.clear();
  }

  function receive(reply) {
    const promise = waiting.get(reply.id);
    if (!promise) return;
    waiting.delete(reply.id);
    if (reply.error !== undefined) {
      const error = new Error(reply.error);
      if ("code" in reply) {
        error.code = reply.code;
        error.diagnosticName = reply.name;
        error.diagnostics = reply.diagnostics;
      }
      promise.reject(error);
    } else if (!(promise.part in reply)) promise.reject(new Error(`The core answered with no ${promise.part}.`));
    else promise.resolve(reply[promise.part]);
  }

  const worker = spawn(options.worker, receive, stop);
  worker.catch(stop);

  /**
   * The message of a request, as the request is now, made when it is sent: the message, and what to call once it was
   * sent. The message has its JSON text, as `requestText` makes it. A request with `files`
   * gives the Worker only the texts that are not those it holds, with the paths of all of them in their order: the
   * Worker keeps the texts, and builds the request with them, the same files in the same order (worker.js). The texts
   * of a large document are then not written out and sent again for each request. The request is read when the call is
   * made, its nested parts too, as a request without files is.
   */
  function message(request) {
    const files = request?.files;
    if (!files || typeof files !== "object" || Array.isArray(files)) {
      const text = requestText(request);
      return () => [{ request: text }, () => {}];
    }
    // (a file whose text is undefined is left out, as JSON leaves it out)
    const named = Object.keys(files).filter((path) => files[path] !== undefined);
    const texts = named.map((path) => files[path]);
    // the paths as the JSON text names them, where a lone surrogate is U+FFFD (`requestText`): the Worker finds the
    // texts by them
    const paths = named.map((path) => JSON.parse(requestText(path)));
    // the rest of the request as it is now, its files in their place (JSON.parse gives a lone surrogate back as it was)
    const rest = JSON.parse(JSON.stringify({ ...request, files: 0 }));
    // (made when it is sent, after those sent before it: what the Worker holds is what they left; and what it holds
    // then is told once it was sent)
    return () => {
      const given = {};
      paths.forEach((path, k) => {
        if (!held.has(path) || held.get(path) !== texts[k]) Object.defineProperty(given, path, { value: texts[k], enumerable: true });
      });
      const text = requestText({ ...rest, files: given });
      return [{ request: text, paths }, () => (held = new Map(paths.map((path, k) => [path, texts[k]])))];
    };
  }

  /** Send a request, and promise the part of the answer wanted. */
  function send(request, part) {
    if (stopped) return Promise.reject(stopped);
    let made;
    try {
      made = message(request);
    } catch (e) {
      return Promise.reject(e);
    }
    const id = next++;
    return new Promise((resolve, reject) => {
      waiting.set(id, { part, resolve, reject });
      worker.then(
        (w) => {
          try {
            const [data, sent] = made();
            w.postMessage({ id, ...data });
            sent();
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
    outline: (request) => send({ ...request, outline: true }, "outline"),
    buildFile: (request) => send(request, "file"),
    hintsOf: (request) => send(request, "hints"),
    restOf: (request) => send(request, "rest"),
    symbols: (request) => send(request, "symbols"),
    write: (request) => send(request, "written"),
    replaceRecords: (request) => send(request, "replaced"),
    catalog: (request) => send(request, "catalog"),
    drawings: (request) => send(request, "drawings"),
    library: (request) => send(request, "library"),
    formats: (request) => send(request, "formats"),
    bundled: () => send({ bundled: true }, "bundled"),
    folders: (paths) => send({ folders: { paths } }, "folders"),
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
  const { answer: document, files } = await gather((r) => core.build(r), request, read);
  return { document, files };
}

/**
 * The outline of a document with the files it needs, read as `collect` reads them: ask for the
 * outline, read what `Outline.missing` lists that was not tried yet, and ask again, until nothing
 * new can be read. No RNL is interpreted.
 *
 * @param {{ outline(request: import("./core.js").Request): object | Promise<object> }} core The core:
 *   from startCore, or from core.js's loadCore.
 * @param {import("./core.js").Request} request What to outline; its `files` are the files known so far.
 * @param {(path: string) => Promise<string | null>} read Read a file by the path the core asked for,
 *   as for `collect`.
 * @returns {Promise<{ outline: object, files: Record<string, string> }>} The last outline, and the
 *   files it was read from.
 */
export async function collectOutline(core, request, read) {
  const { answer: outline, files } = await gather((r) => core.outline(r), request, read);
  return { outline, files };
}

/**
 * The reading of one file of a document with the files it needs, read as `collect` reads them: ask
 * for the request's `file`, read what `FileResult.missing` lists that was not tried yet, and ask
 * again, until nothing new can be read.
 *
 * @param {{ buildFile(request: import("./core.js").Request): object | Promise<object> }} core The core:
 *   from startCore, or from core.js's loadCore.
 * @param {import("./core.js").Request} request What to read, with its `file`; its `files` are the
 *   files known so far.
 * @param {(path: string) => Promise<string | null>} read Read a file by the path the core asked for,
 *   as for `collect`.
 * @returns {Promise<{ result: object, files: Record<string, string> }>} The last reading, and the
 *   files it was made from.
 */
export async function collectFile(core, request, read) {
  const { answer: result, files } = await gather((r) => core.buildFile(r), request, read);
  return { result, files };
}

/**
 * Ask the core, read what its answer lists as missing that was not tried yet, and ask again, until
 * nothing new can be read: the last answer, and the files it was made from.
 */
async function gather(ask, request, read) {
  const files = { ...request.files };
  const tried = new Set(Object.keys(files));
  let wanted = request.entry in files ? [] : [request.entry];
  let answer = null;
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
    if (answer && !found) return { answer, files };
    answer = await ask({ ...request, files });
    wanted = answer.missing.filter((path) => !tried.has(path));
    if (wanted.length === 0) return { answer, files };
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
