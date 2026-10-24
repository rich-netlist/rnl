import { showSource } from "../viewer.js";
// Smoke tests of the web viewer (apps/web): the site that build.mjs makes with the samples, served by
// serve.mjs, with the web shell and the real core as WebAssembly in its Worker. It opens netlists by
// URL, files set on its file inputs, a folder and dropped files, and each Document is the one
// `rnl view` builds for the same files but for the paths that name them in the workspace.

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import { createHash } from "node:crypto";
import os from "node:os";
import path from "node:path";
import { expect, test } from "@playwright/test";
import { PICK, root, UNCALLED } from "../fixtures.js";
import { publicPackageDir, publicPackagesAvailable } from "../../../scripts/packages.mjs";
const hasPublicPackages = publicPackagesAvailable();
test.beforeEach(async ({ page }) => {
  await page.route("https://rich-netlist.github.io/rnl-packages/**", route => route.abort());
});

test("panel keys work at once while the WebAssembly core is still loading", async ({ page }) => {
  // a browser that runs WebAssembly without JIT takes long to have the core ready
  let release;
  const ready = new Promise(resolve => release = resolve);
  await page.route("**/*.wasm", async route => { await ready; await route.continue(); });
  await page.goto(`${process.env.RNL_WEB_URL}/?url=samples/cases/04-hierarchy.sp`);
  await expect(page.locator("#startup")).toHaveCount(0);
  await page.keyboard.press("3");
  await expect(page.locator("#pane-hierarchy")).toBeVisible();
  await page.keyboard.press("1");
  await expect(page.locator("#pane-files")).toBeVisible();
  release();
  await expect(page).toHaveTitle(/^04-hierarchy\.sp/);
  await expect(page.locator("#pane-files")).toBeVisible();
});

test('usage inspection and overlay selection work with the real WASM core', async ({ page }) => {
  await start(page);
  const doc = await opened(page, () => page.locator('#file-input').setInputFiles({
    name: 'usage-inspector.sp', mimeType: 'text/plain',
    buffer: fs.readFileSync(path.join(root, 'tests/viewer/inputs/usage-inspector.sp')),
  }));
  const pair = doc.levels[0].usages.find(u => u.id === '%pair1');
  expect(pair.roles.every(r => r.bounds && r.objects.length)).toBe(true);
  await page.keyboard.press('5');
  await page.getByRole('button', { name: '%pair1', exact: true }).click();
  await expect(page.locator(`#canvas .usage-frame[data-usage="${pair.object}"]`)).toHaveClass(/selected/);
  await expect(page.locator('.usage-source')).toContainText('%pair1');
  await page.keyboard.press('Control+r');
  await expect(page.locator('#canvas .usage-frame')).toHaveCount(1);
  await page.keyboard.press('Escape');
  await expect(page.locator('#canvas .usage-frame')).toHaveCount(0);
});

test('unkeyed wire net labels have independent colours in the real web core', async ({ page }) => {
  await start(page);
  const doc = await opened(page, () => page.locator('#file-input').setInputFiles({
    name: 'wire-labels.sp', mimeType: 'text/plain',
    buffer: fs.readFileSync(path.join(root, 'tests/viewer/inputs/wire-labels.sp')),
  }));
  const labels = doc.levels[0].hits.filter(h => h.kind === 'label' && h.objects.length === 1
    && doc.objects[h.objects[0]].kind === 'net');
  expect(labels).toHaveLength(2);
  expect(labels.every(h => h.id)).toBe(true);
  expect(labels[0].id).not.toBe(labels[1].id);
  const text = h => page.locator(`#canvas g[data-id=${JSON.stringify(h.id)}] text`).first();
  for (const background of ['light', 'dark']) {
    await page.evaluate(b => window.viewer.command(`background-${b}`), background);
    for (const label of labels) {
      await page.evaluate(async object => (await import('./selection.js')).select(object, 'drawing'), label.objects[0]);
      await expect(text(label)).toHaveCSS('fill', 'rgb(232, 89, 12)');
      for (const other of labels.filter(h => h !== label)) await expect(text(other)).not.toHaveCSS('fill', 'rgb(232, 89, 12)');
      await page.keyboard.press('Escape');
      await expect(text(label)).not.toHaveCSS('fill', 'rgb(232, 89, 12)');
    }
  }
});

test("the web core supplies semantic lines and the actual parent symbol", async ({ page }) => {
  await start(page);
  await opened(page, () => page.locator('#file-input').setInputFiles({
    name: 'symbol-inspector.sp', mimeType: 'text/plain',
    buffer: fs.readFileSync(path.join(root, 'tests/viewer/inputs/symbol-inspector.sp')),
  }));
  await page.locator('#tab-netlist').click();
  await page.locator('#rnl-only').check();
  await page.locator('#semantics-only').check();
  expect(await page.locator('#source .file:not([hidden]) .ln').evaluateAll(es => es.map(e => Number(e.dataset.line)))).toEqual([12, 31]);
  await page.evaluate(async () => {
    const index = window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === 'Xparent');
    await (await import('./subcircuits.js')).descend(index);
  });
  await page.locator('#tab-symbol').click();
  await expect(page.locator('.symbol-bounds')).toHaveText('Box = [-4, -2, 4, 2]');
  await expect(page.locator('.symbol-definition')).toContainText('width=8');
  await expect(page.locator('.symbol-source-lines')).toHaveText('Lines 21–26');
  await page.locator('#symbol-pins').uncheck();
  await expect(page.locator('.symbol-pin-marks')).toBeHidden();
});

test("Auto Include resolves indexed library subcircuits, warns on conflicts and reloads the library", async ({ page }) => {
  await start(page);
  const cell = ".subckt cell p n\nR1 p n 1k\n.ends\n";
  await page.locator("#include-input").setInputFiles({ name: "cell.inc", mimeType: "text/plain", buffer: Buffer.from(cell) });
  await expect(page.getByRole("button", { name: "Exclude cell.inc", exact: true })).toBeVisible();
  await page.locator("#file-input").setInputFiles({ name: "top.netlist", mimeType: "text/plain", buffer: Buffer.from("* top\nX1 a 0 cell\n.end\n") });
  await shown(page);
  await page.locator("#tab-netlist").click();
  await expect(page.locator("#auto-include")).toBeChecked();
  const imported = await page.evaluate(() => window.viewer.doc);
  expect(imported.files).toHaveLength(2);
  expect(imported.files[0].text).toBe("* top\nX1 a 0 cell\n.end\n");
  expect(imported.diagnostics.some((d) => d.severity === "warning" && d.message.includes("cell") && d.message.includes("added by Auto Include"))).toBe(true);
  const conflict = await opened(page, () => page.locator("#include-input").setInputFiles({ name: "duplicate.inc", mimeType: "text/plain", buffer: Buffer.from(cell) }));
  expect(conflict.files).toHaveLength(1);
  expect(conflict.diagnostics.some((d) => d.message.includes("conflicting definitions"))).toBe(true);
  await opened(page, () => page.getByRole("button", { name: "Exclude duplicate.inc", exact: true }).click());
  const reloaded = await opened(page, () => page.evaluate(() => window.viewer.command("reload-library")));
  expect(reloaded.files).toHaveLength(2);
  const replacement = cell.replace("cell", "replacement");
  const replaced = await opened(page, () => page.locator("#include-input").setInputFiles({ name: "cell.inc", mimeType: "text/plain", buffer: Buffer.from(replacement) }));
  expect(replaced.files).toHaveLength(1);
  const refreshed = await opened(page, () => page.locator("#file-input").setInputFiles({ name: "fresh.sp", mimeType: "text/plain", buffer: Buffer.from("* fresh\nX2 a 0 replacement\n.end\n") }));
  expect(refreshed.files).toHaveLength(2);
  await page.locator("#tab-netlist").click();
  const disabled = await opened(page, () => page.locator("#auto-include").uncheck());
  expect(disabled.files).toHaveLength(1);
});

test("Auto Include imports the actual nested definition without pulling its testbench into the web document", async ({ page }) => {
  await start(page);
  const results = await page.evaluate(async () => {
    const { startCore } = await import('./core/client.js');
    const core = startCore();
    const files = {
      '/top.scs': 'simulator lang=spectre\nX1 (a b) cell\n// @RNL { #X1 @PLACE(0, 0) }\n',
      '/tb.scs': 'include "top.scs"\ninclude "cells/cell.scs"\nXtb (a b) cell\n// @RNL { #Xtb @PLACE(0, 0) }\n',
      '/cells/cell.scs': 'subckt cell (p n)\nR1 (p n) resistor r=1k\nends cell\n',
    };
    try {
      await core.library({ library: { reload: true }, pool: ['/tb.scs'], files });
      const results = [];
      // A pool source and an independently opened source use different namespaces.
      for (const prefix of ['', '/<pool>']) {
        const request = { entry: '/top.scs', files: {
          ...Object.fromEntries(Object.entries(files).map(([p, text]) => [prefix + p, text])),
          '/top.scs': files['/top.scs'],
        }, pool: [prefix + '/tb.scs'], library_prefix: prefix, auto_include: true, svg: true };
        results.push({ on: await core.build(request), off: await core.build({ ...request, auto_include: false }) });
      }
      return results;
    } finally { core.terminate(); }
  });
  for (const { on, off } of results) {
    expect(on.files).toHaveLength(2);
    expect(on.files.some(f => f.path.endsWith('/cell.scs'))).toBe(true);
    expect(on.files.some(f => f.path.endsWith('tb.scs'))).toBe(false);
    expect(on.objects.find(o => o.name === 'X1').opens).not.toBeNull();
    expect(on.diagnostics.filter(d => d.severity === 'error')).toEqual([]);
    expect(on.diagnostics.some(d => d.message.includes('cells/cell.scs') && d.message.includes('added by Auto Include'))).toBe(true);
    expect(off.files).toHaveLength(1);
    expect(off.objects.find(o => o.name === 'X1').opens).toBeNull();
  }
});

