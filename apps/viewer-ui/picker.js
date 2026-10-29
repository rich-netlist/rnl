// The list that opens beside the pointer when a click on the drawing is on several objects: a row
// for each, with its kind and its name, to choose from with the mouse or with the keys (Up and
// Down, Home and End, Enter to choose, Escape to close). Clicking anywhere else closes it.

import { m, tr } from "./i18n.js";
import { doc } from "./state.js";
import { $, span } from "./util.js";

/** The kinds of the objects of a Document, as the picker and the selection name them. */
const OBJECT_KINDS = { instance: tr("instance"), net: tr("net"), usage: tr("usage"), group: tr("group") };
export const kindName = (kind) => m(OBJECT_KINDS[kind] ?? kind);

const list = $("pick-list");
const GAP = 6; // pixels from the pointer to the list
let rows = [];
let active = 0;
let chosen = null; // what choosing a row does: called with the object, an index into doc.objects

/** Whether the list is open. */
export function pickerOpen() {
  return !list.hidden;
}

/**
 * Open the list of `objects`, indexes into doc.objects, beside the window point (x, y); choosing
 * one calls `onChoose` with it. The first row is the one Enter chooses.
 */
export function openPicker(objects, x, y, onChoose) {
  closePicker();
  chosen = onChoose;
  list.replaceChildren();
  rows = objects.map((o, k) => {
    const obj = doc.objects[o];
    const row = document.createElement("button");
    row.type = "button";
    row.className = "pick-row";
    row.id = `pick-row-${k}`;
    row.tabIndex = -1;
    row.setAttribute("role", "option");
    row.dataset.object = String(o);
    row.append(span("kind", kindName(obj.kind)), span("name", obj.name));
    row.addEventListener("pointermove", () => activate(k));
    row.addEventListener("click", () => choose(k));
    return row;
  });
  list.append(...rows);
  list.hidden = false;
  const { width, height } = list.getBoundingClientRect();
  const left = x + GAP + width > window.innerWidth ? x - GAP - width : x + GAP;
  const top = y + GAP + height > window.innerHeight ? y - GAP - height : y + GAP;
  list.style.left = `${Math.max(0, left)}px`;
  list.style.top = `${Math.max(0, top)}px`;
  activate(0);
  list.focus({ preventScroll: true });
}

/** Close the list, and give the focus back to the drawing if the list had it. */
export function closePicker() {
  if (list.hidden) return;
  const focused = list.contains(document.activeElement) || document.activeElement === list;
  list.hidden = true;
  list.replaceChildren();
  rows = [];
  chosen = null;
  if (focused) $("canvas").focus({ preventScroll: true });
}

function activate(k) {
  active = (k + rows.length) % rows.length;
  rows.forEach((row, i) => row.setAttribute("aria-selected", String(i === active)));
  list.setAttribute("aria-activedescendant", rows[active].id);
  rows[active].scrollIntoView({ block: "nearest" });
}

function choose(k) {
  const o = Number(rows[k].dataset.object);
  const done = chosen;
  closePicker();
  if (done) done(o);
}

list.addEventListener("keydown", (e) => {
  const step = { ArrowDown: 1, ArrowUp: -1 }[e.key];
  if (step !== undefined) activate(active + step);
  else if (e.key === "Home") activate(0);
  else if (e.key === "End") activate(rows.length - 1);
  else if (e.key === "Enter" || e.key === " ") choose(active);
  else if (e.key === "Escape") closePicker();
  else if (e.key === "Tab") closePicker();
  else return;
  // the keys of the list are not the page's: Escape would clear the selection, Enter open a subcircuit
  if (e.key !== "Tab") {
    e.preventDefault();
    e.stopPropagation();
  }
});

// a click anywhere but on the list closes it; so does the window losing the focus or the view moving
document.addEventListener(
  "pointerdown",
  (e) => {
    if (!list.hidden && !list.contains(e.target)) closePicker();
  },
  true,
);
window.addEventListener("blur", closePicker);
window.addEventListener("resize", closePicker);
// the wheel over the list scrolls it, when it has more rows than fit; elsewhere it moves the view
document.addEventListener(
  "wheel",
  (e) => {
    if (!list.contains(e.target)) closePicker();
  },
  { passive: true },
);
