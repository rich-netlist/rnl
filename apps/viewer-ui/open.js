// Opening a document: the shell reads a source and builds its Document, and the page shows it.
// Reading the same source again keeps the level, the hierarchy path to it while its instances are
// still there, the view, the selection and the text's scroll. File > Load from Clipboard opens the
// clipboard's text, once the user has seen it, as a source that holds its text.

import { form, inform, modal } from "./dialog.js";
import { shell } from "./shell.js";
import { current, doc, expanded, selected, setDoc, setExpanded, setSelected, setViewCache, viewCache } from "./state.js";
import { clearDrawing, showLevel, view } from "./drawing.js";
import { renderHierarchy } from "./hierarchy.js";
import { setProblems } from "./panels.js";
import { markDiagnostics, renderDiagnostics } from "./problems.js";
import { select } from "./selection.js";
import { namedPath, restorePath } from "./subcircuits.js";
import { clearSource, fileShown, fileViews, renderFiles, showFile } from "./text.js";
import { $ } from "./util.js";

let lastSource = null; // what was last opened, also when it failed to open, for reloading
let loading = false;

export async function openSource(source, keep) {
  if (!source || loading) return;
  loading = true;
  document.body.classList.add("busy");
  try {
    lastSource = source;
    const d = await shell.build(source);
    const before = keep && doc ? snapshot() : null;
    setDoc(d);
    showDocument(before);
  } catch (e) {
    setDoc(null);
    clearDrawing(`Could not open ${source.name}`);
    clearSource("The file could not be read.");
    renderHierarchy();
    renderDiagnostics([{ severity: "error", message: String(e), file: "", line: 0, col: 0, origin: "" }]);
    setProblems(true);
  } finally {
    loading = false;
    document.body.classList.remove("busy");
  }
}

export async function pickAndOpen(kind) {
  if (loading) return;
  const source = await shell.pick(kind);
  if (source) await openSource(source, false);
}

export function reload() {
  if (lastSource) openSource(lastSource, true);
}

/** What a text from the clipboard can be read as, by the extension of the file it is opened as. */
const KINDS = [
  ["sp", "SPICE netlist"],
  ["scs", "Spectre netlist"],
  ["rnl", "RNL package, checked on its own"],
];
const PREVIEW_LINES = 200; // the lines of the clipboard's text that its preview shows at most

/**
 * What a text most likely is, as the extension of the file to open it as: "rnl" for a package,
 * whose records stand on lines of their own; "scs" for a Spectre netlist, which says so, has a
 * subcircuit with no dot before `subckt` or has records in `//` comments; else "sp", SPICE.
 */
export function guessKind(text) {
  if (/^[ \t]*@RNL\b/m.test(text)) return "rnl";
  if (/^[ \t]*(simulator[ \t]+lang[ \t]*=[ \t]*spectre\b|(inline[ \t]+)?subckt[ \t]|\/\/[ \t]*@RNL\b)/m.test(text)) {
    return "scs";
  }
  return "sp";
}

/**
 * File > Load from Clipboard: read the clipboard's text, show it and what it will be read as, and
 * when the user says so, open it in place of the document shown, alone: a file it includes is
 * missing. Reload builds the same text again.
 */
export async function loadClipboard() {
  if (loading || modal()) return;
  const title = "Load from Clipboard";
  let text;
  try {
    if (!navigator.clipboard || !navigator.clipboard.readText) throw new Error("the browser does not let the page read it");
    text = await navigator.clipboard.readText();
  } catch (e) {
    const why = (e instanceof Error && e.message ? e.message : String(e)).replace(/\.$/, "");
    const again = "Allow the page to read the clipboard if the browser asks, and choose Load from Clipboard again.";
    await inform({ title, paragraphs: [`The clipboard could not be read: ${why}.`, again] });
    return;
  }
  if (!text.trim()) {
    await inform({ title, paragraphs: ["The clipboard holds no text to load."] });
    return;
  }
  const kind = await preview(title, text);
  if (kind) await openSource({ name: `clipboard.${kind}`, text }, false);
}

