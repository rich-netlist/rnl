// The solve of the circuit shown in the web viewer, which reads the file shown alone (solve.js, `settings.file`; `rnl solve
// --file`): only the file of the circuit is read for it, and the preview is that file's, with the outline of the reading it
// was solved from, so that the hierarchy is the whole document's. Another file of the document, visited from the preview,
// is shown as it is read, the preview still on; coming back to the file solved shows the preview again, not solved again.
// The solver is the one in the browser (apps/web/solver-worker.js).
import fs from 'node:fs';
import { test, expect } from '../viewer.js';

test.skip(!process.env.RNL_WEB_URL, 'the web site is not built');

const TOP = `* top
.include "cells.inc"
X1 a b cell
R9 b 0 1k
.end
`;
const CELLS = `* cells
.subckt cell p n
R1 p m 1k
R2 m n 1k
.ends
`;

/** Keep the requests the page sends the solver's Worker, without the files' texts, as `window.__solver`. */
async function watchSolver(page) {
  await page.addInitScript(() => {
    const Original = window.Worker;
    window.__solver = [];
    window.Worker = class extends Original {
      constructor(url, options) {
        super(url, options);
        if (!/solver-worker\.js/.test(String(url))) return;
        const post = this.postMessage.bind(this);
        this.postMessage = (message, ...rest) => {
          if (message.request) {
            const { files, ...request } = message.request;
            window.__solver.push({ ...request, files: Object.keys(files) });
          }
          return post(message, ...rest);
        };
      }
    };
  });
}

async function open(page, files) {
  test.setTimeout(120000);
  await page.route('https://rich-netlist.github.io/rnl-packages/**', route => route.abort());
  await page.addInitScript(() => localStorage.setItem('file-preload', 'false'));
  await watchSolver(page);
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await expect(page.locator('#menubar')).toBeVisible();
  await page.locator('#file-input').setInputFiles(files.map(([name, text]) => ({ name, mimeType: 'text/plain', buffer: Buffer.from(text) })));
  await idle(page);
}
const idle = (page) => page.waitForFunction(() => window.viewer.doc && !document.body.classList.contains('busy'));
const solved = (page) => page.waitForFunction(() => document.body.classList.contains('preview') && !document.body.classList.contains('solving'), null, { timeout: 90000 });
const requests = (page) => page.evaluate(() => window.__solver);
/** What is shown: the file of the Document, whether it is the preview itself, and whether the preview is on. */
const shown = (page) => page.evaluate(async () => {
  const solve = await import('./solve.js');
  return { file: window.viewer.doc.file, preview: solve.previewShown(), on: solve.previewing(), level: document.getElementById('level-name').textContent };
});
const instance = (page, name) => page.evaluate((n) => window.viewer.doc.objects.findIndex((o) => o.kind === 'instance' && o.name === n), name);
async function descend(page, name) {
  const o = await instance(page, name);
  await page.evaluate((k) => import('./subcircuits.js').then((s) => s.descend(k)), o);
  await idle(page);
}

test('a circuit of an included file is solved reading only that file; the way up and back down shows the same preview', async ({ page }) => {
  await open(page, [['top.sp', TOP], ['cells.inc', CELLS]]);
  await descend(page, 'X1');
  expect(await shown(page)).toMatchObject({ file: 'cells.inc', preview: false, on: false, level: 'cell' });
  await page.keyboard.press('F9');
  await solved(page);
  // the request names the circuit and its file; the preview is the reading of that file, its hierarchy the whole document's
  const [request] = await requests(page);
  expect(request).toMatchObject({ level: 'cell', file: 'cells.inc' });
  expect(await shown(page)).toEqual({ file: 'cells.inc', preview: true, on: true, level: 'cell' });
  expect(await page.evaluate(() => window.viewer.doc.levels.map((l) => l.title))).toEqual(['cell']);
  expect(await page.evaluate(() => !!window.viewer.doc.outline)).toBe(true);
  await page.locator('#tab-hierarchy').click();
  await expect(page.locator('#tree .tree-row')).toHaveCount(2);
  await expect(page.locator('#tree .tree-row').nth(1)).toContainText('X1');
  await expect(page.locator('#solve-report .solve-level')).toHaveCount(1);
  // up to the top level, in the other file: shown as it is read, the preview still on, its report and Save Solved Copy too
  await page.keyboard.press('Backspace');
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'top.sp', preview: false, on: true });
  await expect(page.locator('#preview-badge')).toBeVisible();
  await expect(page.locator('#solve-report .solve-level')).toHaveCount(1);
  await expect(page.locator('#tree .tree-row')).toHaveCount(2);
  // and down again: the same preview, not solved again
  await descend(page, 'X1');
  expect(await shown(page)).toEqual({ file: 'cells.inc', preview: true, on: true, level: 'cell' });
  // a pick in the hierarchy goes the same ways
  await page.locator('#tree .tree-row').nth(0).click();
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'top.sp', preview: false, on: true });
  await page.locator('#tree .tree-row').nth(1).click();
  await idle(page);
  expect(await shown(page)).toEqual({ file: 'cells.inc', preview: true, on: true, level: 'cell' });
  expect(await requests(page)).toHaveLength(1);
  // Save Solved Copy saves the solve, from the file solved and from another file visited
  const save = async () => {
    const download = page.waitForEvent('download');
    await page.evaluate(() => { void window.viewer.command('solve-save'); });
    const name = (await download).suggestedFilename();
    // the page says what it downloaded, in a dialog that is closed before going on
    await expect(page.locator('dialog.dialog')).toContainText(name);
    await page.locator('dialog.dialog button').first().click();
    await expect(page.locator('dialog.dialog')).toHaveCount(0);
    return name;
  };
  expect(await save()).toMatch(/\.zip$/);
  // (the top level, by the hierarchy: the level picked there is the root of the path, with nothing up from it)
  await page.locator('#tree .tree-row').nth(0).click();
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'top.sp', preview: false, on: true });
  expect(await save()).toMatch(/\.zip$/);
  // Close Preview from there ends the preview, the file shown as it is
  await page.evaluate(() => window.viewer.command('solve-close'));
  await expect(page.locator('#preview-badge')).toBeHidden();
  expect(await shown(page)).toMatchObject({ file: 'top.sp', preview: false, on: false });
  // the file solved, visited again, is read as it is
  await descend(page, 'X1');
  expect(await shown(page)).toMatchObject({ file: 'cells.inc', preview: false, on: false });
});

