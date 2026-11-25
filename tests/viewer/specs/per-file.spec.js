// Per-file loading in the web viewer (shells/web.js `perFile`, visit-source.js, level-tree.js): the page reads the
// file shown alone, in the context of its entry, with the outline of the whole document. The hierarchy lists the
// levels of every file from the outline; entering a subcircuit another file defines shows that file's reading, with
// the same entry, and the path bar and the tree follow the way back.
import { test, expect } from '../viewer.js';

test.skip(!process.env.RNL_WEB_URL, 'the web site is not built');

const TOP = `* top
.include "cells.inc"
.subckt t p q
R9 p q 1k
.ends
X1 a b cell
X2 a b t
.end
`;
const CELLS = `* cells
.include "leaf.inc"
.subckt cell p n
X1 p n leaf
X2 n p leaf
.ends
`;
const LEAF = `* leaf
.subckt leaf a b
R1 a b 1k
.ends
`;

async function open(page, { preload = false, top = TOP, more = [] } = {}) {
  await page.route('https://rich-netlist.github.io/rnl-packages/**', route => route.abort());
  // preloading reads files while the page is idle: off, but in its own test, so that what is kept is what the test reads
  await page.addInitScript((on) => localStorage.setItem('file-preload', String(on)), preload);
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await expect(page.locator('#menubar')).toBeVisible();
  await page.locator('#file-input').setInputFiles([
    { name: 'top.sp', mimeType: 'text/plain', buffer: Buffer.from(top) },
    { name: 'cells.inc', mimeType: 'text/plain', buffer: Buffer.from(CELLS) },
    { name: 'leaf.inc', mimeType: 'text/plain', buffer: Buffer.from(LEAF) },
    ...more.map(([name, text]) => ({ name, mimeType: 'text/plain', buffer: Buffer.from(text) })),
  ]);
  await idle(page);
}
const idle = (page) => page.waitForFunction(() => window.viewer.doc && !document.body.classList.contains('busy'));

/** What the page shows: the file read, the entry, the levels it has, the level drawn and the path's names. */
const shown = (page) => page.evaluate(() => {
  const d = window.viewer.doc;
  return { file: d.file, entry: d.path.split('/').pop(), levels: d.levels.map(l => l.title), level: d.levels[window.viewer.level].title,
    path: [...document.querySelectorAll('#level-path .crumb')].map(c => c.querySelector('.name, #level-name')?.textContent) };
});

test('the entry is read alone, and the hierarchy lists the levels of every file from the outline', async ({ page }) => {
  await open(page);
  expect(await shown(page)).toMatchObject({ file: 'top.sp', entry: 'top.sp', levels: ['top.sp', 't'], level: 'top.sp' });
  // the other files' texts are in the netlist pane, though only the entry was read
  expect(await page.evaluate(() => window.viewer.doc.files.map(f => [f.path, f.text.split('\n')[0]])))
    .toEqual([['top.sp', '* top'], ['cells.inc', '* cells'], ['leaf.inc', '* leaf']]);
  // the index has the subcircuits of every file, with their uses (the hierarchy is drawn while it is shown)
  await page.locator('#tab-hierarchy').click();
  const index = page.locator('#index .index-row:not(.top)');
  await expect(index.locator('.name')).toHaveText(['cell', 'leaf', 't']);
  await expect(index.locator('.uses')).toHaveText(['1 use', '2 uses', '1 use']);
  // the tree, under the top level: the instances, each with the subcircuit it calls
  const rows = page.locator('#tree .tree-row');
  await expect(rows.locator('.inst')).toHaveText(['top.sp', 'X1', 'X2']);
});

