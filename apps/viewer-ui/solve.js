// The Solve menu of the desktop viewer (decision 0010): the netlist shown solved as `rnl solve`
// solves it, without its records of the solver, by the shell on a thread of its own, with its
// progress and a Cancel button in the status bar; the result drawn as a preview over the file,
// which stays as it is; a report of what became of each fact in the Solve view of the side panel,
// every line of it shown in the netlist when clicked; and a copy of the files solved saved where
// the user says. The solver's options are Solve > Settings (solve-settings.js). A shell without
// `solve`, such as the web's, offers none of it.

import { inform, modal } from "./dialog.js";
import { changes, unappliedPaths } from "./edits.js";
import { editing, stopEditing } from "./editor.js";
import { currentSource, isLoading, openSource, showBuilt } from "./open.js";
import { setPane } from "./panels.js";
import { shell } from "./shell.js";
import { solverSettings } from "./solve-settings.js";
import { doc } from "./state.js";
import { revealLine } from "./text.js";
import { $, span } from "./util.js";

/** What a shell rejects with when the solve was cancelled (rnl_cli::solve::CANCELLED). */
export const CANCELLED = "the solve was cancelled";

let shown = null; // the preview shown: { source, report }, or null
let running = false; // whether a solve is under way
let progressHooked = false;

/** Whether the page shows a solve, not the file as written. */
export function previewing() {
  return shown !== null;
}

/**
 * Solve the netlist shown (Solve > Solve, F9), or, while a preview is shown, solve its file
 * again; File > Reload does the same then. The file is read again first, so that the solve is
 * of the file as it is now.
 */
export async function solve() {
  if (currentSource()?.readonly) return;
  if (!shell || !shell.solve || running || modal() || isLoading()) return;
  const source = shown ? shown.source : currentSource();
  if (!source || !doc) {
    await inform({ title: "Solve", paragraphs: ["Open a netlist first: File › Open (Ctrl+O)."] });
    return;
  }
  hookProgress();
  if (editing && !await stopEditing()) return; // do not solve stale text if applying failed
  running = true;
  document.body.classList.add("solving");
  status(`Solving ${source.name}…`, true);
  try {
    const solved = await shell.solve(source, solverSettings(), changes(true));
    shown = { source, report: solved.report };
    showBuilt(solved.document, true);
    renderReport(solved.report);
    showPreview(true);
    const r = solved.report;
    const discarded = r.levels.reduce((n, l) => n + l.facts.filter((f) => f.status !== "kept").length, 0);
    const later = unappliedPaths().length > 0 ? " Changes not applied yet are not in it." : "";
    status(`Solved: ${r.levels.length} ${r.levels.length === 1 ? "level" : "levels"}, ${discarded} ${discarded === 1 ? "fact" : "facts"} not kept. A preview: the files are unchanged.${later}`, false);
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    if (message === CANCELLED) status("The solve was cancelled; the netlist is shown as it was.", false);
    else {
      status("", false);
      await inform({ title: "Solve", paragraphs: [`The netlist could not be solved: ${message}`] });
    }
  } finally {
    running = false;
    document.body.classList.remove("solving");
  }
}

/** Stop the solve under way (the Cancel button of the status bar). */
export function cancelSolve() {
  if (running && shell.cancelSolve) shell.cancelSolve();
}

/** Solve > Close Preview: show the netlist again, the files as read with the changes applied to them. */
export function closePreview() {
  if (!shown) return;
  const source = shown.source;
  openSource(source, true, changes(true));
}

/**
 * End the preview without reading anything: the page opens or reads a source again, which shows
 * it as written (open.js).
 */
export function endPreview() {
  if (!shown) return;
  shown = null;
  showPreview(false);
  status("", false);
}

/** Solve > Save Solved Copy As: a copy of the solve shown, where the user says. */
export async function saveSolved() {
  if (!shell || !shell.saveSolved || modal()) return;
  const title = "Save Solved Copy";
  if (!shown) {
    await inform({ title, paragraphs: ["Solve the netlist first: Solve › Solve (F9) shows the solve, and this saves it."] });
    return;
  }
  try {
    const said = await shell.saveSolved();
    if (said) await inform({ title, paragraphs: [said, "The files that were opened are unchanged."] });
  } catch (e) {
    await inform({ title, paragraphs: [`The copy was not saved: ${e instanceof Error ? e.message : String(e)}`] });
  }
}

/** Show each level of the solve under way in the status bar, once the shell can say. */
function hookProgress() {
  if (progressHooked || !shell.onSolveProgress) return;
  progressHooked = true;
  shell.onSolveProgress((step) => {
    if (!running) return;
    const again = step.again ? ", again, as raised to 0.3" : "";
    status(`Solving ${step.title} (${step.level + 1} of ${step.levels}${again})…`, true);
  });
}

