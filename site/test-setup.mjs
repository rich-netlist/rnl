import { execFileSync } from 'node:child_process';
import { join, dirname, basename } from 'node:path';
import { tmpdir } from 'node:os';
import { readFileSync, writeFileSync, mkdtempSync, realpathSync, rmSync } from 'node:fs';
import { preparePublicPackageFixture, root } from '../scripts/packages.mjs';

export default function setup() {
  const specs = process.argv.filter(arg => /\.spec\.mjs$/.test(arg));
  if (specs.length && specs.every(spec => /macos-downloads\.spec\.mjs$/.test(spec))) return;
  const fixture = preparePublicPackageFixture();
  execFileSync(process.execPath, ['site/gallery/build.mjs'], {
    cwd: root, stdio: 'inherit',
    env: { ...process.env, RNL_PACKAGE_CATALOG: join(fixture, 'catalog.json') },
  });
  // The third locale exists only in an OS temp directory, never in the published artifact.
  const temp = mkdtempSync(join(tmpdir(), 'rnl-site-i18n-'));
  const config = JSON.parse(readFileSync(join(root, 'site/languages.json'), 'utf8'));
  config.languages.push({ code: 'ja', name: '日本語', short: 'JA', messages: { intro: 'ホーム' } });
  const configPath = join(temp, 'languages.json');
  writeFileSync(configPath, JSON.stringify(config));
  execFileSync(process.env.PYTHON || 'python', ['site/build.py', '--out', join(temp, 'dist')], {
    cwd: root, stdio: 'pipe', env: { ...process.env, RNL_SITE_LANGUAGES: configPath },
  });
  process.env.RNL_I18N_FIXTURE = join(temp, 'dist');
  return () => {
    const target = realpathSync(temp);
    if (dirname(target) !== realpathSync(tmpdir()) || !basename(target).startsWith('rnl-site-i18n-')) {
      throw new Error('Unexpected i18n fixture path');
    }
    rmSync(target, { recursive: true, force: true });
  };
}
