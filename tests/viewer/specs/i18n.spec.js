import { test, expect } from '../viewer.js';

const language = (page, lang) => page.evaluate(lang => window.viewer.command(`language-${lang}`), lang);

for (const platform of ['web', 'desktop']) {
  test(`${platform}: language switches immediately, preserves the document and is remembered`, async ({page,viewer}) => {
    await viewer.open('symbol-inspector', {platform});
    const builds = await viewer.builds();
    await page.evaluate(() => { window.keptDoc=window.viewer.doc; window.keptSvg=document.querySelector('#canvas > svg'); });
    const code = await page.locator('#source .ln .code').allTextContents();
    await language(page,'zh');
    await expect(page.locator('html')).toHaveAttribute('lang','zh-CN');
    await expect(page.getByRole('tab',{name:'网表',exact:true})).toBeVisible();
    await expect(page.locator('#tab-files')).toHaveAccessibleName('资源库');
    await expect(page.locator('#tab-files .tab-key')).toHaveText('1');
    expect(await page.locator('#source .ln .code').allTextContents()).toEqual(code);
    expect(await page.evaluate(() => window.keptDoc===window.viewer.doc && window.keptSvg===document.querySelector('#canvas > svg'))).toBe(true);
    expect(await viewer.builds()).toBe(builds);
    await page.keyboard.press('1');
    await expect(page.locator('#include-file')).toHaveText('加入文件…');
    await expect(page.locator('.lib-title').first()).toHaveText('文件');
    await language(page,'en');
    await expect(page.locator('.lib-title').first()).toHaveText('Files');
    await language(page,'zh');
    await viewer.open('symbol-inspector', {platform});
    await expect(page.locator('#tab-netlist')).toHaveAccessibleName('网表');
    await language(page,'en');
    await expect(page.locator('#tab-netlist')).toHaveAccessibleName('Netlist');
  });
}

test('the web Language submenu changes language and keeps IDs, checks and shortcuts', async ({page,viewer}) => {
  await viewer.open('04-hierarchy');
  await viewer.menuTitle('View').click();
  await page.locator('.menu-item.opens',{hasText:'Language'}).hover();
  await viewer.menuItem('language-zh').click();
  await expect(page.locator('html')).toHaveAttribute('lang','zh-CN');
  await viewer.menuTitle('视图').click();
  await page.locator('.menu-item.opens',{hasText:'语言'}).hover();
  await expect(viewer.menuItem('language-zh')).toHaveAttribute('aria-checked','true');
  await expect(viewer.menuItem('language-en')).toHaveAttribute('aria-checked','false');
  await viewer.menuItem('language-en').click();
  await page.keyboard.press('Control+b');
  await page.keyboard.press('3');
  await expect(page.locator('#pane-hierarchy')).toBeVisible();
});

test('language changes keep unsaved editor content, selection and the translated dialog actions', async ({page,viewer}) => {
  await viewer.open('04-hierarchy', {platform:'desktop'});
  await page.keyboard.press('Control+q');
  const area=page.locator('#source textarea:visible');
  await area.fill('* Library Save Cancel <b>plain text</b>\n.end');
  await area.evaluate(el=>el.setSelectionRange(3,10));
  await language(page,'zh');
  await expect(area).toHaveValue('* Library Save Cancel <b>plain text</b>\n.end');
  expect(await area.evaluate(el=>[el.selectionStart,el.selectionEnd])).toEqual([3,10]);
  await expect(page.locator('#edit-state')).toHaveText('修改尚未应用');
  await page.evaluate(()=>window.viewer.command('discard-edits'));
  const dialog=page.locator('dialog.dialog');
  await expect(dialog).toContainText('放弃尚未应用的修改');
  await dialog.getByRole('button',{name:'取消',exact:true}).click();
  await expect(area).toBeVisible();
  await page.evaluate(()=>window.viewer.command('discard-edits'));
  await dialog.getByRole('button',{name:'放弃修改',exact:true}).click();
  await expect(area).toHaveCount(0);
  expect(await viewer.calls('save')).toEqual([]);
});

test('parser and solver messages stay verbatim while UI headings and counters change', async ({page,viewer}) => {
  await viewer.open('12-diagnostics', {platform:'desktop'});
  const messages = await page.locator('#diags .diag:not(.package-hint) .msg').allTextContents();
  expect(messages.length).toBeGreaterThan(0);
  await language(page,'zh');
  expect(await page.locator('#diags .diag:not(.package-hint) .msg').allTextContents()).toEqual(messages);
  await expect(page.locator('#counts')).not.toContainText('error');
  await viewer.open('T4-miller-conflicts',{platform:'desktop'});
  await page.keyboard.press('F9');
  await page.waitForFunction(()=>document.body.classList.contains('preview')&&!document.body.classList.contains('solving'));
  const report = await page.locator('.solve-line .msg, .solve-line .more, .solve-summary').allTextContents();
  const heading=page.locator('.solve-group summary').first();
  await expect(heading).toContainText('文件');
  await language(page,'en');
  await expect(heading).toContainText('Files');
  expect(await page.locator('.solve-line .msg, .solve-line .more, .solve-summary').allTextContents()).toEqual(report);
});

