// The panel over the drawing while a solve runs: which circuit, of how many, and the stage the solver is in,
// the share of its limit used and the time, how many candidates it has tried and how many better ones it
// found, and each circuit already solved with how its solve ended. It is driven by the events of
// docs/solver-progress.md, which every shell gives the page as they come (shell.onSolveProgress): `run`,
// `level`, `stage`, `work`, `best`, `level-done`; `done` and `error` end the solve, which solve.js knows
// of by the promise. A field or an event it does not know is ignored, and a stage it has no words for is
// shown by its name. Between two events the clock goes on from the last one told, so that the time moves.

import { m, setAttr, setText } from "./i18n.js";
import { $, span } from "./util.js";

/** What each stage of the solve does, as `rnl_cli::progress::STAGES` says it. */
const STAGES = {
  resolve: m("Checking the facts against each other"),
  structure: m("Reading the circuit's structure and groups"),
  search: m("Searching for placements"),
  coords: m("Placing the candidates on the grid"),
  route: m("Routing the wires"),
  score: m("Comparing the candidates"),
  tree: m("Laying out the groups"),
  lines: m("Laying out along the signal lines"),
  junctions: m("Placing the junction dots"),
  write: m("Writing the record back"),
};

/** How a level's solve ended (`level-done`), when it did not simply finish. */
const ENDS = {
  time: m("stopped at the time limit"),
  work: m("stopped at the work limit"),
  safety: m("stopped at the safety limit"),
  cancelled: m("canceled"),
};

/** The most circuits listed as solved; the panel does not grow with the file. */
const LISTED = 6;

let panel = null;
let parts = null;
let timer = 0;
let run = null; // the state of the solve under way, null when none

const now = () => performance.now() / 1000;

function build() {
  panel = document.createElement("div");
  panel.id = "solve-progress";
  panel.className = "solve-progress";
  panel.setAttribute("role", "region");
  setAttr(panel, "aria-label", m("Solve progress"));
  const bar = document.createElement("div");
  bar.className = "solve-progress-bar";
  bar.setAttribute("role", "progressbar");
  bar.setAttribute("aria-valuemin", "0");
  bar.setAttribute("aria-valuemax", "100");
  const fill = document.createElement("div");
  bar.append(fill);
  const done = document.createElement("ul");
  done.className = "solve-progress-done";
  parts = {
    title: span("solve-progress-title", ""),
    of: span("solve-progress-of", ""),
    stage: span("solve-progress-stage", ""),
    bar,
    fill,
    time: span("solve-progress-time", ""),
    tried: span("solve-progress-tried", ""),
    done,
  };
  parts.title.setAttribute("aria-live", "polite");
  const head = document.createElement("div");
  head.className = "solve-progress-head";
  head.append(parts.title, parts.of);
  panel.append(head, parts.stage, bar, parts.time, parts.tried, done);
  $("solve-progress-host").append(panel);
}

const seconds = (s) => (Math.round(s * 10) / 10).toFixed(1);

/** The text and bar of the level under way, from the last events and the clock since. */
function paint() {
  if (!run) return;
  const level = run.level;
  const elapsed = level ? level.elapsed + (now() - level.at) : now() - run.startedAt;
  setText(parts.title, level?.title ?? run.name);
  setText(parts.of, run.levels > 1 && level ? m`${level.index + 1} of ${run.levels}` : "");
  setText(parts.stage, level?.stage ? (STAGES[level.stage] ?? level.stage) : m("Starting"));
  // the share of the limit: the clock moves it between events when the limit is a time
  let share = null;
  if (level?.limit === "time" && level.seconds > 0) {
    share = Math.min(1, elapsed / level.seconds);
    setText(parts.time, m`${seconds(Math.min(elapsed, level.seconds))} s of ${seconds(level.seconds)} s`);
  } else if (level?.limit === "work") {
    share = Math.min(1, level.fraction ?? 0);
    setText(parts.time, m`${Math.round(share * 100)}% of the work limit, ${seconds(elapsed)} s`);
  } else {
    setText(parts.time, m`${seconds(elapsed)} s`);
  }
  panel.classList.toggle("unknown", share === null);
  parts.fill.style.width = share === null ? "0" : `${(share * 100).toFixed(1)}%`;
  if (share === null) parts.bar.removeAttribute("aria-valuenow");
  else parts.bar.setAttribute("aria-valuenow", String(Math.round(share * 100)));
  const tried = level?.tried ?? 0;
  const better = level?.better ?? 0;
  const lines = [];
  if (tried > 0) lines.push(tried === 1 ? m("1 candidate tried") : m`${tried.toLocaleString()} candidates tried`);
  if (better > 0) lines.push(better === 1 ? m("1 better one found") : m`${better} better ones found`);
  setText(parts.tried, lines.length ? lines.map((l) => String(l)).join(" · ") : "");
  parts.tried.hidden = lines.length === 0;
}

function listDone() {
  const items = run.done.slice(-LISTED).map((d) => {
    const li = document.createElement("li");
    const end = ENDS[d.status];
    li.append(span("name", d.title), span("sec", m`${seconds(d.seconds)} s`));
    if (end) li.append(span("end", end));
    return li;
  });
  parts.done.replaceChildren(...items);
  parts.done.hidden = items.length === 0;
}

/** A solve of the file called `name` begins. */
export function progressBegin(name) {
  if (!panel) build();
  run = { name, levels: 1, level: null, done: [], startedAt: now() };
  parts.done.replaceChildren();
  parts.done.hidden = true;
  panel.hidden = false;
  paint();
  clearInterval(timer);
  timer = setInterval(paint, 200);
}

/** One event of the solve under way (docs/solver-progress.md). Returns the level's title and its place, for the
 * status bar, when the event begins a level, else null. */
export function progressEvent(event) {
  if (!run || !event || typeof event !== "object") return null;
  const level = run.level;
  switch (event.event) {
    case "run":
      if (typeof event.levels === "number" && event.levels > 0) run.levels = event.levels;
      break;
    case "level":
      run.level = {
        index: event.level ?? 0,
        title: event.title ?? run.name,
        again: event.again === true,
        limit: event.limit,
        seconds: event.seconds,
        stage: "",
        elapsed: 0,
        at: now(),
        fraction: 0,
        tried: 0,
        better: 0,
      };
      break;
    case "stage":
    case "work":
      if (!level) break;
      if (typeof event.stage === "string") level.stage = event.stage;
      if (typeof event.elapsed === "number") { level.elapsed = event.elapsed; level.at = now(); }
      if (typeof event.fraction === "number") level.fraction = event.fraction;
      if (typeof event.tried === "number") level.tried = Math.max(level.tried, event.tried);
      break;
    case "best":
      if (!level) break;
      level.better += 1;
      if (typeof event.tried === "number") level.tried = Math.max(level.tried, event.tried);
      if (typeof event.elapsed === "number") { level.elapsed = event.elapsed; level.at = now(); }
      break;
    case "level-done":
      if (level) {
        // a level solved again after its file was raised to a newer version is the same circuit: its line is replaced
        if (level.again) run.done = run.done.filter((d) => d.title !== level.title);
        run.done.push({ title: level.title, status: event.status, seconds: typeof event.seconds === "number" ? event.seconds : level.elapsed });
      }
      listDone();
      break;
    default:
      break;
  }
  paint();
  return event.event === "level" ? { title: run.level.title, level: run.level.index, levels: run.levels, again: run.level.again } : null;
}

/** The solve has ended, however it ended. */
export function progressEnd() {
  clearInterval(timer);
  timer = 0;
  run = null;
  if (panel) panel.hidden = true;
}
