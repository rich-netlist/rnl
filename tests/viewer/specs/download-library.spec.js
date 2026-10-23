import { test, expect } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

test.skip(!process.env.RNL_WEB_URL, 'the web site is not built');

async function start(page) {
  await page.route('https://rich-netlist.github.io/rnl-packages/**', route => route.abort());
  // Exercise the ordinary download fallback in both browsers without opening a system picker.
  await page.addInitScript(() => Object.defineProperty(window, 'showSaveFilePicker', { value: undefined, configurable: true, writable: true }));
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await expect(page.locator('#menubar')).toBeVisible();
}
async function ready(page) {
  await page.waitForFunction(() => window.viewer.doc && !document.body.classList.contains('busy'));
}
async function archive(page, action = () => page.evaluate(() => window.viewer.command('download-library'))) {
  const download = page.waitForEvent('download');
  await action();
  const result = await download;
  expect(result.suggestedFilename()).toBe('rnl-library.zip');
  // Python validates the ZIP independently, including every CRC, then returns all file contents.
  const out = execFileSync('python', ['-c',
    'import sys,zipfile,json,base64; z=zipfile.ZipFile(sys.argv[1]); assert z.testzip() is None; print(json.dumps({n:base64.b64encode(z.read(n)).decode() for n in z.namelist()}))',
    await result.path()], { maxBuffer: 32 << 20 });
  return JSON.parse(out.toString());
}
const content = (files, name) => Buffer.from(files[name], 'base64').toString();

test('Download Library includes all built-ins without an open document, and both menu labels translate', async ({ page }) => {
  await start(page);
  await page.getByRole('menuitem', { name: 'File', exact: true }).click();
  await expect(page.locator('[data-id="save"]')).toHaveText('Download FileCtrl+S');
  const files = await archive(page, () => page.locator('[data-id="download-library"]').click());
  await expect(page.locator('#note')).toBeVisible();
  await expect(page.locator('#note')).toHaveText(/Downloaded library \(\d+ files\)\./);
  const bundled = await page.evaluate(async () => (await fetch('libraries/bundled.json')).json());
  expect(Object.keys(files)).toHaveLength(Object.keys(bundled).length);
  for (const [name, text] of Object.entries(bundled)) expect(content(files, name.replace('<bundled>/', 'standard-packages/'))).toBe(text);
  await page.evaluate(() => window.viewer.command('language-zh'));
  await page.getByRole('menuitem', { name: '文件(F)', exact: true }).click();
  await expect(page.locator('[data-id="save"]')).toHaveText('下载文件(D)Ctrl+S');
  await expect(page.locator('[data-id="download-library"]')).toHaveText('下载资源库(Y)');
});

test('Download Library exports unused folder files, duplicate basenames, current edits and public packages', async ({ page }) => {
  await start(page);
  const folder = fs.mkdtempSync(path.join(os.tmpdir(), 'rnl-library-'));
  const files = {
    'top.sp': '* original\r\n.end\r\n',
    'a/parts.inc': '* first\n', 'b/parts.inc': '* second\n', 'lib/未引用.rnl': '* unicode unused package\n',
  };
  try {
    for (const [name, text] of Object.entries(files)) {
      const file = path.join(folder, name);
      fs.mkdirSync(path.dirname(file), { recursive: true });
      fs.writeFileSync(file, text);
    }
    const input = page.locator('#folder-input').setInputFiles(folder).catch(() => {});
    const name = path.basename(folder);
    await expect(page.getByRole('button', { name: `Open ${name}/top.sp`, exact: true })).toBeVisible();
    await input;
    await page.getByRole('button', { name: `Open ${name}/top.sp`, exact: true }).click();
    await ready(page);
    // Install a downloaded public-package item through the same descriptor path as Include Public Package.
    await page.evaluate(async () => {
      const { restorePool } = await import('./pool.js');
      const path = '/library/third-party/unused.rnl', text = '* downloaded package\n';
      await restorePool([{ path, name: 'unused 0.1', read: () => new File([text], 'unused.rnl'),
        source: { path, name: 'unused 0.1', read: async () => text } }]);
      window.beforeExport = window.viewer.doc;
    });
    await page.locator('#tab-netlist').click();
    await page.keyboard.press('Control+q');
    const area = page.locator('#source textarea:visible');
    await area.fill('* 编辑后的内容\nR1 p n 2k\n.end\n');
    const out = await archive(page);
    expect(content(out, `${name}/top.sp`)).toBe('* 编辑后的内容\r\nR1 p n 2k\r\n.end\r\n');
    for (const file of Object.keys(files).filter(n => n !== 'top.sp')) expect(content(out, `${name}/${file}`)).toBe(files[file]);
    expect(content(out, 'library/third-party/unused.rnl')).toBe('* downloaded package\n');
    expect(await page.evaluate(() => window.viewer.doc === window.beforeExport)).toBe(true);
    expect(await page.evaluate(() => window.viewer.edits.unsaved)).toHaveLength(1);
    await expect(area).toHaveValue('* 编辑后的内容\nR1 p n 2k\n.end\n');
    // Download File retains Ctrl+S, downloads only the current text and Apply keeps the edit.
    const download = page.waitForEvent('download');
    await page.keyboard.press('Control+s');
    const single = await download;
    expect(single.suggestedFilename()).toBe('top.sp');
    expect(fs.readFileSync(await single.path(), 'utf8')).toBe(content(out, `${name}/top.sp`));
    await page.keyboard.press('Control+q');
    await expect(page.locator('#source textarea:visible')).toHaveCount(0);
    expect(await page.evaluate(() => window.viewer.doc.files[0].text)).toContain('R1 p n 2k');
  } finally {
    // Only the exact directory created by mkdtemp above is removed.
    fs.rmSync(folder, { recursive: true, force: true });
  }
});

