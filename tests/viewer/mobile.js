// What the mobile tests (specs/mobile*.spec.js) share: fingers on a touch screen, one or two,
// pressed, moved and lifted at page points; the view of the drawing; and whether the page itself
// scrolled or zoomed.
//
// In Chromium they go through the DevTools protocol's Input.dispatchTouchEvent, so the browser
// itself makes the pointer events of them, hit-tests them, honours touch-action and recognises taps:
// what a phone's touch screen gives the page. Playwright has no such way for more than one finger
// in WebKit, so there the page gets pointer events that the test makes, `pointerType` "touch", sent
// to the element under the finger at its press and kept on it, as a browser captures a touch
// pointer: they test what the page does with two fingers, not how WebKit makes them. A browser
// refuses to capture a pointer it did not make, so the init script lets the page capture these.
// A tap is Playwright's touchscreen.tap in both browsers.

import { test as base } from "./viewer.js";

export { expect } from "./viewer.js";

/** Make Element's pointer capture accept the pointers the test makes; before the page loads. */
function acceptSyntheticCapture() {
  const ids = (window.__syntheticPointers = new Set());
  const captured = new WeakMap();
  const proto = Element.prototype;
  const [set, release, has] = [proto.setPointerCapture, proto.releasePointerCapture, proto.hasPointerCapture];
  proto.setPointerCapture = function (id) {
    if (!ids.has(id)) return set.call(this, id);
    if (!captured.has(this)) captured.set(this, new Set());
    captured.get(this).add(id);
  };
  proto.releasePointerCapture = function (id) {
    if (!ids.has(id)) return release.call(this, id);
    captured.get(this)?.delete(id);
  };
  proto.hasPointerCapture = function (id) {
    if (!ids.has(id)) return has.call(this, id);
    return !!captured.get(this)?.has(id);
  };
}

export class Fingers {
  /** `browserName` as Playwright's fixture gives it. */
  constructor(page, browserName) {
    this.page = page;
    this.real = browserName === "chromium";
    // the points of the fingers down, by finger
    this.down = [];
  }

  /** Get the page ready for fingers; before it is opened. */
  async setUp() {
    if (this.real) this.cdp = await this.page.context().newCDPSession(this.page);
    else await this.page.addInitScript(acceptSyntheticCapture);
  }

  /** Press fingers at `points`, which add to those down. */
  async press(...points) {
    const first = this.down.length;
    this.down.push(...points);
    if (this.real) return this.send("touchStart");
    await this.synthetic("pointerdown", points.map((p, i) => [first + i, p]));
  }

  /** Move the fingers down to `points`, one for each, in order. */
  async move(...points) {
    if (points.length !== this.down.length) throw new Error(`${this.down.length} fingers are down, not ${points.length}`);
    this.down = points;
    if (this.real) return this.send("touchMove");
    await this.synthetic("pointermove", points.map((p, i) => [i, p]));
  }

