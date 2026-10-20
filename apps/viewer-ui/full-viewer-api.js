import { startViewerApi } from "./viewer-api.js";
import { shell } from "./shell.js";
import { prepareViewer } from "./shells/web.js";
import { confirmLeave } from "./editor.js";
import { changes, resetBuffers } from "./edits.js";
import { adoptSource, showBuilt, beginOpenOperation } from "./open.js";
import { restorePool, sourceOpened } from "./pool.js";
import { endPreview } from "./solve.js";
import { current, doc } from "./state.js";
import { fit, notice } from "./drawing.js";
import { setBackground } from "./background.js";
import { enterRoot } from "./subcircuits.js";
import { startLoading } from "./loading.js";

async function mayReplace(source, signal) {
  if (await confirmLeave(`open ${source.name}`, signal)) return;
  const error = new Error("declined");
  error.reason = "declined";
  throw error;
}

async function open(source, packages, isCurrent, initial, message, signal) {
  await mayReplace(source, signal);
  if (!isCurrent()) return;
  const approved = JSON.stringify([changes(false), changes(true)]);
  const progress = startLoading();
  let built;
  try {
    built = await shell.build(source, undefined, progress.setStage, packages, signal);
    if (!isCurrent()) return;
    const errors = built.diagnostics.filter(d => d.severity === "error");
    if (built.failed || errors.length) throw new Error(built.failed || errors[0].message);
    // Edits made while files were fetched were not covered by the first confirmation.
    if (JSON.stringify([changes(false), changes(true)]) !== approved) await mayReplace(source, signal);
    if (!isCurrent()) return;
    endPreview();
    void restorePool(packages);
    resetBuffers(built);
    adoptSource(source);
    showBuilt(built, false);
    sourceOpened(source, built, true);
    return { name: built.name, levels: built.levels.map(l => l.title), level: built.levels[current]?.title ?? "",
      errors: 0, warnings: built.diagnostics.filter(d => d.severity === "warning").length };
  } finally {
    if (built) shell.discardBuild?.(built);
    progress.finish();
  }
}

export async function startFullViewerApi() {
  const version = await prepareViewer();
  startViewerApi({ version, opener: true, open, onError: notice, begin(signal) {
    const operation = beginOpenOperation();
    signal.addEventListener("abort", () => operation.abort(), { once: true });
    return operation.signal;
  }, command(message) {
    if (message.type === "fit") fit();
    else if (message.type === "background" && ["light", "dark"].includes(message.value)) setBackground(message.value);
    else if (message.type === "level" && typeof message.name === "string") {
      const level = doc?.levels.findIndex(l => l.title === message.name) ?? -1;
      if (level < 0) throw new Error(`Unknown level: ${message.name}`);
      enterRoot(level, { newWindow: false });
    }
  } });
}
