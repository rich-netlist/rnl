import { m, tr } from "./i18n.js";
// Every user action that changes the displayed circuit MUST go through navigate().
// Rendering/rebuilding a document is not navigation. Targets carry a source, a hierarchy path,
// and optionally an object to select; Shift sends that same target to another viewer.
import { decide, inform, modal } from "./dialog.js";
import { changes, hasBuffer, setText, unappliedPaths } from "./edits.js";
import { applyEdits, currentSource, isLoading, openSource } from "./open.js";
import { poolSnapshot, restorePool } from "./pool.js";
import { shell } from "./shell.js";
import { doc, expanded, path, selected, selectedAsUsage, viewCache } from "./state.js";
import { commitNavigation, namedPath, rememberView, viewOfFile } from "./subcircuits.js";
import { showModuleOf } from "./module-view.js";
import { showFile } from "./text.js";
import { setPane } from "./panels.js";
import { sameEntry, sameSource } from "./visit-source.js";

let shift = false;
window.addEventListener("keydown", (e) => { shift = e.shiftKey; }, true);
window.addEventListener("keyup", (e) => { shift = e.shiftKey; }, true);
window.addEventListener("blur", () => { shift = false; });

export function navigate(target, options = {}) {
  if (isLoading()) return;
  const visit = destination(target);
  if (options.newWindow ?? shift) return newWindow(visit);
  // (a file opened from the Library keeps the Library shown)
  if (target.file !== undefined && !target.keepPane) setPane('netlist');
  if (target.source || target.file !== undefined || !sameSource(visit.entry, currentSource())) {
    rememberView();
    // another file of the same document, read alone: the tree stays as it was opened
    const tree = doc?.outline && sameEntry(visit.entry, currentSource()) ? { expanded } : {};
    // the file is built without the instance, with the parameters that the instance writes bound (params.js)
    return openSource(visit.entry, false, undefined, false,
      { ...visit, ...tree, viewCache: visit.path.length ? viewCache : new Map() }, options.signal, false, { bind: visit.bind, bound: visit.bound });
  }
  const next = visit.path.map(s => ({ ...s,
    level: sameSource(s.entry, currentSource()) ? doc.levels.findIndex(l => l.title === s.title) : -1 }));
  // a module of Verilog-A of the file shown, which no level draws
  if (showModuleOf(next)) return;
  return commitNavigation({ ...target, path: next });
}

/** Resolve the target's owning file before any build changes the document's indices. */
function destination(target) {
  const route = target.path ? namedPath(target.path) : [];
  const last = route.at(-1);
  const step = target.path?.at(-1);
  const file = target.file ?? (step && !step.entry ? doc.levels[step.level]?.lines?.[0] ?? 0 : undefined);
  const entry = target.source || (file === undefined ? last?.entry : fileSource(file)) || currentSource();
  if (last) last.entry = entry;
  const object = target.selection == null ? null : doc?.objects[target.selection];
  return { entry, path: route, level: last?.title,
    select: object ? { kind: object.kind, name: object.name, level: object.level_title,
      ...(target.selectionUsage === undefined ? {} : { usage: target.selectionUsage }) }
      : target.via ? { kind: 'instance', name: target.via, level: last?.title } : null,
    reveal: target.reveal, netlist: target.file !== undefined && !target.keepPane, bind: target.bind, bound: target.bound };
}

function fileSource(file) {
  // a shell that reads the file shown alone: the view of that file, in the context of the entry
  if (doc?.outline) return viewOfFile(doc.files[file].path);
  if (file === 0) return currentSource();
  const source = shell.sourceForFile(currentSource(), doc, file);
  // Navigation promotes a new file but does not refresh an already indexed Library entry.
  source.library = true;
  return source;
}

/** File > New Window keeps the current level and selection. */
export function duplicateWindow() {
  return newWindow(destination({ path: path.slice(), selection: selected,
    selectionUsage: (doc?.objects[selected]?.kind === "usage" || doc?.objects[selected]?.kind === "group") ? selectedAsUsage : undefined }));
}

async function newWindow(visit) {
  if (modal()) return;
  const source = visit.entry;
  // A desktop window reads the files from disk: the texts applied here and not saved go with it, which it shows as not
  // saved (restoreSession); a text not applied yet stays here.
  const desktop = shell.platform === "desktop";
  if (desktop && unappliedPaths().length &&
      await decide({ title: m("New Window"), paragraphs: [m("The new window shows the changes applied here, but not those not applied yet.")],
        action: tr("Open Anyway"), cancel: tr("Cancel") }) !== "Open Anyway") return;
  const edited = desktop ? changes(true) : {};
  const session = {
    ...visit,
    pool: poolSnapshot(),
    ...(Object.keys(edited).length ? { edits: edited } : {}),
  };
  try {
    // Web shells reserve a tab synchronously, before serializing local files loses user activation.
    await shell.newWindow(session, source === currentSource() ? doc : null);
  } catch (e) {
    await inform({ title: m("New Window"), paragraphs: [String(e instanceof Error ? e.message : e)] });
  }
}

/** Restore only after settings, the shell and the Library are ready. */
export async function restoreSession(session) {
  await restorePool(session.pool || []);
  if (!session.entry || !await openSource(session.entry, false, undefined, false, session, null, false, { bind: session.bind, bound: session.bound })) return;
  // the texts applied in the window this one came from and not saved there: applied here too, and not saved
  const edited = Object.entries(session.edits || {}).filter(([path]) => hasBuffer(path));
  if (edited.length) {
    for (const [path, text] of edited) setText(path, text);
    await applyEdits();
  }
  if (session.netlist) setPane('netlist');
  if (session.sourceFile) {
    const file = doc.files.findIndex(f => f.path === session.sourceFile);
    if (file >= 0) { setPane('netlist'); showFile(file); }
  }
}
