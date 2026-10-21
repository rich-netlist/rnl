import { m, setText } from "./i18n.js";
// Shared by normal opens, rebuilds and embeds. A newer operation owns the indicator, so an older
// completion or progress event cannot clear or relabel it. There is no invented percentage.
const stages = { reading: m("Reading files…"), parsing: m("Parsing netlist…"), cached: m("Using cached drawing…"), displaying: m("Displaying netlist…") };
let generation = 0;

/** Give the browser a paint opportunity without stalling a hidden/background iframe. */
export function nextPaint() {
  return new Promise((resolve) => {
    let first, second;
    const done = () => { clearTimeout(timer); cancelAnimationFrame(first); cancelAnimationFrame(second); resolve(); };
    const timer = setTimeout(done, 50);
    first = requestAnimationFrame(() => { second = requestAnimationFrame(done); });
  });
}

export function startLoading() {
  const id = ++generation;
  const el = document.getElementById("loading");
  const app = document.getElementById("app");
  // A cached/small open should not flash an indicator or wait for extra frames solely to paint it.
  el.hidden = true;
  const timer = setTimeout(() => { if (id === generation) el.hidden = false; }, 120);
  app.setAttribute("aria-busy", "true");
  const setStage = (stage) => {
    if (id !== generation || !(stage in stages)) return;
    el.dataset.stage = stage;
    setText(document.getElementById("loading-stage"), stages[stage]);
  };
  setStage("reading");
  return {
    setStage,
    async stage(stage) { setStage(stage); if (!el.hidden) await nextPaint(); },
    finish() {
      clearTimeout(timer);
      if (id !== generation) return;
      el.hidden = true;
      app.removeAttribute("aria-busy");
    },
  };
}
