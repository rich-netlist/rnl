// Every user action that changes the displayed circuit MUST go through navigate().
// Rendering/rebuilding a document is not navigation. Targets carry a source, a hierarchy path,
// and optionally an object to select; Shift sends that same target to another viewer.
import { decide, inform, modal } from "./dialog.js";
import { changes } from "./edits.js";
import { currentSource, openSource } from "./open.js";
import { poolSnapshot, restorePool } from "./pool.js";
import { shell } from "./shell.js";
import { doc, path, selected } from "./state.js";
import { commitNavigation, restorePath } from "./subcircuits.js";
import { select } from "./selection.js";
import { showLevel } from "./drawing.js";
import { revealDefinition } from "./text.js";

let shift = false;
window.addEventListener("keydown", (e) => { shift = e.shiftKey; }, true);
window.addEventListener("keyup", (e) => { shift = e.shiftKey; }, true);
window.addEventListener("blur", () => { shift = false; });

export function navigate(target, options = {}) {
  if (options.newWindow ?? shift) return newWindow(target);
  if (target.source) return openSource(target.source, false);
  return commitNavigation(target);
}

/** File > New Window keeps the current level and selection. */
export function duplicateWindow() {
  return newWindow({ path: path.slice(), selection: selected });
}

async function newWindow(target) {
  if (modal()) return;
  const source = target.source || currentSource();
  // Desktop sessions re-read disk. Pending editor text never becomes a new window's input.
  if (shell.platform === "desktop" && Object.keys(changes(true)).length &&
      await decide({ title: "New Window", paragraphs: ["新窗口会读取磁盘上的文件，看不到这里还没保存的修改。"],
        action: "仍然打开", cancel: "取消" }) !== "仍然打开") return;
  const route = target.path;
  const object = target.selection == null ? null : doc?.objects[target.selection];
  const via = target.via;
  const level = route?.length ? doc.levels[route[route.length - 1].level].title : null;
  const session = {
    entry: source,
    level,
    path: route ? route.map((s) => ({ title: doc.levels[s.level].title, via: s.via })) : [],
    select: object ? { kind: object.kind, name: object.name, level: object.level_title } : via ? { kind: "instance", name: via, level } : null,
    pool: poolSnapshot(),
  };
  try {
    // Web shells reserve a tab synchronously, before serializing local files loses user activation.
    await shell.newWindow(session, source === currentSource() ? doc : null);
  } catch (e) {
    await inform({ title: "New Window", paragraphs: [String(e instanceof Error ? e.message : e)] });
  }
}

/** Restore only after settings, the shell and the Library are ready. */
export async function restoreSession(session) {
  await restorePool(session.pool || []);
  if (!session.entry || !await openSource(session.entry, false)) return;
  const level = doc.levels.findIndex((l) => l.title === session.level);
  if (level >= 0) {
    restorePath(session.path || [], level);
    showLevel(level);
    revealDefinition(level);
  }
  if (session.select) {
    const s = session.select;
    const o = doc.objects.findIndex((o) => o.kind === s.kind && o.name === s.name && o.level_title === s.level);
    if (o >= 0) select(o, "reload");
  }
}
