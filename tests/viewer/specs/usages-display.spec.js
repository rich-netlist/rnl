import { expect, test } from '../viewer.js';

const row = (page, name) => page.locator('.usage-row').filter({ has: page.getByRole('button', { name, exact: true }) });
const frame = (page, object) => page.locator(`#canvas .usage-frame[data-usage="${object}"]`);
const ordinary = page => page.locator('#canvas .overlay rect:not(.label)');
const usage = (page, name) => page.evaluate(name => window.viewer.doc.levels[window.viewer.level].usages.find(u => u.id === name), name);

for (const platform of ['desktop', 'web']) {
  test(`${platform}: source navigation and new-window sessions preserve usage inspection mode`, async ({page, viewer}) => {
    await viewer.open('usage-inspector', {platform});
    await page.keyboard.press('Control+r');
    const other = await page.evaluate(() => window.viewer.doc.levels.find(l => l.title === 'other').usages.find(u => u.id === '%other'));
    await page.keyboard.press('2');
    await page.locator(`#source [data-obj="${other.object}"]`).first().click();
    await viewer.selected('usage %other in other');
    await expect(frame(page, other.object)).toHaveClass(/selected/);
    await viewer.build(() => page.keyboard.press('F5'));
    const object = (await usage(page, '%other')).object;
    await expect(frame(page, object)).toHaveClass(/selected/);
    await page.evaluate(() => window.viewer.command('new-window'));
    const [[session]] = await viewer.calls('newWindow');
    expect(session.select).toMatchObject({kind: 'usage', name: '%other', usage: true});
    await page.evaluate(async session => (await import('./navigation.js')).restoreSession(session), session);
    await viewer.idle();
    await expect(frame(page, (await usage(page, '%other')).object)).toHaveClass(/selected/);
    await viewer.open('virtual-selection', {platform});
    await viewer.click('%ground');
    await page.evaluate(() => window.viewer.command('new-window'));
    const [[ordinarySession]] = await viewer.calls('newWindow');
    expect(ordinarySession.select).toMatchObject({name: '%ground', usage: false});
    await page.evaluate(async session => (await import('./navigation.js')).restoreSession(session), ordinarySession);
    await viewer.idle();
    await expect(ordinary(page)).toHaveCount(1);
    await expect(page.locator('.usage-frame.selected')).toHaveCount(0);
  });

  test(`${platform}: unselected titles appear only when they fit and hidden titles have no hit area`, async ({page, viewer}) => {
    await viewer.open('usage-inspector', {platform});
    const pair = await usage(page, '%pair1');
    // Isolate the frame so another semantic group's border cannot mask the title hit test.
    await page.evaluate(object => {
      const v = window.viewer;
      v.doc.levels[v.level].usages = v.doc.levels[v.level].usages.filter(u => u.object === object);
      window.dispatchEvent(new Event('viewer-level'));
    }, pair.object);
    await page.keyboard.press('5');
    await viewer.expandUsages();
    await page.evaluate(async () => (await import('./drawing.js')).zoomCenter(.02));
    const f = frame(page, pair.object), title = f.locator('.usage-title');
    await expect(f).toBeVisible();
    await expect(title).toBeHidden();
    const hits = await page.evaluate(async object => {
      const canvas = document.getElementById('canvas').getBoundingClientRect();
      const box = document.querySelector(`.usage-frame[data-usage="${object}"] > rect`).getBoundingClientRect();
      return (await import('./usages.js')).usagesAt({x: box.left - canvas.left + 12, y: box.top - canvas.top - 7});
    }, pair.object);
    expect(hits).toEqual([]);
    await row(page, '%pair1').getByRole('button').click();
    await expect(title).toBeVisible();
    await expect(f).toHaveClass(/selected/);
    await page.keyboard.press('Escape');
    await expect(title).toBeHidden();
    await page.evaluate(async () => (await import('./drawing.js')).zoomCenter(50));
    await expect(title).toBeVisible();
    const [tb, fb] = await Promise.all([title.boundingBox(), f.locator(':scope > rect').boundingBox()]);
    expect(tb.x).toBeGreaterThanOrEqual(fb.x);
    expect(tb.x + tb.width).toBeLessThanOrEqual(fb.x + fb.width);
    await page.evaluate(async () => (await import('./drawing.js')).zoomCenter(.02));
    await expect(title).toBeHidden();
  });

  test(`${platform}: focus changes RGB instead of opacity, preserves alpha and follows background changes`, async ({page, viewer}) => {
    await viewer.open('usage-inspector', {platform});
    const sourceSvg = await page.evaluate(() => window.viewer.doc.levels[window.viewer.level].svg);
    await page.evaluate(async () => {
      const v = window.viewer, l = v.doc.levels[v.level];
      const r3 = v.doc.objects.findIndex(o => o.kind === 'instance' && o.name === 'R3' && o.level === l.host);
      const id = l.hits.find(h => h.kind === 'box' && h.objects[0] === r3).id;
      const g = [...document.querySelectorAll('#canvas [data-id]')].find(g => g.dataset.id === id);
      const probe = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
      probe.id = 'alpha-probe';
      for (const [k, value] of Object.entries({x: 0, y: 0, width: 2, height: 2, fill: '#20406080', stroke: '#20406080'})) probe.setAttribute(k, value);
      g.append(probe);
      (await import('./background.js')).colourDrawing(document.querySelector('#canvas > svg'));
    });
    const ink = () => page.evaluate(() => {
      const v = window.viewer, l = v.doc.levels[v.level];
      const stroke = name => {
        const o = v.doc.objects.findIndex(o => o.kind === 'instance' && o.name === name && o.level === l.host);
        const id = l.hits.find(h => h.kind === 'box' && h.objects[0] === o).id;
        const g = [...document.querySelectorAll('#canvas [data-id]')].find(g => g.dataset.id === id);
        const e = g.querySelector('[stroke]:not([stroke="none"])');
        return {stroke: getComputedStyle(e).stroke, opacity: getComputedStyle(g).opacity, filter: getComputedStyle(g).filter};
      };
      const probe = getComputedStyle(document.getElementById('alpha-probe'));
      return {r1: stroke('R1'), r2: stroke('R2'), r3: stroke('R3'), probe: {stroke: probe.stroke, fill: probe.fill}};
    });
    await page.keyboard.press('5');
    await viewer.expandUsages();
    for (const mode of ['light', 'dark', 'light']) {
      await page.keyboard.press('Escape');
      await page.evaluate(mode => window.viewer.command(`background-${mode}`), mode);
      const before = await ink();
      await row(page, '%pair1').getByRole('button').click();
      const focused = await ink();
      expect(focused.r1).toEqual(before.r1);
      expect(focused.r2).toEqual(before.r2);
      expect(focused.r3.opacity).toBe('1');
      expect(focused.r3.filter).toBe('none');
      for (const [base, actual] of [[before.r3.stroke, focused.r3.stroke], [before.probe.stroke, focused.probe.stroke], [before.probe.fill, focused.probe.fill]]) {
        const a = base.match(/[\d.]+/g).map(Number), b = actual.match(/[\d.]+/g).map(Number);
        for (let c = 0; c < 3; c++) expect(b[c]).toBe(Math.round((a[c] + (mode === 'light' ? 255 : 0)) / 2));
        expect(b[3]).toBe(a[3]);
      }
      await page.keyboard.press('Escape');
      expect(await ink()).toEqual(before);
    }
    expect(await page.evaluate(() => window.viewer.doc.levels[window.viewer.level].svg)).toBe(sourceSvg);
  });

  test(`${platform}: hidden virtual usages retain ordinary picking; list selection still forces usage focus`, async ({page, viewer}) => {
    await viewer.open('virtual-selection', {platform});
    const ground = await usage(page, '%ground');
    await page.keyboard.press('5');
    await viewer.expandUsages();
    const g = row(page, '%ground');
    await expect(g.getByRole('checkbox')).not.toBeChecked();
    await viewer.click('%ground');
    await viewer.selected('usage %ground in virtual-selection.sp');
    await expect(ordinary(page)).toHaveCount(1);
    await expect(frame(page, ground.object)).toHaveCount(0);
    await expect(page.locator('#canvas .usage-muted')).toHaveCount(0);
    await expect(g).not.toHaveClass(/selected/);
    await viewer.build(() => page.keyboard.press('F5'));
    await expect(frame(page, (await usage(page, '%ground')).object)).toHaveCount(0);
    await viewer.expandUsages();
    await g.getByRole('button').click();
    const f = frame(page, (await usage(page, '%ground')).object);
    await expect(f).toHaveClass(/selected/);
    await expect(page.locator('#canvas .usage-muted').first()).toBeAttached();
    const b = await f.locator(':scope > rect').boundingBox();
    await page.mouse.click(b.x, b.y + b.height / 2); // The forced but disabled border is not selectable.
    await viewer.selected(null);
    await expect(f).toHaveCount(0);
    await g.getByRole('checkbox').check();
    await viewer.click('%ground');
    await expect(f).toHaveClass(/selected/);
    await page.keyboard.press('Control+r');
    await expect(f).toHaveClass(/selected/); // Existing selection remains forced until the next pick.
    await viewer.click('%ground');
    await expect(f).toHaveCount(0);
    await expect(ordinary(page)).toHaveCount(1);
    await g.getByRole('button').click();
    await expect(f).toHaveClass(/selected/);
    await viewer.click('%ground2');
    await expect(f).toHaveCount(0);
    await expect(ordinary(page)).toHaveCount(1);
    await expect(page.locator('#canvas .usage-muted')).toHaveCount(0);
  });

  test(`${platform}: grounds and supplies frame and focus only their own symbols on a shared net`, async ({page, viewer}) => {
    await viewer.open('virtual-selection', {platform});
    await page.keyboard.press('5');
    await viewer.expandUsages();
    for (const name of ['%ground', '%ground2', '%supply']) {
      const u = await usage(page, name);
      await row(page, name).getByRole('button').click();
      const bounds = await page.evaluate(object => {
        const v = window.viewer;
        const b = v.doc.levels[v.level].usages.find(u => u.object === object).frame.bounds;
        return [[b[0], b[1]], [b[2], b[3]]];
      }, u.object);
      const [a, b] = await Promise.all(bounds.map(([x, y]) => viewer.point(x, y)));
      const box = await frame(page, u.object).locator(':scope > rect').boundingBox();
      const pad = 4 / 3; // Core bounds include padding; browser boxes add half the selected 2pt stroke.
      for (const [actual, expected] of [[box.x, a.x - pad], [box.y, a.y - pad], [box.width, b.x - a.x + pad * 2], [box.height, b.y - a.y + pad * 2]]) {
        expect(Math.abs(actual - expected)).toBeLessThan(.1);
      }
      const other = name === '%ground' ? '%supply' : '%ground';
      const p = await viewer.where(other);
      expect(p.x < box.x || p.x > box.x + box.width || p.y < box.y || p.y > box.y + box.height).toBe(true);
      const muted = await page.evaluate(([own, other]) => {
        const v = window.viewer, l = v.doc.levels[v.level];
        return [own, other].map(name => {
          const o = l.usages.find(u => u.id === name).object;
          const id = l.hits.find(h => h.kind === 'box' && h.objects[0] === o).id;
          return [...document.querySelectorAll('#canvas [data-id]')].find(g => g.dataset.id === id).classList.contains('usage-muted');
        });
      }, [name, other]);
      expect(muted).toEqual([false, true]);
      const badge = await frame(page, u.object).locator('.usage-role-label').first().boundingBox();
      expect(Math.abs(badge.x + badge.width / 2 - (a.x + b.x) / 2)).toBeLessThan(.1);
      expect(Math.abs(badge.y + badge.height / 2 - (a.y + b.y) / 2)).toBeLessThan(.1);
    }
  });

  test(`${platform}: multiple roles of virtual usages stay readable and component roles keep their centers`, async ({page, viewer}) => {
    await viewer.open('usage-virtual-roles', {platform});
    await page.keyboard.press('5');
    await viewer.expandUsages();
    for (const name of ['%pair', '%connections']) {
      const u = await usage(page, name);
      expect(u.has_symbol).toBe(true);
      await row(page, name).getByRole('button').click();
      const labels = frame(page, u.object).locator('.usage-role-label');
      await expect(labels).toHaveCount(2);
      const [a, b] = await Promise.all([labels.nth(0).boundingBox(), labels.nth(1).boundingBox()]);
      expect(a.x + a.width <= b.x || b.x + b.width <= a.x || a.y + a.height <= b.y || b.y + b.height <= a.y).toBe(true);
      if (name === '%pair') for (const [i, r] of u.roles.entries()) {
        const center = await viewer.point((r.bounds[0] + r.bounds[2]) / 2, (r.bounds[1] + r.bounds[3]) / 2);
        const box = await labels.nth(i).boundingBox();
        expect(Math.abs(box.x + box.width / 2 - center.x)).toBeLessThan(.1);
        expect(Math.abs(box.y + box.height / 2 - center.y)).toBeLessThan(.1);
      }
    }
    const route = await usage(page, '%route');
    expect(route.roles[0].objects).toEqual([]);
    await row(page, '%route').getByRole('button').click();
    const wireFocus = await page.evaluate(groups => groups.map(id => [...document.querySelectorAll('#canvas [data-id]')]
      .filter(g => g.dataset.id === id).every(g => !g.classList.contains('usage-muted'))), route.roles[0].groups);
    expect(wireFocus.length).toBeGreaterThan(0);
    expect(wireFocus.every(Boolean)).toBe(true);
    const r = route.frame.bounds;
    const [a, b] = await Promise.all([viewer.point(r[0], r[1]), viewer.point(r[2], r[3])]);
    const rect = await frame(page, route.object).locator(':scope > rect').boundingBox();
    expect(Math.abs(rect.width - (b.x - a.x + 8 / 3))).toBeLessThan(.1);
    const label = await frame(page, route.object).locator('.usage-role-label').boundingBox();
    expect(Math.abs(label.x + label.width / 2 - (a.x + b.x) / 2)).toBeLessThan(.1);
  });
}
