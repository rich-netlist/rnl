import { append, m, setAttr, setText, tr } from "./i18n.js";
// The problems list: the errors, warnings and hints, the most serious first; clicking one shows
// its line in the text. The line numbers of the lines they are on are coloured. The geometry hints of
// the level shown (`Level.hints`, what `rnl check` prints about the drawing) are in the list too, each
// with a small "geo" tag that says where it comes from and its own severity, "warning" or "hint". The
// counts of the heading are by severity alone, as `rnl check` counts, so a geometry hint is counted in
// its severity. Each count is a switch: it shows only the rows of that severity, and again shows all.
// A row is one line, as in an IDE: its severity, the number the toolkit gives its kind of problem (`code`, such as RNL3120),
// the kind's short name (`name`, such as param-wrong-type) and the line; the explanation, the message, is below it, when
// the row is open. A click on a row shows its line in the text and opens or closes the row; the twisty only opens or closes.
// (The levels are the one list SEVERITIES, which the violation of 0.4, between the error and the warning, is in.)

import { fileShown, markSourceDiagnostics, revealLine } from "./text.js";
import { current, doc } from "./state.js";
import { $, span } from "./util.js";
import { shell } from "./shell.js";
import { includePublicPackage } from "./pool.js";
import { matchingPackage, missingPackage } from "./public-packages.js";

/** The severities, the most serious first: the order of the list, and of the counts in the heading. */
const SEVERITIES = [
  ["error", tr("{0} error"), tr("{0} errors")],
  ["violation", tr("{0} violation"), tr("{0} violations")],
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
const SEVERITY = { error: tr("error"), violation: tr("violation"), warning: tr("warning"), hint: tr("hint") };

/**
 * The place of a row, as its line shows it: the line and the column, and the name of the file too when it is not the one the
 * text pane shows, since the line is of the file that the problems follow, the one in the text pane.
 */
function placeOf(file, fileIndex, line, col) {
  const at = line ? `${line}${col ? `:${col}` : ""}` : "";
  if (fileIndex === fileShown || !file) return at;
  // a long name is cut in the middle, so that the name of the problem keeps its room; the whole path is the tooltip
  const name = file.length > 22 ? `${file.slice(0, 8)}…${file.slice(-12)}` : file;
  return at ? `${name}:${at}` : name;
}

/** Say again where the rows are, as the text pane shows another file. */
function placesAgain() {
  for (const loc of ul.querySelectorAll(".loc")) if (loc.place) loc.textContent = placeOf(...loc.place);
}
window.addEventListener("viewer-file", placesAgain);

const KIND_TIP = () => m("The kind of problem: its number and its short name");

/** Open a row, or close it: its explanation is shown only when it is open. */
function toggle(li, open = !li.classList.contains("open")) {
  li.classList.toggle("open", open);
  li.querySelector(".twisty").setAttribute("aria-expanded", String(open));
}

/**
 * A row of the list, in one line: a twisty, the severity, a tag if it has one (a geometry hint's), the kind's number and short
 * name, and the place; with the explanation, `details`, below. Without a kind, the line has the message in the place of the
 * short name, cut to the width. A click on the row, but on its explanation, which is for selecting, shows `where` in the
 * text and opens or closes the row; one on the twisty opens or closes it only.
 */
function problemRow({ severity, tag, code, name, message, file, fileIndex, line, col, fullPath, details, where }) {
  const li = document.createElement("li");
  li.className = `diag ${tag ? "geo " : ""}${severity}`;
  const twisty = document.createElement("button");
  twisty.type = "button";
  twisty.className = "twisty";
  twisty.setAttribute("aria-expanded", "false");
  setAttr(twisty, "aria-label", m("Show or hide the explanation"));
  twisty.append(span("chevron", ""));
  twisty.firstChild.setAttribute("aria-hidden", "true");
  append(li, twisty, span("sev", m(SEVERITY[severity] ?? severity)), span("tag", tag ?? ""));
  const number = span("code", code ?? "");
  if (code) setAttr(number, "title", KIND_TIP());
  const short = span("name", name || message);
  if (!name) short.classList.add("plain"); // the message, as there is no kind to name
  setAttr(short, "title", name ? KIND_TIP() : message);
  const loc = span("loc", placeOf(file, fileIndex, line, col));
  loc.place = [file, fileIndex, line, col];
  if (fullPath || file) setAttr(loc, "title", fullPath || file);
  append(li, number, short, loc);
  const more = document.createElement("div");
  more.className = "details";
  more.append(span("msg", message), ...details.map((text) => span("more", text)));
  li.append(more);
  const located = where && fileIndex !== null && fileIndex !== undefined && line;
  if (located) li.classList.add("located");
  li.addEventListener("click", (e) => {
    if (e.target.closest(".details")) return;
    // a selection made in the line, such as the number to copy, is not a click
    if (!e.target.closest(".twisty") && window.getSelection().toString()) return;
    if (located && !e.target.closest(".twisty")) revealLine(fileIndex, line, true);
    toggle(li);
  });
  return li;
}

function diagnosticRow(d) {
  const details = [];
  if (d.origin) details.push(d.origin);
  if (d.cause) details.push(`upstream cause: ${d.cause}`);
  if (d.full_path) details.push(d.full_path);
  return problemRow({
    severity: d.severity, code: d.code, name: d.name, message: d.message, file: d.file, fileIndex: d.file_index, line: d.line,
    col: d.col, fullPath: d.full_path, details, where: true,
  });
}

/**
 * A geometry hint's row: its own severity, the "geo" tag, its kind, and its place; its explanation is the core's message
 * without the file it begins with, which the place names.
 */
function hintRow(h) {
  const message = h.file && h.message.startsWith(`${h.file}: `) ? h.message.slice(h.file.length + 2) : h.message;
  return problemRow({
    severity: h.severity, tag: "geo", code: h.code, name: h.kind, message, file: h.file, fileIndex: h.file_index, line: h.line,
    col: 0, details: [], where: true,
  });
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
