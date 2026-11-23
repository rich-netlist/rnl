// Draws the five-transistor OTA of the pages with the RNL core, as an RNL viewer draws it from the solved files of
// site/examples: the two looks of the introduction's example (the standard packages, and the same file with the packages
// eda-symbol and eda-style) that the page's style switch shows; the plain look, the netlist with no RNL of its own, as a
// solver lays it out from the connections alone; and the edits look, the file of the language page's fifth example, whose
// editor's record came after the solve. With them go the records of the solver of the four files of the introduction's
// example (the notation, the English sentences, the Chinese ones, and the file with the two packages), as the code
// panel shows them.
//
//   node site/render-examples.mjs [--core DIR] [--packages DIR] [--only LOOK]
//
// --only draws one look (standard or eda) and keeps the other as it is in ota-looks.js, which is for the time the
// packages of one look have no version of the present language yet: that look then stays stale, and site/build.py says so.
//
// --core is the folder with the core as WebAssembly (core.js and rnl_runtime_wasm.wasm; default
// bindings/runtime-wasm/dist, or the folder of the npm package @rich-netlist/runtime), --packages the folder of the
// locked public packages (default .rnl-packages, from `python scripts/packages.py download`). The result is
// site/src/assets/ota-looks.js: the SVG of each look, in which every part that belongs to a device of the netlist
// (its symbol, its name and model) carries data-dev="M1" and every wire data-net="x", so that the page can light up
// what a statement of the file is about. The two looks of the introduction also carry what an RNL viewer marks a
// selection with, as the core gives it: the box of each device (boxes) and the frame of each usage the file names
// (frames), and each is fitted to the frame of the page on its own, both in one shape. Each look is stamped with the name of the file it was drawn from and the start
// of that file's SHA-256: site/build.py checks it, so that an example changed without drawing it again fails the build.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
let coreDir = join(root, "bindings", "runtime-wasm", "dist");
let packages = join(root, ".rnl-packages");
let only = null;
const args = process.argv.slice(2);
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--core" && args[i + 1]) coreDir = resolve(args[++i]);
  else if (args[i] === "--packages" && args[i + 1]) packages = resolve(args[++i]);
  else if (args[i] === "--only" && args[i + 1]) only = args[++i];
  else { console.error("usage: node site/render-examples.mjs [--core DIR] [--packages DIR] [--only LOOK]"); process.exit(2); }
}
const { loadCore } = await import(pathToFileURL(join(coreDir, "core.js")).href);
const core = await loadCore(readFileSync(join(coreDir, "rnl_runtime_wasm.wasm")));
/* the core's version: the package's own, or, for a build of the repository (whose package.json has none), version.js */
let version = JSON.parse(readFileSync(join(coreDir, "package.json"), "utf8")).version;
if (!version) {
  try { version = (await import(pathToFileURL(join(coreDir, "version.js")).href)).coreVersion?.split(" ")[0]; } catch { /* none */ }
}

