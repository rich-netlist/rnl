// Visit identity is independent of the indices and include closure of a compiled Document.
//
// A shell that reads the file shown alone (`shell.perFile`, per-file loading) shows a file of a document in the
// context of its entry: the source of such a view is the entry's source with `view`, the file as the document names
// it (`SourceFile.path`, relative to the entry's folder), and `entrySource`, the entry's own. The entry's own file is
// shown by the entry's source itself.
export function sourceKey(source) {
  const path = (source?.url || source?.path || source?.name || '').replaceAll('\\', '/');
  const key = /^[A-Za-z]:\//.test(path) ? path.toLowerCase() : path;
  return source?.view ? `${key}#${source.view}` : key;
}

export function sameSource(a, b) {
  return a === b || (!!a && !!b && sourceKey(a) === sourceKey(b));
}

/** The source of the entry a source reads: itself, or the entry of a view of one of its files. */
export const entrySource = (source) => source?.entrySource ?? source;

/** Whether two sources read the same entry, whichever of its files they show. */
export const sameEntry = (a, b) => sameSource(entrySource(a), entrySource(b));

/**
 * The view of the file `file` of the document `source` reads, in the context of its entry: the entry's source when
 * `file` is the entry's own file (`entryFile`) or null.
 */
export function fileView(source, file, entryFile) {
  const entry = entrySource(source);
  if (file === null || file === undefined || file === entryFile) return entry;
  return { ...entry, entrySource: entry, view: file };
}
