import { m } from "./i18n.js";
// Projects: the files of the Library in one ZIP archive with their manifest, rnl-project.json
// (manifest.js), which says the entry, the pool in its order, the formats chosen, the level shown,
// the desktop's PATH and the Library's settings. Download Project (web) and Save Project (desktop)
// write one; Open Project reads one back: its files go in the Library, and its entry opens as it was.
import { inform, modal } from "./dialog.js";
import { changes, fileText, hasBuffer } from "./edits.js";
import { confirmLeave, note } from "./editor.js";
import { chosenFormat } from "./formats.js";
import { showLevel } from "./manifest-bar.js";
import { libraryFiles } from "./library-files.js";
import { checkManifest, loadManifest, PROJECT_MANIFEST, writeManifest } from "./manifest.js";
import { currentSource, isLoading, openSource } from "./open.js";
import { setPane, setSidebar } from "./panels.js";
import { isLibraryChanging, librarySnapshot, pathGroups, setProjectFiles } from "./pool.js";
import { shell } from "./shell.js";
import { sourcePath } from "./source-path.js";
import { current, doc } from "./state.js";
import { load } from "./util.js";
import { archiveNames, libraryZip, readZip } from "./zip.js";

const encoder = new TextEncoder();
let busy = false;

const canonical = (path) => path.replace(/^\/?<pool>(?=\/)/, "");
const baseName = (path) => path.replace(/\/+$/, "").split("/").pop();
const stem = (path) => baseName(path).replace(/\.[^.]*$/, "") || "project";

async function bytesOf(value) {
  if (value == null) throw new Error("The file could not be read.");
  if (typeof value === "string") return encoder.encode(value);
  if (value instanceof Blob) return new Uint8Array(await value.arrayBuffer());
  return value instanceof Uint8Array ? value : new Uint8Array(value);
}

/** The level shown, as a manifest names it: a subcircuit's name, or "(top)". */
function levelName() {
  const level = doc?.levels[current];
  return level?.kind === "subckt" ? level.title : "(top)";
}

/** The path of the file the document shown was read from, as the Library names it. */
function entryPath(source) {
  if (doc?.files[0]) {
    if (source?.temporary) return `<temporary>/${source.name || "clipboard.sp"}`;
    return canonical(sourcePath(doc, doc.files[0]));
  }
  return canonical(source.path || `/${source.name}`);
}

/**
 * The names the Library's files have in the archive, relative: the web's are rooted in its workspace
 * already; the desktop's lose the directory all of them are in, so that includes between them, `..`
 * among them, resolve where the project is opened. A file on another drive keeps its drive's name.
 */
function relativeNames(paths) {
  if (shell.platform !== "desktop") return paths;
  const same = (a, b) => (shell.formatKey ? shell.formatKey(a) === shell.formatKey(b) : a === b);
  let common = null;
  for (const path of paths) {
    if (path.startsWith("<")) continue;
    const dir = path.split("/").slice(0, -1);
    if (common === null) { common = dir; continue; }
    let k = 0;
    while (k < common.length && k < dir.length && same(common[k], dir[k])) k++;
    common = common.slice(0, k);
  }
  const depth = common?.length ?? 0;
  return paths.map((path) => (path.startsWith("<") ? path : path.split("/").slice(depth).join("/")));
}

/**
 * What a project holds: its files, by name in the archive, with how to read each, and its manifest's
 * fields. The Library's files, the document's own and those of an external manifest shown come first,
 * then the desktop's PATH directories, each under PATH/<n>-<name>/.
 */