test("New Window hands local files, applied text, visit history and Library to a new tab", async ({ page }, info) => {
  await start(page);
  const top = '* local top\n.include "part.inc"\nX1 a 0 part\n* @RNL { #X1 @PLACE(0, 0) }\n.end\n';
  const part = '* part\n.subckt part p q\nR1 p q 1k\n.ends\n* @RNL(scope="part") { #R1 @PLACE(0, 0) }\n';
  await page.locator("#file-input").setInputFiles({ name: "top.sp", mimeType: "text/plain", buffer: Buffer.from(top) });
  await shown(page);
  await opened(page, () => page.locator("#include-input").setInputFiles({ name: "part.inc", mimeType: "text/plain", buffer: Buffer.from(part) }));
  await page.locator("#tab-netlist").click();
  await page.evaluate(async () => {
    const { select } = await import('./selection.js');
    select(window.viewer.doc.objects.findIndex((o) => o.name === 'X1'), 'drawing');
  });
  await page.locator('#canvas').focus();
  await opened(page, () => page.keyboard.press("e"));
  await expect(page.locator("#level-name")).toHaveText("part");
  await page.keyboard.press("Control+q");
  await page.locator("#source textarea:visible").fill(part.replaceAll("R1", "R7"));
  await opened(page, () => page.keyboard.press("Control+Enter"));
  await page.evaluate(async () => {
    const { select } = await import('./selection.js');
    select(window.viewer.doc.objects.findIndex((o) => o.name === 'R7'), 'drawing');
    (await import('./background.js')).setBackground('dark');
  });
  const before = await page.evaluate(() => ({ level: window.viewer.level, path: window.viewer.path, selected: window.viewer.selected }));
  // Unapplied text must stay in this tab, while its applied predecessor is handed to the child.
  await page.keyboard.press("Control+q");
  await page.locator("#source textarea:visible").fill(part.replaceAll("R1", "R8"));
  await page.getByRole("menuitem", { name: "File", exact: true }).click();
  const popup = page.waitForEvent("popup");
  await page.locator('[data-id="new-window"]').click();
  const child = await popup;
  try {
    await expect(child.locator("#level-name")).toHaveText("part");
    await expect.poll(() => child.evaluate(() => window.viewer.path)).toEqual(before.path);
    expect(await child.evaluate(() => window.viewer.doc.objects[window.viewer.selected].name)).toBe("R7");
    expect(await child.evaluate(() => window.viewer.doc.files.some((f) => f.text.includes("R8")))).toBe(false);
    await expect(child.locator("#pool-list .pool-row")).toHaveCount(2);
    expect(await child.evaluate(async () => (await import('./util.js')).load('background'))).toBe("dark");
    expect(await page.evaluate(() => ({ level: window.viewer.level, path: window.viewer.path, selected: window.viewer.selected }))).toEqual(before);
    if (process.env.RNL_NAV_SCREENSHOTS && info.project.name === "chromium") {
      await child.screenshot({ path: path.join(process.env.RNL_NAV_SCREENSHOTS, "web-new-tab.png") });
      fs.writeFileSync(path.join(process.env.RNL_NAV_SCREENSHOTS, "web-tabs.json"), JSON.stringify({ parent: page.url(), child: child.url(), path: before.path }, null, 2));
    }
  } finally { await child.close(); }
});

test("Shift double click opens a subcircuit in a real new tab and pop-up blocking is explained", async ({ page }, info) => {
  await start(page, "?url=samples/cases/04-hierarchy.sp");
  await shown(page);
  await page.locator("#canvas").focus();
  await page.keyboard.press("f");
  const target = await page.evaluate(async () => {
    const { view } = await import('./drawing.js');
    const { doc, level } = window.viewer;
    const hit = doc.levels[level].hits.find((h) => h.kind === 'box' && h.objects.some((o) => doc.objects[o].name === 'X2'));
    const [a, b] = hit.points;
    const canvas = document.getElementById('canvas'), rect = canvas.getBoundingClientRect();
    return { x: rect.left + ((a[0] + b[0]) / 2 - view.x) * view.k, y: rect.top + ((a[1] + b[1]) / 2 - view.y) * view.k };
  });
  const before = await page.evaluate(() => ({ path: window.viewer.path, selected: window.viewer.selected }));
  const popup = page.waitForEvent("popup");
  await page.keyboard.down("Shift");
  await page.mouse.dblclick(target.x, target.y);
  await page.keyboard.up("Shift");
  const child = await popup;
  try {
    await expect(child.locator("#level-name")).toHaveText("amp");
    expect(await page.evaluate(() => ({ path: window.viewer.path, selected: window.viewer.selected }))).toEqual(before);
    if (process.env.RNL_NAV_SCREENSHOTS && info.project.name === "chromium") await child.screenshot({ path: path.join(process.env.RNL_NAV_SCREENSHOTS, "shift-double-click-new-window.png") });
  } finally { await child.close(); }
  await page.evaluate(() => { window.open = () => null; });
  await page.getByRole("menuitem", { name: "File", exact: true }).click();
  await page.locator('[data-id="new-window"]').click();
  await expect(page.locator("dialog.dialog")).toContainText("The browser blocked the new viewer tab");
});

const visitFiles = {
  'visit-top.sp': '* top\n.include "cell.inc"\n.include "leaf.inc"\nXcell a 0 cell\n* @RNL { #Xcell @PLACE(0, 0) }\n.end\n',
  'cell.inc': '* cell\n.subckt cell p n\nXleaf p n leaf\n* @RNL { #Xleaf @PLACE(0, 0) }\n.ends\n',
  'leaf.inc': '* leaf\n.subckt leaf p n\nRleaf p n 1k\n* @RNL { #Rleaf @PLACE(0, 0) }\n.ends\n',
};
const descendNamed = (page, name) => page.evaluate(async name => {
  const o = window.viewer.doc.objects.findIndex(o => o.kind === 'instance' && o.name === name);
  await (await import('./subcircuits.js')).descend(o);
}, name);
const visitTitles = page => page.locator('#level-path .crumb').evaluateAll(es => es.map(e => e.querySelector('.name, #level-name')?.textContent));

for (const method of ['descend', 'tree', 'index', 'source', 'include']) {
  test(`cross-file ${method} opens independently, joins Files and starts the correct history`, async ({page}) => {
    await page.addInitScript(() => localStorage.setItem('auto-include', 'false'));
    await start(page);
    await opened(page, () => page.locator('#file-input').setInputFiles(Object.entries(visitFiles).map(([name,text]) =>
      ({name,mimeType:'text/plain',buffer:Buffer.from(text)}))));
    await expect(page.locator('#pool-list .pool-row')).toHaveCount(1);
    await page.evaluate(() => window.viewer.command('zoom-in'));
    const view = await page.locator('#canvas > svg').getAttribute('viewBox');
    const child = await opened(page, async () => {
      if (method === 'descend') return descendNamed(page, 'Xcell');
      if (method === 'tree' || method === 'index') {
        await page.locator('#tab-hierarchy').click();
        return page.locator(method === 'tree' ? '.tree-row[data-path="0/0"]' : '.index-row').filter({hasText:/cell/}).first().click();
      }
      await page.locator('#tab-netlist').click();
      if (method === 'include') return page.locator('.include-link').filter({hasText:'cell.inc'}).click();
      await showSource(page, 'cell.inc');
      return page.locator('#source .file:not([hidden]) .obj').filter({hasText:'Xleaf'}).first().click();
    });
    expect(child.path).toBe('/cell.inc');
    expect(child.files.map(f => f.name)).toEqual(['cell.inc']);
    expect(child.objects.find(o => o.name === 'Xleaf').opens).toBeNull();
    expect(child.levels.some(l => l.title === 'leaf')).toBe(false);
    await expect(page.locator('#pool-list .pool-row')).toHaveCount(2);
    expect(await visitTitles(page)).toEqual(method === 'descend' ? ['visit-top.sp','cell'] : ['cell']);
    await expect(page.locator('#auto-include')).not.toBeChecked();
    if (method !== 'descend') { await expect(page.locator('#up')).toBeHidden(); return; }
    // Reloading changes include scope but keeps visit history. The Library index may resolve leaf.
    await page.locator('#tab-netlist').click();
    await opened(page, () => page.locator('#include-input').setInputFiles({ name:'leaf.inc', mimeType:'text/plain', buffer:Buffer.from(visitFiles['leaf.inc']) }));
    await page.locator('#tab-netlist').click();
    const auto = await opened(page, () => page.locator('#auto-include').check());
    expect(auto.objects.find(o => o.name === 'Xleaf').opens).not.toBeNull();
    expect(auto.diagnostics.some(d => d.message.includes('added by Auto Include'))).toBe(true);
    await opened(page, () => descendNamed(page, 'Xleaf'));
    expect(await visitTitles(page)).toEqual(['visit-top.sp','cell','leaf']);
    await expect(page.locator('#pool-list .pool-row')).toHaveCount(3);
    await opened(page, () => page.locator('#level-path .crumb[data-depth="0"]').click());
    expect(await visitTitles(page)).toEqual(['visit-top.sp']);
    expect(await page.locator('#canvas > svg').getAttribute('viewBox')).toBe(view);
    expect(await page.evaluate(() => window.viewer.doc.objects[window.viewer.selected].name)).toBe('Xcell');
    await opened(page, () => descendNamed(page, 'Xcell'));
    await opened(page, () => page.locator('#up').click());
    expect(await visitTitles(page)).toEqual(['visit-top.sp']);
    await page.locator('#tab-files').click();
    await opened(page, () => page.getByRole('button', {name:'Open cell.inc',exact:true}).click());
    expect(await visitTitles(page)).toEqual(['cell']);
  });
}

test('a URL descent transfers independent scope and visit history to a new window', async ({page, context}) => {
  await page.addInitScript(() => localStorage.setItem('auto-include', 'false'));
  await context.route('**/visits/*', route => route.fulfill({contentType:'text/plain', body:visitFiles[new URL(route.request().url()).pathname.split('/').pop()] || ''}));
  await start(page, '?url=visits/visit-top.sp');
  await shown(page);
  await page.evaluate(async () => (await import('./selection.js')).select(window.viewer.doc.objects.findIndex(o => o.name === 'Xcell'), 'drawing'));
  const popup = page.waitForEvent('popup');
  await page.keyboard.press('Shift+e');
  const child = await popup;
  try {
    await expect(child.locator('#level-name')).toHaveText('cell');
    const doc = await shown(child);
    expect(doc.path).toBe('/visits/cell.inc');
    expect(doc.files).toHaveLength(1);
    expect(doc.objects.find(o => o.name === 'Xleaf').opens).toBeNull();
    expect(await visitTitles(child)).toEqual(['visit-top.sp','cell']);
    await opened(child, () => child.locator('#up').click());
    expect(await visitTitles(child)).toEqual(['visit-top.sp']);
    expect((await child.evaluate(() => window.viewer.doc.files)).length).toBe(3);
    await opened(child, () => descendNamed(child, 'Xcell'));
    await opened(child, () => child.evaluate(() => window.viewer.command('reload')));
    expect(await visitTitles(child)).toEqual(['visit-top.sp','cell']);
    expect((await child.evaluate(() => window.viewer.doc.files)).length).toBe(1);
    expect(await visitTitles(page)).toEqual(['visit-top.sp']);
  } finally { await child.close(); }
});

test('a transferred URL snapshot can open a child after the server becomes unavailable', async ({page,context}) => {
  const host=await server({'/top.sp':visitFiles['visit-top.sp'],'/cell.inc':visitFiles['cell.inc'],'/leaf.inc':visitFiles['leaf.inc']});
  await start(page, '?url='+encodeURIComponent(host+'/top.sp')); await shown(page);
  const popup=page.waitForEvent('popup');
  await page.evaluate(()=>window.viewer.command('new-window'));
  const child=await popup;
  try {
    await shown(child);
    await context.route(host+'/**',route=>route.abort());
    const d=await opened(child,()=>descendNamed(child,'Xcell'));
    expect(d.path).toBe('/cell.inc');
    expect(d.levels.some(l=>l.title==='cell')).toBe(true);
    expect(await visitTitles(child)).toEqual(['top.sp','cell']);
  } finally {await child.close();}
});

test('canceling a dirty cross-file descent preserves the entry and visit history', async ({page}) => {
  await start(page);
  await opened(page, () => page.locator('#file-input').setInputFiles(Object.entries(visitFiles).map(([name,text]) =>
    ({name,mimeType:'text/plain',buffer:Buffer.from(text)}))));
  await page.locator('#tab-netlist').click();
  await page.keyboard.press('Control+q');
  await page.locator('#source textarea:visible').fill(visitFiles['visit-top.sp']+'* pending\n');
  await page.evaluate(async () => {
    const {descend}=await import('./subcircuits.js');
    window.pendingDescent=descend(window.viewer.doc.objects.findIndex(o=>o.name==='Xcell'));
  });
  await page.getByRole('button',{name:'Cancel',exact:true}).click();
  await page.evaluate(()=>window.pendingDescent);
  expect(await visitTitles(page)).toEqual(['visit-top.sp']);
  expect(await page.evaluate(()=>window.viewer.doc.path)).toBe('/visit-top.sp');
  await expect(page.locator('#pool-list .pool-row')).toHaveCount(1);
  expect(await page.locator('#source textarea:visible').inputValue()).toContain('* pending');
});