test('entering a subcircuit of another file reads that file in the context of the entry, and the path leads back', async ({ page }) => {
  await open(page);
  const x1 = await page.evaluate(() => window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === 'X1'));
  expect(await page.evaluate((o) => import('./subcircuits.js').then(s => s.canOpen(o)), x1)).toBe(true);
  await page.evaluate((o) => import('./subcircuits.js').then(s => s.descend(o)), x1);
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'cells.inc', entry: 'top.sp', levels: ['cell'], level: 'cell', path: ['top.sp', 'cell'] });
  // the tree keeps the way open to the level shown, from the top level (the hierarchy is drawn while it is shown)
  await page.locator('#tab-hierarchy').click();
  await expect(page.locator('#tree .tree-row.active .inst')).toHaveText('X1');
  // and on down into the third file
  const leaf = await page.evaluate(() => window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === 'X2'));
  await page.evaluate((o) => import('./subcircuits.js').then(s => s.descend(o)), leaf);
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'leaf.inc', entry: 'top.sp', level: 'leaf', path: ['top.sp', 'cell', 'leaf'] });
  // back up the path: the entry's reading again, with the instance it was left through selected
  await page.evaluate(() => import('./subcircuits.js').then(s => s.backTo(0)));
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'top.sp', level: 'top.sp', path: ['top.sp'] });
  await expect.poll(() => page.evaluate(() => window.viewer.doc.objects[window.viewer.selected]?.name)).toBe('X1');
});

test('a level of another file picked in the index is that file\'s reading, as the root of the path', async ({ page }) => {
  await open(page);
  await page.locator('#tab-hierarchy').click();
  await page.locator('#index .index-row:not(.top)', { hasText: 'leaf' }).click();
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'leaf.inc', entry: 'top.sp', levels: ['leaf'], level: 'leaf', path: ['leaf'] });
  await expect(page.locator('#index .index-row.current .name')).toHaveText('leaf');
  // and a level of the entry's own file, from there: the entry's reading
  await page.locator('#index .index-row:not(.top)', { hasText: /^t/ }).click();
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'top.sp', level: 't', path: ['t'] });
});

/** What the readings kept say of themselves (file-reading.js, result-cache.js `stats`). */
const kept = (page) => page.evaluate(async () => (await import('./file-reading.js')).readingStats());
const descendNamed = async (page, name) => {
  await page.evaluate((name) => import('./subcircuits.js').then(s => s.descend(window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === name))), name);
  await idle(page);
};

test('a file read before is shown again from the readings kept, until a file it depends on changes', async ({ page }) => {
  await open(page);
  expect(await kept(page)).toMatchObject({ hits: 0, misses: 1 });
  await descendNamed(page, 'X1');
  await page.evaluate(() => import('./subcircuits.js').then(s => s.backTo(0)));
  await idle(page);
  await descendNamed(page, 'X1');
  expect(await shown(page)).toMatchObject({ file: 'cells.inc', levels: ['cell'], path: ['top.sp', 'cell'] });
  expect(await kept(page)).toMatchObject({ hits: 2, misses: 2 });
  // the core reads it again for the drawings of what has no position, which a kept reading leaves out
  await page.locator('#tab-unplaced').click();
  await expect(page.locator('#unplaced-list svg').first()).toBeVisible();
  await expect(page.locator('#unplaced-list')).toContainText('X2');
  await expect(page.locator('#unplaced-list')).not.toContainText('document was built again');
  await page.locator('#tab-netlist').click();
  // the subcircuit cells.inc calls changes: its reading is made again, and kept again
  await page.keyboard.press('Control+q');
  await page.evaluate(async () => (await import('./text.js')).showFile(window.viewer.doc.files.findIndex(f => f.path === 'leaf.inc')));
  await page.locator('#source .editor:not([hidden]) textarea').fill(LEAF.replace('R1 a b 1k', 'R1 a b 2k'));
  await page.locator('#apply-edits').click();
  await idle(page);
  await expect.poll(() => kept(page)).toMatchObject({ hits: 2, misses: 3 });
  expect(await shown(page)).toMatchObject({ file: 'cells.inc', levels: ['cell'] });
});

