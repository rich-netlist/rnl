import { test, expect } from '../viewer.js';

for (const keyboard of [false, true]) test(`include links open independent files using ${keyboard ? 'Enter' : 'click'}; Shift keeps this viewer`, async ({page,viewer}) => {
  await viewer.open('fold-sections');
  await page.evaluate(() => {
    window.fakeShell.fileSources = {
      'fold-models.inc': {name:'fold-models.inc', fixture:'fold-models'},
      'fold-sections.inc': {name:'fold-sections.inc', fixture:'fold-sections-include'},
    };
  });
  const links = page.locator('#source .file:not([hidden]) .include-link');
  await expect(links).toHaveCount(2);
  const link = links.nth(keyboard ? 0 : 1);
  const name = keyboard ? 'fold-sections.inc' : 'fold-models.inc';
  if (keyboard) { await link.focus(); await page.keyboard.press('Shift+Enter'); }
  else await link.click({modifiers:['Shift']});
  const session = await page.evaluate(() => window.fakeShell.calls.findLast(c => c[0] === 'newWindow')[1]);
  expect(session.entry.name).toBe(name);
  expect(session.path).toEqual([]);
  expect(session.netlist).toBe(true);
  await expect(page.locator('#netlist-file')).toHaveText(/fold-sections\.sp$/);
  await viewer.build(() => keyboard ? page.keyboard.press('Enter') : link.click());
  await expect(page.locator('#netlist-file')).toHaveText(new RegExp(name.replace('.', '\\.')));
  await expect(viewer.line(keyboard ? 2 : 1)).toHaveText(keyboard ? /subckt active_cell/ : /\.param bias=1.2/);
  expect(await page.evaluate(() => window.viewer.doc.files.some(f => f.name === 'fold-sections.sp'))).toBe(false);
});

test('Failed statements search follows same-file circuit navigation', async ({page,viewer}) => {
  await viewer.open('symbol-inspector');
  await page.locator('#show-failed').uncheck();
  await page.locator('#source').focus();
  await page.keyboard.press('Control+f');
  await page.locator('#netlist-query').fill('Xunplaced a');
  await expect(page.locator('#netlist-matches')).toHaveText('No matches');
  await page.evaluate(async () => (await import('./subcircuits.js')).enterRoot(window.viewer.doc.levels.findIndex(l => l.title === 'leaf')));
  await expect(page.locator('#netlist-matches')).toHaveText('1 / 1');
  await page.evaluate(async () => (await import('./subcircuits.js')).enterRoot(0));
  await expect(page.locator('#netlist-matches')).toHaveText('No matches');
});

test('filtered virtual source retains its logical position on reload and repeated level painting', async ({page,viewer}) => {
  await viewer.open('virtual');
  await page.locator('#show-failed').uncheck();
  await page.evaluate(async () => (await import('./text.js')).revealLine(0,1800,false));
  const first = () => page.locator('#source').evaluate(async s => {
    const {fileViews,fileShown} = await import('./text.js');
    const v = fileViews[fileShown];
    return v.lineAt(Math.max(0,Math.floor((s.scrollTop-4+1)/v.height)));
  });
  const before = await first();
  expect(before).toBeGreaterThan(1000);
  await viewer.build(() => page.keyboard.press('F5'));
  expect(await first()).toBe(before);
  await page.evaluate(async () => (await import('./text.js')).markUnplaced(window.viewer.level));
  expect(await first()).toBe(before);
  // A fractional row height rounded down by WebKit must not choose the preceding row as
  // anchor when that row becomes inactive. This exercises remapping independently of reload.
  await page.evaluate(async () => {
    const {fileViews} = await import('./text.js');
    const v = fileViews[0], source = document.getElementById('source');
    source.scrollTop = Math.floor(4 + v.rowOf(103)*v.height);
    v.setInactive(new Map([...v.inactive, [103, 'Not drawn']]));
  });
  expect(await first()).toBe(103);
});

test('Failed statements filters undrawn rows and copy while RNL and semantic filters compose', async ({page,viewer}) => {
  await viewer.open('symbol-inspector');
  await expect(viewer.line(19)).toHaveClass(/not-drawn/);
  await page.locator('#show-failed').uncheck();
  await expect(viewer.line(19)).toHaveCount(0);
  await page.locator('#source').focus();
  await page.keyboard.press('Control+a');
  const copied = await page.locator('#source').evaluate(el => {
    const event = new ClipboardEvent('copy',{bubbles:true,cancelable:true,clipboardData:new DataTransfer()});
    el.dispatchEvent(event); return event.clipboardData.getData('text/plain');
  });
  expect(copied).not.toContain('Xunplaced a');
  expect(copied).toContain('Xparent a');
  await page.keyboard.press('Escape');
  await page.locator('#rnl-only').check();
  await expect(viewer.line(17)).toHaveCount(0);
  await page.locator('#semantics-only').check();
  expect(await page.locator('#source .ln').evaluateAll(es => es.map(e => Number(e.dataset.line)))).toEqual([12,31]);
});
