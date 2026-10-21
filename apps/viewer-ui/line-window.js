// Large files keep the complete text/model and only a viewport plus overscan in the DOM.
// Folded/virtual files retain selection in original source coordinates for complete copies.
import { VirtualSelection } from "./virtual-selection.js";

const LIMIT = 1000;
const OVERSCAN = 40;

export class LineWindow {
  constructor(source, makeModel, decorate, folds = []) {
    this.source = source;
    this.makeModel = makeModel;
    this.folds = new Map(folds.map(([from, to]) => [from - 1, to - 1]));
    this.collapsed = new Set();
    this.decorate = decorate;
    this.el = document.createElement("div");
    this.el.className = "file";
    this.el.hidden = true;
    this.lines = [];
    this.rendered = new Map();
    this.scroll = 0;
    this.showFailed = true;
    this.inactive = new Map();
    this.height = parseFloat(getComputedStyle(source).lineHeight) || 19.375;
    this.first = 0;
    this.last = -1;
  }

  // Most auto-included files are never viewed. Do not tokenize them or create hidden DOM.
  get model() { this.initialize(); return this._model; }

  initialize() {
    if (this._model) return;
    const model = this._model = this.makeModel();
    this.folds = new Map([...model.folds, ...this.folds]);
    const count = model.texts.length;
    this.el.dataset.lineCount = String(count);
    this.gutter = Math.max(4.5, String(count).length + 1.4);
    this.el.style.setProperty("--line-number-width", `${this.gutter}em`);
    this.lines = new Array(count);
    this.virtual = count > LIMIT;
    const edges = new Int32Array(count + 1);
    this.ends = new Set();
    for (const [from, to] of this.folds) {
      if (from < 0 || to <= from || to >= count) { this.folds.delete(from); continue; }
      edges[from]++; edges[to + 1]--;
      this.ends.add(to);
    }
    this.depth = new Int32Array(count);
    let depth = 0;
    for (let i = 0; i < count; i++) this.depth[i] = depth += edges[i];
    if (this.virtual) {
      this.el.classList.add("virtual-file");
      this.before = document.createElement("div");
      this.after = document.createElement("div");
      this.before.className = this.after.className = "line-spacer";
      this.before.setAttribute("aria-hidden", "true");
      this.after.setAttribute("aria-hidden", "true");
      this.rows = document.createElement("div");
      this.el.append(this.before, this.rows, this.after);
      this.after.style.height = `${model.texts.length * this.height}px`;
    } else this.rows = this.el;
    this.selection = new VirtualSelection(this);
    if (this.pendingLine !== undefined) this.unfoldTo(this.pendingLine);
    this.mapLines();
    if (this.pendingLine !== undefined) {
      this.scroll = 4 + this.rowOf(this.pendingLine) * this.height;
      delete this.pendingLine;
    }
  }

  mapLines() {
    const count = this.model.texts.length;
    this.visible = [];
    this.positions = new Int32Array(count);
    let end = -1;
    for (let i = 0; i < count; i++) {
      if (!this.allows(i)) { this.positions[i] = this.visible.length; continue; }
      if (i <= end) { this.positions[i] = this.visible.length - 1; continue; }
      this.positions[i] = this.visible.length;
      this.visible.push(i);
      if (!this.filtered && this.collapsed.has(i)) end = this.folds.get(i);
    }
    this.first = 0;
    this.last = -1;
  }

  get filtered() { return this.rnlOnly || !this.showFailed; }

  allows(i) {
    return (!this.rnlOnly || this.model.rnl.has(i)) &&
      (!this.semanticsOnly || this.model.semantics.has(i)) &&
      (this.showFailed || (!this.model.failed.has(i) && !this.inactive.has(i + 1)));
  }

  setInactive(lines) {
    const same = this.inactive.size === lines.size && [...lines.keys()].every(line => this.inactive.has(line));
    this.inactive = lines;
    if (!same && !this.showFailed) this.refilter(true);
  }

  setFilters(filters) {
    if (Object.entries(filters).every(([key, value]) => this[key] === value)) return;
    Object.assign(this, filters);
    this.refilter();
  }

  refilter(preserve = false) {
    // Keep unopened auto-includes lazy.
    if (!this._model) return;
    const scroll = this.el.hidden ? this.scroll : this.source.scrollTop;
    const anchor = this.lineAt(Math.max(0, Math.floor((scroll - 4 + 1) / this.height)));
    const offset = Math.max(0, scroll - 4 - this.rowOf(anchor) * this.height);
    this.selection?.clear();
    this.mapLines();
    this.scroll = preserve ? Math.max(0, 4 + this.rowOf(anchor) * this.height + offset) : 0;
    // Grow the spacers before restoring scrollTop, which the browser otherwise clamps.
    this.refresh(true);
    if (!this.el.hidden) { this.source.scrollTop = this.scroll; this.refresh(); }
  }

  lineAt(row) { return this.visible?.[Math.max(0, Math.min(this.visible.length - 1, row))] ?? row; }
  rowOf(line) { return this.positions?.[line] ?? line; }

  toggleFold(from) {
    this.selection?.clear();
    const anchor = this.lineAt(Math.max(0, Math.floor((this.source.scrollTop - 4) / this.height)));
    const offset = this.source.scrollTop - 4 - this.rowOf(anchor) * this.height;
    if (this.collapsed.has(from)) this.collapsed.delete(from);
    else this.collapsed.add(from);
    this.mapLines();
    // Replace the spacers before assigning scrollTop, otherwise expansion is clamped to the
    // folded file's old height. Keep the viewport's original logical line whenever possible.
    this.refresh(true);
    this.source.scrollTop = Math.max(0, 4 + this.rowOf(anchor) * this.height + offset);
    this.refresh();
    this.lines[from]?.querySelector('.fold-toggle')?.focus({ preventScroll: true });
  }

