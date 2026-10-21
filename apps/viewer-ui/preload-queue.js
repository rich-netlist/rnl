// Scheduling has no UI side effects: preparing a file is never a navigation or a Library add.
export const IDLE_MS = 1500;
export const MAX_CANDIDATES = 128;

/** Breadth-first descendants first, then the shell's library/search-path files. */
export function preloadSources(doc, current, source, shell) {
  if (!doc || !source || source.readonly || source.temporary || source.text !== undefined) return [];
  const key = path => shell.platform === 'desktop' ? path.replaceAll('\\', '/').toLowerCase() : path;
  const paths = new Set([key(source.path || doc.path)]), levels = new Set([current]), queue = [current], out = [];
  const add = candidate => {
    const path = candidate?.path;
    if (!path || candidate.readonly || paths.has(key(path)) || !/\.(sp|spi|spice|cir|ckt|net|netlist|scs|inc|lib)$/i.test(path)) return;
    paths.add(key(path)); out.push(candidate);
  };
  for (let at = 0; at < queue.length && out.length < MAX_CANDIDATES; at++) {
    const id = queue[at];
    const level = doc.levels[id];
    if (!level) continue;
    const file = level.lines?.[0];
    if (file !== undefined) add(shell.sourceForFile(source, doc, file));
    for (const child of level.children) if (child.level != null && !levels.has(child.level)) {
      levels.add(child.level); queue.push(child.level);
    }
  }
  for (const candidate of shell.preloadCandidates?.() || []) {
    if (out.length >= MAX_CANDIDATES) break;
    add(candidate);
  }
  return out;
}

/** One build at a time; rest for 3x its duration (~25% of one core over repeated jobs).
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
