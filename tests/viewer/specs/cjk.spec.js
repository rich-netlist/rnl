import { test, expect } from '../viewer.js';
import { cjkEvidence, assertCjk } from '../../../apps/desktop/tests/cjk.mjs';

test('Chinese UI and source use real glyphs; mixed text keeps native selection and editor positions', async ({page,viewer},info) => {
  await viewer.open('15-chinese');
  await page.evaluate(() => window.viewer.command('language-zh'));
  const evidence = await page.evaluate(cjkEvidence);
  assertCjk(evidence, process.env.RNL_NO_SYSTEM_CJK === '1');
  await info.attach('font-requests.json',{body:JSON.stringify(await page.evaluate(()=>performance.getEntriesByType('resource')
    .filter(r=>r.name.includes('/fonts/')).map(r=>({name:r.name.split('/fonts/')[1],bytes:r.decodedBodySize}))),null,2),contentType:'application/json'});
  await page.evaluate(()=>window.viewer.command('grid-settings'));
  const dialogEvidence = await page.evaluate(cjkEvidence);
  assertCjk(dialogEvidence, process.env.RNL_NO_SYSTEM_CJK === '1');
  expect(dialogEvidence.some(e=>e.selector==='dialog .dialog-title')).toBe(true);
  await page.locator('dialog').getByRole('button',{name:'取消',exact:true}).click();
  // A missing font must be detected, even though the DOM still contains Chinese text.
  if (process.env.RNL_NO_SYSTEM_CJK === '1') {
    await page.addStyleTag({content:'#source { font-family: monospace !important; }'});
    const broken = await page.evaluate(cjkEvidence);
    expect(() => assertCjk(broken,true)).toThrow();
    await page.locator('head style').last().evaluate(el=>el.remove());
  }
  const svg = await page.locator('#canvas > svg').evaluate(el=>el.outerHTML);
  const source = page.locator('#source');
  // Browser Range geometry, rather than a count of equal-width cells, owns selection.
  const selected = await source.evaluate(el => {
    const node = [...el.querySelectorAll('.code')].find(n=>/[\u4e00-\u9fff]/.test(n.textContent));
    const walker = document.createTreeWalker(node,NodeFilter.SHOW_TEXT);
    let text;
    while ((text=walker.nextNode())) {
      const match = /[\u4e00-\u9fff]+/.exec(text.data);
      if (!match) continue;
      const range=document.createRange(); range.setStart(text,match.index); range.setEnd(text,match.index+match[0].length);
      getSelection().removeAllRanges(); getSelection().addRange(range);
      return {text:getSelection().toString(),width:range.getBoundingClientRect().width};
    }
  });
  expect(selected.width).toBeGreaterThan(1);
  await page.evaluate(()=>window.viewer.command('find-text'));
  await page.locator('#netlist-query').fill(selected.text);
  await expect(page.locator('#netlist-matches')).toContainText('1 /');
  await expect.poll(()=>page.evaluate(()=>getSelection().toString())).toBe(selected.text);
  await page.locator('#netlist-find-close').click();
  await page.evaluate(()=>window.viewer.command('toggle-edit'));
  const editor = source.locator('textarea:visible');
  const mixed = '* abc中文\t电路XYZ\n* ' + '中文abc'.repeat(100) + '\n.end';
  await editor.fill(mixed);
  await editor.evaluate(el=>{el.focus();el.setSelectionRange(7,7);});
  await page.keyboard.press('Shift+ArrowLeft');
  expect(await editor.evaluate(el=>el.value.slice(el.selectionStart,el.selectionEnd))).toBe('文');
  expect(await editor.getAttribute('wrap')).toBe('off');
  expect(await editor.evaluate(el=>el.scrollWidth>el.clientWidth)).toBe(true);
  await page.evaluate(()=>window.viewer.command('language-en'));
  expect(await page.locator('#canvas > svg').evaluate(el=>el.outerHTML)).toBe(svg);
});

test('virtual line width remains conservative with CJK fallback and a recycled wide row', async ({page,viewer})=>{
  await viewer.open('15-chinese');
  const widths=await page.evaluate(async()=>{
    const {lineModel}=await import('./text-lines.js');
    const {LineWindow}=await import('./line-window.js');
    const source=document.getElementById('source');
    const text=['* abc中文\t电路 XYZ'.repeat(100),...Array(1200).fill('* 中文')].join('\n');
    const view=new LineWindow(source,()=>lineModel({text,records:[],failed:[]},0,[],'spice'),()=>{});
    source.replaceChildren(view.el); view.el.hidden=false; view.refresh();
    const row=view.el.querySelector('.ln'), actual=row.scrollWidth, extent=view.el.scrollWidth;
    source.scrollTop=20000; view.refresh();
    return {actual,extent,recycled:view.el.scrollWidth,estimated:view.model.columns};
  });
  expect(widths.extent).toBeGreaterThanOrEqual(widths.actual);
  expect(widths.recycled).toBeGreaterThanOrEqual(widths.actual);
});

