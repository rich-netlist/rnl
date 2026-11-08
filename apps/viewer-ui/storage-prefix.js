// What the page puts before the name of every value it keeps in the browser's storage (util.js `load` and `save`), so that two
// builds of the page on one origin, such as a frozen older Viewer beside the current one, keep their settings apart. Empty
// here; the web build writes the prefix named by RNL_STORAGE_PREFIX over this file in its output (apps/web/build.mjs). It does
// not touch the desktop's settings file, which has the desktop viewer's own.
export const STORAGE_PREFIX = "";
