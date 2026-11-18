// Asking to solve a netlist that is opened with components not drawn: when a file or a project is
// opened, not read again or rebuilt after an edit, and a circuit of it has components with no position
// (the Not drawn list, unplaced.js), a dialog says how many and offers Solve or Not now. Solve runs the
// solve of Solve › Solve (F9), or of Solve All Circuits when they are in other circuits than the one
// shown, with the user's settings, as a preview that changes no file. Not now is remembered for that
// file until the page is closed. Only where a solver is (`shell.solve`), never in an embedded drawing,
// nor for a document with errors. Solve › Ask to Solve on Open, kept in the settings, turns it off.

import { setCheck } from "./commands.js";
import { decide, modal } from "./dialog.js";
import { m, tr } from "./i18n.js";
import { shell } from "./shell.js";
import { solve } from "./solve.js";
import { current, doc } from "./state.js";
import { load, save } from "./util.js";

const KEY = "solve-on-open";
const declined = new Set(); // the files Not now was answered for, by path or name
let asking = false;

/** Whether a file opened with components not drawn asks to be solved: Solve › Ask to Solve on Open. */
export const asksToSolve = () => load(KEY, true) !== false;

/** Show the check of Solve › Ask to Solve on Open as kept. */
export function startSolveOnOpen() {
  setCheck("toggle-solve-on-open", asksToSolve());
}

/** Solve › Ask to Solve on Open: turn it on or off, and keep it. */
export function toggleSolveOnOpen() {
  const on = !asksToSolve();
  save(KEY, on);
  setCheck("toggle-solve-on-open", on);
}

/**
 * The components of a Document that are not drawn, as the Not drawn tab counts them (the instances
 * among a level's unplaced objects): `components`, how many in all, and `levels`, the indexes of the
 * levels that have any.
 */
export function notDrawn(d) {
  let components = 0;
  const levels = [];
  d.levels.forEach((level, k) => {
    const n = new Set((level.unplaced || []).flatMap((u) => u.objects).filter((o) => d.objects[o]?.kind === "instance")).size;
    if (n) {
      components += n;
      levels.push(k);
    }
  });
  return { components, levels };
}

/** The text of the question: how many components, in how many circuits. */
function question({ components, levels }) {
  if (levels.length > 1) return m`${components} components in ${levels.length} circuits have no position yet, so they are not drawn.`;
  if (components === 1) return m("1 component has no position yet, so it is not drawn.");
  return m`${components} components have no position yet, so they are not drawn.`;
}

/**
 * After `d`, built from `source`, was opened and shown: ask whether to solve it, if it has components
 * not drawn and nothing says not to. `signal`, the opening's, cancels the question when another
 * document is opened meanwhile.
 */
export async function askToSolve(d, source, signal) {
  if (!shell?.solve || shell.embed || asking || modal() || !asksToSolve() || d !== doc || signal?.aborted) return;
  const key = source?.path || source?.name;
  if (key && declined.has(key)) return;
  if (d.diagnostics?.some((x) => x.severity === "error")) return;
  const found = notDrawn(d);
  if (!found.components) return;
  asking = true;
  let answer;
  try {
    const action = tr("Solve");
    answer = await decide({
      title: m("Solve the Netlist?"),
      paragraphs: [question(found), m("Solve places them with your solver settings, as a preview: the files are not changed.")],
      action,
      cancel: tr("Not now"),
      signal,
    });
    answer = answer === action;
  } finally {
    asking = false;
  }
  if (d !== doc || signal?.aborted) return;
  if (!answer) {
    if (key) declined.add(key);
    return;
  }
  // the solve of the circuit shown, when it is the one with components not drawn; else of every circuit
  await solve(found.levels.length === 1 && found.levels[0] === current ? {} : { all: true });
}
