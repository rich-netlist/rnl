// A manifest (format 1) read into what a build of the core takes: the texts of its files by
// workspace path, its pool in the manifest's order, its entry and its level. The web viewer's
// `?manifest=` and tools in Node, such as the thumbnails of the RNL gallery, which writes such
// manifests, read a manifest with this one module, so that both draw the same Document.
//
// A manifest names a commit of a public GitHub repository: its files are fetched from `base`, raw
// GitHub, each checked against the SHA-256 of its bytes; a file with a `url` of its own, a package of
// the public catalog under `.packages/`, is fetched from raw GitHub or the catalog's site.
import { decodeText } from "./client.js";

/** The manifest format this module reads. */
export const MANIFEST_FORMAT = 1;
/** At most this many files, this many bytes in one file and this many in all. */
export const MANIFEST_LIMITS = Object.freeze({ files: 300, fileBytes: 2 * 1024 * 1024, bytes: 20 * 1024 * 1024 });
const FILES_BASE = "https://raw.githubusercontent.com/";
const URL_BASES = [FILES_BASE, "https://rich-netlist.github.io/rnl-packages/"];
const ID = /^[A-Za-z0-9_.-]{1,64}$/;
const SHA256 = /^[0-9a-f]{64}$/;

/** What is wrong with a manifest or one of its files; `file` names that file when there is one. */
export class ManifestError extends Error {
  constructor(message, file = null) {
    super(message);
    this.name = "ManifestError";
    this.file = file;
  }
}

/** A relative path inside the manifest's commit: no `..`, no root, no empty part, no backslash. */
function relativePath(path, what) {
  if (typeof path !== "string" || !path || path.includes("\\") || path.startsWith("/") || /^[a-z][a-z0-9+.-]*:/i.test(path)) {
    throw new ManifestError(`${what} is not a relative path: ${JSON.stringify(path)}`, typeof path === "string" ? path : null);
  }
  if (path.split("/").some((part) => part === "" || part === "." || part === "..")) {
    throw new ManifestError(`${what} leaves the commit or has an empty part: ${JSON.stringify(path)}`, path);
  }
  return path;
}

/** Check a manifest and give it in the form the rest of this module reads. */
export function checkManifest(manifest) {
  if (!manifest || typeof manifest !== "object") throw new ManifestError("The manifest is not a JSON object.");
  if (typeof manifest.format !== "number") throw new ManifestError("The manifest has no format.");
  if (manifest.format > MANIFEST_FORMAT) {
    throw new ManifestError(`The manifest is of format ${manifest.format}, newer than this viewer reads (${MANIFEST_FORMAT}): update the viewer.`);
  }
  if (manifest.format !== MANIFEST_FORMAT) throw new ManifestError(`The manifest's format ${manifest.format} is not one this viewer reads.`);
  const { id, base, entry, level } = manifest;
  if (typeof id !== "string" || !ID.test(id) || id === "." || id === "..") {
    throw new ManifestError(`The manifest's id must be 1 to 64 letters, digits, '-', '_' or '.': ${JSON.stringify(id)}`);
  }
  if (typeof base !== "string" || !base.startsWith(FILES_BASE) || !base.endsWith("/")) {
    throw new ManifestError(`The manifest's base must be a directory of ${FILES_BASE}: ${JSON.stringify(base)}`);
  }
  if (!Array.isArray(manifest.files) || !manifest.files.length) throw new ManifestError("The manifest lists no files.");
  if (manifest.files.length > MANIFEST_LIMITS.files) {
    throw new ManifestError(`The manifest lists ${manifest.files.length} files, more than ${MANIFEST_LIMITS.files}.`);
  }
  const files = manifest.files.map((file) => {
    const path = relativePath(file?.path, "A file's path");
    if (typeof file.sha256 !== "string" || !SHA256.test(file.sha256.toLowerCase())) throw new ManifestError(`${path} has no SHA-256.`, path);
    if (!Number.isSafeInteger(file.size) || file.size < 0) throw new ManifestError(`${path} has no size.`, path);
    if (file.size > MANIFEST_LIMITS.fileBytes) throw new ManifestError(`${path} has ${file.size} bytes, more than ${MANIFEST_LIMITS.fileBytes}.`, path);
    if (file.url !== undefined && (typeof file.url !== "string" || !URL_BASES.some((prefix) => file.url.startsWith(prefix)))) {
      throw new ManifestError(`${path} is not on raw GitHub or the public package site: ${JSON.stringify(file.url)}`, path);
    }
    return { path, sha256: file.sha256.toLowerCase(), size: file.size, ...(file.url !== undefined && { url: file.url }) };
  });
  const listed = new Set(files.map((file) => file.path));
  if (listed.size !== files.length) throw new ManifestError("The manifest lists a file twice.");
  const bytes = files.reduce((sum, file) => sum + file.size, 0);
  if (bytes > MANIFEST_LIMITS.bytes) throw new ManifestError(`The manifest's files hold ${bytes} bytes, more than ${MANIFEST_LIMITS.bytes}.`);
  const entryPath = relativePath(entry, "The entry");
  if (!listed.has(entryPath)) throw new ManifestError(`The entry ${entryPath} is not among the manifest's files.`, entryPath);
  if (manifest.pool !== undefined && !Array.isArray(manifest.pool)) throw new ManifestError("The manifest's pool is not a list.");
  const pool = (manifest.pool ?? []).map((path) => {
    const file = relativePath(path, "A path of the pool");
    if (!listed.has(file)) throw new ManifestError(`The pool's ${file} is not among the manifest's files.`, file);
    return file;
  });
  if (new Set(pool).size !== pool.length) throw new ManifestError("The manifest's pool lists a file twice.");
  if (level !== undefined && typeof level !== "string") throw new ManifestError("The manifest's level is not a name.");
  return { ...manifest, id, base, entry: entryPath, level: level ?? null, files, pool };
}