test('independent URL child opens follow redirects for relative includes', async ({page}) => {
  const host = await server({
    '/alias/top.sp':'* top\n.include "cell.inc"\nXcell a 0 cell\n.end\n',
    '/actual/cell.inc':'* cell\n.include "parts.inc"\n.subckt cell p n\nXleaf p n leaf\n.ends\n',
    '/actual/parts.inc':visitFiles['leaf.inc'],
  }, {'/alias/cell.inc':'/actual/cell.inc'});
  await start(page, `?url=${encodeURIComponent(host+'/alias/top.sp')}`);
  await shown(page);
  const child = await opened(page, () => descendNamed(page,'Xcell'));
  expect(child.path).toBe('/actual/cell.inc');
  expect(child.objects.find(o=>o.name==='Xleaf').opens).not.toBeNull();
  expect(child.missing).toEqual([]);
});

test("include links independently open the target in this tab and a Shift-opened tab", async ({ page }) => {
  await start(page);
  await opened(page, () => page.locator('#file-input').setInputFiles({name:'top.sp', mimeType:'text/plain',
    buffer:Buffer.from('* Top\n.include "models.inc"\nR1 a 0 1k\n.end\n')}));
  await opened(page, () => page.locator('#include-input').setInputFiles({name:'models.inc', mimeType:'text/plain',
    buffer:Buffer.from('.param bias=1.2\n.model example d\n')}));
  await page.locator('#tab-netlist').click();
  const link = page.locator('#source .file:not([hidden]) .include-link');
  const popup = page.waitForEvent('popup');
  await link.click({modifiers:['Shift']});
  const child = await popup;
  try {
    await expect(child.locator('#netlist-file')).toHaveText(/models\.inc$/);
    await expect(child.locator('#source .file:not([hidden]) .ln').first()).toContainText('.param bias=1.2');
    expect(await child.evaluate(() => window.viewer.doc.files.length)).toBe(1);
    expect(await child.evaluate(() => window.viewer.doc.name)).toBe('models.inc');
    await expect(page.locator('#netlist-file')).toHaveText(/top\.sp$/);
    await link.click();
    await expect(page.locator('#netlist-file')).toHaveText(/models\.inc$/);
    await expect(page.locator('#up')).toBeHidden();
  } finally { await child.close(); }
});

for (const [kind, directive, line] of [
  ['backslash', 'include \\\n  "models.scs"', '3'],
  ['comment', 'include // explanation\n+ "models.scs"', '3'],
  ['continued comment', 'include \\\n // explanation\n+ "models.scs"', '4'],
]) test(`continued Spectre include links point at the filename on its physical line: ${kind}`, async ({page}) => {
  await start(page);
  await opened(page, () => page.locator('#file-input').setInputFiles({name:'top.scs',mimeType:'text/plain',
    buffer:Buffer.from(`simulator lang=spectre\n${directive}\n`)}));
  await opened(page, () => page.locator('#include-input').setInputFiles({name:'models.scs',mimeType:'text/plain',
    buffer:Buffer.from('parameters bias=1.2\n')}));
  await page.locator('#tab-netlist').click();
  const link = page.locator('#source .include-link');
  await expect(link).toHaveText('"models.scs"');
  await expect(link.locator('..').locator('..')).toHaveAttribute('data-line',line);
  await link.click();
  await expect(page.locator('#netlist-file')).toHaveText(/models\.scs$/);
  await expect(page.locator('#source .file:not([hidden]) .ln').first()).toContainText('parameters bias=1.2');
});

test("Shift Library opens a previous URL with its includes, and the child retains URL pool sources", async ({ page }) => {
  await start(page, "?url=samples/cases/04-hierarchy.sp");
  const expected = await shown(page);
  await opened(page, () => page.locator("#file-input").setInputFiles({ name: "other.sp", mimeType: "text/plain", buffer: Buffer.from("R1 a 0 1k\n.end\n") }));
  await page.locator("#tab-files").click();
  const before = await page.evaluate(() => ({ path: window.viewer.path, text: window.viewer.doc.files[0].text }));
  const popup = page.waitForEvent("popup");
  await page.locator("#pool-list .pool-open").filter({ hasText: "04-hierarchy.sp" }).click({ modifiers: ["Shift"] });
  const child = await popup;
  try {
    await expect(child.locator("#level-name")).toHaveText("04-hierarchy.sp");
    expect((await shown(child)).files).toEqual(expected.files);
    expect(await page.evaluate(() => ({ path: window.viewer.path, text: window.viewer.doc.files[0].text }))).toEqual(before);
    await expect(page.locator("#pane-files")).toBeVisible();
    // Switch away and reopen the inherited URL pool row: its relative include still resolves.
    await child.locator("#tab-files").click();
    await opened(child, () => child.locator("#pool-list .pool-open").filter({ hasText: /^other.sp$/ }).click());
    await opened(child, () => child.locator("#pool-list .pool-open").filter({ hasText: "04-hierarchy.sp" }).click());
    expect(await child.evaluate(() => window.viewer.doc.files)).toEqual(expected.files);
  } finally { await child.close(); }
});

// Playwright builds and serves the site when the run can include these tests (playwright.config.js)
test.skip(!process.env.RNL_WEB_URL, "the site is not built: RNL_WEB=0");

/** The site that the web server serves. */
const site = path.join(root, "apps", "web", "dist");

/** Open the web viewer with `query`, such as "?url=...", and wait until it has started. */
async function start(page, query = "") {
  await page.goto(`${process.env.RNL_WEB_URL}/index.html${query}`);
  await expect(page.locator("#menubar")).toBeVisible();
}

/** Wait until a Document is shown that `seen` does not hold, and nothing is being read; return it. */
async function shown(page) {
  await page.waitForFunction(
    () => {
      const doc = window.viewer.doc;
      return doc && !(window.seen && window.seen.has(doc)) && !document.body.classList.contains("busy");
    },
  );
  return page.evaluate(() => window.viewer.doc);
}

/** Run `action`, which opens or reads a document, and return the Document shown after it. */
async function opened(page, action) {
  await page.evaluate(() => {
    window.seen = new WeakSet(window.viewer.doc ? [window.viewer.doc] : []);
  });
  await action();
  return shown(page);
}

/** The Document that `rnl view` prints for a file, named as given, from the repository's root. */
function rnlView(file) {
  const out = execFileSync(process.env.RNL_BIN, ["view", file], { cwd: root, encoding: "utf8", maxBuffer: 1 << 28 });
  return JSON.parse(out);
}

/**
 * A Document without what names its files in the workspace: the entry's path, and the paths of the
 * files missing, which keep only where they are from the entry's directory, such as ../x.inc.
 */
function unnamed(doc) {
  const names = (p) => p.replaceAll("\\", "/").split("/").filter(Boolean);
  const dir = names(doc.path).slice(0, -1);
  const fromDir = (file) => {
    const to = names(file);
    let k = 0;
    while (k < dir.length && k < to.length - 1 && to[k] === dir[k]) k += 1;
    return [...dir.slice(k).map(() => ".."), ...to.slice(k)].join("/");
  };
  return { ...doc, path: null, missing: doc.missing.map(fromDir) };
}

/**
 * The Document without the drawings of the objects that have no position, as the web viewer asks
 * the core for it (`lazy_unplaced`) and then asks for those of the rows its list shows. The
 * equality test (bindings/runtime-wasm/tests/equality.mjs) compares them with `rnl view`'s.
 */
function lazily(doc) {
  const levels = doc.levels.map((l) => ({ ...l, unplaced: l.unplaced.map(({ svg, primitives, ...u }) => u) }));
  return { ...doc, levels };
}

/** The same Document as `rnl view` builds for `file`, but for the paths that name the files and the drawings of the objects without a position. */
function sameAsRnl(doc, file) {
  expect(unnamed(doc)).toEqual(unnamed(lazily(rnlView(file))));
}

/** The directories the test made, removed after it, and the servers it started, stopped after it. */
const dirs = [];
const servers = [];
test.afterEach(async () => {
  for (const dir of dirs.splice(0)) fs.rmSync(dir, { recursive: true, force: true });
  for (const s of servers.splice(0)) {
    s.closeAllConnections();
    await new Promise((resolve) => s.close(resolve));
  }
});

/**
 * A web server of the test's own on 127.0.0.1, which lets any page read what it serves (CORS): each
 * of `files` by its path, and a redirect for each path of `moved` to the path it gives. Its address.
 */
async function server(files, moved = {}) {
  const s = http.createServer((request, response) => {
    const at = decodeURIComponent(new URL(request.url, "http://host").pathname);
    response.setHeader("Access-Control-Allow-Origin", "*");
    if (at in moved) response.writeHead(302, { Location: moved[at] }).end();
    else if (at in files) response.writeHead(200, { "Content-Type": "text/plain" }).end(files[at]);
    else response.writeHead(404).end();
  });
  servers.push(s);
  await new Promise((resolve) => s.listen(0, "127.0.0.1", resolve));
  return `http://127.0.0.1:${s.address().port}`;
}

/** A new directory under `parent` with files, each a text or bytes by its path in it. */
function folder(files, parent = os.tmpdir()) {
  const dir = fs.mkdtempSync(path.join(parent, "rnl-web-"));
  dirs.push(dir);
  write(dir, files);
  return dir;
}

/** Write files into a directory, each a text or bytes by its path in it. */
function write(dir, files) {
  for (const [name, content] of Object.entries(files)) {
    fs.mkdirSync(path.dirname(path.join(dir, name)), { recursive: true });
    fs.writeFileSync(path.join(dir, name), content);
  }
}

/** What the page shows of the Document: the drawing, the text, the problems and the hierarchy. */
async function showsDocument(page, doc) {
  await expect(page).toHaveTitle(`${doc.name} - RNL Viewer`);
  // the drawing of the first level, the SVG the core wrote
  await expect(page.locator("#level-name")).toHaveText(doc.levels[0].title);
  const groups = doc.levels[0].svg.split("<g data-id=").length - 1;
  await expect(page.locator("#canvas > svg g[data-id]")).toHaveCount(groups);
  // the text: a tab for each file, and the lines of the first
  await expect(page.locator("#netlist-file")).toHaveText(doc.path);
  const lines = doc.files[0].text.replace(/\r?\n$/, "").split(/\r?\n/).length;
  await expect(page.locator("#source .file:not([hidden]) .ln")).toHaveCount(lines);
  // Core diagnostics retain their counts; viewer-only package remedies follow the relevant error.
  await expect(page.locator("#diags .diag:not(.package-hint)")).toHaveCount(doc.diagnostics.length + (doc.failed ? 1 : 0));
  // the hierarchy: the top level, and every subcircuit in the index
  await page.locator("#tab-hierarchy").click();
  await expect(page.locator("#tree .tree-row").first()).toContainText(doc.levels[0].title);
  await expect(page.locator("#index .index-row:not(.top)")).toHaveCount(doc.levels.length - 1);
  await page.locator("#tab-netlist").click();
}

