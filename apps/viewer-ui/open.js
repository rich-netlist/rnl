// Opening a document: the shell reads a source and builds its Document, and the page shows it.
// Reading the same source again keeps the level, the hierarchy path to it while its instances are
// still there, the view, the selection and the text's scroll.

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
