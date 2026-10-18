// The name shown beside the pointer while it rests on an instance or a net of the drawing (View >
// Show Names on Hover). The box is beside the pointer, not under it, takes no clicks, and goes with
// the pointer: the part of the drawing pointed at stays in view. The choice is kept for the next start.

import { setCheck } from "./commands.js";
import { $, clamp, save } from "./util.js";

const tip = $("hover-tip");
const GAP_X = 14; // pixels from the pointer to the box, to the right
const GAP_Y = 18; // and below
let on = true;

/** Whether names are shown on hover. */
export function hoverNamesOn() {
  return on;
}

/** Show or hide names on hover, and keep the choice; the menu's check mark follows. */
export function setHoverNames(shown) {
  on = shown;
  save("hover-names", on);
  setCheck("toggle-hover-names", on);
  if (!on) hideTip();
}

/** Show `text` beside the window point (x, y), moving the box if it is already shown. */
export function showTip(text, x, y) {
  if (tip.textContent !== text) tip.textContent = text;
  tip.hidden = false;
  const { width, height } = tip.getBoundingClientRect();
  // to the left of and above the pointer where there is no room on the other side
  const left = x + GAP_X + width > window.innerWidth ? x - GAP_X - width : x + GAP_X;
  const top = y + GAP_Y + height > window.innerHeight ? y - GAP_Y - height : y + GAP_Y;
  tip.style.left = `${clamp(left, 0, Math.max(0, window.innerWidth - width))}px`;
  tip.style.top = `${clamp(top, 0, Math.max(0, window.innerHeight - height))}px`;
}

export function hideTip() {
  tip.hidden = true;
}

// a key may open a dialog or move the view under a pointer that rests: the name goes, and comes
// back with the pointer's next move
document.addEventListener("keydown", hideTip, true);
