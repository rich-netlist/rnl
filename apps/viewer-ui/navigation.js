import { m, tr } from "./i18n.js";
// Every user action that changes the displayed circuit MUST go through navigate().
// Rendering/rebuilding a document is not navigation. Targets carry a source, a hierarchy path,
// and optionally an object to select; Shift sends that same target to another viewer.
import { decide, inform, modal } from "./dialog.js";
import { changes } from "./edits.js";
import { currentSource, isLoading, openSource } from "./open.js";
import { poolSnapshot, restorePool } from "./pool.js";
import { shell } from "./shell.js";
import { doc, path, selected, selectedAsUsage, viewCache } from "./state.js";
import { commitNavigation, namedPath, rememberView } from "./subcircuits.js";
import { showFile } from "./text.js";
import { setPane } from "./panels.js";
import { sameSource } from "./visit-source.js";

let shift = false;
window.addEventListener("keydown", (e) => { shift = e.shiftKey; }, true);
window.addEventListener("keyup", (e) => { shift = e.shiftKey; }, true);
window.addEventListener("blur", () => { shift = false; });

export function navigate(target, options = {}) {
  if (isLoading()) return;
  const visit = destination(target);
  if (options.newWindow ?? shift) return newWindow(visit);
  if (target.file !== undefined) setPane('netlist');
  if (target.source || target.file !== undefined || !sameSource(visit.entry, currentSource())) {
    rememberView();
    // the file is built without the instance, with the parameters that the instance writes bound (params.js)
    return openSource(visit.entry, false, undefined, false,
      { ...visit, viewCache: visit.path.length ? viewCache : new Map() }, options.signal, false, { bind: visit.bind, bound: visit.bound });
  }
  const next = visit.path.map(s => ({ ...s,
    level: sameSource(s.entry, currentSource()) ? doc.levels.findIndex(l => l.title === s.title) : -1 }));
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
    reveal: target.reveal, netlist: target.file !== undefined, bind: target.bind, bound: target.bound };
}

function fileSource(file) {
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
  // Desktop sessions re-read disk. Pending editor text never becomes a new window's input.
  if (shell.platform === "desktop" && Object.keys(changes(true)).length &&
      await decide({ title: m("New Window"), paragraphs: [m("The new window reads files from disk and will not see the unsaved changes here.")],
        action: tr("Open Anyway"), cancel: tr("Cancel") }) !== "Open Anyway") return;
  const session = {
    ...visit,
    pool: poolSnapshot(),
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
  if (session.netlist) setPane('netlist');
  if (session.sourceFile) {
    const file = doc.files.findIndex(f => f.path === session.sourceFile);
    if (file >= 0) { setPane('netlist'); showFile(file); }
  }
}
