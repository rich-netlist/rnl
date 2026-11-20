// The cache of what the core gives for each file, for loading a design file by file (notes of the Viewer line,
// per-file loading): a result is kept so that its file is not interpreted again while the file and what it used are
// the same. A result is kept under a key (`cacheKey`) with the inputs it records, and on the way out it is given only
// when the caller's `valid(inputs)` says they still hold; one that does not is dropped. It is kept in memory for the
// page, and, where the browser lets it, in the site's IndexedDB (`openStore`), where another tab and a later visit find
// it. Each has a limit in bytes, and the least recently used results go first; a limit of 0 keeps nothing. A store is
// stamped with a `format`, the core's version and the shape of its results, and a store of another format is emptied
// when it is opened. Nothing here knows what a result holds: it is any value JSON writes.
//
// Losing the cache only makes loading slower: every failure of the browser's storage is taken as a miss.

import { STORAGE_PREFIX } from "./storage-prefix.js";

const MIB = 1024 * 1024;
/** The memory a page gives to results, by default: the desktop viewer's own cache has as much. */
export const MEMORY_LIMIT = 256 * MIB;
/** The bytes a site keeps in IndexedDB, by default; `storeLimit` lowers it to half of what the browser says it may keep. */
export const STORE_LIMIT = 1024 * MIB;
/** A result larger than this share of a limit is not kept there: it would push out everything else. */
const LARGEST_SHARE = 4;

