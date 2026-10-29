// Serve the web viewer that build.mjs put into apps/web/dist/, on this computer only, to try it and
// for tests:
//
//   node apps/web/serve.mjs [--port N] [--dir DIR]
//
// It answers GET and HEAD with the site's files, with the MIME types a browser needs, such as
// application/wasm for the core and text/javascript for the module Worker, and tells the browser not
// to cache them, so that a rebuilt site and a changed sample are read anew. --port 0 takes a free
// port; 8000 by default. --dir DIR serves the site that build.mjs --out DIR made. It prints the page's address.

import { createServer } from "node:http";
import { existsSync, readFile, stat } from "node:fs";
import { dirname, extname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
let site = join(here, "dist");
const usage = "usage: node apps/web/serve.mjs [--port N] [--dir DIR]";

let port = 8000;
const args = process.argv.slice(2);
for (let k = 0; k < args.length; k++) {
  if (args[k] === "--port" && k + 1 < args.length && /^\d+$/.test(args[k + 1])) port = Number(args[++k]);
  else if (args[k] === "--dir" && k + 1 < args.length) site = resolve(args[++k]);
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

function send(response, status, headers, body) {
  response.writeHead(status, { "Cache-Control": "no-store", "X-Content-Type-Options": "nosniff", ...headers });
  response.end(body);
}

const server = createServer((request, response) => {
  if (request.method !== "GET" && request.method !== "HEAD") {
    send(response, 405, { Allow: "GET, HEAD", "Content-Type": "text/plain; charset=utf-8" }, "Method not allowed\n");
    return;
  }
  const { pathname } = new URL(request.url, "http://localhost");
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
  console.log("Press Ctrl+C to stop.");
});