test('while the page is idle, the files below the file shown are read into the cache, and shown from it', async ({ page }) => {
  await open(page, { preload: true });
  // the entry was read; cells.inc and leaf.inc, below it, are preloaded, which counts as no hit or miss
  await expect.poll(async () => (await kept(page)).entries, { timeout: 20000 }).toBe(3);
  expect(await kept(page)).toMatchObject({ hits: 0, misses: 1 });
  expect(await shown(page)).toMatchObject({ file: 'top.sp', level: 'top.sp' });
  await descendNamed(page, 'X1');
  expect(await shown(page)).toMatchObject({ file: 'cells.inc', levels: ['cell'], path: ['top.sp', 'cell'] });
  expect(await kept(page)).toMatchObject({ hits: 1, misses: 1 });
  // the Library has only the file opened
  await page.locator('#tab-files').click();
  await expect(page.locator('.lib-group[data-group="files"] .pool-row .pool-open')).toHaveText(['top.sp']);
});

test('the readings are kept in memory alone: another visit of the page reads the file again', async ({ page }) => {
  await open(page);
  await expect.poll(async () => (await kept(page)).entries).toBe(1);
  await open(page);
  expect(await kept(page)).toMatchObject({ entries: 1, hits: 0, misses: 1 });
  expect(await shown(page)).toMatchObject({ file: 'top.sp', levels: ['top.sp', 't'] });
  expect(await page.evaluate(async () => (await indexedDB.databases()).map((d) => d.name))).toEqual([]);
});

test('an included file opens from the Library in the context of the entry, and Add to Library makes it a Library file', async ({ page }) => {
  await open(page);
  await page.locator('#tab-files').click();
  const files = page.locator('.lib-group[data-group="files"]');
  await expect(files.locator('.pool-include .pool-include-name')).toHaveText(['cells.inc', 'leaf.inc']);
  await files.locator('.pool-include').getByRole('button', { name: 'Open cells.inc', exact: true }).click();
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'cells.inc', entry: 'top.sp', level: 'cell' });
  await expect(page.locator('#pane-files')).toBeVisible();
  // the rows stay those of the entry, which the Library knows; Add to Library gives leaf.inc a row of its own
  await files.locator('.pool-include').getByRole('button', { name: 'Add leaf.inc to the Library', exact: true }).click();
  await expect(files.locator('.pool-row .pool-open')).toHaveText(['top.sp', 'leaf.inc']);
  await expect(files.locator('.pool-include .pool-include-name')).toHaveText(['cells.inc']);
});

test('a package file a header names by its path is given with the first request of each reader: no reader, the first included, reads twice', async ({ page }) => {
  // the files each request to the core reads alone (FileResult), in the order asked
  await page.addInitScript(() => {
    window.fileReadings = [];
    const post = Worker.prototype.postMessage;
    Worker.prototype.postMessage = function (message, ...rest) {
      const file = typeof message?.request === 'string' ? JSON.parse(message.request).file : undefined;
      if (typeof file === 'string') window.fileReadings.push(file);
      return post.call(this, message, ...rest);
    };
  });
  const header = '* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="acme", version="1.0", path="acme.rnl") }\n';
  const acme = '@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="acme", version="1.0") }\n@RNL { $acme = @DEFINE-STYLE(linewidth=1) {} }\n';
  await open(page, { top: TOP.replace('* top\n', `* top\n${header}`), more: [['acme.rnl', acme]] });
  // the outline lists the package among the files missing, so the first reader of the entry gives it
  // with its first request
  const readings = () => page.evaluate(() => window.fileReadings);
  expect(await readings()).toEqual(['top.sp']);
  // and so does the next reader
  await descendNamed(page, 'X1');
  expect(await shown(page)).toMatchObject({ file: 'cells.inc', level: 'cell' });
  expect(await readings()).toEqual(['top.sp', 'cells.inc']);
  expect(await page.evaluate(() => window.viewer.doc.diagnostics.filter(d => /acme/.test(d.message)).map(d => d.message))).toEqual([]);
});

