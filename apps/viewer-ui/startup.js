// Run before the main module graph is ready, so the loading screen can already name its version.
// Use each platform's existing metadata rather than keeping another version number in the page.
import { nextPaint } from "./loading.js";

// The command table and even the main module graph may still be loading when a user presses a
// panel number. Keep a bounded recency list so an unavailable panel (web Solve) cannot discard
// the last usable request. Run it once the initial layout is visible.
const panelKeys = new Set();
function rememberPanelKey(event) {
  if (!document.body.classList.contains("starting") || new URLSearchParams(location.search).get("embed") === "1") return;
  if (event.defaultPrevented || event.isComposing || event.keyCode === 229 ||
      event.ctrlKey || event.metaKey || event.altKey || event.shiftKey || !/^[1-7]$/.test(event.key)) return;
  if (!(event.target instanceof Element) || event.target.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"]), dialog')) return;
  panelKeys.delete(event.key);
  panelKeys.add(event.key);
  event.preventDefault();
}
document.addEventListener("keydown", rememberPanelKey);

// These stylesheets do not block the HTML startup screen's first paint. Activate each as it
// arrives, and initialize layout only after the main stylesheet has loaded. Fonts are optional.
function styleReady(id, required) {
  const link = document.getElementById(id);
  return new Promise((resolve, reject) => {
    const loaded = () => { link.media = "all"; resolve(); };
    if (link.sheet) { loaded(); return; }
    link.addEventListener("load", loaded, { once: true });
    link.addEventListener("error", () => required ? reject(new Error("The viewer stylesheet could not be loaded.")) : resolve(), { once: true });
    link.media = "print";
    link.rel = "stylesheet";
  });
}
export const stylesReady = styleReady("viewer-style", true);
void styleReady("viewer-fonts", false);
// The main graph might still be downloading when a stylesheet fails.
stylesReady.catch(() => {});

const version = window.__TAURI__
  ? window.__TAURI__.app.getVersion()
  : fetch("about.json").then((response) => response.ok ? response.json() : {}).then((about) => about.version);

const named = Promise.resolve(version).then((value) => {
  const label = document.getElementById("startup-version");
  if (label && value) label.textContent = `v${value}`;
}).catch(() => {}); // Metadata failure must not prevent the viewer from opening.

let nativeHandoff;
function revealNativePage() {
  if (!window.__TAURI__) return;
  return nativeHandoff ??= (async () => {
    await nextPaint();
    await window.__TAURI__.core.invoke("finish_startup");
  })();
}
// The native text covers WebView2 creation. Hand over as soon as the HTML startup screen has
// painted, so it can show the shared file-loading stages while the main graph initializes.
if (window.__TAURI__) void named.then(revealNativePage).catch(() => {});

/** Reveal the viewer after startup, including when startup reports an error. */
export async function finishStartup(runPanelShortcut) {
  document.getElementById("startup")?.remove();
  document.body.classList.remove("starting");
  document.removeEventListener("keydown", rememberPanelKey);
  const keys = [...panelKeys].reverse();
  panelKeys.clear();
  // Replay in the same task as enabling normal keys: a later key must always win, even while
  // the native handoff IPC is pending.
  if (keys.length) runPanelShortcut?.(keys);
  await revealNativePage();
}
