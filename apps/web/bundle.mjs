// Bundle the scripts of the web viewer's site (apps/web/build.mjs --bundle). The page is some seventy ES modules, which a
// browser fetches as seventy files and runs as they are, 630 KB of source (220 KB gzip-compressed); bundled and minified
// they are a dozen files of 100 KB gzip-compressed. Only the site is bundled: the tests and the desktop viewer read the
// modules themselves, one file each, and a spec that imports one module must get the module the page runs.
//
// The entry points are the files that something else names by their address: the two scripts of index.html, the web shell
// (which shell.js imports by a name made at run time), the pages of the other modes (embed and the API) and the core's
// Worker (which client.js starts with `new Worker(new URL("./worker.js", import.meta.url))`, so esbuild does not follow
// it) and that client.js itself: an entry that another entry imports is kept as a chunk of its own, and so in core/ beside
// the Worker, where inlined into a script of another directory its address for the Worker would be that directory's.
// What the entries share goes into chunks in core/, too.

import { cpSync, existsSync, mkdtempSync, readdirSync, rmdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative, resolve, sep } from "node:path";

export const ENTRIES = ["startup.js", "main.js", "shells/web.js", "embed.js", "full-viewer-api.js", "core/client.js", "core/worker.js"];

/**
 * Replace the scripts under `dir` that the entry points reach with their bundles; returns { entries, chunks, removed }:
 * the files written, and how many modules they replaced. `esbuild` is the module, passed so that a caller that has
 * not got it can say so.
 */
export async function bundleScripts(dir, esbuild) {
  const root = resolve(dir);
  const staging = mkdtempSync(join(tmpdir(), "rnl-bundle-"));
  try {
    const result = await esbuild.build({
      entryPoints: ENTRIES.filter((path) => existsSync(join(root, path))).map((path) => join(root, path)),
      absWorkingDir: root,
      outdir: staging,
      outbase: root,
      bundle: true,
      splitting: true,
      format: "esm",
      // the browser baseline (decision 0008, README: Browsers); esbuild lowers what these lack and fails on what it cannot
      target: ["chrome109", "safari16", "firefox114"],
      minify: true,
      legalComments: "none",
      chunkNames: "core/chunk-[hash]",
      // the Node branches of core.js and client.js, which a browser never takes
      external: ["node:*"],
      metafile: true,
      logLevel: "warning",
    });
    // the sources that went into the bundles are not carried to the site beside them
    const sources = Object.keys(result.metafile.inputs).map((path) => resolve(root, path)).filter((path) => path.startsWith(root + sep));
    for (const path of sources) rmSync(path, { force: true });
    removeEmptyDirectories(root);
    cpSync(staging, root, { recursive: true });
    const written = Object.keys(result.metafile.outputs).map((path) => relative(staging, resolve(root, path)).split(sep).join("/"));
    return {
      entries: written.filter((path) => !path.includes("/chunk-")).length,
      chunks: written.filter((path) => path.includes("/chunk-")).length,
      removed: sources.length,
    };
  } finally {
    rmSync(staging, { recursive: true, force: true });
  }
}

/** The directories under `dir` with nothing in them, which removing the sources can leave. */
function removeEmptyDirectories(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const path = join(dir, entry.name);
    removeEmptyDirectories(path);
    if (readdirSync(path).length === 0) rmdirSync(path);
  }
}