test('the outline kept holds for the next reader of the entry, also when the entry calls a subcircuit no file defines', async ({ page }) => {
  // the outlines the page asks its cores for, from the start
  await page.addInitScript(() => {
    window.outlineRequests = 0;
    const post = Worker.prototype.postMessage;
    Worker.prototype.postMessage = function (message, ...rest) {
      if (typeof message?.request === 'string' && JSON.parse(message.request).outline === true) window.outlineRequests++;
      return post.call(this, message, ...rest);
    };
  });
  // (a subcircuit no file defines, such as a device of a PDK: a file of the Library may come to define it)
  await open(page, { top: TOP.replace('X2 a b t\n', 'X2 a b t\nX3 a b pdkcell\n') });
  // (one for each time the core asks for the files the entry includes)
  const opened = await page.evaluate(() => window.outlineRequests);
  expect(opened).toBeGreaterThan(0);
  // the entry joined the Library by being opened: the outline kept still holds for the reader of the next file
  await descendNamed(page, 'X1');
  expect(await shown(page)).toMatchObject({ file: 'cells.inc', level: 'cell' });
  expect(await page.evaluate(() => window.outlineRequests)).toBe(opened);
});

// the outline and file requests to each Worker's core, in the order sent: the entry, the file, and the reading cache's fields
const watchRequests = (page) => page.addInitScript(() => {
  window.coreRequests = [];
  window.coreWorkers = 0;
  const workers = new WeakMap();
  const post = Worker.prototype.postMessage;
  Worker.prototype.postMessage = function (message, ...rest) {
    const r = typeof message?.request === 'string' ? JSON.parse(message.request) : null;
    if (r && (r.outline || typeof r.file === 'string' || typeof r.rest_of === 'string')) {
      if (!workers.has(this)) workers.set(this, ++window.coreWorkers);
      window.coreRequests.push({ worker: workers.get(this), outline: !!r.outline, file: r.file ?? null, restOf: r.rest_of ?? null,
        hints: r.hints ?? null, spans: r.spans ?? null, texts: r.texts ?? null, entry: r.entry, cache: r.cache === true, forget: r.forget === true });
    }
    return post.call(this, message, ...rest);
  };
});

test('the outline and the readings of files go through the reading cache of the core, which a reader of another entry empties first', async ({ page }) => {
  await watchRequests(page);
  await open(page);
  await descendNamed(page, 'X1');
  expect(await shown(page)).toMatchObject({ file: 'cells.inc', level: 'cell' });
  // another entry, then the first again
  await page.locator('#file-input').setInputFiles([{ name: 'leaf.inc', mimeType: 'text/plain', buffer: Buffer.from(LEAF) }]);
  await idle(page);
  await page.locator('#file-input').setInputFiles([{ name: 'top.sp', mimeType: 'text/plain', buffer: Buffer.from(TOP) },
    { name: 'cells.inc', mimeType: 'text/plain', buffer: Buffer.from(CELLS) }, { name: 'leaf.inc', mimeType: 'text/plain', buffer: Buffer.from(LEAF) }]);
  await idle(page);
  // (the rest of the file shown is asked for after it: below)
  const requests = (await page.evaluate(() => window.coreRequests)).filter(r => !r.restOf);
  expect(requests.length).toBeGreaterThanOrEqual(4);
  // each asks for the cache; each Worker's cache is emptied by the first request of another entry than its last, only
  const last = new Map();
  for (const r of requests) {
    expect(r.cache).toBe(true);
    expect(r.forget).toBe(last.has(r.worker) && last.get(r.worker) !== r.entry);
    last.set(r.worker, r.entry);
  }
  expect(requests.map(r => r.entry.split('/').pop())).toEqual(expect.arrayContaining(['top.sp', 'leaf.inc']));
});