function projectContents() {
  const source = currentSource();
  if (!source) throw new Error(m("Open a netlist first."));
  const snapshot = librarySnapshot();
  const items = snapshot.items.filter((item) => !item.builtin);
  const edited = (path) => (hasBuffer(path) ? fileText(path) : null);
  const files = libraryFiles({ items, temporary: snapshot.temporary }, source, doc, edited, changes(false), shell.readFile)
    .filter((file) => !file.path.startsWith("<bundled>"));
  const external = source.external;
  if (external) {
    for (const [path, text] of Object.entries(external.files)) {
      if (!files.some((file) => file.path === path)) files.push({ path, read: () => text });
    }
  }
  const entry = entryPath(source);
  const names = relativeNames(files.map((file) => file.path));
  const byPath = new Map(files.map((file, k) => [file.path, k]));
  const pool = [...(external ? source.pool : []), ...items.map((item) => canonical(item.path))]
    .filter((path, k, all) => byPath.has(path) && all.indexOf(path) === k);
  const groups = pathGroups();
  const paths = [];
  // a file's address, when it has one, says where it was from
  const entries = files.map((file, k) => ({ name: names[k], path: file.path, read: file.read, url: addressOf(file.path, items, source) ?? undefined }));
  const searched = pool.map((path) => byPath.get(path));
  groups.forEach((group, n) => {
    const dir = `PATH/${n + 1}-${baseName(group.dir) || "path"}`;
    const prefix = group.dir.replace(/\\/g, "/").replace(/\/?$/, "/");
    paths.push({ dir, subfolders: group.subfolders });
    for (const path of group.files) {
      const rest = path.startsWith(prefix) ? path.slice(prefix.length) : baseName(path);
      searched.push(entries.length);
      entries.push({ name: `${dir}/${rest}`, path, read: () => shell.readFile(path) });
    }
  });
  const final = archiveNames([PROJECT_MANIFEST, ...entries.map((e) => e.name)]).slice(1);
  entries.forEach((e, k) => { e.name = final[k]; });
  const formats = {};
  for (const e of entries) {
    const name = chosenFormat(e.path, e.url);
    if (name) formats[e.name] = name;
  }
  return {
    entries,
    entry: entries[byPath.get(entry)]?.name ?? entries[0]?.name,
    pool: searched.map((k) => entries[k].name),
    formats,
    level: levelName(),
    paths: paths.map(({ dir, subfolders }) => ({ path: dir, subfolders })),
    settings: { autoInclude: load("auto-include", true) === true },
    name: stem(entry),
  };
}

/** The archive of the project shown: rnl-project.json first, then its files. */
async function projectZip(contents) {
  const files = [];
  for (const e of contents.entries) {
    try { files.push({ path: e.name, bytes: await bytesOf(await e.read()), url: e.url }); }
    catch (error) { throw new Error(`${e.path}: ${error.message || error}`); }
  }
  const manifest = await writeManifest({ archive: true, entry: contents.entry, files, pool: contents.pool, formats: contents.formats,
    level: contents.level, paths: contents.paths, settings: contents.settings });
  const json = `${JSON.stringify(manifest, null, 2)}\n`;
  return libraryZip([{ path: PROJECT_MANIFEST, read: () => json }, ...files.map((f) => ({ path: f.path, read: () => f.bytes }))]);
}

/** File > Download Project (web) or Save Project (desktop). */
export async function saveProject() {
  if (!shell.saveProject || busy || modal() || isLibraryChanging()) return;
  const title = shell.platform === "web" ? m("Download Project") : m("Save Project");
  busy = true;
  let preparing = false;
  try {
    const contents = projectContents();
    const saved = await shell.saveProject(`${contents.name}.zip`, async () => {
      preparing = true;
      note(m("Preparing project ZIP…"), 0);
      return projectZip(contents);
    });
    if (saved) note(shell.platform === "web" ? m`Downloaded project (${contents.entries.length} files).` : m`Saved project ${saved}.`);
  } catch (error) {
    if (preparing) note("");
    await inform({ title, paragraphs: [m`Could not save the project: ${error.message || error}`] });
  } finally {
    busy = false;
  }
}

/**
 * File > Open Project: read a project's archive, put its files where the shell keeps them (the web's
 * page, a folder the desktop asks for), make them the Library's files, with the formats and settings
 * it gives, and open its entry at its level.
 */
export async function openProject() {
  if (!shell.pickProject || busy || modal() || isLibraryChanging() || isLoading()) return;
  busy = true;
  try {
    const picked = await shell.pickProject();
    if (!picked) return;
    const archive = readZip(picked.bytes);
    const listed = archive.get(PROJECT_MANIFEST);
    if (!listed) throw new Error(m`${picked.name} is not a project: it has no ${PROJECT_MANIFEST}.`);
    let json;
    try { json = JSON.parse(new TextDecoder().decode(await listed.read())); }
    catch { throw new Error(m`${PROJECT_MANIFEST} is not JSON.`); }
    checkManifest(json, { archive: true });
    if (!(await confirmLeave(m("open a project")))) return;
    const root = await shell.projectRoot(picked, json, archive);
    if (!root) return;
    const read = await loadManifest(json, { root, readFile: async (path) => (archive.has(path) ? archive.get(path).read() : null) });
    const { source, items } = await shell.projectSource(read, picked);
    if (!(await setProjectFiles(items, read.formats, read.settings))) return;
    setSidebar(true);
    setPane("files");
    if (await openSource(source, false, undefined, false, null, null, true)) await showLevel(read.level);
  } catch (error) {
    await inform({ title: m("Open Project"), paragraphs: [m`Could not open the project: ${error.message || error}`] });
  } finally {
    busy = false;
  }
}

