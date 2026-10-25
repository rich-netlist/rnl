/** A manifest read into what a build of the core takes, or written (manifest.js). */

/** The manifest format this module reads and writes. */
export declare const MANIFEST_FORMAT: 1;
/** The name of a project's manifest in its archive. */
export declare const PROJECT_MANIFEST: "rnl-project.json";
/**
 * At most `files` files, `fileBytes` bytes in one file and `bytes` in all for the files a manifest
 * fetches; at most `projectFiles` files and `projectBytes` bytes in a project's archive.
 */
export declare const MANIFEST_LIMITS: Readonly<{ files: number; fileBytes: number; bytes: number; projectFiles: number; projectBytes: number }>;

/** What is wrong with a manifest or one of its files; `file` names that file when there is one. */
export declare class ManifestError extends Error {
  file: string | null;
  constructor(message: string, file?: string | null);
}

/**
 * A manifest, format 1: a design of the gallery (its files under `base`), a project's
 * rnl-project.json (its files in the archive with it), or a manifest of links (each file's `url`).
 */
export interface Manifest {
  format: 1;
  /** Letters, digits, '-', '_' and '.'; the entry's name without its extension when not given. */
  id?: string;
  title?: { en: string; zh?: string };
  description?: { en: string; zh?: string };
  /** Its authors in their order, a url (https) each may have; one name alone is read as a list of it. */
  author?: { name: string; url?: string }[] | string;
  /** `url` links the design's directory at `commit`. */
  source?: { repo: string; commit: string; url?: string };
  license?: string;
  /** The http(s) address of the directory the files without a `url` are under, ending in `/`. */
  base?: string;
  /** The netlist to open, a path of `files`. */
  entry: string;
  /**
   * Each file by its relative path, with the SHA-256 and size of its bytes. A file with a `url` is
   * fetched from there (in an archive, the archive's copy is read and `url` says where it was from).
   */
  files: { path: string; sha256: string; size: number; url?: string }[];
  /** Files of `files`, in the order a bare include and a package by name look for them: a build's `pool`. */
  pool?: string[];
  /** The parse format chosen for some files, by path: a build's `formats`. */
  formats?: Record<string, string>;
  /** The level to show: a subcircuit's name, or "(top)" for the netlist's own. */
  level?: string;
  /** The desktop's PATH directories a project copied, in their order: a record; `pool` has their files. */
  paths?: { path: string; subfolders?: boolean }[];
  /** The Library's settings. */
  settings?: { autoInclude?: boolean };
  /** Display settings proposed to the viewer. */
  display?: DisplaySettings;
}

/** Display settings a manifest proposes; a viewer asks before it applies them, for its page or window only. */
export interface DisplaySettings {
  /** The view of `level`: fitted to the window, or centred on a point of the drawing at a scale. */
  view?: { fit: true } | { center: [number, number]; scale: number };
  background?: "light" | "dark";
  /** Colours by the viewer's names (component, net, rnl, keyword, definition, local, string, parameter, free, text, background, highlight, subcircuit). */
  colors?: Record<string, string>;
  usages?: boolean;
  currents?: boolean;
  grid?: "none" | "dots" | "lines";
}

/** A manifest read: every path is under `root`, `/<id>/` by default. */
export interface ManifestRead {
  id: string;
  title: Manifest["title"] | null;
  description: Manifest["description"] | null;
  author: Manifest["author"] | null;
  source: Manifest["source"] | null;
  license: string | null;
  /** The path to open. */
  entry: string;
  level: string | null;
  /** The text of every file, by path. */
  files: Record<string, string>;
  /** The bytes of every file, by path. */
  bytes: Record<string, Uint8Array>;
  /** The address of every file, by path, or null for a file of an archive without one. */
  urls: Record<string, string | null>;
  /** The manifest's pool, in its order: a build's `pool`. */
  pool: string[];
  /** The formats chosen, by path: a build's `formats`. */
  formats: Record<string, string>;
  paths: { path: string; subfolders: boolean }[];
  settings: { autoInclude?: boolean };
  /** The display settings of the shapes this module reads; empty when none. */
  display: DisplaySettings;
  /** The names of the display settings left out, of another shape or unknown. */
  displayIgnored: string[];
}

interface ReadOptions {
  fetch?: typeof globalThis.fetch;
  onProgress?: (done: number, total: number) => void;
  signal?: AbortSignal;
  concurrency?: number;
  /** The workspace directory of the files, ending in "/"; `/<id>/` by default. */
  root?: string;
  /** Some of MANIFEST_LIMITS replaced, for a caller that holds less (a web page opens smaller projects). */
  limits?: Partial<typeof MANIFEST_LIMITS>;
}

/** Check a manifest and give it normalized; `archive` when it was read from a project's archive. Throws a ManifestError. */
export declare function checkManifest(manifest: unknown, options?: { archive?: boolean; limits?: Partial<typeof MANIFEST_LIMITS> }): Manifest & {
  id: string; level: string | null; pool: string[]; formats: Record<string, string>; paths: { path: string; subfolders: boolean }[];
  settings: { autoInclude?: boolean };
};

/** Read the files of a manifest already parsed: from `readFile` (a project's archive), else fetched. */
export declare function loadManifest(
  json: unknown,
  options?: ReadOptions & { readFile?: (path: string) => Promise<Uint8Array | null> | Uint8Array | null },
): Promise<ManifestRead>;

/** Read the manifest at `url` and its files, each checked against its SHA-256. */
export declare function readManifest(url: string, options?: ReadOptions): Promise<ManifestRead>;

/** Write the manifest of `files`, with their SHA-256 and sizes; checked as a reader checks it. */
export declare function writeManifest(options: {
  files: { path: string; bytes: Uint8Array; url?: string }[];
  entry: string;
  pool?: string[];
  formats?: Record<string, string>;
  level?: string | null;
  paths?: { path: string; subfolders?: boolean }[];
  settings?: { autoInclude?: boolean };
  display?: DisplaySettings;
  /** The files go in the archive with the manifest (a project). */
  archive?: boolean;
  id?: string;
  title?: Manifest["title"];
  description?: Manifest["description"];
  author?: Manifest["author"];
  source?: Manifest["source"];
  license?: string;
  base?: string;
}): Promise<Manifest>;