test('what is shown is the same with the reading cache as without it', async ({ page, context }) => {
  const without = await context.newPage();
  await without.addInitScript(() => { globalThis.rnlReadingCache = false; });
  await watchRequests(page);
  await watchRequests(without);
  const doc = (p) => p.evaluate(() => JSON.stringify(window.viewer.doc));
  for (const p of [page, without]) await open(p);
  expect(await doc(page)).toBe(await doc(without));
  for (const p of [page, without]) await descendNamed(p, 'X1');
  expect(await shown(page)).toMatchObject({ file: 'cells.inc', level: 'cell' });
  expect(await doc(page)).toBe(await doc(without));
  // the page without asked for no cache
  expect((await without.evaluate(() => window.coreRequests)).some(r => r.cache || r.forget)).toBe(false);
  expect((await page.evaluate(() => window.coreRequests)).every(r => r.cache || r.restOf)).toBe(true);
});

// R2 drawn over R1: the level has geometry hints
const HINTED = `* hinted
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4") }
R1 a b 1k
R2 b 0 1k
* @RNL { #R1 @PLACE(0, 0); #R2 @PLACE(0, 0) }
.end
`;

test('the first answer of the file shown has no texts, spans or geometry hints, which its Worker gives right after, into the Document, the text and the problems list', async ({ page, context }) => {
  const without = await context.newPage();
  await without.addInitScript(() => { globalThis.rnlReadingCache = false; });
  await watchRequests(page);
  await watchRequests(without);
  for (const p of [page, without]) await open(p, { top: HINTED });
  // the reading of the file shown asks for the smaller answer; its Worker is asked for the rest next, before anything else
  const requests = await page.evaluate(() => window.coreRequests);
  const read = requests.findLastIndex(r => r.file === 'top.sp');
  expect(requests[read]).toMatchObject({ hints: false, spans: false, texts: false, cache: true });
  const next = requests.slice(read + 1).find(r => r.worker === requests[read].worker);
  expect(next).toMatchObject({ restOf: 'top.sp', entry: requests[read].entry, forget: false });
  // the page without the reading cache asks for the whole answer, as before
  const plain = await without.evaluate(() => window.coreRequests);
  expect(plain.some(r => r.restOf)).toBe(false);
  expect(plain.filter(r => r.file === 'top.sp').every(r => r.hints === true && r.spans === null && r.texts === null)).toBe(true);
  // the Document shown is the whole one: its texts, its spans and the hints of its levels
  const doc = (p) => p.evaluate(() => window.viewer.doc);
  const shownDoc = await doc(page);
  expect(shownDoc.levels.flatMap(l => l.hints ?? []).length).toBeGreaterThan(0);
  expect(shownDoc.spans.length).toBeGreaterThan(0);
  expect(shownDoc.files.find(f => f.path === 'top.sp').text).toBe(HINTED);
  expect(shownDoc).toEqual(await doc(without));
  // the problems list lists the hints of the level shown, and the text has the spans
  const listed = await page.evaluate(() => window.viewer.doc.levels[window.viewer.level].hints?.length ?? 0);
  expect(listed).toBeGreaterThan(0);
  for (const p of [page, without]) await expect(p.locator('#diags > li.diag.geo')).toHaveCount(listed);
  const objects = (p) => p.locator('#source [data-obj]:not([data-obj=""])').count();
  expect(await objects(page)).toBeGreaterThan(0);
  expect(await objects(page)).toBe(await objects(without));
});

