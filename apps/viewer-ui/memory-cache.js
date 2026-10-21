import { form, inform, modal } from './dialog.js';
import { isLoading } from './open.js';
import { shell } from './shell.js';
import { save } from './util.js';

/** File > Memory Cache: a per-window LRU budget, independent of the current drawing. */
export async function memoryCache() {
  if (!shell.memoryCache || isLoading() || modal()) return;
  try {
    const stats = await shell.memoryCache();
    if (isLoading() || modal()) return;
    const label = document.createElement('label');
    label.className = 'dialog-line';
    label.textContent = 'Cache limit (MiB)';
    const limit = document.createElement('input');
    limit.type = 'number'; limit.min = '0'; limit.max = '4096'; limit.step = '1';
    limit.value = String(stats.limit / 1024 / 1024);
    label.append(limit);
    const state = document.createElement('p');
    const show = s => { state.textContent = `${s.entries} cached document${s.entries === 1 ? '' : 's'} · ${(s.used / 1024 / 1024).toFixed(1)} MiB retained · ${s.hits} cache hits`; };
    show(stats);
    const error = document.createElement('p');
    error.className = 'dialog-error'; error.setAttribute('role', 'alert');
    const clear = document.createElement('button');
    clear.className = 'settings-restore';
    let clearing = Promise.resolve();
    clear.type = 'button'; clear.textContent = 'Clear Cache';
    clear.addEventListener('click', async () => {
      clear.disabled = true;
      try { clearing = shell.memoryCache(true); show(await clearing); error.textContent = ''; }
      catch (e) { error.textContent = String(e); }
      finally { clear.disabled = false; }
    });
    await form({ title: 'Memory Cache',
      text: 'Reuse compiled netlists and drawings. Least recently used results are removed first. Set 0 to disable caching. This budget excludes the current view and temporary build memory.',
      parts: [label, state, clear, error], focus: limit, action: 'Apply',
      answer: async () => {
        const value = limit.valueAsNumber;
        if (!limit.checkValidity() || !Number.isInteger(value)) { error.textContent = 'Enter a whole number from 0 to 4096.'; return; }
        try { await clearing.catch(() => {}); await save('render-cache-mib', value); await shell.memoryCache(); return true; }
        catch (e) { error.textContent = String(e); }
      },
    });
  } catch (e) { await inform({ title: 'Memory Cache', paragraphs: [String(e)] }); }
}
