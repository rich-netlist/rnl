// The tree of a level's groups, as the core gives it (`Usage.group_parent`, `same_as`, `crosses` and the states "same",
// "left-out" and "disabled" of a group's usage: rnl_runtime::group_tree), and structural attachment queries and
// layout-table geometry for the Solve overlays. Attachment queries follow language relations (`pin_of`, `parent`,
// `solver_config` and point-mark structure) without naming any package or definition.

const PAD = 14; // SVG pixels from the outermost boxes to the table's frame

/** The objects of a group: the objects of all its roles. */
const membersOf = (u) => new Set((u.roles ?? []).flatMap((r) => r.objects ?? []));
const union = (a, b) => [Math.min(a[0], b[0]), Math.min(a[1], b[1]), Math.max(a[2], b[2]), Math.max(a[3], b[3])];

/**
 * The tree of the level's groups: `nodes` (the level first, as `group: null`, then each group in the tree, in the order of
 * the Document's usages, which is the solver's), each with its `parent` node, `children` nodes and `own` objects (the
 * members no child holds); `left` the groups left out, each with `crosses`, the group it crosses, when it crosses one;
 * `off` those turned off; `same` the groups that are another one, each with `as`. A level that is not of 0.4 has no tree:
 * its groups are all at the top.
 */
export function groupTree(level) {
  const all = (level?.usages ?? []).filter((u) => u.kind === "group");
  const byObject = new Map(all.map((u) => [u.object, u]));
  const stateOf = (u) => u.state ?? "in-effect";
  const root = { group: null, members: null, parent: null, children: [], own: [] };
  const nodes = [root];
  const nodeOfGroup = new Map();
  for (const u of all) {
    if (stateOf(u) !== "in-effect") continue;
    const node = { group: u, members: membersOf(u), parent: null, children: [], own: [] };
    nodes.push(node);
    nodeOfGroup.set(u.object, node);
  }
  for (const n of nodes.slice(1)) {
    n.parent = nodeOfGroup.get(n.group.group_parent) ?? root;
    n.parent.children.push(n);
  }
  const deepen = (n, depth) => { n.depth = depth; for (const c of n.children) deepen(c, depth + 1); };
  deepen(root, 0);
  for (const n of nodes.slice(1)) {
    n.own = [...n.members].filter((k) => !n.children.some((c) => c.members.has(k)));
  }
  const left = all.filter((u) => stateOf(u) === "left-out")
    .map((u) => ({ group: u, crosses: byObject.get(u.crosses) ?? null, reason: u.reason ?? null }));
  const sameAs = all.filter((u) => stateOf(u) === "same")
    .map((u) => ({ group: u, as: byObject.get(u.same_as) ?? null, reason: u.reason ?? null }));
  return { nodes, root, left, off: all.filter((u) => stateOf(u) === "disabled"), same: sameAs };
}

/** The innermost node of `tree` that holds object `k`: the level when no group does. */
export function nodeOf(tree, k) {
  let best = tree.root;
  for (const n of tree.nodes) if (n.members?.has(k) && (best.members === null || n.members.size < best.members.size)) best = n;
  return best;
}

/**
 * Language-driven attachment and point-mark classification for `level`:
 * - `hostOf(k)` maps a usage attached to a pin (`pin_of`), or nested inside one (`parent`), to the owner of that pin;
 * - `junction(k)` identifies point-mark usages (configured via `solver_config` or drawn at their own point with no
 *   role attachments, no standing net and no off-origin pins).
 */
export function attachments(level, doc = null) {
  const host = new Map();
  const junctions = new Set();
  const usages = level?.usages ?? [];
  const byObject = new Map(usages.filter((u) => u.object != null).map((u) => [u.object, u]));

  for (const u of usages) {
    if (u.kind === "group" || u.object == null) continue;
    let s = u;
    const seen = new Set();
    while (s && s.pin_of == null && s.parent != null && !seen.has(s.object)) {
      seen.add(s.object);
      s = byObject.get(s.parent);
    }
    if (s?.pin_of != null) host.set(u.object, s.pin_of);
  }

  const configured = new Set();
  for (const entry of level?.constraints?.solver_config ?? []) {
    if (entry?.key === "junction" && typeof entry.value === "string" && entry.value.length > 0) {
      configured.add(entry.value);
      if (!entry.value.startsWith("$")) configured.add(`$${entry.value}`);
    }
  }

  const boxHits = new Map();
  for (const h of level?.hits ?? []) {
    if (h.kind !== "box" || !h.objects?.length) continue;
    const k = h.objects[0];
    if (!boxHits.has(k)) boxHits.set(k, []);
    boxHits.get(k).push(h);
  }

  const nonMarkPins = new Set();
  for (const o of doc?.objects ?? []) {
    if (o?.kind !== "pin" || o.pin?.owner == null) continue;
    const [lx = 0, ly = 0] = o.pin.local_at ?? [0, 0];
    if (Math.abs(lx) >= 1e-6 || Math.abs(ly) >= 1e-6 || (o.pin.nets ?? []).length > 0) {
      nonMarkPins.add(o.pin.owner);
    }
  }

  for (const u of usages) {
    if (u.kind === "group" || u.object == null) continue;
    if (configured.has(u.definition)) {
      junctions.add(u.object);
      continue;
    }
    if (!u.has_symbol || host.has(u.object) || nonMarkPins.has(u.object)) continue;
    if ((u.roles ?? []).some((r) => (r.objects ?? []).length > 0)) continue;
    const hits = boxHits.get(u.object);
    if (hits?.length && hits.every((h) => h.objects.length === 1 && h.objects[0] === u.object)) {
      junctions.add(u.object);
    }
  }

  const hostOf = (k) => {
    let cur = k;
    const seen = new Set();
    while (host.has(cur) && !seen.has(cur)) {
      seen.add(cur);
      cur = host.get(cur);
    }
    return cur;
  };
  return { hostOf, junction: (k) => junctions.has(k) };
}

