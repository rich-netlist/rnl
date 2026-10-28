// Shared by the web viewer and Node builds. All hashes cover the downloaded raw bytes.
export const DEFAULT_CATALOG = 'https://rich-netlist.github.io/rnl-packages/catalog.json';
export async function sha256(bytes) {
  const hash = await crypto.subtle.digest('SHA-256', bytes);
  return Array.from(new Uint8Array(hash), n => n.toString(16).padStart(2, '0')).join('');
}
export function validateCatalog(data, url) {
  if (data?.schemaVersion !== 1 || !Array.isArray(data.packages)) throw new Error('Unsupported public package catalog format.');
  const seen = new Set();
  return data.packages.map(p => {
    const key = `${p.name} ${p.version}`;
    if (!/^[a-z0-9-]+$/.test(p.name) || !/^\d+\.\d+$/.test(p.version) || !/^[a-f0-9]{64}$/.test(p.sha256) ||
        !['pdk', 'extension'].includes(p.kind) || typeof p.summary?.en !== 'string' ||
        Object.values(p.summary).some(text => typeof text !== 'string') || seen.has(key))
      throw new Error(`Invalid public package record: ${key}`);
    seen.add(key);
    const resolved = new URL(p.url, url);
    if (!['https:', 'http:', 'file:'].includes(resolved.protocol)) throw new Error('Invalid public package URL.');
    return { ...p, url: resolved.href, builtin: false };
  });
}
export async function readCatalog(url, fetcher = fetch) {
  const response = await fetcher(url);
  if (!response.ok) throw new Error(`The public package catalog could not be read (${response.status}).`);
  const bytes = await response.arrayBuffer();
  return { packages: validateCatalog(JSON.parse(new TextDecoder().decode(bytes)), response.url || url), sha256: await sha256(bytes) };
}
export async function readPublicPackage(pkg, fetcher = fetch) {
  const response = await fetcher(pkg.url);
  if (!response.ok) throw new Error(`The public package ${pkg.name} could not be read (${response.status}).`);
  const bytes = await response.arrayBuffer();
  if (await sha256(bytes) !== pkg.sha256) throw new Error(`SHA-256 mismatch for ${pkg.name} ${pkg.version}.`);
  return bytes;
}
export function latestPackages(packages) {
  const selected = new Map();
  for (const p of packages) {
    const old = selected.get(p.name);
    const [a,b] = p.version.split('.').map(Number), [c,d] = (old?.version || '-1.-1').split('.').map(Number);
    if (!old || a > c || a === c && b > d) selected.set(p.name, p);
  }
  return [...selected.values()].sort((a,b) => a.path.localeCompare(b.path));
}

/** Match the core's named-package lookup: exact, compatible newer, or newest when unpinned. */
export function matchingPackage(packages, name, version) {
  const parts = v => v.split('.').map(Number);
  const order = (a, b) => parts(a.version)[0] - parts(b.version)[0] || parts(a.version)[1] - parts(b.version)[1];
  const named = packages.filter(p => !p.builtin && p.name === name).sort(order);
  if (!version) return named.at(-1);
  const exact = named.find(p => p.version === version);
  if (exact) return exact;
  const [major, minor] = parts(version);
  // Series 0.x are separate incompatible revisions; only stable major series can advance.
  return major > 0 ? named.find(p => parts(p.version)[0] === major && parts(p.version)[1] > minor) : undefined;
}

/** Only explicit named USE-PACKAGE failures qualify, never missing symbols, aliases or includes. */
export function missingPackage(diagnostic) {
  if (diagnostic.severity !== 'error') return null;
  const match = /^package (\S+)(?: (\S+))? cannot be found: this USE-PACKAGE fails/.exec(diagnostic.message);
  return match ? { name: match[1], version: match[2] } : null;
}
