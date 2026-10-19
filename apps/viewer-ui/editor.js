// Editing the text (decision 0011): the netlist pane can be switched to edit, with a text box for
// each file of the document, the tabs as they are. A change is shown as not applied until the user
// exits editing or chooses Apply and Exit Editing, which builds the Document again from the texts in memory, as
// opening the files would, and draws it; solving remains a separate action. Nothing is
// written to a file until the user saves (File > Save, and Save As on the desktop; on the web, Save
// downloads the file shown), and a document, or the window, is not left with changes unsaved without
// asking. A solve (solve.js) is of the text applied.

import { setCheck } from "./commands.js";
import { decide, inform, modal } from "./dialog.js";
import { applyEdits, currentSource, openSource } from "./open.js";
import * as edits from "./edits.js";
import { shell } from "./shell.js";
import { previewing } from "./solve.js";
import { doc } from "./state.js";
import { fileShown, fileViews, showFile } from "./text.js";
import { $ } from "./util.js";

const source = $("source");
const pane = $("pane-netlist");
const tabs = $("file-tabs");

/** Whether the netlist pane is switched to edit. */
export let editing = false;

const editors = new Map(); // by the file's path: { el, area, gutter, lines, caret, scroll }
let notified = false; // what the shell was last told about changes not saved
let noteTimer = 0;
let exiting = false;
const positions = new Map(); // per path: caret and first visible line, independent of line height

// A browser can round scrollTop to whole CSS pixels. Snap within one pixel of a line boundary,
// so a fractional line height does not move the viewport back one row on every mode change.
const firstLine = (scroll, height) => Math.max(0, Math.floor((scroll - 4 + 1) / height));

function keepPosition() {
  if (!doc) return;
  for (const [k, file] of doc.files.entries()) {
    const e = editors.get(file.path);
    const old = positions.get(file.path) || { caret: 0, line: 0 };
    if (editing && e) {
      const height = parseFloat(getComputedStyle(e.area).lineHeight) || 19;
      positions.set(file.path, { caret: e.area.selectionStart, line: firstLine(e.area.scrollTop, height) });
    } else {
      const v = fileViews[k];
      const height = v?.lines[0]?.getBoundingClientRect().height || 19;
      const scroll = k === fileShown ? source.scrollTop : v?.scroll || 0;
      positions.set(file.path, { ...old, line: firstLine(scroll, height) });
    }
  }
}

function restorePosition() {
  if (!doc) return;
  for (const [k, file] of doc.files.entries()) {
    const p = positions.get(file.path);
    if (!p) continue;
    const e = editors.get(file.path);
    if (editing && e) {
      e.area.setSelectionRange(p.caret, p.caret);
      e.area.scrollTop = 4 + p.line * (parseFloat(getComputedStyle(e.area).lineHeight) || 19);
      e.gutter.scrollTop = e.area.scrollTop;
    } else if (fileViews[k]) {
      const height = fileViews[k].lines[0]?.getBoundingClientRect().height || 19;
      fileViews[k].scroll = 4 + p.line * height;
      if (k === fileShown) source.scrollTop = fileViews[k].scroll;
    }
  }
}

/** The files shown in the pane, as their paths. */
const paths = () => (doc ? doc.files.map((f) => f.path) : []);

/** Make the text boxes of the files of the document, if the pane is in edit; those of a document read again start from the buffers. */
export function renderEditors() {
  const kept = new Map([...editors].map(([path, e]) => [path, { caret: e.area.selectionStart, scroll: e.area.scrollTop }]));
  const focused = [...editors].find(([, e]) => e.area === document.activeElement);
  for (const e of editors.values()) e.el.remove();
  editors.clear();
  source.classList.toggle("editing", editing);
  if (editing && doc) {
    for (const [k, path] of paths().entries()) {
      const e = makeEditor(path, doc.files[k].name);
      const was = kept.get(path) || { caret: 0, scroll: 0 };
      editors.set(path, e);
      source.append(e.el);
      e.area.value = edits.textOf(path);
      number(e);
      const caret = Math.min(was.caret, e.area.value.length);
      e.area.setSelectionRange(caret, caret);
      e.area.scrollTop = e.scroll = was.scroll;
      e.caret = caret;
    }
    // a text box that had the focus has it again, as the Document changed under it
    if (focused && editors.has(focused[0])) editors.get(focused[0]).area.focus({ preventScroll: true });
  }
  showFile(fileShown); // the lines or the text box of the file shown, as the mode says
  refreshEditing();
}

