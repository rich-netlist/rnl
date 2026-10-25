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
// fixture NAME-preview of the source, which `rnl preview` printed, and the solver's defaults, which
// Solve > Settings shows, are DEFAULTS below; otherwise it offers the web's commands and the page
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
];
const detect = (path) => (/\.scs$/i.test(path) ? "spectre" : "spice");
const platform = params.get("platform") === "desktop" ? "desktop" : "web";

/** A source: a fixture, by its name or as { name, fixture }; null for nothing. */
const source = (s) => (s === null || s === undefined ? null : typeof s === "string" ? { name: s, fixture: s } : s);

let handler = null; // the page's handler of the commands chosen in the platform's menu

/**
 * The solver's options without any setting, as the desktop's `solver_defaults` gives them from
 * `SolverOptions::default()`: the page keeps none of them, so what the tests need is that they have
 * the shape, not that they are the solver's present values.
 */
const DEFAULTS = {
  tolerance: 0.000001,
  grid: 1,
  beam: 4,
  budget: { candidates: 256, astar_expansions: 200000 },
  time_limit: 5,
  hints_for_low: false,
  objectives: [
    "current_direction", "turns", "ground_direction", "component_alignment", "distribution", "symmetry",
    "terminal_wire_alignment", "area", "crossings", "wire_length", "aspect",
  ].map((name) => ({ items: [{ name, weight: 1 }], tolerance: 0 })),
  names: [
    "current_direction", "ground_direction", "signal_flow", "component_alignment", "distribution", "symmetry",
    "turns", "crossings", "wire_length", "area", "aspect", "terminal_wire_alignment", "labels", "bend_alignment",
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
  /** Tell the page that the solve under way begins a level. */
  step(step) {
    if (progress) progress(step);
  },
  /** The edits each build was given, in order: undefined for a build of the files as they are. */
  buildEdits: [],
  /** The format choices each build was given, in order, `{ path: format }`, as the shell sends them. */
  buildFormats: [],
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
  /** What the solver says is wrong with the settings it is asked to check; null when they will do. */
  optionsError: null,
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
let progress = null; // the page's handler of each level a solve begins
let opening = null;

/**
 * The desktop's Solve menu: a solve records ["solve", name, settings] and gives the fixture
 * NAME-preview; a cancel ["cancelSolve"], and rejects a solve that waits as the desktop does; a copy
 * ["saveSolved"]; the solver's defaults ["solverDefaults"]; and whether it takes some options
 * ["checkSolverOptions", options], which `optionsError` makes it refuse.
 */
const solving = {
  async solverDefaults() {
    fake.calls.push(["solverDefaults"]);
    return structuredClone(DEFAULTS);
  },
  async checkSolverOptions(options) {
    fake.calls.push(["checkSolverOptions", options]);
    return fake.optionsError;
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
    if (solveCancelled) throw "the solve was cancelled";
    if (fake.hold) await new Promise((resolve, reject) => (held = { resolve, reject }));
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
const PSEUDO = { code: "qps-ploc", lang: "qps", name: "Pseudo", matches: [] };
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
    return Object.fromEntries([...keys].map((key) => [key, pseudo(key)]));
  },
  // Most fixtures exercise same-document UI. A test can supply independent file fixtures.
  sourceForFile(src, document, index) { return fake.fileSources?.[document.files[index].path] || src; },
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
  detectFormat: async (path) => { fake.calls.push(["detectFormat", path]); return fake.detected[path] ?? detect(path); },
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
  async build(src, edits) {
    fake.calls.push(src.text === undefined ? ["build", src.name] : ["build", src.name, src.text]);
    fake.buildEdits.push(edits);
    const formats = fileFormats();
    fake.buildFormats.push(formats);
    const [fail, name] = [fake.fail, fake.next || src.fixture || (params.has("pool") ? "04-hierarchy" : undefined)];
    fake.fail = null;
    fake.next = null;
    // a plain message, as the desktop shell's invoke rejects with
    if (fail !== null) throw fail;
    const response = await fetch(`/fixtures/${encodeURIComponent(name)}.json`);
    if (!response.ok) throw `${src.name}: there is no such fixture`;
    const document = await response.json();
    if (params.has("pool") && src.temporary) {
      src.fixture = name;
      document.files[0].text = edits?.[document.files[0].path] ?? src.text;
    }
    for (const file of document.files) if (edits?.[file.path] !== undefined) file.text = edits[file.path];
    // the formats a core gives: each host file's default and, where the build was given one, the choice
    document.formats ??= FORMATS;
    for (const file of document.files) {
      if (file.format === null || /\.rnl$/i.test(file.path)) { file.format = file.default_format = null; continue; }
      file.default_format ??= file.format ?? detect(file.path);
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
  ...(platform === "desktop"
    ? {
        async saveAs(src, file, encoding) {
          (fake.encodings ||= []).push(["saveAs", encoding]);
          fake.calls.push(["saveAs", src.name]);
          fake.writesAs.push(file);
          const fail = fake.writeFail;
          fake.writeFail = null;
          if (fail !== null) throw fail;
          const next = fake.savedAs;
          fake.savedAs = null;
          return source(next);
        },
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
