// The core in a module Worker: it loads core.js and the module next to it, and answers each message
// { id, request } with { id, document } or { id, error }, in the order they came. The request is an
// object or, as client.js sends it, its JSON text. It runs in browsers and, for the tests, as a Node
// worker thread.

import { loadCore } from "./core.js";

const core = loadCore(new URL("./rnl_runtime_wasm.wasm", import.meta.url));
core.catch(() => {}); // a module that cannot be loaded is the error of every answer

const node = typeof postMessage !== "function";
const port = node ? (await import("node:worker_threads")).parentPort : globalThis;

async function answer({ id, request }) {
  let reply;
  try {
    reply = { id, document: (await core).build(request) };
  } catch (e) {
    reply = { id, error: e instanceof Error ? e.message : String(e) };
  }
  port.postMessage(reply);
}

if (node) port.on("message", answer);
else port.addEventListener("message", (e) => answer(e.data));
