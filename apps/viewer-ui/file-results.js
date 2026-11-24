// The rules of the readings kept (file-reading.js): the key each is kept under, and when one kept still holds. The
// cache itself is result-cache.js, which knows nothing of what it keeps. Every shell keeps its readings by these rules:
// they are the page's, not the shell's.
//
// A reading of one file (a FileResult) is found by its key: the core's version and the shape of its results, the
// options that change it, the entry, the file and the hash of its text. It is given again only while:
//   - the hash of every file and package it records it read (`inputs.files`) is the same, a bundled package aside,
//     which only another core changes, and the key has the core;
//   - the outline of the document has the same shape (`outlineShape`): the levels, their numbers and where they are;
//   - for a reading that depends on the Library (`inputs.library`), the Library is the same (`libraryStamps`).
// The outline of a document is kept too, by its entry and the options, while the hash of each file it read is the same,
// the files it found missing are still missing and, when it names a subcircuit no file defines, the Library is the same.
// The entry of a document that joined the Library by being opened is not looked at for its own readings and outline
// (`libraryFor`): its text is among what they read. A real change of the Library empties the cache (`libraryChanged`).
// Nothing else is looked at: the netlist is not read.

import { cacheKey } from "./result-cache.js";

/** The hash the core gives a text (`OutlineFile.text_hash`): the first 16 hexadecimal digits of the SHA-256 of its UTF-8 bytes. */
export async function textHash(text) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
  return [...new Uint8Array(digest).slice(0, 8)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/** The key of the outline of the document `entry`, as the shell reads it with `options`, with the core `core`. */
export function outlineKey(core, entry, options) {
  return cacheKey({ kind: "outline", core: core.version, format: core.format, entry, options });
}

/** The key of the reading of `file` of the document `entry`, whose text has the hash `hash`, as the shell reads it with `options`. */
export function readingKey(core, entry, options, file, hash) {
  return cacheKey({ kind: "reading", core: core.version, format: core.format, entry, file, hash, options });
}

/** The file `name`, as the document names it (relative to the entry's folder, or a path of its own), as the files of a web request name it. */
export function requestPath(entry, name) {
  // (a file of the Library, which the document names under <pool>/, is under /<pool>/ in the files of the request)
  if (name.startsWith("<pool>/")) return `/${name}`;
  if (/^(?:[A-Za-z]:\/|\/|<)/.test(name)) return name;
  return `${entry.slice(0, entry.lastIndexOf("/") + 1)}${name}`;
}

/**
 * The shape of an outline, as a text: what a reading of one file takes from the outline of the whole document. The
 * user's rule (decided): a change of the structure, the levels and their numbers, makes every reading of the document
 * made again; a change of values, positions or RNL records alone keeps them. So the shape is the files in their order
 * with what each includes, the levels in their order (their kind, name, file, parent, ports and the levels their
 * instances call), the roots and the global nets; lines, counts of instances, the language versions and packages of
 * the files (RNL records: a reading has the files above it among its inputs), diagnostics and hashes are not in it.
 * To make the rule stricter or looser, change what this gives.
 */
export function outlineShape(outline) {
  return JSON.stringify([
    outline.dialect,
    outline.files.map((f) => [f.path, f.format, f.default_format, f.includes.map((i) => i.file), f.levels]),
    outline.levels.map((l) => [l.kind, l.title, l.file, l.parent, l.circuit, l.ports, l.calls.map((c) => [c.master, c.level])]),
    outline.roots,
    outline.globals,
  ]);
}

/**
 * The stamps of the Library: `user`, of the files the user put there (Include, PATH, a manifest), and `all`, with those
 * that joined it by being opened. Each is made of the path and the hash of the text of each file; `items` are the
 * Library's files as pool.js gives them to the shell (`opened` for one that joined by being opened), `hashOf(path)` the
 * hash of one's text, or null when it cannot be read.
 */
export async function libraryStamps(items, hashOf) {
  const listed = await Promise.all(items.filter((i) => !i.builtin).map(async (i) => [i.path, await hashOf(i.path), !!i.opened]));
  listed.sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
  return {
    user: await textHash(JSON.stringify(listed.filter(([, , opened]) => !opened).map(([p, h]) => [p, h]))),
    all: await textHash(JSON.stringify(listed.map(([p, h]) => [p, h]))),
    listed,
  };
}

// the stamps of the Library for the readings of each entry, made once for each stamps
const forEntry = new WeakMap();

/**
 * The stamps of the Library `stamps` (`libraryStamps`) for the readings and the outline of the document `entry`: `all`
 * without the entry itself when it joined the Library by being opened, as opening it makes it join after its first
 * reading. Its text is among what they read, and they hold or not by it already.
 */
export function libraryFor(stamps, entry) {
  const at = (path) => path.replaceAll("\\", "/");
  const own = stamps.listed?.find(([path, , opened]) => opened && at(path) === at(entry));
  if (!own) return Promise.resolve(stamps);
  if (!forEntry.has(stamps)) forEntry.set(stamps, new Map());
  const made = forEntry.get(stamps);
  if (!made.has(entry)) {
    made.set(entry, textHash(JSON.stringify(stamps.listed.filter((l) => l !== own).map(([p, h]) => [p, h])))
      .then((all) => ({ ...stamps, all })));
  }
  return made.get(entry);
}

/**
 * Whether the Library really changed, from the stamps before (null at the start) to those now, which empties the cache
 * (decided): a scan at the start, or one again, that finds the same files with the same texts changes nothing, nor
 * does a file that joins it by being opened; a reading that depends on the Library looks at `all` itself.
 */
export function libraryChanged(before, now) {
  return before !== null && before.user !== now.user;
}

/** Whether the outline names a subcircuit no file of the document defines, which a file of the Library may come to. */
export const outlineUsesLibrary = (outline) => outline.levels.some((l) => l.calls.some((c) => c.level === null));

/**
 * Whether a kept reading `kept` (`{ result, shape, library }`) holds now: `hashes` the hash of each file it read now
 * (by its path as `inputs.files` names it; null when it is not there), `shape` the shape of the outline now, `library`
 * the Library's stamps now.
 */
export function readingHolds(kept, hashes, shape, library) {
  for (const input of kept.result.inputs.files) {
    if (!input.bundled && hashes.get(input.path) !== input.hash) return false;
  }
  if (kept.shape !== shape) return false;
  return !kept.result.inputs.library || kept.library === library.all;
}

/** The paths whose hashes `readingHolds` compares: those of the files and packages the reading read, a bundled one aside. */
export const readingPaths = (result) => result.inputs.files.filter((i) => !i.bundled).map((i) => i.path);

/**
 * Whether a kept outline `kept` (`{ outline, library }`) holds now: `hashes` the hash of each file it read and of each
 * it found missing, by their paths (null for one not there), `library` the Library's stamps now.
 */
export function outlineHolds(kept, hashes, library) {
  if (kept.outline.files.some((f) => hashes.get(f.path) !== f.text_hash)) return false;
  if (kept.outline.missing.some((p) => hashes.get(p) !== null)) return false;
  return !outlineUsesLibrary(kept.outline) || kept.library === library.all;
}

/** The paths whose hashes `outlineHolds` compares. */
export const outlinePaths = (outline) => [...outline.files.map((f) => f.path), ...outline.missing];
