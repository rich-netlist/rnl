import { append, m, setText, tr } from "./i18n.js";
import { form, inform, modal } from './dialog.js';
import { memoryCache as readings, readsByFile } from './file-reading.js';
import { isLoading } from './open.js';
import { shell } from './shell.js';
import { load, save } from './util.js';

const MIB = 1024 * 1024;
const mib = (bytes) => (bytes / MIB).toFixed(1);

/**
 * File > Memory Cache, the same in every shell that reads the file shown alone: the readings of each file kept in
 * memory for this window or page (file-reading.js): a limit, 0 to keep none, how many readings it holds, their size
 * and the hits and misses, and Clear Cache, which empties it, the document shown staying as it is.
 */
export async function memoryCache() {
  if (!readsByFile() || shell.embed || isLoading() || modal()) return;
  try {
    const stats = await readings();
    if (isLoading() || modal()) return;
    const label = document.createElement('label');
    label.className = 'dialog-line';
    setText(label, m('Limit in memory (MiB)'));
    const input = document.createElement('input');
    input.type = 'number'; input.min = '0'; input.max = '4096'; input.step = '1';
    input.value = String(load('file-cache-memory-mib', stats.limit / MIB));
    append(label, input);
    const state = document.createElement('p');
    const show = (s) => setText(state, m`Readings in memory: ${s.entries} (${mib(s.used)} MiB) · ${s.hits} hits, ${s.misses} misses`);
    show(stats);
    const error = document.createElement('p');
    error.className = 'dialog-error'; error.setAttribute('role', 'alert');
    const clear = document.createElement('button');
    clear.className = 'settings-restore';
    let clearing = Promise.resolve();
    clear.type = 'button'; setText(clear, m('Clear Cache'));
    clear.addEventListener('click', async () => {
      clear.disabled = true;
      try { clearing = readings(true); show(await clearing); setText(error, ''); }
      catch (e) { setText(error, String(e)); }
      finally { clear.disabled = false; }
    });
    const whole = () => input.checkValidity() && Number.isInteger(input.valueAsNumber) && input.valueAsNumber <= 4096;
    await form({ title: m('Memory Cache'),
      text: m("Keep the reading of each file in memory for this page, so that a file shown again is not read again. The least recently used go first. Set 0 to keep none."),
      parts: [label, state, clear, error], focus: input, action: tr('Apply'),
      answer: async () => {
        if (!whole()) { setText(error, m('Enter a whole number from 0 to 4096.')); return; }
        try {
          await clearing.catch(() => {});
          await save('file-cache-memory-mib', input.valueAsNumber);
          await readings();
          return true;
        }
        catch (e) { setText(error, String(e)); }
      },
    });
  } catch (e) { await inform({ title: m('Memory Cache'), paragraphs: [String(e)] }); }
}
