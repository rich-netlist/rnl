// What the drawing shows of a constraint that is clicked in the Constraints view (constraints.js): a box around the
// objects it acts on, as a usage's frame is drawn (usages.js), with what it says at its upper left, and the order of
// the objects it lists beside each. Here are the parts that know the Document and not the DOM: the title, the members
// in the order the constraint lists them, and where each is in the drawing, in the SVG pixels of the hits and the
// frames. `usages.js` draws it. The box is a hint of the page: it is not in an exported image, and not under the pointer.

/** A number as the title writes it: the integer or the decimals the value has, not a float's tail. */
const num = (v) => String(Math.round(v * 1e6) / 1e6);

/** An orientation code or a list of allowed codes as the title writes it: `R90`, `[MY, R0]`. */
function codeText(list, single, what, pattern) {
  if (Array.isArray(list) && list.length > 0) return list.length === 1 ? list[0] : `[${list.join(", ")}]`;
  return pattern?.exec(what ?? "")?.[1] ?? single ?? "";
}

/**
 * What a fact says without the objects it names, from the fields of its kind: `ALIGN y`, `ORDER x increasing`,
 * `DISTRIBUTE x increasing`, `DISTANCE x ≥ 2 ≤ 4`, `PLACE-x = 3`, `ORIENT R90`, `ORIENT [MY, R0]`, `TABLE x`. A
 * distance with the same least and most is `= 8`.
 */
export function boxTitle(fact) {
  switch (fact.kind) {
    case "ALIGN":
    case "TABLE":
      return `${fact.kind} ${fact.axis}`;
    case "DISTRIBUTE": {
      // a DISTRIBUTE of 0.3 runs either way on its axis; one of 0.4 says which
      const sense = fact.sense ?? /^DISTRIBUTE \S+ (increasing|decreasing)\b/.exec(fact.what ?? "")?.[1];
      return ["DISTRIBUTE", fact.axis, sense && sense !== "either" && sense].filter(Boolean).join(" ");
    }
    case "ORDER":
      return `ORDER ${fact.axis} ${fact.sense}`;
    case "DISTANCE": {
      const { min, max } = fact;
      const range = min !== null && min === max ? [`= ${num(min)}`] : [min !== null && `≥ ${num(min)}`, max !== null && `≤ ${num(max)}`];
      return ["DISTANCE", fact.axis, fact.sense !== "either" && fact.sense, ...range].filter(Boolean).join(" ");
    }
    case "ORIENT":
      return `ORIENT ${codeText(fact.codes, fact.code, fact.what, /^the orientation (\[[^\]]+\]) /)}`.trim();
    case "ORIENT-RELATION":
      return `ORIENT-RELATION ${codeText(fact.relations, fact.relation, fact.what, /^ORIENT-RELATION (\[[^\]]+\]) /)}`.trim();
    case "PIN-ORIENT":
      return `PIN-ORIENT ${codeText(fact.codes, fact.code, fact.what, /^ORIENT (\[[^\]]+\]) /)}`.trim();
    case "PIN-ORIENT-RELATION":
      return `PIN-ORIENT-RELATION ${codeText(fact.relations, fact.relation, fact.what, /^ORIENT-RELATION (\[[^\]]+\]) /)}`.trim();
    default:
      // the coordinate kinds: PLACE-x, POINT-y, TEXT-dx
      return typeof fact.value === "number" ? `${fact.kind} = ${num(fact.value)}` : fact.kind;
  }
}

/** The cells of a TABLE constraint, numbered by their indices along its lists (`1`, `1.2`, ...). */
function tableMembers(cell, prefix = "") {
  if (!cell) return [];
  if (cell.point) return [{ point: cell.point, tag: prefix || null }];
  if (Array.isArray(cell.list)) {
    return cell.list.flatMap((item, k) => tableMembers(item, prefix ? `${prefix}.${k + 1}` : String(k + 1)));
  }
  return [];
}

