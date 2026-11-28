// What the drawing says of objects with no position yet (the Not drawn list, unplaced.js), with two links: one that opens
// the Not drawn view, as its key (6) does, expanding a collapsed side panel, and one that solves the level shown, as
// Solve › Solve (F9) does: a bar over the drawing while the level shown has any and draws something,
// and a line in the page that says why nothing is drawn while it draws nothing (drawing.js). It is shown whenever such
// a level is shown, and asks nothing: no dialog opens on its own. Only where a solver is (`shell.solve`), never in an
// embedded drawing. The bar lies over the top of the drawing, so the drawing keeps its size.

import { m } from "./i18n.js";
import { shell } from "./shell.js";

const $ = (id) => document.getElementById(id);
// what stands for each link in a sentence, as a language words it: the sentence is split there
const VIEW = "\u0001";
const SOLVE = "\u0002";

/** Whether the level `lv` has objects with no position yet that a solve can place: a solver is there to place them. */
export const solvable = (lv) => !!shell?.solve && !shell.embed && (lv?.unplaced?.length ?? 0) > 0;

/** A link of the sentence: a button with `text` that runs `run`. */
function link(text, className, run) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = className;
  button.textContent = text;
  button.addEventListener("pointerdown", (e) => e.stopPropagation());
  button.addEventListener("click", run);
  return button;
}

/** Fill `span` with its sentence (`kind`: "bar" or "page"), the links in it, in the language shown. */
function fill(span) {
  const text = String(span.dataset.solveLink === "bar"
    ? m("Some objects have no position yet: see them in {0}, or try {1}.", VIEW, SOLVE)
    : m("You can see them in {0}, or try {1}.", VIEW, SOLVE));
  // (panels.js and solve.js draw through the page's modules, which draw this: they are looked up when a link is clicked)
  // the view by the name of its tab and its key, as the tab shows them
  const view = link(String(m("Not drawn (6)")), "solve-link unplaced-link", () => void import("./panels.js").then((p) => p.showPane("unplaced")));
  const solve = link(String(m("Solve (F9)")), "solve-link", () => void import("./solve.js").then((s) => s.solve()));
  const parts = [];
  for (const piece of text.split(/(\u0001|\u0002)/)) parts.push(piece === VIEW ? view : piece === SOLVE ? solve : piece);
  span.replaceChildren(...parts);
}

/** A sentence with the links that show the objects and solve them, for a bar or a page (`kind`). */
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
