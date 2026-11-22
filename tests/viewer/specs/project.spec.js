// Projects on the web (apps/viewer-ui/project.js): Download Project, Open Project, and the manifest of
// links, Download / Load Manifest. Download Project replaced Download Library; its archive keeps
// what that one kept, the Library's own files and the edits, with a manifest, rnl-project.json.
import { test, expect } from '@playwright/test';
import { dialogGone } from '../viewer.js';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { zhMenu } from '../zh.js';

test.skip(!process.env.RNL_WEB_URL, 'the web site is not built');

async function start(page, query = '') {
  await page.route('https://rich-netlist.github.io/rnl-packages/**', route => route.abort());
  // Exercise the ordinary download fallback in both browsers without opening a system picker.
  await page.addInitScript(() => Object.defineProperty(window, 'showSaveFilePicker', { value: undefined, configurable: true, writable: true }));
  await page.goto(`${process.env.RNL_WEB_URL}/index.html${query}`);
  await expect(page.locator('#menubar')).toBeVisible();
}
async function ready(page) {
  await page.waitForFunction(() => window.viewer.doc && !document.body.classList.contains('busy'));
}
/** Python reads the ZIP independently, checking every CRC, and gives each file's bytes. */
function unzip(file) {
  const out = execFileSync('python', ['-c',
    'import sys,zipfile,json,base64; z=zipfile.ZipFile(sys.argv[1]); assert z.testzip() is None; print(json.dumps({n:base64.b64encode(z.read(n)).decode() for n in z.namelist()}))',
    file], { maxBuffer: 32 << 20 });
  return JSON.parse(out.toString());
}
/** Answer the question a save asks first: whether to write the display settings shown, then `button`. */
async function answer(page, { write = false, button = 'Download' } = {}) {
  const box = page.locator('#write-display');
  await box.waitFor();
  if (write) await box.check();
  await page.getByRole('dialog').getByRole('button', { name: button, exact: true }).click();
}
async function project(page, { write = false } = {}) {
  const download = page.waitForEvent('download');
  await page.evaluate(() => { window.viewer.command('save-project'); });
  await answer(page, { write });
  // a dialog instead of the download says why
  const dialog = page.getByRole('dialog');
  const result = await Promise.race([download, dialog.waitFor().then(async () => { throw new Error(await dialog.innerText()); })]);
  const saved = path.join(test.info().outputDir, result.suggestedFilename());
  await result.saveAs(saved);
  const files = unzip(saved);
  return { name: result.suggestedFilename(), saved, files, manifest: JSON.parse(content(files, 'rnl-project.json')) };
}
const content = (files, name) => Buffer.from(files[name], 'base64').toString();
const fixtureFolder = (files) => {
  const folder = fs.mkdtempSync(path.join(os.tmpdir(), 'rnl-project-'));
  for (const [name, text] of Object.entries(files)) {
    const file = path.join(folder, name);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, text);
  }
  return folder;
};
/** Choose File > `command` in the menu, as the user does, and `files` in the chooser it shows. */
async function choose(page, command, id, files) {
  const chooser = page.waitForEvent('filechooser');
  await page.getByRole('menuitem', { name: 'File', exact: true }).click();
  await page.locator(`[data-id="${command}"]`).click();
  await (await chooser).setFiles(files);
}