/** Netlists of the samples, by their paths under samples/ and tests/conformance/. */
const SAMPLES = ["cases/04-hierarchy.sp", "cases/01-rc-lowpass.sp", "cases/12-diagnostics.sp", "equivalence/inverter-chain.scs"];

for (const file of SAMPLES) {
  test(`?url=samples/${file} opens it as rnl view does`, async ({ page }) => {
    await start(page, `?url=samples/${file}`);
    const doc = await shown(page);
    expect(doc.path).toBe(`/samples/${file}`);
    sameAsRnl(doc, `tests/conformance/${file}`);
    await showsDocument(page, doc);
    await page.locator("#tab-files").click();
    await expect(page.locator("#pool-list .pool-row.current")).toHaveAttribute("data-path", `/samples/${file}`);
    await expect(page.locator("#pool-list .pool-row.current")).toHaveAttribute("aria-current", "true");
    // the core builds in a Worker
    expect(page.workers().map((w) => new URL(w.url()).pathname)).toContain("/core/worker.js");
  });
}

test("a URL's include is read from its server, and the levels, files and problems are shown", async ({ page }) => {
  await start(page, "?url=samples/cases/04-hierarchy.sp");
  const doc = await shown(page);
  expect(doc.files.map((f) => f.path)).toEqual(["04-hierarchy.sp", "04-hierarchy-lib.inc"]);
  expect(doc.missing).toEqual([]);
  await page.locator("#tab-hierarchy").click();
  await expect(page.locator("#tree .tree-row")).toHaveText(["04-hierarchy.sptop", "X1amp", "X2amp", "X3rdiv"]);
  // the include's subcircuit opens, and its file is shown
  await page.locator("#index .index-row", { hasText: "rdiv" }).click();
  await expect(page.locator("#level-name")).toHaveText("rdiv");
  await page.locator("#tab-netlist").click();
  await showSource(page, "04-hierarchy-lib.inc");
  const first = doc.files[1].text.split(/\r?\n/)[0];
  await expect(page.locator("#source .file:not([hidden]) .ln").first()).toContainText(first);

  // the problems of 12-diagnostics, with their files and lines
  await start(page, "?url=samples/cases/12-diagnostics.sp");
  const diagnostics = await shown(page);
  const errors = diagnostics.diagnostics.filter((d) => d.severity === "error");
  expect(errors.length).toBeGreaterThan(0);
  await expect(page.locator("#counts .n-error")).toHaveText(`${errors.length} ${errors.length === 1 ? "error" : "errors"}`);
  const [e] = errors;
  await expect(page.locator("#diags .diag.error .loc").first()).toHaveText(`${e.file}:${e.line}${e.col ? `:${e.col}` : ""}`);
  expect(diagnostics.diagnostics.filter((d) => d.line > 0).every((d) => d.file_index === 0)).toBe(true);
});

test("a URL that redirects opens as the one it redirects to, with its includes above it", async ({ page }) => {
  const base = await server(
    {
      "/a/b/top.sp": "* top\n.include \"x.inc\"\n.include \"../lib.inc\"\nX1 a 0 inner\nX2 a 0 outer\n.end\n",
      "/a/b/x.inc": "* x\n.subckt inner p q\nR1 p q 1k\n.ends\n",
      "/a/lib.inc": "* lib\n.subckt outer p q\nR2 p q 1k\n.ends\n",
    },
    { "/short": "/a/b/top.sp", "/top.sp": "/a/b/top.sp" },
  );
  await start(page, `?url=${base}/a/b/top.sp`);
  const doc = await shown(page);
  expect(doc.path).toBe("/a/b/top.sp");
  expect(doc.files.map((f) => f.path)).toEqual(["top.sp", "x.inc", "/a/lib.inc"]);
  expect(doc.missing).toEqual([]);
  expect(doc.diagnostics.filter((d) => d.severity === "error")).toEqual([]);
  // a short address, and a netlist at the server's root, that the server sends on to it
  for (const from of ["/short", "/top.sp"]) {
    await start(page, `?url=${base}${from}`);
    expect(await shown(page)).toEqual(doc);
  }
  await showsDocument(page, doc);
});

test("a netlist served at the root of its server is named after the server", async ({ page }) => {
  // named as the page names it, for rnl view
  const files = {
    "127.0.0.1": "* top\n.include \"x.inc\"\nX1 a 0 inner\n.end\n",
    "x.inc": "* x\n.subckt inner p q\nR1 p q 1k\n.ends\n",
  };
  const base = await server({ "/": files["127.0.0.1"], "/x.inc": files["x.inc"] });
  await start(page, `?url=${base}/`);
  const doc = await shown(page);
  expect([doc.path, doc.name]).toEqual(["/127.0.0.1", "127.0.0.1"]);
  expect(doc.files.map((f) => f.path)).toEqual(["127.0.0.1", "x.inc"]);
  // every problem is at a line of one of the files
  expect(doc.diagnostics.length).toBeGreaterThan(0);
  for (const d of doc.diagnostics) expect(doc.files[d.file_index]?.name).toBe(d.file);
  sameAsRnl(doc, path.join(folder(files), "127.0.0.1"));
  await showsDocument(page, doc);
});

test("files set on the file input open as with Open, and the page asks which is the netlist", async ({ page }) => {
  await start(page);
  const cases = (...names) => names.map((name) => path.join(root, "tests", "conformance", "cases", name));
  // a netlist and its include: the include is mostly included, so the netlist opens
  let doc = await opened(page, () =>
    page.locator("#file-input").setInputFiles(cases("04-hierarchy.sp", "04-hierarchy-lib.inc")),
  );
  expect(doc.path).toBe("/04-hierarchy.sp");
  expect(doc.files.map((f) => f.path)).toEqual(["04-hierarchy.sp", "04-hierarchy-lib.inc"]);
  sameAsRnl(doc, "tests/conformance/cases/04-hierarchy.sp");
  await showsDocument(page, doc);

  // two netlists: the page asks, the netlists first, and opens the one chosen
  await page.locator("#file-input").setInputFiles(cases("04-hierarchy-lib.inc", "12-diagnostics.sp", "01-rc-lowpass.sp"));
  const dialog = page.locator("dialog.dialog");
  await expect(dialog.locator(".dialog-title")).toHaveText("Open");
  await expect(dialog.locator(".dialog-option")).toHaveText(["01-rc-lowpass.sp", "12-diagnostics.sp", "04-hierarchy-lib.inc"]);
  doc = await opened(page, async () => {
    await dialog.locator(".dialog-option", { hasText: "12-diagnostics.sp" }).click();
    await dialog.getByRole("button", { name: "Open" }).click();
  });
  await expect(dialog).toHaveCount(0);
  expect(doc.path).toBe("/12-diagnostics.sp");
  sameAsRnl(doc, "tests/conformance/cases/12-diagnostics.sp");

  // cancelled, the dialog opens nothing
  await page.locator("#file-input").setInputFiles(cases("12-diagnostics.sp", "01-rc-lowpass.sp"));
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await expect(dialog).toHaveCount(0);
  expect(await page.evaluate(() => window.viewer.doc.path)).toBe("/12-diagnostics.sp");
});

test("Include Folder adds netlists and packages, and clicking a pooled file opens it", async ({ page }) => {
  await start(page);
  const dir = folder({
    "top.sp": "* top\n.include \"lib/parts.inc\"\nX1 a 0 part\n.end\n",
    "lib/parts.inc": ".subckt part p q\nR1 p q 1k\n.ends\n",
    "notes.txt": "notes\n",
  });
  const setting = page.locator("#folder-input").setInputFiles(dir).catch(() => {});
  const name = path.basename(dir);
  const row = page.locator("#pool-list .pool-row", { has: page.getByRole("button", { name: `Open ${name}/top.sp`, exact: true }) });
  await expect(row).toBeVisible();
  await Promise.race([setting, new Promise((resolve) => setTimeout(resolve, 3000))]);
  expect(await page.evaluate(() => window.viewer.doc)).toBeNull();
  await expect(page.locator("#pool-list")).not.toContainText("notes.txt");
  const doc = await opened(page, () => row.locator(".pool-open").click());
  expect(doc.path).toBe(`/${name}/top.sp`);
  expect(doc.files.map((f) => f.path)).toEqual(["top.sp", "lib/parts.inc"]);
  sameAsRnl(doc, path.join(dir, "top.sp"));
  await showsDocument(page, doc);
});

test("files dropped on the page open as with Open", async ({ browserName, page }) => {
  // A drop made in the page: Chromium gives its items no entries, so the page opens the files as
  // with Open, and Firefox gives them entries that read the files, as a drop from the computer does.
  // WebKit gives them entries whose files cannot be read (NotFoundError), which a drop from the
  // computer does not.
  test.skip(browserName === "webkit", "WebKit's entries of a drop made in the page cannot read their files");
  await start(page);
  const files = {
    "top.sp": "* top\n.include \"parts.inc\"\nX1 a 0 part\n.end\n",
    "parts.inc": ".subckt part p q\nR1 p q 1k\n.ends\n",
  };
  const dataTransfer = await page.evaluateHandle((files) => {
    const transfer = new DataTransfer();
    for (const [name, text] of Object.entries(files)) transfer.items.add(new File([text], name));
    return transfer;
  }, files);
  const doc = await opened(page, () => page.dispatchEvent("#app", "drop", { dataTransfer }));
  await expect(page.locator("#pool-list .pool-row.current")).toHaveCount(1);
  expect(doc.path).toBe("/top.sp");
  expect(doc.files.map((f) => f.path)).toEqual(["top.sp", "parts.inc"]);
  sameAsRnl(doc, path.join(folder(files), "top.sp"));
  await showsDocument(page, doc);
});

test("a Latin-1 file is read as rnl reads it", async ({ page }) => {
  await start(page);
  const [micro, degree] = [0xb5, 0xb0].map((c) => String.fromCharCode(c));
  const bytes = Buffer.from(`* 10${micro}s at 25${degree}C\nR1 a 0 1k\n.end\n`, "latin1");
  const doc = await opened(page, () =>
    page.locator("#file-input").setInputFiles({ name: "latin1.sp", mimeType: "text/plain", buffer: bytes }),
  );
  expect(doc.files[0].text.split("\n")[0]).toBe(`* 10${micro}s at 25${degree}C`);
  sameAsRnl(doc, path.join(folder({ "latin1.sp": bytes }), "latin1.sp"));
  await expect(page.locator("#source .ln").first()).toContainText(`10${micro}s at 25${degree}C`);
});

