// Missing named dependencies are searched after display, without delaying a build or editing.
import { append, m, setText } from './i18n.js';
import { decide, modal } from './dialog.js';
import { editing } from './editor.js';
import { currentSource, isLoading } from './open.js';
import { includePublicPackages, isLibraryChanging } from './pool.js';
import { matchingPackage, missingPackage } from './public-packages.js';
import { shell } from './shell.js';
import { doc } from './state.js';

const offered = new WeakMap(); // a dismissal is remembered for this opened source's session

export async function offerMissingPackages(built, source, signal) {
  if (shell.platform !== 'web' || shell.embed || !shell.publicPackages || !shell.includePackage) return;
  const requested = new Map();
  const seen = offered.get(source) || new Set();
  for (const diagnostic of built.diagnostics) {
    const missing = missingPackage(diagnostic);
    if (!missing) continue;
    const key = `${missing.name} ${missing.version || ''}`;
    if (!seen.has(key)) requested.set(key, missing);
  }
  if (!requested.size) return;
  const valid = () => !signal.aborted && built === doc && source === currentSource();
  try {
    // The shared catalog request has a total ten-second deadline and aborts its network work.
    const catalog = await shell.publicPackages();
    if (!valid()) return;
    const matches = [...requested].map(([key, request]) => ({ key, request,
      pkg: matchingPackage(catalog, request.name, request.version) })).filter(item => item.pkg);
    if (!matches.length) return;
    // Do not stack dialogs or interrupt typing. A newer open/apply aborts this offer instead.
    while (valid() && (isLoading() || isLibraryChanging() || editing || modal() || document.hidden)) {
      await new Promise(resolve => setTimeout(resolve, 250));
    }
    if (!valid()) return;
    const list = document.createElement('ul');
    const packages = new Map();
    for (const { request, pkg } of matches) {
      const key = `${pkg.name} ${pkg.version}`;
      if (packages.has(key)) continue;
      packages.set(key, pkg);
      const row = document.createElement('li');
      setText(row, request.version && request.version !== pkg.version
        ? m`${key} (requested ${request.version})` : key);
      append(list, row);
    }
    const choice = await decide({ title: m('Public Packages Found'), signal,
      paragraphs: [m('The following explicitly referenced packages are available in the public library:'), list,
        m('Add them to Library and reload the current circuit? Pending edits will be kept.')],
      action: 'Add to Library' });
    if (!valid()) return;
    for (const { key } of matches) seen.add(key);
    offered.set(source, seen);
    if (choice === 'Add to Library' &&
        await includePublicPackages([...packages.values()], valid, signal) !== true) {
      // Busy/cancelled/failed approvals may be offered again; a successful rebuild or an
      // explicit dismissal stays remembered, preventing repeated offers for invalid packages.
      for (const { key } of matches) seen.delete(key);
    }
  } catch {
    // Offline/timeout failures leave the original diagnostics and manual Include action intact.
  }
}