async function sha256Hex(bytes) {
  // browsers and Node 19+ have WebCrypto as a global; Node 18 has it as node:crypto's webcrypto
  const crypto = globalThis.crypto?.subtle ? globalThis.crypto : (await import("node:crypto")).webcrypto;
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/** The bytes at `url`, checked against `sha256` (and `size` when known); `name` is what an error names. */
async function fetchChecked(fetch, url, { sha256, size, name, signal }) {
  let response;
  try {
    response = await fetch(url, { signal });
  } catch (error) {
    if (signal?.aborted) throw error;
    throw new ManifestError(`${name} could not be fetched: ${error.message || error}`, name);
  }
  if (!response.ok) throw new ManifestError(`${name} could not be fetched: ${response.status} ${response.statusText || ""}`.trim(), name);
  const bytes = new Uint8Array(await response.arrayBuffer());
  if (size !== undefined && bytes.length !== size) throw new ManifestError(`${name} has ${bytes.length} bytes, not the ${size} the manifest lists.`, name);
  if ((await sha256Hex(bytes)) !== sha256) throw new ManifestError(`${name} does not match the SHA-256 the manifest lists.`, name);
  return bytes;
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
 * Read the manifest at `url` and its files. `fetch` defaults to the global one; `onProgress(done,
 * total)` follows the files fetched; `signal` stops it. The result's paths are under `/<id>/`, the
 * manifest's own directory in the workspace:
 * - `files`: the text of every file, by path, decoded after its bytes were checked;
 * - `pool`: the manifest's pool, in its order, where a bare include and a package by name are
 *   looked for;
 * - `entry`: the path to open; `level`: the level to show, a subcircuit's name or "(top)", or null;
 *   and the manifest's `id`, `title`, `description`, `author`, `source` and `license` as it gives them.
 * Rejects with a ManifestError that names the file, when a file is the trouble.
 */
export async function readManifest(url, { fetch = globalThis.fetch, onProgress, signal, concurrency = 6 } = {}) {
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
  const manifest = checkManifest(parsed);
  const root = `/${manifest.id}/`;
  const total = manifest.files.length;
  let done = 0;
  onProgress?.(done, total);
  const files = {};
  // the first file that fails stops the fetches still running, as the caller's signal does
  const stop = new AbortController();
  const stopAll = () => stop.abort(signal?.reason);
  signal?.addEventListener("abort", stopAll, { once: true });
  try {
    await inParallel(manifest.files, concurrency, async (file) => {
      if (stop.signal.aborted) return;
      const data = await fetchChecked(fetch, file.url ?? new URL(file.path, manifest.base).href, { ...file, name: file.path, signal: stop.signal });
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
    pool: manifest.pool.map((path) => root + path),
  };
}
