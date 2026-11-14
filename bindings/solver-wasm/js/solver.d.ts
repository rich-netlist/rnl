// solver.js as TypeScript declarations, written by hand; keep them with the JSDoc in solver.js.
// The Document is rnl-view's (document.d.ts), which build.mjs copies next to this file.

import type { Document } from "./document.js";

/** What to solve. Fields the solver does not know are ignored. */
export interface SolveRequest {
  /** The path of the file to solve, as the frontend names it. */
  entry: string;
  /** The texts of the workspace by path, the entry's among them. */
  files?: Record<string, string>;
  /** The directories, among the paths of `files`, whose packages are found by name. */
  package_dirs?: string[];
  /** The format of the entry, instead of guessing it. */
  dialect?: "spice" | "spectre";
  /** The format set for files, by path as in `files`. */
  formats?: Record<string, string>;
  /** Use the standard package at the root; true when not given. */
  std?: boolean;
  /** Add the standard package to draft 0.4 files. */
  include_std?: boolean;
  /** The solver settings as RNL named parameters, such as `limit="time",seconds=10`. */
  solver_config?: string;
  /** Only the level with this title. */
  level?: string;
  /** Space each level again from its record of the solver, nothing searched. */
  respace?: boolean;
}

/** The Document drawn as solved, the report and the texts written, by the paths of the request. */
export interface Solved {
  document: Document;
  report: Record<string, unknown>;
  files: Record<string, string>;
}

export type SolveAnswer = Solved | { cancelled: true } | { error: string };

/** One progress event, as `rnl solve --progress=json` writes them. */
export interface SolveEvent {
  event: string;
  [key: string]: unknown;
}

export interface Solver {
  /**
   * Solve, synchronously: run it in a Worker of its own. When `onProgress` returns `false` the
   * solve is cancelled and the answer is `{ cancelled: true }`; when it throws, the solve is
   * cancelled and `solve` throws what it threw. After a failure of the module `solve` throws, and
   * the next call has a new instance.
   */
  solve(request: SolveRequest, onProgress?: (event: SolveEvent) => boolean | void): SolveAnswer;
}

/**
 * Instantiate the solver from the module's bytes or a compiled module. `now` gives milliseconds,
 * which the time limit is measured on; `performance.now()` when not given. It has to go on.
 */
export function loadSolver(source: BufferSource | WebAssembly.Module, options?: { now?: () => number }): Promise<Solver>;