  unfoldTo(line) {
    let changed = false;
    for (const from of this.collapsed) if (from < line && line <= this.folds.get(from)) {
      this.collapsed.delete(from);
      changed = true;
    }
    return changed;
  }

  expandTo(line) {
    if (this.unfoldTo(line)) { this.mapLines(); this.refresh(true); }
  }

  restoreLine(line) {
    // Editing a different file may rebuild this view before it has ever been displayed.
    // Preserve a logical position until the folded row map exists; do not force tokenization.
    if (!this._model) { this.pendingLine = line; return; }
    this.expandTo(line);
    this.scroll = 4 + this.rowOf(line) * this.lineHeight();
  }

  decorateFold(el, i) {
    el.classList.toggle('subckt-line', this.depth[i] > 0);
    el.classList.toggle('subckt-start', this.folds.has(i));
    el.classList.toggle('subckt-end', this.ends.has(i));
    if (!this.folds.has(i)) return;
    let toggle = el.querySelector('.fold-toggle');
    if (!toggle) {
      toggle = document.createElement('button');
      toggle.className = 'fold-toggle';
      toggle.type = 'button';
      toggle.addEventListener('click', e => { e.stopPropagation(); this.toggleFold(i); });
      toggle.addEventListener('dblclick', e => e.stopPropagation());
      el.querySelector('.no').append(toggle);
    }
    const collapsed = this.collapsed.has(i);
    toggle.hidden = this.filtered;
    toggle.textContent = collapsed ? '+' : '−';
    toggle.setAttribute('aria-expanded', String(!collapsed));
    toggle.setAttribute('aria-label', `${collapsed ? 'Expand' : 'Collapse'} subcircuit at line ${i + 1}`);
    toggle.title = `${collapsed ? 'Expand' : 'Collapse'} lines ${i + 1}–${this.folds.get(i) + 1}`;
    el.classList.toggle('subckt-collapsed', collapsed);
  }

  lineHeight() {
    // Rect edges lose subpixel precision millions of pixels from the viewport. CSS's used height
    // stays stable, including fractional line heights, when the scrollbar jumps across a huge file.
    const row = this.rendered.values().next().value;
    const measured = row && parseFloat(getComputedStyle(row).height);
    return measured || this.height;
  }

  refresh(force = false) {
    if (this.el.hidden) return;
    this.initialize();
    const source = this.source;
    this.selection?.syncNative();
    const height = this.lineHeight(), heightChanged = height !== this.height;
    this.height = height;
    const count = this.visible.length;
    if (!this.empty) {
      this.empty = document.createElement("p");
      this.empty.className = "placeholder";
      this.el.append(this.empty);
    }
    this.empty.textContent = this.semanticsOnly ? "No semantic RNL lines in this file." : "No lines match these filters.";
    this.empty.hidden = !this.filtered || count !== 0;
    const first = this.virtual ? Math.min(Math.max(0, count - 1), Math.max(0, Math.floor((source.scrollTop - 4) / this.height) - OVERSCAN)) : 0;
    const last = this.virtual ? Math.min(count - 1, Math.ceil((source.scrollTop + source.clientHeight) / this.height) + OVERSCAN) : count - 1;
    if (!force && !heightChanged && this.first === this.lineAt(first) && this.last === this.lineAt(last)) return;
    const scroll = source.scrollTop;
    this.first = this.lineAt(first);
    this.last = this.lineAt(last);
    for (const [i, el] of this.rendered) {
      const row = this.rowOf(i);
      if (row >= first && row <= last && this.visible[row] === i) continue;
      el.remove();
      this.rendered.delete(i);
      delete this.lines[i];
    }
    let at = this.rows.querySelector('.ln');
    for (let row = first; row <= last; row++) {
      const i = this.visible[row];
      let el = this.rendered.get(i);
      if (!el) {
        el = this.model.render(i);
        this.decorate(el, i + 1);
        this.decorateFold(el, i);
        this.rendered.set(i, el);
        this.lines[i] = el;
      } else if (force) { this.decorate(el, i + 1); this.decorateFold(el, i); }
      if (el !== at) this.rows.insertBefore(el, at);
      at = el.nextSibling;
    }
    if (this.virtual) {
      this.before.style.height = `${first * this.height}px`;
      this.after.style.height = `${Math.max(0, count - last - 1) * this.height}px`;
      // Keep horizontal extent stable when a wide line scrolls out of the DOM.
      const style = getComputedStyle(source), fontSize = parseFloat(style.fontSize);
      this.width ??= this.model.columns * fontSize * 0.62 + fontSize * (this.gutter + 2.8) + 3;
      for (const el of this.rendered.values()) this.width = Math.max(this.width, el.scrollWidth);
      this.el.style.minWidth = `${Math.ceil(this.width)}px`;
      source.scrollTop = scroll;
    }
    this.selection?.paint();
  }

  reveal(line, atTop = false) {
    let n = Math.max(0, Math.min(this.model.texts.length - 1, line - 1));
    if (this.filtered) {
      if (!this.visible.length) return;
      n = this.lineAt(this.rowOf(n));
    }
    this.expandTo(n);
    const top = this.virtual ? 4 + this.rowOf(n) * this.height : this.lines[n]?.offsetTop;
    if (top === undefined) return;
    this.source.scrollTop = Math.max(0, atTop ? top - 4 : top - this.source.clientHeight / 2 + this.height / 2);
    this.refresh();
    return this.lines[n];
  }

  destroy() { this.selection?.destroy(); }
}
