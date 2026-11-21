import { append, m, setText, tr } from "./i18n.js";
import { form, inform, modal } from './dialog.js';
import { memoryCache as readings, readsByFile } from './file-reading.js';
import { isLoading } from './open.js';
import { shell } from './shell.js';
import { load, save } from './util.js';

const MIB = 1024 * 1024;
const mib = (bytes) => (bytes / MIB).toFixed(1);

/**
 * File > Memory Cache, the same in every shell that reads the file shown alone (file-reading.js): whether the readings
 * of each file are kept in memory for this window or page, their limit (0 keeps none), whether the files below the
 * file shown are read while the viewer is idle, how many readings it holds, their size and the hits and misses, and
 * Clear Cache, which empties it, the document shown staying as it is.
 */
export async function memoryCache() {
  if (!readsByFile() || shell.embed || isLoading() || modal()) return;
  try {
    const stats = await readings();
    if (isLoading() || modal()) return;
    const check = (text, checked) => {
      const label = document.createElement('label');
      label.className = 'dialog-line';
      const input = document.createElement('input');
      input.type = 'checkbox'; input.checked = checked;
      append(label, input, text);
      return { label, input };
    };
    const enabled = check(m('Keep the readings in memory'), load('file-cache-enabled', true) === true);
    const label = document.createElement('label');
    label.className = 'dialog-line';
    setText(label, m('Limit in memory (MiB)'));
    const input = document.createElement('input');
    input.type = 'number'; input.min = '0'; input.max = '4096'; input.step = '1';
    input.value = String(load('file-cache-memory-mib', 256));
    append(label, input);
    const preload = check(m('Preload the files below the file shown while idle'), load('file-preload', true) === true);
    // the limit and preloading are for readings kept: with none kept, they have nothing to do
    const follow = () => { input.disabled = !enabled.input.checked; preload.input.disabled = !enabled.input.checked; };
    enabled.input.addEventListener('change', follow);
    follow();
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
      text: m("Keep the reading of each file in memory while the viewer is open, so that a file shown again is not read again. The least recently used go first. Preloading reads the files of the subcircuits below the file shown while the viewer is idle, into the room that is free."),
      parts: [enabled.label, label, preload.label, state, clear, error], focus: enabled.input, action: tr('Apply'),
      answer: async () => {
        if (enabled.input.checked && !whole()) { setText(error, m('Enter a whole number from 0 to 4096.')); return; }
        try {
          await clearing.catch(() => {});
          await save('file-cache-enabled', enabled.input.checked);
          if (enabled.input.checked) await save('file-cache-memory-mib', input.valueAsNumber);
          await save('file-preload', preload.input.checked);
          await readings();
          window.dispatchEvent(new Event('preload-reset'));
          return true;
        }
        catch (e) { setText(error, String(e)); }
      },
    });
  } catch (e) { await inform({ title: m('Memory Cache'), paragraphs: [String(e)] }); }
}
