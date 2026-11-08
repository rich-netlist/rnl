import { append, m, setAttr, setText, tr } from "./i18n.js";
// The problems list: the errors, warnings and hints, the most serious first; clicking one shows
// its line in the text. The line numbers of the lines they are on are coloured. The geometry hints of
// the level shown (`Level.hints`, what `rnl check` prints about the drawing) are in the list too, each
// with a small "geo" tag that says where it comes from and its own severity, "warning" or "hint". The
// counts of the heading are by severity alone, as `rnl check` counts, so a geometry hint is counted in
// its severity. Each count is a switch: it shows only the rows of that severity, and again shows all.
// (The levels are the one list SEVERITIES: another, such as the "violation" of 0.4, is one more entry.)

import { markSourceDiagnostics, revealLine } from "./text.js";
import { current, doc } from "./state.js";
import { $, span } from "./util.js";
import { shell } from "./shell.js";
import { includePublicPackage } from "./pool.js";
import { matchingPackage, missingPackage } from "./public-packages.js";

/** The severities, the most serious first: the order of the list, and of the counts in the heading. */
const SEVERITIES = [
  ["error", tr("{0} error"), tr("{0} errors")],
  ["warning", tr("{0} warning"), tr("{0} warnings")],
  ["hint", tr("{0} hint"), tr("{0} hints")],
];
const SEVERITY_ORDER = Object.fromEntries(SEVERITIES.map(([s], i) => [s, i]));
let renderVersion = 0;
const BATCH = 100;
const body = $("problems-body"), ul = $("diags");

// Thousands of rows must not all take part in the first layout after navigation.
// Keep every row in the model and append another batch as the list is read.
let rows = [], shown = 0, more = null, frame = 0;

/** The node or nodes of a row of the list: a diagnostic's, with the hint to look for a package it may have, or a geometry hint's. */
function nodesOf(row) {
  return row.geo ? [hintRow(row.item)] : [diagnosticRow(row.item), packageHint(row.item, renderVersion)];
}

function appendBatch() {
  more?.remove();
  more = null;
  const fragment = document.createDocumentFragment();
  const end = Math.min(shown + BATCH, rows.length);
  for (; shown < end; shown++) append(fragment, ...nodesOf(rows[shown]).filter(Boolean));
  if (shown < rows.length) {
    more = document.createElement("li");
    more.className = "diagnostics-more";
    const button = document.createElement("button");
    button.type = "button";
    setText(button, m`${shown} of ${rows.length} shown: scroll for more`);
    button.addEventListener("click", appendBatch);
    append(more, button);
    append(fragment, more);
  }
  append(ul, fragment);
  scheduleMore();
}

function scheduleMore() {
  if (frame || !more) return;
  frame = requestAnimationFrame(() => {
    frame = 0;
    if (more && body.clientHeight > 0 && body.scrollHeight - body.scrollTop - body.clientHeight < 100) appendBatch();
  });
}
body.addEventListener("scroll", scheduleMore, { passive: true });
new ResizeObserver(scheduleMore).observe(body);

function packageHint(d, version) {
  // Path-based includes and alias failures need different remedies. Keep the core diagnostic intact.
  const missing = missingPackage(d);
  if (!missing) return null;
  const li = document.createElement("li");
  li.className = "diag hint package-hint";
  const message = span("msg", m("Look for it in File › Include Public Package..."));
  append(li, span("sev", "hint"), message);
  if (shell.platform === "desktop") {
    // the desktop downloads no package: the bundled ones are in the Library, the others are added by hand
    setText(message, m("Look for it in Help › Public Packages."));
  } else if (shell.publicPackages) {
    shell.publicPackages().then((packages) => {
      if (version !== renderVersion || !li.isConnected) return;
      const pkg = matchingPackage(packages, missing.name, missing.version);
      if (!pkg) return;
      setText(message, m("This package is published. "));
      const include = document.createElement("button");
      include.type = "button";
      setText(include, m("Include"));
      include.addEventListener("click", async () => {
        include.disabled = true;
        try { await includePublicPackage(pkg); }
        finally { include.disabled = false; }
      });
      append(message, include);
    }).catch(() => { /* The manual File menu hint remains available offline. */ });
  }
  return li;
}

let diagnosticList = [];
let hints = []; // the geometry hints of the level shown
let hintsOf = undefined; // the `hints` of the level they are from, to tell whether another level is shown
let filter = null; // the severity whose count is on, so that only its rows are listed; null shows all

const hintsOfLevel = () => doc?.levels?.[current]?.hints;

export function renderDiagnostics(list) {
  ++renderVersion;
  diagnosticList = list;
  hintsOf = hintsOfLevel();
  hints = hintsOf ?? [];
  refresh();
}

/** The geometry hints of another level, when one is shown. */
function levelChanged() {
  if (hintsOfLevel() === hintsOf) return;
  hintsOf = hintsOfLevel();
  hints = hintsOf ?? [];
  refresh();
}
window.addEventListener("viewer-level", levelChanged);

