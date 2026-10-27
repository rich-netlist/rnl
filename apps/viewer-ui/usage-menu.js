// The context menu of a reference in the netlist text, `#M1` or `&vdd` or `%dp1`: Show its usages selects the
// object and shows the usages it is a member of, in the lower half of the Usage panel (usage-table.js).

import { m, setText } from "./i18n.js";
import { setPane, setSidebar } from "./panels.js";
import { select } from "./selection.js";
import { doc } from "./state.js";
import { $ } from "./util.js";

const source = $("source");
let menu = null;

function close() {
  menu?.remove();
  menu = null;
}

/** Select `o` as an object, even a usage, and show the usages it is a member of. */
export function showUsagesOf(o) {
  select(o, "source", { usage: false });
  setSidebar(true);
  setPane("usages");
}

function open(x, y, o) {
  close();
  menu = document.createElement("div");
  menu.className = "context-menu";
  menu.setAttribute("role", "menu");
  const item = document.createElement("button");
  item.type = "button";
  item.className = "context-item";
  item.setAttribute("role", "menuitem");
  setText(item, m("Show its usages"));
  item.addEventListener("click", () => { close(); showUsagesOf(o); });
  menu.append(item);
  document.body.append(menu);
  const { width, height } = menu.getBoundingClientRect();
  menu.style.left = `${Math.max(0, Math.min(x, window.innerWidth - width))}px`;
  menu.style.top = `${Math.max(0, Math.min(y, window.innerHeight - height))}px`;
  item.focus();
}

source.addEventListener("contextmenu", (e) => {
  const place = e.target.closest("[data-obj]");
  if (!doc || !place) return;
  e.preventDefault();
  // the first object of a place is the one written most narrowly: `#M1` in `#M1:d` is M1
  open(e.clientX, e.clientY, Number(place.dataset.obj.split(" ")[0]));
});
document.addEventListener("pointerdown", (e) => { if (menu && !menu.contains(e.target)) close(); }, true);
document.addEventListener("keydown", (e) => { if (menu && e.key === "Escape") { close(); e.stopPropagation(); } }, true);
window.addEventListener("blur", close);
window.addEventListener("resize", close);
source.addEventListener("scroll", close, { passive: true });
window.addEventListener("language-changed", close);
