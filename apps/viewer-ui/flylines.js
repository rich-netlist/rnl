// View > Show Flylines: the flylines (13.7) of the level shown, the dashed lines for a wire through a point that has no
// position yet, which no wire can be drawn through. The core draws each as a group of class "flyline" in the level's SVG;
// hidden, they are hidden by a class on the page, so that showing them again needs no new drawing, and left out of
// File > Export Image (export-image.js). Off to begin with, and kept with the other View settings.

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

export function toggleFlylines() {
  shown = !shown;
  save("flylines-shown", shown);
  apply();
}
