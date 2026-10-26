import { append, m, setAttr, setText } from "./i18n.js";
// Solve > Settings (desktop): the options of the solver, as `rnl solve --solver-options` has them
// (crates/rnl-solver/src/options.rs): the time limit of the resolution, the tolerance, the grid, the
// beam, the work of the search and the router, whether to seek every conflict group of the facts of
// priority low, and the objectives, the order in which the solver ranks candidates, with their levels.
// The solver's defaults come from the shell, so the page keeps none of them; what the user changed
// is kept in the shell's settings under `solve-options`, as only the settings that differ from the defaults,
// and so follows the solver's defaults where it was not changed. A solve gets them as the text
// `--solver-options` takes (`solverSettings`), which the solver reads, so its rules are the only ones.

import { form, inform, modal } from "./dialog.js";
import { shell } from "./shell.js";
import { load, save, span } from "./util.js";

const KEY = "solve-options";

/** What each objective of the solver does, for the dialog; one the page does not know has none. */
const WHAT = {
  current_direction: m("the sources of transistors towards the ground"),
  ground_direction: m("ground symbols below their pins"),
  signal_flow: m("input ports left of output ports"),
  component_alignment: m("connected elements in few columns and rows"),
  distribution: m("elements evenly spaced"),
  symmetry: m("mirror pairs placed symmetrically"),
  turns: m("wires with fewer bends"),
  crossings: m("wires of different nets crossing less"),
  wire_length: m("shorter wires"),
  area: m("a smaller drawing"),
  aspect: m("a drawing about as wide as 16 to 9"),
  terminal_wire_alignment: m("wires leaving terminals in line"),
  labels: m("the places of labels"),
  bend_alignment: m("the bends of wires in line"),
};

/**
 * What the user changed, as kept for the next start: any of `time_limit`, `tolerance`, `grid`,
 * `beam`, `budget` ({ candidates, astar_expansions }), `objectives` (the layers) and
 * `hints_for_low`; a setting not there is the solver's default. The time limit that Solve > Time
 * Limit kept before the settings took it over is read as one.
 */
function changed() {
  const stored = load(KEY, null);
  if (stored !== null && typeof stored === "object" && !Array.isArray(stored)) return stored;
  const limit = load("solve-time-limit", null);
  return limit === null ? {} : { time_limit: limit };
}

/** A number, or "none", for a setting that takes either. */
const numberOrNone = (v) => v === "none" || (typeof v === "number" && Number.isFinite(v));

/** Whether a value is a list of layers of objectives: each a list of items, and a tolerance. */
const layersOk = (v) =>
  Array.isArray(v) &&
  v.every(
    (layer) =>
      layer !== null &&
      typeof layer === "object" &&
      typeof layer.tolerance === "number" &&
      Array.isArray(layer.items) &&
      layer.items.length > 0 &&
      layer.items.every((i) => i !== null && typeof i === "object" && typeof i.name === "string" && Number.isInteger(i.weight)),
  );

/**
 * The settings a solve gets from the page: the changed settings written as `--solver-options` takes
 * them, such as `beam=8;time_limit=none`, empty when none is changed, and whether to seek every
 * conflict group of the facts of priority low, which that text cannot say.
 *
 * @returns {{ options: string, hintsForLow: boolean }}
 */
export function solverSettings() {
  return textOf(changed());
}

/** The text of changed settings, in the order the solver writes its own; what is not valid is left out. */
function textOf(c) {
  const parts = [];
  if (typeof c.tolerance === "number") parts.push(`tolerance=${c.tolerance}`);
  if (numberOrNone(c.grid)) parts.push(`grid=${c.grid}`);
  if (layersOk(c.objectives)) parts.push(`objectives=${objectivesText(c.objectives)}`);
  if (typeof c.beam === "number") parts.push(`beam=${c.beam}`);
  const budget = c.budget || {};
  const counts = ["candidates", "astar_expansions"].filter((k) => typeof budget[k] === "number");
  if (counts.length > 0) parts.push(`budget=${counts.map((k) => `${k}:${budget[k]}`).join(",")}`);
  if (numberOrNone(c.time_limit)) parts.push(`time_limit=${c.time_limit}`);
  return { options: parts.join(";"), hintsForLow: c.hints_for_low === true };
}

