import { expect, test } from '../viewer.js';

test('large Problems lists load on demand, retain total counts and reveal later diagnostics', async ({ page, viewer }) => {
  await viewer.open('04-hierarchy');
  await page.evaluate(async () => {
    const list = Array.from({length: 1500}, (_, i) => ({severity: 'warning', message: `Warning ${i}`,
      file: '04-hierarchy.sp', file_index: 0, line: 8, col: 0, origin: ''}));
    (await import('./problems.js')).renderDiagnostics(list);
  });
  await expect(page.locator('#counts')).toContainText('1500 warnings');
  await expect(page.locator('#diags .diag')).toHaveCount(100);
  await page.locator('#problems-body').evaluate(el => { el.scrollTop = el.scrollHeight; });
  await expect(page.locator('#diags .diag')).toHaveCount(200);
  await page.locator('#diags .diag').nth(199).click();
  await expect(viewer.line(8)).toHaveClass(/flash/);
  // Every item remains reachable; each click loads another bounded batch.
  await page.evaluate(() => {
    for (let i = 0; i < 15; i++) document.querySelector('.diagnostics-more button')?.click();
  });
  await expect(page.locator('#diags .diag')).toHaveCount(1500);
  await expect(page.locator('.diagnostics-more')).toHaveCount(0);
  await expect(page.locator('#diags .diag').last()).toContainText('Warning 1499');
  await page.evaluate(async () => (await import('./problems.js')).renderDiagnostics([]));
  await expect(page.locator('#diags')).toHaveText('No errors, warnings or hints.');
});

test('token-heavy files below a thousand lines also keep a bounded viewport', async ({ page, viewer }) => {
  await viewer.open('04-hierarchy');
  await page.evaluate(async () => {
    const {doc} = await import('./state.js');
    const text = doc.files[0].text + '\n' + Array.from({length: 600}, (_, i) => `* dense line ${i}`).join('\n');
    (await import('./open.js')).showBuilt({...doc, files: [{...doc.files[0], text}, ...doc.files.slice(1)]}, false);
  });
  expect(await page.locator('#source .ln').count()).toBeLessThan(180);
  await page.evaluate(async () => (await import('./text.js')).revealLine(0, 450, false));
  await expect(viewer.line(450)).toBeVisible();
  expect(await page.locator('#source .ln').count()).toBeLessThan(180);
});

test('finishing a quick load prevents a delayed loading flash', async ({ page, viewer }) => {
  await viewer.open('04-hierarchy');
  await page.evaluate(async () => {
    const {startLoading} = await import('./loading.js');
    const progress = startLoading();
    await progress.stage('cached');
    progress.finish();
  });
  await page.waitForTimeout(180);
  await expect(page.locator('#loading')).toBeHidden();
  await expect(page.locator('#app')).not.toHaveAttribute('aria-busy');
});