test("a netlist of subcircuits alone opens on the one nothing calls, as the core reads it", async ({ page }) => {
  await start(page);
  const open = (name, lines) => {
    const text = lines.concat("").join(String.fromCharCode(10));
    return { text, file: { name, mimeType: "text/plain", buffer: Buffer.from(text) } };
  };
  const [name, lines] = UNCALLED.uncalled;
  const { text, file } = open(name, lines);
  const doc = await opened(page, () => page.locator("#file-input").setInputFiles(file));
  sameAsRnl(doc, path.join(folder({ [name]: text }), name));
  expect(doc.levels.map((l) => l.title)).toEqual(["uncalled.sp", "ota_5t", "Testbench"]);
  // the top level has no objects: the testbench opens, drawn, and the status bar says why
  await expect(page.locator("#level-name")).toHaveText("Testbench");
  await expect(page.locator("#notice")).toHaveText("The top level is empty; showing Testbench.");
  await expect(page.locator("#canvas > svg g[data-id]")).not.toHaveCount(0);
  await page.locator("#tab-hierarchy").click();
  await expect(page.locator("#tree .tree-row")).toHaveText(["uncalled.sptop", "Testbenchnot called", "XDUTota_5t"]);
  await page.locator('#tree .tree-row[data-path="r2/0"]').click();
  await expect(page.locator("#level-name")).toHaveText("ota_5t");
  await expect(page.locator("#notice")).toBeHidden();

  // a file with no object at all says so, and has no hierarchy to point to
  const empty = open(...UNCALLED.nothing);
  await opened(page, () => page.locator("#file-input").setInputFiles(empty.file));
  await expect(page.locator("#level-name")).toHaveText("nothing.sp");
  await expect(page.locator("#empty")).toHaveText("This level has no objects, so nothing is drawn.");
});

test("an include that is not there is missing, and a problem at its line", async ({ page }) => {
  await start(page);
  const text = "* top\n.include \"gone.inc\"\nR1 a 0 1k\n.end\n";
  const doc = await opened(page, () =>
    page.locator("#file-input").setInputFiles({ name: "top.sp", mimeType: "text/plain", buffer: Buffer.from(text) }),
  );
  expect(doc.missing).toEqual(["/gone.inc"]);
  const problem = doc.diagnostics.find((d) => d.message.includes("gone.inc cannot be found"));
  expect(problem).toMatchObject({ severity: "error", file: "top.sp", file_index: 0, line: 2 });
  sameAsRnl(doc, path.join(folder({ "top.sp": text }), "top.sp"));
  await expect(page.locator("#diags .diag.error .msg")).toContainText(["gone.inc cannot be found"]);
  await expect(page.locator("#source .ln").nth(1)).toHaveClass(/\bd-error\b/);
});

test("an include above the files opened is missing, not a file of that name opened with them", async ({ page }) => {
  await start(page);
  // top.sp and x.inc chosen in sub/, whose top.sp includes the x.inc above it, which is not there
  const dir = folder({
    "sub/top.sp": "* top\n.include \"../x.inc\"\nX1 a 0 inner\n.end\n",
    "sub/x.inc": "* sub's own x.inc\n.subckt inner p q\nR1 p q 1k\n.ends\n",
  });
  const doc = await opened(page, () =>
    page.locator("#file-input").setInputFiles(["top.sp", "x.inc"].map((name) => path.join(dir, "sub", name))),
  );
  expect(doc.path).toBe("/top.sp");
  expect(doc.files.map((f) => f.path)).toEqual(["top.sp"]);
  expect(doc.missing).toEqual(["/../x.inc"]);
  const problem = doc.diagnostics.find((d) => d.message === "the included file ../x.inc cannot be found");
  expect(problem).toMatchObject({ severity: "error", file: "top.sp", file_index: 0, line: 2 });
  sameAsRnl(doc, path.join(dir, "sub", "top.sp"));
});

test("Reload reads a served netlist again after it changed, with an include that now is there", async ({ page }) => {
  // a directory of the site, served like the samples
  const dir = folder({ "top.sp": "* top\n.include \"parts.inc\"\nR1 a 0 1k\n* @RNL {#R1 @PLACE(0, 0)}\n.end\n" }, site);
  const name = path.basename(dir);
  await start(page, `?url=${name}/top.sp`);
  let doc = await shown(page);
  expect(doc.missing).toEqual([`/${name}/parts.inc`]);
  expect(doc.diagnostics.some((d) => d.message.includes("parts.inc cannot be found"))).toBe(true);
  sameAsRnl(doc, path.join(dir, "top.sp"));

  write(dir, {
    "top.sp": "* top, changed\n.include \"parts.inc\"\nR1 a 0 2k\nX1 a 0 part\n* @RNL {#R1 @PLACE(0, 0)}\n.end\n",
    "parts.inc": ".subckt part p q\nR2 p q 1k\n.ends\n",
  });
  doc = await opened(page, () => page.keyboard.press("F5"));
  expect(doc.missing).toEqual([]);
  expect(doc.files.map((f) => f.path)).toEqual(["top.sp", "parts.inc"]);
  expect(doc.files[0].text).toContain("R1 a 0 2k");
  sameAsRnl(doc, path.join(dir, "top.sp"));
  await showsDocument(page, doc);
  // the page was not loaded again
  expect(await page.evaluate(() => !!window.seen)).toBe(true);
});

test("Help > About names the version and the commit the site was built from", async ({ page }) => {
  await start(page, "?url=samples/cases/01-rc-lowpass.sp");
  await shown(page);
  // what build.mjs wrote: the workspace's version, and the commit when git knows it
  const about = JSON.parse(fs.readFileSync(path.join(site, "about.json"), "utf8"));
  expect(about.version).toMatch(/^\d+\.\d+\.\d+/);
  await page.locator("#menubar .menu-title", { hasText: "Help" }).click();
  await page.locator('#menubar .menu-item[data-id="about"]').click();
  const dialog = page.locator("dialog.dialog");
  await expect(dialog.locator(".dialog-title")).toHaveText("About RNL Viewer");
  await expect(dialog.locator(".dialog-name")).toHaveText(`RNL Viewer ${about.version}${about.commit ? ` (${about.commit})` : ""}`);
  await dialog.getByRole("button", { name: "Close" }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.locator("#level-name")).toHaveText("01-rc-lowpass.sp");
});

test("Help > Desktop Viewer opens the site's page of the tools in a new tab", async ({ context, page }) => {
  // the tab is given a page of the test's own instead of the site's
  await context.route("https://rich-netlist.github.io/**", (route) =>
    route.fulfill({ contentType: "text/html", body: "<!doctype html><title>RNL tools</title>" }),
  );
  await start(page, "?url=samples/cases/01-rc-lowpass.sp");
  await shown(page);
  await page.locator("#menubar .menu-title", { hasText: "Help" }).click();
  const [tab] = await Promise.all([
    context.waitForEvent("page"),
    page.locator('#menubar .menu-item[data-id="desktop-viewer"]').click(),
  ]);
  // Playwright's browsers prefer en-US, so the English page
  await expect(tab).toHaveURL("https://rich-netlist.github.io/rnl/en/tools.html");
  expect(await tab.evaluate(() => window.opener)).toBe(null);
  await tab.close();
  await expect(page.locator("#level-name")).toHaveText("01-rc-lowpass.sp");
});

test("File > Load from Clipboard opens the text alone, as rnl view reads it in a file of its own", async ({ page }) => {
  const text = fs.readFileSync(path.join(root, "tests", "conformance", "cases", "04-hierarchy.sp"), "utf8");
  await start(page);
  // the test gives the page the clipboard's text, as a browser would once it lets the page read it
  await page.evaluate((t) => {
    navigator.clipboard.readText = async () => t;
  }, text);
  const doc = await opened(page, async () => {
    await page.locator("#menubar .menu-title", { hasText: "File" }).click();
    await page.locator('#menubar .menu-item[data-id="load-clipboard"]').click();
    await page.locator("dialog.dialog").getByRole("button", { name: "Load" }).click();
  });
  expect([doc.name, doc.files.length, doc.missing]).toEqual(["clipboard.sp", 1, ["/04-hierarchy-lib.inc"]]);
  sameAsRnl(doc, path.join(folder({ "clipboard.sp": text }), "clipboard.sp"));
  await showsDocument(page, doc);
});

test("the Not drawn list asks the core for the drawings of its rows, which the Document does not carry", async ({ page }) => {
  const text = "* none placed\nR1 a b 1k\nC1 b 0 1p\nM1 d g s 0 nmos\n.model nmos nmos\n.end\n";
  await start(page);
  await page.evaluate((t) => {
    navigator.clipboard.readText = async () => t;
  }, text);
  const doc = await opened(page, async () => {
    await page.locator("#menubar .menu-title", { hasText: "File" }).click();
    await page.locator('#menubar .menu-item[data-id="load-clipboard"]').click();
    await page.locator("dialog.dialog").getByRole("button", { name: "Load" }).click();
  });
  const listed = doc.levels[0].unplaced;
  expect(listed.length).toBeGreaterThanOrEqual(3);
  expect(listed.some((u) => u.svg || u.primitives)).toBe(false);
  await page.locator("#tab-unplaced").click();
  const rows = page.locator("#unplaced-list .unplaced-row");
  await expect(rows).toHaveCount(listed.length);
  await expect(rows.locator(".thumb svg")).toHaveCount(listed.length);
  // each is drawn with its own symbol and the nets at its pins
  const nets = rows.filter({ has: page.locator(".name", { hasText: /^M1$/ }) }).locator(".thumb svg text");
  expect((await nets.allTextContents()).sort()).toEqual(["0", "d", "g", "s"]);
});

test("the core's labels are clicked and marked, the pointer shows names, and a click on two objects lists them", async ({ page }) => {
  await start(page);
  await page.evaluate((t) => {
    navigator.clipboard.readText = async () => t;
  }, PICK.concat("").join("\n"));
  const doc = await opened(page, async () => {
    await page.locator("#menubar .menu-title", { hasText: "File" }).click();
    await page.locator('#menubar .menu-item[data-id="load-clipboard"]').click();
    await page.locator("dialog.dialog").getByRole("button", { name: "Load" }).click();
  });
  const labels = doc.levels[0].hits.filter((h) => h.kind === "label");
  expect(labels.map((h) => h.id).sort()).toEqual(["#C1::name", "#C1::value", "#R1::name", "#R1::value", "#R2::name", "#R2::value", "a text"]);
  expect(labels.filter((h) => h.objects.length === 0).map((h) => h.id)).toEqual(["a text"]);
  /** The page point of a point of the drawing, in its pixels. */
  const point = ([u, v]) =>
    page.evaluate(
      ([u, v]) => {
        const canvas = document.getElementById("canvas");
        const [x, y, w] = document.querySelector("#canvas > svg").getAttribute("viewBox").split(" ").map(Number);
        const r = canvas.getBoundingClientRect();
        const k = canvas.clientWidth / w;
        return { x: r.left + (u - x) * k, y: r.top + (v - y) * k };
      },
      [u, v],
    );
  const middle = (h) => [(h.points[0][0] + h.points[1][0]) / 2, (h.points[0][1] + h.points[1][1]) / 2];
  const selected = () => page.evaluate(() => (window.viewer.selected === null ? null : window.viewer.doc.objects[window.viewer.selected].name));
  // a click on C1's value selects C1, and its two labels are marked
  const value = await point(middle(labels.find((h) => h.id === "#C1::value")));
  await page.mouse.click(value.x, value.y);
  await expect.poll(selected).toBe("C1");
  await expect(page.locator("#canvas svg .overlay rect.label")).toHaveCount(2);
  // the pointer at rest on C1's symbol shows its name
  const c1 = await point(middle(doc.levels[0].hits.find((h) => h.kind === "box" && h.objects.some((o) => doc.objects[o].name === "C1"))));
  await page.mouse.move(c1.x, c1.y);
  await expect(page.locator("#hover-tip")).toHaveText("C1");
  // a click where R1's box and R2's overlap lists both
  const box = (name) => doc.levels[0].hits.find((h) => h.kind === "box" && h.objects.some((o) => doc.objects[o].name === name)).points;
  const [[a0, b0], [a1, b1]] = [box("R1"), box("R2")];
  const both = await point([(Math.max(a0[0], a1[0]) + Math.min(b0[0], b1[0])) / 2, (Math.max(a0[1], a1[1]) + Math.min(b0[1], b1[1])) / 2]);
  await page.mouse.click(both.x, both.y);
  await expect(page.locator("#pick-list .pick-row")).toHaveText(["instanceR1", "instanceR2"]);
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect.poll(selected).toBe("R2");
});