/** Take the text boxes away, as when no document is shown. */
export function clearEditors() {
  for (const e of editors.values()) e.el.remove();
  editors.clear();
  refreshEditing();
}

/** A file's text box with the numbers of its lines beside it. */
function makeEditor(path, name) {
  const el = document.createElement("div");
  el.className = "editor";
  el.hidden = true;
  const gutter = document.createElement("div");
  gutter.className = "gutter";
  gutter.setAttribute("aria-hidden", "true");
  const area = document.createElement("textarea");
  area.className = "area";
  area.readOnly = currentSource()?.readonly === true;
  area.spellcheck = false;
  area.autocomplete = "off";
  area.autocapitalize = "off";
  area.wrap = "off";
  area.setAttribute("aria-label", `Text of ${name}`);
  el.append(gutter, area);
  const e = { el, area, gutter, lines: 0, caret: 0, scroll: 0 };
  area.addEventListener("input", () => {
    edits.setText(path, area.value);
    number(e);
    refreshEditing();
  });
  area.addEventListener("scroll", () => {
    gutter.scrollTop = area.scrollTop;
    e.scroll = area.scrollTop;
  });
  area.addEventListener("blur", () => (e.caret = area.selectionStart));
  return e;
}

/** The numbers beside a text box, as many as it has lines. */
function number(e) {
  const lines = e.area.value.split("\n").length;
  if (lines === e.lines) return;
  e.lines = lines;
  e.gutter.textContent = Array.from({ length: lines }, (_, k) => k + 1).join("\n");
}

/** Show the text box of the file at index `f` when the pane is in edit, and no other. */
export function showEditor(f) {
  const path = doc && doc.files[f] ? doc.files[f].path : null;
  for (const [p, e] of editors) e.el.hidden = !editing || p !== path;
  const e = editors.get(path);
  if (editing && e) e.gutter.scrollTop = e.area.scrollTop;
}

/** Give the focus to the text box of the file shown, as when the user chooses its tab or begins to edit. */
export function focusEditor() {
  const path = doc && doc.files[fileShown] ? doc.files[fileShown].path : null;
  const e = editors.get(path);
  if (editing && e && !modal()) e.area.focus({ preventScroll: true });
}

/** Show a line of a file in its text box, selected, in the middle of it; the text box is not given the focus. */
export function revealInEditor(f, line) {
  const path = doc && doc.files[f] ? doc.files[f].path : null;
  const e = editors.get(path);
  if (!e) return;
  const text = e.area.value;
  let from = 0;
  for (let n = 1; n < line; n++) {
    const next = text.indexOf("\n", from);
    if (next < 0) break;
    from = next + 1;
  }
  const end = text.indexOf("\n", from);
  e.area.setSelectionRange(from, end < 0 ? text.length : end);
  const height = parseFloat(getComputedStyle(e.area).lineHeight) || 19;
  e.area.scrollTop = Math.max(0, (line - 1) * height - e.area.clientHeight / 2 + height / 2);
}

