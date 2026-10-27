// Draws the op-amp of the introduction page with the RNL core, as an RNL viewer draws it from the file: the SVGs
// that the page shows beside its own small renderer (the "same file, two looks" of the advantages).
//
//   node site/render-examples.mjs [--core DIR] [--packages DIR]
//
// --core is the folder with the core as WebAssembly (core.js and rnl_runtime_wasm.wasm; default
// bindings/runtime-wasm/dist, or the folder of the npm package @rich-netlist/runtime), --packages the folder of the
// locked public packages (default .rnl-packages, from `python scripts/packages.py download`). Each SVG starts with a
// comment that names the file it was drawn from and the start of that file's SHA-256: site/build.py checks it, so
// that an example changed without drawing it again fails the build.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
let coreDir = join(root, "bindings", "runtime-wasm", "dist");
let packages = join(root, ".rnl-packages");
const args = process.argv.slice(2);
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--core" && args[i + 1]) coreDir = resolve(args[++i]);
  else if (args[i] === "--packages" && args[i + 1]) packages = resolve(args[++i]);
  else { console.error("usage: node site/render-examples.mjs [--core DIR] [--packages DIR]"); process.exit(2); }
}
const { loadCore } = await import(pathToFileURL(join(coreDir, "core.js")).href);
const core = await loadCore(readFileSync(join(coreDir, "rnl_runtime_wasm.wasm")));
const version = JSON.parse(readFileSync(join(coreDir, "package.json"), "utf8")).version;

/** The text of an example with LF line ends, as site/build.py reads it. */
const example = (name) => readFileSync(join(root, "site", "examples", name), "utf8").replace(/\r\n/g, "\n");
const library = {
  "/lib/eda-symbol/eda-symbol.rnl": readFileSync(join(packages, "eda-symbol", "0.1", "eda-symbol.rnl"), "utf8"),
  "/lib/eda-style/eda-style.rnl": readFileSync(join(packages, "eda-style", "0.1", "eda-style.rnl"), "utf8"),
};

for (const [name, out, withPackages] of [["opamp2.sp", "opamp-standard.svg", false], ["opamp2-eda.sp", "opamp-eda.svg", true]]) {
  const text = example(name);
  const doc = await core.build({
    entry: "/top.sp",
    files: withPackages ? { "/top.sp": text, ...library } : { "/top.sp": text },
    package_dirs: withPackages ? ["/lib/eda-symbol", "/lib/eda-style"] : [],
  });
  const bad = doc.diagnostics.filter((d) => d.severity === "error" || d.severity === "warning");
  if (bad.length) throw new Error(`${name}: ${bad.map((d) => `${d.severity} ${d.line}: ${d.message}`).join("; ")}`);
  const level = doc.levels.find((l) => l.kind === "subckt");
  const sha = createHash("sha256").update(text).digest("hex").slice(0, 12);
  const svg = level.svg.replace(/^(<svg[^>]*>)/, `$1\n<!-- drawn by the RNL core ${version} from site/examples/${name} sha256:${sha}, with site/render-examples.mjs -->`);
  writeFileSync(join(root, "site", "src", "assets", out), svg.endsWith("\n") ? svg : svg + "\n");
  console.log(`${out}: ${name} sha256:${sha}`);
}
