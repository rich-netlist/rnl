// The touch gestures of a view that is panned and zoomed by changing its coordinates, shared by the
// drawing (drawing.js) and the preview of a symbol (svg-viewer.js), so that a finger does the same on
// both. Touch is `pointerType === "touch"` of Pointer Events, as a phone or a tablet gives it: one
// finger drags the view once it has moved more than TOUCH_SLOP, two zoom it around their middle as
// they spread or pinch and drag it as their middle moves, a third is let be. A touch that neither
// moved nor ever had two fingers is a tap, and two taps soon and near each other a double tap, which
// the page tells itself, as WebKit makes no double click of taps. The element takes `touch-action:
// none` in its style, so that the browser neither scrolls the page nor zooms it under the fingers.

import { clamp } from "./util.js";

const TOUCH_SLOP = 10; // a fingertip is not still: pixels a touch may move and still be a tap
const DOUBLE_TAP_MS = 350; // a second tap this soon, and near, is a double tap
const DOUBLE_TAP_PX = 30;
// a click, a double click or a mouse move this soon after a touch is the browser's of the touch,
// which the page has dealt with itself
export const TOUCH_CLICK_MS = 800;

/**
 * The gestures of `element`, whose view is {x, y, k}: the top-left corner in the view's units, and
 * pixels per unit. `view()` gives it, null while there is none, `setView(v)` takes the next; `min`
 * and `max` limit k. `onTap(e)` and `onDoubleTap(e)` take the pointer event that lifted the finger;
 * `onDown(e)` is told when a finger goes down. Give the pointer events of touch to `down`, `move`
 * and `up` (`cancelled` for pointercancel and lostpointercapture); `touchedAt()` is when a finger
 * last left, for ignoring what the browser makes of it.
 */
export function touchGestures({ element, view, setView, min, max, onTap, onDoubleTap, onDown }) {
  // the fingers by pointer id, each where it is; and the touch they make: where its first finger went
  // down, whether it moved past a tap or ever had two fingers; and the last tap
  const fingers = new Map();
  let touch = null;
  let lastTap = null;
  let touchedAt = -Infinity;

  return {
    touchedAt: () => touchedAt,

    down(e) {
      e.preventDefault();
      if (!view() || fingers.size >= 2) return;
      onDown?.(e);
      try { element.setPointerCapture(e.pointerId); } catch { /* a pointer the page did not make */ }
      fingers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      if (fingers.size === 1) touch = { x0: e.clientX, y0: e.clientY, moved: false, multi: false };
      else touch.multi = true;
    },

    move(e) {
      const finger = fingers.get(e.pointerId);
      const v = view();
      if (!finger || !v) return;
      const before = [...fingers.values()].map((f) => ({ ...f }));
      finger.x = e.clientX;
      finger.y = e.clientY;
      const after = [...fingers.values()];
      const r = element.getBoundingClientRect();
      if (after.length === 1) {
        if (!touch.moved && Math.hypot(e.clientX - touch.x0, e.clientY - touch.y0) <= TOUCH_SLOP) return;
        // the first move past a tap drags from where the finger went down, so the view stays under it
        const from = touch.moved ? before[0] : { x: touch.x0, y: touch.y0 };
        touch.moved = true;
        setView({ ...v, x: v.x - (e.clientX - from.x) / v.k, y: v.y - (e.clientY - from.y) / v.k });
      } else {
        touch.moved = true;
        const middle = (fs) => ({ x: (fs[0].x + fs[1].x) / 2 - r.left, y: (fs[0].y + fs[1].y) / 2 - r.top });
        const distance = (fs) => Math.hypot(fs[0].x - fs[1].x, fs[0].y - fs[1].y);
        const [m0, m1] = [middle(before), middle(after)];
        const [d0, d1] = [distance(before), distance(after)];
        // the point under the fingers' middle goes with it, at the zoom their distance gives
        const ux = v.x + m0.x / v.k, uy = v.y + m0.y / v.k;
        const k = d0 > 0 ? clamp(v.k * (d1 / d0), min, max) : v.k;
        setView({ k, x: ux - m1.x / k, y: uy - m1.y / k });
      }
    },

    up(e, cancelled = false) {
      if (!fingers.delete(e.pointerId)) return;
      if (element.hasPointerCapture?.(e.pointerId)) element.releasePointerCapture(e.pointerId);
      touchedAt = performance.now();
      if (fingers.size) return;
      const tap = touch && !touch.moved && !touch.multi && !cancelled;
      touch = null;
      if (!tap) { lastTap = null; return; }
      const now = performance.now();
      if (lastTap && now - lastTap.time < DOUBLE_TAP_MS && Math.hypot(e.clientX - lastTap.x, e.clientY - lastTap.y) < DOUBLE_TAP_PX) {
        lastTap = null;
        onDoubleTap?.(e);
        return;
      }
      lastTap = { time: now, x: e.clientX, y: e.clientY };
      onTap?.(e);
    },
  };
}
