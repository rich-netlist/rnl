// Pure geometry and hover hit-testing for the Solve skeleton-line overlays (skeleton-lines.js).
// Computed directly from the level's hits and objects so any placed drawing can show them, with no
// solver report and no package or definition names hardcoded.

import { attachments } from "./group-tree.js";

const PAD = 24; // SVG pixels the skeleton lines reach beyond the drawing

function levelExtent(level) {
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (const h of level?.hits ?? []) {
    if (h.kind === "flyline") continue;
    for (const [x, y] of h.points ?? []) {
      if (x < x0) x0 = x;
      if (y < y0) y0 = y;
      if (x > x1) x1 = x;
      if (y > y1) y1 = y;
    }
  }
  return x0 <= x1 ? [x0 - PAD, y0 - PAD, x1 + PAD, y1 + PAD] : null;
}

/** Clip the infinite line through `(px, py)` along unit `(ux, uy)` to `box = [x0, y0, x1, y1]`. */
function clipInfiniteLine(px, py, ux, uy, [x0, y0, x1, y1]) {
  let tMin = -Infinity;
  let tMax = Infinity;
  const clipAxis = (p, u, lo, hi) => {
    if (Math.abs(u) <= 1e-9) return p >= lo - 1e-6 && p <= hi + 1e-6;
    const t0 = (lo - p) / u;
    const t1 = (hi - p) / u;
    tMin = Math.max(tMin, Math.min(t0, t1));
    tMax = Math.min(tMax, Math.max(t0, t1));
    return tMax >= tMin - 1e-6;
  };
  if (!clipAxis(px, ux, x0, x1) || !clipAxis(py, uy, y0, y1) || tMax - tMin <= 1e-6) return null;
  return [
    [px + tMin * ux, py + tMin * uy],
    [px + tMax * ux, py + tMax * uy],
  ];
}

function objectLabel(level, doc, k, usageByObj, depth = 0) {
  const u = usageByObj.get(k);
  if (u) {
    if (u.id?.startsWith("%")) return u.id;
    const roleObj = depth === 0 ? u.roles?.find((r) => r.objects?.length === 1)?.objects[0] : null;
    const target = roleObj != null ? ` (${objectLabel(level, doc, roleObj, usageByObj, 1)})` : "";
    return `${u.definition || u.id || `#${k}`}${target}`;
  }
  const o = doc?.objects?.[k];
  if (!o) return `#${k}`;
  if (o.kind === "instance") return o.name.startsWith("#") ? o.name : `#${o.name}`;
  if (o.kind === "net") return o.name.startsWith("&") ? o.name : `&${o.name}`;
  return o.name;
}

/**
 * Compute deduplicated skeleton lines of `level` in SVG pixels:
 * - component coordinate crosses (horizontal + vertical through each non-mark box's `origin`);
 * - pin directional lines (along each pin's direction across the drawing);
 * - wire segment lines (each segment's straight line extended across the drawing).
 */
