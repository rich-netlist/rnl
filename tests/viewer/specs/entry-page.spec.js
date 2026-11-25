// The entry page of the web viewer, which reads the file shown alone (entry-page.js): an entry that only includes other
// files has an empty top level, and the card in place of its drawing lists the subcircuits nothing calls, with their
// symbols, files and instances, and whether their files' readings are kept; a click opens one. With one alone, the
// entry opens it at once: when its file holds no record besides its header, that root's file is read in place of its own.
import { test, expect } from '../viewer.js';

test.skip(!process.env.RNL_WEB_URL, 'the web site is not built');

const A = `* a
.subckt alpha p q
R1 p q 1k
.ends
`;
const B = `* b
.subckt beta x y z
R1 x y 1k
R2 y z 1k
.ends
`;

async function open(page, files) {
  await page.route('https://rich-netlist.github.io/rnl-packages/**', route => route.abort());
  // no preloading, which reads files while the page is idle: what is kept is what the test reads
  await page.addInitScript(() => localStorage.setItem('file-preload', 'false'));
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await expect(page.locator('#menubar')).toBeVisible();
  await page.locator('#file-input').setInputFiles(Object.entries(files).map(([name, text]) => ({ name, mimeType: 'text/plain', buffer: Buffer.from(text) })));
  await idle(page);
}
const idle = (page) => page.waitForFunction(() => window.viewer.doc && !document.body.classList.contains('busy'));
const card = (page) => page.locator('#entry-card');
const roots = (page) => card(page).locator('.entry-root');
/** The file read and the level drawn. */
const shown = (page) => page.evaluate(() => ({ file: window.viewer.doc.file, level: window.viewer.doc.levels[window.viewer.level]?.title }));

test('an entry that only includes other files lists the subcircuits nothing calls, and a click opens one', async ({ page }) => {
  await open(page, { 'top.sp': '* top\n.include "a.inc"\n.include "b.inc"\n.end\n', 'a.inc': A, 'b.inc': B });
  expect(await shown(page)).toEqual({ file: 'top.sp', level: 'top.sp' });
  await expect(card(page)).toBeVisible();
  await expect(page.locator('#empty')).toBeHidden();
  await expect(card(page).locator('.entry-heading')).toHaveText('top.sp only includes other files');
  await expect(roots(page).locator('.entry-title')).toHaveText(['alpha', 'beta']);
  await expect(roots(page).locator('.entry-file')).toHaveText(['a.inc', 'b.inc']);
  await expect(roots(page).locator('.entry-instances')).toHaveText(['1 instance', '2 instances']);
  // each with a symbol of its ports, as it gives itself none; neither file has been read
  await expect(roots(page).nth(1).locator('.entry-symbol svg text')).toHaveText(['x', 'y', 'z']);
  await expect(roots(page).locator('.entry-status')).toHaveText(['', '']);
  // the symbols the subcircuits give themselves, which the core tells
  const summaries = await page.evaluate(() => import('./file-reading.js').then(s => s.fileSummaries(['b.inc'])));
  expect(summaries['b.inc']).toMatchObject({ kept: false, took: null, subckts: [{ title: 'beta', ports: ['x', 'y', 'z'], symbol: null }] });
  // a click opens one, as the root of the path
  await roots(page).nth(1).click();
  await idle(page);
  expect(await shown(page)).toEqual({ file: 'b.inc', level: 'beta' });
  await expect(card(page)).toBeHidden();
  // the entry again: the file read is kept now, and its row says so
  await page.locator('#tab-hierarchy').click();
  await page.locator('#index .index-row.top').click();
  await idle(page);
  expect(await shown(page)).toEqual({ file: 'top.sp', level: 'top.sp' });
  await expect(roots(page).locator('.entry-status')).toHaveText(['', 'Kept: opens at once']);
  // not opened again by itself: the entry was shown, not opened
  expect(await shown(page)).toEqual({ file: 'top.sp', level: 'top.sp' });
});

test('an entry that only includes a subcircuit nothing calls opens it at once', async ({ page }) => {
  await open(page, { 'top.sp': '* top\n.include "a.inc"\n.end\n', 'a.inc': A });
  await expect.poll(() => shown(page)).toEqual({ file: 'a.inc', level: 'alpha' });
  await idle(page);
  await expect(card(page)).toBeHidden();
  await expect(page.locator('#level-path .crumb .name, #level-path #level-name')).toHaveText(['alpha']);
});

