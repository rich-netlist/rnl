// The solver service of the web viewer's development server (serve.mjs --rnl PATH): the page's Solve
// runs a debug build of the `rnl` command, so that a change to the solver is seen by building `rnl`
// again and pressing Solve, without building the viewer or restarting anything.
//
//   GET  /rnl-solver        {"service": "rnl-solver", "version": ..., "path": ..., "options": ...}
//   POST /rnl-solver/solve  {"entry", "files", "pool", "options"}  ->  {"document", "report"}
//
// A solve writes the files the page sends into a directory of its own, runs `rnl preview` on the entry
// there, as the desktop viewer solves (decision 0010), and gives what it prints: the Document of the
// netlist as solved, with its constraints, and the report of what became of each fact. The directory is
// removed afterwards.
// Nothing is written but there, and the `rnl` that runs is the one named when the server started.
//
// The service answers only the page of this server: the Host must be this computer's address and port,
// the Origin, when there is one, the server's own, and the body JSON, which another site cannot send
// without a preflight that no header here allows. It never takes an address from a request.

import { spawn } from "node:child_process";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, posix, resolve } from "node:path";

/** The most the page may send in one solve. */
const MAX_BODY = 512 * 1024 * 1024;

/** The result of `rnl --version`, for a path that does not run an `rnl`: an error. */
export function runVersion(rnl, prefix = []) {
  return new Promise((done, fail) => {
    const child = spawn(rnl, [...prefix, "--version"], { stdio: ["ignore", "pipe", "pipe"] });
    let out = "";
    child.stdout.on("data", (chunk) => (out += chunk));
    child.on("error", (e) => fail(new Error(`${rnl} cannot be run: ${e.message}`)));
    child.on("close", (code) => (code === 0 && /^rnl /.test(out) ? done(out.trim()) : fail(new Error(`${rnl} is not the rnl command`))));
  });
}

/**
 * A path of the page's workspace as a path under the solve's directory: the segments as they are, but
 * for those a file system cannot hold, such as the `<pool>` of the page's Library, which become `_pool_`.
 * Null for a path that is not absolute or that goes up.
 */