/** Partition `items` into runs along `axis` (0 for x, 1 for y) separated by straight cuts; touching boxes separate. */
function runs(items, axis) {
  const sorted = [...items].sort((a, b) => a.box[axis] - b.box[axis]);
  const out = [];
  let run = [sorted[0]];
  let reach = sorted[0].box[axis + 2];
  for (const it of sorted.slice(1)) {
    if (it.box[axis] >= reach) { out.push(run); run = [it]; } else run.push(it);
    reach = Math.max(reach, it.box[axis + 2]);
  }
  out.push(run);
  return out;
}

const extent = (run, axis) => [Math.min(...run.map((it) => it.box[axis])), Math.max(...run.map((it) => it.box[axis + 2]))];

/**
 * Compute the layout table of `level` in SVG pixels: entries `{ rect, kind, key }` with `kind`
 * `"frame"`, `"cell"`, `"group"`, `"stuck"` or `"loose"`. Empty when the level draws no boxes.
 */
export function layoutTable(level, document = null) {
  const { hostOf, junction } = attachments(level, document);
  const boxes = new Map();
  for (const h of level?.hits ?? []) {
    if (h.kind !== "box" || !h.points?.length || !h.objects?.length) continue;
    const k = h.objects[0];
    if (junction(k)) continue;
    const xs = h.points.map((p) => p[0]);
    const ys = h.points.map((p) => p[1]);
    const b = [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
    const o = hostOf(k);
    boxes.set(o, boxes.has(o) ? union(boxes.get(o), b) : b);
  }
  if (!boxes.size) return [];
  const tree = groupTree(level);
  const itemsOf = new Map(tree.nodes.map((n) => [n, []]));
  for (const [k, b] of boxes) itemsOf.get(nodeOf(tree, k)).push({ box: b, node: null, key: k });
  const boxOf = new Map();
  const nodeBox = (n) => {
    if (boxOf.has(n)) return boxOf.get(n);
    let b = null;
    for (const it of itemsOf.get(n)) b = b ? union(b, it.box) : it.box;
    for (const c of n.children) { const cb = nodeBox(c); if (cb) b = b ? union(b, cb) : cb; }
    boxOf.set(n, b);
    return b;
  };
  for (const n of tree.nodes) {
    for (const c of n.children) {
      const b = nodeBox(c);
      if (b) itemsOf.get(n).push({ box: b, node: c, key: c.group.object });
    }
  }
  const all = nodeBox(tree.root);
  if (!all) return [];
  const frame = [all[0] - PAD, all[1] - PAD, all[2] + PAD, all[3] + PAD];
  const out = [{ rect: frame, kind: "frame" }];
  const lay = (node, region) => cut(itemsOf.get(node), region, null);
  const cut = (items, region, last) => {
    if (items.length === 0) return;
    if (items.length === 1) {
      const it = items[0];
      out.push({ rect: region, kind: it.node ? "group" : "cell", key: it.key });
      if (it.node) lay(it.node, region);
      return;
    }
    for (const axis of last === 0 ? [1, 0] : [0, 1]) {
      const parts = runs(items, axis);
      if (parts.length < 2) continue;
      let start = region[axis];
      parts.forEach((part, i) => {
        const end = i === parts.length - 1 ? region[axis + 2] : (extent(part, axis)[1] + extent(parts[i + 1], axis)[0]) / 2;
        const r = [...region];
        r[axis] = start;
        r[axis + 2] = end;
        cut(part, r, axis);
        start = end;
      });
      return;
    }
    out.push({ rect: region, kind: "stuck" });
    for (const it of items) {
      out.push({ rect: it.box, kind: "loose", key: it.key });
      if (it.node) lay(it.node, it.box);
    }
  };
  lay(tree.root, frame);
  return out;
}
