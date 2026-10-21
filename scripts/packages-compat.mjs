// The daily compatibility check uses the current catalog and the main runtime supplied by CI.
import { execFileSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { packageCache, root } from './packages.mjs';

const runtimeRoot = resolve(process.argv[2] || root);
const catalog = JSON.parse(readFileSync(join(packageCache, 'catalog.json'), 'utf8'));
const paths = catalog.packages.flatMap(p => ['--path', join(packageCache, p.name, p.version)]);
for (const p of catalog.packages) execFileSync(join(runtimeRoot, 'target/debug/rnl'), [
  'check', join(packageCache, p.name, p.version, `${p.name}.rnl`), ...paths,
], { stdio: 'inherit' });
execFileSync(process.execPath, [join(root, 'site/gallery/build.mjs'), '--core', join(runtimeRoot, 'bindings/runtime-wasm/dist')], {
  cwd: root, stdio: 'inherit', env: { ...process.env, RNL_PACKAGE_CATALOG: join(packageCache, 'catalog.json') },
});
for (const file of readdirSync(join(root, 'site/dist/gallery')).filter(p => p.endsWith('.js') && p !== 'index.js')) {
  const text = readFileSync(join(root, 'site/dist/gallery', file), 'utf8');
  const group = JSON.parse(text.slice(text.indexOf('(') + 1, text.lastIndexOf(');')));
  for (const p of group.packages) {
    const problems = [...p.problems, ...p.statements.flatMap(s => s.problems || [])];
    if (problems.some(p => JSON.stringify(p).includes('error:'))) throw new Error(`${p.name}: ${JSON.stringify(problems)}`);
  }
}
console.log('ok: latest public packages read and every symbol renders with main');
