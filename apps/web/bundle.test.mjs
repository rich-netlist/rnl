// node --test apps/web/bundle.test.mjs   (needs esbuild: npm ci --prefix apps/web)
import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { bundleScripts } from "./bundle.mjs";
import { preloadTags } from "./preload-hints.mjs";

const esbuild = await import("esbuild").catch(() => null);
const skip = esbuild ? false : "esbuild is not installed: npm ci --prefix apps/web";

function site() {
  const dir = mkdtempSync(join(tmpdir(), "rnl-bundle-test-"));
  for (const d of ["shells", "core"]) mkdirSync(join(dir, d));
  const put = (path, text) => writeFileSync(join(dir, path), text);
  put("index.html", '<!doctype html><html><head></head><body><script type="module" src="startup.js"></script><script type="module" src="main.js"></script></body></html>\n');
  put("startup.js", 'import { ready } from "./loading.js";\nready();\n');
  put("loading.js", "export const ready = () => globalThis.ready = true;\n");
  put("main.js", 'import { panel } from "./panels.js";\nimport { ready } from "./loading.js";\nready();\npanel();\nconst shell = await import("./shells/web.js");\nglobalThis.shell = shell.default;\n');
  put("panels.js", "export const panel = () => 1;\n");
  put("shells/web.js", 'import { start } from "../core/client.js";\nimport { ready } from "../loading.js";\nexport default { start, ready };\n');
  put("shells/desktop.js", 'import "../not-in-the-site.js";\n');
  put("embed.js", 'import { panel } from "./panels.js";\nexport const startEmbed = () => panel();\n');
  put("full-viewer-api.js", 'import { panel } from "./panels.js";\nexport const startFullViewerApi = () => panel();\n');
  put("core/client.js", 'export const start = () => new Worker(new URL("./worker.js", import.meta.url), { type: "module" });\n');
  put("core/worker.js", 'import { load } from "./core.js";\nload(new URL("./rnl_runtime_wasm.wasm", import.meta.url));\n');
  put("core/core.js", 'export const load = async (url) => (await import("node:fs/promises")).readFile(url);\n');
  put("core/rnl_runtime_wasm.wasm", "");
  return dir;
}

test("the scripts the entry points reach are replaced by bundles, with their chunks in core/", { skip }, async () => {
  const dir = site();
  try {
    const result = await bundleScripts(dir, esbuild);
    assert.ok(result.chunks >= 1, "what the entries share is a chunk");
    assert.equal(result.entries, 7);
    for (const gone of ["panels.js", "loading.js", "core/core.js"]) assert.ok(!existsSync(join(dir, gone)), `${gone} is bundled`);
    for (const kept of ["startup.js", "main.js", "shells/web.js", "embed.js", "full-viewer-api.js", "core/client.js", "core/worker.js", "core/rnl_runtime_wasm.wasm"]) assert.ok(existsSync(join(dir, kept)), kept);
    // the desktop shell is not an entry, and is not in a site
    assert.ok(existsSync(join(dir, "shells/desktop.js")), "a script no entry reaches is left as it is");
    // the Worker reads the core by the address it always had, beside it
    assert.match(readFileSync(join(dir, "core/worker.js"), "utf8"), /new URL\("\.\/rnl_runtime_wasm\.wasm",import\.meta\.url\)/);
    // the shell stays where main.js asks for it, and the core's client starts the Worker from core/, beside it
    assert.match(readFileSync(join(dir, "main.js"), "utf8"), /import\("\.\/shells\/web\.js"\)/);
    const inCore = readdirSync(join(dir, "core")).filter((n) => n.startsWith("chunk-")).map((n) => readFileSync(join(dir, "core", n), "utf8"));
    assert.ok(inCore.some((text) => /new URL\("\.\/worker\.js",import\.meta\.url\)/.test(text)), "the client is a chunk of core/");
    assert.doesNotMatch(readFileSync(join(dir, "shells/web.js"), "utf8"), /worker\.js/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("the preload hints follow the bundles' chunks, which are files", { skip }, async () => {
  const dir = site();
  try {
    await bundleScripts(dir, esbuild);
    const tags = preloadTags(dir, readFileSync(join(dir, "index.html"), "utf8"));
    const modules = tags.filter((t) => t.includes("modulepreload")).map((t) => /href="([^"]+)"/.exec(t)[1]);
    assert.ok(modules.includes("shells/web.js"));
    assert.ok(modules.includes("core/worker.js"));
    assert.ok(modules.some((m) => m.startsWith("core/chunk-")), "the chunks");
    assert.ok(!modules.includes("main.js") && !modules.includes("startup.js"), "the scripts the page runs are not hinted");
    for (const m of modules) assert.ok(existsSync(join(dir, m)), m);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
