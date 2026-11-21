import { modal } from './dialog.js';
import { dirtyPaths, unappliedPaths } from './edits.js';
import { editing } from './editor.js';
import { preloadAllowed, preloadFile, stopPreload } from './file-reading.js';
import { currentSource, isLoading } from './open.js';
import { preloadFiles, PreloadQueue } from './preload-queue.js';
import { shell } from './shell.js';
import { previewing } from './solve.js';
import { doc } from './state.js';

/**
 * Preloading, the same in every shell that reads by file: while the viewer is idle, the files of the subcircuits below
 * the file shown are read, one at a time, into the room the cache has free (file-reading.js). Nothing is shown, and
 * nothing joins the Library. An embedded viewer does not preload.
 */
export function startPreload() {
  if (!shell.reader || shell.embed) return;
  const queue = new PreloadQueue({
    candidates: () => preloadFiles(doc),
    prepare: file => preloadFile(currentSource(), doc, file),
    cancel: () => stopPreload(),
    allowed: () => preloadAllowed() && !document.hidden && !isLoading() && !modal() && !editing && !previewing() &&
      !document.body.classList.contains('solving') && !dirtyPaths().length && !unappliedPaths().length,
  });
  for (const event of ['pointermove', 'pointerdown', 'wheel', 'keydown'])
    window.addEventListener(event, () => queue.activity(), { capture: true, passive: true });
  for (const event of ['viewer-level', 'preload-reset', 'open-finished']) window.addEventListener(event, () => queue.reset());
  document.addEventListener('visibilitychange', () => queue.reset());
  // a reading of the page's own, or a solve, stops the preloading before it starts
  const observer = new MutationObserver(() => {
    if (document.body.matches('.busy, .solving') || editing) queue.reset();
  });
  for (const node of [document.body, document.getElementById('pane-netlist')])
    observer.observe(node, { attributes: true, attributeFilter: ['class'] });
  setInterval(() => queue.tick(), 500);
}