/** Show what the bar above the text says, the marks of the tabs, and tell the shell whether changes are not saved. */
export function refreshEditing() {
  const unapplied = edits.unappliedPaths();
  const dirty = edits.dirtyPaths();
  pane.classList.toggle("editing", editing);
  const state = $("edit-state");
  state.hidden = !editing;
  state.dataset.state = unapplied.length > 0 ? "unapplied" : dirty.length > 0 ? "unsaved" : "clean";
  state.textContent = unapplied.length > 0 ? "Changes not applied yet" : dirty.length > 0 ? "Applied, not saved" : "No changes";
  $("apply-edits").hidden = !editing;
  $("apply-edits").disabled = exiting;
  [...tabs.children].forEach((tab, k) => {
    const path = doc && doc.files[k] ? doc.files[k].path : "";
    tab.dataset.unsaved = String(dirty.includes(path));
    tab.dataset.unapplied = String(unapplied.includes(path));
    tab.title = [path, unapplied.includes(path) ? "changes not applied" : "", dirty.includes(path) ? "not saved" : ""]
      .filter(Boolean)
      .join(", ");
  });
  const any = dirty.length > 0;
  if (any !== notified) {
    notified = any;
    if (shell && shell.setDirty) shell.setDirty(any);
  }
}

/** A line in the status bar that goes after a few seconds: what was done. */
function note(text) {
  const el = $("note");
  el.textContent = text;
  el.hidden = text === "";
  clearTimeout(noteTimer);
  if (text) noteTimer = setTimeout(() => note(""), 6000);
}

/** Edit > Edit Text: enter editing, or apply pending text and return to reading. */
export async function toggleEdit() {
  if (currentSource()?.readonly) return;
  if (modal()) return;
  if (!editing) {
    if (!doc) {
      await inform({ title: "Edit Text", paragraphs: ["Open a netlist first: File › Open (Ctrl+O)."] });
      return;
    }
    if (previewing()) {
      await inform({
        title: "Edit Text",
        paragraphs: ["A solve is shown as a preview, which is not edited. Solve › Close Preview shows the netlist again."],
      });
      return;
    }
    keepPosition();
    editing = true;
    setCheck("toggle-edit", true);
    renderEditors();
    // Set the caret before focusing: Firefox otherwise schedules a caret reveal that overrides
    // the restored first visible line, even when focus itself uses preventScroll.
    restorePosition();
    focusEditor();
    return;
  }
  await stopEditing();
}

/** All exits apply first, including programmatic exits such as Solve. */
export async function stopEditing() {
  return applyChanges();
}

function finishEditing() {
  editing = false;
  setCheck("toggle-edit", false);
  renderEditors();
  restorePosition();
  if (document.activeElement === document.body) source.focus({ preventScroll: true });
}

/** Apply and exit, even when a rebuild reports errors. No changes means no rebuild. */
export async function applyChanges() {
  if (!doc || !editing || exiting) return false;
  exiting = true;
  keepPosition();
  try {
    const ok = edits.unappliedPaths().length === 0 || await applyEdits();
    if (ok) note("Changes applied; editing closed.");
    return ok;
  } finally {
    exiting = false;
    finishEditing();
  }
}

/** Discard only pending edits; already applied, unsaved text remains available to Save. */
export async function discardChanges() {
  if (!editing || exiting || modal()) return;
  if (edits.unappliedPaths().length && await decide({
    title: "Discard Changes",
    paragraphs: ["Discard the changes that have not been applied and exit editing?"],
    action: "Discard Changes",
  }) !== "Discard Changes") return;
  keepPosition();
  for (const path of edits.unappliedPaths()) edits.discard(path);
  finishEditing();
}

/** The files to write or download for File > Save: the files with changes not saved; on the web the file shown. */
function filesToSave() {
  if (shell.platform === "web") {
    const f = doc.files[fileShown];
    return f ? [{ path: f.path, name: f.name, text: edits.fileText(f.path) }] : [];
  }
  return edits.dirty();
}

/**
 * File > Save: write the files with changes not saved where they were read from, or download the
 * file shown on the web. A file that is no longer what was read, because another program changed
 * it, is not written, nor are the others: the user is asked, and with `overwrite` they have said.
 * True when saved.
 */
