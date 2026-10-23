import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {expect, test} from '../viewer.js';
import {root} from '../fixtures.js';

for (const platform of ['web', 'desktop']) {
  test(`${platform}: current switch starts off and persists independently of usages`, async ({page, viewer}) => {
    await viewer.open('usage-current', {platform});
    expect(await page.evaluate(() => window.viewer.checked('toggle-currents'))).toBe(false);
    await expect(page.locator('.current-arrow')).toHaveCount(0);
    if (platform === 'web') {
      await viewer.menuTitle('View').click();
      await page.locator('.menu-item[data-id="toggle-currents"]').click();
    } else { await page.evaluate(() => window.fakeShell.menu('toggle-currents')); }
    await expect(page.locator('.current-arrow')).toHaveCount(7);
    await page.keyboard.press('Control+r');
    await expect(page.locator('.current-arrow')).toHaveCount(7);
    await viewer.build(() => page.keyboard.press('F5'));
    await expect(page.locator('.current-arrow')).toHaveCount(7);
    await viewer.open('usage-current', {platform});
    expect(await page.evaluate(() => window.viewer.checked('toggle-currents'))).toBe(true);
    await expect(page.locator('.current-arrow')).toHaveCount(7);
    await page.evaluate(async () => (await import('./subcircuits.js')).goTo(1));
    await expect(page.locator('.current-arrow')).toHaveCount(4);
    await page.evaluate(() => window.viewer.command('toggle-currents'));
    await expect(page.locator('.current-arrow')).toHaveCount(0);
  });

  test(`${platform}: Viewer projects core frames and current paths exactly like CLI`, async ({page, viewer}, info) => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rnl-overlay-browser-'));
    try {
      execFileSync(process.env.RNL_BIN, ['render', path.join(root, 'tests/viewer/inputs/usage-current.sp'), '-o', dir, '--usages', 'all', '--currents', '--usage-roles']);
      await viewer.open('usage-current', {platform});
      await page.evaluate(() => window.viewer.command('toggle-currents'));
      await page.keyboard.press('5');
      for (const [level, file] of [[0, 'usage-current.svg'], [1, 'usage-current.cell.svg']]) {
        await page.evaluate(async level => (await import('./subcircuits.js')).goTo(level), level);
        // Display every usage; selection still controls only emphasis and role labels.
        for (const control of await page.locator('.usage-group-head input').all()) await control.check();
        const exported = fs.readFileSync(path.join(dir, file), 'utf8');
        for (const zoom of [1, 1.5]) {
          if (zoom !== 1) await page.evaluate(async z => (await import('./drawing.js')).zoomCenter(z), zoom);
          const geometry = await page.evaluate(exported => {
            const cli = new DOMParser().parseFromString(exported, 'image/svg+xml');
            const root = document.querySelector('#canvas > svg');
            const frames = [...root.querySelectorAll('.usage-frame')].map(f => {
              const rect = f.querySelector(':scope > rect');
              const matrix = root.getCTM().inverse().multiply(rect.getCTM());
              const p = (x,y) => new DOMPoint(x,y).matrixTransform(matrix);
              const a = p(rect.x.baseVal.value, rect.y.baseVal.value);
              const b = p(rect.x.baseVal.value+rect.width.baseVal.value, rect.y.baseVal.value+rect.height.baseVal.value);
              const expected = cli.querySelector(`.usage-frame[data-usage="${f.dataset.usage}"] > rect`);
              const v = window.viewer;
              const core = v.doc.levels[v.level].usages.find(u => u.object === Number(f.dataset.usage)).frame.bounds;
              return {actual: [a.x,a.y,b.x,b.y], core, expected: ['x','y','width','height'].map(k => Number(expected.getAttribute(k)))};
            });
            const paths = [...root.querySelectorAll('.current-arrow')].map(p => {
              const expected = [...cli.querySelectorAll('.current-arrow')].find(c => c.dataset.terminal === p.dataset.terminal);
              return [p.getAttribute('d'), expected.getAttribute('d')];
            });
            return {frames, paths};
          }, exported);
          for (const {actual,core,expected: [x,y,w,h]} of geometry.frames) {
            expect([core[0],core[1],core[2]-core[0],core[3]-core[1]]).toEqual([x,y,w,h]);
            [x,y,x+w,y+h].forEach((v,i) => expect(actual[i]).toBeCloseTo(v, 3));
          }
          for (const [actual,expected] of geometry.paths) expect(actual).toBe(expected);
        }
        const inspected = await page.evaluate(() => {
          const v = window.viewer;
          return v.doc.levels[v.level].usages.find(u => u.frame && u.roles.length).object;
        });
        await page.locator(`.usage-row[data-object="${inspected}"] .usage-name`).click();
        const roles = await page.evaluate(([exported,object]) => {
          const cli = new DOMParser().parseFromString(exported, 'image/svg+xml');
          const root = document.querySelector('#canvas > svg');
          const v = window.viewer, entry = v.doc.levels[v.level].usages.find(u => u.object === object);
          return [...root.querySelectorAll('.usage-frame.selected .usage-role-label text')].map((text,i) => {
            const matrix = root.getCTM().inverse().multiply(text.getCTM());
            const p = new DOMPoint(entry.frame.role_widths[i]/2, 0).matrixTransform(matrix);
            const expected = cli.querySelectorAll(`.usage-frame[data-usage="${object}"] .usage-role-label text`)[i];
            return {actual: [p.x,p.y], expected: [Number(expected.getAttribute('x')),Number(expected.getAttribute('y'))]};
          });
        }, [exported,inspected]);
        expect(roles.length).toBeGreaterThan(0);
        for (const {actual,expected} of roles) expected.forEach((v,i) => expect(actual[i]).toBeCloseTo(v,3));
      }
      await page.evaluate(async () => (await import('./subcircuits.js')).goTo(0));
      await page.locator('.usage-row').filter({has: page.getByRole('button', {name: '%pair', exact: true})}).getByRole('button').click();
      for (const lang of ['en','zh']) for (const theme of ['light','dark']) {
        await page.emulateMedia({colorScheme: theme});
        await page.evaluate(([lang,theme]) => {window.viewer.command(`language-${lang}`); window.viewer.command(`background-${theme}`); window.viewer.command('fit');}, [lang,theme]);
        const review = process.env.RNL_OVERLAY_REVIEW_DIR;
        if (review && info.project.name === 'chromium') {
          fs.mkdirSync(review, {recursive: true});
          await page.screenshot({path: path.join(review, `viewer-${platform}-${lang}-${theme}.png`)});
          for (const file of ['usage-current.svg','usage-current.cell.svg']) fs.copyFileSync(path.join(dir,file), path.join(review,file));
          if (platform === 'web') {
            await viewer.menuTitle(lang === 'zh' ? '视图' : 'View').click();
            await expect(viewer.menuItem('toggle-currents')).toHaveAttribute('aria-checked','true');
            await page.screenshot({path: path.join(review, `viewer-menu-${lang}-${theme}.png`)});
            await page.keyboard.press('Escape');
          }
        }
      }
      if (process.env.RNL_OVERLAY_REVIEW_DIR && info.project.name === 'chromium' && platform === 'web') {
        const preview = await page.context().newPage();
        for (const file of ['usage-current.svg','usage-current.cell.svg']) {
          await preview.setContent(fs.readFileSync(path.join(dir,file), 'utf8'));
          await preview.locator('svg').screenshot({path: path.join(process.env.RNL_OVERLAY_REVIEW_DIR, `cli-${file}.png`)});
        }
        await preview.close();
      }
    } finally { fs.rmSync(dir, {recursive: true, force: true}); }
  });
}
