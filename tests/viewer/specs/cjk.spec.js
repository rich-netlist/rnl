import { test, expect } from '../viewer.js';
import { cjkEvidence, assertCjk } from '../../../apps/desktop/tests/cjk.mjs';

test('Chinese UI and source use real glyphs; mixed text keeps native selection and editor positions', async ({page,viewer},info) => {
  await viewer.open('04-hierarchy');
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