/** Show the clipboard's text before it is loaded, and ask what to read it as; null when cancelled. */
function preview(title, text) {
  const lines = text.split(/\r?\n/);
  if (lines.length > 1 && lines[lines.length - 1] === "") lines.pop();
  const warning = document.createElement("p");
  warning.className = "dialog-warning";
  warning.textContent =
    "This text from the clipboard will be opened in place of the document shown. Check that it is the netlist you mean to load.";
  const box = document.createElement("pre");
  box.className = "clip-preview";
  box.tabIndex = 0;
  box.setAttribute("aria-label", "The clipboard's text");
  box.textContent = lines.slice(0, PREVIEW_LINES).join("\n");
  const size = document.createElement("p");
  size.className = "dialog-note";
  const more = lines.length > PREVIEW_LINES ? `; the first ${PREVIEW_LINES} are shown` : "";
  size.textContent = `${count(lines.length, "line")}, ${count(text.length, "character")}${more}.`;
  const set = document.createElement("fieldset");
  const legend = document.createElement("legend");
  legend.textContent = "Read it as";
  set.append(legend);
  const guess = guessKind(text);
  const inputs = KINDS.map(([ext, label]) => {
    const input = document.createElement("input");
    input.type = "radio";
    input.name = "clipboard-kind";
    input.value = ext;
    input.checked = ext === guess;
    const choice = document.createElement("label");
    choice.className = "dialog-choice";
    choice.append(input, `${label} (.${ext})`);
    set.append(choice);
    return input;
  });
  const alone = document.createElement("p");
  alone.className = "dialog-note";
  alone.textContent = "It is read alone, as clipboard.sp, .scs or .rnl: files that it includes are missing.";
  return form({
    title,
    parts: [warning, box, size, set, alone],
    answer: () => inputs.find((i) => i.checked).value,
    action: "Load",
    wide: true,
  });
}

/** A number of things, such as "1 line" or "12 lines". */
function count(n, thing) {
  return `${n} ${thing}${n === 1 ? "" : "s"}`;
}

/** What to keep when the same source is read again. */
function snapshot() {
  const sel = selected === null ? null : doc.objects[selected];
  if (fileViews[fileShown]) fileViews[fileShown].scroll = $("source").scrollTop;
  return {
    level: doc.levels[current] ? doc.levels[current].title : null,
    view,
    files: doc.files.map((f, k) => ({ path: f.path, scroll: fileViews[k] ? fileViews[k].scroll : 0 })),
    file: doc.files[fileShown] ? doc.files[fileShown].path : null,
    selected: sel && { kind: sel.kind, name: sel.name, level: sel.level },
    expanded,
    path: namedPath(),
    viewCache,
  };
}

function showDocument(before) {
  setSelected(null);
  setExpanded(before ? before.expanded : new Set(["0"]));
  setViewCache(before ? before.viewCache : new Map());
  renderFiles();
  const diags = doc.diagnostics.slice();
  if (doc.failed) diags.unshift({ severity: "error", message: doc.failed, file: "", line: 0, col: 0, origin: "" });
  renderDiagnostics(diags);
  markDiagnostics(diags);
  $("search").value = "";

  if (before) {
    for (const f of before.files) {
      const k = doc.files.findIndex((x) => x.path === f.path);
      if (k >= 0) fileViews[k].scroll = f.scroll;
    }
  }
  const file = before ? doc.files.findIndex((f) => f.path === before.file) : -1;
  showFile(file >= 0 ? file : 0);

  if (doc.levels.length === 0) {
    clearDrawing("Nothing to draw.");
    renderHierarchy();
    return;
  }
  const level = before ? doc.levels.findIndex((l) => l.title === before.level) : -1;
  // the path by the names of its levels and instances, or else one that starts at the level
  restorePath(before ? before.path : [], level >= 0 ? level : 0);
  showLevel(level >= 0 ? level : 0, level >= 0 ? before.view : null);

  if (before && before.selected) {
    const s = before.selected;
    const o = doc.objects.findIndex((x) => x.kind === s.kind && x.name === s.name && x.level === s.level);
    if (o >= 0) select(o, "reload");
  }
}
