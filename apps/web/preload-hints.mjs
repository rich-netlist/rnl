// The preload hints of the web viewer's index.html: written into the page when apps/web/build.mjs builds the
// site, since only the build knows the page's modules, and the files it reads when it starts. A browser finds the
// scripts of a page one level of imports at a time, and the core's WebAssembly only once the shell, the core's client
// and its Worker are there, so the first visit waits for a chain of round trips; with the hints it asks for all of
// them at once. (The desktop viewer reads its page from disk and has no use for them: its index.html has none.)

import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, posix } from "node:path";

const BEGIN = "<!-- preload hints, written by apps/web/preload-hints.mjs -->";
const END = "<!-- end of the preload hints -->";

/** The files the page reads with fetch() when it starts, as the web shell, i18n.js and commands.js name them. */
export const STARTUP_FILES = ["about.json", "commands.json", "libraries/bundled.json", "public-packages.json", "locales/languages.json"];

/** The modules a script imports statically, `import … from`, `export … from` and `import "…"`, by their written paths. */
export function staticImports(source) {
  const found = [];
  const pattern = /(?:^|[;{}\n])\s*(?:import|export)\s*(?:[\w*{][^"';]*?\bfrom\s*)?["']([^"']+)["']/g;
  for (const m of source.matchAll(pattern)) if (/^\.\.?\//.test(m[1])) found.push(m[1]);
  return found;
}

/**
 * The modules reachable from `roots` by static imports, in the order a breadth-first walk finds them, as paths
 * relative to `dir` with `/`. A module that is not a file is left out, as is one that is only imported dynamically.
 */
export function moduleGraph(dir, roots) {
  const seen = new Set(roots);
  const queue = [...roots];
  while (queue.length) {
    const path = queue.shift();
    const file = join(dir, path);
    if (!existsSync(file)) continue;
    for (const written of staticImports(readFileSync(file, "utf8"))) {
      const target = posix.normalize(posix.join(posix.dirname(path), written));
      if (!seen.has(target)) {
        seen.add(target);
        queue.push(target);
      }
    }
  }
  return [...seen].filter((path) => existsSync(join(dir, path)));
}

/** The scripts index.html runs as modules. */
export function entryScripts(html) {
  return [...html.matchAll(/<script[^>]*\btype="module"[^>]*\bsrc="([^"]+)"/g)].map((m) => m[1]);
}

/** The tags: modulepreload for the modules, preload for the core and the files read at start. */
export function preloadTags(dir, html) {
  const entries = entryScripts(html);
  // the shell the page loads when it starts, which imports the core's client; the Worker's own files
  const roots = [...entries, "shells/web.js", "core/worker.js", "core/packages.js"];
  const modules = moduleGraph(dir, roots).filter((path) => !entries.includes(path));
  const tags = modules.map((path) => `<link rel="modulepreload" href="${path}">`);
  // read by fetch(), in a Worker for the core: `crossorigin` is the request's mode, the same origin's credentials
  const fetched = ["core/rnl_runtime_wasm.wasm", ...STARTUP_FILES];
  if (existsSync(join(dir, "locales"))) {
    for (const name of readdirSync(join(dir, "locales")).filter((n) => n.endsWith(".json") && n !== "languages.json").sort()) fetched.push(`locales/${name}`);
  }
  for (const path of fetched) if (existsSync(join(dir, path))) tags.push(`<link rel="preload" as="fetch" crossorigin href="${path}">`);
  return tags;
}

/** Write the hints into dir/index.html, in place of the ones written before; returns how many tags. */
export function addPreloadHints(dir) {
  const file = join(dir, "index.html");
  let html = readFileSync(file, "utf8");
  const block = new RegExp(`${BEGIN}[\\s\\S]*?${END}\\n?`);
  html = html.replace(block, "");
  const tags = preloadTags(dir, html);
  const anchor = "</head>";
  if (!html.includes(anchor)) throw new Error("index.html has no </head> to put the preload hints before");
  html = html.replace(anchor, `${BEGIN}\n${tags.join("\n")}\n${END}\n${anchor}`);
  writeFileSync(file, html);
  return tags.length;
}
