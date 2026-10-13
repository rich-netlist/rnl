// The page's own menu bar, for a platform that shows none (shell.nativeMenu false), built from the
// command table. A click on a title opens its menu, pointing at another title while a menu is open
// switches to it, and Escape or a click elsewhere closes it; that click does nothing else. From the
// keyboard, Alt alone or F10 moves to the bar and Alt with an underlined letter opens a menu; then
// the arrows move, Enter chooses, and the underlined letters choose too.

import { command, isChecked, menus, shortcut } from "./commands.js";
import { modal } from "./dialog.js";
import { $ } from "./util.js";

const bar = $("menubar");
let built = false;
let at = -1; // the menu whose title is marked, or -1 while the bar is not in use
let open = false; // whether that menu's list is shown
let keyboard = false; // whether the bar is in use from the keyboard, which underlines the letters
let altAlone = false; // Alt is down, and no other key or click came since
let presses = 0; // the pointer presses so far
let closedBy = 0; // the press that last closed an open menu, or 0
let closing = false; // that press is not over yet

/** Show or hide the menu bar. */
export function showMenuBar(shown) {
  if (shown && !built) build();
  leave();
  bar.hidden = !shown;
  document.body.classList.toggle("menubar-shown", shown);
}

function build() {
  menus().forEach((m, i) => {
    const menu = document.createElement("div");
    menu.className = "menu";
    const title = document.createElement("button");
    title.type = "button";
    title.className = "menu-title";
    title.tabIndex = -1;
    title.setAttribute("role", "menuitem");
    title.setAttribute("aria-haspopup", "menu");
    title.setAttribute("aria-expanded", "false");
    const [text, letter] = label(m.label);
    title.dataset.letter = letter;
    title.append(text);
    // no focus moves to the bar, so that the keys act where they did once a menu closes
    title.addEventListener("pointerdown", (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      if (at === i && open) leave();
      else show(i, true, false);
    });
    title.addEventListener("pointerenter", () => {
      if (open && at !== i) show(i, true, keyboard);
    });
    const list = document.createElement("div");
    list.className = "menu-list";
    list.setAttribute("role", "menu");
    list.hidden = true;
    for (const item of m.items) list.append(item === "-" ? separator() : entry(item));
    list.addEventListener("pointerleave", () => mark(null));
    menu.append(title, list);
    bar.append(menu);
  });
  built = true;
}

/** A label's text with the letter after `&` underlined, and that letter in lower case. */
function label(text) {
  const el = document.createElement("span");
  el.className = "label";
  const amp = text.indexOf("&");
  if (amp < 0 || amp === text.length - 1) {
    el.textContent = text;
    return [el, ""];
  }
  const u = document.createElement("u");
  u.textContent = text[amp + 1];
  el.append(text.slice(0, amp), u, text.slice(amp + 2));
  return [el, text[amp + 1].toLowerCase()];
}

function separator() {
  const el = document.createElement("div");
  el.className = "menu-separator";
  el.setAttribute("role", "separator");
  return el;
}

function entry(item) {
  const b = document.createElement("button");
  b.type = "button";
  b.className = "menu-item";
  b.tabIndex = -1;
  b.dataset.id = item.id;
  b.setAttribute("role", item.check ? "menuitemcheckbox" : "menuitem");
  const check = document.createElement("span");
  check.className = "check";
  check.setAttribute("aria-hidden", "true");
  const [text, letter] = label(item.label);
  b.dataset.letter = letter;
  const keys = document.createElement("span");
  keys.className = "keys";
  keys.textContent = shortcut(item);
  b.append(check, text, keys);
  b.addEventListener("pointerdown", (e) => e.preventDefault());
  b.addEventListener("pointerenter", () => mark(b));
  b.addEventListener("click", () => choose(b));
  return b;
}

const titles = () => [...bar.querySelectorAll(".menu-title")];
const lists = () => [...bar.querySelectorAll(".menu-list")];
const items = () => (at < 0 ? [] : [...lists()[at].querySelectorAll(".menu-item")]);

/** Mark menu `i` and, with `withList`, open it; `fromKeys` when the keyboard uses the bar. */
function show(i, withList, fromKeys) {
  at = i;
  open = withList;
  keyboard = fromKeys;
  bar.classList.toggle("keys", keyboard);
  titles().forEach((t, k) => {
    t.classList.toggle("at", k === i);
    t.setAttribute("aria-expanded", String(k === i && withList));
  });
  lists().forEach((l, k) => {
    l.hidden = !(k === i && withList);
  });
  if (!withList) return;
  for (const b of items()) {
    if (b.getAttribute("role") === "menuitemcheckbox") b.setAttribute("aria-checked", String(isChecked(b.dataset.id)));
  }
  mark(keyboard ? items()[0] : null);
}