/**
 * The address of the Library's file at `path`, or null when it has none: a public package's, a file's
 * of a URL and those it includes (the source's `address`), or an external manifest's.
 */
function addressOf(path, items, source) {
  for (const item of items) {
    if (canonical(item.path) !== path) continue;
    if (item.url) return item.url;
    const address = item.source?.address?.(path);
    if (address) return address;
  }
  if (source?.external) return source.external.urls?.[path] ?? null;
  return source?.address?.(path) ?? null;
}

/**
 * File > Download Manifest (web): the manifest of the Library's files by their addresses alone, with
 * how they are set up, which Load Manifest or `?manifest=` opens as it is shown. Each file is fetched
 * again, for its SHA-256 and to see that it is still what the page holds. A file without an address,
 * or with changes, cannot be in it: Download Project keeps those.
 */
export async function downloadManifest() {
  if (!shell.saveManifest || busy || modal() || isLibraryChanging()) return;
  busy = true;
  const title = m("Download Manifest");
  try {
    const contents = projectContents();
    const source = currentSource();
    const items = librarySnapshot().items.filter((item) => !item.builtin);
    if (Object.keys(changes(false)).length) throw new Error(m("The library has changes that are not saved. Download Project keeps them."));
    const local = contents.entries.filter((e) => !addressOf(e.path, items, source));
    if (local.length) {
      throw new Error(m`${local.slice(0, 3).map((e) => e.name).join(", ")}${local.length > 3 ? "…" : ""} has no address on the web. Download Project keeps such files.`);
    }
    note(m("Fetching the library's files again…"), 0);
    const files = [];
    for (const e of contents.entries) {
      const url = addressOf(e.path, items, source);
      const response = await fetch(url, { cache: "no-store" });
      if (!response.ok) throw new Error(m`${url} could not be fetched: ${response.status}.`);
      const bytes = new Uint8Array(await response.arrayBuffer());
      const held = await bytesOf(await e.read());
      if (!sameText(bytes, held)) {
        throw new Error(m`${e.name} is not what ${url} holds now. Download Project keeps the file as it is shown.`);
      }
      files.push({ path: e.name, bytes, url });
    }
    const manifest = await writeManifest({ entry: contents.entry, files, pool: contents.pool, formats: contents.formats, level: contents.level,
      settings: contents.settings, id: contents.name });
    await shell.saveManifest(`${contents.name}.json`, `${JSON.stringify(manifest, null, 2)}\n`);
    note(m`Downloaded the manifest of ${files.length} files.`);
  } catch (error) {
    note("");
    await inform({ title, paragraphs: [m`Could not download the manifest: ${error.message || error}`] });
  } finally {
    busy = false;
  }
}

/** Whether two files hold the same text, as the core reads them, whatever their bytes. */
function sameText(a, b) {
  const text = (bytes) => {
    try { return new TextDecoder("utf-8", { fatal: true }).decode(bytes).replace(/^\uFEFF/, ""); }
    catch { return [...bytes].map((b) => String.fromCharCode(b)).join(""); }
  };
  return text(a) === text(b);
}

/** File > Load Manifest (web): a manifest file the user chooses, opened as `?manifest=` opens one. */
export async function openManifest() {
  if (!shell.pickManifest || busy || modal() || isLibraryChanging() || isLoading()) return;
  busy = true;
  try {
    const picked = await shell.pickManifest();
    if (!picked) return;
    let json;
    try { json = JSON.parse(picked.text); }
    catch { throw new Error(m`${picked.name} is not JSON.`); }
    checkManifest(json);
    await openSource(shell.sourceFromManifestJson(json, picked.name), false);
  } catch (error) {
    await inform({ title: m("Load Manifest"), paragraphs: [m`Could not load the manifest: ${error.message || error}`] });
  } finally {
    busy = false;
  }
}
