// What the RNL core draws of the brand's logo with its text, for site/render-logo.py: prints JSON with two SVGs, the one the
// viewer's easter egg shows (apps/web/build.mjs captions brand/rnl-logo.sp with "RNL" and "logo by RNL") and the same
// without "logo by RNL", which is what the homepage shows.
//
//   node site/render-logo-core.mjs --core bindings/runtime-wasm/dist
//
// The caption is read from apps/web/build.mjs, so that the two cannot differ.
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const arg = process.argv.indexOf("--core");
const coreDir = resolve(arg > 0 ? process.argv[arg + 1] : join(root, "bindings", "runtime-wasm", "dist"));
const { loadCore } = await import(pathToFileURL(join(coreDir, "core.js")).href);
const core = await loadCore(readFileSync(join(coreDir, "rnl_runtime_wasm.wasm")));

const logo = readFileSync(join(root, "brand", "rnl-logo.sp"), "utf8");
const build = readFileSync(join(root, "apps", "web", "build.mjs"), "utf8");
const literal = build.match(/const caption = ('(?:[^'\\]|\\.)*');/);
if (!literal) throw new Error("apps/web/build.mjs: the caption of the interactive logo is not where site/render-logo-core.mjs looks for it");
const caption = new Function(`return ${literal[1]}`)();
const lastLine = /;\n\*   \(28, -4\.5\) @TEXT\("logo by RNL"[^\n]*/;
if (!lastLine.test(caption)) throw new Error('the caption has no "logo by RNL" statement of the form this script removes');
const plain = caption.replace(lastLine, "");

async function render(withCaption) {
  const text = logo.replace(/^\.end[ \t]*\r?$/im, withCaption);
  const doc = await core.build({ entry: "/top.sp", files: { "/top.sp": text } });
  const bad = doc.diagnostics.filter((d) => d.severity === "error" || d.severity === "warning");
  if (bad.length) throw new Error(bad.map((d) => `${d.severity} ${d.line}: ${d.message}`).join("; "));
  return doc.levels[0].svg;
}

process.stdout.write(JSON.stringify({ egg: await render(caption), homepage: await render(plain) }));
