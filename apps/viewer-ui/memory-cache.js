import { append, m, setText, tr } from "./i18n.js";
import { form, inform, modal } from './dialog.js';
import { isLoading } from './open.js';
import { shell } from './shell.js';
import { load, save } from './util.js';

const MIB = 1024 * 1024;
const mib = (bytes) => (bytes / MIB).toFixed(1);

/**
 * File > Memory Cache. On the desktop, a per-window LRU budget, independent of the current drawing. In a shell that
 * reads the file shown alone (the web viewer and the local program's page), the readings of each file kept in memory
 * for the page (`ResultCache` of result-cache.js): a limit, 0 to keep none, and Clear Cache empties it.
 */
export async function memoryCache() {
  if (!shell.memoryCache || isLoading() || modal()) return;
  try {
    const stats = await shell.memoryCache();
    if (isLoading() || modal()) return;
    // the readings of each file kept by the web shell
    const web = stats.kind === 'readings';
    const field = (text, value, max) => {
      const label = document.createElement('label');
      label.className = 'dialog-line';
      setText(label, text);
      const input = document.createElement('input');
      input.type = 'number'; input.min = '0'; input.max = String(max); input.step = '1';
      input.value = String(value);
      append(label, input);
      return { label, input };
    };
    const limit = web
      ? field(m('Limit in memory (MiB)'), load('file-cache-memory-mib', stats.limit / MIB), 4096)
      : field(m('Cache limit (MiB)'), stats.limit / MIB, 4096);
    const state = document.createElement('p');
    const prepared = document.createElement('p');
    prepared.hidden = !shell.preload;
    const show = s => {
      setText(prepared, m`${s.prefetched || 0} preloaded documents. Uses the same cache budget; starts new jobs only while idle.`);
      if (!web) {
        setText(state, m`${s.entries} cached document${s.entries === 1 ? '' : 's'} · ${mib(s.used)} MiB retained · ${s.hits} cache hits`);
        return;
      }
      setText(state, m`Readings in memory: ${s.entries} (${mib(s.used)} MiB) · ${s.hits} hits, ${s.misses} misses`);
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
    const whole = (input, max) => input.checkValidity() && Number.isInteger(input.valueAsNumber) && input.valueAsNumber <= max;
    await form({ title: m('Memory Cache'),
      text: web
        ? m("Keep the reading of each file in memory for this page, so that a file shown again is not read again. The least recently used go first. Set 0 to keep none.")
        : m('Reuse compiled netlists and drawings. Least recently used results are removed first. Set 0 to disable caching. This budget excludes the current view and temporary build memory.'),
      parts: [limit.label, preload, state, prepared, clear, error], focus: limit.input, action: tr('Apply'),
      answer: async () => {
        if (!whole(limit.input, 4096)) { setText(error, m('Enter a whole number from 0 to 4096.')); return; }
        try {
          await clearing.catch(() => {});
          await save(web ? 'file-cache-memory-mib' : 'render-cache-mib', limit.input.valueAsNumber);
          if (shell.preload) await save('background-preload', enabled.checked);
          await shell.memoryCache(); window.dispatchEvent(new Event('preload-reset')); return true;
        }
        catch (e) { setText(error, String(e)); }
      },
    });
  } catch (e) { await inform({ title: m('Memory Cache'), paragraphs: [String(e)] }); }
}
