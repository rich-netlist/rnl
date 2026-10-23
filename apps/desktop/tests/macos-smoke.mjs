// Native macOS smoke for an installed .app copied out of its DMG. Uses WindowServer pixels
// and Apple's Vision OCR; DOM existence alone cannot pass. Run on a logged-in macOS runner.
import assert from 'node:assert/strict';
import {execFileSync, spawnSync} from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

assert.equal(process.platform,'darwin');
const app=path.resolve(process.argv[process.argv.indexOf('--app')+1]);
assert.ok(app.endsWith('.app') && fs.existsSync(app),'Pass --app PATH.app');
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../../..');
const output=process.env.SMOKE_OUTPUT_DIR || fs.mkdtempSync(path.join(os.tmpdir(),'rnl-macos-smoke-'));
fs.mkdirSync(output,{recursive:true});
const sample=path.join(root,'tests/conformance/cases/04-hierarchy.sp');
const original=fs.readFileSync(sample);
const observer=path.join(output,'observe');
execFileSync('swiftc',[path.join(root,'apps/desktop/tests/macos-observe.swift'),'-o',observer]);
execFileSync('codesign',['--verify','--deep','--strict','--verbose=2',app],{stdio:'inherit'});
const signature=spawnSync('codesign',['-dvv',app],{encoding:'utf8'});
fs.writeFileSync(path.join(output,'signature.txt'),signature.stdout+signature.stderr);
assert.match(signature.stderr,/Signature=adhoc/,'The shipped bundle must be ad-hoc signed.');
const osa=script=>execFileSync('osascript',['-e',script],{encoding:'utf8',timeout:15000});
const key=(key,modifiers='')=>osa(`tell application "System Events" to keystroke ${JSON.stringify(key)} ${modifiers}`);
const menu=(name,item)=>osa(`tell application "System Events" to tell (first application process whose bundle identifier is "io.github.rich-netlist.viewer")
  click menu item ${JSON.stringify(item)} of menu 1 of menu bar item ${JSON.stringify(name)} of menu bar 1
end tell`);
async function observe(label,check) {
  const end=Date.now()+90000; let last;
  while(Date.now()<end) {
    try {
      const result=execFileSync(observer,[path.join(output,`${label}.png`)],{encoding:'utf8',timeout:20000});
      last=JSON.parse(result);
      fs.writeFileSync(path.join(output,`${label}.json`),JSON.stringify(last,null,2));
      if(check(last)) return last;
    } catch(error) { last=String(error); }
    await new Promise(resolve=>setTimeout(resolve,500));
  }
  throw new Error(`Timed out: ${label}: ${JSON.stringify(last)}`);
}
const text=r=>r.words.map(w=>w.text).join(' ');
const drawn=(r,left=0.38)=>{
  const labels=r.words.filter(w=>w.x>left), amps=labels.filter(w=>/\bamp\b/.test(w.text));
  // X1/X2 are only ~10 pixels high on the runner's 1024px desktop and Vision can omit them.
  // Two separated stage names plus a source/supply/third-instance label prove the schematic
  // independently of source text. Individual tiny labels can be omitted by Vision.
  return r.circuitInk>200 && r.circuitDensity<0.35 && amps.length>=2 &&
    Math.max(...amps.map(w=>w.x))-Math.min(...amps.map(w=>w.x))>0.08 &&
    labels.some(w=>/\b(?:Vin|Vdd|X3)\b/i.test(w.text));
};

try {
  // This is the documented terminal workaround, applied to the installed bundle, not its build tree.
  execFileSync('xattr',['-w','com.apple.quarantine','0083;00000000;RNL CI;',app]);
  execFileSync('xattr',['-dr','com.apple.quarantine',app]);
  execFileSync('open',['-n','-a',app,sample]);
  await observe('finder-cold',r=>text(r).includes('04-hierarchy.sp') && drawn(r));
  osa('tell application "System Events" to tell (first application process whose bundle identifier is "io.github.rich-netlist.viewer") to set frontmost to true');
  const menus=osa('tell application "System Events" to tell (first application process whose bundle identifier is "io.github.rich-netlist.viewer") to get name of every menu bar item of menu bar 1');
  fs.writeFileSync(path.join(output,'menus.txt'),menus);
  menu('RNL Viewer','About RNL Viewer');
  await observe('about',r=>/RNL core/.test(text(r)));
  osa('tell application "System Events" to key code 53');
  // Cmd+B belongs to the native menu; physical Control+Q edits, leaving Cmd+Q to Quit.
  key('b','using command down');
  await observe('sidebar-hidden',r=>!r.words.some(w=>w.x<0.35 && /Netlist/.test(w.text)) && drawn(r,0.03));
  key('b','using command down');
  await observe('sidebar-restored',r=>r.words.some(w=>w.x<0.35 && /Netlist/.test(w.text)) && drawn(r));
  menu('Help','Check for Updates');
  await observe('updates',r=>/Check for Updates/.test(text(r)) && /latest|newer|available|no release|could not/i.test(text(r)));
  osa('tell application "System Events" to key code 53');
  key('q','using control down');
  await observe('editing',r=>/Apply and Exit/.test(text(r)));
  key('a','using command down'); key('c','using command down');
  assert.match(execFileSync('pbpaste',{encoding:'utf8'}),/Two-stage buffer/,'Native Cmd+A/C must select/copy the source.');
  key('q','using control down');
  await observe('reading',drawn);
  // A second LaunchServices open reaches an already-running app, as Finder's Open With does.
  const next=path.join(root,'tests/conformance/equivalence/inverter-chain.scs');
  execFileSync('open',['-a',app,next]);
  await observe('finder-warm',r=>text(r).includes('inverter-chain.scs') && r.circuitInk>200 && r.circuitDensity<0.35);
  execFileSync('open',['-a',app,sample]);
  await observe('finder-return',drawn);
  osa('tell application "System Events" to tell appearance preferences to set dark mode to true');
  await observe('dark',r=>drawn(r) && r.footerBrightness<0.3);
  osa('tell application "System Events" to tell appearance preferences to set dark mode to false');
  await observe('light',r=>drawn(r) && r.footerBrightness>0.7);
  const settings=path.join(os.homedir(),'Library/Application Support/io.github.rich-netlist.viewer/settings.json');
  assert.ok(fs.existsSync(settings),'Settings must use the macOS application support directory.');
  fs.copyFileSync(settings,path.join(output,'settings.json'));
  assert.deepEqual(fs.readFileSync(sample),original,'Opening the sample must not write it.');
  key('q','using command down');
  console.log('Packaged app: ad-hoc signature, cold/warm Finder opens, real schematic pixels/OCR, menus, Cmd/Control keys, settings and dark/light screenshots passed.');
} finally {
  spawnSync('pkill',['-x','rnl-viewer']);
  osa('tell application "System Events" to tell appearance preferences to set dark mode to false');
}