/** The rows of the list: by severity, the diagnostics before the geometry hints of a severity; only one severity's when a count is on. */
function listed() {
  const all = [
    ...diagnosticList.map((item, i) => ({ item, geo: false, i })),
    ...hints.map((item, i) => ({ item, geo: true, i })),
  ].sort((a, b) => SEVERITY_ORDER[a.item.severity] - SEVERITY_ORDER[b.item.severity] || a.geo - b.geo || a.i - b.i);
  return filter ? all.filter((r) => r.item.severity === filter) : all;
}

/** Make the list and the counts again, from the first row. */
function refresh() {
  // a switch whose count is gone, such as the hints of a level that has none, is off
  if (filter && ![...diagnosticList, ...hints].some((r) => r.severity === filter)) filter = null;
  cancelAnimationFrame(frame);
  frame = 0;
  more = null;
  shown = 0;
  rows = listed();
  ul.replaceChildren();
  body.scrollTop = 0;
  if (diagnosticList.length === 0 && hints.length === 0) {
    const li = document.createElement("li");
    li.className = "nothing";
    setText(li, m("No errors, warnings or hints."));
    append(ul, li);
  }
  appendBatch();
  renderCounts();
}

/** One count of the heading, which is a switch: it shows only the rows of its severity, and all again. */
function count(kind, text) {
  const el = span(`n-${kind} chip${filter === kind ? " on" : ""}`, text);
  el.setAttribute("role", "button");
  el.tabIndex = 0;
  el.setAttribute("aria-pressed", String(filter === kind));
  setAttr(el, "title", m("Show only these; click again for all"));
  const toggle = (e) => {
    // the heading is a button that folds the problems: a switch in it does not
    e.preventDefault();
    e.stopPropagation();
    const focused = document.activeElement === el;
    filter = filter === kind ? null : kind;
    refresh();
    // the counts are made again: a switch that had the focus keeps it, for the keyboard
    if (focused) $("counts").querySelector(`.n-${kind}`)?.focus();
  };
  el.addEventListener("click", toggle);
  el.addEventListener("keydown", (e) => { if (e.key === "Enter" || e.key === " ") toggle(e); });
  return el;
}

function renderCounts() {
  const counts = $("counts");
  counts.replaceChildren();
  const every = [...diagnosticList, ...hints];
  for (const [s, one, many] of SEVERITIES) {
    const k = every.filter((r) => r.severity === s).length;
    if (k) append(counts, count(s, m(k === 1 ? one : many, k)));
  }
  if (every.length === 0) append(counts, span("none", m("none")));
}

// the levels of diagnostics (7.6), as a row names them
const SEVERITY = { error: tr("error"), warning: tr("warning"), hint: tr("hint") };

function diagnosticRow(d) {
  const li = document.createElement("li");
  li.className = `diag ${d.severity}`;
  append(li, span("sev", m(SEVERITY[d.severity] ?? d.severity)), span("msg", d.message));
  if (d.file) {
    const loc = span("loc", `${d.file}:${d.line}${d.col ? `:${d.col}` : ""}`);
    if (d.full_path) setAttr(loc, "title", d.full_path);
    append(li, loc);
  }
  if (d.origin) append(li, span("more", d.origin));
  if (d.cause) append(li, span("more", `upstream cause: ${d.cause}`));
  if (d.file_index !== null && d.file_index !== undefined && d.line) {
    li.classList.add("located");
    setAttr(li, "title", m("Show in the netlist"));
    li.addEventListener("click", () => revealLine(d.file_index, d.line, true));
  }
  return li;
}

/**
 * A geometry hint's row: its own severity, the "geo" tag and the core's message, without the file it begins with, which the
 * place below it names; a click shows the line in the text, and its kind is the tooltip.
 */
function hintRow(h) {
  const li = document.createElement("li");
  li.className = `diag geo ${h.severity}`;
  const message = h.file && h.message.startsWith(`${h.file}: `) ? h.message.slice(h.file.length + 2) : h.message;
  const msg = span("msg", "");
  msg.append(span("tag", "geo"), message);
  append(li, span("sev", m(SEVERITY[h.severity] ?? h.severity)), msg);
  if (h.file) append(li, span("loc", `${h.file}:${h.line}`));
  if (h.file_index !== null && h.file_index !== undefined && h.line) {
    li.classList.add("located");
    li.addEventListener("click", () => revealLine(h.file_index, h.line, true));
  }
  li.title = h.kind;
  return li;
}

/** Colour the line numbers of lines with diagnostics, by the most serious one; the geometry hints are not among them. */
export function markDiagnostics(list) {
  const worst = new Map();
  for (const d of list) {
    if (d.file_index === null || d.file_index === undefined || !d.line) continue;
    const key = `${d.file_index}:${d.line}`;
    const was = worst.get(key);
    if (!was || SEVERITY_ORDER[d.severity] < SEVERITY_ORDER[was]) worst.set(key, d.severity);
  }
  markSourceDiagnostics(worst);
}
