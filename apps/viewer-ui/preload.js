import { modal } from './dialog.js';
import { dirtyPaths, unappliedPaths } from './edits.js';
import { editing } from './editor.js';
import { currentSource, isLoading } from './open.js';
import { preloadSources, PreloadQueue } from './preload-queue.js';
import { shell } from './shell.js';
import { previewing } from './solve.js';
import { current, doc } from './state.js';
import { load } from './util.js';

/** Native PATH preloading. Shells without an isolated compiler simply omit this capability. */
export function startPreload() {
  if (!shell.preload || shell.embed) return;
  const queue = new PreloadQueue({
    candidates: () => preloadSources(doc, current, currentSource(), shell),
    prepare: source => shell.preload(source),
    cancel: () => shell.cancelPreload(),
    allowed: () => load('background-preload', true) === true && load('render-cache-mib', 256) > 0 &&
      !document.hidden && !isLoading() && !modal() && !editing && !previewing() &&
      !document.body.classList.contains('solving') && !dirtyPaths().length && !unappliedPaths().length,
  });
  for (const event of ['pointermove', 'pointerdown', 'wheel', 'keydown'])
    window.addEventListener(event, () => queue.activity(), { capture: true, passive: true });
  for (const event of ['viewer-level', 'preload-reset']) window.addEventListener(event, () => queue.reset());
  document.addEventListener('visibilitychange', () => queue.reset());
  // A navigation/solve cancels before its async build; no compiler is queued behind speculation.
  const observer = new MutationObserver(() => {
    if (document.body.matches('.busy, .solving') || editing) queue.reset();
  });
  for (const node of [document.body, document.getElementById('pane-netlist')])
    observer.observe(node, { attributes: true, attributeFilter: ['class'] });
  setInterval(() => queue.tick(), 500);
}
