// The core in a module Worker: it loads core.js and the module next to it, and answers each message
// { id, request } with { id, document } for a build, { id, written } for a write, or { id, error },
// in the order they came. The request is an object or, as client.js sends it, its JSON text. Every
// request gets the packages this build brings (packages.js): their files under the request's own,
// their directories after the request's. It runs in browsers and, for the tests, as a Node worker
// thread.

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
  return { ...r, files: { ...packageFiles, ...r.files }, package_dirs: [...(r.package_dirs ?? []), ...packageDirs] };
}

async function answer({ id, request }) {
  let reply;
  try {
    const response = (await core).respond(withPackages(request));
    reply = { id, ...response };
  } catch (e) {
    reply = { id, error: e instanceof Error ? e.message : String(e) };
  }
  port.postMessage(reply);
}

if (node) port.on("message", answer);
else port.addEventListener("message", (e) => answer(e.data));
