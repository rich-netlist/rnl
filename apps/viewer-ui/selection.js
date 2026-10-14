// The selection: an instance or a net, followed between the drawing and the text.

import { current, doc, selected, setSelected } from "./state.js";
import { paintDrawingSelection, showLevel } from "./drawing.js";
import { opens } from "./subcircuits.js";
import { paintSourceSelection } from "./text.js";
import { $ } from "./util.js";

const canvas = $("canvas");
const source = $("source");

/** Select an object. `from` is "drawing", "source" or "reload". */
export function select(o, from) {
  if (!doc || o === undefined || o === null || !doc.objects[o]) return;
  setSelected(o);
  const obj = doc.objects[o];
  // a name picked in the text is shown on the level that has it
  if (from === "source" && (!doc.levels[current] || doc.levels[current].host !== obj.level)) {
    const k = doc.levels.findIndex((l) => l.host === obj.level);
    if (k >= 0) showLevel(k, null, null);
  }
  markSubcircuit(opens(o) !== null);
  paintDrawingSelection();
  paintSourceSelection(from === "drawing");
  const sel = $("selection");
  sel.replaceChildren();
  const b = document.createElement("b");
  b.textContent = obj.name;
  if (opens(o) !== null) {
    sel.append(b, ` instance of ${doc.levels[obj.opens].title} in ${obj.level_title} · Enter opens it`);
  } else {
    sel.append(b, ` ${obj.kind === "instance" ? "instance" : "net"} in ${obj.level_title}`);
  }
}

export function clearSelection() {
  if (selected === null) return;
  setSelected(null);
  markSubcircuit(false);
  paintDrawingSelection();
  paintSourceSelection(false);
  $("selection").textContent = "";
}

/** A selected subcircuit instance is marked in its own colour, in the drawing and the text. */
function markSubcircuit(on) {
  for (const el of [canvas, source, $("selection")]) el.classList.toggle("sel-sub", on);
}
