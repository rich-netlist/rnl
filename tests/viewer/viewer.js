// What the page tests share: opening the page with the fake shell, waiting for what it does, reading
// its state through window.viewer (apps/viewer-ui/README.md), and finding on the screen where the
// objects of the level shown are drawn. Every wait is for a condition, never for a time.

import { expect, test as base } from "@playwright/test";

export { expect };

/** The page with the fake shell. */
class Viewer {
  constructor(page) {
    this.page = page;
  }

  /**
   * Open the page with the fake shell, with `query` added to its URL. With a fixture, wait until
   * its Document is shown; without, until the page has asked the shell what to open at start.
   */
  async open(fixture, query = {}) {
    const params = new URLSearchParams({ shell: "fake", ...query });
    if (fixture) params.set("open", fixture);
    await this.page.goto(`${process.env.RNL_VIEWER_URL}/index.html?${params}`);
    await this.started();
    if (fixture) await this.idle();
  }

  /** Wait until the page has started: its shell has the command handler and the menu is built. */
  async started() {
    await this.page.waitForFunction(() => window.fakeShell && window.fakeShell.calls.some((c) => c[0] === "initial"));
  }

  /** Wait until a Document is shown and nothing is being read. */
  async idle() {
    await this.page.waitForFunction(() => window.viewer.doc && !document.body.classList.contains("busy"));
  }

  /** Run `action`, then wait until the build it asks the shell for is done. */
  async build(action) {
    const before = await this.builds();
    await action();
    await this.page.waitForFunction(
      (n) => window.fakeShell.calls.filter((c) => c[0] === "build").length > n && !document.body.classList.contains("busy"),
      before,
    );
  }

  /** How many builds the page has asked the shell for. */
  builds() {
    return this.page.evaluate(() => window.fakeShell.calls.filter((c) => c[0] === "build").length);
  }

  /** The calls the shell has had of one kind, such as "pick", "build" or "setCheck". */
  calls(kind) {
    return this.page.evaluate((k) => window.fakeShell.calls.filter((c) => c[0] === k).map((c) => c.slice(1)), kind);
  }

  /** What the page shows: the level's index and title, and the selected object as "kind name", in its level. */
  state() {
    return this.page.evaluate(() => {
      const v = window.viewer;
      const o = v.doc && v.selected !== null ? v.doc.objects[v.selected] : null;
      return {
        level: v.level,
        title: v.doc && v.doc.levels[v.level] ? v.doc.levels[v.level].title : null,
        selected: o ? `${o.kind} ${o.name} in ${o.level_title}` : null,
      };
    });
  }

  /** Wait until the selection is `text`, as state() gives it, or null. */
  async selected(text) {
    await expect.poll(async () => (await this.state()).selected).toBe(text);
  }

  /** The index in doc.objects of the object of a kind and a name on the level with that title. */
  object(kind, name, title) {
    return this.page.evaluate(
      ([kind, name, title]) =>
        window.viewer.doc.objects.findIndex((o) => o.kind === kind && o.name === name && o.level_title === title),
      [kind, name, title],
    );
  }

  /** The drawing's view box, which the view sets. */
  viewBox() {
    return this.page.locator("#canvas > svg").getAttribute("viewBox");
  }

  /**
   * Wait until the view fits the canvas's size: when a panel is shown or hidden, the page moves the
   * view once the canvas has its new size, before the next frame.
   */
  async settled() {
    await this.page.waitForFunction(() => {
      const canvas = document.getElementById("canvas");
      const svg = document.querySelector("#canvas > svg");
      if (!svg) return true;
      const [, , w, h] = svg.getAttribute("viewBox").split(" ").map(Number);
      return Math.abs(w * canvas.clientHeight - h * canvas.clientWidth) <= 1e-6 * w * canvas.clientHeight;
    });
  }

  /** The page point of a point of the drawing shown, in its own units, as a click there hits it. */
  async point(u, v) {
    await this.settled();
    return this.page.evaluate(
      ([u, v]) => {
        const canvas = document.getElementById("canvas");
        const [x, y, w] = document.querySelector("#canvas > svg").getAttribute("viewBox").split(" ").map(Number);
        const r = canvas.getBoundingClientRect();
        const k = canvas.clientWidth / w;
        return { x: r.left + (u - x) * k, y: r.top + (v - y) * k };
      },
      [u, v],
    );
  }

