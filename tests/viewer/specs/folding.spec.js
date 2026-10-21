import { test, expect, showSource } from '../viewer.js';

const reveal = (page, line) => page.evaluate(async line => (await import('./text.js')).revealLine(0, line, false, true), line);
const copy = page => page.locator('#source').evaluate(source => {
  const e = new ClipboardEvent('copy', { bubbles: true, cancelable: true, clipboardData: new DataTransfer() });
  source.dispatchEvent(e);
  return e.clipboardData.getData('text/plain');
});

test('unselected library sections can be folded and commented declarations are ignored', async ({ page, viewer }) => {
  await viewer.open('fold-sections');
  await showSource(page, 1);
  await expect(page.locator('#source .file:not([hidden]) .fold-toggle')).toHaveCount(2);
  await viewer.line(8).locator('.fold-toggle').click();
  await expect(viewer.line(9)).toHaveCount(0);
  await expect(viewer.line(11)).toBeVisible();
  await viewer.line(8).locator('.fold-toggle').click();
  await expect(viewer.line(9)).toBeVisible();
});

test('source navigation can still reach and edit a parameter/model-only include', async ({ page, viewer }) => {
  await viewer.open('fold-sections');
  await showSource(page, 'fold-models.inc');
  await expect(page.locator('#source-file')).toHaveText('fold-models.inc');
  await expect(page.locator('#netlist-file')).toHaveText('fold-sections.sp');
  await page.keyboard.press('Control+q');
  await expect(page.locator('#source textarea:visible')).toHaveValue(/\.param bias=1.2/);
});

test('editing an include keeps the logical position of a hidden folded root after rebuilding', async ({ page, viewer }) => {
  await viewer.open('virtual');
  const from = await page.evaluate(() => window.viewer.doc.levels.find(l => l.lines?.[0] === 0).lines[1]);
  await reveal(page, from);
  await viewer.line(from).locator('.fold-toggle').click();
  const last = await page.evaluate(() => window.viewer.doc.files[0].text.split(/\r?\n/).length - 100);
  await reveal(page, last);
  const first = () => page.evaluate(async () => {
    const v = (await import('./text.js')).fileViews[0];
    return v.lineAt(Math.max(0, Math.floor((document.getElementById('source').scrollTop - 4 + 1) / v.lineHeight())));
  });
  const original = await first();
  await page.keyboard.press('Control+q');
  const before = await page.locator('#source textarea:visible').evaluate((a, line) => {
    const height = parseFloat(getComputedStyle(a).lineHeight);
    a.scrollTop = 4 + (line - 50) * height;
    a.dispatchEvent(new Event('scroll'));
    return Math.max(0, Math.floor((a.scrollTop - 4 + 1) / height));
  }, original);
  await showSource(page, 1);
  // A queued browser scroll from the now-hidden root must not discard its saved position.
  await page.locator('#source .editor[hidden] textarea').first().evaluate(a => {
    a.scrollTop = 0;
    a.dispatchEvent(new Event('scroll'));
  });
  const area = page.locator('#source textarea:visible');
  await area.fill((await area.inputValue()) + '\n* changed include\n');
  await viewer.build(() => page.keyboard.press('Control+Enter'));
  // The root can stay lazy during the include's rebuild.
  expect(await page.evaluate(async () => !!(await import('./text.js')).fileViews[0]._model)).toBe(false);
  expect(await page.evaluate(async () => (await import('./text.js')).fileViews[0].pendingLine)).toBe(before);
  // A second editing session must not overwrite that deferred logical position with a row offset.
  await page.keyboard.press('Control+q');
  await page.keyboard.press('Control+q');
  await viewer.build(() => page.evaluate(() => window.viewer.command('toggle-auto-include')));
  await page.locator('#netlist-file').click();
  expect(await first()).toBe(before);
});

