import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { addPreloadHints, moduleGraph, preloadTags, staticImports } from "./preload-hints.mjs";

test("static imports are found in every form, and dynamic ones are not", () => {
  const source = [
    'import { a } from "./a.js";',
    'import b from "../b.js";',
    'import * as c from "./c/c.js";',
    'import {',
    '  d,',
    '  e,',
    '} from "./d.js";',
    'import "./side.js";',
    'export { f } from "./f.js";',
    'export * from "./g.js";',
    'import { fs } from "node:fs";',
    'const later = await import("./later.js");',
    'const text = "import x from \'./not-a-module.js\'";',
  ].join("\n");
  assert.deepEqual(staticImports(source), ["./a.js", "../b.js", "./c/c.js", "./d.js", "./side.js", "./f.js", "./g.js"]);
});

function site() {
  const dir = mkdtempSync(join(tmpdir(), "rnl-hints-"));
  mkdirSync(join(dir, "shells"));
  mkdirSync(join(dir, "core"));
  mkdirSync(join(dir, "locales"));
  const put = (path, text = "") => writeFileSync(join(dir, path), text);
  put("index.html", '<!doctype html><html><head><title>x</title></head><body><script type="module" src="startup.js"></script><script type="module" src="main.js"></script></body></html>\n');
  put("startup.js", 'import { nextPaint } from "./loading.js";\n');
  put("loading.js");
  put("main.js", 'import {\n  one,\n  two,\n} from "./panels.js";\nimport "./menu.js";\nconst shell = await import("./shells/web.js");\n');
  put("panels.js", 'import { s } from "./state.js";\n');
  put("state.js");
  put("menu.js");
  put("shells/web.js", 'import { startCore } from "../core/client.js";\n');
  put("shells/desktop.js", 'import "../not-hinted.js";\n');
  put("core/client.js", 'import { requestText } from "./core.js";\n');
  put("core/core.js");
  put("core/worker.js", 'import { loadCore } from "./core.js";\nimport { files } from "./packages.js";\n');
  put("core/packages.js");
  put("core/rnl_runtime_wasm.wasm");
  for (const name of ["about.json", "commands.json", "public-packages.json"]) put(name, "{}");
  mkdirSync(join(dir, "libraries"));
  put("libraries/bundled.json", "{}");
  for (const name of ["languages", "de", "zh-CN"]) put(`locales/${name}.json`, "{}");
  return dir;
}

test("the module graph is the static imports from the scripts, the web shell and the core's Worker", () => {
  const dir = site();
  try {
    const html = readFileSync(join(dir, "index.html"), "utf8");
    const modules = moduleGraph(dir, ["startup.js", "main.js", "shells/web.js", "core/worker.js", "core/packages.js"]);
    assert.deepEqual([...modules].sort(), [
      "core/client.js", "core/core.js", "core/packages.js", "core/worker.js", "loading.js", "main.js", "menu.js", "panels.js", "shells/web.js", "startup.js", "state.js",
    ]);
    const tags = preloadTags(dir, html);
    assert.ok(!tags.some((t) => t.includes("shells/desktop.js") || t.includes("not-hinted")), "the desktop shell is not the web page's");
    assert.ok(!tags.some((t) => t.includes('href="startup.js"') || t.includes('href="main.js"')), "the scripts the page runs are not preloaded");
    assert.ok(tags.includes('<link rel="modulepreload" href="panels.js">'));
    assert.ok(tags.includes('<link rel="preload" as="fetch" crossorigin href="core/rnl_runtime_wasm.wasm">'));
    for (const file of ["about.json", "commands.json", "libraries/bundled.json", "public-packages.json", "locales/languages.json", "locales/de.json", "locales/zh-CN.json"]) {
      assert.ok(tags.includes(`<link rel="preload" as="fetch" crossorigin href="${file}">`), file);
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("the hints go before </head>, and written again they replace the first", () => {
  const dir = site();
  try {
    const first = addPreloadHints(dir);
    const once = readFileSync(join(dir, "index.html"), "utf8");
    assert.equal(once.split("<!-- preload hints").length, 2);
    assert.ok(once.indexOf("modulepreload") < once.indexOf("</head>"));
    assert.equal(addPreloadHints(dir), first);
    assert.equal(readFileSync(join(dir, "index.html"), "utf8"), once);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("a file the site does not have is not hinted", () => {
  const dir = site();
  try {
    rmSync(join(dir, "libraries"), { recursive: true });
    const tags = preloadTags(dir, readFileSync(join(dir, "index.html"), "utf8"));
    assert.ok(!tags.some((t) => t.includes("libraries/bundled.json")));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
