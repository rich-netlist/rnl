import { test, expect } from '../../tests/viewer/node_modules/@playwright/test/index.mjs';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

const datesOf = timeline => timeline.locator('time').evaluateAll(times => times.map(time => time.textContent.match(/\d{4}-\d{2}-\d{2}/g).at(-1)));

for (const language of ['zh', 'en']) for (const theme of ['light', 'dark']) for (const width of [1440, 390]) {
  test(`timeline: the homepage shows the latest few entries, ${language} ${theme} at ${width}px`, async ({ page }, info) => {
    await page.setViewportSize({ width, height: 1000 });
    await page.addInitScript(theme => localStorage.setItem('rnl-intro-theme', theme), theme);
    await page.goto(`/${language}/index.html#status`);
    const timeline = page.locator('#status .tl');
    const published = timeline.locator(':scope > li:has(time)');
    await expect(published).toHaveCount(3);
    await expect(timeline.locator('.now')).toHaveCount(1);
    await expect(published.last()).toHaveClass('now');
    await expect(timeline).toContainText('2026-10-25');
    await expect(timeline.locator('.next')).toHaveCount(2);
    // the older entries are on the full timeline, which the list links to, between the entries and what comes next
    const more = timeline.locator('.tl-more a');
    await expect(more).toHaveCount(1);
    await expect(more).toHaveText(language === 'zh' ? '完整时间线' : 'Full timeline');
    await expect(more).toHaveAttribute('href', 'timeline.html');
    expect(await timeline.locator(':scope > li').evaluateAll(items => items.map(item => item.className))).toEqual(
      ['', '', 'now', 'tl-more', 'tl-sep', 'next', 'next']);
    const dates = await datesOf(timeline);
    expect(dates.every((date, index) => !index || dates[index - 1] <= date)).toBe(true);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width + 1);
    if (process.env.RNL_TIMELINE_SCREENSHOTS && info.project.name === 'chromium') {
      await page.evaluate(() => document.fonts.ready);
      mkdirSync(process.env.RNL_TIMELINE_SCREENSHOTS, { recursive: true });
      await timeline.screenshot({
        path: join(process.env.RNL_TIMELINE_SCREENSHOTS, `home-${language}-${theme}-${width}.png`),
        style: '.topbar { visibility: hidden !important; }',
      });
    }
  });

  test(`timeline: the full timeline has every entry, the latest first, ${language} ${theme} at ${width}px`, async ({ page }, info) => {
    await page.setViewportSize({ width, height: 1000 });
    await page.addInitScript(theme => localStorage.setItem('rnl-intro-theme', theme), theme);
    await page.goto(`/${language}/timeline.html`);
    const timeline = page.locator('#timeline .tl');
    const entries = timeline.locator(':scope > li');
    expect(await entries.count()).toBeGreaterThan(3);
    await expect(timeline.locator('.now')).toHaveCount(1);
    await expect(entries.first()).toHaveClass('now');
    // what the old homepage listed is all here: the first and the later releases, with their notes
    await expect(timeline).toContainText('0.1.2');
    await expect(timeline).toContainText('0.1.3');
    await expect(timeline).toContainText('0.1.4');
    await expect(timeline).toContainText('preview.15');
    await expect(timeline.locator('a[href*="viewer-v0.1.2"]')).toHaveCount(1);
    await expect(timeline.locator('.next, .tl-sep, .tl-more')).toHaveCount(0);
    const dates = await datesOf(timeline);
    expect(dates.every((date, index) => !index || dates[index - 1] >= date)).toBe(true);
    // the page is part of the site: the language switch keeps it, and the menu does not mark another page
    await expect(page.locator('[data-lang-link]').first()).toHaveAttribute('href', /timeline\.html/);
    await expect(page.locator('nav.pages a[aria-current="page"]')).toHaveCount(0);
    await expect(page.locator('a[href="./#status"]').first()).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width + 1);
    if (process.env.RNL_TIMELINE_SCREENSHOTS && info.project.name === 'chromium') {
      await page.evaluate(() => document.fonts.ready);
      mkdirSync(process.env.RNL_TIMELINE_SCREENSHOTS, { recursive: true });
      await page.mouse.move(width - 2, 0);
      await page.screenshot({ path: join(process.env.RNL_TIMELINE_SCREENSHOTS, `full-${language}-${theme}-${width}.png`) });
    }
  });
}