for (const fixture of ['04-hierarchy', 'inverter-chain', 'virtual']) {
  test(`${fixture}: folds keep original line numbers, full-text copy, find and edit positions`, async ({ page, viewer }) => {
    await viewer.open(fixture);
    const [f, from, to] = await page.evaluate(() => window.viewer.doc.levels.find(l => l.lines && l.lines[0] === 0 && l.lines[2] > l.lines[1] + 1).lines);
    await reveal(page, from);
    const toggle = viewer.line(from).locator('.fold-toggle');
    await expect(toggle).toHaveAttribute('aria-expanded', 'true');
    await expect(viewer.line(from)).toHaveClass(/subckt-start/);
    await toggle.click();
    await expect(toggle).toHaveAttribute('aria-expanded', 'false');
    await expect(viewer.line(from + 1)).toHaveCount(0);
    await expect(viewer.line(to)).toHaveCount(0);
    await expect(viewer.line(to + 1)).toBeAttached();
    await page.locator('#source').focus();
    await page.keyboard.press('Control+a');
    expect(await copy(page)).toBe(await page.evaluate(() => window.viewer.doc.files[0].text.replace(/\r\n/g, '\n')));
    await page.keyboard.press('Escape');
    // Search sees hidden lines and expands the containing block.
    const query = await page.evaluate(line => window.viewer.doc.files[0].text.split(/\r?\n/)[line - 1].trim(), from + 1);
    await page.keyboard.press('Control+f');
    await page.locator('#netlist-query').fill(query);
    await expect(toggle).toHaveAttribute('aria-expanded', 'true');
    await expect(viewer.line(from + 1)).toBeAttached();
    await page.locator('#netlist-query').press('Escape');
    await reveal(page, from);
    await toggle.click();
    // Switching files keeps fold state. Reload of unchanged text keeps it too.
    if (await page.evaluate(() => window.viewer.doc.files.length > 1)) {
      await showSource(page, 1);
      await page.locator('#netlist-file').click();
      await expect(toggle).toHaveAttribute('aria-expanded', 'false');
    }
    await viewer.build(() => page.evaluate(() => window.viewer.command('reload')));
    await reveal(page, from);
    await expect(toggle).toHaveAttribute('aria-expanded', 'false');
    await reveal(page, to + 1);
    const before = await page.evaluate(async () => {
      const { fileViews } = await import('./text.js');
      const v = fileViews[0];
      return v.lineAt(Math.max(0, Math.floor((document.getElementById('source').scrollTop - 4 + 1) / v.lineHeight())));
    });
    await page.keyboard.press('Control+q');
    const area = page.locator('#source textarea:visible');
    await expect(area).toBeVisible();
    const editing = await area.evaluate(a => Math.max(0, Math.floor((a.scrollTop - 4 + 1) / parseFloat(getComputedStyle(a).lineHeight))));
    expect(editing).toBe(before);
    await page.keyboard.press('Control+q');
    if (fixture === 'virtual') expect(await page.locator('#source .file:not([hidden]) .ln').count()).toBeLessThan(300);
  });
}

test('unvisited includes have no tokenized model or rows; diagnostics still reveal their source', async ({ page, viewer }) => {
  await viewer.open('04-hierarchy');
  expect(await page.evaluate(async () => {
    const v = (await import('./text.js')).fileViews[1];
    return { initialized: !!v._model, rows: v.rendered.size };
  })).toEqual({ initialized: false, rows: 0 });
  await page.locator('#tab-hierarchy').click();
  await page.locator('#index .index-row', { hasText: 'rdiv' }).click();
  await page.locator('#tab-netlist').click();
  await expect(viewer.line(2).locator('.fold-toggle')).toBeVisible();
  await viewer.line(2).locator('.fold-toggle').click();
  await page.evaluate(async () => (await import('./text.js')).revealLine(1, 3, true));
  await expect(viewer.line(3)).toHaveClass(/flash/);
  await expect(viewer.line(2).locator('.fold-toggle')).toHaveAttribute('aria-expanded', 'true');
  await viewer.build(() => page.evaluate(() => window.fakeShell.open({ name: '04-hierarchy', fixture: '04-hierarchy' })));
  expect(await page.evaluate(async () => !!(await import('./text.js')).fileViews[1]._model)).toBe(false);
});
