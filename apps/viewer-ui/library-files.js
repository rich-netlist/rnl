// Capture file identities and editor text before awaiting a picker or reading any files.
import { sourcePath } from "./source-path.js";

const canonical = path => path.replace(/^\/?<pool>(?=\/)/, "");
const temporaryPath = source => `<temporary>/${source.name || "clipboard.sp"}`;

/** Lazy archive entries. Folder selections retain unreferenced files; loaded URL includes are local snapshots. */
export function libraryFiles({ items, temporary }, source, document, editedText, otherEdits = {}) {
  const files = new Map(), folders = new Set(), raw = new Set();
  const add = (path, read, original = false) => {
    path = canonical(path);
    files.set(path, { path, read });
    if (original) raw.add(path); else raw.delete(path);
  };
  for (const item of items) {
    const owner = item.source;
    if (owner.files && !folders.has(owner.files)) {
      folders.add(owner.files);
      for (const [path, read] of owner.files) add(path, read, true);
    }
  }
  // A root re-opened with newer bytes wins over old companion snapshots held by another root.
  for (const item of items) {
    const owner = item.source;
    if (owner.text !== undefined) {
      const text = owner.text;
      add(item.path, () => text);
    } else {
      const original = owner.files?.get(item.path);
      add(item.path, original || item.read, !!original);
    }
  }
  if (temporary) {
    const text = temporary.text;
    add(temporaryPath(temporary), () => text);
  }
  if (document) for (const [index, file] of document.files.entries()) {
    const path = index === 0 && source?.temporary ? temporaryPath(source) : sourcePath(document, file);
    const text = editedText(file.path) ?? file.text;
    // The displayed document may already have applied edits, and URL includes may be offline.
    // Other library files retain their original bytes, including encoding/BOM.
    if (!raw.has(canonical(path)) || text !== file.text || Object.hasOwn(otherEdits, file.path)) add(path, () => text);
  }
  // Apply can remove an include without discarding its unsaved buffer. Export its edits too,
  // provided that the file still belongs to this Library snapshot.
  const present = new Set(document?.files.map(file => file.path));
  if (document) for (const [path, text] of Object.entries(otherEdits)) {
    if (present.has(path)) continue;
    const full = sourcePath(document, { path });
    if (files.has(canonical(full))) add(full, () => text);
  }
  return [...files.values()];
}
