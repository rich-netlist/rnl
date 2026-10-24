/** A gallery manifest read into what a build of the core takes (manifest.js). */

/** The manifest format this module reads. */
export declare const MANIFEST_FORMAT: 1;
/** At most this many files, and this many bytes in all. */
export declare const MANIFEST_LIMITS: Readonly<{ files: number; bytes: number }>;

/** What is wrong with a manifest or one of its files; `file` names that file when there is one. */
export declare class ManifestError extends Error {
  file: string | null;
  constructor(message: string, file?: string | null);
}

/** A gallery manifest, format 1, as the gallery's CI writes it. */
export interface Manifest {
  format: 1;
  id: string;
  title?: { en: string; zh?: string };
  author?: string;
  source?: { repo: string; commit: string; url: string };
  license?: string;
  /** A directory of https://raw.githubusercontent.com/ at one commit, ending in `/`. */
  base: string;
  /** The netlist to open, relative to `base`. */
  entry: string;
  /** Directories relative to `base` that a bare include searches, in order. */
  paths?: string[];
  files: { path: string; sha256: string; size: number }[];
  packages?: ({ path: string } | { name: string; version: string; url: string; sha256: string })[];
  /** The level to show: a subcircuit's name, or "(top)". */
  level?: string;
}

/** A manifest read: every path is under `/<id>/`. */
export interface ManifestRead {
  id: string;
  title: Manifest["title"] | null;
  author: string | null;
  source: Manifest["source"] | null;
  license: string | null;
  /** The path to open. */
  entry: string;
  level: string | null;
  /** The text of every file and package, by path. */
  files: Record<string, string>;
  /** The files of each search path in order, then the packages: a build's `pool`. */
  pool: string[];
  /** The paths of the packages, in the manifest's order. */
  packages: string[];
}

/** Check a manifest and give it normalized; throws a ManifestError. */
export declare function checkManifest(manifest: unknown): Manifest & { level: string | null; paths: string[] };

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
