// ZIP STORE, UTF-8 names and CRC-32 (PKWARE APPNOTE 4.3). No runtime dependency or network request.
// Blob parts keep file bodies separate; yield during checksums so large libraries do not freeze UI.
const encoder = new TextEncoder();
const MAX = 0xffffffff;
const crcTable = Uint32Array.from({ length: 256 }, (_, i) => {
  for (let n = 0; n < 8; n++) i = i & 1 ? 0xedb88320 ^ (i >>> 1) : i >>> 1;
  return i >>> 0;
});

async function crc32(bytes) {
  let crc = MAX;
  for (let start = 0; start < bytes.length; start += 1048576) {
    const end = Math.min(start + 1048576, bytes.length);
    for (let i = start; i < end; i++) crc = crcTable[(crc ^ bytes[i]) & 255] ^ (crc >>> 8);
    await new Promise(resolve => setTimeout(resolve, 0));
  }
  return (crc ^ MAX) >>> 0;
}

/** Relative, portable ZIP paths; never let URL paths or virtual roots escape the archive. */
function archivePath(path) {
  path = path.replace(/^<bundled>\//, "standard-packages/").replace(/^<temporary>\//, "temporary/");
  return path.replaceAll("\\", "/").split("/").filter(p => p && p !== ".").map(part => {
    if (part === "..") return "%2E%2E";
    let safe = part.replace(/[%<>:"|?*\x00-\x1f]/g, c => `%${c.charCodeAt(0).toString(16).toUpperCase().padStart(2, "0")}`)
      .replace(/[. ]+$/, tail => [...tail].map(c => `%${c.charCodeAt(0).toString(16).toUpperCase()}`).join(""));
    if (/^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(safe)) safe = `_${safe}`;
    return safe;
  }).join("/");
}

/**
 * The names the files at `paths` get in an archive, in their order: relative, portable, and distinct
 * even where a file system ignores case. Names that are already such are kept, so the names given
 * back give themselves again.
 */
export function archiveNames(paths) {
  const names = paths.map(archivePath), used = new Set(), directories = new Set();
  // Reserve every required directory before naming files, independent of input order. A file
  // called lib.sp must be renamed when another entry needs lib.sp/ as a directory.
  for (const path of names) {
    for (let at = path.indexOf("/"); at >= 0; at = path.indexOf("/", at + 1)) directories.add(path.slice(0, at).toLowerCase());
  }
  return names.map((name, index) => {
    if (!name) throw new Error(`An archive file has no name: ${paths[index]}`);
    const base = name, dot = name.lastIndexOf(".") > name.lastIndexOf("/") ? name.lastIndexOf(".") : name.length;
    for (let suffix = 2; used.has(name.toLowerCase()) || directories.has(name.toLowerCase()); suffix++) name = `${base.slice(0, dot)} (${suffix})${base.slice(dot)}`;
    used.add(name.toLowerCase());
    return name;
  });
}

/** Entries have { path, read }, whose reader returns a string, Blob or Uint8Array. */
export async function libraryZip(files) {
  if (files.length >= 65535) throw new Error("The library has too many files for a ZIP archive (maximum 65534).");
  const bodies = [], directory = [];
  const names = archiveNames(files.map(file => file.path));
  let offset = 0, directorySize = 0;
  for (const [index, file] of files.entries()) {
    const name = names[index];
    const encoded = encoder.encode(name);
    if (encoded.length > 65535) throw new Error(`The archive path is too long: ${file.path}`);
    let value;
    try { value = await file.read(); }
    catch (e) { throw new Error(`${file.path}: ${e.message || e}`); }
    if (value == null) throw new Error(`${file.path} could not be read.`);
    const bytes = typeof value === "string" ? encoder.encode(value)
      : value instanceof Blob ? new Uint8Array(await value.arrayBuffer()) : value;
    if (!(bytes instanceof Uint8Array)) throw new Error(`${file.path} did not provide file bytes.`);
    const size = bytes.byteLength;
    directorySize += 46 + encoded.length;
    if (offset + 30 + encoded.length + size + directorySize + 22 >= MAX) throw new Error("The library is too large for a ZIP archive (maximum 4 GiB).");
    const crc = await crc32(bytes);
    const header = new Uint8Array(30), local = new DataView(header.buffer);
    local.setUint32(0, 0x04034b50, true);
    local.setUint16(4, 20, true); // ZIP 2.0
    local.setUint16(6, 0x800, true); // UTF-8
    local.setUint16(12, 33, true); // 1980-01-01; no filesystem timestamp is claimed
    local.setUint32(14, crc, true);
    local.setUint32(18, size, true);
    local.setUint32(22, size, true);
    local.setUint16(26, encoded.length, true);
    const entry = new Uint8Array(46), central = new DataView(entry.buffer);
    central.setUint32(0, 0x02014b50, true);
    central.setUint16(4, 20, true);
    entry.set(header.subarray(4, 30), 6);
    central.setUint32(42, offset, true);
    bodies.push(header, encoded, new Blob([bytes]));
    directory.push(entry, encoded);
    offset += 30 + encoded.length + size;
  }
  const end = new Uint8Array(22), record = new DataView(end.buffer);
  record.setUint32(0, 0x06054b50, true);
  record.setUint16(8, files.length, true);
  record.setUint16(10, files.length, true);
  record.setUint32(12, directorySize, true);
  record.setUint32(16, offset, true);
  return new Blob([...bodies, ...directory, end], { type: "application/zip" });
}

/**
 * The files of a ZIP archive, by name: each reads its bytes, inflated when they were deflated (the
 * browser's DecompressionStream), and checked against its CRC-32. Directories are left out; ZIP64,
 * encryption and other methods are refused with a message that says so.
 *
 * @param {Uint8Array} bytes
 * @returns {Map<string, { size: number, read: () => Promise<Uint8Array> }>}
 */
export function readZip(bytes) {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let end = -1;
  for (let at = bytes.length - 22; at >= Math.max(0, bytes.length - 22 - 65535); at--) {
    if (view.getUint32(at, true) === 0x06054b50) { end = at; break; }
  }
  if (end < 0) throw new Error("The file is not a ZIP archive.");
  const count = view.getUint16(end + 10, true);
  let at = view.getUint32(end + 16, true);
  if (count === 0xffff || at === MAX) throw new Error("ZIP64 archives are not supported.");
  const files = new Map();
  const latin1 = new TextDecoder("latin1"), utf8 = new TextDecoder();
  for (let i = 0; i < count; i++) {
    if (at + 46 > bytes.length || view.getUint32(at, true) !== 0x02014b50) throw new Error("The ZIP archive's directory is damaged.");
    const flags = view.getUint16(at + 8, true), method = view.getUint16(at + 10, true);
    const crc = view.getUint32(at + 16, true), packed = view.getUint32(at + 20, true), size = view.getUint32(at + 24, true);
    const nameLength = view.getUint16(at + 28, true), extra = view.getUint16(at + 30, true), comment = view.getUint16(at + 32, true);
    const local = view.getUint32(at + 42, true);
    const rawName = bytes.subarray(at + 46, at + 46 + nameLength);
    const name = (flags & 0x800 ? utf8 : latin1).decode(rawName).replaceAll("\\", "/");
    at += 46 + nameLength + extra + comment;
    if (name.endsWith("/")) continue;
    if (flags & 1) throw new Error(`${name} is encrypted.`);
    if (method !== 0 && method !== 8) throw new Error(`${name} is compressed in a way this viewer does not read (method ${method}).`);
    files.set(name, { size, read: async () => {
      if (view.getUint32(local, true) !== 0x04034b50) throw new Error(`${name}: the ZIP archive is damaged.`);
      const start = local + 30 + view.getUint16(local + 26, true) + view.getUint16(local + 28, true);
      const body = bytes.subarray(start, start + packed);
      const data = method === 0 ? body.slice() : await inflate(body);
      if (data.length !== size || await crc32(data) !== crc) throw new Error(`${name}: the ZIP archive is damaged.`);
      return data;
    } });
  }
  return files;
}

async function inflate(body) {
  if (typeof DecompressionStream !== "function") throw new Error("This browser cannot read compressed ZIP archives.");
  const stream = new Blob([body]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}
