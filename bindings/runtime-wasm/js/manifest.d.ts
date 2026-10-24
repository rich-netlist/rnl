/** A manifest read into what a build of the core takes (manifest.js). */

/** The manifest format this module reads. */
export declare const MANIFEST_FORMAT: 1;
/** At most this many files, this many bytes in one file and this many in all. */
export declare const MANIFEST_LIMITS: Readonly<{ files: number; fileBytes: number; bytes: number }>;

/** What is wrong with a manifest or one of its files; `file` names that file when there is one. */
export declare class ManifestError extends Error {
  file: string | null;
  constructor(message: string, file?: string | null);
}

/** A manifest, format 1, such as the RNL gallery's CI writes. */
export interface Manifest {
  format: 1;
  id: string;
  title?: { en: string; zh?: string };
  description?: { en: string; zh?: string };
  author?: string;
  /** `url` links the design's directory at `commit`. */
  source?: { repo: string; commit: string; url?: string };
  license?: string;
  /** A directory of https://raw.githubusercontent.com/ at one commit, ending in `/`. */
  base: string;
  /** The netlist to open, relative to `base`. */
  entry: string;
  /**
   * Each file relative to `base`, with the SHA-256 and size of its bytes; a file with a `url` (a
   * package of the public catalog, under `.packages/`) is fetched from there instead.
   */
  files: { path: string; sha256: string; size: number; url?: string }[];
  /** Files of `files`, in the order a bare include and a package by name look for them: a build's `pool`. */
  pool?: string[];
  /** The level to show: a subcircuit's name, or "(top)" for the netlist's own. */
  level?: string;
}

/** A manifest read: every path is under `/<id>/`. */
export interface ManifestRead {
  id: string;
  title: Manifest["title"] | null;
  description: Manifest["description"] | null;
  author: string | null;
  source: Manifest["source"] | null;
  license: string | null;
  /** The path to open. */
  entry: string;
  level: string | null;
  /** The text of every file, by path. */
  files: Record<string, string>;
  /** The manifest's pool, in its order: a build's `pool`. */
  pool: string[];
}

/** Check a manifest and give it normalized; throws a ManifestError. */
export declare function checkManifest(manifest: unknown): Manifest & { level: string | null; pool: string[] };

/** Read the manifest at `url` and its files, each checked against its SHA-256. */
export declare function readManifest(
  url: string,
  options?: {
    fetch?: typeof globalThis.fetch;
    onProgress?: (done: number, total: number) => void;
    signal?: AbortSignal;
    concurrency?: number;
  },
): Promise<ManifestRead>;
