// Step 1 settings: a TOML configuration file and scalar rule overrides. Numeric text is
// preserved through the shell settings store and IPC; objective editing returns in Step 3.
import { form, inform, modal } from "./dialog.js";
import { shell } from "./shell.js";
import { load, save, span } from "./util.js";

const KEY = "solve-options";
const NOTICE = "solve-objectives-migration-notice";
const MIGRATED = "Old solver objective settings were removed. Objectives now come from the solver TOML configuration.";
const RULES = [
  ["time_limit", "Time limit"], ["tolerance", "Tolerance"], ["grid", "Grid"],
  ["beam", "Beam"], ["route_beam", "Route beam"], ["separate_gap", "Symbol gap"],
  ["route_clearance", "Route clearance"],
];
const BUDGETS = [["candidates", "Candidates"], ["astar_expansions", "Router cells"], ["soft_expansions", "Soft router cells"], ["symmetry_steps", "Symmetry steps"]];
const scalar = (value) => typeof value === "string" || typeof value === "number";
// Config decimals become reduced fractions at the host boundary. Render their exact finite
// decimal expansion so an edited field can be sent back through the decimal override grammar.
function written(value) {
  if (value === null || value === undefined) return "none";
  const text = String(value);
  const match = /^(-?)(\d+)\/(\d+)$/.exec(text);
  if (!match) return text;
  const denominator = BigInt(match[3]);
  if (!denominator) return text;
  let finite = denominator;
  for (const factor of [2n, 5n]) while (finite % factor === 0n) finite /= factor;
  if (finite !== 1n) return text;
  const numerator = BigInt(match[2]);
  let result = `${match[1]}${numerator / denominator}`;
  let remainder = numerator % denominator;
  if (remainder) result += ".";
  while (remainder) {
    remainder *= 10n;
    result += String(remainder / denominator);
    remainder %= denominator;
  }
  return result;
}

function changed() {
  const raw = load(KEY, null);
  const c = raw && typeof raw === "object" && !Array.isArray(raw) ? { ...raw } : {};
  if (Object.hasOwn(c, "objectives")) {
    delete c.objectives;
    save(KEY, c);
    save(NOTICE, true);
  }
  if (raw === null) {
    const limit = load("solve-time-limit", null);
    if (limit !== null) c.time_limit = limit;
  }
  return c;
}

/** Consume the one-time migration notice in Settings or the next solve report. */
export function migrationNotice() {
  changed();
  if (!load(NOTICE, false)) return null;
  save(NOTICE, false);
  return MIGRATED;
}

function textOf(c) {
  const parts = RULES.filter(([key]) => scalar(c[key])).map(([key]) => `${key}=${c[key]}`);
  if (typeof c.separate_symbols === "boolean") parts.push(`separate_symbols=${c.separate_symbols}`);
  const budget = c.budget || {};
  const counts = BUDGETS.filter(([key]) => scalar(budget[key])).map(([key]) => `${key}:${budget[key]}`);
  if (counts.length) parts.push(`budget=${counts.join(",")}`);
  const settings = { options: parts.join(";"), hintsForLow: c.hints_for_low === true };
  if (typeof c.solver_config === "string" && c.solver_config.trim()) settings.solverConfig = c.solver_config.trim();
  return settings;
}

export function solverSettings() { return textOf(changed()); }

function inputField(label, type = "text") {
  const input = document.createElement("input");
  input.type = type;
  const caption = document.createElement("label");
  caption.className = "settings-label";
  caption.append(span("name", label), input);
  const line = document.createElement("div");
  line.className = "dialog-line settings-line";
  line.append(caption);
  return { input, line };
}

