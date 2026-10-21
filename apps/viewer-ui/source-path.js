// Plain path text: keep the filename visible while the directory shrinks in the middle.
export function setPathText(el, path) {
  if (el.dataset.path === path) return;
  el.dataset.path = path;
  el.title = path;
  const split = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\')) + 1;
  const directory = document.createElement('span'), name = document.createElement('span');
  directory.className = 'path-directory'; name.className = 'path-name';
  directory.textContent = path.slice(0, split); name.textContent = path.slice(split);
  el.replaceChildren(directory, name);
}

/** SourceFile paths are relative to Document.path, except external and bundled files. */
export function sourcePath(document, file) {
  const path = file.path.replaceAll('\\', '/');
  if (/^(?:[A-Za-z]:\/|\/|<)/.test(path)) return path;
  const entry = document.path.replaceAll('\\', '/');
  const parts = (entry.slice(0, entry.lastIndexOf('/') + 1) + path).split('/');
  const out = [];
  for (const part of parts) {
    if (part === '.') continue;
    if (part === '..' && out.length && out.at(-1) !== '..') out.pop();
    else out.push(part);
  }
  return out.join('/');
}
