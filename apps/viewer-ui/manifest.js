// A copy of bindings/runtime-wasm/js/manifest.js: do not change it here; change that file and copy it here.
// A manifest (format 1) read into what a build of the core takes: the texts of its files by workspace
// path, its pool in the manifest's order, the format chosen for some files, its entry and its level.
// One format serves three uses, and this one module reads and writes it for the web viewer, the
// desktop viewer and tools in Node, such as the RNL gallery's thumbnails, so that all of them draw
// the same Document:
//
// - an external manifest (`?manifest=`), such as the gallery writes: its files under `base`, at a
//   commit of a public repository;
// - a project, `rnl-project.json` in a ZIP archive with its files (Download or Save Project);
// - a manifest of links (Download Manifest): each file at an address of its own, `url`.
//
// Each file is taken from its `url`, or from the archive the manifest is in, or from `base` joined
// with its path, and its bytes are checked against their SHA-256 and size before they are decoded.
// In an archive the archive's copy wins over the `url`, which then only says where the file was from.
//
// The module imports nothing, so that the desktop viewer, which has no WebAssembly core, can use it:
// apps/viewer-ui/manifest.js is a copy of it, which a test keeps the same.

/** Text as `rnl` reads a file: UTF-8, without a byte order mark, or else Latin-1 (client.js decodeText). */
function decodeText(bytes) {
  try {
    const text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
    return text.charCodeAt(0) === 0xfeff ? text.slice(1) : text;
  } catch {
    let text = "";
    for (let k = 0; k < bytes.length; k += 0x8000) text += String.fromCharCode(...bytes.subarray(k, k + 0x8000));
    return text;
  }
}

/** The manifest format this module reads and writes. */
export const MANIFEST_FORMAT = 1;
/** The name of a project's manifest in its archive. */
export const PROJECT_MANIFEST = "rnl-project.json";
/**
 * At most this many files, this many bytes in one file and this many in all, as the gallery's CI
 * allows, for the files a manifest fetches; a project's archive holds its files, which may be a
 * model library's: at most `projectFiles` files and `projectBytes` bytes.
 */
export const MANIFEST_LIMITS = Object.freeze({
  files: 300, fileBytes: 2 * 1024 * 1024, bytes: 20 * 1024 * 1024, projectFiles: 5000, projectBytes: 1024 * 1024 * 1024,
});
const ID = /^[A-Za-z0-9_.-]{1,64}$/;
const SHA256 = /^[0-9a-f]{64}$/;
const COLOUR = /^#[0-9a-f]{6}$/i;

/**
 * The display settings a manifest proposes, as a viewer offers them to apply (`display`): the view
 * of its level, `{ fit: true }` or `{ center: [x, y], scale }` in the drawing's units and pixels per
 * unit; the `background`, "light" or "dark"; `colors`, by the viewer's names, each "#RRGGBB", in
 * either case, as the manifest writes it;
 * whether `usages` and `currents` are shown; the `grid`, "none", "dots" or "lines". One of these of
 * another shape is left out of `display`; one this module does not know is kept in it as it is, for
 * a newer viewer and so that writing the manifest again loses nothing, and no viewer applies it.
 * `ignored` names both, what this viewer will not apply, so that it can say so and a manifest
 * written for a newer viewer still opens.
 */
