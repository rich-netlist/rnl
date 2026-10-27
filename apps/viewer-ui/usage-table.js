// The usages of one object, from the Document's reverse index (`Object.usages`, the core's MemberOf): in which
// usages the instance, net or terminal is a member, as which role. The table is shown in the lower half of the
// Usage panel when an object that is no usage is selected; a role note for the pointer's box is made from the
// same entries. The usages the object's usages are members of in turn are listed under them, indented.

import { append, m, setAttr, setText } from "./i18n.js";
import { span } from "./util.js";

const MAX_DEPTH = 8; // a chain of usages in usages is a few deep; this is for the one that loops

/** The role as the table and the note say it: `in_p`, a set role's member as `{tail}`, and its terminal. */
export function roleName(r) {
  const name = r.set ? `{${r.role}}` : r.role;
  return r.terminal ? m`${name} (terminal ${r.terminal})` : name;
}

/** The roles of one entry, joined; a string, for where a message is not wanted. */
function roleList(entry) {
  return entry.roles.map((r) => String(roleName(r))).join(", ");
}

const STATES = {
  failed: () => m("failed"),
  overridden: () => m("overridden"),
  disabled: () => m("disabled"),
};

/** Where an entry's usage comes from, when it is not a statement written in the document. */
function origin(entry) {
  if (entry.source === "solver") return m("written back by a solver");
  if (entry.source === "rule") return entry.rule?.package ? m`rule of ${entry.rule.package}` : m("rule of this document");
  if (entry.source === "body") return m`made in ${entry.made_in}`;
  return null;
}

/** The index of the Document's file that an entry's statement is in, or -1. */
function fileIndex(doc, entry) {
  return entry.file ? doc.files.findIndex((f) => f.path === entry.file) : -1;
}

/**
 * The note of the box beside the pointer for an object: its first usage in effect, as `in_p of
 * $DIFFERENTIAL_PAIR %dp1`, and how many more there are; null when it is in none.
 */
export function roleNote(doc, o) {
  const entries = (doc?.objects[o]?.usages ?? []).filter((e) => e.state === "in-effect");
  if (entries.length === 0) return null;
  const [first] = entries;
  const note = String(m`${roleList(first)} of ${first.definition} ${first.id}`);
  return entries.length > 1 ? `${note} · +${entries.length - 1}` : note;
}

/**
 * The table of an object's usages: a heading, and a row for each usage, with its role, its id, its
 * definition, where its statement is and, when it is not in effect, why. `on` has the callbacks:
 * `select(object)` for a click on a usage, `hover(object | null)` when the pointer or the focus is on
 * a row, and `reveal(file, line)` for the link to the statement.
 */
export function objectUsages(doc, o, on) {
  const object = doc.objects[o];
  const out = document.createElement("div");
  out.className = "usage-membership";
  const title = document.createElement("h2");
  setText(title, m`Usages of ${object.name}`);
  append(out, title);
  const list = document.createElement("ul");
  list.className = "usage-membership-list";
  const seen = new Set([o]);
  const add = (entries, depth) => {
    for (const entry of entries) {
      append(list, row(doc, entry, depth, on));
      // the usage's own memberships go under it, once each: one that holds itself is cut
      if (entry.usage !== null && depth < MAX_DEPTH && !seen.has(entry.usage)) {
        seen.add(entry.usage);
        add(doc.objects[entry.usage]?.usages ?? [], depth + 1);
        seen.delete(entry.usage);
      }
    }
  };
  add(object.usages ?? [], 0);
  if (!list.children.length) append(out, span("placeholder", m("It is in no usage.")));
  else append(out, list);
  return out;
}

function row(doc, entry, depth, on) {
  const li = document.createElement("li");
  li.className = "usage-membership-row";
  li.dataset.state = entry.state;
  if (entry.usage !== null) li.dataset.usage = String(entry.usage);
  li.style.setProperty("--depth", String(depth));
  li.classList.toggle("nested", depth > 0);
  li.classList.toggle("greyed", entry.state !== "in-effect");
  const head = document.createElement("div");
  head.className = "usage-membership-head";
  append(head, span("membership-role", roleList(entry)));
  // a failed application made no usage: its id is text, not something to select
  let id;
  if (entry.usage !== null) {
    id = document.createElement("button");
    id.type = "button";
    id.addEventListener("click", () => on.select(entry.usage));
    setAttr(id, "title", m("Select this usage"));
  } else id = document.createElement("span");
  id.className = "membership-id";
  id.textContent = entry.id;
  append(head, id, span("membership-definition", entry.definition));
  append(li, head);
  const detail = document.createElement("div");
  detail.className = "usage-membership-detail";
  const file = fileIndex(doc, entry);
  if (entry.file && entry.line) {
    const name = entry.file.split(/[\\/]/).pop();
    if (file >= 0) {
      const link = document.createElement("button");
      link.type = "button";
      link.className = "membership-where";
      link.textContent = `${name}:${entry.line}`;
      setAttr(link, "title", entry.file);
      link.addEventListener("click", () => on.reveal(file, entry.line));
      append(detail, link);
    } else append(detail, span("membership-where", `${name}:${entry.line}`));
  }
  const from = origin(entry);
  if (from) append(detail, span("membership-origin", from));
  if (entry.state !== "in-effect") append(detail, span("membership-state", STATES[entry.state]?.() ?? entry.state));
  if (entry.reason) append(detail, span("membership-reason", entry.reason));
  if (detail.childNodes.length) append(li, detail);
  if (entry.usage !== null) {
    li.addEventListener("pointerenter", () => on.hover(entry.usage));
    li.addEventListener("pointerleave", () => on.hover(null));
    li.addEventListener("focusin", () => on.hover(entry.usage));
    li.addEventListener("focusout", () => on.hover(null));
  }
  return li;
}