test('the File menu offers the project and manifest commands, in both languages, and no longer Download Library', async ({ page }) => {
  await start(page);
  await page.getByRole('menuitem', { name: 'File', exact: true }).click();
  await expect(page.locator('[data-id="open-project"]')).toHaveText('Open Project...');
  await expect(page.locator('[data-id="load-manifest"]')).toHaveText('Load Manifest...');
  await expect(page.locator('[data-id="save-project"]')).toHaveText('Download Project...');
  await expect(page.locator('[data-id="download-manifest"]')).toHaveText('Download Manifest...');
  await expect(page.locator('[data-id="download-library"]')).toHaveCount(0);
  await page.keyboard.press('Escape');
  await page.evaluate(() => window.viewer.command('language-zh'));
  await page.getByRole('menuitem', { name: zhMenu('&File'), exact: true }).click();
  await expect(page.locator('[data-id="open-project"]')).toHaveText(zhMenu('Open Projec&t...'));
  await expect(page.locator('[data-id="load-manifest"]')).toHaveText(zhMenu('Load &Manifest...'));
  await expect(page.locator('[data-id="save-project"]')).toHaveText(zhMenu('Download Pro&ject...'));
  await expect(page.locator('[data-id="download-manifest"]')).toHaveText(zhMenu('Do&wnload Manifest...'));
  if (process.env.RNL_REVIEW_DIR && test.info().project.name === 'chromium') {
    await page.screenshot({ path: path.join(process.env.RNL_REVIEW_DIR, 'project-menu-zh.png') });
  }
  await page.keyboard.press('Escape');
  await page.evaluate(() => window.viewer.command('language-en'));
  // without a netlist there is no project
  await page.evaluate(() => { window.viewer.command('save-project'); });
  await expect(page.getByRole('dialog', { name: 'Download Project', exact: true })).toContainText('Open a netlist first.');
});

test('Download Project keeps unused folder files, duplicate basenames, edits and public packages, with the manifest', async ({ page }) => {
  await start(page);
  const files = {
    'top.sp': '* original\r\n.end\r\n',
    'a/parts.inc': '* first\n', 'b/parts.inc': '* second\n', 'lib/未引用.rnl': '* unicode unused package\n',
  };
  const folder = fixtureFolder(files);
  try {
    const input = page.locator('#folder-input').setInputFiles(folder).catch(() => {});
    const name = path.basename(folder);
    await expect(page.getByRole('button', { name: `Open ${name}/top.sp`, exact: true })).toBeVisible();
    // the page has the folder once its file is listed; in WebKit the setting of a folder's files may never return,
    // so it is not waited for long, as in web.spec.js
    await Promise.race([input, new Promise((resolve) => setTimeout(resolve, 3000))]);
    await page.getByRole('button', { name: `Open ${name}/top.sp`, exact: true }).click();
    await ready(page);
    await page.evaluate(async (name) => {
      const { restorePool } = await import('./pool.js');
      const path = '/library/third-party/unused.rnl', text = '* downloaded package\n';
      await restorePool([{ path, name: 'unused 0.1', url: 'https://example.test/unused.rnl', read: () => new File([text], 'unused.rnl'),
        source: { path, name: 'unused 0.1', read: async () => text } }]);
      (await import('./formats.js')).chooseFormat(`/${name}/b/parts.inc`, 'spectre');
      window.beforeExport = window.viewer.doc;
    }, name);
    await page.locator('#tab-netlist').click();
    await page.keyboard.press('Control+q');
    const area = page.locator('#source textarea:visible');
    await area.fill('* 编辑后的内容\nR1 p n 2k\n.end\n');
    const out = await project(page);
    expect(out.name).toBe('top.zip');
    expect(content(out.files, `${name}/top.sp`)).toBe('* 编辑后的内容\r\nR1 p n 2k\r\n.end\r\n');
    for (const file of Object.keys(files).filter(n => n !== 'top.sp')) expect(content(out.files, `${name}/${file}`)).toBe(files[file]);
    expect(content(out.files, 'library/third-party/unused.rnl')).toBe('* downloaded package\n');
    // the built-in packages are the viewer's own: not in a project
    expect(Object.keys(out.files).some((n) => n.startsWith('standard-packages/'))).toBe(false);
    const m = out.manifest;
    expect(m.format).toBe(1);
    expect(m.entry).toBe(`${name}/top.sp`);
    expect(m.level).toBe('(top)');
    expect(m.formats).toEqual({ [`${name}/b/parts.inc`]: 'spectre' });
    expect(m.settings).toEqual({ autoInclude: true });
    expect(m.pool).toContain('library/third-party/unused.rnl');
    expect(m.files.map((f) => f.path).sort()).toEqual(Object.keys(out.files).filter((n) => n !== 'rnl-project.json').sort());
    expect(m.files.find((f) => f.path === 'library/third-party/unused.rnl').url).toBe('https://example.test/unused.rnl');
    expect(await page.evaluate(() => window.viewer.doc === window.beforeExport)).toBe(true);
    expect(await page.evaluate(() => window.viewer.edits.unsaved)).toHaveLength(1);
    await expect(area).toHaveValue('* 编辑后的内容\nR1 p n 2k\n.end\n');
  } finally {
    // Only the exact directory created by mkdtemp above is removed.
    fs.rmSync(folder, { recursive: true, force: true });
  }
});

