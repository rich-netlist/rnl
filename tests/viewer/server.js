// The test server: the page of apps/viewer-ui as it is, with the fake shell of this directory as
// shells/fake.js, the fixtures under fixtures/ and the interface's languages under locales/, from
// i18n/. Node only, with no dependencies.
//
//   node server.js [PORT]
//
// builds the fixtures and serves them until stopped, to look at the page with the fake shell, such
// as http://127.0.0.1:PORT/?shell=fake&open=04-hierarchy.

import fs from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { buildRnl, root, writeFixtures } from "./fixtures.js";
import { uiCatalog, uiLanguages } from "../../scripts/i18n/viewer-locales.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const page = path.join(root, "apps", "viewer-ui");

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
};

/** The file under `dir` that a URL path names, or null when it would be outside it. */
function inside(dir, rel) {
  const file = path.resolve(dir, rel);
  return file.startsWith(dir + path.sep) ? file : null;
}

/** The file a URL path names: the fake shell, a fixture, or a file of the page. */
function fileFor(pathname, fixtures) {
  if (pathname === "/shells/fake.js") return path.join(here, "fake-shell.js");
  if (pathname.startsWith("/fixtures/")) return inside(fixtures, pathname.slice("/fixtures/".length));
  return inside(page, pathname === "/" ? "index.html" : pathname.slice(1));
}

/**
 * Serve the page and the fixtures in `fixtures` on 127.0.0.1, on `port` or any free one; resolves
 * to the server's URL and a function that stops it.
 */
export function serve(fixtures, port = 0) {
  const server = http.createServer((req, res) => {
    let pathname;
    try {
      pathname = decodeURIComponent(new URL(req.url, "http://localhost").pathname);
    } catch {
      pathname = "";
    }
    // the languages of the interface and their catalogs, from i18n/, as the web viewer's build writes them
    const locale = /^\/locales\/(.+)\.json$/.exec(pathname);
    if (locale && req.method === "GET") {
      const body = locale[1] === "languages" ? uiLanguages(root) : uiCatalog(root, locale[1]);
      res.writeHead(body ? 200 : 404, { "content-type": TYPES[".json"], "cache-control": "no-store" });
      res.end(body ? JSON.stringify(body) : "not found");
      return;
    }
    const file = fileFor(pathname, fixtures);
    fs.readFile(file || "", (err, body) => {
      if (err || req.method !== "GET") {
        res.writeHead(err ? 404 : 405, { "content-type": "text/plain" });
        res.end(err ? "not found" : "only GET");
        return;
      }
      res.writeHead(200, {
        "content-type": TYPES[path.extname(file)] || "application/octet-stream",
        "cache-control": "no-store",
      });
      res.end(body);
    });
  });
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, "127.0.0.1", () => {
      resolve({
        url: `http://127.0.0.1:${server.address().port}`,
        close: () => {
          server.closeAllConnections();
          return new Promise((done) => server.close(done));
        },
      });
    });
  });
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "rnl-viewer-"));
  writeFixtures(buildRnl(), dir);
  const { url } = await serve(dir, Number(process.argv[2] || 0));
  console.log(`serving the page with the fake shell at ${url}/?shell=fake&open=04-hierarchy`);
  console.log(`fixtures: ${fs.readdirSync(dir).filter((f) => f.endsWith(".json")).map((f) => f.slice(0, -5)).join(", ")}`);
}
