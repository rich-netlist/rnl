// The usages of one object, from the Document's reverse index (`Object.usages`, the core's MemberOf): in which
// usages the instance, net or terminal is a member, as which role. The table is shown in the lower half of the
// Usage panel when an object that is no usage is selected; a role note for the pointer's box is made from the
// same entries. Each row reads as a sentence, `in_p of usage %dp1 (definition $DIFFERENTIAL_PAIR)`, and the
// usages the object's usages are members of in turn are listed under them, indented to a few levels, each
// naming whose member it is, `%outer is in turn first of usage %top (definition $TWO)`.

import { append, m, setAttr, setText } from "./i18n.js";
import { span } from "./util.js";

const MAX_DEPTH = 16; // a chain of usages in usages is a few deep; this is for the one that loops
const INDENT_LEVELS = 3; // the rows are indented this many levels; deeper ones go no further, their sentence says whose they are

/** A private-use character that stands for the node of a sentence's slot until the sentence is translated. */
const slot = (i) => String.fromCharCode(0xe000 + i);

/** A sentence of the catalog with its slots filled by nodes: the template is translated whole, as a language orders it. */
function sentence(message, ...nodes) {
  const out = document.createElement("span");
  out.className = "membership-sentence";
  for (const part of String(message).split(/([-])/)) {
    if (!part) continue;
    const i = part.charCodeAt(0) - 0xe000;
    if (part.length === 1 && i >= 0 && i < nodes.length) out.append(nodes[i]);
    else {
      // a closing bracket stays with the name before it: the name's last character and the bracket are one
      // piece, so a name that is longer than a line breaks before that, never leaves the bracket to begin one
      const [, closing, rest] = part.match(/^([)）]*)([^]*)$/);
      const before = out.lastChild;
      if (closing && before?.nodeType === 1 && before.textContent) {
        const letters = Array.from(before.textContent);
        const tail = document.createElement("span");
        tail.className = "membership-tail";
        tail.textContent = letters.pop() + closing;
        before.textContent = letters.join("");
        before.append(tail);
      } else if (closing) out.append(document.createTextNode(closing));
      if (rest) out.append(document.createTextNode(rest));
    }
  }
  return out;
}

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

/** A state that is not "in-effect", as the rows say it. */
export const stateName = (state) => STATES[state]?.() ?? state;

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
  const note = String(first.kind === "group" ? m`member of group ${first.id}` : m`${roleList(first)} of ${first.definition} ${first.id}`);
  return entries.length > 1 ? `${note} · +${entries.length - 1}` : note;
}

/**
 * The usages an object is in, as the table lists them: each entry of its `usages`, and under a usage its own, with the
 * depth and the id of the usage they are of; once each, so that one that holds itself is cut.
 */
function* memberships(doc, o) {
  const seen = new Set([o]);
  function* walk(entries, depth, subject) {
    for (const entry of entries) {
      yield { entry, depth, subject };
      if (entry.usage !== null && depth < MAX_DEPTH && !seen.has(entry.usage)) {
        seen.add(entry.usage);
        yield* walk(doc.objects[entry.usage]?.usages ?? [], depth + 1, entry.id);
        seen.delete(entry.usage);
      }
    }
  }
  yield* walk(doc.objects[o].usages ?? [], 0, null);
}

/** The objects of the usages and groups listed in the table of the usages of object `o`. */
export function relatedUsages(doc, o) {
  return new Set([...memberships(doc, o)].map(({ entry }) => entry.usage).filter((u) => u !== null));
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
  // a usage's own memberships go under it
  for (const { entry, depth, subject } of memberships(doc, o)) append(list, row(doc, entry, depth, subject, on));
  if (!list.children.length) append(out, span("placeholder", m("It is in no usage.")));
  else append(out, list);
  return out;
}

/**
 * The sentence of a group (11.5) the object, or the usage or group `subject` it is in turn, is in: a member of it, or,
 * with no role, within it, as a group is within the groups that hold all its members. A terminal written into a
 * group is a member through the terminal.
 */
function groupSentence(entry, subject, id) {
  const member = entry.roles.length > 0;
  const terminal = entry.roles[0]?.terminal;
  if (subject === null) {
    if (!member) return sentence(m`within group ${slot(0)}`, id);
    return terminal
      ? sentence(m`member (through terminal ${slot(0)}) of group ${slot(1)}`, span("membership-role", terminal), id)
      : sentence(m`member of group ${slot(0)}`, id);
  }
  const whose = span("membership-subject", subject);
  return member
    ? sentence(m`${slot(0)} is in turn a member of group ${slot(1)}`, whose, id)
    : sentence(m`${slot(0)} is in turn within group ${slot(1)}`, whose, id);
}

function row(doc, entry, depth, subject, on) {
  const li = document.createElement("li");
  li.className = "usage-membership-row";
  li.dataset.state = entry.state;
  li.dataset.kind = entry.kind;
  if (entry.usage !== null) li.dataset.usage = String(entry.usage);
  li.dataset.depth = String(depth);
  li.style.setProperty("--depth", String(Math.min(depth, INDENT_LEVELS)));
  li.classList.toggle("nested", depth > 0);
  li.classList.toggle("greyed", entry.state !== "in-effect");
  const main = document.createElement("div");
  main.className = "usage-membership-main";
  const head = document.createElement("div");
  head.className = "usage-membership-head";
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
  // the row is one sentence of the catalog: the role of the object in the usage, the usage, its definition
  const role = span("membership-role", roleList(entry));
  const definition = span("membership-definition", entry.definition);
  if (entry.kind === "group") append(head, groupSentence(entry, subject, id));
  else if (subject === null) append(head, sentence(m`${slot(0)} of usage ${slot(1)} (definition ${slot(2)})`, role, id, definition));
  else {
    const whose = span("membership-subject", subject);
    append(head, sentence(m`${slot(0)} is in turn ${slot(1)} of usage ${slot(2)} (definition ${slot(3)})`, whose, role, id, definition));
  }
  append(main, head);
  // a usage that is not in effect says so at the right of its row
  if (entry.state !== "in-effect") append(main, span("membership-state", STATES[entry.state]?.() ?? entry.state));
  append(li, main);
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
