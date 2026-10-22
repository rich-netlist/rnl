import { expect, test } from '../viewer.js';

test('anonymous usage selection reveals its source even without a named token', async ({page, viewer}) => {
  await viewer.open('usage-anonymous');
  const u = await page.evaluate(() => window.viewer.doc.levels[0].usages.find(u => u.definition === '$PAIR'));
  await expect(page.locator(`#source .ln[data-line="${u.first_line}"]`)).not.toBeInViewport();
  await page.keyboard.press('5');
  expect(u.id.startsWith('@')).toBe(true);
  await page.locator(`.usage-row[data-object="${u.object}"] .usage-name`).click();
  await page.keyboard.press('2');
  await expect(page.locator('#source .hl-line')).toContainText(['#R1(left) #R2(right) @AS $PAIR']);
  const [line, source] = await Promise.all([page.locator('#source .hl-line').boundingBox(), page.locator('#source').boundingBox()]);
  expect(line.y).toBeGreaterThanOrEqual(source.y);
  expect(line.y + line.height).toBeLessThanOrEqual(source.y + source.height);
});

test('large usage groups defer hidden DOM and expose their names in bounded batches', async ({page, viewer}) => {
  await viewer.open('usage-anonymous');
  await page.evaluate(() => {
    const d = window.viewer.doc, l = d.levels[0], u = l.usages.find(u => u.definition === '$PAIR');
    l.usages = Array.from({length: 350}, (_, i) => {
      const object = d.objects.length;
      d.objects.push({...d.objects[u.object], name: `%pair${i}`});
      return {...u, object, id: `%pair${i}`};
    });
    window.dispatchEvent(new Event('viewer-level'));
  });
  await expect(page.locator('#usage-list .usage-row')).toHaveCount(0);
  await page.keyboard.press('5');
  await expect(page.locator('.usage-expand')).toHaveAttribute('aria-expanded', 'false');
  await page.locator('.usage-expand').click();
  await expect(page.locator('.usage-row')).toHaveCount(100);
  await page.locator('.usage-more').click();
  await expect(page.locator('.usage-row')).toHaveCount(200);
});

test('selected semantic usage screenshot for visual review', async ({page,viewer}, info) => {
  await viewer.open('03-five-transistor-ota');
  await page.keyboard.press('5');
  const u = await page.evaluate(() => window.viewer.doc.levels[0].usages.find(u => u.definition === '$DIFFPAIR'));
  await page.locator(`.usage-row[data-object="${u.object}"] .usage-name`).click();
  const visible = await page.locator('#canvas .usage-frame').evaluateAll(es => es.map(e => Number(e.dataset.usage)));
  const drawn = await page.evaluate(() => window.viewer.doc.levels[0].usages.filter(u => u.has_drawing).map(u => u.object));
  expect(drawn.some(o => visible.includes(o))).toBe(false);
  await page.keyboard.press('Control+r'); // The selected usage remains visible while all others are hidden.
  await page.evaluate(() => window.viewer.command('background-dark'));
  await page.screenshot({path: info.outputPath('usage-dark-selected.png')});
  await page.evaluate(() => window.viewer.command('background-light'));
  await page.screenshot({path: info.outputPath('usage-light-selected.png')});
  await expect(page.locator('.usage-frame.selected')).toBeVisible();
});
