import { append, m, setAttr, setText, tr } from "./i18n.js";
// The parameters of the subcircuit shown, in the bar between the path and the drawing: the values the
// core gives (Level.params, and Child.effective for the instance it was entered through), each with
// where it comes from, and the dialog that changes them. The drawing keeps the text it is written
// with, `${w}`; the values show here. A change builds the Document again with the values bound to
// the subcircuit (`bind` of the build), which changes only these lists; nothing in the netlist does.
import { form, modal } from "./dialog.js";
import { changes } from "./edits.js";
import { currentSource, isLoading, openSource } from "./open.js";
import { setEnabled } from "./commands.js";
import { previewing } from "./solve.js";
import { shell } from "./shell.js";
import { bindings, current, doc, path } from "./state.js";
import { sameSource } from "./visit-source.js";
import { $, span } from "./util.js";

const bar = $("params-bar");
const FOLD_AT = 4; // more parameters than this show as a summary, until the bar is unfolded
let unfolded = false; // for as long as the page is open
let editing = false; // the dialog is open
const asked = new WeakMap(); // a Document -> the subcircuits it asked the values of when they were shown by themselves

/**
 * The parameters of level `i`, each with where it comes from: those the instance it was entered through
 * gives, when that is on the level above in this document, else the level's own, as its definition writes
 * them, or as they were bound; `via` is the instance's name when the values are its.
 */
export function paramsOf(i) {
  const level = doc?.levels[i];
  if (!level || level.kind !== "subckt") return { params: [], via: null };
  const bound = bindings.get(doc);
  if (bound && bound.subckt === level.title) {
    // the values the instance gave stay its, until they are changed
    return { params: (level.params || []).map((p) => p.from === "bound" && bound.supplied.has(p.name) ? { ...p, from: "instance" } : p), via: bound.via };
  }
  const [step, above] = [path[path.length - 1], path[path.length - 2]];
  if (step?.via !== null && step?.via !== undefined && above && sameSource(above.entry, currentSource())) {
    const child = doc.levels[above.level]?.children.find((c) => c.instance === step.via && c.level === i);
    if (child?.effective?.length) return { params: child.effective, via: step.via };
  }
  return { params: level.params || [], via: null };
}

/** Where a value comes from, as the bar says it. */
const origin = (p, via) =>
  p.from === "instance" ? m`from ${via}` : p.from === "bound" ? m`changed` : p.from === "unset" ? m`not set` : m`default`;

function item(p, via) {
  const unset = p.from === "unset" || p.value === null;
  const el = span(`param ${p.from}`, "");
  const name = document.createElement("b");
  // a value that cannot be evaluated stays as written, with a question mark
  append(name, `${p.name} = `, unset ? "?" : p.number === null ? `${p.value} ?` : p.value);
  append(el, name);
  if (!unset && p.number === null) setAttr(el, "title", m`This value cannot be evaluated; it is shown as written.`);
  const where = document.createElement("i");
  append(where, origin(p, via));
  append(el, where);
  return el;
}

/** The counts, by where the values come from: "3 from X1, 9 default". */
function summary(params, via) {
  const count = (from) => params.filter((p) => p.from === from).length;
  const parts = [];
  if (count("instance")) parts.push(m`${count("instance")} from ${via}`);
  if (count("bound")) parts.push(m`${count("bound")} changed`);
  if (count("default")) parts.push(m`${count("default")} default`);
  if (count("unset")) parts.push(m`${count("unset")} not set`);
  const text = document.createElement("span");
  append(text, m`${params.length} parameters`, ": ");
  parts.forEach((part, k) => append(text, k ? ", " : "", part));
  return text;
}

/** Whether the level shown has parameters to change: the Document is the one shown, not a solved preview. */
export const canEditParams = () => !!doc && !previewing() && paramsOf(current).params.length > 0;

/** Show the parameters of the level shown, or hide the bar when it has none. */
export function updateParams() {
  const { params, via } = paramsOf(current);
  bar.hidden = params.length === 0;
  bar.replaceChildren();
  setEnabled("edit-params", canEditParams());
  if (bar.hidden) return;
  const title = document.createElement("strong");
  title.className = "params-title";
  append(title, m`Parameters`);
  append(bar, title);
  const items = params.map((p) => item(p, via));
  const fold = document.createElement("button");
  fold.type = "button";
  fold.className = "params-fold";
  fold.addEventListener("click", () => { unfolded = !unfolded; updateParams(); });
  // as many as fit; more than a few, or a row that wraps, fold into the summary
  append(bar, ...items);
  const wraps = items.length > 1 && items[items.length - 1].offsetTop > items[0].offsetTop;
  const folds = params.length > FOLD_AT || wraps;
  if (folds && !unfolded) {
    items.forEach((el) => el.remove());
    append(bar, summary(params, via));
    append(fold, m`Show all`, " ▾");
    append(bar, fold);
  } else if (folds) {
    append(fold, m`Fold`, " ▴");
    append(bar, fold);
  }
  const edit = document.createElement("button");
  edit.type = "button";
  edit.className = "params-edit";
  append(edit, m`Edit Parameters...`);
  edit.addEventListener("click", () => void editParameters());
  append(bar, edit);
  if (via === null && params.some((p) => p.from === "unset")) void askMissing();
}

