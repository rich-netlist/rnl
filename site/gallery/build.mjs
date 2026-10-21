// Build the data of the gallery of the standard packages (site/src/packages.html): every package
// file under libraries/, read with package-file.mjs, and every symbol drawn by the core as
// WebAssembly, the same core the web viewer runs.
//
//   node site/gallery/build.mjs [--core DIR] [--out DIR] [--libraries DIR]
//
// --core is the built binding, bindings/runtime-wasm/dist/ by default (node bindings/runtime-wasm/
// build.mjs, or node apps/web/build.mjs, writes it); --out is the built site, site/dist/ by default;
// --libraries is the directory of the package files, libraries/ by default.
// It writes into OUT/gallery/: index.js, the list of the groups of packages, and one file for each
// group, such as standard-0.2.js. A group is a directory of package files: libraries/standard/0.2/
// is the standard series 0.2. They are scripts rather than JSON so that the page also works opened
// from the disk; the pages of both languages load the same files.
//
// Each symbol is drawn on its own: the build writes a netlist with a subcircuit for each symbol of a
// package, in which one instance is drawn with the symbol at the origin, and the core draws every
// subcircuit as a level of its own. The labels the symbol writes with ${…} show their keys instead
// of an instance's attributes. The pins and the box come from the core as well; the parser adds
// what only the text says, such as the labels of the pins.

import { mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { readPackage, resolve, terminalNames } from "./package-file.mjs";
import { publicGalleryPackages } from "./public-packages.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const usage = "usage: node site/gallery/build.mjs [--core DIR] [--out DIR] [--libraries DIR]";

let coreDir = join(root, "bindings", "runtime-wasm", "dist");
let out = join(root, "site", "dist");
let libraries = join(root, "libraries");
const args = process.argv.slice(2);
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--core" && args[i + 1]) coreDir = args[++i];
  else if (args[i] === "--out" && args[i + 1]) out = args[++i];
  else if (args[i] === "--libraries" && args[i + 1]) libraries = args[++i];
  else {
    console.error(usage);
    process.exit(2);
  }
}

const { loadCore } = await import(pathToFileURL(join(coreDir, "core.js")).href);
const core = await loadCore(readFileSync(join(coreDir, "rnl_runtime_wasm.wasm")));

// ---------------------------------------------------------------- the package files

/** Every directory under libraries/ that holds package files, with its files. */
function groupDirs(dir, found = []) {
  const entries = readdirSync(dir).sort();
  const files = entries.filter((e) => e.endsWith(".rnl") && statSync(join(dir, e)).isFile());
  if (files.length) found.push({ dir, files });
  for (const e of entries) if (statSync(join(dir, e)).isDirectory()) groupDirs(join(dir, e), found);
  return found;
}

/** Version strings compared by their numbers: "0.10" after "0.9". */
function compareVersions(a, b) {
  const pa = String(a).split(".").map(Number);
  const pb = String(b).split(".").map(Number);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] || 0) - (pb[i] || 0);
    if (d) return d;
  }
  return 0;
}

const posix = (p) => p.split(sep).join("/");

