import { test, expect } from '../../tests/viewer/node_modules/@playwright/test/index.mjs';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

for (const language of ['zh', 'en']) for (const theme of ['light', 'dark']) for (const width of [1440, 390]) {
  test(`timeline: ${language} ${theme} at ${width}px`, async ({ page }, info) => {
    await page.setViewportSize({ width, height: 1000 });
    await page.addInitScript(theme => localStorage.setItem('rnl-intro-theme', theme), theme);
    await page.goto(`/${language}/index.html#status`);
    const timeline = page.locator('#status .tl');
    const published = timeline.locator(':scope > li:has(time)');
    await expect(timeline.locator('.now')).toHaveCount(1);
    await expect(published.last()).toHaveClass('now');
    await expect(timeline).toContainText('0.1.2');
    await expect(timeline).toContainText('0.1.3');
    await expect(timeline).toContainText('0.1.4');
    await expect(timeline).toContainText('preview.15');
    await expect(timeline).toContainText('2026-10-22');
    await expect(timeline.locator('.next')).toHaveCount(2);
    await expect(timeline.locator('a[href*="viewer-v0.1.2"]')).toHaveCount(1);
    expect(await timeline.locator('time').evaluateAll(times => {
      const dates = times.map(time => time.textContent.match(/\d{4}-\d{2}-\d{2}/g).at(-1));
      return dates.every((date, index) => !index || dates[index - 1] <= date);
    })).toBe(true);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width + 1);
    if (process.env.RNL_TIMELINE_SCREENSHOTS && info.project.name === 'chromium') {
      await page.evaluate(() => document.fonts.ready);
      mkdirSync(process.env.RNL_TIMELINE_SCREENSHOTS, { recursive: true });
      await timeline.screenshot({
        path: join(process.env.RNL_TIMELINE_SCREENSHOTS, `${language}-${theme}-${width}.png`),
        // The full timeline is taller than the viewport; don't let the sticky header cover its first entry.
        style: '.topbar { visibility: hidden !important; }',
      });
    }
  });
}