function displaySettings(display) {
  const out = {}, ignored = [];
  if (display === undefined) return { display: out, ignored };
  if (!display || typeof display !== "object" || Array.isArray(display)) return { display: out, ignored: ["display"] };
  const finite = (n) => typeof n === "number" && Number.isFinite(n);
  for (const [key, value] of Object.entries(display)) {
    let ok = false;
    if (key === "view") {
      if (value?.fit === true) { out.view = { fit: true }; ok = true; }
      else if (Array.isArray(value?.center) && value.center.length === 2 && value.center.every(finite) && finite(value.scale) && value.scale > 0) {
        out.view = { center: [value.center[0], value.center[1]], scale: value.scale };
        ok = true;
      }
    } else if (key === "background") {
      if (value === "light" || value === "dark") { out.background = value; ok = true; }
    } else if (key === "colors") {
      if (value && typeof value === "object" && !Array.isArray(value)) {
        const colors = Object.entries(value).filter(([, c]) => typeof c === "string" && COLOUR.test(c));
        for (const [name] of Object.entries(value)) if (!colors.some(([n]) => n === name)) ignored.push(`colors.${name}`);
        if (colors.length) out.colors = Object.fromEntries(colors);
        ok = true;
      }
    } else if (key === "usages" || key === "currents") {
      if (typeof value === "boolean") { out[key] = value; ok = true; }
    } else if (key === "grid") {
      if (["none", "dots", "lines"].includes(value)) { out.grid = value; ok = true; }
    } else {
      out[key] = value;
    }
    if (!ok) ignored.push(key);
  }
  return { display: out, ignored };
}

/** What is wrong with a manifest or one of its files; `file` names that file when there is one. */
export class ManifestError extends Error {
  constructor(message, file = null) {
    super(message);
    this.name = "ManifestError";
    this.file = file;
  }
}

/** A relative path inside the manifest's files: no `..`, no root, no empty part, no backslash. */
function relativePath(path, what) {
  if (typeof path !== "string" || !path || path.includes("\\") || path.startsWith("/") || /^[a-z][a-z0-9+.-]*:/i.test(path)) {
    throw new ManifestError(`${what} is not a relative path: ${JSON.stringify(path)}`, typeof path === "string" ? path : null);
  }
  if (path.split("/").some((part) => part === "" || part === "." || part === "..")) {
    throw new ManifestError(`${what} leaves the manifest's files or has an empty part: ${JSON.stringify(path)}`, path);
  }
  return path;
}

/**
 * An https address, or an http one on this computer, as a page in development has: every file is
 * checked against its SHA-256 and size, wherever it is fetched from.
 */
function address(url) {
  try {
    const parsed = new URL(url);
    const local = ["localhost", "127.0.0.1", "[::1]"].includes(parsed.hostname) || parsed.hostname.endsWith(".localhost");
    return parsed.protocol === "https:" || (parsed.protocol === "http:" && local);
  } catch {
    return false;
  }
}

/** The id a manifest without one gets: its entry's name, without the extension, in the id's letters. */
function defaultId(entry) {
  const name = String(entry ?? "").split("/").pop().replace(/\.[^.]*$/, "").replace(/[^A-Za-z0-9_.-]+/g, "-").replace(/^[.-]+|[.-]+$/g, "");
  return name.slice(0, 64) || "project";
}

/**
 * Check a manifest and give it in the form the rest of this module reads. `archive` says that it was
 * read from a project's archive, whose files are in the archive: they need no address then. `limits`
 * replaces some of MANIFEST_LIMITS, as a caller that holds less needs: a web page's WebAssembly core
 * holds a project's texts several times over, so it opens smaller projects than the desktop does.
 */
