import { append, m, setAttr, setText } from "./i18n.js";
// The parameters of the subcircuit shown, in the bar between the path and the drawing: the values the
// core gives (Level.params, and Child.effective for the instance it was entered through), each with
// where it comes from. The drawing keeps the text it is written with, `${w}`; the values show here.
import { currentSource } from "./open.js";
import { current, doc, path } from "./state.js";
import { sameSource } from "./visit-source.js";
import { $, span } from "./util.js";

const bar = $("params-bar");
const FOLD_AT = 4; // more parameters than this show as a summary, until the bar is unfolded
let unfolded = false; // for as long as the page is open

/**
 * The parameters of level `i`, each with where it comes from: those the instance it was entered through
 * gives, when that is on the level above in this document, else the level's own, as its definition writes
 * them. `via` is the instance's name when the values are its.
 */
export function paramsOf(i) {
  const level = doc?.levels[i];
  if (!level || level.kind !== "subckt") return { params: [], via: null };
  const [step, above] = [path[path.length - 1], path[path.length - 2]];
  if (step?.via !== null && step?.via !== undefined && above && sameSource(above.entry, currentSource())) {
    const child = doc.levels[above.level]?.children.find((c) => c.instance === step.via && c.level === i);
    if (child?.effective?.length) return { params: child.effective, via: step.via };
  }
  return { params: level.params || [], via: null };
}

/** Where a value comes from, as the strip says it. */
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

/** Show the parameters of the level shown, or hide the bar when it has none. */
export function updateParams() {
  const { params, via } = paramsOf(current);
  bar.hidden = params.length === 0;
  bar.replaceChildren();
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
  if (!unfolded && (params.length > FOLD_AT || wraps)) {
    items.forEach((el) => el.remove());
    append(bar, summary(params, via));
    append(fold, m`Show all`, " ▾");
    append(bar, fold);
  } else if (unfolded && (params.length > FOLD_AT || wraps)) {
    append(fold, m`Fold`, " ▴");
    append(bar, fold);
  }
}

/** Hide the bar: no document is shown. */
export function hideParams() {
  bar.hidden = true;
  bar.replaceChildren();
}

window.addEventListener("resize", () => { if (doc && !bar.hidden) updateParams(); });
