// The core as WebAssembly: the module the binding builds (rnl_runtime_wasm.wasm), behind one call
// that takes a request and returns the Document, or for a write the netlist written and its
// Document. The call is synchronous, so the web viewer runs it in a Worker (worker.js); the tests
// run it in Node.

/**
 * What to build: the workspace and how. Fields the core does not know are ignored.
 *
 * @typedef {object} Request
 * @property {string} entry The path of the file to open, as the frontend names it, such as `/w/top.sp`.
 * @property {Record<string, string>} [files] The texts of the workspace by path, the entry's among them.
 * @property {string[]} [package_dirs] The directories, among the paths of `files`, whose packages are
 *   found by name.
 * @property {"spice" | "spectre"} [dialect] The dialect, instead of guessing it from the entry.
 * @property {boolean} [std] Use the standard package at the root; true when not given.
 * @property {boolean} [svg] Write each level's SVG; true when not given.
 * @property {boolean} [primitives] Give each level's primitives; false when not given.
 * @property {boolean} [draft] For development only: read the files that declare the draft 0.3 as
 *   well; false when not given.
 */

/**
 * What to write: an edit transaction's ops into its netlist, and how to build what was written.
 * The netlist is the entry; the other fields are a Request's.
 *
 * @typedef {object} WriteRequest
 * @property {{ dialect: "spice" | "spectre", netlist: string, author: string, ops: object[] }} write
 *   The netlist, without RNL records, its dialect, the author of the records, and the ops as the
 *   editor interface draft writes them.
 * @property {Record<string, string>} [files] Further files, such as packages.
 * @property {string[]} [package_dirs]
 * @property {boolean} [std]
 * @property {boolean} [svg]
 * @property {boolean} [primitives]
 */

/**
 * The core, loaded.
 *
 * @typedef {object} Core
 * @property {(request: Request | string) => object} build Build the Document of a request, given as
 *   an object or as the JSON text `requestText` makes of one. Throws an Error with the core's message
 *   when there is none, such as when the entry is not among the files.
 * @property {(request: WriteRequest | string) => { text: string, document: object }} write Write the
 *   ops of a request into its netlist, and build what was written. Throws an Error with the core's
 *   message when an op cannot be written.
 * @property {(request: Request | WriteRequest | string) => object} respond The core's answer as it
 *   gives it: `{ document }`, `{ written }` or `{ error }`. Throws only when the core stops.
 */

const encoder = new TextEncoder();
const decoder = new TextDecoder();
const REPLACEMENT = String.fromCharCode(0xfffd);

/**
 * A request as the JSON text the core reads: what JSON.stringify makes of it, so that functions
 * are left out and a Proxy is read through. A lone surrogate, which JSON.stringify writes as an
 * escape the core refuses, becomes U+FFFD, as TextEncoder makes it.
 *
 * @param {Request} request
 * @returns {string}
 */
export function requestText(request) {
  // escapes are read one at a time, so that an escaped backslash is not taken for the start of one
  const escape = /\\(?:u(d[89a-f][0-9a-f]{2})|[^])/g;
  return (JSON.stringify(request) ?? "null").replace(escape, (e, lone) => (lone ? REPLACEMENT : e));
}

/**
 * Load the core.
 *
 * @param {string | URL | Response | BufferSource | WebAssembly.Module} wasm The module: where it is,
 *   the response that fetched it, its bytes, or the module compiled.
 * @returns {Promise<Core>}
 */
export async function loadCore(wasm) {
  const module = await compile(wasm);
  let instance = await WebAssembly.instantiate(module, {});

  /**
   * After a trap the instance must not answer again: make a new one. A page's main thread may refuse
   * to make one synchronously; it is then made asynchronously, and builds fail until it is there.
   */
  function restart() {
    try {
      instance = new WebAssembly.Instance(module, {});
    } catch {
      instance = null;
      WebAssembly.instantiate(module, {}).then((i) => (instance = i));
    }
  }

  function respond(request) {
    if (!instance) throw new Error("The core is starting again after a failure; build again in a moment.");
    const exports = instance.exports;
    const bytes = encoder.encode(typeof request === "string" ? request : requestText(request));
    let answer;
    try {
      const at = exports.rnl_request(bytes.length) >>> 0;
      new Uint8Array(exports.memory.buffer, at, bytes.length).set(bytes);
      const length = exports.rnl_build() >>> 0;
      answer = text(exports.memory, exports.rnl_response(), length);
    } catch (e) {
      // a panic, or a trap such as running out of memory
      const message = stopMessage(exports);
      restart();
      throw new Error(`The core stopped: ${message || e.message}`, { cause: e });
    }
    return JSON.parse(answer);
  }

  /** The part of the answer a call wants, or the core's error. */
  function part(request, name) {
    const response = respond(request);
    if ("error" in response) throw new Error(response.error);
    if (!(name in response)) throw new Error(`The core answered with no ${name}: the request is not a ${name === "document" ? "build" : "write"}.`);
    return response[name];
  }

  return {
    respond,
    build: (request) => part(request, "document"),
    write: (request) => part(request, "written"),
  };
}

/** The module compiled, from wherever it is. */
async function compile(wasm) {
  if (wasm instanceof WebAssembly.Module) return wasm;
  if (typeof wasm === "string" || wasm instanceof URL) wasm = await fetchModule(new URL(wasm, globalThis.location?.href));
  if (wasm instanceof Response) {
    if (!wasm.ok) throw new Error(`The core could not be loaded from ${wasm.url}: ${wasm.status} ${wasm.statusText}`);
    // streaming needs the right media type, which a plain file server may not send
    const type = (wasm.headers.get("content-type") || "").split(";")[0].trim();
    if (type === "application/wasm" && WebAssembly.compileStreaming) return WebAssembly.compileStreaming(wasm);
    wasm = await wasm.arrayBuffer();
  }
  return WebAssembly.compile(wasm);
}

/** Fetch the module; in Node, whose fetch does not read files, read a file: URL from the disk. */
async function fetchModule(url) {
  if (url.protocol !== "file:") return fetch(url);
  const { readFile } = await import("node:fs/promises");
  return readFile(url);
}

/**
 * Why the last call stopped: the message the panic hook kept, or that an allocation failed, which
 * aborts without a panic; "" when the module does not say or cannot be read.
 */
function stopMessage(exports) {
  try {
    const length = exports.rnl_panic_len() >>> 0;
    if (length) return text(exports.memory, exports.rnl_panic_message(), length);
    const failed = exports.rnl_alloc_failed() >>> 0;
    if (!failed) return "";
    const used = Math.round(exports.memory.buffer.byteLength / 2 ** 20);
    return `it ran out of memory, with ${used} MB in use, when it asked for ${failed} bytes more`;
  } catch {
    return "";
  }
}

/** UTF-8 text in the module's memory. */
function text(memory, at, length) {
  return decoder.decode(new Uint8Array(memory.buffer, at >>> 0, length));
}
