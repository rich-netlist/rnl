// The fake shell: the Shell of apps/viewer-ui/shell.js for the page tests, which the test server
// serves as shells/fake.js, so that `?shell=fake` loads it. It opens the fixtures, Documents that
// the global setup built natively with `rnl view`; it records what the page asks of it, and does
// what a test tells it through `window.fakeShell`.
//
// The URL sets it up: `open=NAME` opens the fixture NAME at start; `fail-start=MESSAGE` rejects the
// question of what to open at start with that plain message, as Tauri's invoke rejects; and
// `platform=desktop` offers the desktop's commands with the menu shown natively, as the desktop
// shell does, and what Help > Check for Updates needs, the version that runs being the one
// `version=VERSION` gives, 0.1.0 by default, and what the Solve menu needs: a solve gives the
// fixture NAME-preview of the source, which `rnl preview` printed, and the table of the solver's settings, which
// Solve > Settings draws its dialog from, is TABLE below; otherwise it offers the web's commands and the page
// shows its own menu bar.
//
// A build gives the Document as the real shells' cores do: without the drawings of the objects that
// have no position, which it keeps and gives on request (`unplacedDrawings`), the fixture having
// them; a solve's Document carries them, as the desktop's does.

import { fileFormats } from "../formats.js";
import { sourcePath } from "../source-path.js";