/** Close the open menu and leave the bar. */
function leave() {
  if (at < 0) return;
  at = -1;
  open = false;
  keyboard = false;
  bar.classList.remove("keys");
  for (const t of titles()) {
    t.classList.remove("at");
    t.setAttribute("aria-expanded", "false");
  }
  for (const l of lists()) l.hidden = true;
  mark(null);
}

function mark(item) {
  for (const b of bar.querySelectorAll(".menu-item.active")) b.classList.remove("active");
  if (item) item.classList.add("active");
}

function choose(item) {
  leave();
  command(item.dataset.id);
}

/** The menu whose underlined letter a key types, or -1. */
function titleWith(key) {
  const letter = key.length === 1 ? key.toLowerCase() : "";
  return letter ? titles().findIndex((t) => t.dataset.letter === letter) : -1;
}

/** Act on a key while the bar is in use; false lets the page have the key. */
function move(e) {
  if (e.ctrlKey || e.metaKey || e.key === "Tab") {
    leave();
    return false;
  }
  const n = titles().length;
  const list = items();
  const active = list.findIndex((b) => b.classList.contains("active"));
  switch (e.key) {
    case "Escape":
      if (open && keyboard) show(at, false, true);
      else leave();
      break;
    case "F10":
      leave();
      break;
    case "ArrowLeft":
    case "ArrowRight":
      show((at + (e.key === "ArrowLeft" ? n - 1 : 1)) % n, open, keyboard);
      break;
    case "ArrowDown":
    case "ArrowUp": {
      const up = e.key === "ArrowUp";
      if (!open) {
        show(at, true, keyboard);
        if (up) mark(items()[items().length - 1]);
      } else {
        mark(list[active < 0 ? (up ? list.length - 1 : 0) : (active + (up ? list.length - 1 : 1)) % list.length]);
      }
      break;
    }
    case "Enter":
    case " ":
      if (!open) show(at, true, keyboard);
      else if (active >= 0) choose(list[active]);
      break;
    default: {
      const letter = e.key.length === 1 ? e.key.toLowerCase() : "";
      const item = open && letter ? list.find((b) => b.dataset.letter === letter) : null;
      if (item) choose(item);
      else if (!open && titleWith(e.key) >= 0) show(titleWith(e.key), true, keyboard);
    }
  }
  return true;
}

// in the capture phase, so that the bar has the keys before the page while it is in use
document.addEventListener(
  "keydown",
  (e) => {
    if (bar.hidden || modal()) return;
    if (e.key === "Alt") {
      altAlone = !e.ctrlKey && !e.shiftKey && !e.metaKey;
      return;
    }
    altAlone = false;
    if (at >= 0) {
      if (!move(e)) return;
    } else if (e.key === "F10" && !e.altKey && !e.ctrlKey && !e.shiftKey && !e.metaKey) {
      show(0, false, true);
    } else if (e.altKey && !e.ctrlKey && !e.metaKey && titleWith(e.key) >= 0) {
      show(titleWith(e.key), true, true);
    } else {
      return;
    }
    e.preventDefault();
    e.stopPropagation();
  },
  true,
);

document.addEventListener(
  "keyup",
  (e) => {
    if (e.key !== "Alt" || !altAlone || bar.hidden || modal()) return;
    altAlone = false;
    if (at < 0) show(0, false, true);
    else leave();
    e.preventDefault();
  },
  true,
);

const swallow = (e) => {
  e.preventDefault();
  e.stopPropagation();
};

// a press anywhere but on a menu leaves the bar; as in a native menu, one that closes an open menu
// does nothing else: it neither selects nor clears the selection, nor starts a drag
document.addEventListener(
  "pointerdown",
  (e) => {
    altAlone = false;
    presses += 1;
    closing = false;
    if (at < 0 || e.target.closest("#menubar .menu")) return;
    if (open) {
      closing = true;
      closedBy = presses;
      swallow(e);
    }
    leave();
  },
  true,
);

// the rest of that press: its release, and the click or page menu that the release makes in the
// same task; the press is over in the next task, or at a key, which clicks on its own. A double
// click that the press begins goes too.
for (const type of ["pointerup", "pointercancel"]) {
  document.addEventListener(
    type,
    (e) => {
      if (!closing) return;
      swallow(e);
      const press = presses;
      setTimeout(() => {
        if (presses === press) closing = false;
      });
    },
    true,
  );
}

document.addEventListener(
  "keydown",
  () => {
    closing = false;
  },
  true,
);

for (const type of ["click", "auxclick", "contextmenu", "dblclick"]) {
  document.addEventListener(
    type,
    (e) => {
      if (closing || (type === "dblclick" && closedBy > 0 && presses === closedBy + 1)) swallow(e);
    },
    true,
  );
}

window.addEventListener("blur", leave);
