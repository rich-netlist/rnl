// Text selection is stored as file coordinates, so recycling a row never truncates copied text.
// The native selection paints the visible intersection. Copy reads the complete model instead.
const compare = (a, b) => a.line - b.line || a.col - b.col;

export class VirtualSelection {
  constructor(view) {
    this.view = view;
    this.down = (e) => this.pointerDown(e);
    view.el.addEventListener("pointerdown", this.down);
  }
  has() { return this.anchor && this.focus && compare(this.anchor, this.focus) !== 0; }
  clear() {
    this.stopDrag();
    this.anchor = this.focus = this.applied = null;
    this.all = false;
    const selection = window.getSelection();
    if (this.view.el.contains(selection.anchorNode)) selection.removeAllRanges();
  }
  position(node, offset) {
    const el = node?.nodeType === 1 ? node : node?.parentElement;
    const row = el?.closest(".ln"), code = row?.querySelector(".code");
    if (!row || !this.view.rows.contains(row)) return null;
    let col = 0;
    if (code.contains(node)) {
      const range = document.createRange();
      range.setStart(code, 0);
      range.setEnd(node, offset);
      col = range.toString().length;
    } else if (node === row && offset > 1) col = code.textContent.length;
    return { line: Number(row.dataset.line) - 1, col };
  }
  syncNative() {
    const selection = window.getSelection(), a = this.applied;
    if (a && selection.anchorNode === a.node && selection.anchorOffset === a.offset &&
      selection.focusNode === a.focus && selection.focusOffset === a.focusOffset) return;
    const anchor = this.position(selection.anchorNode, selection.anchorOffset);
    const focus = this.position(selection.focusNode, selection.focusOffset);
    if (!anchor || !focus) return;
    if (!a || selection.anchorNode !== a.node || selection.anchorOffset !== a.offset) this.anchor = anchor;
    this.focus = focus;
    this.all = false;
  }
  point(pos) {
    const code = this.view.lines[pos.line]?.querySelector(".code");
    if (!code) return null;
    const walker = document.createTreeWalker(code, NodeFilter.SHOW_TEXT);
    let node, left = pos.col;
    while ((node = walker.nextNode())) {
      if (left <= node.length) return [node, left];
      left -= node.length;
    }
    return [code, code.childNodes.length];
  }
  paint() {
    if (!this.has()) return;
    const v = this.view, backwards = compare(this.anchor, this.focus) > 0;
    let start = backwards ? this.focus : this.anchor, end = backwards ? this.anchor : this.focus;
    if (end.line < v.first || start.line > v.last) return;
    if (start.line < v.first) start = { line: v.first, col: 0 };
    if (end.line > v.last) end = { line: v.last, col: v.model.texts[v.last].length };
    const a = this.point(backwards ? end : start), b = this.point(backwards ? start : end);
    if (!a || !b) return;
    const selection = window.getSelection();
    selection.setBaseAndExtent(...a, ...b);
    this.applied = { node: selection.anchorNode, offset: selection.anchorOffset,
      focus: selection.focusNode, focusOffset: selection.focusOffset };
  }
  atPoint(x, y) {
    const v = this.view, rect = v.source.getBoundingClientRect();
    y = Math.max(rect.top + 1, Math.min(rect.bottom - 1, y));
    const line = v.lineAt(Math.floor((y - rect.top + v.source.scrollTop - 4) / v.height));
    const code = v.lines[line]?.querySelector(".code");
    if (!code) return { line, col: 0 };
    const caret = document.caretPositionFromPoint?.(x, y);
    const range = !caret && document.caretRangeFromPoint?.(x, y);
    const pos = caret ? this.position(caret.offsetNode, caret.offset) : range ? this.position(range.startContainer, range.startOffset) : null;
    return pos?.line === line ? pos : { line, col: x < code.getBoundingClientRect().left ? 0 : v.model.texts[line].length };
  }
  pointerDown(e) {
    if (e.button !== 0 || e.pointerType !== "mouse" || !e.target.closest(".ln") || e.target.closest('button, a')) return;
    this.syncNative();
    const pos = this.atPoint(e.clientX, e.clientY);
    if (!e.shiftKey || !this.anchor || e.target.closest("[data-obj]")) this.anchor = pos;
    this.focus = pos;
    this.all = false;
    e.preventDefault();
    this.view.source.focus({ preventScroll: true });
    if (e.detail >= 2) {
      const text = this.view.model.texts[pos.line];
      const segment = [...new Intl.Segmenter(undefined, { granularity: "word" }).segment(text)]
        .find(s => s.index <= pos.col && s.index + s.segment.length > pos.col);
      if (segment) {
        this.anchor = { line: pos.line, col: segment.index };
        this.focus = { line: pos.line, col: segment.index + segment.segment.length };
      }
    }
    if (this.has()) this.paint();
    else window.getSelection().removeAllRanges();
    this.stopDrag();
    this.drag = new AbortController();
    const signal = this.drag.signal;
    const move = (event) => {
      this.mouse = { x: event.clientX, y: event.clientY };
      this.focus = this.atPoint(event.clientX, event.clientY);
      this.paint();
      if (!this.frame) this.frame = requestAnimationFrame(() => this.autoScroll());
    };
    window.addEventListener("pointermove", move, { signal });
    for (const type of ["pointerup", "pointercancel", "blur"]) {
      window.addEventListener(type, () => this.stopDrag(), { signal, once: true });
    }
  }
  stopDrag() {
    this.drag?.abort();
    cancelAnimationFrame(this.frame);
    this.frame = 0;
    this.mouse = null;
  }
  autoScroll() {
    this.frame = 0;
    if (!this.mouse) return;
    const v = this.view, rect = v.source.getBoundingClientRect(), y = this.mouse.y;
    const speed = y < rect.top + 18 ? -Math.max(4, (rect.top + 18 - y) / 2) :
      y > rect.bottom - 18 ? Math.max(4, (y - rect.bottom + 18) / 2) : 0;
    if (!speed) return;
    const before = v.source.scrollTop;
    v.source.scrollTop += speed;
    if (v.source.scrollTop === before) return;
    v.refresh();
    this.focus = this.atPoint(this.mouse.x, y);
    this.paint();
    this.frame = requestAnimationFrame(() => this.autoScroll());
  }
  key(e) {
    if ((e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === "a") {
      if (!this.view.visible.length) { e.preventDefault(); return; }
      this.anchor = { line: this.view.filtered ? this.view.visible[0] : 0, col: 0 };
      const last = this.view.filtered ? this.view.visible.at(-1) : this.view.model.texts.length - 1;
      this.focus = { line: last, col: this.view.model.texts[last].length };
      this.all = true;
      this.paint();
      e.preventDefault();
    } else if (e.key === "Escape" && this.has()) { this.clear(); e.preventDefault(); }
  }
  copy(e) {
    this.syncNative();
    if (!this.has() || !e.clipboardData) return;
    const texts = this.view.model.texts;
    let a = this.anchor, b = this.focus;
    if (compare(a, b) > 0) [a, b] = [b, a];
    const indices = this.view.filtered ? this.view.visible.filter(n => n >= a.line && n <= b.line)
      : Array.from({length: b.line - a.line + 1}, (_, i) => a.line + i);
    const lines = indices.map(n => texts[n]);
    if (!lines.length) return;
    if (lines.length === 1) lines[0] = lines[0].slice(a.col, b.col);
    else { lines[0] = lines[0].slice(a.col); lines[lines.length - 1] = lines.at(-1).slice(0, b.col); }
    e.clipboardData.setData("text/plain", this.all && !this.view.filtered ? this.view.model.text : lines.join("\n"));
    e.preventDefault();
  }
  destroy() {
    this.stopDrag();
    this.view.el.removeEventListener("pointerdown", this.down);
    this.clear();
  }
}