/** Layers of objectives as the solver writes them: `area+wire_length*2~0.5,turns`, `none` for no layer. */
function objectivesText(layers) {
  if (layers.length === 0) return "none";
  const one = (layer) => {
    const items = layer.items.map((i) => (i.weight === 1 ? i.name : `${i.name}*${i.weight}`)).join("+");
    return layer.tolerance ? `${items}~${layer.tolerance}` : items;
  };
  return layers.map(one).join(",");
}

/** A row of the objectives: an objective, whether it is on, and how it joins the layers. */
function rowsOf(layers, names) {
  const rows = [];
  const seen = new Set();
  for (const layer of layers) {
    layer.items.forEach((item, k) => {
      if (!names.includes(item.name) || seen.has(item.name)) return;
      seen.add(item.name);
      rows.push({ name: item.name, on: true, together: k > 0, weight: item.weight, tolerance: k === 0 ? layer.tolerance : 0 });
    });
  }
  for (const name of names) if (!seen.has(name)) rows.push({ name, on: false, together: false, weight: 1, tolerance: 0 });
  return rows;
}

/** The layers the rows make: the objectives that are on, in order, each starting a layer unless it is together with the one above. */
function layersOf(rows) {
  const layers = [];
  for (const r of rows) {
    if (!r.on) continue;
    if (r.together && layers.length > 0) layers[layers.length - 1].items.push({ name: r.name, weight: r.weight });
    else layers.push({ items: [{ name: r.name, weight: r.weight }], tolerance: r.tolerance });
  }
  return layers;
}

/** The values of the dialog's fields: the solver's defaults, with what the user changed over them. */
function valuesOf(defaults, c) {
  const budget = c.budget || {};
  const layers = layersOk(c.objectives) ? c.objectives : defaults.objectives;
  return {
    time_limit: numberOrNone(c.time_limit) ? c.time_limit : defaults.time_limit === null ? "none" : defaults.time_limit,
    tolerance: typeof c.tolerance === "number" ? c.tolerance : defaults.tolerance,
    grid: numberOrNone(c.grid) ? c.grid : defaults.grid === null ? "none" : defaults.grid,
    beam: typeof c.beam === "number" ? c.beam : defaults.beam,
    candidates: typeof budget.candidates === "number" ? budget.candidates : defaults.budget.candidates,
    astar_expansions:
      typeof budget.astar_expansions === "number" ? budget.astar_expansions : defaults.budget.astar_expansions,
    hints_for_low: typeof c.hints_for_low === "boolean" ? c.hints_for_low : defaults.hints_for_low,
    rows: rowsOf(layers, defaults.names),
  };
}

/** What differs from the defaults in values read from the dialog: the settings to keep. */
function differences(defaults, v) {
  const c = {};
  const orNone = (d) => (d === null ? "none" : d);
  if (v.time_limit !== orNone(defaults.time_limit)) c.time_limit = v.time_limit;
  if (v.tolerance !== defaults.tolerance) c.tolerance = v.tolerance;
  if (v.grid !== orNone(defaults.grid)) c.grid = v.grid;
  if (v.beam !== defaults.beam) c.beam = v.beam;
  const budget = {};
  if (v.candidates !== defaults.budget.candidates) budget.candidates = v.candidates;
  if (v.astar_expansions !== defaults.budget.astar_expansions) budget.astar_expansions = v.astar_expansions;
  if (Object.keys(budget).length > 0) c.budget = budget;
  if (v.hints_for_low !== defaults.hints_for_low) c.hints_for_low = v.hints_for_low;
  const layers = layersOf(v.rows);
  if (objectivesText(layers) !== objectivesText(defaults.objectives)) c.objectives = layers;
  return c;
}

/**
 * Solve > Settings: show the settings in a dialog, ask the shell whether the solver takes them, and
 * keep them for the next start. Cancel keeps nothing; Restore Defaults sets the fields to the
 * solver's defaults, which OK then keeps as no changes at all.
 */
