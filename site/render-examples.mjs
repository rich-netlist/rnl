// Draws the five-transistor OTA of the introduction page with the RNL core, as an RNL viewer draws it from the file: the
// two looks of the example (the standard packages, and the same file with the packages eda-symbol and eda-style) that
// the page's style switch shows.
//
//   node site/render-examples.mjs [--core DIR] [--packages DIR]
//
// --core is the folder with the core as WebAssembly (core.js and rnl_runtime_wasm.wasm; default
// bindings/runtime-wasm/dist, or the folder of the npm package @rich-netlist/runtime), --packages the folder of the
// locked public packages (default .rnl-packages, from `python scripts/packages.py download`). The result is
// site/src/assets/ota-looks.js: the SVG of each look, in which every part that belongs to a device of the netlist
// (its symbol, its name and model) carries data-dev="M1" and every wire data-net="x", so that the page can light up
// what a statement of the file is about. Each look is stamped with the name of the file it was drawn from and the start
// of that file's SHA-256: site/build.py checks it, so that an example changed without drawing it again fails the build.
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

/** The name a part of the drawing answers to on the page: the device, or a display object (%pi → PI) */
function refOf(objects, ids) {
  for (const i of ids) if (objects[i] && objects[i].kind === "instance") return objects[i].name;
  for (const i of ids) if (objects[i] && objects[i].kind === "usage" && objects[i].name.startsWith("%")) return objects[i].name.slice(1).toUpperCase();
  /* a supply or ground symbol of the standard package answers to its net: VDD, VSS */
  const net = ids.map((i) => objects[i]).find((o) => o && o.kind === "net");
  const anon = ids.some((i) => objects[i] && objects[i].kind === "usage" && objects[i].name.startsWith("@rnl-std"));
  return net && anon ? net.name.toUpperCase() : null;
}

const looks = {};
for (const [look, name, withPackages] of [["standard", "ota5t-review.sp", false], ["eda", "ota5t-review-eda.sp", true]]) {
  const text = example(name);
  const doc = await core.build({
    entry: "/top.sp",
    files: withPackages ? { "/top.sp": text, ...library } : { "/top.sp": text },
    package_dirs: withPackages ? ["/lib/eda-symbol", "/lib/eda-style"] : [],
  });
  const bad = doc.diagnostics.filter((d) => d.severity === "error" || d.severity === "warning");
  if (bad.length) throw new Error(`${name}: ${bad.map((d) => `${d.severity} ${d.line}: ${d.message}`).join("; ")}`);
  const level = doc.levels.find((l) => l.kind === "subckt");
  const attrs = new Map();
  for (const h of level.hits) {
    if (!h.id || attrs.has(h.id)) continue;
    if (h.kind === "wire") {
      const net = (h.objects || []).map((i) => doc.objects[i]).find((o) => o && o.kind === "net");
      if (net) attrs.set(h.id, ` data-net="${net.name}"`);
    } else {
      const dev = refOf(doc.objects, h.objects || []);
      if (dev) attrs.set(h.id, ` data-dev="${dev}"`);
    }
  }
  let svg = level.svg.replace(/<g data-id="([^"]*)">/g, (m, id) => (attrs.has(id) ? `<g data-id="${id}"${attrs.get(id)}>` : m));
  svg = svg.replace(/^(<svg[^>]*?) width="[^"]*" height="[^"]*"/, "$1").replace(/<rect class="backdrop"[^>]*\/>\n?/, "");
  looks[look] = { file: name, sha256: createHash("sha256").update(text).digest("hex").slice(0, 12), svg: svg.trim() };
  console.log(`${look}: ${name} sha256:${looks[look].sha256}, ${attrs.size} parts`);
}

/* the same view box for both looks, so that the placements stay where they are when the look changes */
const boxes = Object.values(looks).map((l) => /viewBox="([^"]*)"/.exec(l.svg)[1].split(" ").map(Number));
const x0 = Math.min(...boxes.map((b) => b[0])), y0 = Math.min(...boxes.map((b) => b[1]));
const x1 = Math.max(...boxes.map((b) => b[0] + b[2])), y1 = Math.max(...boxes.map((b) => b[1] + b[3]));
const viewBox = [x0, y0, x1 - x0, y1 - y0].map((v) => Math.round(v * 100) / 100).join(" ");
for (const l of Object.values(looks)) l.svg = l.svg.replace(/viewBox="[^"]*"/, `viewBox="${viewBox}"`);

const head = `/* Drawn by the RNL core ${version} from the example files of site/examples with site/render-examples.mjs; do not edit.
   site/build.py checks that each look is drawn from the current text of its file (the sha256 below). */\n`;
writeFileSync(join(root, "site", "src", "assets", "ota-looks.js"), `${head}window.RNLOtaLooks = ${JSON.stringify({ core: version, looks }, null, 1)};\n`);
