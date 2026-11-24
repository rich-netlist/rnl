// The cache of what the core gives for each file, for loading a design file by file (notes of the Viewer line,
// per-file loading): a result is kept so that its file is not interpreted again while the file and what it used are
// the same. A result is kept under a key (`cacheKey`) with the inputs it records, and on the way out it is given only
// when the caller's `valid(inputs)` says they still hold; one that does not is dropped. It is kept in memory for the
// page alone, within a limit in bytes, and the least recently used results go first; a limit of 0 keeps nothing.
// A value is kept as it is given, and given as it is kept: never copied.
// Nothing is written to the browser's storage: `removeOldStore` deletes the store of results that earlier versions of
// the page kept in the site's IndexedDB. Nothing here knows what a result holds: it is any value JSON writes.

import { STORAGE_PREFIX } from "./storage-prefix.js";

const MIB = 1024 * 1024;
/** The memory a page gives to results, by default: the desktop viewer's own cache has as much. */
export const MEMORY_LIMIT = 256 * MIB;
/** A result larger than this share of a limit is not kept there: it would push out everything else. */
const LARGEST_SHARE = 4;

/**
 * About how many bytes `value` takes as JSON text, at two bytes a character, as the limits count: the lengths of its
 * strings, numbers, names and punctuation, escapes aside. Unlike `JSON.stringify(value).length`, it makes no text.
 */
export function sizeOf(value) {
  let size = 0;
  const stack = [value];
  while (stack.length) {
    const v = stack.pop();
    if (typeof v === "string") size += v.length + 2;
    else if (typeof v === "number") {
      size += Number.isInteger(v) && v >= 0 && v < 1e6 ? (v < 10 ? 1 : v < 100 ? 2 : v < 1000 ? 3 : v < 1e4 ? 4 : v < 1e5 ? 5 : 6) : String(v).length;
    } else if (typeof v === "boolean") size += 5;
    else if (v === null || typeof v !== "object") size += 4;
    else if (Array.isArray(v)) {
      size += 1 + Math.max(v.length, 1);
      for (let k = 0; k < v.length; k++) stack.push(v[k]);
    } else {
      size += 1;
      for (const key in v) {
        if (v[key] === undefined) continue;
        size += key.length + 4;
        stack.push(v[key]);
      }
    }
  }
  return size * 2;
}

/** `value` as JSON with the keys of every object sorted, so that the same value is the same text however it was built. */
function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().filter((k) => value[k] !== undefined).map((k) => `${JSON.stringify(k)}:${canonical(value[k])}`).join(",")}}`;
  }
  return JSON.stringify(value ?? null);
}

/** The key of a result: the SHA-256, in hexadecimal, of `parts` written as canonical JSON. */
export async function cacheKey(parts) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(canonical(parts)));
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/**
 * Delete the store of results that the page kept in the site's IndexedDB before it kept them in memory alone (the
 * database `results` after the page's storage prefix), so that nothing of it is left on the computer. A browser that has
 * none, or refuses, is left as it is.
 */
export function removeOldStore(name = "results") {
  try {
    if (typeof indexedDB !== "undefined") indexedDB.deleteDatabase(STORAGE_PREFIX + name);
  } catch { /* nothing to delete, or not ours to */ }
}

/**
 * The results kept for a page, in memory, within `memoryLimit` bytes (their JSON text, counted at two bytes a character,
 * as `sizeOf` estimates it). A value is kept as it is given, not copied: what holds it must not change it.
 * A result kept with `counted: false`, such as the outline of a document, takes its room like any other but is not
 * among the entries, hits and misses the stats tell.
 */
export class ResultCache {
  constructor({ memoryLimit = MEMORY_LIMIT } = {}) {
    this.memory = new Map(); // key → { value, inputs, size, counted }, the least recently used first
    this.memoryUsed = 0;
    this.memoryLimit = memoryLimit;
    this.hits = 0;
    this.misses = 0;
  }

  /** The value kept under `key` when `valid(inputs)` says that what it was made from still holds; null otherwise. */
  async get(key, valid = async () => true, { counted = true } = {}) {
    const entry = this.memory.get(key);
    if (!entry || !(await valid(entry.inputs))) {
      if (entry && this.memory.get(key) === entry) this.forget(key);
      if (counted) this.misses++;
      return null;
    }
    if (counted) this.hits++;
    this.remember(key, entry);
    return entry.value;
  }

  /**
   * The value kept under `key` when `valid(inputs)` says that what it was made from still holds, as `get` gives it, but
   * looked at only: it counts as no hit or miss, is not made the most recently used, and is not dropped when it does not
   * hold. Null otherwise.
   */
  async peek(key, valid = async () => true) {
    const entry = this.memory.get(key);
    return entry && (await valid(entry.inputs)) ? entry.value : null;
  }

  /**
   * Keep `value`, made from `inputs`, under `key`; with `evict` false, only into the room that is free, nothing else
   * going for it (a reading preloaded). Whether it was kept.
   */
  async put(key, value, inputs = [], { counted = true, evict = true } = {}) {
    const size = sizeOf(value);
    const free = this.memoryLimit - this.memoryUsed + (this.memory.get(key)?.size ?? 0);
    if (this.fits(size) && (evict || size <= free)) {
      this.remember(key, { value, inputs, size, counted });
      this.trimMemory();
      return true;
    }
    if (evict) this.forget(key);
    return false;
  }

  /** Whether the memory has no room left. */
  full() { return this.memoryUsed >= this.memoryLimit; }

  async delete(key) {
    this.forget(key);
  }

  /** A new limit, in bytes; what no longer fits goes, the least recently used first. */
  async setLimits({ memoryLimit = this.memoryLimit } = {}) {
    this.memoryLimit = memoryLimit;
    this.trimMemory();
  }

  /** Empty the memory, and count again from 0, unless `counts` is false. */
  async clear({ counts = true } = {}) {
    this.memory.clear();
    this.memoryUsed = 0;
    if (counts) this.hits = this.misses = 0;
  }

  /** What is kept, for File › Memory Cache: the entries and bytes in memory, the limit, hits and misses. */
  stats() {
    let entries = 0;
    for (const entry of this.memory.values()) if (entry.counted !== false) entries++;
    return { kind: "readings", entries, used: this.memoryUsed, limit: this.memoryLimit, hits: this.hits, misses: this.misses };
  }

  /** Whether a result of `size` bytes is kept in memory at all. */
  fits(size) { return this.memoryLimit > 0 && size * LARGEST_SHARE <= this.memoryLimit; }

  remember(key, entry) {
    this.forget(key);
    this.memory.set(key, entry);
    this.memoryUsed += entry.size;
  }

  forget(key) {
    const old = this.memory.get(key);
    if (!old) return;
    this.memory.delete(key);
    this.memoryUsed -= old.size;
  }

  trimMemory() {
    for (const [key, entry] of this.memory) {
      if (this.memoryUsed <= this.memoryLimit) break;
      this.memory.delete(key);
      this.memoryUsed -= entry.size;
    }
  }
}
