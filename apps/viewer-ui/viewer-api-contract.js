// Public capability discovery also lets callers identify a pre-API complete Viewer promptly.
export const API_FEATURES = ["requestId", "packages", "opener", "manifest"];
// what only the canvas-only viewer of an iframe takes, as messages: it is not in viewer-api.json, which is the complete Viewer's
export const EMBED_FEATURES = ["usages", "currents"];
export const MAX_OPEN_BYTES = 64 * 1024 * 1024;
