import { execFileSync } from 'node:child_process';
import { join } from 'node:path';
import { preparePublicPackageFixture, root } from '../scripts/packages.mjs';

export default function setup() {
  const fixture = preparePublicPackageFixture();
  execFileSync(process.execPath, ['site/gallery/build.mjs'], {
    cwd: root, stdio: 'inherit',
    env: { ...process.env, RNL_PACKAGE_CATALOG: join(fixture, 'catalog.json') },
  });
}
