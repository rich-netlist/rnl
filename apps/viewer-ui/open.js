import { append, m, setAttr, setText, tr } from "./i18n.js";
// Opening a document: the shell reads a source and builds its Document, and the page shows it.
// Reading the same source again keeps the level, the visit history to it while its circuits are
// still there, the view, the selection and the text's scroll. File > Load from Clipboard opens the
// clipboard's text, once the user has seen it, as a source that holds its text. A solve shown as a
// preview (solve.js) ends when a source is opened or read again, which shows it as written; while
// one is shown, File > Reload solves again. Text edited in the page (editor.js) is built over the
// files with Apply Changes; opening another file or reading it again asks first about what is not
// saved.

import { form, inform, modal } from "./dialog.js";
import { sourceOpened } from "./pool.js";
import { adoptBuffers, changes, resetBuffers } from "./edits.js";
import { confirmLeave } from "./editor.js";
import { shell } from "./shell.js";
import { bindings, current, doc, expanded, selected, selectedAsUsage, setDoc, setExpanded, setSelected, setViewCache, viewCache } from "./state.js";
import { clearDrawing, notice, showLevel, view } from "./drawing.js";
import { renderHierarchy, rootPath } from "./hierarchy.js";
import { setProblems } from "./panels.js";
import { markDiagnostics, renderDiagnostics } from "./problems.js";
import { startLevel, uncalledRoots } from "./roots.js";
import { clearSelection, select } from "./selection.js";
import { endPreview, previewing, solve } from "./solve.js";
import { levelNamed, namedPath, restorePath, visitView } from "./subcircuits.js";
import { clearSource, fileShown, fileViews, renderFiles, revealDefinition, showFile } from "./text.js";
import { $ } from "./util.js";
import { setEnabled } from "./commands.js";
import { startLoading, nextPaint } from "./loading.js";
import { offerMissingPackages } from "./package-offers.js";

let lastSource = null; // what was last opened, also when it failed to open, for reloading
let loading = false;
let operation = null;

/** API imports and ordinary opens/rebuilds share ownership of the displayed document. */
export function beginOpenOperation() {
  operation?.abort();
  operation = new AbortController();
  loading = false;
  document.body.classList.remove("busy");
  return operation;
}

/**
 * Open a source, or read it again with `keep`. Opening another one asks first about changes not
 * saved. `edited` are texts to build over the files, by the path the Document names them by (a
 * preview closed shows the text applied); without them the files are read as they are, and the
 * text edited is what they hold. `confirmed` when the user was asked already (Open Project). With
 * `options.bind`, the parameters of a subcircuit are built with those values, to show, and `options.bound`
 * is what the page keeps of that for the Document (params.js).
 */
export async function openSource(source, keep, edited, fresh = false, navigation = null, signal = null, confirmed = false, options = {}) {
  if (!source || loading) return;
  // Initial API level selection is part of its import, not a competing user navigation.
  const operation = signal ? { signal } : beginOpenOperation();
  if (!keep && !edited && !confirmed && !(await confirmLeave(m("open another file"), operation.signal))) return;
  if (operation.signal.aborted) return;
  if (loading) return;
  loading = true;
  document.body.classList.add("busy");
  const progress = startLoading();
  let d;
  try {
    await progress.stage("reading");
    d = await shell.build(source, edited, stage => {
      if (operation.signal.aborted) throw new Error('Opening superseded');
      progress.setStage(stage);
    }, undefined, operation.signal, { fresh, bind: options.bind });
    if (operation.signal.aborted) return;
    if (options.bound) bindings.set(d, options.bound);
    await progress.stage("displaying");
    if (operation.signal.aborted) return;
    lastSource = source;
    endPreview();
    if (edited) adoptBuffers(d);
    else resetBuffers(d);
    showBuilt(d, keep, navigation);
    await sourceOpened(source, d, !keep, !!edited);
    await nextPaint();
    return true;
  } catch (e) {
    if (operation.signal.aborted) return;
    lastSource = source;
    progress.finish();
    if (keep && edited) {
      await inform({ title: m("Rebuild"), paragraphs: [m`The document could not be rebuilt: ${e}. Your edits are still kept.`] });
      return;
    }
    endPreview();
    resetBuffers(null);
    setDoc(null);
    clearDrawing(m`Could not open ${source.name}`);
    clearSource(m("The file could not be read."));
    renderHierarchy();
    renderDiagnostics([{ severity: "error", message: String(e), file: "", line: 0, col: 0, origin: "" }]);
    setProblems(true);
    await sourceOpened(source, null, false);
  } finally {
    if (d) shell.discardBuild?.(d);
    progress.finish();
    if (!operation.signal.aborted) {
      loading = false;
      document.body.classList.remove("busy");
      if (d === doc) void offerMissingPackages(d, source, operation.signal);
    }
  }
}