test("the text edited and applied is built by the core as rnl view builds the same files, and Save downloads the file shown", async ({ page }) => {
  const top = "* top\n.include \"parts.inc\"\nR1 a 0 1k\nX1 a 0 part\n* @RNL {#R1 @PLACE(0, 0)}\n.end\n";
  const parts = ".subckt part p q\nR2 p q 1k\n.ends\n";
  const dir = folder({ "top.sp": top, "parts.inc": parts }, site);
  await start(page, `?url=${path.basename(dir)}/top.sp`);
  await shown(page);
  // edit the netlist and its include, each in the text box of its own tab
  await page.keyboard.press("Control+q");
  const area = page.locator("#source .editor:not([hidden]) textarea");
  const editedTop = top.replace("R1 a 0 1k", "R1 a 0 2k");
  const editedParts = parts.replace("R2", "R7");
  await area.fill(editedTop);
  await showSource(page, "parts.inc");
  await area.fill(editedParts);
  await expect(page.locator("#edit-state")).toHaveText("Changes not applied yet");
  // applied, the core builds the files with the texts in place of what is read, as it builds the same texts from disk
  const doc = await opened(page, () => page.locator("#apply-edits").click());
  expect(doc.files.map((f) => [f.path, f.text])).toEqual([["top.sp", editedTop], ["parts.inc", editedParts]]);
  expect(doc.objects.some((o) => o.kind === "instance" && o.name === "R7")).toBe(true);
  sameAsRnl(doc, path.join(folder({ "top.sp": editedTop, "parts.inc": editedParts }), "top.sp"));
  await expect(page.locator("#source .editor")).toHaveCount(0);
  // the server's files are as they were: nothing was written
  expect(fs.readFileSync(path.join(dir, "parts.inc"), "utf8")).toBe(parts);
  // Save downloads the file shown, as the text box holds it
  await page.evaluate(() => window.viewer.command('language-zh'));
  const [download] = await Promise.all([page.waitForEvent("download"), page.keyboard.press("Control+s")]);
  expect(download.suggestedFilename()).toBe("parts.inc");
  expect(fs.readFileSync(await download.path(), "utf8")).toBe(editedParts);
  await expect(page.locator("#note")).toHaveText("已下载 parts.inc。");
  await expect(page.locator('dialog.dialog')).toHaveCount(0);
  await page.evaluate(() => window.viewer.command('language-en'));
  await expect(page.locator("#note")).toHaveText("Downloaded parts.inc.");
  // the file downloaded is what was read from now on; the netlist's change is still not saved
  await expect(page.locator("#netlist-file[data-unsaved='true']")).toHaveText(/parts\.inc$/);
});


test("Save before Apply retains a newly typed USE-PACKAGE through repeated edit and library rebuilds", async ({ page }) => {
  await start(page);
  const original = '* save before apply\n* @RNL(type="header") { @VERSION("0.3") }\nR1 a 0 1k\n.end\n';
  const edited = original.replace('@VERSION("0.3")', '@VERSION("0.3"); @USE-PACKAGE(name="save-regression", version="0.1")');
  await opened(page, () => page.locator('#file-input').setInputFiles({name:'save-first.sp',mimeType:'text/plain',buffer:Buffer.from(original)}));
  await page.keyboard.press('Control+q');
  const area = page.locator('#source .editor:not([hidden]) textarea');
  await area.fill(edited);
  const [download] = await Promise.all([page.waitForEvent('download'), page.keyboard.press('Control+s')]);
  expect(fs.readFileSync(await download.path(),'utf8')).toBe(edited);
  await expect(page.locator('#note')).toHaveText('Downloaded save-first.sp.');
  expect(await page.evaluate(() => window.viewer.doc.files[0].text)).toBe(original);
  const applied = await opened(page, () => page.keyboard.press('Control+q'));
  expect(applied.files[0].text).toBe(edited);
  expect(applied.diagnostics.some(d => d.message.includes('package save-regression 0.1 cannot be found'))).toBe(true);
  expect(await page.evaluate(() => window.viewer.edits)).toEqual({unsaved:[],unapplied:[]});
  await page.keyboard.press('Control+q');
  await expect(area).toHaveValue(edited);
  await page.keyboard.press('Control+q');
  await expect(area).toHaveCount(0);
  const rebuilt = await opened(page, () => page.evaluate(() => window.viewer.command('reload-library')));
  expect(rebuilt.files[0].text).toBe(edited);
});

async function mockPackageCatalog(page, versions = ['0.1'], names = ['offer-test']) {
  const packages = names.flatMap(name => versions.map(version => {
    const text = `@RNL(type="header") { @VERSION("0.3"); @PACKAGE(name="${name}", version="${version}") }\n`;
    const relative = `mock/${name}-${version}.rnl`;
    return {name,version,path:relative,url:relative,kind:'extension',summary:{en:'Test package',zh:'测试包'},
      sha256:createHash('sha256').update(text).digest('hex'),text};
  }));
  for (const pkg of packages) await page.route(`**/public-packages/${pkg.url}`, route => route.fulfill({body:pkg.text}));
  const catalog = {schemaVersion:1,packages:packages.map(({text,...pkg})=>pkg)};
  await page.route('**/public-packages/catalog.json',route=>route.fulfill({json:catalog}));
  return catalog;
}
const packageTop = reference => `* package offers\n* @RNL(type="header") { @VERSION("0.3"); ${reference} }\nR1 a 0 1k\n.end\n`;
const packageFile = (text, name='offer.sp') => ({name,mimeType:'text/plain',buffer:Buffer.from(text)});

test('clipboard Save then Apply can subsequently return to the original text',async({page})=>{
  await start(page);
  const original='* clipboard source baseline\nR1 a 0 1k\n.end\n';
  const edited=original.replace('1k','2k');
  await page.evaluate(text=>{navigator.clipboard.readText=async()=>text;window.viewer.command('load-clipboard');},original);
  await opened(page,()=>page.getByRole('dialog').getByRole('button',{name:'Load',exact:true}).click());
  await page.keyboard.press('Control+q');
  const area=page.locator('#source .editor:not([hidden]) textarea');
  await area.fill(edited);
  await Promise.all([page.waitForEvent('download'),page.keyboard.press('Control+s')]);
  await expect(page.locator('#note')).toHaveText('Downloaded clipboard.sp.');
  const first=await opened(page,()=>page.keyboard.press('Control+q'));
  expect(first.files[0].text).toBe(edited);
  await page.keyboard.press('Control+q');
  await area.fill(original);
  const restored=await opened(page,()=>page.keyboard.press('Control+q'));
  expect(restored.files[0].text).toBe(original);
  expect(await page.evaluate(()=>window.viewer.edits.unapplied)).toEqual([]);
});

test('automatic package offers wait for an existing manual Library download',async({page})=>{
  const catalog=await mockPackageCatalog(page,['0.1'],['offer-test','offer-other']);
  let catalogRequest, downloadRequest;
  await page.route('**/public-packages/catalog.json',route=>{catalogRequest=route;});
  await page.route('**/public-packages/mock/offer-other-0.1.rnl',route=>{downloadRequest=route;});
  await start(page);
  await opened(page,()=>page.locator('#file-input').setInputFiles(packageFile(packageTop('@USE-PACKAGE(name="offer-test", version="0.1")'))));
  await page.evaluate(()=>window.viewer.command('include-library'));
  await catalogRequest.fulfill({json:catalog});
  const chooser=page.getByRole('dialog',{name:'Include Public Package'});
  await chooser.locator('input[type=search]').fill('offer-other');
  await chooser.locator('input[type=search]').press('Enter');
  await expect.poll(()=>!!downloadRequest).toBe(true);
  // Allow a full automatic-offer polling interval while the manual download stays blocked.
  await page.evaluate(()=>new Promise(resolve=>setTimeout(resolve,350)));
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await downloadRequest.fallback();
  const prompt=page.getByRole('dialog',{name:'Public Packages Found'});
  await expect(prompt).toBeVisible();
  const built=await opened(page,()=>prompt.getByRole('button',{name:'Add to Library',exact:true}).click());
  expect(built.diagnostics.filter(d=>d.severity==='error')).toEqual([]);
});

for (const auto of [false,true]) test(`public package offers are independent of Auto Include (${auto}) and batch duplicate references`, async ({page}) => {
  await page.addInitScript(auto => localStorage.setItem('auto-include',JSON.stringify(auto)),auto);
  await mockPackageCatalog(page,['0.1'],['offer-test','offer-other']);
  await start(page);
  const text = packageTop('@USE-PACKAGE(name="offer-test", version="0.1"); @USE-PACKAGE(name="offer-test", version="0.1"); @USE-PACKAGE(name="offer-other", version="0.1")');
  await opened(page,()=>page.locator('#file-input').setInputFiles(packageFile(text)));
  const prompt=page.getByRole('dialog',{name:'Public Packages Found'});
  await expect(prompt).toBeVisible();
  await expect(prompt.locator('li')).toHaveText(['offer-test 0.1','offer-other 0.1']);
  const built=await opened(page,()=>prompt.getByRole('button',{name:'Add to Library',exact:true}).click());
  expect(built.diagnostics.filter(d=>d.severity==='error')).toEqual([]);
  await page.locator('#tab-files').click();
  for (const name of ['offer-test','offer-other']) await expect(page.getByRole('button',{name:`Exclude ${name} 0.1`,exact:true})).toBeVisible();
});

test('public package offers choose the newest unpinned version and remember Cancel across Library rebuilds',async({page})=>{
  await mockPackageCatalog(page,['0.1','0.2']);
  await start(page);
  await opened(page,()=>page.locator('#file-input').setInputFiles(packageFile(packageTop('@USE-PACKAGE(name="offer-test")'))));
  const prompt=page.getByRole('dialog',{name:'Public Packages Found'});
  await expect(prompt.locator('li')).toHaveText(['offer-test 0.2']);
  await prompt.getByRole('button',{name:'Cancel',exact:true}).click();
  await opened(page,()=>page.evaluate(()=>window.viewer.command('reload-library')));
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.locator('.package-hint button')).toHaveText('Include');
});

test('unresolved symbols do not offer packages and delayed catalog results cannot affect a different file',async({page})=>{
  const catalog=await mockPackageCatalog(page);
  let pending;
  await page.route('**/public-packages/catalog.json',route=>{pending=route;});
  await start(page);
  await opened(page,()=>page.locator('#file-input').setInputFiles(packageFile(packageTop('@USE-PACKAGE(name="offer-test", version="0.1")'))));
  const plain='* no package reference\nR1 a 0 1k\n* @RNL { #R1 @SET-SYMBOL $MISSING_SYMBOL }\n.end\n';
  await opened(page,()=>page.locator('#file-input').setInputFiles(packageFile(plain,'other.sp')));
  await pending.fulfill({json:catalog});
  await page.evaluate(async()=>{await (await import('./shells/web.js')).default.publicPackages();});
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.locator('.package-hint')).toHaveCount(0);
  await page.locator('#tab-files').click();
  await expect(page.getByRole('button',{name:'Exclude offer-test 0.1',exact:true})).toHaveCount(0);
});

