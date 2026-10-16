// The packages a build of the binding brings, given to every request the Worker answers (worker.js):
// the package files by path, and the directories among those paths whose packages are found by
// name (16.2). `build.mjs --packages DIR` fills them in from the .rnl files of DIR; without it, as
// for the web viewer, there are none.

/** @type {Record<string, string>} */
export const files = {};

/** @type {string[]} */
export const packageDirs = [];
