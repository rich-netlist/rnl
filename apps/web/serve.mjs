// Serve the web viewer that build.mjs put into apps/web/dist/, on this computer only, to try it and
// for tests:
//
//   node apps/web/serve.mjs [--port N]
//
// It answers GET and HEAD with the site's files, with the MIME types a browser needs, such as
// application/wasm for the core and text/javascript for the module Worker, and tells the browser not
// to cache them, so that a rebuilt site and a changed sample are read anew. --port 0 takes a free
// port; 8000 by default. It prints the page's address.
//
// With --rnl PATH (or RNL_SOLVER) it also offers the page a Solve that runs the `rnl` command at PATH,
// such as target/debug/rnl, on every press (solver-service.mjs): build `rnl` again after a change to the
// solver, press Solve, and the new result is shown, without building the viewer or restarting this.
// --solver-options S gives every solve those settings, as `rnl solve --solver-options` reads them,
// such as time_limit=none. Only the page of this server may use it.

import { createServer } from "node:http";
import { existsSync, readFile, stat } from "node:fs";
import { dirname, extname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createService, runVersion } from "./solver-service.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const site = join(here, "dist");
const usage = "usage: node apps/web/serve.mjs [--port N] [--rnl PATH] [--solver-options S]";

let port = 8000;
let rnl = process.env.RNL_SOLVER || null;
let solverOptions = "";
const args = process.argv.slice(2);
for (let k = 0; k < args.length; k++) {
  if (args[k] === "--port" && k + 1 < args.length && /^\d+$/.test(args[k + 1])) port = Number(args[++k]);
  else if (args[k] === "--rnl" && k + 1 < args.length) rnl = resolve(args[++k]);
  else if (args[k] === "--solver-options" && k + 1 < args.length) solverOptions = args[++k];
  else {
    console.error(usage);
    process.exit(2);
  }
}
if (!existsSync(join(site, "index.html"))) {
  console.error(`${site} has no site: build it with node apps/web/build.mjs`);
  process.exit(1);
}

/** The MIME types of the site's files by extension; any other file is sent as bytes. */
const types = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".json": "application/json",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".wasm": "application/wasm",
  ".woff2": "font/woff2",
  ".png": "image/png",
  ".ico": "image/x-icon",
  ".txt": "text/plain; charset=utf-8",
};

/** The file a request's path names under the site, or null when it names none there. */
function fileOf(pathname) {
  let path;
  try {
    path = decodeURIComponent(pathname);
  } catch {
    return null;
  }
  if (path.includes("\0")) return null;
  const file = resolve(site, `.${path}`);
  const inside = relative(site, file);
  return inside.startsWith("..") || isAbsolute(inside) ? null : file;
}

let service = null;
if (rnl) {
  try {
    service = createService({ rnl, version: await runVersion(rnl), options: solverOptions });
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
}

function send(response, status, headers, body) {
  response.writeHead(status, { "Cache-Control": "no-store", "X-Content-Type-Options": "nosniff", ...headers });
  response.end(body);
}

const server = createServer(async (request, response) => {
  const { pathname } = new URL(request.url, "http://localhost");
  if (service && (await service.handle(request, response, pathname, server.address().port))) return;
  if (!service && pathname === "/rnl-solver") {
    // the page asks on this computer's address whether a solver is here: no, and not a 404 in its console
    send(response, 200, { "Content-Type": "application/json" }, JSON.stringify({ service: "rnl-solver", version: null }));
    return;
  }
  if (request.method !== "GET" && request.method !== "HEAD") {
    send(response, 405, { Allow: "GET, HEAD", "Content-Type": "text/plain; charset=utf-8" }, "Method not allowed\n");
    return;
  }
  const named = fileOf(pathname);
  const notFound = () => send(response, 404, { "Content-Type": "text/plain; charset=utf-8" }, "Not found\n");
  if (!named) {
    notFound();
    return;
  }
  stat(named, (error, stats) => {
    if (error) {
      notFound();
      return;
    }
    const file = stats.isDirectory() ? join(named, "index.html") : named;
    readFile(file, (e, body) => {
      if (e) {
        notFound();
        return;
      }
      const headers = { "Content-Type": types[extname(file).toLowerCase()] || "application/octet-stream" };
      headers["Content-Length"] = body.length;
      send(response, 200, headers, request.method === "HEAD" ? undefined : body);
    });
  });
});

server.listen(port, "127.0.0.1", () => {
  console.log(`The web viewer: http://127.0.0.1:${server.address().port}/`);
  if (service) console.log(`Solve runs ${service.info.version} at ${rnl}${solverOptions ? ` with --solver-options "${solverOptions}"` : ""}`);
  console.log("Press Ctrl+C to stop.");
});