test('a stalled public catalog times out after ten seconds while viewing and editing continue',async({page})=>{
  const catalog=await mockPackageCatalog(page);
  let pending;
  await page.route('**/public-packages/catalog.json',route=>{pending=route;});
  await start(page);
  const text=packageTop('@USE-PACKAGE(name="offer-test", version="0.1")');
  await opened(page,()=>page.locator('#file-input').setInputFiles(packageFile(text)));
  await expect(page.locator('body')).not.toHaveClass(/busy/);
  await page.keyboard.press('Control+q');
  const area=page.locator('#source .editor:not([hidden]) textarea');
  await area.fill(text+'* typed while offline\n');
  await opened(page,()=>page.keyboard.press('Control+q'));
  expect(await page.evaluate(()=>window.viewer.doc.files[0].text)).toContain('* typed while offline');
  await expect(page.locator('#public-package-status')).toContainText('timed out after 10 seconds',{timeout:15000});
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await pending.fulfill({json:catalog}).catch(()=>{});
  await expect(page.getByRole('dialog')).toHaveCount(0);
});

test("Include File resolves a package with a different filename and Exclude removes it from search", async ({ page }) => {
  await start(page);
  await page.locator("#file-input").setInputFiles({ name: "top.sp", mimeType: "text/plain", buffer: Buffer.from(
    '* top\n* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="acme", version="1.0") }\nR1 a 0 1k\n.end\n') });
  await shown(page);
  await page.locator("#tab-files").click();
  expect(await page.locator("#pool-list .pool-row").count()).toBe(1);
  const d = await opened(page, () => page.locator("#include-input").setInputFiles({ name: "custom.rnl", mimeType: "text/plain", buffer: Buffer.from(
    '@RNL(type="header") { @VERSION("0.3"); @PACKAGE(name="acme", version="1.0") }\n') }));
  expect(d.diagnostics.some((d) => d.message.includes("acme 1.0 cannot be found"))).toBe(false);
  const excluded = await opened(page, () => page.getByRole("button", { name: "Exclude custom.rnl", exact: true }).click());
  expect(excluded.diagnostics.some((d) => d.message.includes("acme 1.0 cannot be found"))).toBe(true);
});

test("the web library adds SKY130 without duplicating built-ins, which open read-only", async ({ page }) => {
  await start(page);
  await page.locator("#tab-files").click();
  await page.locator('[data-group="builtin"] .lib-toggle').click();
  await expect(page.getByRole("button", { name: "Open 0.2/rnl-std.rnl", exact: true })).toBeVisible();
  await expect(page.locator('[data-group="builtin"]').getByRole("button", { name: /Exclude/ })).toHaveCount(0);
  await page.locator("#include-library").click();
  const dialog = page.locator("dialog.dialog");
  await expect(dialog.locator(".dialog-option")).toContainText(["pdk-sky130"]);
  await expect(dialog.locator(".dialog-option")).not.toContainText(["rnl-std-basic"]);
  await dialog.locator("input[type=search]").fill("sky130");
  await dialog.locator("input[type=search]").press("Enter");
  await expect(page.getByRole("button", { name: "Exclude pdk-sky130 0.1", exact: true })).toBeVisible();
  await page.locator("#file-input").setInputFiles({ name: "sky.sp", mimeType: "text/plain", buffer: Buffer.from(
    '* sky\n* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="pdk-sky130", version="0.1") }\nXM1 d g 0 0 sky130_fd_pr__nfet_01v8\n.end\n') });
  const doc = await shown(page);
  expect(doc.diagnostics.filter((d) => d.severity === "error")).toEqual([]);
  await page.locator("#tab-files").click();
  const builtin = await opened(page, () => page.getByRole("button", { name: "Open 0.3/rnl-std.rnl", exact: true }).click());
  expect(builtin.path).toContain("<bundled>");
  await page.evaluate(() => window.viewer.command("toggle-edit"));
  expect(await page.evaluate(() => window.viewer.checked("toggle-edit"))).toBe(false);
  await expect(page.locator("#source")).toContainText("@PACKAGE");
});

test("a missing published Canvas package offers Include and rebuilds; other versions offer the File menu hint", async ({ page }, info) => {
  test.skip(!hasPublicPackages, "Run python scripts/packages.py download");
  await start(page);
  const source = fs.readFileSync(path.join(root, "tests/viewer/inputs/missing-canvas-package.sp"));
  const before = await opened(page, () => page.locator("#file-input").setInputFiles({ name: "canvas.sp", mimeType: "text/plain", buffer: source }));
  expect(before.diagnostics.some(d => d.message.includes("package analog-canvas-extensions 0.1 cannot be found"))).toBe(true);
  await expect(page.locator(".package-hint")).toContainText("This package is published.");
  const prompt = page.getByRole('dialog', { name: 'Public Packages Found' });
  await expect(prompt).toBeVisible();
  await prompt.getByRole('button', { name: 'Cancel', exact: true }).click();
  if (process.env.RNL_PACKAGE_SCREENSHOTS && info.project.name === "chromium") {
    await page.screenshot({ path: path.join(process.env.RNL_PACKAGE_SCREENSHOTS, "web-missing-canvas-include.png") });
  }
  const after = await opened(page, () => page.locator(".package-hint").getByRole("button", { name: "Include", exact: true }).click());
  expect(after.diagnostics.filter(d => d.severity === "error")).toEqual([]);
  await expect(page.locator(".package-hint")).toHaveCount(0);
  await page.locator("#tab-files").click();
  await expect(page.getByRole("button", { name: "Exclude analog-canvas-extensions 0.1", exact: true })).toBeVisible();
  const builtin = page.locator('[data-group="builtin"]');
  await builtin.locator(".lib-toggle").click();
  await expect(builtin.locator(".pool-row")).toHaveCount(25);
  expect(await builtin.locator(".pool-open").allTextContents()).toEqual(expect.arrayContaining(["0.3/rnl-std.rnl", "0.2/rnl-std.rnl"]));
  for (const name of await builtin.locator(".pool-open").allTextContents()) expect(name).toMatch(/^0\.[23]\/rnl-(std|zh)/);
  if (process.env.RNL_PACKAGE_SCREENSHOTS && info.project.name === "chromium") {
    await page.screenshot({ path: path.join(process.env.RNL_PACKAGE_SCREENSHOTS, "web-library-standard-builtins.png") });
  }
  await opened(page, () => page.locator("#file-input").setInputFiles({ name: "canvas-newer.sp", mimeType: "text/plain", buffer: Buffer.from(source.toString().replace('version="0.1"', 'version="99.0"')) }));
  await expect(page.locator(".package-hint")).toContainText("Look for it in File › Include Public Package…");
  await expect(page.locator(".package-hint button")).toHaveCount(0);
});


test("independent public EDA packages draw their gallery and custom terminal markers", async ({ page }) => {
  test.skip(!hasPublicPackages, "Run python scripts/packages.py download");
  await start(page);
  const { packages: index } = await (await page.request.get(`${process.env.RNL_WEB_URL}/public-packages/catalog.json`)).json();
  for (const name of ["eda-style", "eda-symbol"]) {
    expect(index.filter((p) => p.name === name)).toMatchObject([
      { version: "0.1", path: `${name}/${name}.rnl` },
    ]);
  }
  expect(index.every((p) => p.path.endsWith(".rnl"))).toBe(true);
  const bundled = await (await page.request.get(`${process.env.RNL_WEB_URL}/libraries/bundled.json`)).json();
  expect(Object.keys(bundled).some((p) => /eda-(style|symbol)/.test(p))).toBe(false);
  for (const name of ["eda-style", "eda-symbol"]) {
    await page.locator("#tab-files").click();
    await page.locator("#include-library").click();
    const dialog = page.locator("dialog.dialog");
    await dialog.locator("input[type=search]").fill(name);
    await dialog.getByRole("option", { name: new RegExp(`^${name} 0\\.1 —`) }).dblclick();
    await expect(page.getByRole("button", { name: `Exclude ${name} 0.1`, exact: true })).toBeVisible();
    if (name === "eda-style") {
      await expect(page.getByRole("button", { name: "Exclude eda-symbol 0.1", exact: true })).toHaveCount(0);
    }
  }
  const doc = await opened(page, () => page.locator("#file-input").setInputFiles(path.join(publicPackageDir("eda-style"), "eda-style-gallery.scs")));
  expect(doc.diagnostics.filter((d) => ["error", "warning"].includes(d.severity))).toEqual([]);
  expect(doc.levels).toHaveLength(7);
  const svg = doc.levels[0].svg;
  expect(svg).toContain("#00cc66");
  expect(svg).toContain("#ff0000");
  expect(svg).toContain('font-family="Lucida Console, Monaco, DejaVu Sans Mono, Consolas, monospace"');
  const custom = await opened(page, () => page.locator("#file-input").setInputFiles(path.join(publicPackageDir("eda-style"), "examples/custom-symbol.sp")));
  expect(custom.diagnostics.filter((d) => ["error", "warning"].includes(d.severity))).toEqual([]);
  expect(await page.evaluate((svg) => new DOMParser().parseFromString(svg, "image/svg+xml").querySelectorAll('rect[fill="#ff0000"]').length, custom.levels[0].svg)).toBe(6);
});

test("relative files win over pooled files with the same name, including deferred URL collection", async ({ page }) => {
  await start(page);
  await page.locator("#include-input").setInputFiles({ name: "part.inc", mimeType: "text/plain", buffer: Buffer.from("Rpool a 0 1k\n") });
  await page.locator("#tab-files").click();
  await expect(page.getByRole("button", { name: "Exclude part.inc", exact: true })).toBeVisible();
  const local = await opened(page, () => page.locator("#file-input").setInputFiles([
    { name: "top.sp", mimeType: "text/plain", buffer: Buffer.from('* top\n.include "part.inc"\n.end\n') },
    { name: "part.inc", mimeType: "text/plain", buffer: Buffer.from("Rrelative a 0 2k\n") },
  ]));
  expect(local.files[1].text).toContain("Rrelative");
  await page.route("**/priority/top.sp", (route) => route.fulfill({ body: '* top\n.include "part.inc"\n.end\n', contentType: "text/plain" }));
  await page.route("**/priority/part.inc", (route) => route.fulfill({ body: "Rremote a 0 3k\n", contentType: "text/plain" }));
  await page.evaluate(() => window.viewer.command("open-url"));
  const dialog = page.locator("dialog.dialog");
  await dialog.locator("input").fill(`${process.env.RNL_WEB_URL}/priority/top.sp`);
  const remote = await opened(page, () => dialog.locator("input").press("Enter"));
  expect(remote.files[1].text).toContain("Rremote");
  expect(remote.diagnostics.some((d) => d.message.includes("resolved through"))).toBe(false);
});

/** Load actual clipboard content through the shared preview and the WebAssembly shell. */
async function pasteText(page, text) {
  await page.evaluate((text) => {
    navigator.clipboard.readText = async () => text;
    window.viewer.command("load-clipboard");
  }, text);
  await expect(page.locator("dialog.dialog .dialog-title")).toHaveText("Load from Clipboard");
  return opened(page, () => page.locator("dialog.dialog").getByRole("button", { name: "Load", exact: true }).click());
}