test('Download Project keeps loaded URL includes for an offline project, with their addresses', async ({ page }) => {
  await start(page, '?url=samples/cases/04-hierarchy.sp');
  await ready(page);
  const document = await page.evaluate(() => window.viewer.doc);
  await page.route('**/samples/**', route => route.abort());
  const out = await project(page);
  for (const file of document.files) expect(content(out.files, `samples/cases/${file.path}`)).toBe(file.text);
  for (const file of out.manifest.files) expect(file.url).toMatch(/\/samples\/cases\/04-hierarchy/);
  expect(out.manifest.level).toBe('(top)');
});

test('Download Project keeps the bytes of files with a BOM or in Latin-1', async ({ page }) => {
  await start(page);
  const originals = {
    'bom.sp': Buffer.concat([Buffer.from([239, 187, 191]), Buffer.from('* BOM\n.end\n')]),
    'latin.sp': Buffer.from('* café\n.end\n', 'latin1'),
    'current.sp': Buffer.from('* current\n.end\n'),
  };
  for (const [name, buffer] of Object.entries(originals)) {
    await page.locator('#file-input').setInputFiles({ name, mimeType: 'text/plain', buffer });
    await expect.poll(() => page.evaluate(() => window.viewer.doc?.path)).toBe(`/${name}`);
    await ready(page);
  }
  const out = await project(page);
  for (const [name, bytes] of Object.entries(originals)) expect(Buffer.from(out.files[name], 'base64')).toEqual(bytes);
  expect(out.manifest.entry).toBe('current.sp');
});

test('a project whose file has % in its name is downloaded and opened again', async ({ page }) => {
  await start(page);
  await page.locator('#file-input').setInputFiles({ name: 'gain%20.sp', mimeType: 'text/plain', buffer: Buffer.from('* gain\nR1 a 0 1k\n.end\n') });
  await expect.poll(() => page.evaluate(() => window.viewer.doc?.path)).toBe('/gain%20.sp');
  await ready(page);
  const out = await project(page);
  // the manifest lists the file by its own path, and its name in the archive, escaped once to be portable
  expect(out.manifest.entry).toBe('gain%20.sp');
  expect(out.manifest.files.map((f) => [f.path, f.member])).toEqual([['gain%20.sp', 'gain%2520.sp']]);
  expect(Object.keys(out.files)).toContain('gain%2520.sp');
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await choose(page, 'open-project', 'project-input', out.saved);
  await ready(page);
  expect(await page.evaluate(() => window.viewer.doc.files[0].text)).toBe('* gain\nR1 a 0 1k\n.end\n');
  expect(await page.evaluate(() => window.viewer.doc.diagnostics.filter((d) => d.severity === 'error').length)).toBe(0);
});

test('a file included by a name with % in it is included again when the project is opened', async ({ page }) => {
  await start(page);
  await page.locator('#file-input').setInputFiles([
    { name: 'top.sp', mimeType: 'text/plain', buffer: Buffer.from('* top\n.include "gain%20.inc"\nX1 a b gain\n.end\n') },
    { name: 'gain%20.inc', mimeType: 'text/plain', buffer: Buffer.from('.subckt gain a b\nR1 a b 1k\n.ends\n') },
  ]);
  await ready(page);
  const errors = () => page.evaluate(() => window.viewer.doc.diagnostics.filter((d) => d.severity === 'error').map((d) => d.message));
  expect(await errors()).toEqual([]);
  const out = await project(page);
  expect(out.manifest.entry).toBe('top.sp');
  expect(out.manifest.files.find((f) => f.path === 'gain%20.inc')?.member).toBe('gain%2520.inc');
  expect(content(out.files, 'gain%2520.inc')).toBe('.subckt gain a b\nR1 a b 1k\n.ends\n');
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await choose(page, 'open-project', 'project-input', out.saved);
  await ready(page);
  // the netlist's include finds the file by the path it gives
  expect(await page.evaluate(() => window.viewer.doc.files.map((f) => f.path.split('/').pop()))).toEqual(['top.sp', 'gain%20.inc']);
  expect(await errors()).toEqual([]);
});

