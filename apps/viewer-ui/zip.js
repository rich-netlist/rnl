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

/** Entries have { path, read }, whose reader returns a string, Blob or Uint8Array. */
export async function libraryZip(files) {
  if (files.length >= 65535) throw new Error("The library has too many files for a ZIP archive (maximum 65534).");
  const bodies = [], directory = [], used = new Set();
  const paths = files.map(file => archivePath(file.path));
  const directories = new Set();
  // Reserve every required directory before naming files, independent of input order. A file
  // called lib.sp must be renamed when another entry needs lib.sp/ as a directory.
  for (const path of paths) {
    for (let at = path.indexOf("/"); at >= 0; at = path.indexOf("/", at + 1)) directories.add(path.slice(0, at).toLowerCase());
  }
  let offset = 0, directorySize = 0;
  for (const [index, file] of files.entries()) {
    let name = paths[index];
    if (!name) throw new Error("An archive file has no name.");
    const base = name, dot = name.lastIndexOf(".") > name.lastIndexOf("/") ? name.lastIndexOf(".") : name.length;
    for (let suffix = 2; used.has(name.toLowerCase()) || directories.has(name.toLowerCase()); suffix++) name = `${base.slice(0, dot)} (${suffix})${base.slice(dot)}`;
    used.add(name.toLowerCase());
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