/** The solve's line of the status bar, with the Cancel button while a solve is under way. */
function status(text, cancellable) {
  const el = $("solve-status");
  el.hidden = text === "";
  $("solve-step").textContent = text;
  $("solve-step").title = text;
  $("solve-cancel").hidden = !cancellable;
}

/** Show or hide what says that the drawing is a preview, and the report of the solve. */
function showPreview(on) {
  document.body.classList.toggle("preview", on);
  $("preview-badge").hidden = !on;
  $("preview-close").hidden = !on;
  $("tab-solve").hidden = !on;
  if (!on && !$("pane-solve").hidden) setPane("netlist");
  if (!on) $("solve-report").replaceChildren();
  if (on) setPane("solve");
}

/** A line of the report, which shows its place in the netlist when clicked. */
function line(cls, parts, at) {
  const li = document.createElement("li");
  li.className = cls;
  li.append(...parts);
  if (at) {
    li.classList.add("located");
    li.title = "Show in the netlist";
    li.addEventListener("click", () => {
      setPane("netlist");
      revealLine(at.file_index, at.line, true);
    });
  }
  return li;
}

/** A list of the report under a heading, folded when `open` is false; nothing when it is empty. */
function group(parent, heading, items, open = true) {
  if (items.length === 0) return;
  const details = document.createElement("details");
  details.className = "solve-group";
  details.open = open;
  const summary = document.createElement("summary");
  summary.textContent = `${heading} (${items.length})`;
  const ul = document.createElement("ul");
  ul.className = "solve-lines";
  ul.append(...items);
  details.append(summary, ul);
  parent.append(details);
}

const STATUS_ORDER = ["failed", "discarded", "undecided", "kept"];
const STATUS_HEADINGS = {
  failed: "Failed: they contradict themselves (8.5, 7.6)",
  discarded: "Discarded in the order of 8.5",
  undecided: "Undecided within the time limit (8.5)",
  kept: "Kept",
};

/** The report of a solve, in the Solve view of the side panel (crates/rnl-cli/src/preview.rs). */
function renderReport(report) {
  const box = $("solve-report");
  box.replaceChildren();
  const note = document.createElement("p");
  note.className = "solve-note";
  note.textContent =
    "A preview of the solve: nothing was written, and the files are as they were. Solve › Save Solved Copy As saves the files solved elsewhere; Solve › Close Preview shows the file as written.";
  box.append(note);
  const plain = (text, cls = "solve-line") => line(cls, [span("msg", text)], null);
  group(box, "Files", report.notes.map((n) => plain(n)));
  for (const level of report.levels) {
    const section = document.createElement("section");
    section.className = "solve-level";
    const head = document.createElement("h3");
    head.append(span("title", level.title), span("kind", level.kind === "subckt" ? "subcircuit" : "file level"));
    const summary = document.createElement("p");
    summary.className = "solve-summary";
    summary.textContent = level.summary;
    section.append(head, summary);
    for (const status of STATUS_ORDER) {
      const facts = level.facts.filter((f) => f.status === status);
      const items = facts.map((f) => {
        const parts = [span("st", status), span("msg", f.text)];
        if (f.why) parts.push(span("more", f.why));
        return line(`solve-line fact ${status}`, parts, f.at);
      });
      group(section, STATUS_HEADINGS[status], items, status !== "kept");
    }
    group(section, "Statements whose facts took part in discarding (8.5)", level.statements.map((s) => line("solve-line", [span("msg", s.text)], s.at)));
    group(section, "Errors", level.errors.map((e) => plain(e, "solve-line error")));
    group(section, "Warnings", level.warnings.map((w) => plain(w, "solve-line warning")));
    group(section, "What the record cannot hold", level.unsolved.map((u) => plain(u)));
    group(section, "ROUTEs left in parts", level.unrouted.map((u) => plain(u)));
    group(section, "Notes", level.notes.map((n) => plain(n)), false);
    box.append(section);
  }
  const counts = $("solve-counts");
  counts.replaceChildren();
  const n = (s) => report.levels.reduce((k, l) => k + l.facts.filter((f) => f.status === s).length, 0);
  for (const [s, cls] of [["failed", "n-error"], ["discarded", "n-warning"], ["undecided", "n-warning"]]) {
    const k = n(s);
    if (k) counts.append(span(cls, `${k} ${s}`));
  }
  if (counts.childElementCount === 0) counts.append(span("none", "every fact kept"));
}

$("solve-cancel").addEventListener("click", cancelSolve);
$("preview-close").addEventListener("click", closePreview);