const groups = groupDirs(libraries).map(({ dir, files }) => {
  const rel = posix(join("libraries", relative(libraries, dir)));
  const m = /^libraries\/standard\/([^/]+)$/.exec(rel);
  const packages = files
    .map((file) => ({ file, path: `${rel}/${file}`, ...readPackage(readFileSync(join(dir, file), "utf8")) }))
    .filter((p) => {
      if (!p.header.name) console.warn(`${p.path}: no PACKAGE in its header record, so it is not a package; left out`);
      return !!p.header.name;
    });
  return {
    id: rel.replace(/^libraries\//, "").replace(/[^\w.-]+/g, "-"),
    dir: rel,
    kind: m ? "standard" : rel.startsWith("libraries/pdk/") ? "pdk" : "extension",
    series: m ? m[1] : null,
    packages,
  };
});
const publicCatalog = args.includes("--libraries") ? { packages: [], provenance: null } : await publicGalleryPackages();
for (const pkg of publicCatalog.packages) {
  const rel = `libraries/${pkg.path.slice(0, pkg.path.lastIndexOf("/"))}`;
  groups.push({ id: rel.replace(/^libraries\//, "").replace(/[^\w.-]+/g, "-"), dir: rel,
    kind: pkg.kind, series: null,
    packages: [{ file: `${pkg.name}.rnl`, path: `libraries/${pkg.path}`, source: pkg.source,
      summary: pkg.summary, ...readPackage(pkg.text) }],
  });
}

groups.sort((a, b) => {
  if (a.series && b.series) return compareVersions(b.series, a.series);
  if (a.series || b.series) return a.series ? -1 : 1;
  return a.dir < b.dir ? -1 : 1;
});

// The workspace every request gets: all the package files, each directory a package directory, so
// that the gallery draws what libraries/ holds even when the core bundles another state of it.
const workspace = {};
const packageDirs = [];
for (const g of groups) {
  packageDirs.push(`/${g.dir}`);
  for (const p of g.packages) workspace[`/${p.path}`] = p.text;
}

// ---------------------------------------------------------------- what the packages reference

/** The packages of a group by name, for the references, which name a version as well. */
function byName(group) {
  return new Map(group.packages.filter((p) => p.header.name).map((p) => [p.header.name, p]));
}

/** The order in which using `name` brings in packages: what it references first, each once (16.2). */
function expansion(group, name, seen = new Set(), order = []) {
  if (seen.has(name)) return order;
  seen.add(name);
  const p = byName(group).get(name);
  for (const r of p?.header.references || []) expansion(group, r.name, seen, order);
  order.push(name);
  return order;
}

// ---------------------------------------------------------------- drawing the symbols

const SCALE_UNIT = 13; // display pixels per grid step on the cards: every symbol at the same scale
const MARGIN = 0.6; // grid steps around what a symbol draws

/** The direction of a pin (0 right, 90 up, …; null for none) as a unit vector in scene units, y up. */
const unit = (dir) =>
  dir === null || dir === undefined ? [0, 0] : [Math.round(Math.cos((dir * Math.PI) / 180)), Math.round(Math.sin((dir * Math.PI) / 180))];

const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const num = (x) => (Math.round(x * 100) / 100).toString();
// All overlays use the core's scene-to-SVG transform (scene y points up).
const svgPoint = ([x, y], scale) => [Number(num(x * scale)), Number(num(-y * scale))];

/** Primitive boxes exclude outlines. Include their stroke envelope, including SVG's miter limit. */
function paintedBounds(primitive, scale) {
  const outline = Math.max(0, ...(primitive.strokes || []).map((s) =>
    s.width / (s.unit === "pixel" ? scale : 1) * (primitive.kind === "path" && s.join === "miter" ? 2 : 0.5)));
  const [x0, y0, x1, y1] = primitive.bbox;
  return [x0 - outline, y0 - outline, x1 + outline, y1 + outline];
}

/** Round outwards, so serializing a viewBox cannot trim the last fraction of a stroke. */
function viewBox([x0, y0, x1, y1], scale, margin) {
  const low = (x) => Math.floor(x * 100) / 100;
  const high = (x) => Math.ceil(x * 100) / 100;
  const left = low((x0 - margin) * scale), top = low(-(y1 + margin) * scale);
  return [left, top, high((x1 + margin) * scale) - left, high(-(y0 - margin) * scale) - top];
}

/** The netlist that draws each symbol of `symbols` in a subcircuit of its own, as `X1`. */
function galleryNetlist(pkg, symbols) {
  const lines = [
    `* The gallery of ${pkg.header.name} ${pkg.header.version}: each symbol drawn once, by site/gallery/build.mjs`,
    `* @RNL(type="header") { @VERSION("${pkg.header.language}"); @USE-PACKAGE(name="${pkg.header.name}", version="${pkg.header.version}") }`,
  ];
  symbols.forEach((s, k) => {
    const terminals = [];
    for (const pin of s.allPins) for (const t of [].concat(pin.terminal ?? [])) if (!terminals.includes(t)) terminals.push(t);
    const named = terminals.some((t) => typeof t === "string");
    const count = named ? terminals.length : Math.max(1, ...terminals.filter((t) => typeof t === "number"));
    const nets = Array.from({ length: count }, (_, i) => `t${i + 1}`);
    const master = `cell${k + 1}`;
    if (named) lines.push(`.subckt ${master} ${terminals.join(" ")}`, ".ends");
    lines.push(`.subckt g${k + 1}`, `X1 ${nets.join(" ")} ${master}`);
    const statements = [`#X1 @SET-SYMBOL ${s.name}`, "#X1 @PLACE(0, 0)"];
    for (const l of s.allLabels) if (l.text.includes("${")) statements.push(`#X1 @TEXT("${l.key}", key="${l.key}")`);
    lines.push(`* @RNL { ${statements.join("; ")} }`, ".ends");
  });
  return `${lines.join("\n")}\n`;
}

/** Draw the symbols of a package; returns, for each, its card's SVG, pins, box and problems. */
function draw(pkg, symbols) {
  const entry = `/gallery/${pkg.header.name}.sp`;
  const netlist = galleryNetlist(pkg, symbols);
  const snapshot = core.snapshot({
    entry,
    files: { ...workspace, [entry]: netlist },
    package_dirs: packageDirs,
    dialect: "spice",
    primitives: true,
    parts: { drawing: true, model: true },
  });
  const doc = snapshot.drawing;
  if (doc.failed) throw new Error(`${pkg.path}: the core could not read the gallery: ${doc.failed}`);
  pkg.problems = doc.diagnostics
    .filter((d) => d.severity !== "hint" && d.full_path === `/${pkg.path}`)
    .map((d) => ({ line: d.line, text: `${d.severity}: ${d.message}` }));
  return symbols.map((s, k) => {
    const title = `g${k + 1}`;
    const level = doc.levels.find((l) => l.title === title);
    if (!level) throw new Error(`${pkg.path}: the core drew no level for ${s.name}`);
    const model = snapshot.levels.find((l) => l.id === `subckt:${title}`)?.model;
    const object = model?.objects.find((o) => o.id === "#X1");
    const [first, last] = level.lines ? [level.lines[1], level.lines[2]] : [0, 0];
    const problems = doc.diagnostics
      .filter((d) => d.severity !== "hint" && d.file_index === 0 && d.line >= first && d.line <= last)
      .map((d) => `${d.severity}: ${d.message}`);
    const scale = level.scale;
    const boxHit = level.hits.find((h) => h.kind === "box");
    const box = boxHit
      ? [boxHit.points[0][0] / scale, -boxHit.points[1][1] / scale, boxHit.points[1][0] / scale, -boxHit.points[0][1] / scale]
      : null;
    const names = terminalNames(s.docs);
    const pins = (object?.pins || []).map((p, i) => {
      const written = s.allPins.find((w) => Math.abs(w.at[0] - p.at[0]) < 1e-6 && Math.abs(w.at[1] - p.at[1]) < 1e-6) || s.allPins[i] || {};
      const terminal = written.terminal ?? p.terminals.map((t) => t.replace(/^#X1:/, "")).join(",");
      return {
        terminal,
        name: names && typeof terminal === "number" ? names[terminal - 1] ?? null : null,
        at: p.at,
        dir: p.dir,
        direction: written.direction ?? null,
        label: written.label ?? null,
      };
    });
    if (pins.length !== s.allPins.length) {
      problems.push(`note: the core gives ${pins.length} pins, the text ${s.allPins.length}`);
    }
    // what is drawn: the primitives, the box and the pins, in scene units
    let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
    const grow = (a, b, c, d) => {
      x0 = Math.min(x0, a);
      y0 = Math.min(y0, b);
      x1 = Math.max(x1, c);
      y1 = Math.max(y1, d);
    };
    for (const prim of level.primitives || []) if (!prim.owner.startsWith("#X1::")) grow(...paintedBounds(prim, scale));
    if (box) grow(...box);
    // Include the registration point even when it lies outside the symbol's drawing.
    grow(0, 0, 0, 0);
    const origin = svgPoint([0, 0], scale);
    // a thumbnail shows the symbol and its origin, without labels, pins and their tags
    const thumb = Number.isFinite(x0)
      ? viewBox([x0, y0, x1, y1], scale, MARGIN).map(num).join(" ")
      : null;
    for (const prim of level.primitives || []) grow(...paintedBounds(prim, scale));
    // The selectable box is also visible when the Box toggle is on (1.2px outline).
    if (box) grow(box[0] - 0.6 / scale, box[1] - 0.6 / scale, box[2] + 0.6 / scale, box[3] + 0.6 / scale);
    // a pin's mark, and its terminal number and direction beyond its end (see card)
    for (const p of pins) {
      grow(p.at[0] - 0.3, p.at[1] - 0.3, p.at[0] + 0.3, p.at[1] + 0.3);
      grow(...pinTag(p, scale).box);
    }
    if (!Number.isFinite(x0)) [x0, y0, x1, y1] = [-1, -1, 1, 1];
    x0 -= MARGIN;
    y0 -= MARGIN;
    x1 += MARGIN;
    y1 += MARGIN;
    const svg = card(level.svg, scale, [x0, y0, x1, y1], box, pins);
    return { svg, thumb, origin, width: num((x1 - x0) * SCALE_UNIT), height: num((y1 - y0) * SCALE_UNIT), box, pins, problems };
  });
}

/** The direction a pin points, away from the symbol, where its wire leaves (9.2.4). */
const ARROWS = { 0: "→", 90: "↑", 180: "←", 270: "↓" };
const TAG_SIZE = 11; // the size of a pin's tag in the core's pixels, as the page's stylesheet sets it

/**
 * Where a pin's tag goes: its terminal number with the arrow of its direction after it, such as
 * `1↑`, beyond the end of the pin: beside a pin that points left or right, above or below one that
 * points up or down. Returns the anchor of the text in the core's pixels (y down), and the box it
 * takes in scene units (y up), from an estimate of its width: a monospaced figure is 0.6 em.
 */
function pinTag(p, scale) {
  const text = [].concat(p.terminal).join(",");
  const [dx, dy] = unit(p.dir);
  const [x, y] = svgPoint(p.at, scale);
  // Leave an em for the direction glyph, whose fallback font can be wider than a mono figure.
  const width = (text.length * 0.7 + (ARROWS[p.dir] ? 1 : 0)) * TAG_SIZE;
  const anchor = dx > 0.5 ? "start" : dx < -0.5 ? "end" : p.dir === null ? "start" : "middle";
  const tx = x + dx * 5 + (p.dir === null ? 5 : 0);
  const ty = y - dy * 6 + (Math.abs(dy) > 0.5 ? (dy < 0 ? 8 : 0) : 3.8);
  // the box in pixels, from the baseline up by the figures' height and down by a little
  const left = anchor === "start" ? tx : anchor === "end" ? tx - width : tx - width / 2;
  const [top, bottom] = [ty - TAG_SIZE, ty + 0.3 * TAG_SIZE];
  return { text, anchor, x: tx, y: ty, box: [left / scale, -bottom / scale, (left + width) / scale, -top / scale] };
}

/**
 * The core's SVG of a level as a card shows it: framed on what is drawn, without its backdrop, the
 * labels marked, and the box and the pins drawn over it, which the page shows or hides. Colours stay
 * as the core writes them; the page's stylesheet maps them to the theme.
 */
function card(svg, scale, [x0, y0, x1, y1], box, pins) {
  const body = svg
    .replace(/^<svg[^>]*>\n?/, "")
    .replace(/<\/svg>\s*$/, "")
    .replace(/<rect class="backdrop"[^>]*\/>\n?/, "")
    .replace(/<g data-id="#X1::([^"]+)">/g, (_, key) => `<g class="g-lbl" data-key="${esc(key)}">`)
    .replace(/<g data-id="[^"]*">/g, "<g>");
  const px = (x) => svgPoint([x, 0], scale)[0];
  const py = (y) => svgPoint([0, y], scale)[1];
  const over = [];
  if (box) {
    over.push(
      `<rect class="g-bx" x="${px(box[0])}" y="${py(box[3])}" width="${num((box[2] - box[0]) * scale)}" height="${num((box[3] - box[1]) * scale)}"/>`,
    );
  }
  for (const p of pins) {
    const [x, y] = svgPoint(p.at, scale);
    const tag = pinTag(p, scale);
    const arrow = ARROWS[p.dir] ? `<tspan class="g-dir">${ARROWS[p.dir]}</tspan>` : "";
    over.push(
      `<g class="g-pn"><circle cx="${x}" cy="${y}" r="2.6"/>` +
        `<text x="${num(tag.x)}" y="${num(tag.y)}" text-anchor="${tag.anchor}">${esc(tag.text)}${arrow}</text></g>`,
    );
  }
  const vb = viewBox([x0, y0, x1, y1], scale, 0).map(num).join(" ");
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${vb}" width="${num((x1 - x0) * SCALE_UNIT)}" height="${num((y1 - y0) * SCALE_UNIT)}"` +
    ` font-family="Noto Sans SC, Noto Sans CJK SC, Source Han Sans SC, sans-serif">` +
    `<g class="g-art">${body}</g><g class="g-ov">${over.join("")}</g></svg>`
  );
}

// ---------------------------------------------------------------- the data of a group

let warnings = 0;
/** What the build noticed about a package, as a warning annotation when it runs in GitHub Actions. */
function warn(path, line, text) {
  warnings++;
  if (process.env.GITHUB_ACTIONS) console.log(`::warning file=${path},line=${line}::${text}`);
  else console.warn(`${path}:${line}: ${text}`);
}

function groupData(group) {
  const names = byName(group);
  // the packages that using rnl-std brings in, which every document of the series has (16.3)
  const implicit = new Set(names.has("rnl-std") ? expansion(group, "rnl-std") : []);
  const referencedBy = new Map();
  for (const p of group.packages) {
    for (const r of p.header.references) {
      if (!referencedBy.has(r.name)) referencedBy.set(r.name, []);
      referencedBy.get(r.name).push(p.header.name);
    }
  }
  // every symbol of the group by name, for the parts a symbol draws; a package's own first
  const all = new Map();
  for (const p of group.packages) for (const s of p.statements) if (s.kind === "symbol" && !all.has(s.name)) all.set(s.name, s);
  const counts = { packages: 0, symbols: 0 };
  const order = [];
  for (const top of ["rnl-std", "rnl-std-full"]) {
    if (!names.has(top)) continue;
    for (const n of [top, ...expansion(group, top).slice(0, -1)]) if (!order.includes(n)) order.push(n);
  }
  const rank = (p) => (order.includes(p.header.name) ? order.indexOf(p.header.name) : order.length);
  const sorted = [...group.packages].sort((a, b) => rank(a) - rank(b) || (a.header.name < b.header.name ? -1 : 1));
  const packages = sorted.map((pkg) => {
    const own = new Map(pkg.statements.filter((s) => s.kind === "symbol").map((s) => [s.name, s]));
    const lookup = { get: (n) => own.get(n) || all.get(n) };
    for (const s of own.values()) Object.assign(s, (({ pins, labels }) => ({ allPins: pins, allLabels: labels }))(resolve(s, lookup)));
    const drawn = [...own.values()].filter((s) => s.name.startsWith("$"));
    const pictures = drawn.length ? draw(pkg, drawn) : [];
    drawn.forEach((s, k) => (s.card = pictures[k]));
    for (const p of pkg.problems || []) warn(pkg.path, p.line, p.text);
    for (const s of drawn) for (const p of s.card.problems) warn(pkg.path, s.line, `${s.name}: ${p}`);
    counts.packages++;
    counts.symbols += drawn.length;
    return {
      name: pkg.header.name,
      version: pkg.header.version,
      language: pkg.header.language,
      file: pkg.file,
      path: pkg.path,
      ...(pkg.source ? { source: pkg.source, summary: pkg.summary } : {}),
      implicit: implicit.has(pkg.header.name),
      references: pkg.header.references,
      referencedBy: referencedBy.get(pkg.header.name) || [],
      expansion: expansion(group, pkg.header.name),
      aliases: pkg.header.aliases,
      description: pkg.description,
      problems: pkg.problems || [],
      text: pkg.text,
      statements: pkg.statements.map((s) => {
        if (s.kind !== "symbol") return s;
        const { allPins, allLabels, card, pins: _own, labels: _ownLabels, ...rest } = s;
        if (!card) return { ...rest, labels: allLabels, pins: allPins };
        const { pins, box: drawnBox, ...picture } = card;
        return { ...rest, labels: allLabels, pins, drawnBox, ...picture };
      }),
    };
  });
  return {
    id: group.id,
    dir: group.dir,
    kind: group.kind,
    series: group.series,
    packages,
    counts,
  };
}

// ---------------------------------------------------------------- writing

const target = join(out, "gallery");
rmSync(target, { recursive: true, force: true });
mkdirSync(target, { recursive: true });
if (publicCatalog.provenance) writeFileSync(join(target, "catalog-provenance.json"), JSON.stringify(publicCatalog.provenance, null, 2) + "\n");
const index = [];
for (const group of groups) {
  const data = groupData(group);
  writeFileSync(join(target, `${group.id}.js`), `RNLGallery.group(${JSON.stringify(data)});\n`);
  index.push({
    id: data.id,
    dir: data.dir,
    kind: data.kind,
    series: data.series,
    packages: data.packages.map((p) => ({ name: p.name, version: p.version })),
    counts: data.counts,
  });
  console.log(`${group.dir}: ${data.counts.packages} packages, ${data.counts.symbols} symbols drawn`);
}
writeFileSync(join(target, "index.js"), `RNLGallery.index(${JSON.stringify({ groups: index })});\n`);
console.log(`wrote ${posix(relative(root, target))}/${warnings ? `, with ${warnings} warnings` : ""}`);