/**
 * The members of a fact, in the order it lists them, each with its number as the box writes it and the point or
 * object it is: `{ tag, point }` for a point of the Constraint kind (`origin`, `object` with a `pin`, `centre`), and
 * `{ tag, object }` for an object of the constraints' list that is named by its index. A fact that does not list its
 * objects (a PLACE, an ORIENT, a PIN-ORIENT, a POINT, a TEXT) has members with no tag. A distance, or a relation of
 * orientations, lists two sets and numbers each: `from1 from2 to1`. A TABLE numbers its cells by their indices.
 */
export function factMembers(fact) {
  const numbered = (list, prefix, make) => list.map((member, k) => ({ ...make(member), tag: `${prefix}${k + 1}` }));
  const point = (p) => ({ point: p });
  const bare = (o) => ({ object: o });
  switch (fact.kind) {
    case "ALIGN":
    case "ORDER":
    case "DISTRIBUTE":
      return numbered(fact.members, "", point);
    case "TABLE":
      return tableMembers(fact.rows);
    case "DISTANCE":
      return [...numbered(fact.from, "from", point), ...numbered(fact.to, "to", point)];
    case "ORIENT-RELATION":
      return [...numbered(fact.from, "from", bare), ...numbered(fact.to, "to", bare)];
    case "PIN-ORIENT-RELATION":
      return [{ point: fact.from, tag: "from1" }, { point: fact.to, tag: "to1" }];
    case "PLACE-x":
    case "PLACE-y":
      return [{ point: fact.at, tag: null }];
    case "TEXT-dx":
    case "TEXT-dy":
      return [{ object: fact.text, tag: null }, { point: fact.from, tag: null }];
    case "POINT-x":
    case "POINT-y":
      return [{ object: fact.point, tag: null }];
    case "ORIENT":
      return [{ object: fact.object, tag: null }];
    case "PIN-ORIENT":
      return [{ point: fact.pin, tag: null }];
    default:
      return [];
  }
}

/**
 * Where the things of a level are in the drawing, in the SVG pixels of its hits: by the objects of the Document, by an
 * id (what a label stands for), and the point a literal `(x, y)` is written at.
 */
function placesOf(level) {
  const boxes = [];
  const labels = new Map();
  for (const h of level.hits) {
    if (h.kind === "box") boxes.push(h);
    else if (h.kind === "label" && h.id) labels.set(h.id, h);
  }
  const corners = (h) => [h.points[0][0], h.points[0][1], h.points[1][0], h.points[1][1]];
  const scale = level.scale;
  const written = new Map(); // a point's coordinates, in the scene, as the facts that give both write them
  for (const f of level.constraints.facts) {
    if (f.kind !== "POINT-x" && f.kind !== "POINT-y") continue;
    const at = written.get(f.point) ?? {};
    at[f.kind === "POINT-x" ? "x" : "y"] = f.value;
    written.set(f.point, at);
  }
  return {
    /** The boxes that draw a Document object: an instance, or a usage that has a symbol. */
    boxOf: (d) => boxes.filter((h) => h.objects.includes(d)).map(corners),
    /** The frame of a usage that is drawn as a frame only. */
    frameOf: (d) => level.usages.find((u) => u.object === d)?.frame?.bounds ?? null,
    /** The one usage of the level that has this definition and a symbol of its own; none when there are none or several. */
    symbolOf: (definition) => {
      const found = level.usages.filter((u) => u.definition === definition && u.has_symbol);
      return found.length === 1 ? found[0].object : null;
    },
    labelOf: (id) => (labels.has(id) ? corners(labels.get(id)) : null),
    /** The point `(x, y)` written for a point of the constraints' list; none when it is free or gives one coordinate. */
    pointAt: (o) => {
      const at = written.get(o);
      return at && "x" in at && "y" in at ? [at.x * scale, -at.y * scale, at.x * scale, -at.y * scale] : null;
    },
  };
}

/**
 * The rectangles (`[left, top, right, bottom]`; a point has both corners the same) a member of a fact is drawn as, in the SVG pixels of the level's hits; none when the level does not
 * draw it, which is the case of an object of another level or one the solver has yet to place. `textBox(id)` gives the
 * rectangle of a text the hits do not know (a text written in the scene), from the drawing itself; null when none.
 */
