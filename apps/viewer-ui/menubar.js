// The page's own menu bar, for a platform that shows none (shell.nativeMenu false), built from the
// command table. A click on a title opens its menu, pointing at another title while a menu is open
// switches to it, and Escape or a click elsewhere closes it; that click does nothing else. Pointing
// at a submenu's entry opens the submenu beside it, and pointing at another entry of its menu closes
// it. From the keyboard, Alt alone or F10 moves to the bar and Alt with an underlined letter opens a
// menu; then the arrows move, ArrowRight opens a submenu and ArrowLeft closes it, Enter chooses, and
// the underlined letters choose too.

import { command, isChecked, menus, shortcut } from "./commands.js";
import { modal } from "./dialog.js";
import { $ } from "./util.js";

const bar = $("menubar");
let built = false;
let at = -1; // the menu whose title is marked, or -1 while the bar is not in use
let open = false; // whether that menu's list is shown
let subs = []; // the entries whose submenus are open in it, the outermost first
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
    menu.append(title, list(m.items, "menu-list"));
    bar.append(menu);
  });
  built = true;
}

/** The list of a menu's entries, or of a submenu's ("submenu"). */
function list(items, cls) {
  const el = document.createElement("div");
  el.className = cls;
  el.setAttribute("role", "menu");
  el.hidden = true;
  for (const item of items) el.append(item === "-" ? separator() : item.items ? submenu(item) : entry(item));
  // the mark leaves with the pointer, but for the entry whose submenu is open
  el.addEventListener("pointerleave", () => mark(el, subs.find((b) => listIn(b) === el) || null));
  return el;
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

/** An entry of a list: a check mark, the label and the keys. */
function button(text, role) {
  const b = document.createElement("button");
  b.type = "button";
  b.className = "menu-item";
  b.tabIndex = -1;
  b.setAttribute("role", role);
  const check = document.createElement("span");
  check.className = "check";
  check.setAttribute("aria-hidden", "true");
  const [el, letter] = label(text);
  b.dataset.letter = letter;
  const keys = document.createElement("span");
  keys.className = "keys";
  b.append(check, el, keys);
  b.addEventListener("pointerdown", (e) => e.preventDefault());
  b.addEventListener("pointerenter", () => point(b));
  return b;
}

function entry(item) {
  const b = button(item.label, item.radio ? "menuitemradio" : item.check ? "menuitemcheckbox" : "menuitem");
  b.dataset.id = item.id;
  b.lastChild.textContent = shortcut(item);
  b.addEventListener("click", () => choose(b));
  return b;
}

/** A submenu's entry, with the submenu's list next to it. */
function submenu(item) {
  const el = document.createElement("div");
  el.className = "menu-sub";
  const b = button(item.label, "menuitem");
  b.classList.add("opens");
  b.setAttribute("aria-haspopup", "menu");
  b.setAttribute("aria-expanded", "false");
  b.addEventListener("click", () => openSub(b, false));
  el.append(b, list(item.items, "submenu"));
  return el;
}

const titles = () => [...bar.querySelectorAll(".menu-title")];
const lists = () => [...bar.querySelectorAll(".menu-list")];
/** The entries of a list, without those of its submenus. */
const entriesOf = (el) =>
  [...el.children]
    .map((c) => (c.classList.contains("menu-sub") ? c.firstElementChild : c))
    .filter((c) => c.classList.contains("menu-item"));
/** The list an entry is in, and the list of a submenu's entry. */
const listIn = (b) => b.closest(".menu-list, .submenu");
const subList = (b) => b.nextElementSibling;
/** The list the keys act on: the innermost submenu open, or the open menu's own. */
const keyList = () => (subs.length > 0 ? subList(subs[subs.length - 1]) : lists()[at]);
const opensSub = (b) => !!b && b.classList.contains("opens");

/** Mark menu `i` and, with `withList`, open it; `fromKeys` when the keyboard uses the bar. */
function show(i, withList, fromKeys) {
  closeSubs(0);
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
    mark(l, null);
  });
  if (!withList) return;
  showChecks(lists()[i]);
  mark(lists()[i], keyboard ? entriesOf(lists()[i])[0] : null);
}

