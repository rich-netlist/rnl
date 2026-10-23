import { expect, test } from "../viewer.js";
import { screenshotEvidence, assertPainted } from "../../../apps/desktop/tests/visual.mjs";

test("native smoke pixel checks reject missing content even with a valid DOM and painted grid", async ({ page, viewer }) => {
  await viewer.open("04-hierarchy");
  await expect(page.locator('#startup')).toHaveCount(0);
  const evidence = async () => page.evaluate(screenshotEvidence, (await page.screenshot()).toString('base64'));
  assertPainted(await evidence());
  await page.emulateMedia({colorScheme:'dark'});
  assertPainted(await evidence());
  for (const selector of ['.view-tabs button', '#source .code', '#source', '#canvas > svg']) {
    const sheet = await page.addStyleTag({content:`${selector} {visibility:hidden !important}`});
    try {
      await expect(page.locator('#canvas > svg')).toHaveCount(1);
      const blank = await evidence();
      expect(() => assertPainted(blank)).toThrow(/Screenshot has no painted content/);
    } finally { await sheet.evaluate(el => el.remove()); }
  }
  // The DOM can report visible while a compositor supplies a flat layer of the wrong colour.
  const solid = await page.evaluate(async png => {
    const c=document.createElement('canvas'); c.width=innerWidth; c.height=innerHeight;
    const image=new Image(); image.src=`data:image/png;base64,${png}`; await image.decode();
    const ctx=c.getContext('2d'); ctx.drawImage(image,0,0,c.width,c.height);
    const r=document.getElementById('source').getBoundingClientRect();
    ctx.fillStyle='#232327'; ctx.fillRect(r.x,r.y,r.width,r.height);
    return c.toDataURL('image/png').split(',')[1];
  }, (await page.screenshot()).toString('base64'));
  const blank = await page.evaluate(screenshotEvidence, solid);
  expect(blank.regions.find(r=>r.selector==='#source').density).toBeGreaterThan(0.6);
  expect(() => assertPainted(blank)).toThrow(/Screenshot has no painted content in #source/);
});

test("loading reports stages without a percentage and clears when the document is painted", async ({ page, viewer }) => {
  await viewer.open();
  await page.evaluate(async () => {
    const { shell } = await import('./shell.js');
    const build = shell.build;
    shell.build = async (source, edits, progress) => {
      await new Promise(resolve => window.finishReading = resolve);
      progress('parsing');
      await new Promise(resolve => window.finishParsing = resolve);
      return build(source, edits);
    };
    window.loadingStages = [];
    new MutationObserver(() => window.loadingStages.push(document.getElementById('loading').dataset.stage))
      .observe(document.getElementById('loading'), { attributes: true, attributeFilter: ['data-stage'] });
    window.fakeShell.open('04-hierarchy');
  });
  const loading = page.locator('#loading');
  await expect(loading).toBeVisible();
  await expect(loading).toHaveAttribute('data-stage', 'reading');
  await expect(loading.getByRole('progressbar')).not.toHaveAttribute('aria-valuenow');
  await expect(page.locator('#app')).toHaveAttribute('aria-busy', 'true');
  await page.waitForFunction(() => window.finishReading);
  await page.evaluate(() => window.finishReading());
  await expect(loading).toHaveAttribute('data-stage', 'parsing');
  await page.evaluate(() => window.finishParsing());
  await viewer.idle();
  await expect(loading).toBeHidden();
  await expect(page.locator('#app')).not.toHaveAttribute('aria-busy');
  expect(await page.evaluate(() => window.loadingStages)).toContain('displaying');
});

test("a failed open clears its loading indicator and still reports the error", async ({ page, viewer }) => {
  await viewer.open();
  await page.evaluate(() => { window.fakeShell.fail = 'cannot read fixture'; window.fakeShell.open('04-hierarchy'); });
  await expect(page.locator('#diags')).toContainText('cannot read fixture');
  await expect(page.locator('#loading')).toBeHidden();
  await expect(page.locator('#app')).not.toHaveAttribute('aria-busy');
});
