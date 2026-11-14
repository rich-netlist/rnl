import { append, m, setAttr, setText } from "./i18n.js";
// Solve > Settings: the settings of the solver, drawn from the table the solver gives (the shell's `solverTable`,
// `rnl solver-defaults --json`, docs/solver-settings-table.md), so that the page names no key, bound, default or
// objective of its own: a setting the solver adds is in the dialog, and one it changes has the new default. What the
// user changed is kept under `solve-config` as only the settings that differ from the defaults, and so follows the
// solver's defaults where it was not changed. A solve gets them as RNL named parameters (`solverSettings`), as
// `rnl solve --solver-config` takes them, which the solver reads, so its rules are the only ones; the shell can also
// check them (`checkSettings`), and the dialog keeps open on what the solver refuses. The settings that a netlist and
// its packages give are those of the last solve's report, for the dialog to say what a value set here overrides.
// The time for the whole file (`rnl solve --timeall`) is not a setting of the table, so the dialog has a box of its own
// for it, kept under `solve-timeall`, and a solve gets it beside the settings.

import { form, inform, modal } from "./dialog.js";
import { shell } from "./shell.js";
import { current, doc } from "./state.js";
import { load, save, span } from "./util.js";

const KEY = "solve-config";
const TIMEALL = "solve-timeall";

/** The seconds kept for the whole file, or null for none: each level then has its own limit only. */
function keptTimeall() {
  const seconds = load(TIMEALL, null);
  return typeof seconds === "number" && Number.isFinite(seconds) && seconds > 0 ? seconds : null;
}

/**
 * What the user changed, as kept for the next start: a value by the key of its setting. A setting not there is the
 * solver's default. A value is a number, a string, a boolean or `false`; for a setting of a type that the page does
 * not know, `{ literal }`, the text as the user wrote it.
 */
function changed() {
  const stored = load(KEY, null);
  return stored !== null && typeof stored === "object" && !Array.isArray(stored) ? stored : {};
}

/**
 * The settings a solve gets from the page: the settings changed as RNL named parameters, such as
 * `spacing_x=2,beam=8`, empty when none is changed.
 *
 * @returns {{ config: string, timeall: number | null }} `timeall`: the seconds that all levels of a time limit
 *   share (`rnl solve --timeall`), null for none.
 */
export function solverSettings() {
  return { config: configText(changed()), timeall: keptTimeall() };
}

/** A number as RNL writes it: with no exponent. */
function numberText(n) {
  const text = String(n);
  return /e/i.test(text) ? n.toFixed(20).replace(/\.?0+$/, "") : text;
}

/** A value as an RNL literal; null for one that has none. */
function literal(value) {
  if (typeof value === "number" && Number.isFinite(value)) return numberText(value);
  if (typeof value === "boolean") return String(value);
  if (typeof value === "string") return JSON.stringify(value);
  if (value !== null && typeof value === "object" && typeof value.literal === "string") return value.literal;
  return null;
}

/** The text of changed settings, in the order of their keys; what has no literal is left out. */
export function configText(values) {
  return Object.keys(values)
    .sort()
    .map((key) => [key, literal(values[key])])
    .filter(([, text]) => text !== null && text !== "")
    .map(([key, text]) => `${key}=${text}`)
    .join(",");
}

// What the last solve's report said of the settings of each level (`levels[].check`), for the dialog to tell what the
// netlist and its packages give.
let lastLevels = [];

/** Keep the settings that the report of a solve says each of its levels took, and where each came from. */
export function noteReport(report) {
  lastLevels = report && Array.isArray(report.levels) ? report.levels : [];
}

// ---- objectives -------------------------------------------------------------------------------------------------

