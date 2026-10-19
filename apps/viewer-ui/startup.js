// Run before the main module graph is ready, so the loading screen can already name its version.
// Use each platform's existing metadata rather than keeping another version number in the page.
const version = window.__TAURI__
  ? window.__TAURI__.app.getVersion()
  : fetch("about.json").then((response) => response.ok ? response.json() : {}).then((about) => about.version);

Promise.resolve(version).then((value) => {
  const label = document.getElementById("startup-version");
  if (label && value) label.textContent = `v${value}`;
}).catch(() => {}); // Metadata failure must not prevent the viewer from opening.

/** Reveal the viewer after startup, including when startup reports an error. */
export function finishStartup() {
  document.getElementById("startup")?.remove();
  document.body.classList.remove("starting");
}
