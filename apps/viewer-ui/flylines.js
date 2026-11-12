// View > Show Flylines: the flylines (13.3, 13.7) of the level shown, the thin dashed lines between the connected parts of
// every net and over the gaps of a wire through a point that has no position yet, to check what is not wired. The core draws
// each as a group of class "flyline" in the level's SVG, and gives a hit for it that stands for its net (hits.js); the page
// asks for them always. Hidden, they are hidden by a class on the page, so that showing them again needs no new drawing, and
// they are not hit, and left out of File > Export Image (export-image.js). Off to begin with, and kept with the other View settings.

import { setCheck } from "./commands.js";
import { load, save } from "./util.js";

let shown = false;

/** Whether the flylines are shown. */
export const flylinesShown = () => shown;

function apply() {
  document.documentElement.classList.toggle("flylines-hidden", !shown);
  setCheck("toggle-flylines", shown);
}

export function startFlylines() {
  shown = load("flylines-shown", false) === true;
  apply();
}

export function setFlylines(value, { persist = true } = {}) {
  shown = !!value;
  if (persist) save("flylines-shown", shown);
  apply();
}

export function toggleFlylines() {
  setFlylines(!shown);
}
