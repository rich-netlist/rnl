// The problems list: the errors, warnings and hints, the most serious first; clicking one shows
// its line in the text. The line numbers of the lines they are on are coloured.

import { fileViews, revealLine } from "./text.js";
import { $, span } from "./util.js";

const SEVERITY_ORDER = { error: 0, warning: 1, hint: 2 };

export function renderDiagnostics(list) {
  const sorted = list
    .map((d, i) => ({ d, i }))
    .sort((a, b) => SEVERITY_ORDER[a.d.severity] - SEVERITY_ORDER[b.d.severity] || a.i - b.i)
    .map((x) => x.d);
  const ul = $("diags");
  ul.replaceChildren();
  for (const d of sorted) {
    const li = document.createElement("li");
    li.className = `diag ${d.severity}`;
    li.append(span("sev", d.severity), span("msg", d.message));
    if (d.file) {
      const loc = span("loc", `${d.file}:${d.line}${d.col ? `:${d.col}` : ""}`);
      if (d.full_path) loc.title = d.full_path;
      li.append(loc);
    }
    if (d.origin) li.append(span("more", d.origin));
    if (d.cause) li.append(span("more", `upstream cause: ${d.cause}`));
    if (d.file_index !== null && d.file_index !== undefined && d.line) {
      li.classList.add("located");
      li.title = "Show in the netlist";
      li.addEventListener("click", () => revealLine(d.file_index, d.line, true));
    }
    ul.append(li);
  }
  if (sorted.length === 0) {
    const li = document.createElement("li");
    li.className = "nothing";
    li.textContent = "No errors, warnings or hints.";
    ul.append(li);
  }
  const counts = $("counts");
  counts.replaceChildren();
  const n = (s) => list.filter((d) => d.severity === s).length;
  for (const [s, one, many] of [["error", "error", "errors"], ["warning", "warning", "warnings"], ["hint", "hint", "hints"]]) {
    const k = n(s);
    if (k) counts.append(span(`n-${s}`, `${k} ${k === 1 ? one : many}`));
  }
  if (list.length === 0) counts.append(span("none", "none"));
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
  for (const [key, sev] of worst) {
    const [f, line] = key.split(":").map(Number);
    const el = fileViews[f] && fileViews[f].lines[line - 1];
    if (el) el.classList.add(`d-${sev}`);
  }
}
