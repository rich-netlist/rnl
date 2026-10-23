// Native CLI and npm/WASM inspection overlays must export identical SVG bytes.
// Run after building the CLI and bindings/runtime-wasm/dist.
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {mkdtemp,readFile,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {dirname,join,resolve} from 'node:path';
import {test} from 'node:test';
import {fileURLToPath,pathToFileURL} from 'node:url';
const here = dirname(fileURLToPath(import.meta.url));
const dist = resolve(process.argv[2] || resolve(here, '../dist'));
const {loadCore} = await import(pathToFileURL(join(dist, 'core.js')));
const core = await loadCore(pathToFileURL(join(dist, 'rnl_runtime_wasm.wasm')));

test("inspection overlays match native CLI export and keep default SVG bytes", async () => {
  const source = await readFile(new URL('../../../tests/viewer/inputs/usage-current.sp', import.meta.url), 'utf8');
  const dir = await mkdtemp(join(tmpdir(), 'rnl-overlays-'));
  try {
    const entry = join(dir, 'usage-current.sp').replaceAll('\\', '/');
    await writeFile(entry, source);
    const request = {entry, files: {[entry]: source}};
    const plain = core.build(request);
    const disabled = core.build({...request, overlays: {}});
    assert.deepEqual(disabled, plain);
    const overlays = {usages: 'all', currents: true, roles: true};
    const rendered = core.build({...request, overlays});
    assert.deepEqual(rendered.levels.map(l => l.currents), plain.levels.map(l => l.currents));
    assert.deepEqual(rendered.levels.map(l => l.usages), plain.levels.map(l => l.usages));
    const rnl = process.env.RNL_BIN || resolve(here, '../../../target/debug', process.platform === 'win32' ? 'rnl.exe' : 'rnl');
    const out = join(dir, 'out');
    const args = ['render', entry, '-o', out, '--usages', 'all', '--currents', '--usage-roles'];
    execFileSync(rnl, args);
    for (const [k, lv] of rendered.levels.entries()) {
      const name = k === 0 ? 'usage-current.svg' : 'usage-current.cell.svg';
      assert.equal(lv.svg, await readFile(join(out, name), 'utf8'));
    }
    const lazy = core.build({...request, overlays, lazy_unplaced: true});
    assert.equal(lazy.levels[0].svg, rendered.levels[0].svg);
    const snapshot = core.snapshot({...request, overlays, parts: {drawing: true}});
    assert.equal(snapshot.drawing.levels[0].svg, rendered.levels[0].svg);
    const style = {palette: ['#123456'], width: 3, dashes: [], current_color: '#654321'};
    await writeFile(join(dir, 'style.json'), JSON.stringify(style));
    execFileSync(rnl, [...args, '--overlay-style', join(dir, 'style.json')]);
    const custom = core.build({...request, overlays: {...overlays, style}});
    assert.equal(custom.levels[0].svg, await readFile(join(out, 'usage-current.svg'), 'utf8'));
    assert.throws(() => core.build({...request, overlays: {style: {palette: []}}}), /palette/);
  } finally { await rm(dir, {recursive: true, force: true}); }
});