  /**
   * Where an object of the level shown is drawn, in page pixels: the middle of a "box" that stands
   * for it, the symbol of an instance or a ground of a net; a "pin" on its net; or the middle of
   * the first segment of a "wire" of its net.
   */
  async where(name, kind = "box") {
    const [u, v] = await this.page.evaluate(
      ([name, kind]) => {
        const doc = window.viewer.doc;
        const level = doc.levels[window.viewer.level];
        const hit = level.hits.find((h) => h.kind === kind && h.objects.some((o) => doc.objects[o].name === name));
        if (!hit) throw new Error(`${level.title} has no ${kind} for ${name}`);
        const [a, b = a] = hit.points;
        return [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
      },
      [name, kind],
    );
    const p = await this.point(u, v);
    const r = await this.page.locator("#canvas").boundingBox();
    if (p.x < r.x || p.x > r.x + r.width || p.y < r.y || p.y > r.y + r.height) {
      throw new Error(`the ${kind} of ${name} is outside the view`);
    }
    return p;
  }

  /** Click where an object of the level shown is drawn; see where(). */
  async click(name, kind = "box") {
    const p = await this.where(name, kind);
    await this.page.mouse.click(p.x, p.y);
  }

  /** A point of the drawing's canvas where nothing is drawn, near its top left corner while the level is fitted. */
  async empty() {
    const r = await this.page.locator("#canvas").boundingBox();
    return { x: r.x + 6, y: r.y + 6 };
  }

  /** A line of the file shown in the text pane. */
  line(n) {
    return this.page.locator(`#source .file:not([hidden]) .ln[data-line="${n}"]`);
  }

  /** Whether an element is wholly within the scrolled view of the box around it, such as #source. */
  within(locator, box = "#source") {
    return locator.evaluate((e, box) => {
      const [r, b] = [document.querySelector(box).getBoundingClientRect(), e.getBoundingClientRect()];
      return b.height > 0 && b.top >= r.top - 0.5 && b.bottom <= r.bottom + 0.5;
    }, box);
  }

  /** The places of the text pane's file shown that name an object, by its index in doc.objects. */
  places(o) {
    return this.page.locator(`#source .file:not([hidden]) [data-obj~="${o}"]`);
  }

  /** The numbers of the lines marked as the selected instance's statement. */
  markedLines() {
    return this.page.locator("#source .ln.hl-line").evaluateAll((els) => els.map((e) => Number(e.dataset.line)));
  }

  /** The lines of the file shown that name an object, from the Document's spans. */
  spanLines(o) {
    return this.page.evaluate((o) => {
      const shown = [...document.querySelectorAll("#source .file")].findIndex((f) => !f.hidden);
      const lines = window.viewer.doc.spans.filter((s) => s.object === o && s.file === shown).map((s) => s.line);
      return [...new Set(lines)].sort((a, b) => a - b);
    }, o);
  }

  /** The lines of the file shown that have a marked place of the selection. */
  markedPlaces() {
    return this.page
      .locator("#source .file:not([hidden]) .hl")
      .evaluateAll((els) => [...new Set(els.map((e) => Number(e.closest(".ln").dataset.line)))].sort((a, b) => a - b));
  }

  /** The data-id of the SVG groups drawn as selected, each once, sorted. */
  selectedGroups() {
    return this.page
      .locator("#canvas > svg g.sel")
      .evaluateAll((els) => [...new Set(els.map((e) => e.getAttribute("data-id")))].sort());
  }

  /** The page's own menu bar: a menu's title, or an item by its command id. */
  menuTitle(text) {
    return this.page.locator("#menubar .menu-title", { hasText: text });
  }

  menuItem(id) {
    return this.page.locator(`#menubar .menu-item[data-id="${id}"]`);
  }

  /** Drag from one page point to another, in steps, with the left button or the one named. */
  async drag(from, to, button = "left") {
    await this.page.mouse.move(from.x, from.y);
    await this.page.mouse.down({ button });
    await this.page.mouse.move(to.x, to.y, { steps: 8 });
    await this.page.mouse.up({ button });
  }

  /** The middle of an element, in page pixels. */
  async middle(locator) {
    const r = await locator.boundingBox();
    return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
  }

  /** A value the page keeps in localStorage, parsed. */
  stored(key) {
    return this.page.evaluate((k) => JSON.parse(localStorage.getItem(k)), key);
  }
}

/** Playwright's test, with `viewer`, the page with the fake shell. */
export const test = base.extend({
  viewer: async ({ page }, use) => {
    await use(new Viewer(page));
  },
});
