// A virtualized file cannot delegate offscreen matches to the browser's DOM search. Search the
// full text model in short slices, then reveal and select a match in the rendered window.
export class TextFind {
  constructor(view, reveal) {
    this.view = view;
    this.reveal = reveal;
    this.bar = document.getElementById("netlist-find");
    this.input = document.getElementById("netlist-query");
    this.count = document.getElementById("netlist-matches");
    this.previous = document.getElementById("netlist-previous");
    this.next = document.getElementById("netlist-next");
    this.matches = [];
    this.version = 0;
    this.index = 0;
    this.input.addEventListener("input", () => {
      ++this.version;
      clearTimeout(this.timer);
      this.timer = setTimeout(() => { void this.scan(); }, 100);
    });
    this.input.addEventListener("keydown", e => {
      if ((e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === "f") { e.preventDefault(); this.input.select(); }
      if (e.key === "Enter") { e.preventDefault(); this.move(e.shiftKey ? -1 : 1); }
      if (e.key === "Escape") { e.preventDefault(); this.close(); }
    });
    this.previous.addEventListener("click", () => this.move(-1));
    this.next.addEventListener("click", () => this.move(1));
    document.getElementById("netlist-find-close").addEventListener("click", () => this.close());
  }
  open() {
    this.bar.hidden = false;
    this.input.focus();
    this.input.select();
    void this.scan();
  }
  changed(force = false) {
    if (!force && this.model === this.view()?.model) return;
    this.model = this.view()?.model;
    ++this.version;
    if (!this.bar.hidden) void this.scan();
  }
  async scan() {
    const version = ++this.version, view = this.view(), query = this.input.value;
    this.matches = [];
    this.index = 0;
    this.previous.disabled = this.next.disabled = true;
    this.count.textContent = query ? "Searching…" : "";
    if (!query || !view) return;
    const pattern = new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "giu");
    const matches = [];
    let yielded = performance.now();
    for (let line = 0; line < view.model.texts.length; line++) {
      if (view.semanticsOnly && !view.model.semantics.has(line)) continue;
      pattern.lastIndex = 0;
      let match;
      while ((match = pattern.exec(view.model.texts[line]))) matches.push({ line, col: match.index, length: match[0].length });
      if (line % 256 === 0 && performance.now() - yielded > 8) {
        await new Promise(resolve => setTimeout(resolve, 0));
        if (version !== this.version) return;
        yielded = performance.now();
      }
    }
    if (version !== this.version) return;
    this.matches = matches;
    this.previous.disabled = this.next.disabled = matches.length === 0;
    this.show();
  }
  move(by) {
    if (!this.matches.length) return;
    this.index = (this.index + by + this.matches.length) % this.matches.length;
    this.show();
  }
  show() {
    const match = this.matches[this.index], view = this.view();
    this.count.textContent = match ? `${this.index + 1} / ${this.matches.length}` : "No matches";
    if (!match || !view) return;
    this.reveal(match.line + 1);
    if (view.selection) {
      view.selection.anchor = { line: match.line, col: match.col };
      view.selection.focus = { line: match.line, col: match.col + match.length };
      view.selection.all = false;
      view.selection.paint();
    } else {
      const code = view.lines[match.line]?.querySelector(".code");
      if (!code) return;
      const point = (offset) => {
        const walker = document.createTreeWalker(code, NodeFilter.SHOW_TEXT);
        let node;
        while ((node = walker.nextNode())) { if (offset <= node.length) return [node, offset]; offset -= node.length; }
        return [code, code.childNodes.length];
      };
      window.getSelection().setBaseAndExtent(...point(match.col), ...point(match.col + match.length));
    }
  }
  close(focus = true) {
    ++this.version;
    clearTimeout(this.timer);
    this.bar.hidden = true;
    if (focus) this.view()?.source.focus({ preventScroll: true });
  }
}