/** The store's limit for this browser: `wanted`, and no more than half of what the browser says the site may keep. */
export async function storeLimit(wanted = STORE_LIMIT) {
  try {
    const { quota } = await navigator.storage.estimate();
    return Number.isFinite(quota) && quota > 0 ? Math.min(wanted, Math.floor(quota / 2)) : wanted;
  } catch {
    return wanted;
  }
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

/** The bytes of `text`, gzip-compressed where the browser can. */
async function pack(text) {
  const bytes = new TextEncoder().encode(text);
  if (typeof CompressionStream !== "function") return { bytes, gzip: false };
  const packed = await new Response(new Blob([bytes]).stream().pipeThrough(new CompressionStream("gzip"))).arrayBuffer();
  return { bytes: new Uint8Array(packed), gzip: true };
}

async function unpack({ bytes, gzip }) {
  const raw = gzip ? await new Response(new Blob([bytes]).stream().pipeThrough(new DecompressionStream("gzip"))).arrayBuffer() : bytes;
  return new TextDecoder().decode(raw);
}

/** A request of IndexedDB as a promise. */
const done = (request) => new Promise((resolve, reject) => {
  request.onsuccess = () => resolve(request.result);
  request.onerror = () => reject(request.error);
});
const finished = (tx) => new Promise((resolve, reject) => {
  tx.oncomplete = () => resolve();
  tx.onerror = tx.onabort = () => reject(tx.error ?? new Error("aborted"));
});

/**
 * The site's store of results in IndexedDB, emptied when it was written with another `format`; null where the browser keeps
 * nothing (a private window that refuses it, storage turned off, no IndexedDB). The database is `name` after the page's
 * storage prefix, so that a frozen build beside the current one keeps its own.
 */
export async function openStore(format, name = "results") {
  name = STORAGE_PREFIX + name;
  try {
    if (typeof indexedDB === "undefined") return null;
    const opening = indexedDB.open(name, 1);
    opening.onupgradeneeded = () => {
      const db = opening.result;
      db.createObjectStore("entries", { keyPath: "key" }).createIndex("used", "used");
      db.createObjectStore("meta");
    };
    const db = await done(opening);
    const stamp = JSON.stringify(format);
    const tx = db.transaction(["entries", "meta"], "readwrite");
    const meta = tx.objectStore("meta");
    if ((await done(meta.get("format"))) !== stamp) {
      tx.objectStore("entries").clear();
      meta.put(stamp, "format");
    }
    await finished(tx);
    return new Store(db);
  } catch {
    return null;
  }
}

/** When an entry was used: the time, made later than the last one given, so that two uses in one millisecond keep their order. */
let lastUse = 0;
const now = () => (lastUse = Math.max(Date.now(), lastUse + 1));

class Store {
  constructor(db) { this.db = db; }

  /** The entry of `key`, marked as used now; null when there is none. */
  async get(key) {
    const tx = this.db.transaction("entries", "readwrite");
    const entries = tx.objectStore("entries");
    const entry = await done(entries.get(key));
    if (entry) entries.put({ ...entry, used: now() });
    await finished(tx);
    return entry ?? null;
  }

  async put(entry) {
    const tx = this.db.transaction("entries", "readwrite");
    tx.objectStore("entries").put({ ...entry, used: now() });
    await finished(tx);
  }

  async delete(key) {
    const tx = this.db.transaction("entries", "readwrite");
    tx.objectStore("entries").delete(key);
    await finished(tx);
  }

  /** Remove the least recently used entries until those left take no more than `limit` bytes; the bytes and the count left. */
  async trim(limit) {
    const tx = this.db.transaction("entries", "readwrite");
    const all = [];
    await new Promise((resolve, reject) => {
      const cursor = tx.objectStore("entries").index("used").openCursor();
      cursor.onerror = () => reject(cursor.error);
      cursor.onsuccess = () => {
        const at = cursor.result;
        if (!at) return resolve();
        all.push([at.value.key, at.value.size]);
        at.continue();
      };
    });
    let used = all.reduce((sum, [, size]) => sum + size, 0);
    let count = all.length;
    for (const [key, size] of all) {
      if (used <= limit) break;
      tx.objectStore("entries").delete(key);
      used -= size;
      count--;
    }
    await finished(tx);
    return { used, count };
  }

  async clear() {
    const tx = this.db.transaction("entries", "readwrite");
    tx.objectStore("entries").clear();
    await finished(tx);
  }
}

/**
 * The results kept for a page: in memory, within `memoryLimit` bytes (their JSON text, counted at two bytes a character),
 * and in `store` (openStore), within `storeLimit` bytes (compressed), if there is one.
 */
export class ResultCache {
  constructor({ store = null, memoryLimit = MEMORY_LIMIT, storeLimit = STORE_LIMIT } = {}) {
    this.store = store;
    this.memory = new Map(); // key → { value, inputs, size }, the least recently used first
    this.memoryUsed = 0;
    this.memoryLimit = memoryLimit;
    this.storeLimit = storeLimit;
    this.storeUsed = null; // not known until the store is trimmed once
    this.hits = 0;
    this.misses = 0;
  }

  /** The value kept under `key` when `valid(inputs)` says that what it was made from still holds; null otherwise. */
  async get(key, valid = async () => true) {
    let entry = this.memory.get(key);
    let fromStore = false;
    if (!entry && this.store) {
      try {
        const kept = await this.store.get(key);
        if (kept) {
          const text = await unpack(kept);
          entry = { value: JSON.parse(text), inputs: kept.inputs, size: text.length * 2 };
          fromStore = true;
        }
      } catch { entry = undefined; }
    }
    if (!entry || !(await valid(entry.inputs))) {
      if (entry) await this.delete(key);
      this.misses++;
      return null;
    }
    this.hits++;
    if (this.fits(entry.size)) {
      this.remember(key, entry);
      if (fromStore) this.trimMemory();
    }
    return entry.value;
  }

  /**
   * The value kept under `key` when `valid(inputs)` says that what it was made from still holds, as `get` gives it, but
   * looked at only: it counts as no hit or miss, is not made the most recently used, and is not dropped when it does not
   * hold. Null otherwise.
   */
  async peek(key, valid = async () => true) {
    let entry = this.memory.get(key);
    if (!entry && this.store) {
      try {
        const kept = await this.store.get(key);
        if (kept) entry = { value: JSON.parse(await unpack(kept)), inputs: kept.inputs };
      } catch { entry = undefined; }
    }
    return entry && (await valid(entry.inputs)) ? entry.value : null;
  }

  /** Keep `value`, made from `inputs`, under `key`, in memory and in the store. */
  async put(key, value, inputs = []) {
    const text = JSON.stringify(value);
    const size = text.length * 2;
    if (this.fits(size)) {
      this.remember(key, { value, inputs, size });
      this.trimMemory();
    } else this.forget(key);
    if (!this.store || this.storeLimit <= 0) return;
    try {
      const packed = await pack(text);
      if (packed.bytes.length * LARGEST_SHARE > this.storeLimit) return;
      await this.store.put({ key, inputs, ...packed, size: packed.bytes.length });
      ({ used: this.storeUsed } = await this.store.trim(this.storeLimit));
    } catch { /* not kept: only slower */ }
  }

  async delete(key) {
    this.forget(key);
    try { await this.store?.delete(key); } catch { /* gone or not, the entry is not given again */ }
  }

  /** New limits, in bytes; what no longer fits goes, the least recently used first. */
  async setLimits({ memoryLimit = this.memoryLimit, storeLimit = this.storeLimit } = {}) {
    this.memoryLimit = memoryLimit;
    this.storeLimit = storeLimit;
    this.trimMemory();
    try { if (this.store) ({ used: this.storeUsed } = await this.store.trim(storeLimit)); } catch { /* trimmed later */ }
  }

  /** Empty the memory and the store, and count again from 0. */
  async clear() {
    this.memory.clear();
    this.memoryUsed = 0;
    this.hits = this.misses = 0;
    try { await this.store?.clear(); this.storeUsed = this.store ? 0 : null; } catch { /* the store is not the browser's to empty */ }
  }

  /** What is kept, for File › Memory Cache: the entries and bytes in memory, the bytes in the store (null until known), hits and misses. */
  stats() {
    return { entries: this.memory.size, used: this.memoryUsed, limit: this.memoryLimit, stored: this.storeUsed,
      storeLimit: this.storeLimit, persistent: !!this.store, hits: this.hits, misses: this.misses };
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
