import { append, m, setText, tr } from "./i18n.js";
import { form, inform, modal } from './dialog.js';
import { isLoading } from './open.js';
import { shell } from './shell.js';
import { load, save } from './util.js';

/** File > Memory Cache: a per-window LRU budget, independent of the current drawing. */
export async function memoryCache() {
  if (!shell.memoryCache || isLoading() || modal()) return;
  try {
    const stats = await shell.memoryCache();
    if (isLoading() || modal()) return;
    const label = document.createElement('label');
    label.className = 'dialog-line';
    setText(label, m('Cache limit (MiB)'));
    const limit = document.createElement('input');
    limit.type = 'number'; limit.min = '0'; limit.max = '4096'; limit.step = '1';
    limit.value = String(stats.limit / 1024 / 1024);
    append(label, limit);
    const state = document.createElement('p');
    const prepared = document.createElement('p');
    prepared.hidden = !shell.preload;
    const show = s => {
      setText(prepared, m`${s.prefetched || 0} preloaded documents. Uses the same cache budget; starts new jobs only while idle.`);
      setText(state, m`${s.entries} cached document${s.entries === 1 ? '' : 's'} · ${(s.used / 1024 / 1024).toFixed(1)} MiB retained · ${s.hits} cache hits`);
    };
    show(stats);
    const preload = document.createElement('label');
    preload.className = 'dialog-line';
    preload.hidden = !shell.preload;
    const enabled = document.createElement('input');
    enabled.type = 'checkbox'; enabled.checked = load('background-preload', true) === true;
    append(preload, enabled, m('Preload nearby netlists while idle'));
    const error = document.createElement('p');
    error.className = 'dialog-error'; error.setAttribute('role', 'alert');
    const clear = document.createElement('button');
    clear.className = 'settings-restore';
    let clearing = Promise.resolve();
    clear.type = 'button'; setText(clear, m('Clear Cache'));
    clear.addEventListener('click', async () => {
      clear.disabled = true;
      try { clearing = shell.memoryCache(true); show(await clearing); setText(error, ''); }
      catch (e) { setText(error, String(e)); }
      finally { clear.disabled = false; }
    });
    await form({ title: m('Memory Cache'),
      text: m('Reuse compiled netlists and drawings. Least recently used results are removed first. Set 0 to disable caching. This budget excludes the current view and temporary build memory.'),
      parts: [label, preload, state, prepared, clear, error], focus: limit, action: tr('Apply'),
      answer: async () => {
        const value = limit.valueAsNumber;
        if (!limit.checkValidity() || !Number.isInteger(value)) { setText(error, m('Enter a whole number from 0 to 4096.')); return; }
        try {
          await clearing.catch(() => {}); await save('render-cache-mib', value);
          if (shell.preload) await save('background-preload', enabled.checked);
          await shell.memoryCache(); window.dispatchEvent(new Event('preload-reset')); return true;
        }
        catch (e) { setText(error, String(e)); }
      },
    });
  } catch (e) { await inform({ title: m('Memory Cache'), paragraphs: [String(e)] }); }
}
