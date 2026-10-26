import {expect, test} from '../viewer.js';
import { zh } from '../zh.js';

const defaults = {
  component: '#2275E2', net: '#A01008', rnl: '#E1FEE9', keyword: '#000000', definition: '#2B7800',
  local: '#4B21E4', string: '#895D7C', parameter: '#1D4781', free: '#6B6B6B', text: '#000000',
  background: '#FFFFFF', highlight: '#E8590C', subcircuit: '#C2255C',
};
const rgb = hex => `rgb(${[1,3,5].map(i => parseInt(hex.slice(i,i+2),16)).join(', ')})`;
const box = (page, key) => page.locator(`#netlist-color-${key}`);
const picker = (page, key) => page.locator(`.netlist-color-row[data-color="${key}"] input[type=color]`);
async function open(page, platform) {
  if (platform === 'web') {
    await page.locator('#menubar .menu-title', {hasText: /^View$/}).click();
    await page.locator('#menubar [data-id="netlist-colors"]').click();
  } else await page.evaluate(() => window.fakeShell.menu('netlist-colors'));
  await expect(page.getByRole('dialog')).toBeVisible();
}
const palette = page => page.locator('.netlist-color-row').evaluateAll(rows => Object.fromEntries(rows.map(r =>
  [r.dataset.color, r.querySelector('input[type=text]').value])));
const stored = (page, platform) => page.evaluate(platform => platform === 'desktop'
  ? JSON.parse(sessionStorage.getItem('fake-settings'))['netlist-colors']
  : JSON.parse(localStorage.getItem('netlist-colors')), platform);

