// Shared by normal opens, rebuilds and embeds. A newer operation owns the indicator, so an older
// completion or progress event cannot clear or relabel it. There is no invented percentage.
const stages = { reading: "Reading files…", parsing: "Parsing netlist…", cached: "Using cached drawing…", displaying: "Displaying netlist…" };
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
  el.hidden = false;
  app.setAttribute("aria-busy", "true");
  const setStage = (stage) => {
    if (id !== generation || !(stage in stages)) return;
    el.dataset.stage = stage;
    document.getElementById("loading-stage").textContent = stages[stage];
  };
  setStage("reading");
  return {
    setStage,
    async stage(stage) { setStage(stage); await nextPaint(); },
    finish() {
      if (id !== generation) return;
      el.hidden = true;
      app.removeAttribute("aria-busy");
    },
  };
}
