import { showSource } from "../viewer.js";
import { expect, test } from "../viewer.js";

const reveal = (page, file, line) => page.evaluate(async ({ file, line }) => {
  (await import('./text.js')).revealLine(file, line, false);
}, { file, line });
const lineOf = (page, fragment, file = 0) => page.evaluate(({ fragment, file }) =>
  window.viewer.doc.files[file].text.split(/\r?\n/).findIndex(s => s.includes(fragment)) + 1, { fragment, file });
const copy = (page) => page.locator("#source").evaluate(source => {
  const event = new ClipboardEvent('copy', {
    bubbles: true, cancelable: true, clipboardData: new DataTransfer(),
  });
  source.dispatchEvent(event);
  // Firefox creates its own data store instead of using the constructor argument.
  // Read the same store as the copy handler (Mozilla bug 2027025).
  return event.clipboardData.getData('text/plain');
});

test("offscreen object, diagnostic, scope and unplaced marks survive recycling", async ({ page, viewer }) => {
  await viewer.open("virtual");
  const x2 = await lineOf(page, "X2 b c");
  await viewer.click("X2");
  await expect(viewer.line(x2)).toHaveClass(/hl-line/);
  await expect.poll(() => viewer.within(viewer.line(x2))).toBe(true);
  await page.locator("#source").evaluate(s => { s.scrollTop = 0; });
  await expect(viewer.line(x2)).toHaveCount(0);
  await reveal(page, 0, x2);
  await expect(viewer.line(x2)).toHaveClass(/hl-line/);

  const problem = await page.evaluate(() => window.viewer.doc.diagnostics.find(d => d.file_index === 0 && d.message.includes("missing")));
  expect(problem).toBeTruthy();
  await page.locator("#diags li.located", { hasText: problem.message }).first().click();
  await expect(viewer.line(problem.line)).toHaveClass(/d-error/);
  await expect(viewer.line(problem.line)).toHaveClass(/flash/);
  await page.locator("#source").evaluate(s => { s.scrollTop = 0; });
  await expect(viewer.line(problem.line)).toHaveCount(0);
  await reveal(page, 0, problem.line);
  await expect(viewer.line(problem.line)).toHaveClass(/d-error/);
  const unplaced = await lineOf(page, "Runplaced");
  await reveal(page, 0, unplaced);
  await expect(viewer.line(unplaced)).toHaveClass(/not-drawn/);

  await viewer.click("X2");
  await page.keyboard.press("e");
  const inside = await lineOf(page, "M1 out in");
  await reveal(page, 0, inside);
  await expect(viewer.line(inside)).toHaveClass(/in-scope/);
  await reveal(page, 0, unplaced);
  await expect(viewer.line(unplaced)).not.toHaveClass(/in-scope/);
});

test("random access preserves multiline syntax, per-file scroll and edit position", async ({ page, viewer }) => {
  await viewer.open("virtual");
  const comment = await lineOf(page, "inside RNL comment 100");
  await reveal(page, 0, comment);
  await expect(viewer.line(comment).locator(".s-comment")).toContainText("inside RNL comment 100");
  const first = () => page.locator("#source").evaluate(async s => {
    const { sourceLineHeight, fileShown } = await import('./text.js');
    return Math.max(0, Math.floor((s.scrollTop - 4 + 1) / sourceLineHeight(fileShown)));
  });
  const before = await first();
  await showSource(page, 1);
  const included = await lineOf(page, "R2 top tap", 1);
  await reveal(page, 1, included || 1105);
  const other = await first();
  await showSource(page, 0);
  expect(await first()).toBe(before);
  await page.locator('#tab-hierarchy').click();
  await page.locator('#tab-netlist').click();
  expect(await first()).toBe(before);
  await page.keyboard.press('Control+b');
  await page.keyboard.press('Control+b');
  expect(await first()).toBe(before);
  await page.keyboard.press("Control+q");
  const area = page.locator("#source textarea:visible");
  await expect(area).toBeVisible();
  const editing = await area.evaluate(a => Math.max(0, Math.floor((a.scrollTop - 4 + 1) / parseFloat(getComputedStyle(a).lineHeight))));
  expect(editing).toBe(before);
  await page.keyboard.press("Control+q");
  await expect(area).toHaveCount(0);
  expect(await first()).toBe(before);
  await showSource(page, 1);
  expect(await first()).toBe(other);
});