test('translated dialogs preserve child controls and validation; filenames are never translated', async ({page,viewer}) => {
  await viewer.open('04-hierarchy',{platform:'desktop'});
  await language(page,'zh');
  await page.evaluate(()=>window.viewer.command('grid-settings'));
  const dialog=page.locator('dialog.dialog');
  await expect(dialog).toContainText('网格设置');
  const inputs=await dialog.locator('input').count();
  await page.evaluate(async()=>{ const {setLanguage}=await import('./i18n.js'); await setLanguage('en'); });
  await expect(dialog).toContainText('Grid Settings');
  expect(await dialog.locator('input').count()).toBe(inputs);
  await dialog.getByRole('button',{name:'Cancel',exact:true}).click();
  const result=await page.evaluate(async()=>{
    const {m,setText,append,setLanguage}=await import('./i18n.js');
    const container=document.createElement('div');
    document.body.append(container);
    setText(container,m('Save'));
    const raw=document.createElement('span'); setText(raw,'Save'); append(container,raw,m`Open ${'<b>Library</b>'}`);
    await setLanguage('zh-CN');
    const value={text:container.textContent,html:container.querySelector('b'),raw:raw.textContent};
    container.remove();return value;
  });
  expect(result).toEqual({text:'保存Save打开 <b>Library</b>',html:null,raw:'Save'});
});

test('Chinese system language is the initial default, explicit English wins on restart', async ({page,viewer}) => {
  await page.addInitScript(()=>Object.defineProperty(navigator,'languages',{get:()=>['zh-CN','en-US']}));
  await viewer.open(null);
  await expect(page.locator('#tab-files')).toHaveAccessibleName('资源库');
  await language(page,'en');
  await viewer.open(null);
  await expect(page.locator('#tab-files')).toHaveAccessibleName('Library');
});

test('an open clipboard dialog updates its labels without changing text or selected format', async ({page,viewer}) => {
  await viewer.open('04-hierarchy');
  await page.evaluate(() => { navigator.clipboard.readText = async () => '* Save Library\nR1 p n 1k'; });
  await page.evaluate(() => window.viewer.command('load-clipboard'));
  const dialog = page.locator('dialog.dialog');
  await dialog.locator('input[value="scs"]').check();
  const code = await dialog.locator('.clip-preview').textContent();
  await page.evaluate(async () => (await import('./i18n.js')).setLanguage('zh-CN'));
  await expect(dialog.locator('.dialog-note').first()).toContainText('2 行');
  await expect(dialog.getByRole('radio', {name:'Spectre 网表 (.scs)'})).toBeChecked();
  await expect(dialog.locator('.clip-preview')).toHaveText(code);
  await dialog.getByRole('button',{name:'取消',exact:true}).click();
});

test('symbol labels switch live and obsolete source tooltips stay removed after changing level', async ({page,viewer}) => {
  await viewer.open('symbol-inspector');
  await page.locator('#show-failed').check();
  const line=viewer.line(19);
  await expect(line).toHaveAttribute('title', /Not drawn/);
  await page.evaluate(async () => {
    const doc=window.viewer.doc;
    const level=doc.levels.findIndex(l=>l.title==='leaf');
    // Exercise a label whose automatic position is optional in the document protocol.
    for (const s of Object.values(doc.symbols)) for(const l of s.labels) l.at=null;
    (await import('./subcircuits.js')).enterRoot(level);
  });
  await expect(line).not.toHaveAttribute('title');
  await page.keyboard.press('4');
  await expect(page.locator('.symbol-label-list')).toContainText('automatic position');
  await viewer.symbolFitted();
  const svg=await page.locator('.symbol-preview > svg').getAttribute('viewBox');
  await language(page,'zh');
  await expect(page.locator('.symbol-label-list')).toContainText('自动位置');
  await expect(page.locator('.symbol-origin-info')).toHaveText('定义于 leaf');
  await expect(line).not.toHaveAttribute('title');
  await expect(page.locator('.symbol-preview > svg')).toHaveAttribute('viewBox',svg);
});

test('a pseudo language shows every text of the menu bar and the panel tabs translated (decision 0015)', async ({page,viewer}) => {
  // only the fake shell offers it, with ?pseudo: every text of the catalogs, its letters accented, in ［…］
  await viewer.open('04-hierarchy', {pseudo: '1'});
  await language(page,'qps');
  await expect(page.locator('html')).toHaveAttribute('lang','qps-ploc');
  await expect(page.locator('#tab-files')).toHaveAccessibleName(/^［/);
  // every title and entry of the web's menu bar but the names of the languages, which are their own
  const plain = await page.evaluate(() => [...document.querySelectorAll('.menu-title, .menu-item')]
    .filter((el) => !el.dataset.id?.startsWith('language-'))
    .map((el) => (el.querySelector('.label') ?? el).textContent.trim())
    .filter((text) => /[A-Za-z]/.test(text) && !text.startsWith('［')));
  expect(plain).toEqual([]);
  for (const tab of ['files','netlist','hierarchy','symbol']) await expect(page.locator(`#tab-${tab}`)).toHaveAccessibleName(/^［/);
  await language(page,'en');
  await expect(page.locator('html')).toHaveAttribute('lang','en');
  await expect(page.locator('#tab-files')).toHaveAccessibleName('Library');
});
