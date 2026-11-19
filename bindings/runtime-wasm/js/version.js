// The version of the core this package holds, with its commit, and the version of the shape of its
// readings of one file: FileResult.inputs.core and FileResult.inputs.format, for a frontend to key
// what it keeps before it loads the module. build.mjs writes them as the module it builds gives
// them; null in the sources.

/** @type {string | null} */
export const coreVersion = null;

/** @type {number | null} */
export const fileResultFormat = null;