test('Download Project keeps edited files after Apply removes their include', async ({ page }) => {
  await start(page);
  await page.locator('#file-input').setInputFiles([
    { name: 'top.sp', mimeType: 'text/plain', buffer: Buffer.from('* top\n.include "parts.inc"\n.end\n') },
    { name: 'parts.inc', mimeType: 'text/plain', buffer: Buffer.from('* original part\nR1 p n 1k\n') },
  ]);
  await ready(page);
  await page.evaluate(async () => {
    const { setText } = await import('./edits.js');
    const { applyEdits } = await import('./open.js');
    setText('parts.inc', '* edited part\nR1 p n 2k\n');
    await applyEdits();
    setText('top.sp', '* no longer includes parts\n.end\n');
    await applyEdits();
  });
  const out = await project(page);
  expect(content(out.files, 'parts.inc')).toBe('* edited part\nR1 p n 2k\n');
  expect(content(out.files, 'top.sp')).toBe('* no longer includes parts\n.end\n');
  expect(await page.evaluate(() => window.viewer.edits.unsaved)).toContain('parts.inc');
});

test('Download Project uses the save picker, captures edits before it waits and prevents duplicate exports', async ({ page }) => {
  await start(page);
  await page.locator('#file-input').setInputFiles({ name: 'capture.sp', mimeType: 'text/plain', buffer: Buffer.from('* original\n.end\n') });
  await ready(page);
  await page.keyboard.press('Control+q');
  await page.locator('#source textarea:visible').fill('* first snapshot\n.end\n');
  await page.evaluate(() => {
    window.pickerCalls = 0;
    window.showSaveFilePicker = async options => {
      window.pickerCalls++;
      window.pickerOptions = options;
      await new Promise(resolve => { window.finishPicker = resolve; });
      return { createWritable: async () => ({
        write: async blob => { window.writtenZip = Array.from(new Uint8Array(await blob.arrayBuffer())); },
        close: async () => { window.zipClosed = true; }, abort: async () => {},
      }) };
    };
    window.viewer.command('save-project');
    window.viewer.command('save-project');
  });
  await answer(page);
  await expect.poll(() => page.evaluate(() => window.pickerCalls)).toBe(1);
  await page.locator('#source textarea:visible').fill('* later typing\n.end\n');
  await page.evaluate(() => window.finishPicker());
  await expect.poll(() => page.evaluate(() => window.zipClosed)).toBe(true);
  const result = await page.evaluate(() => ({ options: window.pickerOptions, bytes: window.writtenZip }));
  expect(result.options.suggestedName).toBe('capture.zip');
  const saved = path.join(test.info().outputDir, 'picked.zip');
  fs.mkdirSync(path.dirname(saved), { recursive: true });
  fs.writeFileSync(saved, Buffer.from(result.bytes));
  expect(content(unzip(saved), 'capture.sp')).toBe('* first snapshot\n.end\n');
  await expect(page.locator('#source textarea:visible')).toHaveValue('* later typing\n.end\n');
});

test('canceling the save picker reads nothing, and an unreadable file fails without a partial archive', async ({ page }) => {
  await start(page);
  await page.locator('#file-input').setInputFiles({ name: 'top.sp', mimeType: 'text/plain', buffer: Buffer.from('* top\n.end\n') });
  await ready(page);
  let downloads = 0;
  page.on('download', () => downloads++);
  await page.evaluate(() => {
    window.showSaveFilePicker = async () => { throw new DOMException('Canceled', 'AbortError'); };
    window.viewer.command('save-project');
  });
  await answer(page);
  await dialogGone(page);
  expect(downloads).toBe(0);
  // cancelling the question saves nothing either
  await page.evaluate(() => { window.viewer.command('save-project'); });
  await page.locator('#write-display').waitFor();
  await page.getByRole('dialog').getByRole('button', { name: 'Cancel', exact: true }).click();
  await dialogGone(page);
  expect(downloads).toBe(0);
  await page.evaluate(async () => {
    window.showSaveFilePicker = undefined;
    const { restorePool } = await import('./pool.js');
    await restorePool([{ path: '/missing.inc', name: 'missing.inc', source: {}, read: () => { throw Error('removed file'); } }]);
    window.viewer.command('save-project');
  });
  await answer(page);
  const dialog = page.getByRole('dialog', { name: 'Download Project', exact: true });
  await expect(dialog).toContainText('/missing.inc: removed file');
  await expect(page.locator('#note')).not.toContainText('Preparing');
  expect(downloads).toBe(0);
});