export function skeletonLines(level, doc = null, { symbols = true, wires = true } = {}) {
  const extent = levelExtent(level);
  if (!extent || (!symbols && !wires)) return { extent, lines: [] };

  const { junction } = attachments(level, doc);
  const usageByObj = new Map((level?.usages ?? []).filter((u) => u.object != null).map((u) => [u.object, u]));
  const byKey = new Map();

  const addLine = (px, py, dx, dy, source, owner, countSegments = false) => {
    const len = Math.hypot(dx, dy);
    if (len <= 1e-6) return;
    let ux = dx / len;
    let uy = dy / len;
    if (ux < -1e-9 || (Math.abs(ux) <= 1e-9 && uy < 0)) {
      ux = -ux;
      uy = -uy;
    }
    const seg = clipInfiniteLine(px, py, ux, uy, extent);
    if (!seg) return;
    const nx = -uy;
    const ny = ux;
    const c = nx * px + ny * py;
    const key = `${Math.round(nx * 10000)}:${Math.round(ny * 10000)}:${Math.round(c * 10)}`;
    let entry = byKey.get(key);
    if (!entry) {
      entry = { a: seg[0], b: seg[1], sources: new Set(), owners: [], ownerSet: new Set(), segCounts: new Map() };
      byKey.set(key, entry);
    }
    entry.sources.add(source);
    if (owner) {
      if (countSegments) entry.segCounts.set(owner, (entry.segCounts.get(owner) ?? 0) + 1);
      if (!entry.ownerSet.has(owner)) {
        entry.ownerSet.add(owner);
        entry.owners.push(owner);
      }
    }
  };

  if (symbols) {
    const seenOrigins = new Set();
    const boxes = [];
    for (const h of level?.hits ?? []) {
      if (h.kind !== "box" || !h.objects?.length) continue;
      const k = h.objects[0];
      if (junction(k)) continue;
      if (h.points?.length >= 2) {
        const xs = h.points.map((p) => p[0]);
        const ys = h.points.map((p) => p[1]);
        boxes.push([Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys), k]);
      }
      if (Array.isArray(h.origin) && h.origin.length >= 2) {
        const [ox, oy] = h.origin;
        const ok = `${k}:${Math.round(ox * 10)}:${Math.round(oy * 10)}`;
        if (!seenOrigins.has(ok)) {
          seenOrigins.add(ok);
          const name = objectLabel(level, doc, k, usageByObj);
          addLine(ox, oy, 0, 1, "symbols", `x of ${name}`);
          addLine(ox, oy, 1, 0, "symbols", `y of ${name}`);
        }
      }
    }

    const scale = level?.scale ?? 1;
    const pinObjects = (doc?.objects ?? []).filter(
      (o) => o?.kind === "pin" && (level?.host == null || o.level === level.host) && Array.isArray(o.pin?.at) && o.pin.dir != null,
    );
    if (pinObjects.length > 0) {
      for (const o of pinObjects) {
        if (o.pin.owner != null && junction(o.pin.owner)) continue;
        const px = o.pin.at[0] * scale;
        const py = -o.pin.at[1] * scale;
        const rad = (o.pin.dir * Math.PI) / 180;
        const label = o.pin.reference && !o.pin.reference.startsWith("@")
          ? `pin ${o.pin.reference}`
          : o.pin.owner != null
            ? `pin of ${objectLabel(level, doc, o.pin.owner, usageByObj)}`
            : `pin ${o.name}`;
        addLine(px, py, Math.cos(rad), -Math.sin(rad), "symbols", label);
      }
    } else {
      for (const h of level?.hits ?? []) {
        if (h.kind !== "pin" || !h.points?.length) continue;
        const [px, py] = h.points[0];
        let best = null;
        let bestDist = 2.5;
        for (const box of boxes) {
          const dx = px < box[0] ? box[0] - px : px > box[2] ? px - box[2] : 0;
          const dy = py < box[1] ? box[1] - py : py > box[3] ? py - box[3] : 0;
          const d = Math.hypot(dx, dy);
          if (d <= bestDist) {
            bestDist = d;
            best = box;
          }
        }
        if (!best) continue;
        const edgeX = Math.min(Math.abs(px - best[0]), Math.abs(px - best[2]));
        const edgeY = Math.min(Math.abs(py - best[1]), Math.abs(py - best[3]));
        const ownerName = objectLabel(level, doc, best[4], usageByObj);
        const netSuffix = h.objects?.length ? ` (${objectLabel(level, doc, h.objects[0], usageByObj)})` : "";
        if (edgeX <= edgeY) addLine(px, py, 1, 0, "symbols", `pin of ${ownerName}${netSuffix}`);
        else addLine(px, py, 0, 1, "symbols", `pin of ${ownerName}${netSuffix}`);
      }
    }
  }

  if (wires) {
    for (const h of level?.hits ?? []) {
      if (h.kind !== "wire" || !Array.isArray(h.points) || h.points.length < 2) continue;
      const label = h.objects?.length ? `wire ${objectLabel(level, doc, h.objects[0], usageByObj)}` : "wire";
      for (let i = 1; i < h.points.length; i++) {
        const [ax, ay] = h.points[i - 1];
        const [bx, by] = h.points[i];
        addLine(ax, ay, bx - ax, by - ay, "wires", label, true);
      }
    }
  }

  const lines = [...byKey.values()].map(({ a, b, sources, owners, segCounts }) => ({
    a,
    b,
    kind: sources.size > 1 ? "shared" : sources.has("symbols") ? "symbol" : "wire",
    owners: owners.map((o) => {
      const n = segCounts.get(o) ?? 0;
      return n > 1 ? `${o} (${n} segments)` : o;
    }),
  }));
  return { extent, lines };
}

/** Distance from `(ux, uy)` to segment `[a, b]`. */
function segmentDistance(ux, uy, [x1, y1], [x2, y2]) {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const len2 = dx * dx + dy * dy;
  if (len2 <= 1e-12) return Math.hypot(ux - x1, uy - y1);
  const t = Math.max(0, Math.min(1, ((ux - x1) * dx + (uy - y1) * dy) / len2));
  return Math.hypot(ux - (x1 + t * dx), uy - (y1 + t * dy));
}

/** Describe the skeleton lines within `maxDist` SVG pixels of `(ux, uy)`, or `null` when none. */
export function skeletonHit(lines, ux, uy, maxDist) {
  if (!lines?.length) return null;
  const hits = [];
  for (const line of lines) {
    const d = segmentDistance(ux, uy, line.a, line.b);
    if (d <= maxDist) hits.push([d, line]);
  }
  if (!hits.length) return null;
  hits.sort((a, b) => (Math.abs(a[0] - b[0]) < 1e-9 ? 0 : a[0] - b[0]));
  const seen = new Set();
  const labels = [];
  for (const [, line] of hits) {
    for (const owner of line.owners) {
      if (!seen.has(owner)) {
        seen.add(owner);
        labels.push(owner);
      }
    }
  }
  if (!labels.length) return null;
  return labels.length <= 10
    ? labels.join("\n")
    : `${labels.slice(0, 10).join("\n")}\n… (+${labels.length - 10})`;
}