export function checkManifest(manifest, { archive = false, limits = {} } = {}) {
  const limit = { ...MANIFEST_LIMITS, ...limits };
  if (!manifest || typeof manifest !== "object" || Array.isArray(manifest)) throw new ManifestError("The manifest is not a JSON object.");
  if (typeof manifest.format !== "number") throw new ManifestError("The manifest has no format.");
  if (manifest.format > MANIFEST_FORMAT) {
    throw new ManifestError(`The manifest is of format ${manifest.format}, newer than this viewer reads (${MANIFEST_FORMAT}): update the viewer.`);
  }
  if (manifest.format !== MANIFEST_FORMAT) throw new ManifestError(`The manifest's format ${manifest.format} is not one this viewer reads.`);
  const { base, entry, level } = manifest;
  const id = manifest.id ?? defaultId(entry);
  if (typeof id !== "string" || !ID.test(id) || id === "." || id === "..") {
    throw new ManifestError(`The manifest's id must be 1 to 64 letters, digits, '-', '_' or '.': ${JSON.stringify(id)}`);
  }
  if (base !== undefined && (typeof base !== "string" || !address(base) || !base.endsWith("/"))) {
    throw new ManifestError(`The manifest's base must be the https address of a directory, ending in '/': ${JSON.stringify(base)}`);
  }
  if (!Array.isArray(manifest.files) || !manifest.files.length) throw new ManifestError("The manifest lists no files.");
  const most = archive ? limit.projectFiles : limit.files;
  if (manifest.files.length > most) throw new ManifestError(`The manifest lists ${manifest.files.length} files, more than ${most}.`);
  const files = manifest.files.map((file) => {
    const path = relativePath(file?.path, "A file's path");
    if (typeof file.sha256 !== "string" || !SHA256.test(file.sha256.toLowerCase())) throw new ManifestError(`${path} has no SHA-256.`, path);
    if (!Number.isSafeInteger(file.size) || file.size < 0) throw new ManifestError(`${path} has no size.`, path);
    if (!archive && file.size > limit.fileBytes) throw new ManifestError(`${path} has ${file.size} bytes, more than ${limit.fileBytes}.`, path);
    if (file.url !== undefined && (typeof file.url !== "string" || !address(file.url))) {
      throw new ManifestError(`${path} has a url that is not an https address: ${JSON.stringify(file.url)}`, path);
    }
    if (!archive && file.url === undefined && base === undefined) throw new ManifestError(`${path} has no url, and the manifest no base.`, path);
    return { path, sha256: file.sha256.toLowerCase(), size: file.size, ...(file.url !== undefined && { url: file.url }) };
  });
  const listed = new Set(files.map((file) => file.path));
  if (listed.size !== files.length) throw new ManifestError("The manifest lists a file twice.");
  const bytes = files.reduce((sum, file) => sum + file.size, 0);
  const total = archive ? limit.projectBytes : limit.bytes;
  if (bytes > total) throw new ManifestError(`The manifest's files hold ${bytes} bytes, more than ${total}.`);
  const entryPath = relativePath(entry, "The entry");
  if (!listed.has(entryPath)) throw new ManifestError(`The entry ${entryPath} is not among the manifest's files.`, entryPath);
  if (manifest.pool !== undefined && !Array.isArray(manifest.pool)) throw new ManifestError("The manifest's pool is not a list.");
  const pool = (manifest.pool ?? []).map((path) => {
    const file = relativePath(path, "A path of the pool");
    if (!listed.has(file)) throw new ManifestError(`The pool's ${file} is not among the manifest's files.`, file);
    return file;
  });
  if (new Set(pool).size !== pool.length) throw new ManifestError("The manifest's pool lists a file twice.");
  const formats = manifest.formats ?? {};
  if (!formats || typeof formats !== "object" || Array.isArray(formats)) throw new ManifestError("The manifest's formats are not an object.");
  for (const [path, name] of Object.entries(formats)) {
    if (!listed.has(relativePath(path, "A path of the formats"))) throw new ManifestError(`The format of ${path} is given, but it is not among the manifest's files.`, path);
    // the core knows the formats: a name it does not offer is for the caller to leave out (formats.js)
    if (typeof name !== "string" || !name) throw new ManifestError(`The format of ${path} is not a format's name: ${JSON.stringify(name)}`, path);
  }
  if (manifest.paths !== undefined && !Array.isArray(manifest.paths)) throw new ManifestError("The manifest's paths are not a list.");
  const paths = (manifest.paths ?? []).map((dir) => ({ path: relativePath(dir?.path, "A search path"), subfolders: dir.subfolders === true }));
  const settings = manifest.settings ?? {};
  if (!settings || typeof settings !== "object" || Array.isArray(settings)) throw new ManifestError("The manifest's settings are not an object.");
  if (settings.autoInclude !== undefined && typeof settings.autoInclude !== "boolean") throw new ManifestError("The manifest's autoInclude is not true or false.");
  if (level !== undefined && typeof level !== "string") throw new ManifestError("The manifest's level is not a name.");
  const { display, ignored } = displaySettings(manifest.display);
  return { ...manifest, id, base, entry: entryPath, level: level ?? null, files, pool, formats: { ...formats }, paths, settings: { ...settings },
    display, displayIgnored: ignored };
}

