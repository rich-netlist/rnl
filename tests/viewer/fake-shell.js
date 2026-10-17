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

const params = new URLSearchParams(location.search);
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
  /** The edits each solve was given, in order. */
  solveEdits: [],
  /** The files each Save was asked to write, in order: [{ path, name, text }, ...]. */
  writes: [],
  /** A message the next Save rejects with, as a shell does when it cannot write. */
  writeFail: null,
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
};
window.fakeShell = fake;

let held = null; // the solve that waits: { resolve, reject }
let closing = null; // the page's handler of a request to close the window
let progress = null; // the page's handler of each level a solve begins

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
    fake.calls.push(["solve", src.name, settings]);
    fake.solveEdits.push(edits);
    const fail = fake.solveFail;
    fake.solveFail = null;
    if (fail !== null) throw fail;
    const response = await fetch(`/fixtures/${encodeURIComponent(src.fixture)}-preview.json`);
    if (!response.ok) throw `${src.name}: there is no such preview`;
    const solved = await response.json();
    if (fake.hold) await new Promise((resolve, reject) => (held = { resolve, reject }));
    return solved;
  },
  cancelSolve() {
    fake.calls.push(["cancelSolve"]);
    if (held) held.reject("the solve was cancelled");
    held = null;
  },
  async saveSolved() {
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

/** @type {import("../../apps/viewer-ui/shell.js").Shell} */
export default {
  platform,
  nativeMenu: platform === "desktop",
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
    const [fail, name] = [fake.fail, fake.next || src.fixture];
    fake.fail = null;
    fake.next = null;
    // a plain message, as the desktop shell's invoke rejects with
    if (fail !== null) throw fail;
    const response = await fetch(`/fixtures/${encodeURIComponent(name)}.json`);
    if (!response.ok) throw `${src.name}: there is no such fixture`;
    return response.json();
  },
  setCheck(id, checked) {
    fake.calls.push(["setCheck", id, checked]);
    fake.checks[id] = checked;
  },
  onCommand(h) {
    handler = h;
  },
  browse(url) {
    fake.calls.push(["browse", url]);
  },
  /** Decision 0011: Save writes the files and says so; on the desktop Save As asks where; the platform is told of changes not saved. */
  async save(src, files) {
    fake.calls.push(["save", src.name]);
    fake.writes.push(files);
    const fail = fake.writeFail;
    fake.writeFail = null;
    if (fail !== null) throw fail;
    return `Saved ${files.map((f) => f.name).join(", ")}.`;
  },
  ...(platform === "desktop"
    ? {
        async saveAs(src, file) {
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
