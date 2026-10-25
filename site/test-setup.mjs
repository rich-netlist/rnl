import { execFileSync } from 'node:child_process';
import { join, dirname, basename } from 'node:path';
import { tmpdir } from 'node:os';
import { readFileSync, writeFileSync, mkdirSync, mkdtempSync, realpathSync, rmSync } from 'node:fs';
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
  const config = JSON.parse(readFileSync(join(root, 'i18n/languages.json'), 'utf8'));
  config.languages.push({ code: 'ja', name: '日本語', names: { en: 'Japanese' }, short: 'JA', status: 'published' });
  const configPath = join(temp, 'languages.json');
  writeFileSync(configPath, JSON.stringify(config));
  mkdirSync(join(temp, 'ja', 'site'), { recursive: true });
  writeFileSync(join(temp, 'ja', 'site', 'messages.json'), JSON.stringify({ intro: 'ホーム' }));
  // Its catalogs, beside its registry as i18n/<lang>/site/ is beside i18n/languages.json: the gallery page
  // and its scripts but for two texts, so that it is shown translated with the quieter notice; the other
  // pages have none and are shown in English with the notice that they are not translated (decision 0015).
  const englishTexts = file => [...readFileSync(join(root, 'site/src', file), 'utf8').matchAll(/<en(?: ctx=([a-z0-9-]+))?>([\s\S]*?)<\/en>/g)]
    .map(m => (m[1] ? `${m[1]}|` : '') + m[2].split(/\s+/).filter(Boolean).join(' '));
  for (const [file, leave] of [['gallery.html', 0], ['assets/circuits.js', 2], ['assets/common.js', 0]]) {
    const path = join(temp, 'ja', 'site', `${file}.json`);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, JSON.stringify(Object.fromEntries(englishTexts(file).slice(leave).map(key => [key, `（訳）${key.replace(/^[a-z0-9-]+\|/, '')}`]))));
  }
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