/** The layers of objectives a text writes: `area+wire_length*2~0.5,turns`, `none` for no layer. */
function parseObjectives(text) {
  if (typeof text !== "string" || text.trim() === "none") return [];
  return text.split(",").map((part) => {
    const [items, tolerance] = part.split("~");
    return {
      items: items.split("+").map((item) => {
        const [name, weight] = item.trim().split("*");
        return { name, weight: weight === undefined ? 1 : Number(weight) };
      }),
      tolerance: tolerance === undefined ? 0 : Number(tolerance),
    };
  });
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
function rowsOf(layers, objectives) {
  const rows = [];
  const seen = new Set();
  const known = new Map(objectives.map((o) => [o.name, o]));
  for (const layer of layers) {
    layer.items.forEach((item, k) => {
      if (!known.has(item.name) || seen.has(item.name)) return;
      seen.add(item.name);
      rows.push({ name: item.name, on: true, together: k > 0, weight: item.weight, tolerance: k === 0 ? layer.tolerance : 0 });
    });
  }
  for (const { name } of objectives) if (!seen.has(name)) rows.push({ name, on: false, together: false, weight: 1, tolerance: 0 });
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

// ---- the fields -------------------------------------------------------------------------------------------------

/** A line of a dialog of a few parts side by side. */
function row(...parts) {
  const el = document.createElement("div");
  el.className = "dialog-line settings-line";
  append(el, ...parts);
  return el;
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
 * The control of one setting by its type: `el`, `set(value)` to fill it, `get()` for `{ value }` or `{ error }` (a text
 * that says what is wrong), and `shown()` for the text its value counts as in a condition (`applies`, `shown_when`
 * and `required_when` of the table).
 */
function control(setting) {
  switch (setting.type) {
    case "integer":
    case "number":
      return numberControl(setting, setting.type === "integer", null);
    case "number-or-false":
      return numberControl(setting, false, setting.false_what ?? "false");
    case "boolean":
      return booleanControl(setting);
    case "choice":
      return choiceControl(setting);
    case "string":
      return textControl(setting, false);
    default:
      // a type the page does not know is a text box, whose text is written as it stands
      return textControl(setting, true);
  }
}

function boundsError(setting, n) {
  if (setting.min !== undefined && n < setting.min) return m`${setting.key} must be at least ${setting.min}.`;
  if (setting.min_exclusive !== undefined && n <= setting.min_exclusive) return m`${setting.key} must be more than ${setting.min_exclusive}.`;
  if (setting.max !== undefined && n > setting.max) return m`${setting.key} must be at most ${setting.max}.`;
  return null;
}

function numberControl(setting, integer, falseLabel) {
  const input = document.createElement("input");
  input.type = "number";
  input.step = integer ? "1" : "any";
  if (setting.min !== undefined) input.min = String(setting.min);
  if (setting.max !== undefined) input.max = String(setting.max);
  setAttr(input, "aria-label", setting.key);
  const no = falseLabel === null ? null : checkField(falseLabel);
  no?.input.addEventListener("change", () => (input.disabled = no.input.checked));
  return {
    el: [input, ...(no ? [no.caption] : [])],
    first: input,
    set(value) {
      const off = value === false;
      if (no) no.input.checked = off;
      input.disabled = off;
      input.value = typeof value === "number" ? String(value) : "";
    },
    get() {
      if (no?.input.checked) return { value: false };
      const text = input.value.trim();
      const n = Number(text);
      if (text === "" || !Number.isFinite(n)) return { error: m`${setting.key} must be a number.` };
      if (integer && !Number.isInteger(n)) return { error: m`${setting.key} must be a whole number.` };
      const bad = boundsError(setting, n);
      return bad ? { error: bad } : { value: n };
    },
    shown: () => (no?.input.checked ? "false" : input.value.trim()),
  };
}

function booleanControl(setting) {
  const input = document.createElement("input");
  input.type = "checkbox";
  setAttr(input, "aria-label", setting.key);
  return {
    el: [input],
    first: input,
    set: (value) => (input.checked = value === true),
    get: () => ({ value: input.checked }),
    shown: () => String(input.checked),
  };
}

function choiceControl(setting) {
  const select = document.createElement("select");
  setAttr(select, "aria-label", setting.key);
  for (const choice of setting.choices ?? []) {
    const option = document.createElement("option");
    option.value = choice.value;
    option.textContent = choice.value;
    select.append(option);
  }
  const what = span("settings-hint choice-what", "");
  const sync = () => setText(what, (setting.choices ?? []).find((c) => c.value === select.value)?.what ?? "");
  select.addEventListener("change", sync);
  return {
    el: [select, what],
    first: select,
    set(value) {
      select.value = typeof value === "string" ? value : "";
      sync();
    },
    get: () => ({ value: select.value }),
    shown: () => select.value,
  };
}

function textControl(setting, raw) {
  const input = document.createElement("input");
  input.type = "text";
  setAttr(input, "aria-label", setting.key);
  return {
    el: [input],
    first: input,
    set: (value) => (input.value = raw ? (value?.literal ?? "") : typeof value === "string" ? value : ""),
    get: () => ({ value: raw ? { literal: input.value } : input.value }),
    shown: () => input.value,
  };
}

/** Whether the values of other settings are as a condition of the table (`applies` and the like) says; `of(key)` is a setting's. */
function holds(when, of) {
  return Object.entries(when).every(([key, values]) => values.includes(of(key)));
}

/**
 * The objectives as an ordered list of rows, each with buttons that move it up and down, a check
 * box that turns it on, one that puts it in the level of the objective above, its weight, and for
 * the first of a level how far from the best a value still counts as equal to it. `set(text)`
 * fills them, and `read()` gives `{ value }`, the text of the layers the rows make, or `{ error }`.
 */
function objectiveRows(objectives) {
  const el = document.createElement("div");
  el.className = "objectives";
  const note = document.createElement("p");
  note.className = "dialog-note";
  setText(note, m("The solver ranks valid placements by these objective tiers from top to bottom. Objectives combined in the same tier are summed by weight; placements within the tier's tolerance of the best score tie and are broken by the next tier. Disabled objectives are skipped."));
  const list = document.createElement("ol");
  list.className = "objective-list";
  append(el, note, list);
  const info = new Map(objectives.map((o) => [o.name, o]));
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
    const about = info.get(r.name);
    const what = span("objective-what", about?.what ?? "");
    if (about?.implemented === false) append(what, " ", m("(not computed by this version)"));
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
    first: null,
    set(text) {
      rows = rowsOf(parseObjectives(text), objectives).map(make);
      list.replaceChildren(...rows.map((x) => x.el));
      refresh();
    },
    get() {
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
      return { value: objectivesText(layersOf(out)) };
    },
    shown: () => "",
  };
}

/**
 * The fields of the dialog, from the table, as an element: `set(values)` fills them with a value for each key
 * (`undefined` where a setting is not given), `read()` gives `{ values }` as they hold or `{ error }`, `focus` has the
 * focus at first, and `mark(keys)` marks the fields the solver refused.
 */
function fieldsOf(table) {
  const el = document.createElement("div");
  el.className = "settings";
  const fields = new Map(); // by key: { setting, control, line, use, required }
  const byGroup = new Map(table.groups.map((g) => [g.name, []]));
  for (const setting of table.settings) {
    if (!byGroup.has(setting.group)) byGroup.set(setting.group, []);
    const hint = span("settings-hint", setting.what ?? "");
    const kind = setting.type === "objectives"
      ? objectiveRows(table.objectives ?? [])
      : control(setting);
    // a setting with no default is given or not: a box says which
    const optional = setting.default === undefined && setting.type !== "boolean" && setting.type !== "objectives";
    const use = optional ? checkField(m("set")) : null;
    const line = document.createElement("div");
    line.className = "setting";
    line.dataset.key = setting.key;
    if (setting.type === "objectives") {
      append(line, span("settings-key", setting.key), kind.el);
    } else {
      // not a label: it holds the labels of its boxes, and a label holds no label
      const caption = document.createElement("div");
      caption.className = "settings-label";
      append(caption, span("name", setting.key), ...(use ? [use.caption] : []), ...kind.el);
      append(line, row(caption), hint);
    }
    if (use) {
      use.input.addEventListener("change", () => toggleUse(field));
    }
    const field = { setting, kind, line, use, hint };
    fields.set(setting.key, field);
    byGroup.get(setting.group).push(field);
  }
  /** A setting that is not given has its box left empty. */
  function toggleUse(field) {
    const on = field.use ? field.use.input.checked : true;
    for (const node of field.line.querySelectorAll(".settings-label input:not([type=checkbox]), .settings-label select")) node.disabled = !on;
  }
  for (const [name, group] of byGroup) {
    if (group.length === 0) continue;
    const set = document.createElement("fieldset");
    const legend = document.createElement("legend");
    setText(legend, table.groups.find((g) => g.name === name)?.what ?? name);
    append(set, legend, ...group.map((f) => f.line));
    append(el, set);
  }

  /** What each setting counts as in a condition: the text of its value, empty when it is not given. */
  const of = (key) => {
    const field = fields.get(key);
    if (!field || (field.use && !field.use.input.checked)) return "";
    return field.kind.shown();
  };
  /** Show each setting as what it is now: hidden, or greyed where the solver does not use it, or required. */
  function refresh() {
    for (const { setting, line, use } of fields.values()) {
      if (setting.shown_when) line.hidden = !holds(setting.shown_when, of);
      line.classList.toggle("unused", setting.applies !== undefined && !holds(setting.applies, of));
      if (setting.applies !== undefined) {
        setAttr(line, "title", holds(setting.applies, of) ? "" : m("The solver does not use this with the other settings as they are."));
      }
      if (setting.required_when && use) {
        const required = holds(setting.required_when, of);
        if (required) use.input.checked = true;
        use.input.disabled = required;
        toggleUse(fields.get(setting.key));
      }
    }
  }
  el.addEventListener("input", refresh);
  el.addEventListener("change", refresh);

  return {
    el,
    focus: fields.values().next().value.kind.first ?? null,
    set(values) {
      for (const { setting, kind, use, line } of fields.values()) {
        const value = values[setting.key];
        if (use) use.input.checked = value !== undefined;
        kind.set(value === undefined ? setting.default : value);
        line.classList.remove("refused");
        toggleUse(fields.get(setting.key));
      }
      refresh();
    },
    read() {
      const values = {};
      for (const { setting, kind, use } of fields.values()) {
        if (use && !use.input.checked) continue;
        const got = kind.get();
        if (got.error) return { error: got.error };
        values[setting.key] = got.value;
      }
      return { values };
    },
    mark(keys) {
      for (const [key, field] of fields) field.line.classList.toggle("refused", keys.includes(key));
    },
  };
}

/** The settings as the table has them, with the changed ones over the defaults: a value for each key, none where it has no default. */
function effective(table, values) {
  const out = {};
  for (const setting of table.settings) {
    const value = values[setting.key];
    out[setting.key] = value === undefined ? setting.default : value;
    if (value === undefined && setting.default === undefined) out[setting.key] = undefined;
  }
  return out;
}

/** What differs from the defaults in the values read from the dialog: the settings to keep. */
function differences(table, values) {
  const kept = {};
  for (const setting of table.settings) {
    const value = values[setting.key];
    if (value === undefined) continue;
    const same =
      setting.type === "objectives"
        ? typeof setting.default === "string" && objectivesText(parseObjectives(setting.default)) === value
        : JSON.stringify(value) === JSON.stringify(setting.default);
    if (!same) kept[setting.key] = value;
  }
  return kept;
}

/** The settings the netlist and its packages give to the level shown, from the report of the last solve: `[key, text, where]`. */
function givenByFile() {
  const title = doc?.levels?.[current]?.title;
  const level = lastLevels.find((l) => l.title === title) ?? (lastLevels.length === 1 ? lastLevels[0] : null);
  const settings = level?.check?.settings ?? {};
  return Object.entries(settings)
    .filter(([, s]) => s.from !== "default" && s.from !== "command-line")
    .map(([key, s]) => [key, typeof s.value === "string" ? JSON.stringify(s.value) : String(s.value), [s.from, s.at].filter(Boolean).join(", ")]);
}

/**
 * Solve > Settings: show the settings in a dialog, ask the shell whether the solver takes them, and
 * keep them for the next start. Cancel keeps nothing; Restore Defaults sets the fields to the
 * solver's defaults, which OK then keeps as no changes at all.
 */
export async function settings() {
  if (!shell || !shell.solverTable || modal()) return;
  let table;
  try {
    table = await shell.solverTable();
  } catch (e) {
    await inform({ title: m("Solver Settings"), paragraphs: [m`The solver's settings could not be read: ${e instanceof Error ? e.message : String(e)}`] });
    return;
  }
  const fields = fieldsOf(table);
  fields.set(effective(table, changed()));
  const error = document.createElement("p");
  error.className = "dialog-error";
  error.hidden = true;
  const restore = document.createElement("button");
  restore.type = "button";
  restore.className = "settings-restore";
  setText(restore, m("Restore Defaults"));
  const whole = timeallField();
  restore.addEventListener("click", () => {
    whole.reset();
    fields.set(effective(table, {}));
    fields.mark([]);
    error.hidden = true;
  });
  // the settings as the command line takes them, for whoever solves outside the viewer too
  const asText = span("settings-text", "");
  const showText = () => {
    const read = fields.read();
    const config = read.error ? null : configText(differences(table, read.values));
    const time = whole.read();
    const all = time.error || time.value === null ? "" : ` --timeall ${time.value}`;
    setText(asText, config === null ? "" : config || all ? m`Equivalent CLI command: rnl solve${all}${config ? ` --solver-config '${config}'` : ""}` : m("The defaults of rnl solve."));
  };
  for (const type of ["input", "change", "click"]) for (const part of [fields.el, whole.el]) part.addEventListener(type, showText);
  restore.addEventListener("click", showText);
  showText();
  const given = givenByFile();
  const parts = [priorityNote(), fields.el, whole.el];
  if (given.length > 0) parts.push(givenNote(given));
  parts.push(document.createElement("hr"), row(restore, asText), error);
  const chosen = await form({
    title: m("Solver Settings"),
    parts,
    answer: async () => {
      fields.mark([]);
      const read = fields.read();
      if (read.error) return fail(error, read.error);
      const time = whole.read();
      if (time.error) return fail(error, time.error);
      const kept = differences(table, read.values);
      if (shell.checkSettings) {
        let checked;
        try {
          checked = await shell.checkSettings(configText(kept));
        } catch (e) {
          return fail(error, e instanceof Error ? e.message : String(e));
        }
        const errors = (checked?.levels ?? []).flatMap((l) => l.errors ?? []);
        if (errors.length > 0) {
          fields.mark(errors.map((e) => e.key).filter(Boolean));
          return fail(error, errors.map((e) => e.message).join("; "));
        }
      }
      return { kept, timeall: time.value };
    },
    focus: fields.focus ?? undefined,
    wide: true,
  });
  if (chosen) {
    save(KEY, chosen.kept);
    save(TIMEALL, chosen.timeall);
  }
}

/**
 * The box of the time for the whole file: a check box that sets it and the seconds. Not a setting of the table, so it is
 * given to a solve beside them and not checked with them (`rnl solve --check` refuses it).
 */
function timeallField() {
  const input = document.createElement("input");
  input.type = "number";
  input.min = "0";
  input.step = "any";
  setAttr(input, "aria-label", "timeall");
  const use = checkField(m("set"));
  use.input.addEventListener("change", () => (input.disabled = !use.input.checked));
  const note = document.createElement("div");
  note.className = "settings-hint";
  setText(note, m("Seconds that every level with a time limit shares, the whole file's time: each level takes the least of its own seconds and an even share of what is left, and only its seed once nothing is left. Not read by a respace."));
  const line = document.createElement("div");
  line.className = "setting";
  const caption = document.createElement("div");
  caption.className = "settings-label";
  append(caption, span("settings-key", "timeall"), use.caption, input);
  append(line, caption, note);
  const set = (seconds) => {
    use.input.checked = seconds !== null;
    input.disabled = seconds === null;
    input.value = seconds === null ? "" : String(seconds);
  };
  set(keptTimeall());
  const box = document.createElement("fieldset");
  const legend = document.createElement("legend");
  setText(legend, m("Time for the whole file"));
  append(box, legend, line);
  return {
    el: box,
    reset: () => set(null),
    read() {
      if (!use.input.checked) return { value: null };
      const n = Number(input.value.trim());
      if (input.value.trim() === "" || !Number.isFinite(n) || n <= 0) return { error: m("timeall must be a positive number of seconds.") };
      return { value: n };
    },
  };
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
  setText(p, m`${m`${m("The priority of a constraint, low, normal or high, is written in the netlist (7.6, 8.5), not set here. ")}${m("These settings are for what the solver does after the constraints are satisfied: which layout it makes, which objective counts ")}`}${m("first and which next when it ranks the placements that satisfy them, and how long and how hard it works.")}`);
  return p;
}

/** What the netlist and its packages give, which a value set here overrides. */
function givenNote(given) {
  const box = document.createElement("div");
  box.className = "settings-given";
  const head = document.createElement("p");
  head.className = "dialog-note";
  setText(head, m("This netlist and its packages give these settings to the circuit shown; a value set here wins over them."));
  const list = document.createElement("ul");
  list.className = "settings-given-list";
  for (const [key, value, where] of given) {
    const li = document.createElement("li");
    append(li, span("key", key), span("value", ` = ${value}`), span("settings-hint", ` (${where})`));
    list.append(li);
  }
  append(box, head, list);
  return box;
}