test('an entry of an earlier version that only includes a subcircuit nothing calls stays, with the notice of its version', async ({ page }) => {
  await open(page, {
    'old.sp': '* old\n* @RNL(type="header") { @VERSION("0.3") }\n.include "cell.inc"\n.end\n',
    'cell.inc': '* cell\n* @RNL(type="header") { @VERSION("0.4") }\n.subckt cell a b\nR1 a b 1k\n.ends\n',
  });
  // time for the root to open, were it opened at once
  await page.waitForTimeout(500);
  await idle(page);
  expect(await shown(page)).toEqual({ file: 'old.sp', level: 'old.sp' });
  // the page of the entry says which viewer reads it, in place of the entry page, and nothing solves it
  await expect(page.locator('#empty .old-version')).toBeVisible();
  await expect(card(page)).toBeHidden();
  expect(await page.evaluate(async () => (await import('./commands.js')).isEnabled('solve'))).toBe(false);
});

/** Count the requests for the symbols of a file (`FileSymbols`) that the page sends its cores, from the start. */
const countSymbols = (page) => page.addInitScript(() => {
  window.symbolRequests = 0;
  const post = Worker.prototype.postMessage;
  Worker.prototype.postMessage = function (message, ...rest) {
    if (typeof message?.request === 'string' && typeof JSON.parse(message.request).symbols === 'string') window.symbolRequests++;
    return post.call(this, message, ...rest);
  };
});
const symbolRequests = (page) => page.evaluate(() => window.symbolRequests);

/** Note, from the start, each Document shown and each file the page asks its cores to read alone. */
const watch = (page) => page.addInitScript(() => {
  window.filesShown = [];
  window.filesRead = [];
  window.addEventListener('document-shown', () => window.filesShown.push(window.viewer.doc.file));
  const post = Worker.prototype.postMessage;
  Worker.prototype.postMessage = function (message, ...rest) {
    const file = typeof message?.request === 'string' ? JSON.parse(message.request).file : undefined;
    if (typeof file === 'string') window.filesRead.push(file);
    return post.call(this, message, ...rest);
  };
});
const watched = (page) => page.evaluate(() => ({ shown: window.filesShown, read: window.filesRead }));

test('an entry whose file draws nothing is read as the file of its one root, with no entry page shown', async ({ page }) => {
  await watch(page);
  await open(page, { 'top.sp': '* top\n* @RNL(type="header") { @VERSION("0.4") }\n.include "a.inc"\n.end\n', 'a.inc': A });
  await expect.poll(() => shown(page)).toEqual({ file: 'a.inc', level: 'alpha' });
  await idle(page);
  // the entry's own file is neither read nor shown
  expect(await watched(page)).toEqual({ shown: ['a.inc'], read: ['a.inc'] });
  await expect(card(page)).toBeHidden();
  await expect(page.locator('#level-path .crumb .name, #level-path #level-name')).toHaveText(['alpha']);
  // the entry is there as before: its page lists the root, whose file's reading is kept
  await page.locator('#tab-hierarchy').click();
  await page.locator('#index .index-row.top').click();
  await idle(page);
  expect(await shown(page)).toEqual({ file: 'top.sp', level: 'top.sp' });
  await expect(roots(page).locator('.entry-title')).toHaveText(['alpha']);
  await expect(roots(page).locator('.entry-status')).toHaveText(['Kept: opens at once']);
});

test('an entry with a record besides its header is read and shown before its one root opens, as before', async ({ page }) => {
  await watch(page);
  await open(page, { 'top.sp': '* top\n* @RNL(type="header") { @VERSION("0.4") }\n* @RNL { }\n.include "a.inc"\n.end\n', 'a.inc': A });
  await expect.poll(() => shown(page)).toEqual({ file: 'a.inc', level: 'alpha' });
  await idle(page);
  expect(await watched(page)).toEqual({ shown: ['top.sp', 'a.inc'], read: ['top.sp', 'a.inc'] });
  await expect(card(page)).toBeHidden();
  await expect(page.locator('#level-path .crumb .name, #level-path #level-name')).toHaveText(['alpha']);
});

test('an entry with its header and another record on one line is shown, as with them on two lines', async ({ page }) => {
  await watch(page);
  // two records on the line of the header: something to draw, not the header alone
  await open(page, { 'top.sp': '* top\n* @RNL(type="header") { @VERSION("0.4") } @RNL { (0, 0) @TEXT("ENTRY") }\n.include "a.inc"\n.end\n', 'a.inc': A });
  // time for the root to open, were it opened at once
  await page.waitForTimeout(500);
  await idle(page);
  expect(await shown(page)).toEqual({ file: 'top.sp', level: 'top.sp' });
  expect(await watched(page)).toEqual({ shown: ['top.sp'], read: ['top.sp'] });
});