export async function pickAndOpen(kind) {
  if (loading) return;
  const source = await shell.pick(kind);
  if (source) await openSource(source, false);
}

export async function reload() {
  if (previewing()) solve();
  else if (lastSource && (await confirmLeave(m("read the file again")))) openSource(lastSource, true, undefined, true);
}

/**
 * Edit > Apply Changes: build the Document again from the texts the editor holds (edits.js) over
 * the files as they are, as opening them would, and show it, with the level, the view and the
 * selection kept. True when it was shown; a message tells why not.
 */
export async function applyEdits() {
  if (!lastSource || !doc || loading || previewing()) return false;
  const operation = beginOpenOperation();
  loading = true;
  document.body.classList.add("busy");
  const progress = startLoading();
  let failed = null;
  let d;
  try {
    await progress.stage("reading");
    d = await shell.build(lastSource, changes(false), progress.setStage, undefined, operation.signal);
    if (operation.signal.aborted) return false;
    await progress.stage("displaying");
    if (operation.signal.aborted) return false;
    adoptBuffers(d);
    showBuilt(d, true);
    await sourceOpened(lastSource, d, false, true);
    await nextPaint();
  } catch (e) {
    if (operation.signal.aborted) return false;
    failed = e instanceof Error ? e.message : String(e);
  } finally {
    if (d) shell.discardBuild?.(d);
    progress.finish();
    if (!operation.signal.aborted) {
      loading = false;
      document.body.classList.remove("busy");
      if (d === doc) void offerMissingPackages(d, lastSource, operation.signal);
    }
  }
  if (failed !== null) {
    renderDiagnostics([...doc.diagnostics, { severity: "error", message: m`The changes could not be applied: ${failed}`, file: "", line: 0, col: 0, origin: "" }]);
    setProblems(true);
  }
  return failed === null;
}

/** What was last opened, also when it failed to open; null before anything was. */
export function currentSource() {
  return lastSource;
}

/** A shell that builds its own opening flow (the web embed) publishes the displayed source here. */
export function adoptSource(source) {
  lastSource = source;
}

/** Whether a source is being read. */
export function isLoading() {
  return loading;
}

/**
 * Show a Document built for the source last opened, such as its solve (solve.js); with `keep`,
 * with what reading the same source again keeps: the level, the view, the selection and the text's
 * scroll.
 */
export function showBuilt(d, keep, navigation = null) {
  shell.commitBuild?.(d);
  if (shell.platform === "web") document.title = `${d.name} - RNL Viewer`;
  const before = keep && doc ? snapshot() : null;
  setDoc(d);
  showDocument(before, navigation);
  // a file opened, not one read again: a phone shows its drawing
  if (!keep) window.dispatchEvent(new Event("viewer-navigate"));
}

/** What a text from the clipboard can be read as, by the extension of the file it is opened as. */
const KINDS = [
  ["sp", m("SPICE netlist")],
  ["scs", m("Spectre netlist")],
  ["rnl", m("RNL package, checked on its own")],
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
  const title = m("Load from Clipboard");
  let text;
  try {
    if (!navigator.clipboard || !navigator.clipboard.readText) throw new Error(m("the browser does not allow this page to read the clipboard"));
    text = await navigator.clipboard.readText();
  } catch (e) {
    const why = (e instanceof Error && e.message ? e.message : String(e)).replace(/\.$/, "");
    const again = m("Allow the page to read the clipboard if the browser asks, and choose Load from Clipboard again.");
    await inform({ title, paragraphs: [m`The clipboard could not be read: ${why}.`, again] });
    return;
  }
  if (!text.trim()) {
    await inform({ title, paragraphs: [m("The clipboard holds no text to load.")] });
    return;
  }
  const kind = await preview(title, text);
  if (kind) await openSource({ name: `clipboard.${kind}`, text, temporary: true }, false);
}

/** Show the clipboard's text before it is loaded, and ask what to read it as; null when cancelled. */
function preview(title, text) {
  const lines = text.split(/\r?\n/);
  if (lines.length > 1 && lines[lines.length - 1] === "") lines.pop();
  const warning = document.createElement("p");
  warning.className = "dialog-warning";
  setText(warning, m("This text from the clipboard will be opened in place of the document shown. Check that it is the netlist you mean to load."));
  const box = document.createElement("pre");
  box.className = "clip-preview";
  box.tabIndex = 0;
  setAttr(box, "aria-label", m("The clipboard's text"));
  setText(box, lines.slice(0, PREVIEW_LINES).join("\n"));
  const size = document.createElement("p");
  size.className = "dialog-note";
  const more = lines.length > PREVIEW_LINES ? m`; first ${PREVIEW_LINES} lines shown` : "";
  const n = lines.length, c = text.length;
  setText(size, m`${n === 1 ? m("{0} line", n) : m("{0} lines", n)}, ${c === 1 ? m("{0} character", c) : m("{0} characters", c)}${more}.`);
  const set = document.createElement("fieldset");
  const legend = document.createElement("legend");
  setText(legend, m("Read it as"));
  append(set, legend);
  const guess = guessKind(text);
  const inputs = KINDS.map(([ext, label]) => {
    const input = document.createElement("input");
    input.type = "radio";
    input.name = "clipboard-kind";
    input.value = ext;
    input.checked = ext === guess;
    const choice = document.createElement("label");
    choice.className = "dialog-choice";
    append(choice, input, m`${label} (.${ext})`);
    append(set, choice);
    return input;
  });
  const alone = document.createElement("p");
  alone.className = "dialog-note";
  setText(alone, m("Opened without a folder as clipboard.sp, .scs or .rnl; included files are looked up in Library (and in PATH in the desktop viewer)."));
  return form({
    title,
    parts: [warning, box, size, set, alone],
    answer: () => inputs.find((i) => i.checked).value,
    action: tr("Load"),
    wide: true,
  });
}