export async function settings() {
  if (!shell?.solverDefaults || modal()) return;
  const before = changed();
  let defaults;
  try { defaults = await shell.solverDefaults(before.solver_config || null); }
  catch (e) {
    await inform({ title: "Solver Settings", paragraphs: [String(e)] });
    return;
  }
  const el = document.createElement("div");
  el.className = "settings";
  const config = inputField("Configuration file");
  const browse = document.createElement("button");
  browse.type = "button";
  browse.textContent = "Choose File…";
  browse.disabled = !shell.pickSolverConfig;
  browse.addEventListener("click", async () => {
    try {
      const path = await shell.pickSolverConfig();
      if (path) { revision++; config.input.value = path; refreshDefaults(); }
    } catch (e) { fail(String(e)); }
  });
  config.line.append(browse);
  const source = span("settings-source", `Default configuration: ${defaults.source || "builtin"}`);
  const note = span("settings-note", "Objectives are read from the TOML configuration. A blank file uses the environment, application directory, then builtin configuration. Rule overrides below apply last. The priority of a constraint is written in the netlist. Use none for no time limit or no grid rounding.");
  el.append(note, config.line, source);
  const fields = new Map();
  for (const [key, label] of [...RULES, ...BUDGETS]) {
    const field = inputField(label);
    fields.set(key, field.input);
    el.append(field.line);
  }
  const separate = inputField("Separate symbols", "checkbox");
  const hints = inputField("Seek every conflict group of the facts of priority low", "checkbox");
  el.append(separate.line, hints.line, span("settings-hint", "Text positions stay at their anchors in this version."));
  const error = span("dialog-error", "");
  error.hidden = !defaults.config_error;
  error.textContent = defaults.config_error || "";
  const fail = (message) => { error.textContent = message; error.hidden = false; return undefined; };
  const restore = document.createElement("button");
  restore.type = "button";
  restore.textContent = "Restore Defaults";
  const asText = span("settings-text", "");
  const explicit = new Set();
  let requestedPath = before.solver_config?.trim() || "";
  let generation = 0;
  let pending = Promise.resolve();
  let loadError = defaults.config_error || null;
  let revision = 0;
  const set = (c) => {
    config.input.value = c.solver_config || "";
    explicit.clear();
    for (const [key] of RULES) {
      if (Object.hasOwn(c, key)) explicit.add(key);
      fields.get(key).value = written(c[key] ?? defaults[key]);
    }
    for (const [key] of BUDGETS) {
      if (Object.hasOwn(c.budget || {}, key)) explicit.add(key);
      fields.get(key).value = written(c.budget?.[key] ?? defaults.budget?.[key]);
    }
    for (const key of ["separate_symbols", "hints_for_low"]) if (Object.hasOwn(c, key)) explicit.add(key);
    separate.input.checked = c.separate_symbols ?? defaults.separate_symbols ?? true;
    hints.input.checked = c.hints_for_low ?? defaults.hints_for_low ?? false;
  };
  const read = () => {
    const c = {};
    if (config.input.value.trim()) c.solver_config = config.input.value.trim();
    for (const [key] of RULES) {
      const value = fields.get(key).value.trim();
      if (!value) return { error: `${key} requires a value.` };
      if (explicit.has(key)) c[key] = value;
    }
    const budget = {};
    for (const [key] of BUDGETS) {
      const value = fields.get(key).value.trim();
      if (!value) return { error: `${key} requires a value.` };
      if (explicit.has(key)) budget[key] = value;
    }
    if (Object.keys(budget).length) c.budget = budget;
    if (explicit.has("separate_symbols")) c.separate_symbols = separate.input.checked;
    if (explicit.has("hints_for_low")) c.hints_for_low = hints.input.checked;
    return c;
  };
  function refreshDefaults(force = false) {
    const path = config.input.value.trim();
    if (!force && path === requestedPath) return pending;
    requestedPath = path;
    const ticket = ++generation;
    source.textContent = "Loading configuration…";
    pending = (async () => {
      try {
        const loaded = await shell.solverDefaults(path || null);
        if (ticket !== generation) return;
        defaults = loaded;
        loadError = loaded.config_error || null;
        for (const [key] of RULES) if (!explicit.has(key)) fields.get(key).value = written(defaults[key]);
        for (const [key] of BUDGETS) if (!explicit.has(key)) fields.get(key).value = written(defaults.budget?.[key]);
        if (!explicit.has("separate_symbols")) separate.input.checked = defaults.separate_symbols ?? true;
        if (!explicit.has("hints_for_low")) hints.input.checked = defaults.hints_for_low ?? false;
        source.textContent = `Default configuration: ${defaults.source || "builtin"}`;
        error.textContent = loadError || "";
        error.hidden = !loadError;
        showText();
      } catch (e) {
        if (ticket !== generation) return;
        loadError = String(e);
        source.textContent = "Configuration could not be loaded";
        fail(loadError);
      }
    })();
    return pending;
  }
  function showText() {
    const c = read();
    const settings = c.error ? null : textOf(c);
    asText.textContent = !settings ? "" : `rnl solve${settings.solverConfig ? ` --solver-config "${settings.solverConfig}"` : ""}${settings.options ? ` --solver-options "${settings.options}"` : ""}${settings.hintsForLow ? " --report" : ""}`;
  }
  set(before);
  showText();
  restore.addEventListener("click", () => { revision++; set({}); refreshDefaults(true); });
  for (const [key, input] of [...fields, ["separate_symbols", separate.input], ["hints_for_low", hints.input]]) {
    input.addEventListener("input", () => explicit.add(key));
    input.addEventListener("change", () => explicit.add(key));
  }
  config.input.addEventListener("input", () => refreshDefaults());
  config.input.addEventListener("change", () => refreshDefaults());
  el.addEventListener("input", () => { revision++; showText(); });
  el.addEventListener("change", () => { revision++; showText(); });
  const migrated = migrationNotice();
  const chosen = await form({
    title: "Solver Settings",
    parts: [...(migrated ? [span("settings-migration", migrated)] : []), el, restore, asText, error],
    focus: config.input,
    wide: true,
    answer: async () => {
      // A path can change while an earlier host read is pending. Only the newest result may
      // update the form, and saving must wait for that result before reading rule overrides.
      for (;;) {
        for (;;) {
          refreshDefaults();
          const latest = pending;
          await latest;
          if (latest === pending && config.input.value.trim() === requestedPath) break;
        }
        if (loadError) return fail(loadError);
        const kept = read();
        if (kept.error) return fail(kept.error);
        const value = textOf(kept);
        const checking = revision;
        const wrong = await shell.checkSolverOptions(value.options, value.solverConfig || null);
        if (checking !== revision || config.input.value.trim() !== requestedPath) continue;
        if (wrong) return fail(wrong);
        return { kept };
      }
    },
  });
  if (chosen) save(KEY, chosen.kept);
}
