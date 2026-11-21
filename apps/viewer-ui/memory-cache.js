import { append, m, setText, tr } from "./i18n.js";
import { form, inform, modal } from './dialog.js';
import { isLoading } from './open.js';
import { shell } from './shell.js';
import { load, save } from './util.js';

const MIB = 1024 * 1024;
const mib = (bytes) => (bytes / MIB).toFixed(1);

/**
 * File > Memory Cache: the readings of each file kept, independent of the current drawing. In the web viewer and the
 * local program's page (`ResultCache` of result-cache.js), a limit in memory for the page and one in the browser's
 * storage for the site; on the desktop (`disk` in the stats: build_cache.rs, disk_cache.rs), a limit in memory for the
 * window and one on disk for every window. Each is 0 to keep none there, and Clear Cache empties both.
 */
export async function memoryCache() {
  if (!shell.memoryCache || isLoading() || modal()) return;
  try {
    const stats = await shell.memoryCache();
    if (isLoading() || modal()) return;
    // the second limit: in the browser's storage, or on disk
    const disk = stats.disk === true;
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
    const limit = field(m('Limit in memory (MiB)'), load('file-cache-memory-mib', stats.limit / MIB), 4096);
    const stored = field(disk ? m('Limit on disk (MiB)') : m("Limit in this browser's storage (MiB)"),
      load('file-cache-store-mib', Math.floor(stats.storeLimit / MIB)), 65536);
    if (!stats.persistent) stored.label.hidden = true;
    const state = document.createElement('p');
    const kept = document.createElement('p');
    const here = document.createElement('p');
    here.hidden = !disk;
    setText(here, m("The readings on disk are kept on this computer, in the app's cache folder; Clear Cache deletes them."));
    const prepared = document.createElement('p');
    prepared.hidden = !shell.preload;
    const show = s => {
      setText(prepared, m`${s.prefetched || 0} preloaded documents. Uses the same cache budget; starts new jobs only while idle.`);
      setText(state, m`Readings in memory: ${s.entries} (${mib(s.used)} MiB) · ${s.hits} hits, ${s.misses} misses`);
      if (!s.persistent) setText(kept, disk ? '' : m("This browser keeps nothing for the site, as in a private window: the readings are kept in memory alone."));
      else if (s.stored === null) setText(kept, '');
      else if (disk) setText(kept, m`Readings on disk: ${s.storedEntries} (${mib(s.stored)} MiB)`);
      else setText(kept, m`Readings in this browser's storage: ${s.storedEntries} (${mib(s.stored)} MiB)`);
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
      text: disk
        ? m("Keep the reading of each file, so that a file shown again, also after the app is started again, is not read again: in memory for this window, and on disk for every window. The least recently used go first. Set 0 to keep none there.")
        : m("Keep the reading of each file, so that a file shown again, also in a later visit, is not read again: in memory for this page, and in this browser's storage for this site. The least recently used go first. Set 0 to keep none there."),
      parts: [limit.label, stored.label, preload, state, kept, here, prepared, clear, error], focus: limit.input, action: tr('Apply'),
      answer: async () => {
        if (!whole(limit.input, 4096)) { setText(error, m('Enter a whole number from 0 to 4096.')); return; }
        if (!whole(stored.input, 65536)) { setText(error, m('Enter a whole number from 0 to 65536.')); return; }
        try {
          await clearing.catch(() => {});
          await save('file-cache-memory-mib', limit.input.valueAsNumber);
          await save('file-cache-store-mib', stored.input.valueAsNumber);
          if (shell.preload) await save('background-preload', enabled.checked);
          await shell.memoryCache(); window.dispatchEvent(new Event('preload-reset')); return true;
        }
        catch (e) { setText(error, String(e)); }
      },
    });
  } catch (e) { await inform({ title: m('Memory Cache'), paragraphs: [String(e)] }); }
}
