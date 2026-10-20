import { execFileSync } from 'node:child_process';
import { join } from 'node:path';
import { packageCache, publicPackagesAvailable, root } from '../scripts/packages.mjs';

export default function setup() {
  if (!publicPackagesAvailable()) return;
  execFileSync('python', ['scripts/packages.py', 'fixture'], { cwd: root });
  execFileSync(process.execPath, ['site/gallery/build.mjs'], {
    cwd: root, stdio: 'inherit',
    env: { ...process.env, RNL_PACKAGE_CATALOG: join(packageCache, 'fixture/catalog.json') },
  });
}