test('a reading whose spans and hints could not be had is shown without them, and not kept', async ({ page }) => {
  // the rest of the first reading fails, and so does the reading of the file again in its place: as when the open is
  // superseded and its core stopped before the rest came
  await page.addInitScript(() => {
    let failing = 2;
    const post = Worker.prototype.postMessage;
    Worker.prototype.postMessage = function (message, ...rest) {
      const r = typeof message?.request === 'string' ? JSON.parse(message.request) : null;
      const fail = failing === 2 ? typeof r?.rest_of === 'string' : failing === 1 && typeof r?.file === 'string' && r.hints !== false;
      if (!fail) return post.call(this, message, ...rest);
      failing--;
      const error = failing === 1 ? 'not kept' : 'The core was stopped.';
      setTimeout(() => this.dispatchEvent(new MessageEvent('message', { data: { id: message.id, error } })));
    };
  });
  await open(page, { top: HINTED });
  // shown without them, with the problems listed all the same
  let d = await page.evaluate(() => window.viewer.doc);
  expect(d.spans).toEqual([]);
  expect(d.levels.flatMap(l => l.hints ?? [])).toEqual([]);
  await expect(page.locator('#counts')).not.toBeEmpty();
  // not kept: opened again, the file is read again, whole
  await page.locator('#file-input').setInputFiles([{ name: 'top.sp', mimeType: 'text/plain', buffer: Buffer.from(HINTED) },
    { name: 'cells.inc', mimeType: 'text/plain', buffer: Buffer.from(CELLS) }, { name: 'leaf.inc', mimeType: 'text/plain', buffer: Buffer.from(LEAF) }]);
  await idle(page);
  expect(await kept(page)).toMatchObject({ hits: 0, misses: 2 });
  d = await page.evaluate(() => window.viewer.doc);
  expect(d.spans.length).toBeGreaterThan(0);
  expect(d.levels.flatMap(l => l.hints ?? []).length).toBeGreaterThan(0);
  await expect(page.locator('#diags > li.diag.geo')).not.toHaveCount(0);
});

test('the spans and hints of a reading again of a file that changed are not put into the reading shown', async ({ page }) => {
  // the rest of the first reading is not kept, and the file changes before it is read again in its place: a line
  // before R1 and another resistor, as when a file on disk is saved between the two
  await page.addInitScript(() => {
    let step = 0;
    const post = Worker.prototype.postMessage;
    Worker.prototype.postMessage = function (message, ...rest) {
      const r = typeof message?.request === 'string' ? JSON.parse(message.request) : null;
      if (step === 0 && typeof r?.rest_of === 'string') {
        step = 1;
        setTimeout(() => this.dispatchEvent(new MessageEvent('message', { data: { id: message.id, error: 'not kept' } })));
        return;
      }
      if (step === 1 && typeof r?.file === 'string' && r.hints !== false) {
        step = 2;
        r.files = { ...r.files, [r.entry]: '* changed\n* a line\nR2 m n 2k\nR1 n 0 1k\n.end\n' };
        return post.call(this, { ...message, request: JSON.stringify(r) }, ...rest);
      }
      return post.call(this, message, ...rest);
    };
  });
  await open(page, { top: '* first\nR1 n 0 1k\n.end\n' });
  // each span names the object it marks, in the text shown
  const d = await page.evaluate(() => window.viewer.doc);
  const text = d.files.find((f) => f.path === 'top.sp').text.split('\n');
  for (const s of d.spans.filter((s) => s.kind === 'name' && d.files[s.file]?.path === 'top.sp')) {
    expect((text[s.line - 1] ?? '').slice(s.col - 1, s.col - 1 + s.len)).toBe(d.objects[s.object].name);
  }
  // shown without them, and not kept
  expect(d.spans).toEqual([]);
  await page.locator('#file-input').setInputFiles([{ name: 'top.sp', mimeType: 'text/plain', buffer: Buffer.from('* first\nR1 n 0 1k\n.end\n') },
    { name: 'cells.inc', mimeType: 'text/plain', buffer: Buffer.from(CELLS) }, { name: 'leaf.inc', mimeType: 'text/plain', buffer: Buffer.from(LEAF) }]);
  await idle(page);
  expect(await kept(page)).toMatchObject({ hits: 0, misses: 2 });
  expect((await page.evaluate(() => window.viewer.doc)).spans.length).toBeGreaterThan(0);
});
