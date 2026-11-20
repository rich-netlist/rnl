// The modules of Verilog-A in the web viewer, which reads the file shown alone with the outline of the whole document
// (level-tree.js, module-view.js): the hierarchy lists the modules, marked so, with the modules their instances name;
// an instance of a module, in the netlist or in a module, opens it as its ports, with its instances, each with its line
// in the text, and a name no module has is not defined.
import { test, expect } from '@playwright/test';

test.skip(!process.env.RNL_WEB_URL, 'the web site is not built');

const TOP = `// top
simulator lang=spectre
ahdl_include "a.va"
ahdl_include "b.va"
I1 (in out 0) top_va k=3
R1 (in 0) resistor r=1k
`;
const A = `module top_va(x, y, g);
  inout x, y, g;
  electrical x, y, g, mid;
  gain #(.k(2)) u1 (x, mid);
  amp u2 (mid, y);
  ghost u3 (y, g);
  gain u4 (.a(y), .b(g));
  analog V(y) <+ 0;
endmodule
module gain(a, b);
  inout a, b;
  electrical a, b;
endmodule
`;
const B = `module amp(inp, outp);
  inout inp, outp;
  electrical inp, outp;
endmodule
`;

async function open(page) {
  await page.route('https://rich-netlist.github.io/rnl-packages/**', route => route.abort());
  await page.goto(`${process.env.RNL_WEB_URL}/index.html`);
  await expect(page.locator('#menubar')).toBeVisible();
  await page.locator('#file-input').setInputFiles([
    { name: 'top.scs', mimeType: 'text/plain', buffer: Buffer.from(TOP) },
    { name: 'a.va', mimeType: 'text/plain', buffer: Buffer.from(A) },
    { name: 'b.va', mimeType: 'text/plain', buffer: Buffer.from(B) },
  ]);
  // the page asks which of the files is the top level: the netlist, listed first
  const dialog = page.getByRole('dialog');
  await expect(dialog.getByRole('option', { name: 'top.scs' })).toHaveAttribute('aria-selected', 'true');
  await dialog.getByRole('button', { name: 'Open' }).click();
  await idle(page);
}
const idle = (page) => page.waitForFunction(() => window.viewer.doc && !document.body.classList.contains('busy'));

/** What the page shows: the file read, the module shown, the kind of the level and the path's names. */
const shown = (page) => page.evaluate(() => ({
  file: window.viewer.doc.file,
  module: document.getElementById('module-card').hidden ? null : document.querySelector('#module-card .module-name').textContent,
  kind: document.getElementById('level-kind').textContent,
  path: [...document.querySelectorAll('#level-path .crumb')].map(c => c.querySelector('.name, #level-name')?.textContent),
}));
const card = (page) => page.locator('#module-card');
const instances = (page) => card(page).locator('.module-instance');

test('the hierarchy lists the modules of Verilog-A, and the modules their instances name', async ({ page }) => {
  await open(page);
  await page.locator('#tab-hierarchy').click();
  // the index: each module, marked so, with its uses
  const index = page.locator('#index .index-row:not(.top)');
  await expect(index.locator('.name')).toHaveText(['amp', 'gain', 'top_va']);
  await expect(index.locator('.badge')).toHaveText(['Verilog-A', 'Verilog-A', 'Verilog-A']);
  await expect(index.locator('.uses')).toHaveText(['1 use', '2 uses', '1 use']);
  // the tree: the netlist's instance of the module, and under it the module's instances, a name no module has as one row
  await page.locator('#tree .tree-row', { hasText: 'I1' }).locator('.twisty').click();
  const rows = page.locator('#tree .tree-row');
  await expect(rows.locator('.inst')).toHaveText(['top.scs', 'I1', 'u1', 'u4', 'u2', '1×']);
  await expect(rows.nth(1).locator('.badge')).toHaveText('Verilog-A');
  await expect(rows.nth(5)).toHaveClass(/undefined/);
  await expect(rows.nth(5).locator('.master')).toHaveText('ghost');
  await expect(rows.nth(5).locator('.badge')).toHaveText('not defined');
});