/** What to keep when the same source is read again. */
function snapshot() {
  const sel = selected === null ? null : doc.objects[selected];
  if (fileViews[fileShown]) fileViews[fileShown].scroll = $("source").scrollTop;
  return {
    level: doc.levels[current] ? doc.levels[current].title : null,
    view,
    files: doc.files.map((f, k) => {
      const v = fileViews[k];
      return { path: f.path, text: f.text, scroll: v?.scroll || 0,
        pendingLine: v?.pendingLine ?? (v?._model && v.filtered ? v.lineAt(Math.max(0, Math.floor((v.scroll - 4 + 1) / v.height))) : undefined),
        collapsed: [...(v?.collapsed || [])] };
    }),
    file: doc.files[fileShown] ? doc.files[fileShown].path : null,
    selected: sel && { kind: sel.kind, name: sel.name, level: sel.level, usage: selectedAsUsage },
    expanded,
    roots: uncalledRoots().map(rootPath),
    path: namedPath(),
    viewCache,
  };
}

function showDocument(before, navigation) {
  setSelected(null);
  setEnabled("descend-new-window", false);
  // the trees of the subcircuits that the top level does not call start open, also those that a
  // document read again gained
  const roots = uncalledRoots().map(rootPath);
  const gained = before ? roots.filter((r) => !before.roots.includes(r)) : roots;
  setExpanded(new Set([...(before ? before.expanded : ["0"]), ...gained]));
  setViewCache(before?.viewCache || navigation?.viewCache || new Map());
  const file = before ? doc.files.findIndex((f) => f.path === before.file) : -1;
  renderFiles(file >= 0 ? file : 0);
  const diags = doc.diagnostics.slice();
  if (doc.failed) diags.unshift({ severity: "error", message: doc.failed, file: "", line: 0, col: 0, origin: "" });
  renderDiagnostics(diags);
  markDiagnostics(diags);
  $("search").value = "";

  if (before) {
    for (const f of before.files) {
      const k = doc.files.findIndex((x) => x.path === f.path);
      if (k >= 0) {
        const v = fileViews[k];
        v.scroll = f.scroll;
        // A changed source can move its definitions. Preserve folds only for unchanged text.
        if (doc.files[k].text === f.text && f.collapsed.length) {
          v.collapsed = new Set(f.collapsed);
          if (v._model) { v.mapLines(); v.refresh(true); }
        }
        if (f.pendingLine !== undefined) v.restoreLine(f.pendingLine);
      }
    }
  }
  showFile(file >= 0 ? file : 0);

  if (doc.levels.length === 0) {
    clearDrawing(m("Nothing to draw."));
    renderHierarchy();
    return;
  }
  const wanted = before?.level || navigation?.level;
  const kept = wanted ? levelNamed(wanted) : -1;
  // the level it was, else the top level, or, when that has no objects, the first subcircuit that
  // nothing calls, with a line that says so
  const start = kept >= 0 ? { level: kept, note: "" } : startLevel();
  // the path by the names of its levels and instances, or else one that starts at the level
  restorePath(before?.path || navigation?.path || [], start.level);
  showLevel(start.level, kept >= 0 ? before?.view || visitView() : null);
  notice(start.note);

  let selectedHere = false;
  if (navigation?.reveal) revealDefinition(start.level);
  if (navigation?.select) {
    const s = navigation.select;
    const o = doc.objects.findIndex(o => o.kind === s.kind && o.name === s.name && o.level_title === s.level);
    if (o >= 0) { select(o, "ascend", { usage: s.usage }); selectedHere = true; }
  }

  if (before && before.selected) {
    const s = before.selected;
    const o = doc.objects.findIndex((x) => x.kind === s.kind && x.name === s.name && x.level === s.level);
    if (o >= 0) { select(o, "reload", { usage: s.usage }); selectedHere = true; }
  }
  // what was selected in another document is no selection of this one
  if (!selectedHere) clearSelection();
}
