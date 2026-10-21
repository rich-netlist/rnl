import { test, expect } from '../../tests/viewer/node_modules/@playwright/test/index.mjs';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { publicPackagesAvailable } from '../../scripts/packages.mjs';

for (const language of ['zh', 'en']) {
  test(`public package file label uses its source repository path: ${language}`, async ({ page }) => {
    test.skip(!publicPackagesAvailable(), 'Run python scripts/packages.py download');
    await page.goto(`/${language}/packages.html?series=eda-style&package=eda-style`);
    await expect(page.locator('.gl-meta a').first()).toHaveText('packages/eda-style/0.1/eda-style.rnl');
  });
  test(`standard package file label stays unchanged: ${language}`, async ({ page }) => {
    await page.goto(`/${language}/packages.html?series=standard-0.3&package=rnl-std-style`);
    await expect(page.locator('.gl-meta a').first()).toHaveText('libraries/standard/0.3/rnl-std-style.rnl');
  });
}

for (const language of ['zh', 'en']) for (const theme of ['light', 'dark']) {
  test(`public package source links: ${language} ${theme}`, async ({ page }, info) => {
    test.skip(!process.env.RNL_SITE_SCREENSHOTS || info.project.name !== 'chromium', 'review screenshots use Chromium');
    await page.addInitScript(theme => localStorage.setItem('rnl-intro-theme', theme), theme);
    await page.goto(`/${language}/packages.html?series=eda-style&package=eda-style`);
    await expect(page.locator('.gl-pkg')).toBeVisible();
    await expect(page.locator('.gl-meta a').first()).toHaveAttribute('href', /^https:\/\/github.com\/rich-netlist\/rnl-packages\//);
    await expect(page.locator('.gl-meta a').first()).toHaveText('packages/eda-style/0.1/eda-style.rnl');
    await page.evaluate(() => document.fonts.ready);
    mkdirSync(process.env.RNL_SITE_SCREENSHOTS, { recursive: true });
    await page.screenshot({ path: join(process.env.RNL_SITE_SCREENSHOTS, `${language}-${theme}-packages.png`), fullPage: true });
  });
}