test("select all and copy include offscreen text without adding offscreen rows", async ({ page, viewer }) => {
  await viewer.open("virtual");
  await page.locator("#source").focus();
  await page.keyboard.press("Control+a");
  const copied = await copy(page);
  expect(copied).toBe(await page.evaluate(() => window.viewer.doc.files[0].text.replace(/\r\n/g, '\n')));
  await page.locator("#source").evaluate(s => { s.scrollTop = s.scrollHeight; });
  await expect(viewer.line(1)).toHaveCount(0);
  const again = await copy(page);
  expect(again).toBe(copied);
  expect(await page.locator("#source .file:not([hidden]) .ln").count()).toBeLessThan(300);
});

test("find searches the full file and follows file changes outside the DOM window", async ({ page, viewer }) => {
  await viewer.open("virtual");
  await page.locator("#source").focus();
  await page.keyboard.press("Control+f");
  const query = page.getByRole('searchbox', { name: 'Find in netlist' });
  await expect(query).toBeFocused();
  await query.fill('suffix 1299');
  await expect(page.locator('#netlist-matches')).toHaveText('1 / 1');
  const row = await lineOf(page, 'suffix 1299');
  await expect.poll(() => viewer.within(viewer.line(row))).toBe(true);
  expect(await page.evaluate(() => String(window.getSelection()))).toBe('suffix 1299');
  await showSource(page, 1);
  await query.fill('include 1000');
  await expect(page.locator('#netlist-matches')).toHaveText('1 / 1');
  const included = await lineOf(page, 'include 1000', 1);
  await expect.poll(() => viewer.within(viewer.line(included))).toBe(true);
  await query.press('Escape');
  await expect(page.locator('#netlist-find')).toBeHidden();
  expect(await page.locator('#source .file:not([hidden]) .ln').count()).toBeLessThan(300);
});

test("a dragged text selection copies the same text after both endpoints scroll out", async ({ page, viewer }) => {
  await viewer.open("virtual");
  await reveal(page, 0, 100);
  const a = await viewer.line(96).locator('.code').boundingBox();
  const b = await viewer.line(104).locator('.code').boundingBox();
  await page.mouse.move(a.x + 2, a.y + a.height / 2);
  await page.mouse.down();
  await page.mouse.move(b.x + 70, b.y + b.height / 2, { steps: 8 });
  await page.mouse.up();
  const before = await copy(page);
  expect(before).toContain('prefix 100');
  await page.locator('#source').evaluate(s => { s.scrollTop = s.scrollHeight; });
  await expect(viewer.line(96)).toHaveCount(0);
  expect(await copy(page)).toBe(before);
});

test("drag selection auto-scrolls and stops when the window loses focus", async ({ page, viewer }) => {
  await viewer.open('virtual');
  await reveal(page, 0, 100);
  const row = await viewer.line(100).locator('.code').boundingBox();
  const box = await page.locator('#source').boundingBox();
  const start = await page.locator('#source').evaluate(s => s.scrollTop);
  await page.mouse.move(row.x + 3, row.y + row.height / 2);
  await page.mouse.down();
  await page.mouse.move(row.x + 60, box.y + box.height + 30, { steps: 5 });
  await expect.poll(() => page.locator('#source').evaluate(s => s.scrollTop)).toBeGreaterThan(start + 300);
  await page.evaluate(() => window.dispatchEvent(new Event('blur')));
  const stopped = await page.locator('#source').evaluate(s => s.scrollTop);
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  expect(await page.locator('#source').evaluate(s => s.scrollTop)).toBe(stopped);
  await page.mouse.up();
});