/** Close the open menu and leave the bar. */
function leave() {
  if (at < 0) return;
  closeSubs(0);
  at = -1;
  open = false;
  keyboard = false;
  bar.classList.remove("keys");
  for (const t of titles()) {
    t.classList.remove("at");
    t.setAttribute("aria-expanded", "false");
  }
  for (const l of lists()) {
    l.hidden = true;
    mark(l, null);
  }
}

/** Show the check marks of a list's entries as the commands have them. */
function showChecks(el) {
  for (const b of entriesOf(el)) {
    if (!b.dataset.id || b.getAttribute("role") === "menuitem") continue;
    b.setAttribute("aria-checked", String(isChecked(b.dataset.id)));
  }
}

/** Mark an entry of a list, or none. */
function mark(el, item) {
  for (const b of entriesOf(el)) b.classList.toggle("active", b === item);
}

/** How many submenus are open down to a list: 0 for a menu's own list. */
function depthOf(el) {
  return subs.findIndex((b) => subList(b) === el) + 1;
}

/** Close the submenus open below the first `depth`. */
function closeSubs(depth) {
  for (const b of subs.splice(depth)) {
    b.setAttribute("aria-expanded", "false");
    subList(b).hidden = true;
    mark(subList(b), null);
  }
}

/**
 * Open the submenu of an entry beside it, on its right, or on its left where the window has no
 * room; with `fromKeys`, mark its first entry.
 */
function openSub(b, fromKeys) {
  const el = listIn(b);
  const depth = depthOf(el);
  const sub = subList(b);
  if (subs[depth] !== b) {
    closeSubs(depth);
    subs.push(b);
    b.setAttribute("aria-expanded", "true");
    sub.classList.remove("left");
    sub.hidden = false;
    if (sub.getBoundingClientRect().right > document.documentElement.clientWidth) sub.classList.add("left");
    showChecks(sub);
  }
  mark(el, b);
  mark(sub, fromKeys ? entriesOf(sub)[0] : null);
}

/** The pointer is on an entry: mark it, close the submenus open from its list, and open its own. */
function point(b) {
  const el = listIn(b);
  if (opensSub(b)) {
    openSub(b, false);
    return;
  }
  closeSubs(depthOf(el));
  mark(el, b);
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
  const el = open ? keyList() : null;
  const list = el ? entriesOf(el) : [];
  const active = list.findIndex((b) => b.classList.contains("active"));
  switch (e.key) {
    case "Escape":
      if (subs.length > 0) closeSubs(subs.length - 1);
      else if (open && keyboard) show(at, false, true);
      else leave();
      break;
    case "F10":
      leave();
      break;
    case "ArrowLeft":
      if (subs.length > 0) closeSubs(subs.length - 1);
      else show((at + n - 1) % n, open, keyboard);
      break;
    case "ArrowRight":
      if (opensSub(list[active])) openSub(list[active], true);
      else show((at + 1) % n, open, keyboard);
      break;
    case "ArrowDown":
    case "ArrowUp": {
      const up = e.key === "ArrowUp";
      if (!open) {
        show(at, true, keyboard);
        const first = entriesOf(lists()[at]);
        if (up) mark(lists()[at], first[first.length - 1]);
      } else {
        mark(el, list[active < 0 ? (up ? list.length - 1 : 0) : (active + (up ? list.length - 1 : 1)) % list.length]);
      }
      break;
    }
    case "Enter":
    case " ":
      if (!open) show(at, true, keyboard);
      else if (opensSub(list[active])) openSub(list[active], true);
      else if (active >= 0) choose(list[active]);
      break;
    default: {
      const letter = e.key.length === 1 ? e.key.toLowerCase() : "";
      const item = open && letter ? list.find((b) => b.dataset.letter === letter) : null;
      if (opensSub(item)) openSub(item, true);
      else if (item) choose(item);
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