test('Open Project reads a project back: its Library, its entry, its level and the formats chosen', async ({ page }) => {
  await start(page, '?url=samples/cases/04-hierarchy.sp');
  await ready(page);
  // the subcircuit of the included file, which the outline has (the document shown has the levels of its own file)
  const sub = await page.evaluate(() => {
    const { outline } = window.viewer.doc;
    return outline.levels.findIndex((l) => l.kind === 'subckt' && outline.files[l.file].path.endsWith('-lib.inc'));
  });
  const title = await page.evaluate((host) => window.viewer.doc.outline.levels[host].title, sub);
  // it opens with its file shown, read in the context of the entry, which stays the project's entry
  await page.evaluate(async (host) => {
    await (await import('./formats.js')).chooseFormat('/samples/cases/04-hierarchy.sp', 'spice');
    await (await import('./subcircuits.js')).enterHost(host);
  }, sub);
  await expect.poll(() => page.evaluate(() => window.viewer.doc.levels[window.viewer.level].title)).toBe(title);
  await ready(page);
  const document = await page.evaluate(() => window.viewer.doc);
  expect(document.file).toBe('04-hierarchy-lib.inc');
  const out = await project(page);
  expect(out.manifest.entry).toBe('samples/cases/04-hierarchy.sp');
  expect(out.manifest.level).toBe(title);
  expect(out.manifest.file).toBe('samples/cases/04-hierarchy-lib.inc');
  // a new page, with nothing of the first, opens it
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await page.route('**/samples/**', route => route.abort());
  await choose(page, 'open-project', 'project-input', out.saved);
  await ready(page);
  await expect.poll(() => page.evaluate(() => window.viewer.doc.levels[window.viewer.level].title)).toBe(title);
  const opened = await page.evaluate(() => window.viewer.doc);
  // the file of the level, read in the context of the entry again: the same files, texts and problems
  expect(opened.file).toBe('04-hierarchy-lib.inc');
  expect(opened.outline.files.map((f) => f.path)).toEqual(document.outline.files.map((f) => f.path));
  expect(opened.files.map((f) => f.text)).toEqual(document.files.map((f) => f.text));
  const problems = (d) => d.diagnostics.map((x) => [x.severity, x.message, x.line ?? null]);
  expect(problems(opened)).toEqual(problems(document));
  await page.locator('#tab-files').click();
  // the files go under the archive's name, as a folder's do; the pool's are the Library's files
  const root = out.name.replace(/\.zip$/, '');
  await expect(page.locator('.lib-group[data-group="files"] .pool-open')).toContainText([`${root}/samples/cases/04-hierarchy.sp`]);
  expect(await page.evaluate(async (path) => (await import('./formats.js')).chosenFormat(path), `/${root}/samples/cases/04-hierarchy.sp`)).toBe('spice');
  if (process.env.RNL_REVIEW_DIR && test.info().project.name === 'chromium') {
    await page.screenshot({ path: path.join(process.env.RNL_REVIEW_DIR, 'project-opened.png') });
  }
});

