// Optional release rehearsal: rebuild with only the UI fallback removed, keeping the same
// source commit, toolchain, font payload and bundler. Never replaces the candidate assets.
import {execFileSync} from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
const root=process.cwd(), app=path.join(root,'apps/desktop');
const css=path.join(root,'apps/viewer-ui/style.css');
const fixed=fs.readFileSync(css,'utf8');
const baseline=fixed.replaceAll(', var(--ui-cjk)','').replace(/\/\* Web fonts follow the generic families:[\s\S]*?\*\/\r?\n\r?\n/,'');
const assets=process.env.FONT_SIZE_ASSETS;
const output=process.env.FONT_SIZE_OUTPUT;
if(!assets || !output || baseline===fixed) throw new Error('Pass FONT_SIZE_ASSETS and FONT_SIZE_OUTPUT with the fallback present.');
const version=JSON.parse(fs.readFileSync(path.join(app,'tauri.conf.json'))).version;
const windows=process.platform==='win32';
const bundle=path.join(app,'target/release/bundle');
const pairs=windows ? [
  [`RNL-Viewer-${version}-windows-x64.exe`,path.join(app,'target/release/rnl-viewer.exe')],
  [`RNL-Viewer-${version}-windows-x64-setup.exe`,path.join(bundle,`nsis/RNL Viewer_${version}_x64-setup.exe`)],
  [`RNL-Viewer-${version}-windows-x64.msi`,path.join(bundle,`msi/RNL Viewer_${version}_x64_en-US.msi`)],
] : [
  [`RNL-Viewer-${version}-linux-x86_64.AppImage`,path.join(bundle,`appimage/RNL Viewer_${version}_amd64.AppImage`)],
  [`RNL-Viewer-${version}-linux-x86_64.deb`,path.join(bundle,`deb/RNL Viewer_${version}_amd64.deb`)],
];
const fonts=path.join(root,'apps/viewer-ui/fonts');
const payload=fs.readdirSync(fonts).sort().map(name=>[name,fs.readFileSync(path.join(fonts,name))]);
const digest=()=>createHash('sha256').update(Buffer.concat(payload.flatMap(([name,bytes])=>[Buffer.from(name),bytes]))).digest('hex');
try {
  fs.writeFileSync(css,baseline);
  execFileSync(windows?'npm.cmd':'npm',['exec','--','tauri','build',...(windows?[]:['--bundles','appimage,deb'])],
    {cwd:app,stdio:'inherit',shell:windows,env:{...process.env,APPIMAGE_EXTRACT_AND_RUN:'1'}});
  const comparison={css:{before:Buffer.byteLength(baseline),after:Buffer.byteLength(fixed),delta:Buffer.byteLength(fixed)-Buffer.byteLength(baseline)},
    fonts:{files:payload.length,bytes:payload.reduce((sum,[,data])=>sum+data.length,0),sha256:digest(),addedBytes:0},
    packages:pairs.map(([name,before])=>{const old=fs.statSync(before).size, current=fs.statSync(path.join(assets,name)).size;
      return {name,before:old,after:current,delta:current-old};})};
  fs.mkdirSync(path.dirname(output),{recursive:true});
  fs.writeFileSync(output,JSON.stringify(comparison,null,2));
  console.log(JSON.stringify(comparison,null,2));
} finally { fs.writeFileSync(css,fixed); }
