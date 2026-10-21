// Node consumers of the single public-package lock and the verified checkout-local cache.
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
export const root = dirname(dirname(fileURLToPath(import.meta.url)));
export const packageCache = join(root, '.rnl-packages');
export const packageLocks = JSON.parse(readFileSync(join(root, 'packages.lock.json'), 'utf8')).packages;
export function publicPackageDir(name) {
  const entry = packageLocks.find(p => p.name === name);
  if (!entry) throw new Error(`No lock for ${name}`);
  return join(packageCache, name, entry.version);
}
export function publicPackageText(name) {
  const p = packageLocks.find(p => p.name === name);
  const path = join(publicPackageDir(name), `${name}.rnl`);
  const bytes = readFileSync(path);
  if (createHash('sha256').update(bytes).digest('hex') !== p.sha256) throw new Error(`${path}: SHA-256 mismatch`);
  return bytes.toString('utf8');
}
export function publicPackagesAvailable(names = packageLocks.map(p => p.name)) {
  const missing = names.some(name => !existsSync(join(publicPackageDir(name), `${name}.rnl`)));
  if (missing) {
    const message = 'Public packages are missing; run `python scripts/packages.py download` from the checkout root.';
    if (process.env.CI) throw new Error(message);
    console.warn(`SKIP public package tests: ${message}`);
    return false;
  }
  for (const name of names) publicPackageText(name);
  return true;
}
export function preparePublicPackageFixture() {
  const fixture = join(packageCache, 'fixture');
  if (publicPackagesAvailable()) execFileSync('python', [join(root, 'scripts/packages.py'), 'fixture'], { cwd: root });
  else {
    // Non-package browser tests must remain offline even when the local download was omitted.
    mkdirSync(fixture, { recursive: true });
    writeFileSync(join(fixture, 'catalog.json'), JSON.stringify({ schemaVersion: 1, packages: [] }) + '\n');
  }
  return fixture;
}
