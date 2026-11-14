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
  /** The whole file's time in seconds, as `rnl solve --timeall` takes it: what the levels of a time limit share. */
  timeall?: number;
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
  /**
   * The table of the solver's settings and objectives, as `rnl solver-defaults --json` prints it:
   * what a settings page is drawn from. Fields are only ever added.
   */
  settings(): SettingsTable;
  /**
   * The settings of each level of a request, as `rnl solve --check --json` prints them, nothing
   * solved. Without `entry`, its `solver_config` alone is checked.
   */
  check(request: Partial<SolveRequest>): Checked | { error: string };
}

/** One setting in effect: its value, where it comes from and where it is written. */
export interface CheckedSetting {
  value: number | string | boolean | (number | string | boolean)[];
  /** "default", "package:NAME", "header", "level" or "command-line". */
  from: string;
  /** `file:line`, the file named as the core's diagnostics name it. */
  at?: string;
}

/** A problem of one setting, or of the settings together when `key` is null. */
export interface CheckedProblem {
  key: string | null;
  message: string;
  /** For a value of the file that the solve leaves out: that value, where it comes from and where it is written. */
  refused?: CheckedSetting;
}

export interface Checked {
  ok: boolean;
  levels: {
    title: string | null;
    settings: Record<string, CheckedSetting>;
    warnings: CheckedProblem[];
    errors: CheckedProblem[];
  }[];
}

/** One setting of the table. A field that does not apply to it is left out. */
export interface SettingRow {
  key: string;
  group: string;
  /** "integer", "number", "number-or-false", "boolean", "string", "choice" or "objectives"; a kind not known is text. */
  type: string;
  choices?: { value: string; what: string }[];
  min?: number;
  min_exclusive?: number;
  max?: number;
  /** Its value when nothing gives it, for a 0.4 level; left out when it has none. */
  default?: number | string | boolean;
  /** Whether a file may give it in SOLVER-CONFIG. */
  in_file: boolean;
  what: string;
  false_what?: string;
  /** Where the solver uses it, by the values of other settings. */
  applies?: Record<string, string[]>;
  shown_when?: Record<string, string[]>;
  required_when?: Record<string, string[]>;
}

export interface SettingsTable {
  version: number;
  groups: { name: string; what: string }[];
  settings: SettingRow[];
  objectives: { name: string; what: string; implemented: boolean }[];
  default_objectives: { items: { name: string; weight: number }[]; tolerance: number }[];
  stages: { name: string; what: string }[];
}

/**
 * Instantiate the solver from the module's bytes or a compiled module. `now` gives milliseconds,
 * which the time limit is measured on; `performance.now()` when not given. It has to go on.
 */
export function loadSolver(source: BufferSource | WebAssembly.Module, options?: { now?: () => number }): Promise<Solver>;