export async function settings() {
  if (!shell || !shell.solverDefaults || modal()) return;
  let defaults;
  try {
    defaults = await shell.solverDefaults();
  } catch (e) {
    await inform({ title: m("Solver Settings"), paragraphs: [m`The solver's defaults could not be read: ${e instanceof Error ? e.message : String(e)}`] });
    return;
  }
  const fields = fieldsOf(defaults);
  fields.set(valuesOf(defaults, changed()));
  const error = document.createElement("p");
  error.className = "dialog-error";
  error.hidden = true;
  const restore = document.createElement("button");
  restore.type = "button";
  restore.className = "settings-restore";
  setText(restore, m("Restore Defaults"));
  restore.addEventListener("click", () => {
    fields.set(valuesOf(defaults, {}));
    error.hidden = true;
  });
  // the settings as the command line takes them, for whoever solves outside the viewer too
  const asText = span("settings-text", "");
  const showText = () => {
    const v = fields.read();
    const kept = v.error ? null : textOf(differences(defaults, v));
    const flag = kept && kept.hintsForLow ? " --report" : "";
    const option = kept && kept.options ? ` --solver-options "${kept.options}"` : "";
    setText(asText, !kept ? "" : flag || option ? m`Equivalent CLI command: rnl solve${flag}${option}` : m("The defaults of rnl solve."));
  };
  for (const type of ["input", "change", "click"]) fields.el.addEventListener(type, showText);
  restore.addEventListener("click", showText);
  showText();
  const chosen = await form({
    title: m("Solver Settings"),
    parts: [
      priorityNote(),
      fields.el,
      document.createElement("hr"),
      row(restore, asText),
      error,
    ],
    answer: async () => {
      const v = fields.read();
      if (v.error) return fail(error, v.error);
      const kept = differences(defaults, v);
      const wrong = await shell.checkSolverOptions(textOf(kept).options);
      if (wrong) return fail(error, wrong);
      return { kept };
    },
    focus: fields.first,
    wide: true,
  });
  if (chosen) save(KEY, chosen.kept);
}

/** Say what is wrong in a dialog and keep it open. */
function fail(error, message) {
  setText(error, message);
  error.hidden = false;
  return undefined;
}

/** The note that tells what these settings are not: the priorities of constraints are written in the netlist. */
function priorityNote() {
  const p = document.createElement("p");
  p.className = "dialog-note settings-note";
  setText(p, m`${m`${m("The priority of a constraint, low, normal or high, is written in the netlist (7.6, 8.5), not set here. ")}${m("These settings are for what the solver does after the constraints are satisfied: which objective counts ")}`}${m("first and which next when it ranks the placements that satisfy them, and how long and how hard it works.")}`);
  return p;
}

/** A line of a dialog of a few parts side by side. */
function row(...parts) {
  const el = document.createElement("div");
  el.className = "dialog-line settings-line";
  append(el, ...parts);
  return el;
}

/** An input of a number, with its label, in a line. */
function numberField(label, { step = "any", min = "0", hint = "" } = {}) {
  const input = document.createElement("input");
  input.type = "number";
  input.step = step;
  input.min = min;
  const caption = document.createElement("label");
  caption.className = "settings-label";
  append(caption, span("name", label), input);
  const line = row(caption);
  if (hint) append(line, span("settings-hint", hint));
  return { input, line, label };
}

/** A checkbox with its label. */
function checkField(label) {
  const input = document.createElement("input");
  input.type = "checkbox";
  const caption = document.createElement("label");
  caption.className = "dialog-choice";
  append(caption, input, label);
  return { input, caption };
}

/**
 * The fields of the dialog, as an element: `set(values)` fills them, and `read()` gives the values
 * they hold in the form `valuesOf` makes, or `{ error }` when one is not a number; `first` has the
 * focus at first.
 */
