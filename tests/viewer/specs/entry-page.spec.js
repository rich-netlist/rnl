// The entry page of the web viewer, which reads the file shown alone (entry-page.js): an entry that only includes other
// files has an empty top level, and the card in place of its drawing lists the subcircuits nothing calls, with their
// symbols, files and instances, and whether their files' readings are kept; a click opens one. With one alone, the
// entry opens it at once.
import { test, expect } from '@playwright/test';

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