function rectsOf(member, c, places, textBox) {
  if (member.point?.centre) return member.point.centre.flatMap((p) => rectsOf({ point: p }, c, places, textBox));
  const pin = member.point && "pin" in member.point ? member.point : null;
  const index = pin ? pin.object : member.point ? member.point.origin : member.object;
  const o = c.objects[index];
  if (!o) return [];
  // the pin a fact names is where the core draws it, with a net on it or not; the object's box when it says none
  const at = pin ? o.pins?.[pin.pin] : null;
  if (at) return [[at[0], at[1], at[0], at[1]]];
  // a usage that a rule made, such as the ground of a stub, has no id in the Document: its symbol is the level's only
  // usage of its definition that has one
  let held = o.objects;
  if (o.kind === "usage" && held.length === 0) {
    const made = c.usages.find((u) => u.generated && u.object === null && u.label === o.label);
    const symbol = made ? places.symbolOf(made.definition) : null;
    if (symbol !== null) held = [symbol];
  }
  const drawn = held.flatMap((d) => {
    const boxes = places.boxOf(d);
    if (boxes.length > 0) return boxes;
    const frame = places.frameOf(d);
    return frame ? [frame] : [];
  });
  if (drawn.length > 0) return drawn;
  if (o.kind === "point") return [places.pointAt(index)].filter(Boolean);
  if (o.kind === "text") return [places.labelOf(o.id) ?? textBox(o.id) ?? textBox(o.label)].filter(Boolean);
  return [];
}

/**
 * The box of a fact on the level's drawing: its `title`, its `items` (a rectangle of the drawing for each object the
 * fact names, with its number, `tag`, when it lists its objects) and the `bounds` that hold them all; null when none of
 * its objects is on the drawing. `textBox` is for the texts that only the drawing knows.
 */
export function constraintBox(level, fact, textBox = () => null) {
  const c = level?.constraints;
  if (!c) return null;
  const places = placesOf(level);
  const items = [];
  for (const member of factMembers(fact)) {
    for (const rect of rectsOf(member, c, places, textBox)) items.push({ rect, tag: member.tag });
  }
  if (items.length === 0) return null;
  const bounds = items.reduce((b, { rect }) => [Math.min(b[0], rect[0]), Math.min(b[1], rect[1]), Math.max(b[2], rect[2]), Math.max(b[3], rect[3])], [Infinity, Infinity, -Infinity, -Infinity]);
  return { title: boxTitle(fact), items, bounds };
}

/** The objects of the constraints' list a fact acts on, by their index there. */
export function factObjects(fact) {
  const out = new Set();
  const visit = (value, bare) => {
    if (Array.isArray(value)) value.forEach((v) => visit(v, bare));
    else if (typeof value === "number") { if (bare) out.add(value); }
    else if (value && typeof value === "object") {
      if (typeof value.origin === "number") out.add(value.origin);
      else if (typeof value.object === "number") out.add(value.object);
      else if (value.centre) visit(value.centre, false);
      else if (value.point) visit(value.point, false);
      else if (value.list) visit(value.list, false);
    }
  };
  for (const key of ["members", "at", "from", "to", "pin", "rows"]) visit(fact[key], fact.kind === "ORIENT-RELATION" && key !== "at");
  for (const key of ["object", "point", "text"]) if (typeof fact[key] === "number") out.add(fact[key]);
  return out;
}

/** The ids of the texts a fact names, which stay lit with the components that draw the objects it names. */
export function factTextIds(level, fact) {
  const c = level?.constraints;
  if (!c) return [];
  const ids = [];
  const visit = (value) => {
    if (Array.isArray(value)) value.forEach(visit);
    else if (typeof value === "number") ids.push(value);
    else if (value && typeof value === "object") {
      if (typeof value.origin === "number") ids.push(value.origin);
      else if (typeof value.object === "number") ids.push(value.object);
      else if (value.centre) visit(value.centre);
    }
  };
  for (const m of factMembers(fact)) visit(m.point ?? m.object);
  return ids.map((i) => c.objects[i]).filter((o) => o?.kind === "text" && o.id).map((o) => o.id);
}
