// Small helpers for building the page.

export const $ = (id) => document.getElementById(id);

export function clamp(v, lo, hi) {
  return Math.min(hi, Math.max(lo, v));
}

export function span(cls, text) {
  const s = document.createElement("span");
  s.className = cls;
  s.textContent = text;
  return s;
}

export function svgElement(name, attrs) {
  const el = document.createElementNS("http://www.w3.org/2000/svg", name);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v));
  return el;
}

let preferences = null;
let ephemeral = false;
let persist = null;
let pending = Promise.resolve();
export let settingsPath = null;

/** Load native settings before any view writes its initial state; migrate known browser keys once. */
export async function startSettings(shell) {
  ephemeral = !!shell.embed;
  if (ephemeral) return;
  if (!shell.loadSettings) return;
  const keys = ["sidebar-width", "sidebar-shown", "side-pane", "problems-height", "problems-shown",
    "index-height", "show-failed", "hover-names", "grid-shown", "grid-style", "grid-spacing",
    "grid-major", "grid-axes", "solve-options", "solve-time-limit", "search-paths", "background", "auto-include"];
  const legacy = {};
  for (const key of keys) {
    const value = load(key, undefined);
    if (value !== undefined) legacy[key] = value;
  }
  const loaded = await shell.loadSettings(legacy);
  preferences = loaded.values;
  settingsPath = loaded.path;
  persist = shell.saveSetting;
  for (const key of keys) {
    try { localStorage.removeItem(key); } catch { /* browser storage may be disabled */ }
  }
}

/** Wait for changes already queued, such as PATH, before a native build or solve. */
export const flushSettings = () => pending;

/** A persisted value, or `fallback` when there is none to read. */
export function load(key, fallback) {
  if (ephemeral) return fallback;
  if (preferences) return Object.hasOwn(preferences, key) ? preferences[key] : fallback;
  try {
    const v = localStorage.getItem(key);
    return v === null ? fallback : JSON.parse(v);
  } catch {
    return fallback;
  }
}

/** Keep settings in the native configuration file, or browser storage on the web. */
export function save(key, value) {
  if (ephemeral) return;
  if (preferences) {
    const before = preferences[key];
    if (JSON.stringify(before) === JSON.stringify(value)) return pending;
    preferences[key] = value;
    pending = pending.catch(() => {}).then(() => persist(key, value)).catch((error) => {
      if (preferences[key] === value) {
        if (before === undefined) delete preferences[key];
        else preferences[key] = before;
      }
      throw error;
    });
    pending.catch((error) => window.dispatchEvent(new CustomEvent("settings-error", { detail: String(error) })));
    return pending;
  }
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // the next start has the default
  }
}