test('an entry page left at once for its one root reads no symbols; one that stays reads them', async ({ page }) => {
  await countSymbols(page);
  await open(page, { 'top.sp': '* top\n.include "a.inc"\n.end\n', 'a.inc': A });
  await expect.poll(() => shown(page)).toEqual({ file: 'a.inc', level: 'alpha' });
  await idle(page);
  expect(await symbolRequests(page)).toBe(0);
  // the entry shown again, which stays: its root's row has the symbol the core tells
  await page.locator('#tab-hierarchy').click();
  await page.locator('#index .index-row.top').click();
  await idle(page);
  expect(await shown(page)).toEqual({ file: 'top.sp', level: 'top.sp' });
  await expect(roots(page).locator('.entry-status')).toHaveText(['Kept: opens at once']);
  expect(await symbolRequests(page)).toBe(1);
});

test('an entry page whose one root was not opened, as another open took over, reads the symbols then', async ({ page }) => {
  const C = `* c
.subckt gamma a b
* @RNL { %s = @DEFINE-SYMBOL(box=[-1, -1, 1, 1]) {@LINE {(-1, 0) (1, 0)}; @LINE {(0, -1) (0, 1)}; (-1, 0) @PIN(terminal="a"); (1, 0) @PIN(terminal="b")}; # @SET-SYMBOL %s }
R1 a b 1k
.ends
`;
  await countSymbols(page);
  // the spans and the hints of the entry are held back, so the open is under way with its entry page shown
  await page.addInitScript(() => {
    let held = true;
    const waiting = [];
    window.releaseRest = () => { held = false; for (const go of waiting.splice(0)) go(); };
    const post = Worker.prototype.postMessage;
    Worker.prototype.postMessage = function (message, ...rest) {
      const r = typeof message?.request === 'string' ? JSON.parse(message.request) : null;
      if (held && typeof r?.rest_of === 'string') { waiting.push(() => post.call(this, message, ...rest)); return; }
      return post.call(this, message, ...rest);
    };
  });
  await page.route('https://rich-netlist.github.io/rnl-packages/**', route => route.abort());
  await page.addInitScript(() => localStorage.setItem('file-preload', 'false'));
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await expect(page.locator('#menubar')).toBeVisible();
  // a record besides the header: the entry is read and shown before its one root would open
  await page.locator('#file-input').setInputFiles([
    { name: 'top.sp', mimeType: 'text/plain', buffer: Buffer.from('* top\n* @RNL { }\n.include "c.inc"\n.end\n') },
    { name: 'c.inc', mimeType: 'text/plain', buffer: Buffer.from(C) }]);
  await expect(card(page)).toBeVisible();
  // another open takes over, as one the Viewer API asks for whose file cannot be read, and leaves the entry shown
  await page.evaluate(() => import('./open.js').then((o) => o.beginOpenOperation()));
  await page.evaluate(() => window.releaseRest());
  await page.waitForTimeout(500);
  expect(await shown(page)).toEqual({ file: 'top.sp', level: 'top.sp' });
  await expect(card(page)).toBeVisible();
  // the card shown has the symbol gamma gives itself
  await expect.poll(() => symbolRequests(page)).toBe(1);
  await expect(roots(page).nth(0).locator('.entry-symbol svg')).toHaveCount(1);
  await expect(roots(page).nth(0).locator('.entry-symbol svg.module-symbol')).toHaveCount(0);
});

test('an entry with something to draw, or a subcircuit of its own nothing calls, has no entry page', async ({ page }) => {
  await open(page, { 'top.sp': '* top\n.include "a.inc"\n.subckt own p\nR1 p 0 1k\n.ends\n.end\n', 'a.inc': A });
  // the subcircuit of its own file opens, as before
  expect(await shown(page)).toEqual({ file: 'top.sp', level: 'own' });
  await expect(card(page)).toBeHidden();
});

test('a subcircuit that gives itself a symbol shows that symbol', async ({ page }) => {
  const C = `* c
.subckt gamma a b
* @RNL { %s = @DEFINE-SYMBOL(box=[-1, -1, 1, 1]) {@LINE {(-1, 0) (1, 0)}; @LINE {(0, -1) (0, 1)}; (-1, 0) @PIN(terminal="a"); (1, 0) @PIN(terminal="b")}; # @SET-SYMBOL %s }
R1 a b 1k
.ends
`;
  await open(page, { 'top.sp': '* top\n.include "a.inc"\n.include "c.inc"\n.end\n', 'a.inc': A, 'c.inc': C });
  await expect(roots(page).locator('.entry-title')).toHaveText(['alpha', 'gamma']);
  // alpha's is a box of its ports; gamma's, the symbol it gives itself, as the core draws it
  await expect(roots(page).nth(0).locator('.entry-symbol svg.module-symbol')).toHaveCount(1);
  await expect(roots(page).nth(1).locator('.entry-symbol svg')).toHaveCount(1);
  await expect(roots(page).nth(1).locator('.entry-symbol svg.module-symbol')).toHaveCount(0);
  await expect(roots(page).nth(1).locator('.entry-symbol svg line, .entry-symbol svg path, .entry-symbol svg polyline').first()).toBeAttached();
});
