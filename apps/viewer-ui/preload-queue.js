// Scheduling has no UI side effects: preparing a file is never a navigation or a Library add.
export const IDLE_MS = 1500;
export const MAX_CANDIDATES = 128;

/**
 * The files to read while the viewer is idle (file-reading.js `preloadFile`): those of the subcircuits below the file
 * the Document `doc` shows, breadth first, by the levels each level calls, from the outline of the whole document; the
 * file shown aside, and each file once. The Library and PATH are not among them.
 */
export function preloadFiles(doc) {
  const outline = doc?.outline;
  if (!outline) return [];
  const at = outline.files.findIndex((f) => f.path === doc.file);
  if (at < 0) return [];
  const files = new Set([at]), levels = new Set(outline.files[at].levels), queue = [...outline.files[at].levels], out = [];
  for (let k = 0; k < queue.length && out.length < MAX_CANDIDATES; k++) {
    const level = outline.levels[queue[k]];
    if (!level) continue;
    if (level.file !== null && !files.has(level.file)) {
      files.add(level.file);
      out.push(outline.files[level.file].path);
    }
    for (const call of level.calls) {
      if (call.level === null || levels.has(call.level)) continue;
      levels.add(call.level);
      queue.push(call.level);
    }
  }
  return out;
}

/** One reading at a time; rest for 3x its duration (~25% of one core over repeated jobs).
 * This is a duty-cycle target, not a hard CPU cap during an individual compilation.
 */
export class PreloadQueue {
  constructor({ candidates, prepare, cancel, allowed, now = () => performance.now() }) {
    Object.assign(this, { candidates, prepare, cancel, allowed, now });
    this.generation = 0; this.running = false; this.queue = null; this.next = now() + IDLE_MS;
  }
  activity() { this.next = Math.max(this.next, this.now() + IDLE_MS); }
  reset() {
    this.generation++; this.queue = null; this.activity();
    Promise.resolve(this.cancel()).catch(() => {});
  }
  async tick() {
    if (this.running || this.now() < this.next || !this.allowed()) return;
    this.queue ??= this.candidates();
    const source = this.queue.shift();
    if (!source) return;
    const generation = this.generation, begin = this.now();
    this.running = true;
    try {
      const status = await this.prepare(source);
      if (generation !== this.generation) return;
      if (['busy', 'paused', 'cancelled'].includes(status)) this.queue.unshift(source);
      if (['full', 'disabled'].includes(status)) this.queue = [];
    } catch { /* A speculative failure is silent; opening the file reports its real error. */ }
    finally {
      this.next = Math.max(this.next, this.now() + Math.max(IDLE_MS, (this.now() - begin) * 3));
      this.running = false;
    }
  }
}
