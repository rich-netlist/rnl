import { append, m, setAttr, setText, tr } from "./i18n.js";
// The constraints of the netlist, in the Constraints view of the Solve pane (solve.js): what the solver takes in after
// the structures are expanded, which the core gives with the Document (`Level.constraints`, built when the build asks
// for them: state.js `constraintsWanted`). One row for each fact, in the order the solver takes them (8.5): what it says,
// its priority, the structure it comes from, and, once the netlist is solved, what became of it, which the solve's report
// tells. A click on a row lights up the objects it acts on and the usages it comes from in the drawing, as pointing at a
// row of the Usage view does (usages.js); a click on its place shows it in the netlist. Experimental, as the core's
// interface is.

import { clearSelection } from "./selection.js";
import { setPane } from "./panels.js";
import { current, doc } from "./state.js";
import { revealLine } from "./text.js";
import { highlightConstraint } from "./usages.js";
import { $, span, svgElement } from "./util.js";

/**
 * What the mark of a row says, from what became of the fact (`status`, which the solve's resolution decided) and whether
 * it holds on the drawing as solved (`holds`, measured, which a solve's report gives apart from the status: true, false,
 * or null when it cannot be measured). The mark is told by the status first, and `holds` only for a fact that is kept:
 * ✓ kept and it holds; ✗ kept and it does not, the solver's own fault; ! discarded (it conflicts with others) or failed
 * (it contradicts itself); ? undecided, even when the drawing happens to satisfy it, which the tooltip says; none for
 * kept and not measurable, whose tooltip says so. `tip` is a key of the catalog, to be translated where it is shown.
 */
const MEASURED_TIPS = {
  true: tr("Undecided: the check did not finish. Measured on the drawing, it holds."),
  false: tr("Undecided: the check did not finish. Measured on the drawing, it does not hold."),
  null: tr("Undecided: the check did not finish. It cannot be measured on the drawing."),
};
export function statusMark(status, holds = null) {
  switch (status) {
    case "kept":
      if (holds === true) return { glyph: "✓", kind: "holds", tip: tr("Kept, and it holds on the drawing.") };
      if (holds === false) return { glyph: "✗", kind: "broken", tip: tr("Kept, but it does not hold on the drawing: a fault of the solver.") };
      return { glyph: "", kind: "unmeasured", tip: tr("Kept; it cannot be measured on the drawing.") };
    case "discarded":
      return { glyph: "!", kind: "dropped", tip: tr("Discarded: it conflicts with other constraints.") };
    case "failed":
      return { glyph: "!", kind: "failed", tip: tr("Failed: the constraint contradicts itself.") };
    case "undecided":
      return { glyph: "?", kind: "undecided", tip: MEASURED_TIPS[String(holds ?? null)] };
    default:
      return null;
  }
}

/** Whether any level of a Document has its constraints: the build was asked for them. A package file has none either way. */
export const hasConstraints = (d) => !!d && d.levels.some((l) => l.constraints);

/** The objects of the constraints' list a fact acts on, by their index there. */
export function factObjects(fact) {
  const out = new Set();
  const visit = (value, bare) => {
    if (Array.isArray(value)) value.forEach((v) => visit(v, bare));
    else if (typeof value === "number") { if (bare) out.add(value); }
    else if (value && typeof value === "object") {
      if (typeof value.origin === "number") out.add(value.origin);
      else if (typeof value.object === "number") out.add(value.object);
      else if (value.centre) visit(value.centre, false);
    }
  };
  for (const key of ["members", "at", "from", "to"]) visit(fact[key], fact.kind === "ORIENT-RELATION" && key !== "at");
  for (const key of ["object", "point", "text"]) if (typeof fact[key] === "number") out.add(fact[key]);
  return out;
}

/** The report's level that is the solve of this constraints' list: the same host and the same input; null when none. */
function reportOf(level, report) {
  const c = level.constraints;
  return report?.levels.find((r) => r.host === level.host && r.input === c.input) ?? null;
}

/** The place of a fact among the files of the Document, as the report gives its own: null when it has none. */
function placeOf(source) {
  const file = source ? doc.files.findIndex((f) => f.path === source.file) : -1;
  return file < 0 ? null : { file_index: file, line: source.line, col: source.col };
}

/**
 * The rows of one level, from its constraints and, when the report is of the same input, what became of each fact:
 * null when the level has no facts.
 */
export function levelRows(level, report) {
  const c = level.constraints;
  if (!c || c.facts.length === 0) return null;
  const lines = reportOf(level, report)?.facts;
  return c.facts.map((fact, k) => {
    const line = lines?.[k] ?? null;
    const chain = fact.chain.map((u) => c.usages[u]).filter(Boolean);
    const first = chain[0];
    const named = first && typeof first.reference === "string" ? ` ${first.reference}` : "";
    const handle = typeof fact.reference === "string" ? `${fact.reference} = ` : "";
    return {
      rank: k,
      what: `${handle}${fact.what}`,
      priority: fact.priority,
      structure: first ? `${first.definition}${named}` : null,
      chain: chain.map((u) => u.definition).join(" › "),
      status: line?.status ?? null,
      holds: line?.holds ?? null,
      why: line?.why ?? null,
      at: placeOf(fact.source) ?? line?.at ?? null,
      objects: new Set([...factObjects(fact)].flatMap((o) => c.objects[o]?.objects ?? [])),
      usages: new Set(fact.chain.map((u) => c.usages[u]?.object).filter((o) => o !== null && o !== undefined)),
    };
  });
}

let selectedRow = null; // the row clicked, its element
let report = null; // the solve shown, which tells what became of the facts
let view = "constraints";

/** Forget the clicked row, and what it lit in the drawing. */
function unselect() {
  selectedRow?.classList.remove("selected");
  selectedRow?.setAttribute("aria-pressed", "false");
  selectedRow = null;
}