export async function save(overwrite = false) {
  if (currentSource()?.readonly) return false;
  if (!doc || modal()) return false;
  const opened = currentSource();
  if (shell.saveAs && opened && opened.text !== undefined) return saveAs();
  const files = filesToSave();
  if (files.length === 0) {
    note(edits.dirtyPaths().length === 0 ? "No changes to save." : "");
    return true;
  }
  try {
    const result = await shell.save(opened, files, overwrite);
    if (result !== null && typeof result === "object") return changedOnDisk(result.changed);
    edits.markSaved(files.map((f) => f.path));
    refreshEditing();
    note(result || "Saved.");
    return true;
  } catch (e) {
    await inform({ title: "Save", paragraphs: [`The changes were not saved: ${e instanceof Error ? e.message : String(e)}`] });
    return false;
  }
}

/**
 * The files that changed on disk since they were read, which saving would overwrite: nothing was
 * written. The user says: Overwrite them, Reload the files, which drops the changes made here, or
 * Cancel and look at what is on disk first.
 */
async function changedOnDisk(names) {
  const many = names.length > 1;
  const choice = await decide({
    title: "File Changed on Disk",
    paragraphs: [
      `${many ? "These files were" : "This file was"} changed by another program since the viewer read ${many ? "them" : "it"}: ${names.join(", ")}.`,
      "Nothing was saved. Overwrite writes the text in the viewer over what is on disk; Reload reads the files again and drops the changes made here.",
    ],
    action: "Overwrite",
    others: ["Reload"],
  });
  if (choice === "Overwrite") return save(true);
  if (choice === "Reload") {
    const opened = currentSource();
    if (opened) await openSource(opened, true);
  }
  return false;
}

/**
 * File > Save As (desktop): write the netlist that was opened, the first file, where the user says,
 * and open the file written as the document. Another file with changes not saved is saved first, with Save: it
 * belongs to the directory of the netlist as it was. True when it was saved.
 */
export async function saveAs() {
  if (currentSource()?.readonly) return false;
  if (!doc || !shell.saveAs || modal()) return false;
  const entry = doc.files[0];
  const title = "Save As";
  if (!entry) return false;
  if (fileShown !== 0) {
    await inform({
      title,
      paragraphs: ["Save As saves the netlist that was opened, the first file. An included file is saved where it is with File › Save."],
    });
    return false;
  }
  const others = edits.dirty().filter((f) => f.path !== entry.path);
  if (others.length > 0) {
    await inform({
      title,
      paragraphs: [`Save the other files with changes first, with File › Save: ${others.map((f) => f.name).join(", ")}.`],
    });
    return false;
  }
  try {
    const written = await shell.saveAs(currentSource(), { path: entry.path, name: entry.name, text: edits.fileText(entry.path) });
    if (!written) return false;
    edits.markSaved([entry.path]);
    await openSource(written, false);
    note(`Saved as ${written.name}.`);
    return true;
  } catch (e) {
    await inform({ title, paragraphs: [`The netlist was not saved: ${e instanceof Error ? e.message : String(e)}`] });
    return false;
  }
}

/**
 * Whether to go on with something that leaves the document, such as opening another file or
 * closing the window, when files have changes that are not saved: asks to save them, to leave them
 * unsaved, or not to go on. On the web, where saving is a download, it asks to leave them unsaved
 * or not to go on. True to go on.
 */
export async function confirmLeave(what) {
  const files = edits.dirtyPaths().map((path) => (doc && doc.files.find((f) => f.path === path)) || { name: path });
  if (files.length === 0) return true;
  if (modal()) return false;
  const names = files.map((f) => f.name).join(", ");
  const can = shell.platform !== "web" && shell.save;
  const choice = await decide({
    title: "Changes Not Saved",
    paragraphs: [`${files.length === 1 ? "This file has" : "These files have"} changes that are not saved: ${names}.`, `Do you want to ${what}?`],
    action: can ? "Save" : "Discard Changes",
    others: can ? ["Don't Save"] : [],
  });
  if (choice === null) return false;
  if (choice === "Save") return save();
  return true;
}

/** The window is asked to close (desktop): when it may, the shell quits. */
export async function closeRequested() {
  if (await confirmLeave("close the window")) shell.quit();
}

$("apply-edits").addEventListener("click", () => applyChanges());