function fieldsOf(defaults) {
  const el = document.createElement("div");
  el.className = "settings";

  const limit = numberField(m("Time limit"), { hint: m("seconds the solver may spend on checking the facts of each level (8.5)") });
  const noLimit = checkField(m("No limit"));
  limit.line.insertBefore(noLimit.caption, limit.line.children[1]);
  noLimit.input.addEventListener("change", () => (limit.input.disabled = noLimit.input.checked));
  const tolerance = numberField(m("Tolerance"), { hint: m("differences within it count as satisfied (11.7)") });
  const grid = numberField(m("Grid"), { hint: m("free coordinates are rounded to it when the facts allow") });
  const noGrid = checkField(m("No rounding"));
  grid.line.insertBefore(noGrid.caption, grid.line.children[1]);
  noGrid.input.addEventListener("change", () => (grid.input.disabled = noGrid.input.checked));
  const beam = numberField(m("Beam"), { step: "1", min: "1", hint: m("variants of each group the search keeps") });
  const candidates = numberField(m("Candidates"), { step: "1", hint: m("maximum candidate placements evaluated per level") });
  const expansions = numberField(m("Router cells"), { step: "1", hint: m("maximum router cells expanded per net") });
  const hints = checkField(m("Seek every conflict group of the facts of priority low"));
  const hintsNote = span("settings-hint", m("Only the report changes, and it can take far longer."));
  const objectives = objectiveRows(defaults.names);

  const group = (title, ...parts) => {
    const set = document.createElement("fieldset");
    const legend = document.createElement("legend");
    setText(legend, title);
    append(set, legend, ...parts);
    return set;
  };
  append(el, group(m("Time"), limit.line), group(m("Placement"), tolerance.line, grid.line, beam.line), group(m("Work"), candidates.line, expansions.line, row(hints.caption, hintsNote)), group(m("Objectives"), objectives.el));

  /** The number in a field, or null when it holds none. */
  const read = (field, integer = false) => {
    const text = field.input.value.trim();
    const v = Number(text);
    return text === "" || !Number.isFinite(v) || (integer && !Number.isInteger(v)) ? null : v;
  };
  return {
    el,
    first: limit.input,
    set(v) {
      noLimit.input.checked = v.time_limit === "none";
      limit.input.value = v.time_limit === "none" ? "" : String(v.time_limit);
      limit.input.disabled = noLimit.input.checked;
      tolerance.input.value = String(v.tolerance);
      noGrid.input.checked = v.grid === "none";
      grid.input.value = v.grid === "none" ? "" : String(v.grid);
      grid.input.disabled = noGrid.input.checked;
      beam.input.value = String(v.beam);
      candidates.input.value = String(v.candidates);
      expansions.input.value = String(v.astar_expansions);
      hints.input.checked = v.hints_for_low;
      objectives.set(v.rows);
    },
    read() {
      const v = {
        time_limit: noLimit.input.checked ? "none" : read(limit),
        tolerance: read(tolerance),
        grid: noGrid.input.checked ? "none" : read(grid),
        beam: read(beam, true),
        candidates: read(candidates, true),
        astar_expansions: read(expansions, true),
        hints_for_low: hints.input.checked,
      };
      const bad = [
        ["time_limit", m("Time limit must be a number of seconds, or no limit.")],
        ["tolerance", m("Tolerance must be a number.")],
        ["grid", m("Grid must be a number, or no rounding.")],
        ["beam", m("Beam must be a positive whole number.")],
        ["candidates", m("Candidates must be a whole number.")],
        ["astar_expansions", m("Router cells must be a whole number.")],
      ].find(([k]) => v[k] === null);
      if (bad) return { error: bad[1] };
      const rows = objectives.read();
      if (rows.error) return { error: rows.error };
      return { ...v, rows: rows.rows };
    },
  };
}

/**
 * The objectives as an ordered list of rows, each with buttons that move it up and down, a check
 * box that turns it on, one that puts it in the level of the objective above, its weight, and for
 * the first of a level how far from the best a value still counts as equal to it. `set(rows)`
 * fills them, and `read()` gives `{ rows }` as `rowsOf` makes them, or `{ error }`.
 */