test('Download Library snapshots loaded URL includes for an offline export', async ({ page }) => {
  await start(page);
  await page.goto(`${process.env.RNL_WEB_URL}/index.html?url=samples/cases/04-hierarchy.sp`);
  await ready(page);
  const document = await page.evaluate(() => window.viewer.doc);
  await page.route('**/samples/**', route => route.abort());
  const out = await archive(page);
  for (const file of document.files) expect(content(out, `samples/cases/${file.path}`)).toBe(file.text);
});

test('Download Library preserves BOM and Latin-1 bytes of previously opened files', async ({ page }) => {
  await start(page);
  const originals = {
    'bom.sp': Buffer.concat([Buffer.from([239,187,191]), Buffer.from('* BOM\n.end\n')]),
    'latin.sp': Buffer.from('* café\n.end\n', 'latin1'),
    'current.sp': Buffer.from('* current\n.end\n'),
  };
  for (const [name, buffer] of Object.entries(originals)) {
    await page.locator('#file-input').setInputFiles({ name, mimeType: 'text/plain', buffer });
    await expect.poll(() => page.evaluate(() => window.viewer.doc?.path)).toBe(`/${name}`);
    await ready(page);
  }
  const out = await archive(page);
  for (const [name, bytes] of Object.entries(originals)) expect(Buffer.from(out[name], 'base64')).toEqual(bytes);
});

test('Download Library keeps edited files after Apply removes their include', async ({ page }) => {
  await start(page);
  await page.locator('#file-input').setInputFiles([
    { name: 'top.sp', mimeType: 'text/plain', buffer: Buffer.from('* top\n.include "parts.inc"\n.end\n') },
    { name: 'parts.inc', mimeType: 'text/plain', buffer: Buffer.from('* original part\nR1 p n 1k\n') },
  ]);
  await ready(page);
  expect(await page.evaluate(() => window.viewer.doc.files.map(f => f.path))).toContain('parts.inc');
  await page.evaluate(async () => {
    const { setText } = await import('./edits.js');
    const { applyEdits } = await import('./open.js');
    setText('parts.inc', '* edited part\nR1 p n 2k\n');
    await applyEdits();
    setText('top.sp', '* no longer includes parts\n.end\n');
    await applyEdits();
  });
  expect(await page.evaluate(() => window.viewer.doc.files.map(f => f.path))).toEqual(['top.sp']);
  expect(await page.evaluate(() => window.viewer.edits.unsaved)).toContain('parts.inc');
  const out = await archive(page);
  expect(content(out, 'parts.inc')).toBe('* edited part\nR1 p n 2k\n');
  expect(content(out, 'top.sp')).toBe('* no longer includes parts\n.end\n');
  expect(await page.evaluate(() => window.viewer.edits.unsaved)).toContain('parts.inc');
});

test('Download Library uses the save picker, captures edits before it waits and prevents duplicate exports', async ({ page }) => {
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
    window.viewer.command('download-library');
    window.viewer.command('download-library');
  });
  await expect.poll(() => page.evaluate(() => window.pickerCalls)).toBe(1);
  await page.locator('#source textarea:visible').fill('* later typing\n.end\n');
  await page.evaluate(() => window.finishPicker());
  await expect.poll(() => page.evaluate(() => window.zipClosed)).toBe(true);
  const result = await page.evaluate(() => ({ options: window.pickerOptions, bytes: window.writtenZip, dirty: window.viewer.edits.unsaved }));
  expect(result.options.suggestedName).toBe('rnl-library.zip');
  const out = JSON.parse(execFileSync('python', ['-c',
    'import sys,io,zipfile,json; z=zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read())); assert z.testzip() is None; print(json.dumps({n:z.read(n).decode() for n in z.namelist()}))'],
    { input: Buffer.from(result.bytes), maxBuffer: 32 << 20 }).toString());
  expect(out['capture.sp']).toBe('* first snapshot\n.end\n');
  expect(result.dirty).toHaveLength(1);
  await expect(page.locator('#source textarea:visible')).toHaveValue('* later typing\n.end\n');
});

test('canceling the save picker reads nothing, and a subsequent download still works', async ({ page }) => {
  await start(page);
  let downloads = 0;
  page.on('download', () => downloads++);
  await page.evaluate(async () => {
    window.showSaveFilePicker = async () => { throw new DOMException('Canceled', 'AbortError'); };
    await window.viewer.command('download-library');
  });
  expect(downloads).toBe(0);
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.locator('#note')).not.toContainText('Preparing');
  await page.evaluate(() => { window.showSaveFilePicker = async () => { throw new DOMException('Blocked', 'SecurityError'); }; });
  expect(Object.keys(await archive(page)).length).toBeGreaterThan(0);
});

test('unreadable library entries report a failure without a partial ZIP', async ({ page }) => {
  await start(page);
  // A stale File handle is kept visible by Library, which reports its read error on indexing too.
  await page.evaluate(async () => {
    const { restorePool } = await import('./pool.js');
    await restorePool([{ path: '/missing.inc', name: 'missing.inc', source: {}, read: () => { throw Error('removed file'); } }]);
    window.viewer.command('download-library');
  });
  const dialog = page.getByRole('dialog', { name: 'Download Library', exact: true });
  await expect(dialog).toContainText('/missing.inc: removed file');
  await expect(page.locator('#note')).not.toContainText('Preparing');
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
});