test('Open Project refuses an archive without a manifest, and one whose file does not match', async ({ page }) => {
  await start(page);
  const plain = path.join(test.info().outputDir, 'plain.zip');
  fs.mkdirSync(path.dirname(plain), { recursive: true });
  execFileSync('python', ['-c', 'import sys,zipfile; z=zipfile.ZipFile(sys.argv[1],"w"); z.writestr("top.sp","* top\\n"); z.close()', plain]);
  await choose(page, 'open-project', 'project-input', plain);
  const dialog = page.getByRole('dialog', { name: 'Open Project', exact: true });
  await expect(dialog).toContainText('plain.zip is not a project: it has no rnl-project.json.');
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await dialogGone(page);
  const tampered = path.join(test.info().outputDir, 'tampered.zip');
  execFileSync('python', ['-c', [
    'import sys,zipfile,json,hashlib',
    'm={"format":1,"entry":"top.sp","files":[{"path":"top.sp","sha256":hashlib.sha256(b"* top\\n").hexdigest(),"size":6}]}',
    'z=zipfile.ZipFile(sys.argv[1],"w"); z.writestr("rnl-project.json",json.dumps(m)); z.writestr("top.sp","* TOP\\n"); z.close()'].join('\n'), tampered]);
  await choose(page, 'open-project', 'project-input', tampered);
  await expect(dialog).toContainText('top.sp does not match the SHA-256 the manifest lists.');
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await dialogGone(page);
  // a project larger than the web's WebAssembly core holds is for the desktop viewer
  const large = path.join(test.info().outputDir, 'large.zip');
  execFileSync('python', ['-c', [
    'import sys,zipfile,json',
    'm={"format":1,"entry":"top.sp","files":[{"path":"top.sp","sha256":"0"*64,"size":300*1024*1024}]}',
    'z=zipfile.ZipFile(sys.argv[1],"w"); z.writestr("rnl-project.json",json.dumps(m)); z.close()'].join('\n'), large]);
  await choose(page, 'open-project', 'project-input', large);
  await expect(dialog).toContainText('The project holds 300 MB, more than this page opens (256 MB). Open it in the desktop viewer.');
});

test('Download Manifest gives the files by their addresses, which Load Manifest opens again', async ({ page }) => {
  await start(page, '?url=samples/cases/04-hierarchy.sp');
  await ready(page);
  const document = await page.evaluate(() => window.viewer.doc);
  const download = page.waitForEvent('download');
  await page.evaluate(() => { window.viewer.command('download-manifest'); });
  await answer(page);
  const result = await download;
  expect(result.suggestedFilename()).toBe('04-hierarchy.json');
  const saved = path.join(test.info().outputDir, result.suggestedFilename());
  await result.saveAs(saved);
  const manifest = JSON.parse(fs.readFileSync(saved, 'utf8'));
  expect(manifest.entry).toBe('samples/cases/04-hierarchy.sp');
  for (const file of manifest.files) expect(file.url).toMatch(/^http:\/\/.*\/samples\/cases\//);
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await choose(page, 'load-manifest', 'manifest-input', saved);
  await ready(page);
  const opened = await page.evaluate(() => window.viewer.doc);
  expect(opened.files.map((f) => f.text)).toEqual(document.files.map((f) => f.text));
  // a manifest of links has no title, author or source: no bar
  await expect(page.locator('#manifest-bar')).toBeHidden();
  await page.locator('#tab-files').click();
  await expect(page.locator('.lib-group[data-group="external"] .lib-title')).toHaveText('From external manifest · 04-hierarchy · read only');
});

test('Download Manifest refuses files chosen from the computer, and changes not saved', async ({ page }) => {
  await start(page);
  await page.locator('#file-input').setInputFiles({ name: 'local.sp', mimeType: 'text/plain', buffer: Buffer.from('* local\n.end\n') });
  await ready(page);
  await page.evaluate(() => { window.viewer.command('download-manifest'); });
  const dialog = page.getByRole('dialog', { name: 'Download Manifest', exact: true });
  await expect(dialog).toContainText('Local files have no web URL (local.sp). Use Download Project to save them in a ZIP archive.');
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await page.goto(`${process.env.RNL_WEB_URL}/index.html?url=samples/cases/04-hierarchy.sp`);
  await ready(page);
  await page.evaluate(async () => (await import('./edits.js')).setText(window.viewer.doc.files[0].path, '* changed\n.end\n'));
  await page.evaluate(() => { window.viewer.command('download-manifest'); });
  await expect(dialog).toContainText('cannot include unsaved edits');
});