test('symbol details, clipboard previews and web menus share the CJK fallback', async ({page,viewer},info)=>{
  await viewer.open('symbol-inspector');
  await page.evaluate(async()=>{
    window.viewer.command('language-zh');
    const level=window.viewer.doc.levels.findIndex(l=>l.title==='leaf');
    (await import('./subcircuits.js')).enterRoot(level);
  });
  await page.keyboard.press('4');
  await expect(page.locator('.symbol-detail h2')).toBeVisible();
  const symbol=await page.evaluate(cjkEvidence);
  assertCjk(symbol,process.env.RNL_NO_SYSTEM_CJK==='1');
  expect(symbol.some(e=>e.selector==='.symbol-definition')).toBe(true);
  await viewer.menuTitle('视图').click();
  const menu=await page.evaluate(cjkEvidence);
  assertCjk(menu,process.env.RNL_NO_SYSTEM_CJK==='1');
  expect(menu.some(e=>e.selector==='.menu-item')).toBe(true);
  await page.keyboard.press('Escape');
  await page.evaluate(()=>{
    navigator.clipboard.readText=async()=>'* 中文网表电路\nR1 输入 输出 1k\n.end';
    window.viewer.command('load-clipboard');
  });
  await expect(page.locator('.clip-preview')).toBeVisible();
  const clipboard=await page.evaluate(cjkEvidence);
  assertCjk(clipboard,process.env.RNL_NO_SYSTEM_CJK==='1');
  expect(clipboard.some(e=>e.selector==='.clip-preview')).toBe(true);
  await info.attach('cjk-surfaces.json',{body:JSON.stringify({symbol,menu,clipboard},null,2),contentType:'application/json'});
});

test('record web font transfer before and after the fallback on a Chinese sample', async ({browser},info)=>{
  test.skip(process.env.RNL_NO_SYSTEM_CJK!=='1','Transfer differences need the Latin-only Linux font profile.');
  const result={};
  for(const enabled of [false,true]) {
    const context=await browser.newContext();
    const page=await context.newPage();
    await page.route('**/style.css',async route=>{
      const response=await route.fetch();
      const body=await response.text();
      await route.fulfill({response,body:enabled ? body : body.replaceAll(', var(--ui-cjk)','')});
    });
    await page.goto(`${process.env.RNL_VIEWER_URL}/index.html?shell=fake&open=15-chinese`);
    await page.waitForFunction(()=>window.viewer?.doc && !document.body.classList.contains('busy') && !document.body.classList.contains('starting'));
    await page.evaluate(()=>window.viewer.command('language-zh'));
    await page.waitForFunction(()=>!!document.getElementById('viewer-fonts').sheet);
    await page.evaluate(async()=>{
      await new Promise(resolve=>requestAnimationFrame(resolve));
      await document.fonts.ready;
      await new Promise(resolve=>requestAnimationFrame(resolve));
    });
    result[enabled?'after':'before']=await page.evaluate(()=>{
      const files=new Map(performance.getEntriesByType('resource').filter(r=>r.name.includes('/fonts/') && r.name.endsWith('.woff2'))
        .map(r=>[r.name.split('/fonts/')[1],r.decodedBodySize]));
      return {bytes:[...files.values()].reduce((a,b)=>a+b,0),files:Object.fromEntries(files)};
    });
    await context.close();
  }
  result.delta=result.after.bytes-result.before.bytes;
  expect(result.before.bytes).toBeGreaterThan(0); // the unchanged Chinese schematic already uses Noto
  expect(result.delta).toBeGreaterThan(0); // UI/source add slices; no new font payload is shipped
  await info.attach('web-font-transfer.json',{body:JSON.stringify(result,null,2),contentType:'application/json'});
});

for (const [command, code] of [['zh','zh-CN'],['zh-hant','zh-Hant'],['ja','ja']]) {
  test(`the interface in ${code} takes its characters from one small font, not from the slices`, async ({page,viewer}) => {
    const fonts = await page.request.get(`${process.env.RNL_VIEWER_URL}/fonts/fonts.css`);
    test.skip(!fonts.ok() || !(await fonts.text()).includes(`RNL UI ${code}`),'The web fonts are not written; run python scripts/fonts.py web.');
    await viewer.open('04-hierarchy');
    await page.evaluate(command=>window.viewer.command(`language-${command}`),command);
    const catalog = await (await page.request.get(`${process.env.RNL_VIEWER_URL}/locales/${code}.json`)).json();
    // every character of the catalog but ASCII, in the font the page's texts name
    const text = [...new Set(Object.values(catalog).join('').replace(/[\x00-\x7e]/g,''))].join('');
    expect(text.length).toBeGreaterThan(100);
    await page.evaluate(async text=>{
      const probe = document.createElement('span');
      probe.id = 'ui-cjk-probe';
      probe.style.cssText = 'font-family: var(--ui-cjk); position: fixed; top: 0; left: 0';
      probe.textContent = text;
      document.body.append(probe);
      await new Promise(resolve=>requestAnimationFrame(resolve));
      await document.fonts.ready;
    },text);
    const files = await page.evaluate(()=>performance.getEntriesByType('resource').map(r=>r.name.split('/fonts/')[1]).filter(Boolean));
    expect(files).toContain(`ui-${code}-400.woff2`);
    // the face has every character: none falls through to the slices
    expect(files.filter(f=>f.includes('-cjk-'))).toEqual([]);
  });
}