async function sha256Hex(bytes) {
  // browsers and Node 19+ have WebCrypto as a global; Node 18 has it as node:crypto's webcrypto
  const crypto = globalThis.crypto?.subtle ? globalThis.crypto : (await import("node:crypto")).webcrypto;
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/** `bytes`, checked against `sha256` and `size`; `name` is what an error names. */
async function checked(bytes, { sha256, size, name }) {
  if (bytes.length !== size) throw new ManifestError(`${name} has ${bytes.length} bytes, not the ${size} the manifest lists.`, name);
  if ((await sha256Hex(bytes)) !== sha256) throw new ManifestError(`${name} does not match the SHA-256 the manifest lists.`, name);
  return bytes;
}

/** The bytes at `url`; `name` is what an error names. */
async function fetched(fetch, url, { name, signal }) {
  let response;
  try {
    response = await fetch(url, { signal });
  } catch (error) {
    if (signal?.aborted) throw error;
    throw new ManifestError(`${name} could not be fetched: ${error.message || error}`, name);
  }
  if (!response.ok) throw new ManifestError(`${name} could not be fetched: ${response.status} ${response.statusText || ""}`.trim(), name);
  return new Uint8Array(await response.arrayBuffer());
}

/** Run `work` on each item, at most `limit` at a time, in the order given. */
async function inParallel(items, limit, work) {
  const out = new Array(items.length);
  let next = 0;
  const runner = async () => {
    while (next < items.length) {
      const at = next++;
      out[at] = await work(items[at], at);
    }
  };
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, runner));
  return out;
}

/**
 * Read the files of a manifest already parsed, `json`. Each file comes from `readFile(path)` when it
 * is given (the manifest is in a project's archive; it gives the bytes, or null for a file the archive
 * lacks), else from its `url`, else from `base` joined with its path, with `fetch`, the global one by
 * default. `onProgress(done, total)` follows the files read; `signal` stops it; `root` is the
 * workspace directory of the files, `/<id>/` by default; `limits`, as checkManifest takes them. The
 * result's paths are under `root`:
 * - `files`: the text of every file, by path, decoded after its bytes were checked; `bytes`: the bytes;
 *   `urls`: the address of each, its `url` or `base` joined with its path, or null for neither;
 * - `pool`: the manifest's pool, in its order, where a bare include and a package by name are
 *   looked for;
 * - `formats`: the format chosen for some files, `{ path: name }`, as a build takes them;
 * - `entry`: the path to open; `level`: the level to show, a subcircuit's name or "(top)", or null;
 * - `paths`: the search paths it records, `[{ path, subfolders }]`; `settings`;
 * - `display`: the display settings it proposes (displaySettings), which a viewer asks before it
 *   applies, and `displayIgnored`, the names of those it will not apply, of another shape (left out of
 *   `display`) or unknown (kept in it); and the manifest's `id`, `title`,
 *   `description`, `author`, `source` and `license` as it gives them.
 * Rejects with a ManifestError that names the file, when a file is the trouble.
 */
