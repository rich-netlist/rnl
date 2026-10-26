import { append, m, setAttr, setText, tr } from "./i18n.js";
// The problems list: the errors, warnings and hints, the most serious first; clicking one shows
// its line in the text. The line numbers of the lines they are on are coloured.

import { markSourceDiagnostics, revealLine } from "./text.js";
import { $, span } from "./util.js";
import { shell } from "./shell.js";
import { includePublicPackage } from "./pool.js";
import { matchingPackage, missingPackage } from "./public-packages.js";

const SEVERITY_ORDER = { error: 0, warning: 1, hint: 2 };
let renderVersion = 0;
const BATCH = 100;
const body = $("problems-body"), ul = $("diags");
let diagnostics = [], shown = 0, more = null, frame = 0;

// Thousands of diagnostics must not all take part in the first layout after navigation.
// Keep every diagnostic in the model and append another batch as the list is read.
function appendBatch() {
  more?.remove();
  more = null;
  const fragment = document.createDocumentFragment();
  const end = Math.min(shown + BATCH, diagnostics.length);
  for (; shown < end; shown++) {
    const d = diagnostics[shown];
    append(fragment, diagnosticRow(d));
    const hint = packageHint(d, renderVersion);
    if (hint) append(fragment, hint);
  }
  if (shown < diagnostics.length) {
    more = document.createElement("li");
    more.className = "diagnostics-more";
    const button = document.createElement("button");
    button.type = "button";
    setText(button, m`${shown} of ${diagnostics.length} shown: scroll for more`);
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
    setText(message, m("Add the package file to Library or a PATH directory; public packages are listed on the website's "));
    const link = document.createElement("a");
    link.href = "https://rich-netlist.github.io/rnl/en/packages.html";
    setText(link, m("Packages page"));
    link.addEventListener("click", (e) => { e.preventDefault(); shell.browse?.(link.href); });
    append(message, link, ".");
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

export function renderDiagnostics(list) {
  ++renderVersion;
  diagnostics = list
    .map((d, i) => ({ d, i }))
    .sort((a, b) => SEVERITY_ORDER[a.d.severity] - SEVERITY_ORDER[b.d.severity] || a.i - b.i)
    .map((x) => x.d);
  cancelAnimationFrame(frame);
  frame = 0;
  more = null;
  shown = 0;
  ul.replaceChildren();
  body.scrollTop = 0;
  appendBatch();
  if (diagnostics.length === 0) {
    const li = document.createElement("li");
    li.className = "nothing";
    setText(li, m("No errors, warnings or hints."));
    append(ul, li);
  }
  const counts = $("counts");
  counts.replaceChildren();
  const n = (s) => list.filter((d) => d.severity === s).length;
  for (const [s, one, many] of [["error", tr("{0} error"), tr("{0} errors")], ["warning", tr("{0} warning"), tr("{0} warnings")],
    ["hint", tr("{0} hint"), tr("{0} hints")]]) {
    const k = n(s);
    if (k) append(counts, span(`n-${s}`, m(k === 1 ? one : many, k)));
  }
  if (list.length === 0) append(counts, span("none", m("none")));
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

/** Colour the line numbers of lines with diagnostics, by the most serious one. */
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