test('an instance of a module opens its ports and its instances, each with its line, and the path leads back', async ({ page }) => {
  await open(page);
  const i1 = await page.evaluate(() => window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === 'I1'));
  expect(await page.evaluate((o) => import('./subcircuits.js').then(s => s.canOpen(o)), i1)).toBe(true);
  await page.evaluate((o) => import('./subcircuits.js').then(s => s.descend(o)), i1);
  await idle(page);
  expect(await shown(page)).toEqual({ file: 'a.va', module: 'top_va', kind: 'Verilog-A module', path: ['top.scs', 'top_va'] });
  // the module is read without the parameters of the instance, and nothing failed
  expect(await page.evaluate(() => window.viewer.doc.failed)).toBeNull();
  // its ports, as a symbol, and its instances: the module each names, its line, and a name no module has
  await expect(card(page).locator('.module-symbol text')).toHaveText(['x', 'y', 'g']);
  await expect(instances(page).locator('.inst')).toHaveText(['u1', 'u4', 'u2', '1×']);
  await expect(instances(page).locator('.master')).toHaveText(['gain', 'gain', 'amp', 'ghost']);
  await expect(instances(page).locator('.line')).toHaveText(['line 4', 'line 7', 'line 5']);
  await expect(instances(page).nth(3)).toHaveClass(/undefined/);
  // the tree keeps the way open to it, from the top level
  await page.locator('#tab-hierarchy').click();
  await expect(page.locator('#tree .tree-row.active .inst')).toHaveText('I1');
  // a click shows the instance's line in the text
  await page.locator('#tab-netlist').click();
  await instances(page).nth(2).click();
  await expect(instances(page).nth(2)).toHaveClass(/selected/);
  await expect(page.locator('#source .flash')).toContainText('amp u2 (mid, y);');
  // a double click opens the module it names, in its own file
  await instances(page).nth(2).dblclick();
  await idle(page);
  expect(await shown(page)).toEqual({ file: 'b.va', module: 'amp', kind: 'Verilog-A module', path: ['top.scs', 'top_va', 'amp'] });
  await expect(card(page).locator('.module-symbol text')).toHaveText(['inp', 'outp']);
  await expect(card(page).locator('.module-none')).toHaveText('No instances of modules.');
  // up, to the module of the other file; Enter on an instance opens a module of the same file
  await page.evaluate(() => import('./subcircuits.js').then(s => s.ascend()));
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'a.va', module: 'top_va', path: ['top.scs', 'top_va'] });
  await instances(page).nth(0).focus();
  await page.keyboard.press('Enter');
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'a.va', module: 'gain', path: ['top.scs', 'top_va', 'gain'] });
  // up within the file, and up again to the netlist's drawing, with the instance it was left through selected
  await page.evaluate(() => import('./subcircuits.js').then(s => s.ascend()));
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'a.va', module: 'top_va', path: ['top.scs', 'top_va'] });
  await page.evaluate(() => import('./subcircuits.js').then(s => s.ascend()));
  await idle(page);
  expect(await shown(page)).toMatchObject({ file: 'top.scs', module: null, path: ['top.scs'] });
  await expect.poll(() => page.evaluate(() => window.viewer.doc.objects[window.viewer.selected]?.name)).toBe('I1');
});

test('a module picked in the index opens as the root of the path, and the file of modules opened shows its first', async ({ page }) => {
  await open(page);
  await page.locator('#tab-hierarchy').click();
  await page.locator('#index .index-row:not(.top)', { hasText: 'gain' }).click();
  await idle(page);
  expect(await shown(page)).toEqual({ file: 'a.va', module: 'gain', kind: 'Verilog-A module', path: ['gain'] });
  await expect(page.locator('#index .index-row.current .name')).toHaveText('gain');
  // the file of modules opened from the netlist pane: its first module
  await page.locator('#index .index-row.top').click();
  await idle(page);
  await page.evaluate(() => import('./navigation.js').then(n => n.navigate({ file: window.viewer.doc.files.findIndex(f => f.path === 'b.va') })));
  await idle(page);
  expect(await shown(page)).toEqual({ file: 'b.va', module: 'amp', kind: 'Verilog-A module', path: ['amp'] });
});
