import { test, expect } from '../../tests/viewer/node_modules/@playwright/test/index.mjs';
import { execFileSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

// Preview a future release without creating a tag or publishing any artifact.
const pages=JSON.parse(execFileSync(process.env.PYTHON || 'python',['-c',`
import sys,json
sys.path.insert(0,'site')
import build
from unittest.mock import patch
with patch.object(build,'has_macos_release',return_value=True), patch.object(build,'has_linux_release',return_value=True):
    pages=build.build('0.2.0')
print(json.dumps({lang:pages[lang+'/tools.html'] for lang in ['zh','en']}))
`],{encoding:'utf8',maxBuffer:4*1024*1024}));

for(const language of ['zh','en']) for(const theme of ['light','dark']) {
  test(`Mac downloads: ${language} ${theme}`,async({page},info)=>{
    await page.addInitScript(theme=>localStorage.setItem('rnl-intro-theme',theme),theme);
    await page.route(`**/${language}/tools-macos-preview.html`,route=>route.fulfill({contentType:'text/html',body:pages[language]}));
    await page.goto(`/${language}/tools-macos-preview.html`);
    const downloads=page.locator('#macos-downloads');
    await downloads.scrollIntoViewIfNeeded();
    await expect(downloads.locator('a[href$="-macos-arm64.dmg"]')).toHaveCount(1);
    await expect(downloads.locator('a[href$="-macos-x86_64.dmg"]')).toHaveCount(1);
    await expect(downloads).toContainText('xattr -dr com.apple.quarantine');
    await expect(downloads).toContainText('Control+Q');
    await expect(page.locator('html')).toHaveAttribute('data-theme',theme);
    expect(await downloads.evaluate(el=>el.scrollWidth<=el.clientWidth)).toBe(true);
    if(process.env.RNL_MACOS_SCREENSHOTS && info.project.name==='chromium') {
      mkdirSync(process.env.RNL_MACOS_SCREENSHOTS,{recursive:true});
      await page.evaluate(()=>document.fonts.ready);
      await downloads.screenshot({path:join(process.env.RNL_MACOS_SCREENSHOTS,`tools-${language}-${theme}.png`),animations:'disabled'});
    }
  });
}