export function insidePath(path) {
  if (typeof path !== "string" || !path.startsWith("/") || path.includes("\0")) return null;
  const segments = path.split("/").slice(1);
  if (segments.some((s) => s === "" || s === "." || s === "..")) return null;
  return segments.map((s) => s.replace(/[<>:"|?*\\]/g, "_")).join("/");
}

/** Run `rnl` with `args` in `cwd`: its standard output; rejects with its error text on a failure, and kills it when `signal` is aborted. */
function run(rnl, prefix, args, cwd, signal) {
  return new Promise((done, fail) => {
    const child = spawn(rnl, [...prefix, ...args], { cwd, stdio: ["ignore", "pipe", "pipe"], windowsHide: true });
    const out = [];
    const err = [];
    const stop = () => child.kill();
    if (signal.aborted) stop();
    signal.addEventListener("abort", stop, { once: true });
    child.stdout.on("data", (chunk) => out.push(chunk));
    child.stderr.on("data", (chunk) => err.push(chunk));
    child.on("error", (e) => fail(new Error(`${rnl} cannot be run: ${e.message}`)));
    child.on("close", (code) => {
      signal.removeEventListener("abort", stop);
      if (signal.aborted) return fail(new Error("the solve was cancelled"));
      if (code === 0) return done(Buffer.concat(out).toString("utf8"));
      const text = Buffer.concat(err).toString("utf8").trim().split("\n").filter((l) => !/^(the solver is a preview|note: )/i.test(l)).join("\n");
      fail(new Error(text || `rnl ${args[0]} failed with exit code ${code}`));
    });
  });
}

/**
 * The solve of a request: what `rnl preview` prints, the Document and the report, as JSON text. `work` holds the
 * directory the files are written in; `options` are the solver's, as `--solver-options` takes them.
 */
export async function solve({ rnl, prefix = [], work, request, defaults, signal }) {
  const { entry, files, pool = [], options = "" } = request;
  const entryPath = insidePath(entry);
  if (!entryPath || !files || typeof files !== "object" || typeof files[entry] !== "string") throw new Error("the request has no entry file");
  await mkdir(work, { recursive: true });
  const dir = await mkdtemp(join(work, "solve-"));
  try {
    const packageDirs = new Set();
    const searchDirs = new Set();
    const poolSet = new Set(pool);
    for (const [path, text] of Object.entries(files)) {
      const inside = insidePath(path);
      if (inside === null || typeof text !== "string") continue;
      const target = resolve(dir, inside);
      await mkdir(dirname(target), { recursive: true });
      await writeFile(target, text);
      // the files of the Library are found by name: the packages by `--package-dir`, the includes by `--path`
      if (poolSet.has(path)) {
        searchDirs.add(dirname(target));
        if (path.endsWith(".rnl")) packageDirs.add(dirname(target));
      }
    }
    const solverOptions = [defaults, options].filter((o) => typeof o === "string" && o !== "").join(";");
    const common = [posix.basename(entryPath)];
    for (const d of packageDirs) common.push("--package-dir", d);
    for (const d of searchDirs) common.push("--path", d);
    if (solverOptions) common.push("--solver-options", solverOptions);
    const cwd = resolve(dir, dirname(entryPath));
    return await run(rnl, prefix, ["preview", ...common], cwd, signal);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
}

/** The addresses the page of a server on 127.0.0.1 is reached by. */
const hostOk = (host, port) => [`127.0.0.1:${port}`, `localhost:${port}`, `[::1]:${port}`].includes(host);

/**
 * The service for a server: `handle(request, response, pathname)` answers the service's requests and
 * returns true, or returns false for any other. `options` are settings of the solver that every solve
 * takes before the page's own; `prefix` the arguments to give `rnl` first, for a test that runs a script.
 */
export function createService({ rnl, prefix = [], version, options = "", work = join(tmpdir(), "rnl-solver-service") }) {
  const info = { service: "rnl-solver", version, path: rnl, options };
  const reply = (response, status, body) => {
    response.writeHead(status, { "Content-Type": "application/json; charset=utf-8", "Cache-Control": "no-store", "X-Content-Type-Options": "nosniff" });
    response.end(typeof body === "string" ? body : JSON.stringify(body));
  };
  return {
    info,
    async handle(request, response, pathname, port) {
      if (pathname !== "/rnl-solver" && pathname !== "/rnl-solver/solve") return false;
      if (!hostOk(request.headers.host, port)) return reply(response, 403, { error: "not this server's address" }), true;
      if (pathname === "/rnl-solver") {
        if (request.method !== "GET") return reply(response, 405, { error: "GET only" }), true;
        return reply(response, 200, info), true;
      }
      if (request.method !== "POST") return reply(response, 405, { error: "POST only" }), true;
      const origin = request.headers.origin;
      const site = request.headers["sec-fetch-site"];
      if ((origin !== undefined && origin !== `http://${request.headers.host}`) || (site !== undefined && site !== "same-origin")) {
        return reply(response, 403, { error: "only the page of this server may solve" }), true;
      }
      if (!/^application\/json\b/i.test(request.headers["content-type"] || "")) return reply(response, 415, { error: "the body is JSON" }), true;
      const chunks = [];
      let size = 0;
      for await (const chunk of request) {
        size += chunk.length;
        if (size > MAX_BODY) return reply(response, 413, { error: "the files are too large" }), true;
        chunks.push(chunk);
      }
      const abort = new AbortController();
      response.on("close", () => {
        if (!response.writableEnded) abort.abort();
      });
      try {
        const body = JSON.parse(Buffer.concat(chunks).toString("utf8"));
        reply(response, 200, await solve({ rnl, prefix, work, request: body, defaults: options, signal: abort.signal }));
      } catch (e) {
        if (!response.writableEnded && !abort.signal.aborted) reply(response, 400, { error: e instanceof Error ? e.message : String(e) });
      }
      return true;
    },
  };
}