  /** Lift the last `n` fingers down, all of them by default. */
  async lift(n = this.down.length) {
    const kept = this.down.length - n;
    const lifted = this.down.slice(kept).map((p, i) => [kept + i, p]);
    this.down = this.down.slice(0, kept);
    // the protocol lifts the fingers a touchEnd names, and keeps the others down
    if (this.real) {
      const touchPoints = lifted.map(([id, p]) => ({ x: p.x, y: p.y, id }));
      return this.cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints });
    }
    await this.synthetic("pointerup", lifted);
  }

  /** Drag one finger from one point to another, in steps. */
  async drag(from, to, steps = 8) {
    await this.press(from);
    for (let i = 1; i <= steps; i++) await this.move(between(from, to, i / steps));
    await this.lift();
  }

  /**
   * Two fingers on a horizontal line through `center`, `from` pixels apart, spread or pinched to
   * `to` pixels apart in steps while their middle moves by `shift`, then lifted.
   */
  async pinch(center, from, to, { steps = 8, shift = { x: 0, y: 0 } } = {}) {
    const at = (t) => {
      const d = (from + (to - from) * t) / 2;
      const c = { x: center.x + shift.x * t, y: center.y + shift.y * t };
      return [{ x: c.x - d, y: c.y }, { x: c.x + d, y: c.y }];
    };
    await this.press(...at(0));
    for (let i = 1; i <= steps; i++) await this.move(...at(i / steps));
    await this.lift();
  }

  /** A tap, as the browser recognises it. */
  async tap(p) {
    await this.page.touchscreen.tap(p.x, p.y);
  }

  /** Two taps at a point in quick succession. */
  async doubleTap(p) {
    await this.tap(p);
    await this.tap(p);
  }

  /** A touchStart or touchMove of all the fingers down; the protocol presses those it has not seen. */
  send(type) {
    const touchPoints = this.down.map((p, id) => ({ x: p.x, y: p.y, id }));
    return this.cdp.send("Input.dispatchTouchEvent", { type, touchPoints });
  }

  synthetic(type, fingers) {
    return this.page.evaluate(
      ([type, fingers]) => {
        window.__fingerTargets ??= new Map();
        for (const [finger, p] of fingers) {
          const id = 1000 + finger;
          if (type === "pointerdown") {
            window.__syntheticPointers.add(id);
            window.__fingerTargets.set(id, document.elementFromPoint(p.x, p.y));
          }
          const target = window.__fingerTargets.get(id);
          if (!target) continue;
          target.dispatchEvent(
            new PointerEvent(type, {
              pointerId: id,
              pointerType: "touch",
              isPrimary: finger === 0,
              clientX: p.x,
              clientY: p.y,
              screenX: p.x,
              screenY: p.y,
              width: 20,
              height: 20,
              pressure: type === "pointerup" ? 0 : 0.5,
              button: type === "pointermove" ? -1 : 0,
              buttons: type === "pointerup" ? 0 : 1,
              bubbles: true,
              cancelable: true,
              composed: true,
            }),
          );
          if (type === "pointerup") {
            window.__fingerTargets.delete(id);
            window.__syntheticPointers.delete(id);
          }
        }
      },
      [type, fingers],
    );
  }
}

function between(a, b, t) {
  return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t };
}

/** The page tests' `test`, with `fingers` too, ready before the page opens. */
export const test = base.extend({
  fingers: async ({ page, browserName }, use) => {
    const fingers = new Fingers(page, browserName);
    await fingers.setUp();
    await use(fingers);
  },
});

/**
 * The view of the drawing: its view box, `x`, `y`, `w` and `h` in the drawing's units, `k` the page
 * pixels of a unit, and the canvas's box on the page, `r`.
 */
export async function view(page) {
  return page.evaluate(() => {
    const canvas = document.getElementById("canvas");
    const [x, y, w, h] = document.querySelector("#canvas > svg").getAttribute("viewBox").split(/\s+/).map(Number);
    const b = canvas.getBoundingClientRect();
    return { x, y, w, h, k: canvas.clientWidth / w, r: { x: b.left, y: b.top, width: b.width, height: b.height } };
  });
}

/** The point of the drawing under a page point, in its units. */
export async function under(page, p) {
  const v = await view(page);
  return { u: v.x + (p.x - v.r.x) / v.k, v: v.y + (p.y - v.r.y) / v.k };
}

/** The page point of a point of the drawing. */
export async function over(page, { u, v: w }) {
  const v = await view(page);
  return { x: v.r.x + (u - v.x) * v.k, y: v.r.y + (w - v.y) * v.k };
}

/** The middle of the canvas, in page pixels. */
export async function canvasMiddle(page) {
  const { r } = await view(page);
  return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
}

/**
 * Whether the page itself moved: its zoom by the browser (the visual viewport's scale), how far it
 * is scrolled, and how much wider than the screen it is.
 */
export function pageMoved(page) {
  return page.evaluate(() => ({
    scale: window.visualViewport ? window.visualViewport.scale : 1,
    scrollX: window.scrollX,
    scrollY: window.scrollY,
    wider: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth,
  }));
}

/** What pageMoved() gives of a page that did not move. */
export const UNMOVED = { scale: 1, scrollX: 0, scrollY: 0, wider: 0 };
