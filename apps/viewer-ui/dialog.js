// Dialogs in the page's style, for what a shell asks of the user: one of a list, such as the netlist
// to open; a line of text, such as a URL; several values at once, such as the grid's settings; a
// message, such as the About box; a message with a button that acts, such as one that opens a
// release's page; or a message with several, such as the one that asks what to do with changes not
// saved. Each is modal and returns a promise of the answer, null when it is cancelled.
// While one is shown, the page's keys and its menu bar do nothing.

const LIST_MAX = 500; // the items a list shows at most; the filter finds the others

let shown = 0; // the dialogs shown
let made = 0; // the dialogs made so far, for the ids of their titles

/** Whether a dialog is shown. */
export function modal() {
  return shown > 0;
}

/**
 * Ask for one of `items`, each `{ label, value }`, listed in the order given. With `filter`, a box
 * above the list keeps the items whose label has its text, whatever the case. Enter or a double
 * click chooses the marked item.
 *
 * @param {{ title: string, text?: string, items: { label: string, value: any }[], filter?: boolean,
 *   action?: string }} options `action` labels the button that chooses, "Open" by default.
 * @returns {Promise<any>} The value of the item chosen, or null when the dialog is cancelled.
 */
export function choose({ title, text, items, filter = false, action = "Open" }) {
  const d = frame(title, text, action);
  const box = filter ? textBox("search", "Filter by name") : null;
  const list = document.createElement("div");
  list.className = "dialog-list";
  list.id = `${d.id}-list`;
  list.tabIndex = 0;
  list.setAttribute("role", "listbox");
  list.setAttribute("aria-labelledby", d.id);
  const note = document.createElement("p");
  note.className = "dialog-note";
  let matching = items; // the items the filter keeps
  let at = 0; // the marked one among them, while there is one

  function render() {
    const q = box ? box.value.trim().toLowerCase() : "";
    matching = q ? items.filter((x) => x.label.toLowerCase().includes(q)) : items;
    list.replaceChildren(
      ...matching.slice(0, LIST_MAX).map((x, k) => {
        const option = document.createElement("div");
        option.className = "dialog-option";
        option.id = `${list.id}-${k}`;
        option.setAttribute("role", "option");
        option.textContent = x.label;
        option.title = x.label;
        option.addEventListener("click", () => mark(k));
        option.addEventListener("dblclick", () => d.ok.click());
        return option;
      }),
    );
    const more = matching.length - LIST_MAX;
    note.textContent = matching.length === 0 ? "Nothing matches." : more > 0 ? `${more} more; type to narrow the list.` : "";
    note.hidden = !note.textContent;
    d.ok.disabled = matching.length === 0;
    mark(0);
  }

  function mark(k) {
    const options = list.children;
    if (options.length === 0) {
      list.removeAttribute("aria-activedescendant");
      return;
    }
    at = Math.max(0, Math.min(options.length - 1, k));
    for (const o of options) o.setAttribute("aria-selected", "false");
    options[at].setAttribute("aria-selected", "true");
    list.setAttribute("aria-activedescendant", options[at].id);
    options[at].scrollIntoView({ block: "nearest" });
  }

  /** Move the mark by a key, from the list or from the filter box; false for other keys. */
  function move(e) {
    const page = Math.max(1, Math.floor(list.clientHeight / 22));
    const steps = { ArrowDown: 1, ArrowUp: -1, PageDown: page, PageUp: -page };
    if (e.key in steps) mark(at + steps[e.key]);
    else if (e.key === "Home" && e.target === list) mark(0);
    else if (e.key === "End" && e.target === list) mark(list.children.length - 1);
    else return false;
    e.preventDefault();
    return true;
  }

  list.addEventListener("keydown", move);
  if (box) {
    box.addEventListener("input", render);
    box.addEventListener("keydown", move);
  }
  d.body.append(...(box ? [box] : []), list, note);
  render();
  return d.show(box || list, () => (matching[at] ? matching[at].value : undefined));
}

/**
 * Ask for a line of text, such as a URL. `check` says what is wrong with a text, or null when it
 * will do; the dialog stays until it does or is cancelled.
 *
 * @param {{ title: string, text?: string, value?: string, placeholder?: string, action?: string,
 *   check?: (text: string) => string | null }} options `action` labels the button that answers,
 *   "OK" by default.
 * @returns {Promise<string | null>} The text, or null when the dialog is cancelled.
 */
export function ask({ title, text, value = "", placeholder = "", action = "OK", check = () => null }) {
  const d = frame(title, text, action);
  const box = textBox("text", placeholder);
  box.value = value;
  box.setAttribute("aria-labelledby", d.id);
  const error = document.createElement("p");
  error.className = "dialog-error";
  error.hidden = true;
  box.addEventListener("input", () => {
    error.hidden = true;
  });
  d.body.append(box, error);
  return d.show(box, () => {
    const wrong = check(box.value);
    if (wrong === null) return box.value;
    error.textContent = wrong;
    error.hidden = false;
    box.focus();
    return undefined;
  });
}

/**
 * Ask for several values at once, such as settings, in the fields of a form that the caller builds
 * and reads. `answer` reads them when the default button is pressed: it gives the answer, or
 * undefined to keep the dialog, as when a value will not do and it says so; it may be async, as when
 * something outside the page says whether a value will do, and the button waits for it. Enter in a
 * field presses the default button.
 *
 * @param {{ title: string, text?: string, parts: Node[], answer: () => any, focus?: HTMLElement,
 *   action?: string, wide?: boolean }} options `parts` are shown in order; `focus` has the focus at
 *   first, the default button when not given, which `action` labels, "OK" by default; `wide` makes
 *   the dialog wider, for a text to read, and lets its body scroll.
 * @returns {Promise<any>} What `answer` gave, or null when the dialog is cancelled.
 */