const params = new URLSearchParams(location.search);
// the core's parse formats, as Document.formats lists them
const FORMATS = [
  { name: "spice", title: "SPICE", extensions: [], fallback: true },
  { name: "spectre", title: "Spectre", extensions: ["scs"], fallback: false },
  { name: "rnl", title: "RNL package", extensions: ["rnl"], fallback: false },
  { name: "text", title: "Text", extensions: ["va", "vams", "v", "vh", "sv", "svh", "vhd", "vhdl"], fallback: false },
];
// as the core detects (rnl_view::detect_format): by the suffix, and a package by its header record with PACKAGE
const detect = (path, text = "") => (/\.scs$/i.test(path) ? "spectre"
  : /\.rnl$/i.test(path) || /^@RNL\(type="header"\)\s*\{[^}]*@PACKAGE\(/.test(text) ? "rnl" : "spice");
const platform = params.get("platform") === "desktop" ? "desktop" : "web";
const NO_PACKAGES = { bundled: false, catalog: "", fetched: "", packages: [], licenses: [] };
const BUNDLED_PACKAGES = {
  bundled: true, catalog: "https://rich-netlist.github.io/rnl-packages/catalog.json", fetched: "2026-10-27",
  packages: [["analog-canvas-extensions", "0.4", "extension"], ["eda-style", "0.1", "extension"], ["eda-symbol", "0.1", "extension"], ["pdk-sky130", "0.1", "pdk"]]
    .map(([name, version, kind]) => ({ name, version, kind, path: `/app/public-packages/${name}/${version}/${name}.rnl` })),
  licenses: ["/app/public-packages/LICENSES/MIT.txt"],
};

/** A source: a fixture, by its name or as { name, fixture }; null for nothing. */
const source = (s) => (s === null || s === undefined ? null : typeof s === "string" ? { name: s, fixture: s } : s);

let handler = null; // the page's handler of the commands chosen in the platform's menu

/**
 * The table of the solver's settings, as `rnl solver-defaults --json` printed it when this was written
 * (docs/solver-settings-table.md), which the desktop's `solver_table` gives: the page draws the settings
 * dialog from it and keeps none of the settings itself, so what the tests need is that it has the shape,
 * not that it is the solver's present table.
 */
const TABLE = {
  version: 1,
  groups: [
    {"name":"layout","what":"Layout"},
    {"name":"search","what":"Search and preferences"},
    {"name":"run","what":"Limit of each level's solve"},
    {"name":"caps","what":"Caps on the search and the router"},
    {"name":"report","what":"Report"},
  ],
  settings: [
    {"key":"layout","group":"layout","type":"choice","choices":[{"value":"rows","what":"Components in rows by how deep they are along the current, with fixed gaps."},{"value":"tree","what":"Groups arranged in nested rows and columns, the wires routed between them."},{"value":"lines","what":"Components placed along the lines of their wires, then the drawing compacted on the grid."}],"default":"lines","in_file":true,"what":"How the level is laid out."},
    {"key":"spacing_x","group":"layout","type":"integer","min":-1,"max":20,"default":0,"in_file":true,"what":"Grid units added to the gaps between columns of the line layout, -1 to close them up; respacing uses it in every layout.","applies":{"layout":["lines"]}},
    {"key":"spacing_y","group":"layout","type":"integer","min":-1,"max":20,"default":0,"in_file":true,"what":"Grid units added to the gaps between rows of the line layout, -1 to close them up; respacing uses it in every layout.","applies":{"layout":["lines"]}},
    {"key":"junction","group":"layout","type":"string","in_file":true,"what":"The definition drawn where wires meet in a junction dot, by its name; an empty string draws none."},
    {"key":"beam","group":"search","type":"integer","min":1,"max":1024,"default":4,"in_file":true,"what":"How many partial candidates the row search keeps at each step: more searches wider and slower.","applies":{"layout":["rows"]}},
    {"key":"objectives","group":"search","type":"objectives","default":"current_direction,turns,ground_direction,component_alignment,distribution,symmetry,terminal_wire_alignment,area,crossings,wire_length,aspect,crowding","in_file":true,"what":"What the solver prefers among layouts that keep every fact, the most important first: layers separated by commas, the objectives of a layer joined by +, a weight as name*2, a tolerance of a layer as ~0.5; none for no preference."},
    {"key":"grid","group":"search","type":"number-or-false","min_exclusive":0,"default":1,"in_file":true,"what":"The step the positions are rounded to.","false_what":"Positions are not rounded to a grid."},
    {"key":"tolerance","group":"search","type":"number","min":0,"default":0.000001,"in_file":true,"what":"How far a coordinate may miss a fixed value of a fact and the fact still hold."},
    {"key":"limit","group":"run","type":"choice","choices":[{"value":"time","what":"Each level stops after the given seconds."},{"value":"work","what":"Each level stops after the given work: the same result on every machine."},{"value":"seed","what":"Each level takes the layout's first pass, with its own quick improvements, and searches no further."}],"default":"time","in_file":false,"what":"What ends the solve of each level."},
    {"key":"seconds","group":"run","type":"number","min":0,"default":10,"in_file":false,"what":"The seconds each level's solve may take.","shown_when":{"limit":["time"]}},
    {"key":"work","group":"run","type":"integer","min":0,"max":9007199254740992,"in_file":false,"what":"The work each level's solve may do; a solve reports the work it did, to be run again alike.","shown_when":{"limit":["work"]},"required_when":{"limit":["work"]}},
    {"key":"safety_seconds","group":"run","type":"number","min":0,"default":30,"in_file":false,"what":"The seconds after which a solve limited by work or to the seed stops all the same.","shown_when":{"limit":["work","seed"]}},
    {"key":"candidates","group":"caps","type":"integer","min":0,"max":9007199254740992,"default":256,"in_file":false,"what":"The most candidates the row search makes over a level.","applies":{"layout":["rows"]}},
    {"key":"astar_expansions","group":"caps","type":"integer","min":0,"max":9007199254740992,"default":200000,"in_file":false,"what":"The most cells the router expands for one net."},
    {"key":"report_hints","group":"report","type":"boolean","default":false,"in_file":false,"what":"Give every conflict of a discarded fact of priority low in the report, which can take much longer."},
  ],
  objectives: [
    {"name":"current_direction","what":"Transistors with their source or emitter toward the ground, the current running down.","implemented":true},
    {"name":"ground_direction","what":"Ground symbols below their pins.","implemented":true},
    {"name":"signal_flow","what":"Inputs on the left and outputs on the right.","implemented":true},
    {"name":"component_alignment","what":"Components sharing rows and columns.","implemented":true},
    {"name":"distribution","what":"Even gaps between neighbouring components of a group.","implemented":true},
    {"name":"symmetry","what":"Mirror pairs drawn symmetric.","implemented":true},
    {"name":"turns","what":"Wires with fewer bends.","implemented":true},
    {"name":"crossings","what":"Fewer crossings of wires of different nets.","implemented":true},
    {"name":"wire_length","what":"Shorter wires.","implemented":true},
    {"name":"area","what":"A smaller drawing.","implemented":true},
    {"name":"aspect","what":"A drawing near 16:9.","implemented":true},
    {"name":"terminal_wire_alignment","what":"Wires leaving terminals in line.","implemented":false},
    {"name":"labels","what":"Labels clear of the drawing.","implemented":false},
    {"name":"bend_alignment","what":"Bends of different wires in line.","implemented":false},
    {"name":"overlaps","what":"No symbols overlapping.","implemented":true},
    {"name":"crowding","what":"Nothing drawn too close, and no text over another symbol.","implemented":true},
    {"name":"misleading","what":"No wires that seem to connect what they do not.","implemented":true},
    {"name":"wire_over_symbol","what":"No wire through a symbol.","implemented":true},
  ],
  default_objectives: [
    {"items":[{"name":"current_direction","weight":1}],"tolerance":0},
    {"items":[{"name":"turns","weight":1}],"tolerance":0},
    {"items":[{"name":"ground_direction","weight":1}],"tolerance":0},
    {"items":[{"name":"component_alignment","weight":1}],"tolerance":0},
    {"items":[{"name":"distribution","weight":1}],"tolerance":0},
    {"items":[{"name":"symmetry","weight":1}],"tolerance":0},
    {"items":[{"name":"terminal_wire_alignment","weight":1}],"tolerance":0},
    {"items":[{"name":"area","weight":1}],"tolerance":0},
    {"items":[{"name":"crossings","weight":1}],"tolerance":0},
    {"items":[{"name":"wire_length","weight":1}],"tolerance":0},
    {"items":[{"name":"aspect","weight":1}],"tolerance":0},
  ],
  stages: [
    {"name":"resolve","what":"Checking the facts against each other"},
    {"name":"structure","what":"Reading the circuit's structure and groups"},
    {"name":"search","what":"Searching for placements"},
    {"name":"coords","what":"Placing the candidates on the grid"},
    {"name":"route","what":"Routing the wires"},
    {"name":"score","what":"Comparing the candidates"},
    {"name":"tree","what":"Laying out the groups"},
    {"name":"lines","what":"Laying out along the signal lines"},
    {"name":"respace","what":"Spacing the drawing again"},
    {"name":"junctions","what":"Placing the junction dots"},
    {"name":"write","what":"Writing the record back"},
  ],
};

/** What the page asked of the shell, and what it should do next. */
const fake = {
  /**
   * The calls, in order: ["initial"], ["pick", kind], ["build", name], or ["build", name, text] for
   * a source that holds its text, ["setCheck", id, checked], ["browse", url], and on the desktop
   * ["version"].
   */
  calls: [],
  packages: {},
  pathErrors: {},
  /** Deliver a source as a desktop drop or other unsolicited open. */
  open(src) { if (opening) opening(source(src)); },
  /** The check marks as the platform's menu shows them, by command id. */
  checks: {},
  /** What the next picks choose, in order: a source each; nothing is chosen once it is empty. */
  picks: [],
  /** A message the next build rejects with, as a shell does when it cannot read the source. */
  fail: null,
  /**
   * A fixture that the next build returns instead of its source's, as if the file had changed; the
   * one a source that holds its text builds, which has none of its own.
   */
  next: null,
  /** Choose a command in the platform's menu. */
  menu(id) {
    if (handler) handler(id);
  },
  /** A message the next solve rejects with. */
  solveFail: null,
  /** Whether a solve waits, once it has its fixture, until `release()` or a cancel. */
  hold: false,
  /** Let a solve that waits give its result. */
  release() {
    if (held) held.resolve();
    held = null;
  },
  /** Tell the page of one event of the solve under way (docs/solver-progress.md), as the desktop does. */
  step(event) {
    if (progress) progress(event);
  },
  /** The edits each build was given, in order: undefined for a build of the files as they are. */
  buildEdits: [],
  /** The format choices each build was given, in order, `{ path: format }`, as the shell sends them. */
  buildFormats: [],
  /** Whether each build was asked for the constraints of the levels, in order. */
  buildConstraints: [],
  /** The parameters each build was asked to bind, in order, `{ subckt, params }`, or undefined. */
  buildBinds: [],
  /** The default formats `detectFormat` gives, by path; otherwise by the `.scs` extension. */
  detected: {},
  /** Whether a format chosen for a file is kept in the settings (`keepsFormat`); true by default. */
  keepsFormat: true,
  /** The edits each solve was given, in order. */
  solveEdits: [],
  /** The files each Save was asked to write, in order: [{ path, name, text }, ...]. */
  writes: [],
  /** A message the next Save rejects with, as a shell does when it cannot write. */
  writeFail: null,
  /** The names of files that another program changed since they were read: the next Save, unless it overwrites, writes nothing and says so. */
  changedOnDisk: null,
  /** Whether each Save, written or not, was to overwrite, in order. */
  overwrites: [],
  /** What the next Save As gives: a source, a fixture's name or { name, fixture }; null when the user cancels. */
  savedAs: null,
  /** The files each Save As was asked to write, in order. */
  writesAs: [],
  /** Ask the page to close the window, as the platform does while changes are not saved. */
  requestClose() {
    if (closing) closing();
  },
  /** What the next copy saved says, or a message it rejects with when `saveFail` is set. */
  saved: "Saved the solved copy as /copies/solved.sp.",
  saveFail: null,
  /** The check each level of the next solves' reports carries, as `rnl solve --check --json` gives a level's: `{ settings, warnings, errors }`; none leaves what the fixture has. */
  levelCheck: null,
  /** What the solver says is wrong with the settings it is asked to check, `{ key, message }` each; none when they will do. */
  settingErrors: [],
  /**
   * Whether the next builds carry the drawings of the objects without a position in their
   * Documents, as the Documents of a core that was not asked to leave them out do; then no build
   * keeps them to give on request.
   */
  carry: false,
  /** A message the next request for drawings rejects with. */
  drawingsFail: null,
  /** Whether requests for drawings wait, once the shell has them, until `releaseDrawings()`. */
  holdDrawings: false,
  /** Let the requests for drawings that wait give them. */
  releaseDrawings() {
    for (const resolve of heldDrawings.splice(0)) resolve();
  },
};
window.fakeShell = fake;

let held = null; // the solve that waits: { resolve, reject }
let solveCancelled = false; // cancellation can arrive before the fixture fetch finishes
let heldDrawings = []; // the requests for drawings that wait: their resolve
let kept = null; // the drawings of the last build that left them out of its Document, by level: { names, drawings }
let closing = null; // the page's handler of a request to close the window
let progress = null; // the page's handler of the events of a solve
let opening = null;

/**
 * The desktop's Solve menu: a solve records ["solve", name, settings] and gives the fixture
 * NAME-preview; a cancel ["cancelSolve"], and rejects a solve that waits as the desktop does; a copy
 * ["saveSolved"]; the solver's table of settings ["solverTable"]; and whether it takes some settings
 * ["checkSettings", config], which `settingErrors` makes it refuse.
 */
const solving = {
  async solverTable() {
    fake.calls.push(["solverTable"]);
    return structuredClone(TABLE);
  },
  async checkSettings(config) {
    fake.calls.push(["checkSettings", config]);
    // what `rnl solve --check --json` gives for a text with no file: one level, with its problems by key
    const errors = fake.settingErrors;
    if (errors.length > 0) {
      const checked = { ok: false, levels: [{ title: null, settings: {}, warnings: [], errors }] };
      return checked;
    }
    return { ok: true, levels: [{ title: null, settings: {}, warnings: [], errors: [] }] };
  },
  async solve(src, settings, edits) {
    solveCancelled = false;
    fake.calls.push(["solve", src.name, settings]);
    fake.solveEdits.push(edits);
    const fail = fake.solveFail;
    fake.solveFail = null;
    if (fail !== null) throw fail;
    const response = await fetch(`/fixtures/${encodeURIComponent(src.fixture)}-preview.json`);
    if (!response.ok) throw `${src.name}: there is no such preview`;
    const solved = await response.json();
    if (settings?.respace) solved.report.mode = "respace";
    if (solveCancelled) throw "the solve was cancelled";
    if (fake.hold) await new Promise((resolve, reject) => (held = { resolve, reject }));
    // what the check of each level's settings found, as a solve's report gives it (`levels[].check`)
    if (fake.levelCheck) for (const level of solved.report.levels) level.check = structuredClone(fake.levelCheck);
    return solved;
  },
  cancelSolve() {
    solveCancelled = true;
    fake.calls.push(["cancelSolve"]);
    if (held) held.reject("the solve was cancelled");
    held = null;
  },
  async saveSolved(encoding) {
    (fake.encodings ||= []).push(["saveSolved", encoding]);
    fake.calls.push(["saveSolved"]);
    const fail = fake.saveFail;
    fake.saveFail = null;
    if (fail !== null) throw fail;
    return fake.saved;
  },
  onSolveProgress(h) {
    progress = h;
  },
};

/** The version that runs, which Help > Check for Updates asks of the desktop. */
async function version() {
  fake.calls.push(["version"]);
  return params.get("version") || "0.1.0";
}

// A language only these tests offer, with `pseudo` in the URL: every text the catalogs translate, its
// letters accented and made 40% longer, in brackets ［…］, its placeholders, its shortcut after a tab
// and its & kept. A text of the interface that no catalog has shows as plain English among them, and
// one that a longer language would not fit shows so (decision 0015).
const PSEUDO = { code: "qps-ploc", lang: "qps", name: "Pseudo", english: "Pseudo", matches: [] };
const ACCENTS = Object.fromEntries([..."abcdefghijklmnopqrstuvwxyz"].map((c, i) => [c, [..."áƀçðéƒĝĥíĵķĺɱñóþǫŕšţúṽŵẋýž"][i]]));
export function pseudo(text) {
  const [label, keys] = text.split("\t");
  const body = label.split(/(\{\d+\})/).map((part, i) => i % 2 ? part : [...part].map((c) => {
    const a = ACCENTS[c.toLowerCase()];
    return !a ? c : c === c.toLowerCase() ? a : a.toUpperCase();
  }).join("")).join("");
  return `［${body}${"·".repeat(Math.ceil(label.length * 0.4))}］${keys === undefined ? "" : `\t${keys}`}`;
}
const locales = async (name) => (await fetch(`locales/${name}`)).json();

/** @type {import("../../apps/viewer-ui/shell.js").Shell} */
/** Save As: the file is written where the user says, and the source of the file written is the netlist from then on. */
async function saveAs(src, file, encoding) {
  (fake.encodings ||= []).push(["saveAs", encoding]);
  fake.calls.push(["saveAs", src.name]);
  fake.writesAs.push(file);
  const fail = fake.writeFail;
  fake.writeFail = null;
  if (fail !== null) throw fail;
  const next = fake.savedAs;
  fake.savedAs = null;
  return source(next);
}

export default {
  async uiLanguages() {
    const languages = await locales("languages.json");
    return params.has("pseudo") ? [...languages, PSEUDO] : languages;
  },
  async uiCatalog(code) {
    if (code !== PSEUDO.code) return locales(`${code}.json`);
    const keys = new Set();
    for (const { code: other } of (await locales("languages.json")).slice(1)) {
      for (const key of Object.keys(await locales(`${other}.json`))) keys.add(key);
    }
    const walk = (items) => {
      for (const it of items) {
        if (it === "-") continue;
        if (it.label && !it.id?.startsWith("language-")) keys.add(it.label);
        if (it.items) walk(it.items);
      }
    };
    walk((await fetch("commands.json").then((r) => r.json())).menus);
    return Object.fromEntries([...keys].map((key) => [key, pseudo(key)]));
  },
  // Most fixtures exercise same-document UI. A test can supply independent file fixtures.
  sourceForFile(src, document, index) { return fake.fileSources?.[document.files[index].path] || src; },
  sourceForPath(src, path) { fake.calls.push(["sourceForPath", path]); return fake.fileSources?.[path] || { ...src, path, fake: path }; },
  newWindow(session) { fake.calls.push(["newWindow", JSON.parse(JSON.stringify(session))]); },
  include: async (kind) => { fake.calls.push(["include", kind]); return fake.includes || []; },
  ...(params.has("pool") ? {
    bundledFiles: async () => ({ "<bundled>/0.3/rnl-std.rnl": "@RNL(type=\"header\") { @PACKAGE(name=\"rnl-std\", version=\"0.3\") }" }),
    include: async (kind) => { fake.calls.push(["include", kind]); return fake.includes || []; },
    setPool: (items, reload = false, refresh = []) => {
      fake.pool = items.filter((item) => !item.builtin).map((item) => item.path);
      fake.calls.push(["setPool", [...fake.pool], reload]);
      fake.poolRefreshes ??= [];
      fake.poolRefreshes.push(refresh);
      return fake.libraryErrors || [];
    },
    ...(platform === "desktop" ? {
      pickPath: async () => fake.path || null,
      // a directory alone gives its files; with its sub-folders, `deepPackages` and `truncated` say what
      pathPackages: async (dir, subfolders = false) => {
        fake.calls.push(subfolders ? ["pathPackages", dir, "subfolders"] : ["pathPackages", dir]);
        if (fake.pathErrors[dir]) throw fake.pathErrors[dir];
        if (!subfolders) return fake.packages[dir] || [];
        return { files: fake.deepPackages?.[dir] || fake.packages[dir] || [], truncated: !!fake.truncated?.[dir] };
      },
      // projects (project.js): the files on `disk`, the archive saved, the one to open and where it goes
      readFile: async (path) => {
        fake.calls.push(["readFile", path]);
        if (fake.disk?.[path] === undefined) throw new Error(`${path}: there is no such file`);
        return new TextEncoder().encode(fake.disk[path]);
      },
      saveProject: async (name, build) => {
        fake.savedProject = { name, bytes: Array.from(new Uint8Array(await (await build()).arrayBuffer())) };
        return `C:/saved/${name}`;
      },
      pickProject: async () => fake.project ? { name: fake.project.name, path: `C:/projects/${fake.project.name}`, bytes: new Uint8Array(fake.project.bytes) } : null,
      projectRoot: async () => fake.projectRoot ?? null,
      projectSource: async (read) => {
        fake.written = Object.keys(read.bytes);
        return { source: { name: read.entry, path: read.entry, fixture: "04-hierarchy" },
          items: read.pool.map((path) => ({ path, name: path, source: { path, name: path } })) };
      },
    } : { includeLibrary: async () => [] }),
  } : {}),
  ...(params.has("settings") ? {
    loadSettings: async (legacy) => {
      fake.legacy = legacy;
      const values = JSON.parse(sessionStorage.getItem("fake-settings") || JSON.stringify(legacy));
      sessionStorage.setItem("fake-settings", JSON.stringify(values));
      return { path: "/user/config/settings.json", values };
    },
    saveSetting: async (key, value) => {
      const values = JSON.parse(sessionStorage.getItem("fake-settings"));
      values[key] = value;
      sessionStorage.setItem("fake-settings", JSON.stringify(values));
    },
  } : {}),
  platform,
  nativeMenu: platform === "desktop",
  formats: async () => { fake.calls.push(["formats"]); return FORMATS; },
  // the desktop's bundled public packages (public_packages.rs): `bundled=1` in the address is a build that has them
  ...(platform === "desktop" ? { bundledPackages: async () => (params.has("bundled") ? BUNDLED_PACKAGES : NO_PACKAGES) } : {}),
  // given the text of the file too, as the shells give it to the core: that of the fixture a Library file is of
  detectFormat: async (path) => {
    fake.calls.push(["detectFormat", path]);
    if (fake.detected[path]) return fake.detected[path];
    const fixture = (fake.includes || []).find((item) => item.path === path)?.source?.fixture;
    const text = fixture ? (await (await fetch(`/fixtures/${encodeURIComponent(fixture)}.json`)).json()).files[0].text : "";
    return detect(path, text);
  },
  keepsFormat: () => fake.keepsFormat,
  ...(platform === "desktop" ? { version, ...solving } : {}),
  async initial() {
    fake.calls.push(["initial"]);
    if (params.has("fail-start")) throw params.get("fail-start");
    return source(params.get("open"));
  },
  async pick(kind) {
    fake.calls.push(["pick", kind]);
    return source(fake.picks.shift());
  },
  async build(src, edits, _progress, _packages, _signal, { bind, constraints } = {}) {
    fake.calls.push(src.text === undefined ? ["build", src.name] : ["build", src.name, src.text]);
    fake.buildEdits.push(edits);
    fake.buildBinds.push(bind);
    fake.buildConstraints.push(!!constraints);
    const formats = fileFormats();
    fake.buildFormats.push(formats);
    const [fail, name] = [fake.fail, fake.next || src.fixture || (params.has("pool") ? "04-hierarchy" : undefined)];
    fake.fail = null;
    fake.next = null;
    // a plain message, as the desktop shell's invoke rejects with
    if (fail !== null) throw fail;
    // a build asked for the constraints gives the fixture NAME-constraints, `rnl view --constraints`, where there is one
    let response = constraints ? await fetch(`/fixtures/${encodeURIComponent(name)}-constraints.json`) : { ok: false };
    if (!response.ok) response = await fetch(`/fixtures/${encodeURIComponent(name)}.json`);
    if (!response.ok) throw `${src.name}: there is no such fixture`;
    const document = await response.json();
    if (params.has("pool") && src.temporary) {
      src.fixture = name;
      document.files[0].text = edits?.[document.files[0].path] ?? src.text;
    }
    for (const file of document.files) if (edits?.[file.path] !== undefined) file.text = edits[file.path];
    // what the core does with `bind`: the parameters of that subcircuit take the values, and nothing else changes
    for (const level of document.levels) {
      if (!bind || level.title !== bind.subckt) continue;
      for (const p of level.params || []) {
        const text = bind.params?.[p.name];
        if (text === undefined) continue;
        // a number with an SI suffix, as a netlist writes it; anything else the core would evaluate, which this does not
        const [, digits, suffix] = /^([-+]?[\d.]+(?:e[-+]?\d+)?)(f|p|n|u|m|k|meg|g)?$/i.exec(text) || [];
        const scale = { f: 1e-15, p: 1e-12, n: 1e-9, u: 1e-6, m: 1e-3, k: 1e3, meg: 1e6, g: 1e9 }[suffix?.toLowerCase()] ?? 1;
        Object.assign(p, { value: text, number: digits === undefined ? null : Number(digits) * scale, from: "bound" });
      }
    }
    // the formats a core gives: each host file's default and, where the build was given one, the choice
    document.formats ??= FORMATS;
    for (const file of document.files) {
      if (file.format === null) { file.default_format = null; continue; }
      file.default_format ??= file.format ?? detect(file.path, file.text);
      file.format = formats[sourcePath(document, file)] ?? file.default_format;
    }
    if (!fake.carry) {
      kept = document.levels.map((level) => ({
        names: level.unplaced.map((u) => u.name),
        drawings: level.unplaced.map(({ svg, primitives }) => ({ svg, primitives })),
      }));
      for (const u of document.levels.flatMap((level) => level.unplaced)) {
        delete u.svg;
        delete u.primitives;
      }
    }
    return document;
  },
  /**
   * The drawings the last build kept, of the objects at `items` of a level, which must be those of
   * `names`: the core refuses a request about a Document that was built since.
   */
  async unplacedDrawings(level, items, names) {
    fake.calls.push(["unplacedDrawings", level, items]);
    if (fake.holdDrawings) await new Promise((resolve) => heldDrawings.push(resolve));
    const fail = fake.drawingsFail;
    fake.drawingsFail = null;
    if (fail !== null) throw fail;
    if (kept === null) throw "no document has kept drawings";
    const there = kept[level];
    if (!there || items.some((i) => !(i in there.drawings))) throw `level ${level} lists no unplaced object in ${items}`;
    if (items.some((i, k) => there.names[i] !== names[k])) throw "the document was built again";
    return items.map((i) => there.drawings[i]);
  },
  setCheck(id, checked) {
    fake.calls.push(["setCheck", id, checked]);
    fake.checks[id] = checked;
  },
  onCommand(h) {
    handler = h;
  },
  onOpen(h) { opening = h; },
  browse(url) {
    fake.calls.push(["browse", url]);
  },
  /** Decision 0011: Save writes the files and says so; on the desktop Save As asks where; the platform is told of changes not saved. */
  async save(src, files, overwrite, encoding) {
    (fake.encodings ||= []).push(["save", encoding]);
    fake.calls.push(["save", src.name]);
    fake.overwrites.push(overwrite === true);
    if (fake.changedOnDisk !== null && overwrite !== true) {
      const changed = fake.changedOnDisk;
      fake.changedOnDisk = null;
      return { changed };
    }
    fake.changedOnDisk = null;
    fake.writes.push(files);
    const fail = fake.writeFail;
    fake.writeFail = null;
    if (fail !== null) throw fail;
    return `Saved ${files.map((f) => f.name).join(", ")}.`;
  },
  // `overwrites=1`: a web page whose shell writes the files where they were read from, as the local shell does
  ...(platform === "web" && params.get("overwrites") === "1" ? { overwrites: true, saveAs } : {}),
  ...(platform === "desktop"
    ? {
        saveAs,
        onCloseRequested(h) {
          closing = h;
        },
        quit() {
          fake.calls.push(["quit"]);
        },
      }
    : {}),
  setDirty(dirty) {
    fake.calls.push(["setDirty", dirty]);
  },
};