function rowElement(row, levelIndex) {
  const li = document.createElement("li");
  li.className = `solve-line constraint${row.status ? ` ${row.status}` : ""}`;
  li.tabIndex = 0;
  li.setAttribute("role", "button");
  li.setAttribute("aria-pressed", "false");
  li.dataset.text = [row.rank, row.what, row.priority, row.structure || "", row.status || ""].join(" ").toLowerCase();
  const detail = document.createElement("span");
  detail.className = "detail";
  append(detail, span("pri", row.priority));
  append(detail, span("struct", row.structure || m("written in the netlist")));
  const status = row.status ? statusMark(row.status, row.holds) : null;
  if (status) {
    const mark = span(`st ${status.kind}`, status.glyph);
    setAttr(mark, "title", m(status.tip));
    setAttr(mark, "aria-label", m(status.tip));
    mark.setAttribute("role", "img");
    append(detail, mark);
  }
  if (row.at) {
    const where = document.createElement("button");
    where.type = "button";
    where.className = "where";
    // an arrow to the line, as the other icon buttons are drawn: no number, so the name is the label
    const arrow = svgElement("svg", { viewBox: "0 0 16 16", "aria-hidden": "true" });
    arrow.append(svgElement("path", { d: "M2.5 8H11M7.5 4.5 11 8 7.5 11.5M13.5 3v10", fill: "none", stroke: "currentColor", "stroke-width": 1.6, "stroke-linecap": "round", "stroke-linejoin": "round" }));
    where.append(arrow);
    setAttr(where, "title", m("Show in the netlist"));
    setAttr(where, "aria-label", m("Show in the netlist"));
    where.addEventListener("click", (e) => {
      e.stopPropagation();
      setPane("netlist");
      revealLine(row.at.file_index, row.at.line, true);
    });
    append(detail, where);
  }
  append(li, span("rank", `[${row.rank}]`), span("msg", row.what), detail);
  if (row.chain) setAttr(li, "title", row.chain);
  if (row.why) append(li, span("more", row.why));
  const choose = () => {
    if (selectedRow === li) {
      unselect();
      highlightConstraint(null);
      return;
    }
    clearSelection();
    unselect();
    selectedRow = li;
    li.classList.add("selected");
    li.setAttribute("aria-pressed", "true");
    // only the level shown has its objects in the drawing
    highlightConstraint(levelIndex === current ? { objects: row.objects, usages: row.usages } : null);
  };
  li.addEventListener("click", choose);
  li.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") { e.preventDefault(); choose(); }
  });
  return li;
}

/** Show the rows the filter lets through, and how many of them there are. */
function applyFilter() {
  const words = $("constraints-filter").value.toLowerCase().split(/\s+/).filter(Boolean);
  let shown = 0;
  let all = 0;
  for (const li of $("constraints-list").querySelectorAll("li.constraint")) {
    all++;
    const ok = words.every((w) => li.dataset.text.includes(w));
    li.hidden = !ok;
    if (ok) shown++;
  }
  for (const section of $("constraints-list").querySelectorAll("section.constraints-level")) {
    section.hidden = section.querySelectorAll("li.constraint:not([hidden])").length === 0;
  }
  setText($("constraints-count"), words.length ? m`${shown} of ${all} constraints` : all === 1 ? m("1 constraint") : m`${all} constraints`);
}

/** Show the Constraints view or the report of the Solve pane; the report only when there is a solve to report. */
function pick(next) {
  view = next === "report" && report ? "report" : "constraints";
  const constraints = view === "constraints" && !$("solve-constraints").dataset.empty;
  $("solve-report").hidden = constraints;
  $("solve-constraints").hidden = !constraints;
  $("solve-view-constraints").setAttribute("aria-pressed", String(constraints));
  $("solve-view-report").setAttribute("aria-pressed", String(!constraints));
}

/**
 * List the constraints of the Document shown, with what became of each when a solve is shown (`solved`, the report, or
 * null). Called when the Document changes while the Solve pane is in use, and when a solve is shown or closed.
 */
export function showConstraints(solved = report) {
  if (solved !== report) view = "constraints";
  report = solved;
  unselect();
  highlightConstraint(null);
  const list = $("constraints-list");
  list.replaceChildren();
  const sections = doc ? doc.levels.map((level, index) => ({ level, index, rows: levelRows(level, report) })).filter((s) => s.rows) : [];
  for (const { level, index, rows } of sections) {
    const section = document.createElement("section");
    section.className = "constraints-level";
    const head = document.createElement("h3");
    append(head, span("title", level.title), span("kind", level.kind === "subckt" ? m("subcircuit") : m("file level")));
    const ul = document.createElement("ul");
    ul.className = "solve-lines";
    append(ul, ...rows.map((row) => rowElement(row, index)));
    append(section, head, ul);
    append(list, section);
  }
  // with no rows the Constraints view has nothing to show, and the report's place says why
  if (sections.length === 0) $("solve-constraints").dataset.empty = "1";
  else delete $("solve-constraints").dataset.empty;
  $("solve-views").hidden = sections.length === 0;
  $("solve-view-report").hidden = !report;
  applyFilter();
  pick(view === "report" && report ? "report" : "constraints");
  return sections.length > 0;
}

/** Show the solve's report, or the Constraints view again when the report is gone. */
export function showReportView() {
  pick("report");
}

$("solve-view-report").addEventListener("click", () => pick("report"));
$("solve-view-constraints").addEventListener("click", () => pick("constraints"));
$("constraints-filter").addEventListener("input", applyFilter);
// a selection in the drawing, another level or another Document ends what a row lit
window.addEventListener("constraint-cleared", unselect);
