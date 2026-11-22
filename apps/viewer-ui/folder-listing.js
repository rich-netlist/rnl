// A folder included, listed as every viewer lists one: the files the core lists (`rnl_cli::walk::folder_files` with
// `INCLUDE_FOLDER`), which the desktop app and the local program list on disk, and the web shell among the files a
// browser gives it for a folder chosen or dropped. Each folder below the one included is entered down to `depth`
// levels, nothing hidden (a name that starts with a dot) is taken, and no more than `limit` files are: those met first,
// the files of a folder by name before the folders in it, each by name. What is listed is sorted by path.

/** How a folder included is listed: rnl_cli::walk::INCLUDE_FOLDER, which a test holds this to. */
export const INCLUDE_FOLDER = Object.freeze({ depth: 8, limit: 2000 });

/** `a` before `b` by the names of their parts, one by one; a shorter path first where they agree. */
function byParts(a, b) {
  for (let i = 0; i < Math.min(a.length, b.length); i++) {
    if (a[i] !== b[i]) return a[i] < b[i] ? -1 : 1;
  }
  return a.length - b.length;
}

/** `a` before `b` as the walk meets them: in the folder where they part, a file there before a folder, else by name. */
function byWalk(a, b) {
  for (let i = 0; i < Math.min(a.length, b.length); i++) {
    if (a[i] === b[i]) continue;
    const aFile = i === a.length - 1;
    const bFile = i === b.length - 1;
    if (aFile !== bFile) return aFile ? -1 : 1;
    return a[i] < b[i] ? -1 : 1;
  }
  return a.length - b.length;
}

/**
 * The files of the folders included among `items`, each listed as the core lists a folder, and the files given alone
 * that `listed` takes; and whether a folder had more files than are listed. `pathOf` gives an item's path, `/`-separated,
 * whose first part is the folder included, or the file itself when it has no other: "lib/sub/cell.inc", "top.sp" (a
 * leading `/` is not a part). `listed` says whether the core lists a file of that path, by its suffix.
 */
export function folderListing(items, pathOf, listed, walk = INCLUDE_FOLDER) {
  const alone = [];
  const folders = new Map();
  for (const item of items) {
    const parts = pathOf(item).split("/").filter((part) => part !== "");
    if (parts.length === 0 || !listed(parts.join("/"))) continue;
    if (parts.length === 1) {
      alone.push({ item, parts });
      continue;
    }
    // the folder included is the first part; the levels below it, those of the parts between it and the file's name
    const below = parts.slice(1);
    if (below.length - 1 > walk.depth || below.some((part) => part.startsWith("."))) continue;
    if (!folders.has(parts[0])) folders.set(parts[0], []);
    folders.get(parts[0]).push({ item, parts, below });
  }
  let truncated = false;
  const kept = [...alone];
  for (const files of folders.values()) {
    files.sort((a, b) => byWalk(a.below, b.below));
    if (files.length > walk.limit) truncated = true;
    kept.push(...files.slice(0, walk.limit));
  }
  kept.sort((a, b) => byParts(a.parts, b.parts));
  return { items: kept.map(({ item }) => item), truncated };
}