/** The text of an example with LF line ends, as site/build.py reads it. */
const example = (name) => readFileSync(join(root, "site", "examples", name), "utf8").replace(/\r\n/g, "\n");
const library = only === "standard" ? {} : {
  "/lib/eda-symbol/eda-symbol.rnl": readFileSync(join(packages, "eda-symbol", "0.2", "eda-symbol.rnl"), "utf8"),
  "/lib/eda-style/eda-style.rnl": readFileSync(join(packages, "eda-style", "0.2", "eda-style.rnl"), "utf8"),
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

/* a record of the solver (type="generated"), as a whole block */
const GENERATED = /^\* @RNL\(type="generated"[^\n]*(?:\{\}$|\{\n[\s\S]*?\n\* \}$)/gm;

const looks = {};
const outFile = join(root, "site", "src", "assets", "ota-looks.js");
if (only) {
  const old = JSON.parse(/window\.RNLOtaLooks = (.*);\s*$/s.exec(readFileSync(outFile, "utf8"))[1]);
  for (const [k, v] of Object.entries(old.looks)) if (k !== only) looks[k] = v;
}
const LOOKS = [["standard", "ota5t-review.sp", false], ["eda", "ota5t-review-eda.sp", true], ["plain", "ota5t-plain.sp", false],
  ["edits", "ota5t-edits.sp", false]];
for (const [look, name, withPackages] of LOOKS) {
  if (only && look !== only) continue;
  const text = example(name);
  const doc = await core.build({
    entry: "/top.sp",
    files: withPackages ? { "/top.sp": text, ...library } : { "/top.sp": text },
    package_dirs: withPackages ? ["/lib/eda-symbol", "/lib/eda-style"] : [],
  });
  // example 5 shows two statements in error on purpose, the ALIGN of the missing M9 and the PLACE of "2O": their errors
  // are expected, any other is not
  const lines = text.split(/\r?\n/);
  const meant = (d) => look === "edits" && d.severity === "error" && /#M9|\b2O\b/.test(lines[d.line - 1] || "");
  const bad = doc.diagnostics.filter((d) => (d.severity === "error" || d.severity === "warning") && !meant(d));
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
  // for the two looks of the introduction, what an RNL viewer marks when something is selected (apps/viewer-ui): the
  // box of each device, port and supply as the core gives it (a `box` hit), and the frame of each usage the file itself
  // names (%dp1, not one a package makes), with its title and its roles
  const round = (v) => Math.round(v * 100) / 100;
  const marks = look === "standard" || look === "eda";
  const boxes = {};
  for (const h of marks ? level.hits : []) {
    if (h.kind !== "box") continue;
    const ref = refOf(doc.objects, h.objects || []);
    if (ref) (boxes[ref] ||= []).push(h.points.flat().map(round));
  }
  const frames = {};
  const authored = text.replace(GENERATED, "");
  const named = (id) => new RegExp(`${id.replace(/[^\w%]/g, "\\$&")}(?![\\w])`).test(authored);
  for (const u of marks ? level.usages || [] : []) {
    if (!u.id.startsWith("%") || !u.frame?.bounds || !named(u.id)) continue;
    const roles = u.roles.map((r, i) => ({ name: r.name, at: u.frame.roles?.[i], width: u.frame.role_widths?.[i] }))
      .filter((r) => r.at).map((r) => ({ name: r.name, at: r.at.map(round), width: round(r.width) }));
    frames[u.id] = { title: `${u.kind === "group" ? "@GROUP" : u.definition}: ${u.id}`, bounds: u.frame.bounds.map(round),
      at: u.frame.title.map(round), width: round(u.frame.title_width), roles };
  }
  looks[look] = { file: name, sha256: createHash("sha256").update(text).digest("hex").slice(0, 12), svg: svg.trim(),
    ...(marks ? { boxes, frames } : {}) };
  console.log(`${look}: ${name} sha256:${looks[look].sha256}, ${attrs.size} parts, ${Object.keys(frames).length} frames`);
}

/* The two looks of the introduction are shown in one frame: each is fitted to it as the core frames its drawing, and the
   two are given one shape (the taller of the two, each centred in it), so that the frame keeps its size and the circuit
   sits in the same place, filling it, whichever look is shown */
const pair = [looks.standard, looks.eda].filter(Boolean);
const own = pair.map((l) => /viewBox="([^"]*)"/.exec(l.svg)[1].split(" ").map(Number));
const ratio = Math.max(...own.map((b) => b[3] / b[2]));
pair.forEach((l, i) => {
  const [x, y, w, h] = own[i];
  const W = Math.max(w, h / ratio), H = W * ratio;
  const viewBox = [x - (W - w) / 2, y - (H - h) / 2, W, H].map((v) => Math.round(v * 100) / 100).join(" ");
  l.svg = l.svg.replace(/viewBox="[^"]*"/, `viewBox="${viewBox}"`);
});

/* the records of the solver (type="generated") of the four files of the introduction's example, block by block, as
   the code panel shows them: the solver writes them, so the page takes them from the files rather than repeating them */
const records = {};
for (const name of ["ota5t-review-bare.sp", "ota5t-review.sp", "ota5t-review-zh.sp", "ota5t-review-eda.sp"]) {
  const text = example(name);
  const blocks = [...text.matchAll(GENERATED)].map((m) => m[0].split("\n"));
  records[name] = { sha256: createHash("sha256").update(text).digest("hex").slice(0, 12), blocks };
}

const head = `/* Drawn by the RNL core ${version} from the example files of site/examples with site/render-examples.mjs; do not edit.
   site/build.py checks that each look and each file's records are those of the current text of its file (the sha256 below). */\n`;
writeFileSync(outFile, `${head}window.RNLOtaLooks = ${JSON.stringify({ core: version, looks, records }, null, 1)};\n`);