function objectiveRows(names) {
  const el = document.createElement("div");
  el.className = "objectives";
  const note = document.createElement("p");
  note.className = "dialog-note";
  setText(note, m("The solver ranks valid placements by these objective tiers from top to bottom. Objectives combined in the same tier are summed by weight; placements within the tier's tolerance of the best score tie and are broken by the next tier. Disabled objectives are skipped."));
  const list = document.createElement("ol");
  list.className = "objective-list";
  append(el, note, list);
  let rows = []; // { name, el, on, together, weight, tolerance, ... } in the order shown

  /** A row's element, with the controls that set its state. */
  function make(state) {
    const r = { ...state };
    const li = document.createElement("li");
    li.className = "objective";
    li.dataset.name = r.name;
    const badge = span("objective-level", "");
    const on = document.createElement("input");
    on.type = "checkbox";
    setAttr(on, "aria-label", m`${r.name} is on`);
    const name = span("objective-name", r.name);
    const what = span("objective-what", WHAT[r.name] || "");
    const label = document.createElement("label");
    label.className = "objective-on";
    append(label, on, name);
    const together = checkField(m("with the one above"));
    together.caption.classList.add("objective-together");
    setAttr(together.caption, "title", m("Put it in the tier of the objective above: they add up"));
    const weight = numberInput("1", m("weight"), m("Its weight in the tier"));
    const tolerance = numberInput("any", m("equal within"), m("The tolerance of the tier: how far from the best a value counts as equal to it"));
    const up = moveButton(m("Move up"), "↑", r.name);
    const down = moveButton(m("Move down"), "↓", r.name);
    const moves = span("objective-moves", "");
    append(moves, up, down);
    const top = document.createElement("div");
    top.className = "objective-top";
    append(top, badge, label, what, moves);
    const more = document.createElement("div");
    more.className = "objective-more";
    append(more, together.caption, weight.caption, tolerance.caption);
    append(li, top, more);
    on.addEventListener("change", () => {
      r.on = on.checked;
      refresh();
    });
    together.input.addEventListener("change", () => {
      r.together = together.input.checked;
      refresh();
    });
    weight.input.addEventListener("input", () => (r.weight = weight.input.value));
    tolerance.input.addEventListener("input", () => (r.tolerance = tolerance.input.value));
    up.addEventListener("click", () => move(r, -1, up));
    down.addEventListener("click", () => move(r, 1, down));
    Object.assign(r, { el: li, badge, on_: on, together_: together, weight_: weight, tolerance_: tolerance, up, down });
    on.checked = r.on;
    together.input.checked = r.together;
    weight.input.value = String(r.weight);
    tolerance.input.value = String(r.tolerance);
    r.weight = String(r.weight);
    r.tolerance = String(r.tolerance);
    return r;
  }

  function numberInput(step, text, title) {
    const input = document.createElement("input");
    input.type = "number";
    input.step = step;
    input.min = step === "1" ? "1" : "0";
    setAttr(input, "aria-label", text);
    const caption = document.createElement("label");
    caption.className = "objective-number";
    setAttr(caption, "title", title);
    append(caption, span("name", text), input);
    return { input, caption };
  }

  function moveButton(text, arrow, name) {
    const b = document.createElement("button");
    b.type = "button";
    setText(b, arrow);
    setAttr(b, "title", text);
    setAttr(b, "aria-label", m`${text}: ${name}`);
    return b;
  }

  /** Move a row by one place; the button keeps the focus. */
  function move(r, by, button) {
    const k = rows.indexOf(r);
    const to = k + by;
    if (to < 0 || to >= rows.length) return;
    rows.splice(k, 1);
    rows.splice(to, 0, r);
    list.replaceChildren(...rows.map((x) => x.el));
    refresh();
    (button.disabled ? (by < 0 ? r.down : r.up) : button).focus();
  }

  /** Show each row as what it is now: its level, and what applies to it. */
  function refresh() {
    let level = 0;
    let anyOn = false;
    rows.forEach((r, k) => {
      const leader = r.on && !(r.together && anyOn);
      r.el.classList.toggle("off", !r.on);
      setText(r.badge, !r.on ? m("off") : leader ? String(++level) : "+");
      setAttr(r.badge, "title", !r.on ? m("Not computed") : leader ? m`Tier ${level}` : m("Added to the tier above"));
      r.together_.caption.hidden = !(r.on && anyOn);
      r.weight_.input.disabled = !r.on;
      r.tolerance_.caption.hidden = !leader;
      r.up.disabled = k === 0;
      r.down.disabled = k === rows.length - 1;
      anyOn = anyOn || r.on;
    });
  }

  return {
    el,
    set(values) {
      rows = values.map(make);
      list.replaceChildren(...rows.map((x) => x.el));
      refresh();
    },
    read() {
      const out = [];
      let anyOn = false;
      for (const r of rows) {
        const weight = Number(r.weight);
        const tolerance = Number(r.tolerance);
        // the weight counts where an objective is on, and the tolerance where it begins a level
        const leader = r.on && !(r.together && anyOn);
        if (r.on && (r.weight.trim() === "" || !Number.isInteger(weight))) {
          return { error: m`Weight of ${r.name} must be a whole number.` };
        }
        if (leader && (r.tolerance.trim() === "" || !Number.isFinite(tolerance))) {
          return { error: m`Tolerance of ${r.name}'s tier must be a number.` };
        }
        out.push({ name: r.name, on: r.on, together: r.together, weight: r.on ? weight : 1, tolerance: leader ? tolerance : 0 });
        anyOn = anyOn || r.on;
      }
      return { rows: out };
    },
  };
}
