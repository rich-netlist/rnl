import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { readCatalog, readPublicPackage, latestPackages, matchingPackage, missingPackage, sha256, validateCatalog } from '../viewer-ui/public-packages.js';
const bytes = new TextEncoder().encode('a public package\r\n');
const pkg = { name: 'example', version: '0.1', kind: 'extension', summary: { zh: '简介', en: 'Summary' },
  path: 'example/example.rnl', url: 'packages/example.rnl', sha256: await sha256(bytes) };
test('catalog URLs resolve relative to their own location and unsupported records fail', async () => {
  const url = 'https://example.test/catalog/catalog.json';
  const result = await readCatalog(url, async () => new Response(JSON.stringify({ schemaVersion: 1, packages: [pkg] })));
  assert.equal(result.packages[0].url, 'https://example.test/catalog/packages/example.rnl');
  assert.match(result.sha256, /^[a-f0-9]{64}$/);
  for (const data of [{ schemaVersion: 2, packages: [] }, { schemaVersion: 1, packages: [pkg, pkg] },
    { schemaVersion: 1, packages: [{ ...pkg, version: '0.1.1' }] }]) assert.throws(() => validateCatalog(data, url));
});
test('package verification rejects altered raw bytes, including line-ending changes', async () => {
  assert.deepEqual(new Uint8Array(await readPublicPackage(pkg, async () => new Response(bytes))), bytes);
  await assert.rejects(readPublicPackage(pkg, async () => new Response('a public package\n')), /SHA-256 mismatch/);
  await assert.rejects(readPublicPackage(pkg, async () => new Response('', { status: 404 })), /could not be read/);
});
test('latest selection compares the two numeric version parts and keeps names separate', () => {
  assert.deepEqual(latestPackages(['0.9', '0.10', '0.2'].map(version => ({ ...pkg, version }))).map(p => p.version), ['0.10']);
});
test('the npm manifest no longer exports optional public package data', () => {
  const manifest = JSON.parse(readFileSync(new URL('../../bindings/runtime-wasm/js/package.json', import.meta.url)));
  assert.equal(manifest.exports['./packages/analog-canvas-extensions.rnl'], undefined);
});

test('missing named dependencies follow core version rules without guessing from symbol errors', () => {
  const packages = ['0.1','0.2','1.3','1.5','2.0'].map(version => ({...pkg,version}));
  assert.equal(matchingPackage(packages, 'example', '0.1').version, '0.1');
  assert.equal(matchingPackage(packages, 'example', '0.3'), undefined);
  assert.equal(matchingPackage(packages, 'example', '1.2').version, '1.3');
  assert.equal(matchingPackage(packages, 'example', '1.9'), undefined);
  assert.equal(matchingPackage(packages, 'example').version, '2.0');
  assert.equal(matchingPackage(packages, 'other'), undefined);
  const diagnostic = message => ({severity:'error',message});
  assert.deepEqual(missingPackage(diagnostic('package example 0.1 cannot be found: this USE-PACKAGE fails (16.2)')), {name:'example',version:'0.1'});
  assert.deepEqual(missingPackage(diagnostic('package example cannot be found: this USE-PACKAGE fails (16.2)')), {name:'example',version:undefined});
  for (const message of ['$MISSING cannot be found (6.6)', 'package example 0.1 cannot be found: this reference fails (16.2)',
    'the included file example.rnl cannot be found']) assert.equal(missingPackage(diagnostic(message)), null);
});
