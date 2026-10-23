import { expect, test } from "../viewer.js";

const row = (page, name) => page.locator('.usage-row').filter({ has: page.getByRole('button', { name, exact: true }) });
const frame = (page, id) => page.locator(`#canvas .usage-frame[data-usage="${id}"]`);
async function usage(page, name) {
  return page.evaluate(name => window.viewer.doc.levels[window.viewer.level].usages.find(u => u.id === name), name);
}
async function background(page, mode) { await page.evaluate(mode => window.viewer.command(`background-${mode}`), mode); }

for (const platform of ['desktop', 'web']) {
  test(`${platform}: usages are scope-local, grouped, foldable and have editable Color and source`, async ({ page, viewer }) => {
    await viewer.open('usage-inspector', { platform });
    await page.keyboard.press('5');
    await expect(page.locator('#pane-usages')).toBeVisible();
    const types = await page.locator('.usage-group').evaluateAll(rows => rows.map(r => r.dataset.type));
    expect(types).toEqual([...types].sort((a, b) => a.localeCompare(b, undefined, { numeric: true })));
    expect(types).not.toContain('$OTHER');
    const pairGroup = page.locator('.usage-group[data-type="$PAIR"]');
    await expect(pairGroup.locator('.badge')).toHaveText('(3)');
    await pairGroup.locator('.usage-expand').click();
    await expect(row(page, '%pair1')).toHaveCount(0);
    await pairGroup.locator('.usage-expand').click();
    await row(page, '%pair1').getByRole('button').click();
    await expect(page.locator('#usage-detail h2')).toHaveText('$PAIR: %pair1');
    await expect(page.locator('.usage-source')).toContainText('%pair1 = #R1(left) #R2(right) @AS $PAIR');
    await expect(page.locator('.usage-source-file')).toContainText('usage-inspector.sp');
    await expect(page.locator('.usage-roles')).toContainText('left');
    await expect(page.locator('.usage-roles')).toContainText('#R1');
    const colors = await page.locator('.usage-palette button').evaluateAll(buttons => buttons.map(b => b.dataset.color));
    expect(colors).toHaveLength(6);
    const pair = await usage(page, '%pair1');
    await page.locator('.usage-palette [data-color="#9c36b5"]').click();
    await expect(frame(page, pair.object).locator(':scope > rect')).toHaveAttribute('stroke', '#9c36b5');
    await page.evaluate(() => window.viewer.command('language-zh'));
    await expect(page.locator('#tab-usages')).toHaveAccessibleName('用例');
    await expect(page.getByLabel('颜色', { exact: true })).toBeVisible();
    await viewer.build(() => page.keyboard.press('F5'));
    await expect(frame(page, (await usage(page, '%pair1')).object).locator(':scope > rect')).toHaveAttribute('stroke', '#9c36b5');
    await page.evaluate(async () => {
      const v = window.viewer;
      await (await import('./subcircuits.js')).goTo(v.doc.levels.findIndex(l => l.title === 'other'));
    });
    await expect(row(page, '%other')).toBeVisible();
    await expect(row(page, '%pair1')).toHaveCount(0);
    await expect(page.locator('#usage-detail')).not.toContainText('%pair1');
  });

  test(`${platform}: switches, mixed group state and forced selection obey one visibility rule`, async ({ page, viewer }) => {
    await viewer.open('usage-inspector', { platform });
    const pair = await usage(page, '%pair1'), unseen = await usage(page, '%unseen'), ground = await usage(page, '%ground');
    expect(await page.evaluate(() => window.viewer.checked('toggle-usages'))).toBe(true);
    await expect(frame(page, pair.object)).toBeVisible();
    await expect(frame(page, unseen.object)).toHaveCount(0);
    await expect(frame(page, ground.object)).toHaveCount(0);
    await page.keyboard.press('5');
    await expect(row(page, '%ground').getByRole('checkbox')).not.toBeChecked();
    await row(page, '%pair1').getByRole('checkbox').uncheck();
    await expect(frame(page, pair.object)).toHaveCount(0);
    expect(await page.locator('.usage-group[data-type="$PAIR"] > .usage-group-head input').evaluate(e => e.indeterminate)).toBe(true);
    await row(page, '%pair1').getByRole('button').click();
    await expect(frame(page, pair.object)).toHaveClass(/selected/);
    await page.keyboard.press('Control+r');
    expect(await page.evaluate(() => window.viewer.checked('toggle-usages'))).toBe(false);
    await expect(frame(page, pair.object)).toBeVisible();
    await expect(page.locator('.usage-frame')).toHaveCount(1);
    await viewer.click('R3');
    await expect(frame(page, pair.object)).toHaveCount(0);
    await row(page, '%pair1').getByRole('button').click();
    await page.keyboard.press('Escape');
    await expect(frame(page, pair.object)).toHaveCount(0);
    await page.keyboard.press('Control+r');
    const control = page.locator('.usage-group[data-type="$PAIR"] > .usage-group-head input');
    await control.check();
    await expect(row(page, '%pair1').getByRole('checkbox')).toBeChecked();
    await control.uncheck();
    await expect(row(page, '%pair2').getByRole('checkbox')).not.toBeChecked();
    expect(await viewer.calls('build')).toEqual([['usage-inspector']]);
    expect(await viewer.calls('initial')).toEqual([[]]);
  });

  test(`${platform}: frame picking links source and focus; fixed screen geometry follows pan and zoom`, async ({ page, viewer }) => {
    await viewer.open('usage-inspector', { platform });
    const pair = await usage(page, '%pair1');
    const box = frame(page, pair.object).locator(':scope > rect');
    const fill = page.locator(`#canvas .usage-fill[data-usage="${pair.object}"]`);
    const clickBorder = async () => {
      const b = await box.boundingBox();
      await page.mouse.click(b.x, b.y + b.height / 2);
    };
    await clickBorder();
    // Nested semantic groups can have exactly the same frame: normal overlap picking applies.
    await expect(page.locator('#pick-list')).toBeVisible();
    await page.locator(`#pick-list [data-object="${pair.object}"]`).click();
    await expect(frame(page, pair.object)).toHaveClass(/selected/);
    expect(await page.evaluate(() => window.viewer.selected)).toBe(pair.object);
    await expect(fill).toHaveAttribute('fill-opacity', '0.2');
    await expect(frame(page, pair.object).locator('.usage-title text')).toHaveText('$PAIR: %pair1');
    await expect(frame(page, pair.object).locator('.usage-role-label text')).toHaveText(['left', 'right']);
    const opacity = name => page.evaluate(name => {
      const d = window.viewer.doc, l = d.levels[window.viewer.level];
      const o = d.objects.findIndex(o => o.kind === 'instance' && o.name === name && o.level === l.host);
      const h = l.hits.find(h => h.kind === 'box' && h.objects.includes(o));
      return getComputedStyle(document.querySelector(`#canvas [data-id="${h.id}"]`)).opacity;
    }, name);
    expect(await opacity('R1')).toBe('1');
    expect(await opacity('R2')).toBe('1');
    expect(await opacity('R3')).toBe('1');
    const label = frame(page, pair.object).locator('.usage-role-label text').first();
    const before = await label.boundingBox();
    const stroke = await box.getAttribute('stroke-width');
    expect(Number(stroke)).toBeCloseTo(8 / 3);
    await page.keyboard.press('Control+=');
    await page.keyboard.press('Control+=');
    const after = await label.boundingBox();
    expect(Math.abs(before.height - after.height)).toBeLessThan(.2);
    expect(Math.abs(before.width - after.width)).toBeLessThan(.2);
    expect(await box.getAttribute('stroke-width')).toBe(stroke);
    for (const mode of ['dark', 'light']) {
      await background(page, mode);
      await expect(frame(page, pair.object).locator('.usage-role-label rect').first()).toHaveCSS('fill', mode === 'dark' ? 'rgb(0, 0, 0)' : 'rgb(255, 255, 255)');
    }
    await page.keyboard.press('2');
    await expect(page.locator('#source .hl-line')).toContainText(['%pair1']);
    await page.keyboard.press('f');
    await viewer.click('R3');
    await expect(page.locator('.usage-role-label')).toHaveCount(0);
    expect(await opacity('R1')).toBe('1');
    await expect(fill).toHaveAttribute('fill-opacity', '0.1');
    expect(Number(await box.getAttribute('stroke-width'))).toBeCloseTo(4 / 3);
  });
}