test("opened web files join Files once; Exclude keeps the root open and removes package lookup", async ({ page }) => {
  await start(page);
  const pkg = { name: "custom.rnl", mimeType: "text/plain", buffer: Buffer.from(
    '@RNL(type="header") { @VERSION("0.3"); @PACKAGE(name="acme-open", version="1.0") }\n') };
  const top = { name: "top.sp", mimeType: "text/plain", buffer: Buffer.from(
    '* top\n* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="acme-open", version="1.0") }\nR1 a 0 1k\n.end\n') };
  await opened(page, () => page.locator("#file-input").setInputFiles(pkg));
  await opened(page, () => page.locator("#file-input").setInputFiles(pkg));
  await expect(page.locator("#pool-list .pool-row")).toHaveCount(1);
  const loaded = await opened(page, () => page.locator("#file-input").setInputFiles(top));
  expect(loaded.diagnostics.some((d) => d.message.includes("acme-open 1.0 cannot be found"))).toBe(false);
  await page.locator("#tab-files").click();
  await opened(page, () => page.getByRole("button", { name: "Open custom.rnl", exact: true }).click());
  await page.locator("#tab-files").click();
  const excluded = await opened(page, () => page.getByRole("button", { name: "Exclude custom.rnl", exact: true }).click());
  expect(excluded.name).toBe("custom.rnl");
  await expect(page.getByRole("button", { name: "Open custom.rnl", exact: true })).toHaveCount(0);
  const missing = await opened(page, () => page.getByRole("button", { name: "Open top.sp", exact: true }).click());
  expect(missing.diagnostics.some((d) => d.message.includes("acme-open 1.0 cannot be found"))).toBe(true);
});

test("temporary package is absent from include and USE-PACKAGE lookup; applied text survives switching", async ({ page }) => {
  await start(page);
  const pkg = '@RNL(type="header") { @VERSION("0.3"); @PACKAGE(name="acme-temp", version="1.0") }\n';
  await pasteText(page, pkg);
  await expect(page.locator(".pool-temporary.current")).toHaveAttribute("aria-current", "true");
  await expect(page.locator("#pool-list .pool-row")).toHaveCount(0);
  const text = '* top\n* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="acme-temp", version="1.0") }\n.include "clipboard.rnl"\n.end\n';
  const top = { name: "other.sp", mimeType: "text/plain", buffer: Buffer.from(text) };
  const missing = await opened(page, () => page.locator("#file-input").setInputFiles(top));
  expect(missing.missing).toContain("/clipboard.rnl");
  expect(missing.diagnostics.some((d) => d.message.includes("acme-temp 1.0 cannot be found"))).toBe(true);
  await page.locator("#tab-files").click();
  await opened(page, () => page.locator(".pool-temporary .pool-open").click());
  await page.locator("#tab-netlist").click();
  await page.evaluate(() => window.viewer.command("toggle-edit"));
  const edited = pkg.replace("1.0", "2.0");
  await page.locator("#source textarea").fill(edited);
  await opened(page, () => page.locator("#apply-edits").click());
  await page.locator("#tab-files").click();
  await page.getByRole("button", { name: "Open other.sp", exact: true }).click();
  await opened(page, () => page.locator("dialog.dialog").getByRole("button", { name: "Discard Changes", exact: true }).click());
  await page.locator("#tab-files").click();
  const back = await opened(page, () => page.locator(".pool-temporary .pool-open").click());
  expect(back.files[0].text).toBe(edited);
  await pasteText(page, "* next paste\nR2 b 0 2k\n.end\n");
  await expect(page.locator(".pool-temporary")).toHaveCount(1);
  expect((await page.evaluate(() => window.viewer.doc)).files[0].text).toContain("R2 b 0 2k");
});

// ---- gallery designs by manifest (?manifest=, core/manifest.js) ----

const GALLERY_BASE = "https://raw.githubusercontent.com/someone/design/0123456789abcdef0123456789abcdef01234567/";
const GALLERY_MANIFEST = "https://gallery.test/designs/demo.json";
const GALLERY_TREE = "https://github.com/someone/design/tree/0123456789abcdef0123456789abcdef01234567/examples/demo";
const GALLERY_REMOTE = "https://rich-netlist.github.io/rnl-packages/demo-remote/2.0/demo-remote.rnl";
const galleryDir = path.join(root, "tests/viewer/inputs/gallery");
const galleryFiles = ["top.sp", "cells/amp.sp", "lib/models.inc", "pkg/demo-local.rnl"];
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

/** The fixture design's manifest, with `change` over it. */
function galleryManifest(change = {}) {
  const remote = fs.readFileSync(path.join(root, "tests/viewer/inputs/gallery-remote.rnl"));
  return {
    format: 1, id: "demo", title: { en: "Gallery demo", zh: "图库示例" }, author: "someone",
    source: { repo: "someone/design", commit: "0123456789abcdef0123456789abcdef01234567", url: GALLERY_TREE },
    license: "MIT", base: GALLERY_BASE, entry: "top.sp", level: "amp",
    files: [
      ...galleryFiles.map((file) => {
        const bytes = fs.readFileSync(path.join(galleryDir, file));
        return { path: file, sha256: sha256(bytes), size: bytes.length };
      }),
      { path: ".packages/demo-remote/2.0/demo-remote.rnl", sha256: sha256(remote), size: remote.length, url: GALLERY_REMOTE },
    ],
    // a bare include and a package by name look in the pool, in its order
    pool: ["lib/models.inc", "pkg/demo-local.rnl", ".packages/demo-remote/2.0/demo-remote.rnl"],
    ...change,
  };
}

/** Serve the manifest, its files from raw GitHub and the remote package; count the manifest's fetches. */
async function galleryRoutes(page, manifest, served = {}) {
  const counts = { manifest: 0 };
  await page.route(GALLERY_MANIFEST, (route) => {
    counts.manifest += 1;
    return route.fulfill({ status: 200, contentType: "application/json", headers: { "Access-Control-Allow-Origin": "*" }, body: JSON.stringify(manifest) });
  });
  await page.route(`${GALLERY_BASE}**`, (route) => {
    const file = new URL(route.request().url()).pathname.split("/").slice(4).join("/");
    const body = served[file] ?? (galleryFiles.includes(file) ? fs.readFileSync(path.join(galleryDir, file)) : null);
    return body === null ? route.fulfill({ status: 404, body: "missing" })
      : route.fulfill({ status: 200, contentType: "text/plain", headers: { "Access-Control-Allow-Origin": "*" }, body });
  });
  await page.route(GALLERY_REMOTE, (route) => route.fulfill({ status: 200, contentType: "text/plain",
    headers: { "Access-Control-Allow-Origin": "*" }, body: fs.readFileSync(path.join(root, "tests/viewer/inputs/gallery-remote.rnl")) }));
  return counts;
}

test("?manifest= opens a gallery design: its files, search path and packages, its level, the source bar and a read-only Library group", async ({ page }) => {
  // "(top)": the top level of the entry; a level in another file opens that file, as the next tests show
  const counts = await galleryRoutes(page, galleryManifest({ level: "(top)" }));
  await page.goto(`${process.env.RNL_WEB_URL}/index.html?manifest=${encodeURIComponent(GALLERY_MANIFEST)}`);
  const doc = await shown(page);
  // the relative include, the bare one on the search path and both packages are found
  expect(doc.diagnostics.filter((d) => d.severity === "error")).toEqual([]);
  expect(doc.files.map((f) => f.name)).toEqual(expect.arrayContaining(["top.sp", "cells/amp.sp", "lib/models.inc"]));
  expect(await page.evaluate(() => window.viewer.level)).toBe(0);
  const bar = page.locator("#gallery-bar");
  await expect(bar).toBeVisible();
  await expect(bar.locator(".gallery-title")).toHaveText("Gallery demo");
  await expect(bar).toContainText("someone");
  await expect(bar.locator(".gallery-source")).toHaveText("Source: someone/design@0123456");
  await expect(bar.locator(".gallery-source")).toHaveAttribute("href", GALLERY_TREE);
  await expect(bar.locator(".gallery-back")).toHaveAttribute("href", "https://rich-netlist.github.io/rnl/en/gallery.html");
  await page.locator("#tab-files").click();
  const group = page.locator('.lib-group[data-group="gallery"]');
  await expect(group.locator(".lib-title")).toHaveText("From the gallery · demo · read only");
  await expect(group.locator(".pool-open")).toHaveText([".packages/demo-remote/2.0/demo-remote.rnl", "cells/amp.sp", "lib/models.inc", "pkg/demo-local.rnl", "top.sp"]);
  await expect(group.getByRole("button", { name: /Exclude/ })).toHaveCount(0);
  // read only: the text cannot be edited
  await page.evaluate(() => window.viewer.command("toggle-edit"));
  await expect(page.locator("#source textarea")).toHaveCount(0);
  // Chinese: the title and the way back follow the language
  await page.evaluate(() => window.viewer.command("language-zh"));
  await expect(bar.locator(".gallery-title")).toHaveText("图库示例");
  await expect(bar.locator(".gallery-back")).toHaveAttribute("href", "https://rich-netlist.github.io/rnl/zh/gallery.html");
  await page.evaluate(() => window.viewer.command("language-en"));
  if (process.env.RNL_REVIEW_DIR && test.info().project.name === "chromium") {
    await page.screenshot({ path: path.join(process.env.RNL_REVIEW_DIR, "gallery-manifest.png") });
  }
  // Reload reads the manifest again
  const before = counts.manifest;
  await opened(page, () => page.keyboard.press("F5"));
  expect(counts.manifest).toBe(before + 1);
});

test("a gallery file whose SHA-256 does not match is named and nothing opens; a missing level is told", async ({ page }) => {
  await galleryRoutes(page, galleryManifest(), { "cells/amp.sp": "* changed\n" });
  await page.goto(`${process.env.RNL_WEB_URL}/index.html?manifest=${encodeURIComponent(GALLERY_MANIFEST)}`);
  await expect(page.locator("#diags .diag.error .msg")).toContainText("cells/amp.sp");
  expect(await page.evaluate(() => window.viewer.doc)).toBeNull();
  await expect(page.locator("#gallery-bar")).toBeHidden();

  await page.unrouteAll();
  await galleryRoutes(page, galleryManifest({ level: "nothing" }));
  await page.goto(`${process.env.RNL_WEB_URL}/index.html?manifest=${encodeURIComponent(GALLERY_MANIFEST)}`);
  await shown(page);
  await expect(page.locator("#notice")).toContainText("no level nothing");
  // ?level= wins over the manifest's
  await page.goto(`${process.env.RNL_WEB_URL}/index.html?manifest=${encodeURIComponent(GALLERY_MANIFEST)}&level=amp`);
  await shown(page);
  await expect.poll(() => page.evaluate(() => window.viewer.doc.levels[window.viewer.level].title)).toBe("amp");
});

test("an embedded canvas opens a gallery design by ?manifest=", async ({ page }) => {
  await galleryRoutes(page, galleryManifest());
  await page.goto(`${process.env.RNL_WEB_URL}/index.html?embed=1&manifest=${encodeURIComponent(GALLERY_MANIFEST)}`);
  await expect(page.locator("#canvas > svg")).toHaveCount(1);
  await expect.poll(() => page.evaluate(() => window.viewer?.doc?.levels[window.viewer.level]?.title)).toBe("amp");
});
