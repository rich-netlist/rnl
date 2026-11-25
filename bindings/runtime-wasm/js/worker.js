// The core in a module Worker: it loads core.js and the module next to it, and answers each message
// { id, request } with { id, document } for a build, { id, snapshot } for a build with parts,
// { id, outline } for an outline request, { id, file } for the reading of one file, { id, symbols }
// for the symbols of a file, { id, written } for a write, { id, replaced } for a
// replace-records request, { id, catalog } for a catalog request, { id, drawings } for the drawings
// of objects the last build kept, { id, formats } for a formats request, { id, bundled } for the
// bundled packages a frontend lists, { id, folders } for the files of folders a browser gave, or { id, error },
// in the order they came. The request is an object or, as client.js sends it, its JSON text. A message with `paths`
// gives only the texts of its files that the Worker does not hold: it keeps the texts of the last such message, and
// the request's files are those of `paths`, in that order, each with the text given or kept. Every request gets
// the packages this build brings (packages.js): their files under the request's own, their
// directories after the request's; an outline too, which reads the package files its records name by
// path. It runs in browsers and, for the tests, as a Node worker thread.

import { loadCore } from "./core.js";
import { files as packageFiles, packageDirs } from "./packages.js";

const core = loadCore(new URL("./rnl_runtime_wasm.wasm", import.meta.url));
core.catch(() => {}); // a module that cannot be loaded is the error of every answer

const node = typeof postMessage !== "function";
const port = node ? (await import("node:worker_threads")).parentPort : globalThis;

/** The request with the packages this build brings; as it came when there are none. */
function withPackages(request) {
  if (packageDirs.length === 0) return request;
  const r = typeof request === "string" ? JSON.parse(request) : request;
  // the drawings of the last build need no files; a folders request reads no file
  if (r.drawings !== undefined || r.folders !== undefined) return request;
  return { ...r, files: { ...packageFiles, ...r.files }, package_dirs: [...(r.package_dirs ?? []), ...packageDirs] };
}

// the texts of the files of the last message with `paths`, by path
let held = new Map();

/**
 * The request of a message with `paths`: its files those of `paths`, in order, with the texts it gives and those held
 * for the others, which are held for the next. The texts came from JSON text that client.js wrote as `requestText`
 * does, with no lone surrogate: the request is written again as that text, with JSON.stringify alone, unless the
 * packages of the build are added to it (`withPackages`).
 */
function tabled(text, paths) {
  const r = JSON.parse(text);
  const given = r.files ?? {};
  held = new Map(paths.map((path) => [path, Object.hasOwn(given, path) ? given[path] : held.get(path)]));
  r.files = Object.fromEntries(held);
  return packageDirs.length === 0 ? JSON.stringify(r) : r;
}

async function answer({ id, request }) {
  let reply;
  try {
    if (request instanceof Error) throw request;
    const response = (await core).respond(withPackages(request));
    reply = { id, ...response };
  } catch (e) {
    reply = { id, error: e instanceof Error ? e.message : String(e) };
  }
  port.postMessage(reply);
}

/** A message, as it comes: one with `paths` is made a request at once, so that each finds what those before it held. */
function receive({ id, request, paths }) {
  if (paths === undefined) return answer({ id, request });
  let made;
  try {
    made = tabled(request, paths);
  } catch (e) {
    made = e instanceof Error ? e : new Error(String(e));
  }
  return answer({ id, request: made });
}

if (node) port.on("message", receive);
else port.addEventListener("message", (e) => receive(e.data));