for (const platform of ['web', 'desktop']) {
  const settings = {platform, ...(platform === 'desktop' ? {settings:'1'} : {})};
  test(`${platform}: the thirteen defaults colour real syntax and remain independent of UI and drawing themes`, async ({page, viewer}) => {
    await viewer.open('symbol-inspector', settings);
    const styles = {'.s-host':'component', '.s-inst':'component', '.s-net':'net', '.s-keyword':'keyword',
      '.s-def':'definition', '.s-local':'local', '.s-string':'string', '.s-param':'parameter', '.s-free':'free', '.s-number':'text'};
    for (const scheme of ['light', 'dark']) {
      await page.emulateMedia({colorScheme:scheme});
      for (const [selector, key] of Object.entries(styles)) await expect(page.locator(`#source ${selector}`).first()).toHaveCSS('color', rgb(defaults[key]));
      await expect(page.locator('#source')).toHaveCSS('background-color', rgb(defaults.background));
      await expect(page.locator('#source .rnl-line:not(.semantic-rnl):not(.inactive-line)').first()).toHaveCSS('background-color', rgb(defaults.rnl));
    }
    await open(page, platform);
    expect(await palette(page)).toEqual(defaults);
    await expect(page.locator('.netlist-color-row input[type=color]')).toHaveCount(13);
    // the two highlights stand together
    expect((await palette(page).then(Object.keys)).slice(-2)).toEqual(['highlight', 'subcircuit']);
    await expect(page.locator('.dialog-buttons button')).toHaveText(['Restore Defaults', 'Cancel', 'Save']);
    await page.getByRole('button', {name:'Cancel', exact:true}).click();
    await viewer.click('Xparent');
    await expect(page.locator('#source .obj.hl').first()).toHaveCSS('color', rgb(defaults.highlight));
    await expect(page.locator('#source .obj.hl').first()).toHaveCSS('text-decoration-line', 'underline');
    // A subcircuit retains its separate drawing colour; only its Netlist highlight uses the palette.
    await expect(page.locator('#canvas g.sel [stroke]:not([stroke=none])').first()).toHaveCSS('stroke','rgb(194, 37, 92)');
  });

  test(`${platform}: hex and picker changes preview without rebuilding; Cancel and invalid input are reversible`, async ({page, viewer}) => {
    await viewer.open('usage-inspector', settings);
    const builds = await viewer.builds();
    const original = await page.evaluate(() => { window.colorDoc=window.viewer.doc; window.colorSvg=document.querySelector('#canvas > svg'); return document.querySelector('#source').scrollTop; });
    await open(page, platform);
    await box(page,'component').fill('123abc');
    await expect(picker(page,'component')).toHaveValue(/^#123abc$/i);
    await expect(page.locator('#source .s-host').first()).toHaveCSS('color','rgb(18, 58, 188)');
    await picker(page,'background').fill('#f0e8d8');
    await expect(box(page,'background')).toHaveValue('#F0E8D8');
    await expect(page.locator('#source')).toHaveCSS('background-color','rgb(240, 232, 216)');
    await box(page,'highlight').fill('wrong');
    await page.getByRole('button',{name:'Save',exact:true}).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(box(page,'highlight')).toBeFocused();
    await expect(box(page,'highlight')).toHaveAttribute('aria-invalid','true');
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(page.locator('#source .s-host').first()).toHaveCSS('color',rgb(defaults.component));
    await expect(page.locator('#source')).toHaveCSS('background-color',rgb(defaults.background));
    expect(await page.evaluate(() => window.viewer.doc===window.colorDoc && document.querySelector('#canvas > svg')===window.colorSvg)).toBe(true);
    expect(await viewer.builds()).toBe(builds);
    expect(await page.locator('#source').evaluate(e=>e.scrollTop)).toBe(original);
  });

  test(`${platform}: Save persists all colours through reload, restores defaults and translates the dialog`, async ({page, viewer}, info) => {
    await viewer.open('usage-inspector', settings);
    await open(page,platform);
    await box(page,'net').fill('#102030');
    await box(page,'highlight').fill('#087F8C');
    await box(page,'free').fill('#775533');
    await page.getByRole('button',{name:'Save',exact:true}).click();
    await expect(page.getByRole('dialog')).toHaveCount(0);
    const custom={...defaults,net:'#102030',highlight:'#087F8C',free:'#775533'};
    await expect.poll(()=>stored(page,platform)).toEqual(custom);
    await page.reload(); await viewer.started(); await viewer.idle();
    await expect(page.locator('#source .s-net').first()).toHaveCSS('color','rgb(16, 32, 48)');
    await viewer.click('R1');
    await expect(page.locator('#source .obj.hl').first()).toHaveCSS('color','rgb(8, 127, 140)');
    await open(page,platform);
    expect(await palette(page)).toEqual(custom);
    await page.getByRole('button',{name:'Restore Defaults',exact:true}).click();
    expect(await palette(page)).toEqual(defaults);
    await page.getByRole('button',{name:'Cancel',exact:true}).click();
    expect(await stored(page,platform)).toEqual(custom);
    await page.evaluate(()=>window.viewer.command('language-zh'));
    await page.evaluate(()=> { window.viewer.command('netlist-colors'); });
    await expect(page.getByRole('dialog')).toHaveAccessibleName(zh('Color Scheme'));
    await expect(page.getByLabel(zh('Component objects (#)'),{exact:true})).toBeVisible();
    await expect(page.getByLabel(zh('{0} color picker',zh('Strings')),{exact:true})).toBeVisible();
    await page.getByRole('button',{name:zh('Restore Defaults'),exact:true}).click();
    await page.screenshot({path:info.outputPath('netlist-colors-dialog.png')});
    await page.getByRole('button',{name:zh('Save'),exact:true}).click();
    await expect.poll(()=>stored(page,platform)).toEqual(defaults);
    await page.reload(); await viewer.started(); await viewer.idle();
    await expect(page.locator('#source .s-net').first()).toHaveCSS('color',rgb(defaults.net));
  });
}

for (const background of ['light', 'dark']) {
  test(`${background}: the second highlight colours a selected subcircuit on the drawing, not the netlist nor the zoom rectangle`, async ({page, viewer}, info) => {
    if (background === 'dark') await page.emulateMedia({colorScheme: 'dark'});
    await viewer.open('symbol-inspector');
    if (background === 'dark') await page.evaluate(() => window.viewer.command('background-dark'));
    const drawingSub = () => page.locator('#canvas').evaluate(c => getComputedStyle(c).getPropertyValue('--drawing-sub').trim());
    const before = await drawingSub();
    await open(page, 'web');
    await expect(page.getByLabel('Subcircuit selection highlight (drawing)', {exact: true})).toHaveValue(defaults.subcircuit);
    if (process.env.RNL_REVIEW_DIR && info.project.name === 'chromium') {
      await page.getByRole('dialog').screenshot({path: `${process.env.RNL_REVIEW_DIR}/second-highlight-scheme-${background}.png`});
    }
    await box(page, 'subcircuit').fill('#1C7C54');
    // previewed at once on the drawing, lightened on a dark background as the first highlight is
    await expect.poll(drawingSub).not.toBe(before);
    if (background === 'light') expect(await drawingSub()).toBe('#1C7C54');
    await page.getByRole('button', {name: 'Save', exact: true}).click();
    await viewer.click('Xparent');
    await expect(page.locator('#canvas')).toHaveClass(/sel-sub/);
    const stroke = await page.locator('#canvas g.sel [stroke]:not([stroke=none])').first().evaluate(e => getComputedStyle(e).stroke);
    const expected = await page.locator('#canvas').evaluate((c, colour) => {
      const probe = document.createElement('span'); probe.style.color = colour; c.append(probe);
      const value = getComputedStyle(probe).color; probe.remove(); return value;
    }, await drawingSub());
    expect(stroke).toBe(expected);
    // the netlist keeps its own highlight
    await expect(page.locator('#source .obj.hl').first()).toHaveCSS('color', rgb(defaults.highlight));
    // a right drag's rectangle keeps the plain highlight
    const r = await page.locator('#canvas').boundingBox();
    await page.mouse.move(r.x + r.width * 0.2, r.y + r.height * 0.3);
    await page.mouse.down({button: 'right'});
    await page.mouse.move(r.x + r.width * 0.6, r.y + r.height * 0.6, {steps: 5});
    const border = await page.locator('.zoom-box').evaluate(e => getComputedStyle(e).borderTopColor);
    const plain = await page.locator('#canvas').evaluate(c => {
      const probe = document.createElement('span'); probe.style.color = getComputedStyle(c).getPropertyValue('--drawing-select'); c.append(probe);
      const value = getComputedStyle(probe).color; probe.remove(); return value;
    });
    await page.mouse.up({button: 'right'});
    expect(border).toBe(plain);
  });
}

test('saved palette validation ignores invalid colours and saving failure restores the previous display', async ({page,viewer}) => {
  await viewer.open('usage-inspector');
  await page.evaluate(()=>localStorage.setItem('netlist-colors',JSON.stringify({component:'red',net:'#123456',text:'url(evil)'})));
  await page.reload(); await viewer.started(); await viewer.idle();
  await expect(page.locator('#source .s-host').first()).toHaveCSS('color',rgb(defaults.component));
  await expect(page.locator('#source .s-net').first()).toHaveCSS('color','rgb(18, 52, 86)');
  await open(page,'web');
  await box(page,'component').fill('#445566');
  await page.evaluate(()=> { window.realSetItem=Storage.prototype.setItem; Storage.prototype.setItem=function(){throw new Error('storage denied');}; });
  await page.getByRole('button',{name:'Save',exact:true}).click();
  await expect(page.getByRole('dialog')).toContainText('Could not save color scheme');
  await expect(page.locator('#source .s-host').first()).toHaveCSS('color',rgb(defaults.component));
  await page.evaluate(()=>Storage.prototype.setItem=window.realSetItem);
});

test('role/free-text syntax survives virtual-line checkpoints and host strings keep embedded comment characters', async ({page,viewer}) => {
  await viewer.open('usage-inspector');
  const classes=await page.evaluate(async()=> {
    const {lineModel}=await import('./text-lines.js');
    const lines=['* @RNL { #R1(', '* left) is treated @AS $PAIR;',
      '* {#R1 #R2}(group) and (4, 5)(axis);', '* #M<1>(left) #M[2](right) #M<',
      '* 3 /* note */', '* >(tail) #M<1>:d(drain);', '* ?left @PLACE(0, 0) }'];
    const f={text:lines.join('\n'),records:[[1,3,lines.length,lines.at(-1).length]],failed:[]};
    const model=lineModel(f,1,[],'spice');
    const tokens=i=>[...model.render(i).querySelectorAll('[class^="s-"]')].map(e=>[e.textContent,e.className]);
    const {hostLine,rnlRange}=await import('./syntax.js');
    const gap='#R1 (', state={};
    rnlRange(gap,0,gap.length,true,Array(gap.length).fill(''),state);
    const host='parameters note="value ; $ // kept" // comment';
    const cls=Array(host.length).fill(''); hostLine(host,cls,false,null);
    return {role:tokens(1), group:tokens(2), subscript:tokens(3), continued:tokens(5), free:tokens(6),
      pointAfterSpace:state.stack, string:cls.slice(host.indexOf('"'),host.lastIndexOf('"')+1), comment:cls.slice(host.lastIndexOf('//'))};
  });
  expect(classes.role).toContainEqual(['left','s-param']);
  expect(classes.role).toContainEqual(['is','s-free']);
  for (const role of ['group','axis']) expect(classes.group).toContainEqual([role,'s-param']);
  for (const role of ['left','right']) expect(classes.subscript).toContainEqual([role,'s-param']);
  for (const role of ['tail','drain']) expect(classes.continued).toContainEqual([role,'s-param']);
  expect(classes.pointAfterSpace).toBe('p');
  expect(classes.free).toContainEqual(['?left','s-param']);
  expect(new Set(classes.string)).toEqual(new Set(['string']));
  expect(new Set(classes.comment)).toEqual(new Set(['comment']));
});