/**
 * A subcircuit shown by itself that has a parameter with no default, and so no value, asks for the values
 * before it is looked at, once for each Document; one with defaults, or reached through an instance, is
 * shown as it is, and the bar says so.
 */
async function askMissing() {
  const [shownDoc, title] = [doc, doc.levels[current].title];
  if (shell?.embed || bindings.has(doc) || asked.get(doc)?.has(title)) return;
  asked.set(doc, (asked.get(doc) || new Set()).add(title));
  // the dialog waits for what is opening the Document, and for another that is open
  for (let tries = 0; tries < 60 && (isLoading() || modal()); tries++) await new Promise((resolve) => setTimeout(resolve, 50));
  if (doc !== shownDoc || doc.levels[current].title !== title) return;
  await editParameters({ missing: true });
}

/** Hide the bar: no document is shown. */
export function hideParams() {
  bar.hidden = true;
  bar.replaceChildren();
  setEnabled("edit-params", false);
}

/** The values the instance gave, by name, as the level shown has them now. */
function givenBy(params, via) {
  const bound = bindings.get(doc);
  if (bound && bound.subckt === doc.levels[current].title) return bound.given;
  return new Map(via === null ? [] : params.filter((p) => p.from === "instance").map((p) => [p.name, p.value]));
}

/**
 * The dialog of the parameters of the level shown: a field for each, with the default and where the
 * value comes from. Apply builds the Document again with the values written, and an empty field is
 * the default; that shows the same level, view and selection. Resolves to whether it did.
 */
export async function editParameters({ missing = false } = {}) {
  if (editing || modal() || !canEditParams() || isLoading()) return false;
  editing = true;
  try {
    const level = doc.levels[current];
    const { params, via } = paramsOf(current);
    const given = givenBy(params, via);
    const inputs = [];
    const table = document.createElement("table");
    table.className = "params-table";
    const head = table.createTHead().insertRow();
    for (const label of [m`Parameter`, m`Default`, m`Value`, m`Source`]) {
      const cell = document.createElement("th");
      append(cell, label);
      head.append(cell);
    }
    const body = table.createTBody();
    for (const p of params) {
      const row = body.insertRow();
      const name = row.insertCell();
      name.className = "params-name";
      name.textContent = p.name;
      const base = row.insertCell();
      base.className = "params-default";
      base.textContent = p.default ?? "";
      const input = document.createElement("input");
      input.type = "text";
      input.value = p.value ?? "";
      input.placeholder = p.default ?? "";
      input.spellcheck = false;
      input.autocomplete = "off";
      setAttr(input, "aria-label", m`Value of ${p.name}`);
      row.insertCell().append(input);
      const where = row.insertCell();
      where.className = "params-source";
      append(where, origin(p, via));
      inputs.push(input);
    }
    const restore = document.createElement("button");
    restore.type = "button";
    append(restore, via === null ? m("Restore Defaults") : m`Restore the values of ${via}`);
    restore.addEventListener("click", () => params.forEach((p, k) => { inputs[k].value = given.get(p.name) ?? ""; }));
    const result = await form({
      title: m`Parameters of ${level.title}`,
      text: missing
        ? m`Some parameters have no default, so this view has no value for them. Write the values to use; a change applies to this view only, and the netlist is not changed.`
        : m`A change applies to this view only; the netlist is not changed. Values are written as in the netlist, and a value left empty is the parameter's default.`,
      parts: [table], footer: [restore], focus: inputs[Math.max(0, params.findIndex((p) => p.from === "unset"))], action: tr("Apply"), wide: true,
      answer: () => {
        // what is bound: the values that are not the default, as written
        const values = {};
        params.forEach((p, k) => {
          const text = inputs[k].value.trim();
          if (text !== "" && text !== p.default) values[p.name] = text;
        });
        return values;
      },
    });
    if (!result) return false;
    const now = Object.fromEntries(params.filter((p) => p.from !== "default" && p.from !== "unset").map((p) => [p.name, p.value]));
    if (JSON.stringify(Object.entries(result).sort()) === JSON.stringify(Object.entries(now).sort())) return false;
    const supplied = new Set(Object.keys(result).filter((name) => given.get(name) === result[name]));
    const record = { subckt: level.title, via, given, supplied };
    return (await openSource(currentSource(), true, changes(true), false, null, null, true, { bind: { subckt: level.title, params: result }, bound: record })) === true;
  } finally {
    editing = false;
  }
}

window.addEventListener("resize", () => { if (doc && !bar.hidden) updateParams(); });
