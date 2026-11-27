// What every page test of the site starts from: Playwright's test and expect, with the pages' analytics (BEACON of
// site/build.py) answered in the browser. The tests load the pages from a local address, where the beacon would report
// a visit of the test, or be refused across origins, which WebKit reports as an error of the page: its script is
// answered with an empty module, so that it loads and sends nothing, and a report, should one still be sent, with 204.
import { test as base, expect } from "../../tests/viewer/node_modules/@playwright/test/index.mjs";

export const BEACON_SCRIPT = "https://static.cloudflareinsights.com/beacon.min.js";
const REPORT = /^https:\/\/([^/]+\.)?cloudflareinsights\.com\/cdn-cgi\/rum/;

/** Answer the analytics in `context`, a browser context: its script empty, its reports with nothing. */
export async function quietAnalytics(context) {
  await context.route(BEACON_SCRIPT, (route) => route.fulfill({ status: 200, contentType: "application/javascript", body: "" }));
  await context.route(REPORT, (route) => route.fulfill({ status: 204, body: "" }));
}

export const test = base.extend({
  context: async ({ context }, use) => {
    await quietAnalytics(context);
    await use(context);
  },
});

export { expect };
