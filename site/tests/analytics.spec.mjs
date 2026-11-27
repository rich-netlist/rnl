import { test, expect, BEACON_SCRIPT } from './fixtures.mjs';

// The pages carry Cloudflare Web Analytics' beacon (site/build.py), which the tests answer in the browser (fixtures.mjs):
// its script loads, empty, nothing reports a visit and nothing is an error of the page. The single-file versions,
// read offline, have no beacon.
for (const path of ['/en/index.html', '/zh/tools.html']) {
  test(`analytics: ${path} loads the beacon, answered in the test, and reports nothing`, async ({ page }) => {
    const errors = [], analytics = [];
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('request', (request) => { if (/cloudflareinsights\.com/.test(request.url())) analytics.push(request.url()); });
    const script = page.waitForResponse(BEACON_SCRIPT);
    await page.goto(path);
    const response = await script;
    expect(response.status()).toBe(200);
    expect(await response.text()).toBe('');
    await expect(page.locator(`script[src="${BEACON_SCRIPT}"]`)).toHaveCount(1);
    await page.waitForLoadState('load');
    expect(analytics).toEqual([BEACON_SCRIPT]);
    expect(errors).toEqual([]);
  });
}

test('analytics: a single-file version has no beacon', async ({ page }) => {
  const analytics = [];
  page.on('request', (request) => { if (/cloudflareinsights\.com/.test(request.url())) analytics.push(request.url()); });
  await page.goto('/en/rnl-intro-standalone.html');
  await page.waitForLoadState('load');
  await expect(page.locator('script[src*="cloudflareinsights"]')).toHaveCount(0);
  expect(analytics).toEqual([]);
});