test('the preview of a solve ends when another document is opened, from another file visited too', async ({ page }) => {
  await open(page, [['top.sp', TOP], ['cells.inc', CELLS]]);
  await descend(page, 'X1');
  await page.keyboard.press('F9');
  await solved(page);
  await page.keyboard.press('Backspace');
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'top.sp', on: true });
  await page.locator('#file-input').setInputFiles([{ name: 'other.sp', mimeType: 'text/plain', buffer: Buffer.from('* other\nR1 a 0 1k\n.end\n') }]);
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'other.sp', preview: false, on: false });
  await expect(page.locator('#preview-badge')).toBeHidden();
});

test('a module of Verilog-A opened from the preview of a solve, and the way back to the preview', async ({ page }) => {
  const input = (name) => fs.readFileSync(new URL(`../inputs/va-top/${name}`, import.meta.url), 'utf8');
  await open(page, [['top.scs', input('top.scs')], ['a.va', input('a.va')], ['b.va', input('b.va')]]);
  await page.keyboard.press('F9');
  await solved(page);
  const [request] = await requests(page);
  expect(request.file).toBe('top.scs');
  const top = await shown(page);
  expect(top).toMatchObject({ file: 'top.scs', preview: true, on: true });
  // the hierarchy lists the module, from the outline the preview has
  await page.locator('#tab-hierarchy').click();
  await expect(page.locator('#tree .tree-row').nth(1)).toContainText('top_va');
  // the module of I1, in its own file: shown, the preview on
  await descend(page, 'I1');
  await expect(page.locator('#module-card')).toBeVisible();
  await expect(page.locator('#module-card .module-name')).toHaveText('top_va');
  await expect(page.locator('#preview-badge')).toBeVisible();
  // back: the preview again, not solved again
  await page.keyboard.press('Backspace');
  await idle(page);
  await expect(page.locator('#module-card')).toBeHidden();
  expect(await shown(page)).toEqual(top);
  expect(await requests(page)).toHaveLength(1);
});

test('an include removed from the file is not solved or saved with it, though an earlier open read it', async ({ page }) => {
  const UNUSED = `* unused\n.subckt u p\nR1 p 0 1k\n.ends\n`;
  await open(page, [['a.sp', `* a\n.include "unused.inc"\nR1 a 0 1k\n.end\n`], ['unused.inc', UNUSED]]);
  // the same file again, without its include; the included file is still there
  await page.locator('#file-input').setInputFiles([['a.sp', `* a\nR1 a 0 1k\n.end\n`], ['unused.inc', UNUSED]]
    .map(([name, text]) => ({ name, mimeType: 'text/plain', buffer: Buffer.from(text) })));
  await idle(page);
  expect(await page.evaluate(() => window.viewer.doc.files.map((f) => f.path))).toEqual(['a.sp']);
  await page.keyboard.press('F9');
  await solved(page);
  expect((await requests(page)).at(-1).files).not.toContain('/unused.inc');
  const download = page.waitForEvent('download');
  await page.evaluate(() => { void window.viewer.command('solve-save'); });
  expect((await download).suggestedFilename()).toBe('a-solved.sp');
});
