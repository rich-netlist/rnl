import { append, m, setText } from "./i18n.js";
// The selection: an instance, a net or a virtual symbol, followed between drawing and text.

import { current, doc, selected, setSelected } from "./state.js";
import { paintDrawingSelection } from "./drawing.js";
import { kindName } from "./picker.js";
import { candidatesOf, canOpen, goTo, opens } from "./subcircuits.js";
import { paintSourceSelection } from "./text.js";
import { $ } from "./util.js";
import { setEnabled } from "./commands.js";
import { usageSelection } from "./usages.js";

const canvas = $("canvas");
const source = $("source");
const touch = matchMedia("(pointer: coarse)");

/**
 * Select an object. `from` is "drawing", "source", "reload", "ascend" or "list" (an object not
 * drawn, picked in the side panel); the text shows where the object is named when it is selected on
 * the drawing, or is the instance a level was left through.
 */
export function select(o, from, options) {
  if (!doc || o === undefined || o === null || !doc.objects[o]) return;
  const obj = doc.objects[o];
  // a name picked in the text is shown on the level that has it, the path cut back to the level or
  // starting there
  if (from === "source" && (!doc.levels[current] || doc.levels[current].host !== obj.level)) {
    const k = doc.levels.findIndex((l) => l.host === obj.level);
    if (k >= 0) return goTo(k, o, { ...options, usage: obj.kind === "usage" || obj.kind === "group" });
  }
  if (options?.newWindow) return;
  setSelected(o, options?.usage ?? usageSelection(o, from));
  markSubcircuit(canOpen(o));
  paintDrawingSelection();
  paintSourceSelection(from === "drawing" || from === "ascend" || from === "usage");
  window.dispatchEvent(new Event("viewer-selection"));
  const sel = $("selection");
  sel.replaceChildren();
  const b = document.createElement("b");
  setText(b, obj.name);
  if (opens(o) === null && candidatesOf(o).length > 1) {
    // several definitions in the Library, none chosen: entering asks which
    const n = candidatesOf(o).length;
    append(sel, b, touch.matches
      ? m` instance of a subcircuit with ${n} definitions in ${obj.level_title} · a double tap asks which to open`
      : m` instance of a subcircuit with ${n} definitions in ${obj.level_title} · E or Enter asks which to open; Shift opens a new window`);
  } else if (opens(o) !== null) {
    append(sel, b, touch.matches
      ? m` instance of ${doc.levels[obj.opens].title} in ${obj.level_title} · a double tap opens it`
      : m` instance of ${doc.levels[obj.opens].title} in ${obj.level_title} · E or Enter opens it; Shift opens a new window`);
  } else {
    append(sel, b, m` ${kindName(obj.kind)} in ${obj.level_title}`);
  }
}

export function clearSelection() {
  if (selected === null) return;
  setSelected(null);
  markSubcircuit(false);
  paintDrawingSelection();
  paintSourceSelection(false);
  setText($("selection"), "");
  window.dispatchEvent(new Event("viewer-selection"));
}

/** A selected subcircuit instance is marked in its own colour, in the drawing and the text. */
function markSubcircuit(on) {
  setEnabled("descend-new-window", on);
  for (const el of [canvas, source, $("selection")]) el.classList.toggle("sel-sub", on);
}