export function form({ title, text, parts, answer, focus, action = "OK", wide = false }) {
  const d = frame(title, text, action);
  if (wide) d.body.parentElement.classList.add("wide");
  const fields = document.createElement("div");
  fields.className = "dialog-form";
  fields.append(...parts);
  d.body.append(fields);
  return d.show(focus || d.ok, answer);
}

/**
 * Show a message until it is closed.
 *
 * @param {{ title: string, paragraphs: (string | Node)[] }} options Each paragraph a text, or a node
 *   to show as it is.
 * @returns {Promise<void>}
 */
export function inform({ title, paragraphs }) {
  const d = frame(title, "", "Close", false);
  addParagraphs(d.body, paragraphs);
  return d.show(d.ok, () => null).then(() => {});
}

/**
 * Show a message with Cancel and a button that acts, labelled `action`, such as one that opens a
 * page.
 *
 * @param {{ title: string, paragraphs: (string | Node)[], action: string }} options Each paragraph
 *   a text, or a node to show as it is.
 * @returns {Promise<boolean>} Whether the button that acts was pressed.
 */
export function offer({ title, paragraphs, action }) {
  const d = frame(title, "", action);
  addParagraphs(d.body, paragraphs);
  return d.show(d.ok, () => true).then((acted) => acted === true);
}

/**
 * Show a message with Cancel, a button that acts, labelled `action`, which Enter presses, and
 * others between them, such as "Don't Save".
 *
 * @param {{ title: string, paragraphs: (string | Node)[], action: string, others?: string[] }} options
 *   Each paragraph a text, or a node to show as it is; `others` are the labels of the other buttons.
 * @returns {Promise<string | null>} The label of the button pressed, or null when the dialog is
 *   cancelled by Escape or Cancel.
 */
export function decide({ title, paragraphs, action, others = [], cancel = "Cancel" }) {
  const d = frame(title, "", action, cancel, others);
  addParagraphs(d.body, paragraphs);
  return d.show(d.ok, () => action);
}

/** Add paragraphs to a dialog's body: each a text, or a node to show as it is. */
function addParagraphs(body, paragraphs) {
  for (const p of paragraphs) {
    if (typeof p !== "string") {
      body.append(p);
      continue;
    }
    const el = document.createElement("p");
    el.textContent = p;
    body.append(el);
  }
}

/** A text box of a dialog. */
function textBox(type, placeholder) {
  const box = document.createElement("input");
  box.type = type;
  box.placeholder = placeholder;
  box.spellcheck = false;
  box.autocomplete = "off";
  if (type === "search") box.setAttribute("aria-label", placeholder);
  return box;
}

/**
 * A modal dialog, not yet shown: its title, its text, the body that takes its parts, and its
 * buttons: Cancel, if `cancel`, the `others` between it and the default one, each labelled and
 * answering with its label, and the default one labelled `action`, which Enter presses.
 * `show(focus, answer)` shows it with `focus` focused, and a text box's text selected. The default
 * button closes it with what `answer()` gives, awaited, unless that is undefined; the promise it
 * returns resolves with that, or with null when the dialog is cancelled by Escape or Cancel.
 */
function frame(title, text, action, cancel = true, others = []) {
  const id = `dialog-${++made}`;
  const el = document.createElement("dialog");
  el.className = "dialog";
  el.setAttribute("aria-labelledby", id);
  const head = document.createElement("h2");
  head.className = "dialog-title";
  head.id = id;
  head.textContent = title;
  const body = document.createElement("div");
  body.className = "dialog-body";
  if (text) {
    const p = document.createElement("p");
    p.textContent = text;
    body.append(p);
  }
  const buttons = document.createElement("div");
  buttons.className = "dialog-buttons";
  const ok = document.createElement("button");
  ok.type = "button";
  ok.className = "default";
  ok.textContent = action;
  if (cancel) {
    const no = document.createElement("button");
    no.type = "button";
    no.textContent = typeof cancel === "string" ? cancel : "Cancel";
    no.addEventListener("click", () => el.close());
    buttons.append(no);
  }
  let result = null;
  for (const label of others) {
    const other = document.createElement("button");
    other.type = "button";
    other.textContent = label;
    other.addEventListener("click", () => {
      result = label;
      el.close();
    });
    buttons.append(other);
  }
  buttons.append(ok);
  el.append(head, body, buttons);

  function show(focus, answer) {
    let answering = false;
    ok.addEventListener("click", async () => {
      if (answering) return;
      answering = true;
      ok.disabled = true;
      try {
        const a = await answer();
        if (a === undefined) return;
        result = a;
        el.close();
      } finally {
        answering = false;
        ok.disabled = false;
      }
    });
    // the page's keys wait; those the browser has too, F5, Ctrl+R and Ctrl+O, do nothing; Enter
    // presses the default button, or the button or the link that has the focus
    el.addEventListener("keydown", (e) => {
      e.stopPropagation();
      if (e.key === "F5" || ((e.ctrlKey || e.metaKey) && /^[ro]$/i.test(e.key))) e.preventDefault();
      const own = e.target instanceof HTMLButtonElement || e.target instanceof HTMLAnchorElement;
      if (e.key === "Enter" && !e.isComposing && !own) {
        e.preventDefault();
        if (!ok.disabled) ok.click();
      }
    });
    const before = document.activeElement;
    document.body.append(el);
    shown += 1;
    el.showModal();
    focus.focus();
    if (focus instanceof HTMLInputElement) focus.select();
    return new Promise((resolve) => {
      el.addEventListener("close", () => {
        shown -= 1;
        el.remove();
        if (before instanceof HTMLElement && before.isConnected) before.focus();
        resolve(result);
      });
    });
  }

  return { id, body, ok, show };
}
