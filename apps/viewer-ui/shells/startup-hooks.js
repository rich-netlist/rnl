// What each shell does while the page starts, before the shell itself (shell.js `loadShell`) and the main module graph
// are loaded: the version the startup screen names, handing the window over from the native startup text to the page,
// and saying that the page takes the files the system asks it to open. Only the desktop app has the last two. It is
// the shells' part, so that the page itself never calls the desktop app (../startup.js); it imports nothing, so that it
// is there at once.

const desktop = "__TAURI__" in window;

/** The version of the viewer that runs: the desktop app's, else the site's (about.json); undefined when not known. */
export function startupVersion() {
  if (desktop) return window.__TAURI__.app.getVersion();
  return fetch("about.json").then((response) => (response.ok ? response.json() : {})).then((about) => about.version);
}

/** Whether the window has a native startup text for the page to take over from. */
export const nativeStartup = desktop;

/** The page has painted its own startup screen: the native text gives way. */
export async function startupPainted() {
  if (desktop) await window.__TAURI__.core.invoke("finish_startup");
}

/** The page has its listener for files opened by the system, and has asked for its first document. */
export async function openEventsReady() {
  if (desktop) await window.__TAURI__.core.invoke("open_events_ready");
}
