// The Groups view of the Solve pane (solve.js, constraints.js): the groups of the level shown as the tree the solver takes
// them in, which the core gives with the Document (`Usage.group_parent`; group-tree.js). One line for each group: its colour,
// its name and its members; a click lights it in the drawing, as a click on its row in the Usage view does (usages.js), and
// the check shows or hides its frame, as there. The groups the solver does not take in, those left out because they cross
// another, those with the same members as another and those turned off, follow the tree, each with its reason.

import { append, m, setAttr, setText } from "./i18n.js";
import { current, doc, selected, selectedAsUsage } from "./state.js";
import { select } from "./selection.js";
import { $, span } from "./util.js";
import { groupTree } from "./group-tree.js";
import { groupLook, setGroupVisible } from "./usages.js";
import { stateName } from "./usage-table.js";

const groupsOf = (level) => (level?.usages ?? []).filter((u) => u.kind === "group");

/** Whether the level shown has groups: when it has none, the Solve pane has no Groups view. */
export const hasGroups = () => groupsOf(doc?.levels[current]).length > 0;

const membersText = (u) => (u.roles ?? []).flatMap((r) => r.members ?? []).join(" ");

function row(u, { depth = 0, state = "in-effect", note = null } = {}) {
  const el = document.createElement("div");
  el.className = "usage-row group-row";
  el.dataset.object = String(u.object);
  el.classList.toggle("selected", selectedAsUsage && u.object === selected);
  el.classList.toggle("greyed", state !== "in-effect");
  el.style.paddingLeft = `${26 + 14 * depth}px`;
  const look = groupLook(u);
  const button = document.createElement("button");
  button.type = "button";
  button.className = "usage-name";
  button.setAttribute("aria-pressed", String(selectedAsUsage && u.object === selected));
  const dot = span("usage-color-dot", "");
  dot.style.background = look.color;
  append(button, dot, u.id, span("group-members", membersText(u)));
  button.addEventListener("click", () => select(u.object, "usage"));
  append(el, button);
  if (state !== "in-effect") append(el, span("usage-state", stateName(state)));
  const check = document.createElement("input");
  check.type = "checkbox";
  check.checked = look.visible;
  setAttr(check, "aria-label", m`Show ${u.id}`);
  check.addEventListener("change", () => { setGroupVisible(u, check.checked); render(); });
  append(el, check);
  if (note) append(el, span("group-note", note));
  return el;
}

/** The tree below `node`, a group after the group above it, each one step in. */
function branch(node, into) {
  for (const child of node.children) {
    append(into, row(child.group, { depth: child.depth - 1 }));
    branch(child, into);
  }
}

/** Why a group is not in the tree: what the core says, or, for one turned off by its own PRIORITY, that. */
const why = (u) => u.reason ?? m("turned off by PRIORITY");

/** Show the groups of the level shown in the Groups view's place. */
export function render() {
  const host = $("groups-list");
  if ($("solve-groups").hidden) return;
  host.replaceChildren();
  const tree = groupTree(doc?.levels[current]);
  branch(tree.root, host);
  const rest = [...tree.left, ...tree.same].map((x) => x.group).concat(tree.off);
  // in the order of the Document's usages, which is the order the solver takes them in
  const order = groupsOf(doc?.levels[current]);
  rest.sort((a, b) => order.indexOf(a) - order.indexOf(b));
  if (rest.length === 0) return;
  const head = document.createElement("h4");
  head.className = "groups-rest";
  setText(head, m`Not in the tree (${rest.length})`);
  append(host, head);
  for (const u of rest) append(host, row(u, { state: u.state ?? "in-effect", note: why(u) }));
}

// the rows follow the selection, the level shown and the language
window.addEventListener("viewer-selection", render);
window.addEventListener("viewer-level", render);
window.addEventListener("language-changed", render);
