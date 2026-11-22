// What the drawing says of objects with no position yet (the Not drawn list, unplaced.js), with a link that solves the
// level shown, as Solve › Solve (F9) does: a bar over the drawing while the level shown has any and draws something,
// and a line in the page that says why nothing is drawn while it draws nothing (drawing.js). It is shown whenever such
// a level is shown, and asks nothing: no dialog opens on its own. Only where a solver is (`shell.solve`), never in an
// embedded drawing. The bar lies over the top of the drawing, so the drawing keeps its size.

import { m } from "./i18n.js";
import { shell } from "./shell.js";

const $ = (id) => document.getElementById(id);
// what stands for the link in a sentence, as a language words it: the sentence is split there
const LINK = "\u0001";

/** Whether the level `lv` has objects with no position yet that a solve can place: a solver is there to place them. */
export const solvable = (lv) => !!shell?.solve && !shell.embed && (lv?.unplaced?.length ?? 0) > 0;

/** Fill `span` with its sentence (`kind`: "bar" or "page"), the link in it, in the language shown. */
function fill(span) {
  const text = String(span.dataset.solveLink === "bar"
    ? m("Some objects have no position yet; you can try {0}.", LINK)
    : m("You can try {0}.", LINK));
  const [before, after = ""] = text.split(LINK);
  const link = document.createElement("button");
  link.type = "button";
  link.className = "solve-link";
  link.textContent = String(m("Solve (F9)"));
  // (solve.js draws through the page's modules, which draw this: it is looked up when it is clicked)
  link.addEventListener("pointerdown", (e) => e.stopPropagation());
  link.addEventListener("click", () => void import("./solve.js").then((s) => s.solve()));
  span.replaceChildren(before, link, after);
}

/** A sentence with the link that solves, for a bar or a page (`kind`). */
function sentence(kind) {
  const span = document.createElement("span");
  span.dataset.solveLink = kind;
  fill(span);
  return span;
}

/** Show the bar over the drawing when `lv`, the level shown and drawn, has objects with no position yet; hide it otherwise. */
export function showUnplacedBar(lv) {
  const bar = $("unplaced-bar");
  if (!bar) return;
  const on = !!lv && !lv.empty && solvable(lv);
  if (on && !bar.firstChild) bar.append(sentence("bar"));
  bar.hidden = !on;
}

/** The line of the page of a level that draws nothing, `lv`, that offers to solve it: null when there is nothing to place, or no solver. */
export const unplacedLine = (lv) => (solvable(lv) ? sentence("page") : null);

// the bar is the bar's, not the drawing's: a press on it is not a press on the drawing, which would take the pointer and
// the click with it, as the badge that enters a subcircuit is not (drawing.js)
for (const kind of ["pointerdown", "dblclick"]) $("unplaced-bar")?.addEventListener(kind, (e) => e.stopPropagation());

window.addEventListener("language-changed", () => {
  for (const span of document.querySelectorAll("[data-solve-link]")) fill(span);
});