export async function loadManifest(json, { fetch = globalThis.fetch, readFile, onProgress, signal, concurrency = 6, root, limits } = {}) {
  const manifest = checkManifest(json, { archive: !!readFile, limits });
  root ??= `/${manifest.id}/`;
  const total = manifest.files.length;
  let done = 0;
  onProgress?.(done, total);
  const files = {}, bytes = {}, urls = {};
  // the first file that fails stops the reads still running, as the caller's signal does
  const stop = new AbortController();
  const stopAll = () => stop.abort(signal?.reason);
  signal?.addEventListener("abort", stopAll, { once: true });
  try {
    await inParallel(manifest.files, concurrency, async (file) => {
      if (stop.signal.aborted) return;
      let data;
      const url = file.url ?? (manifest.base === undefined ? null : new URL(file.path, manifest.base).href);
      urls[root + file.path] = url;
      if (readFile) {
        data = await readFile(file.path);
        if (data == null) throw new ManifestError(`${file.path} is not in the archive.`, file.path);
      } else {
        data = await fetched(fetch, url, { name: file.path, signal: stop.signal });
      }
      await checked(data, { ...file, name: file.path });
      bytes[root + file.path] = data;
      files[root + file.path] = decodeText(data);
      onProgress?.(++done, total);
    });
  } catch (error) {
    if (signal?.aborted) throw signal.reason ?? error;
    if (!stop.signal.aborted) stop.abort();
    throw error;
  } finally {
    signal?.removeEventListener("abort", stopAll);
  }
  return {
    id: manifest.id,
    title: manifest.title ?? null,
    description: manifest.description ?? null,
    author: manifest.author ?? null,
    source: manifest.source ?? null,
    license: manifest.license ?? null,
    entry: root + manifest.entry,
    level: manifest.level,
    files,
    bytes,
    urls,
    pool: manifest.pool.map((path) => root + path),
    formats: Object.fromEntries(Object.entries(manifest.formats).map(([path, name]) => [root + path, name])),
    paths: manifest.paths.map((dir) => ({ path: root + dir.path, subfolders: dir.subfolders })),
    settings: manifest.settings,
    display: manifest.display,
    displayIgnored: manifest.displayIgnored,
  };
}

/**
 * Read the manifest at `url` and its files, as loadManifest does (without `readFile`): a design of
 * the gallery, or a manifest of links.
 */
export async function readManifest(url, options = {}) {
  const { fetch = globalThis.fetch, signal } = options;
  let response;
  try {
    response = await fetch(url, { signal });
  } catch (error) {
    if (signal?.aborted) throw error;
    throw new ManifestError(`The manifest could not be fetched: ${error.message || error}`);
  }
  if (!response.ok) throw new ManifestError(`The manifest could not be fetched: ${response.status} ${response.statusText || ""}`.trim());
  let parsed;
  try {
    parsed = JSON.parse(await response.text());
  } catch {
    throw new ManifestError("The manifest is not JSON.");
  }
  return loadManifest(parsed, { ...options, readFile: undefined });
}

/**
 * Write the manifest of `files`, `[{ path, bytes, url? }]`, each path relative: their SHA-256 and
 * sizes, with `entry`, `pool`, `formats`, `level`, `paths`, `settings` and, for a design, `id`,
 * `title` and the rest as given. `archive` says the files go in the archive with it (a project);
 * else each needs a `url` or the manifest a `base` (a manifest of links). What it gives is checked
 * as a reader checks it, so a manifest this writes is one it reads.
 */
export async function writeManifest({ files, entry, pool = [], formats = {}, level, paths = [], settings = {}, display = {}, archive = false, ...rest }) {
  const listed = await Promise.all(files.map(async (file) => ({
    path: file.path, sha256: await sha256Hex(file.bytes), size: file.bytes.length, ...(file.url !== undefined && { url: file.url }),
  })));
  const manifest = {
    format: MANIFEST_FORMAT,
    ...Object.fromEntries(Object.entries(rest).filter(([, value]) => value !== undefined && value !== null)),
    entry,
    files: listed,
    pool,
    ...(Object.keys(formats).length && { formats }),
    ...(level !== undefined && level !== null && { level }),
    ...(paths.length && { paths }),
    ...(Object.keys(settings).length && { settings }),
    ...(Object.keys(display).length && { display }),
  };
  checkManifest(manifest, { archive });
  return manifest;
}
