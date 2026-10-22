// The Documents the fake shell serves: `rnl view` of a few files of the conformance corpus, built
// natively before the tests run; a changed copy of one of them, for reading a file again; a netlist
// with many subcircuits, for the subcircuit index; subcircuits nested four deep, for the hierarchy
// path; statements over several lines in both dialects, which the corpus has none of; characters
// outside the BMP, which JavaScript counts as two; a file of many lines; netlists of the solver's
// tests that `rnl solve` solved, for a drawing that no PLACE fixes; what `rnl preview` prints of
// a netlist, for the desktop's Solve menu; a netlist with many objects that have no position, for
// the list of what is not drawn; netlists of subcircuits that nothing calls, with a top level that
// has no objects; and a small netlist before and after an edit, with its line ends, for editing the
// text.

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

/** The repository's root. */
export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

/** The fixtures by name, and the files of the corpus they are built from. */
export const FILES = {
  "wire-labels": "tests/viewer/inputs/wire-labels.sp",
  "usage-inspector": "tests/viewer/inputs/usage-inspector.sp",
  "usage-anonymous": "tests/viewer/inputs/usage-anonymous.sp",
  "usage-virtual-roles": "tests/viewer/inputs/usage-virtual-roles.sp",
  "symbol-inspector": "tests/viewer/inputs/symbol-inspector.sp",
  "fold-sections": "tests/viewer/inputs/fold-sections.sp",
  "fold-sections-include": "tests/viewer/inputs/fold-sections.inc",
  "fold-models": "tests/viewer/inputs/fold-models.inc",
  "missing-canvas-package": "tests/viewer/inputs/missing-canvas-package.sp",
  "virtual-selection": "tests/viewer/inputs/virtual-selection.sp",
  "selection-boxes": "tests/viewer/inputs/selection-boxes.sp",
  "view-improvements": "tests/viewer/inputs/view-improvements.sp",
  "01-rc-lowpass": "tests/conformance/cases/01-rc-lowpass.sp",
  "03-five-transistor-ota": "tests/conformance/cases/03-five-transistor-ota.sp",
  "04-hierarchy": "tests/conformance/cases/04-hierarchy.sp",
  "10-constraints-and-priority": "tests/conformance/cases/10-constraints-and-priority.sp",
  "12-diagnostics": "tests/conformance/cases/12-diagnostics.sp",
  "17-line-styles": "tests/conformance/cases/17-line-styles.sp",
  "inverter-chain": "tests/conformance/equivalence/inverter-chain.scs",
  "T4-miller-conflicts": "tests/integration/solver/T4-miller-conflicts.sp",
};

/**
 * The previews of a solve, as the desktop's Solve menu shows them, each NAME-preview.json, the
 * Document of the files solved and the report, which `rnl preview` prints of the fixture NAME's
 * file: one with facts discarded, conflict groups and hints by statement.
 */
export const PREVIEWS = ["T4-miller-conflicts"];

/**
 * The fixtures of netlists that only a solver draws, and the files of tests/integration/solver they
 * are built from: `rnl solve` solves each file into the fixtures' directory, and the fixture is
 * `rnl view` of the file solved, with the records of the solver it wrote.
 */
export const SOLVED = {
  "T2-common-source-solved": "tests/integration/solver/T2-common-source.sp",
  "T3-ota5t-solved": "tests/integration/solver/T3-ota5t.sp",
};

/** The subcircuits of the "subcircuits" fixture: two under M, one under each other initial, and one under #. */
export const SUBCIRCUITS = [
  "adder", "buffer", "clock", "driver", "encoder", "filter", "gate", "hold", "inverter", "jitter", "keeper", "latch",
  "mixer", "mux", "nand", "oscillator", "pump", "quantizer", "regulator", "sampler", "timer", "unity", "vco", "window",
  "xor", "yield", "zero", "2stage",
];

/**
 * Subcircuits nested four deep, every level drawn: the top level has XK1, a bank, whose XB1 and XB2
 * are blocks, whose XC1 is a cell, whose XL1 and XL2 are leaves; and XC9, a cell at the top level too.
 */
export const NESTED = [
  "* subcircuits nested four deep, and one of them at the top level too",
  ".subckt leaf p q", "R1 p q 1k", ".ends",
  ".subckt cell p q", "XL1 p m leaf", "XL2 m q leaf", ".ends",
  ".subckt block p q", "XC1 p q cell", "R2 p q 1k", ".ends",
  ".subckt bank p q", "XB1 p m block", "XB2 m q block", ".ends",
  "XK1 a 0 bank", "XC9 a 0 cell", "R9 a 0 1k",
  "* @RNL {", "*   #XK1 @PLACE(0, 0); #XC9 @PLACE(8, 0); #R9 @PLACE(16, 0)", "* }",
  '* @RNL(scope="bank") {', "*   #XB1 @PLACE(0, 0); #XB2 @PLACE(8, 0)", "* }",
  '* @RNL(scope="block") {', "*   #XC1 @PLACE(0, 0); #R2 @PLACE(8, 0)", "* }",
  '* @RNL(scope="cell") {', "*   #XL1 @PLACE(0, 0); #XL2 @PLACE(8, 0)", "* }",
  '* @RNL(scope="leaf") {', "*   #R1 @PLACE(0, 0)", "* }",
  ".end",
];

/**
 * A small netlist, `edit`, and the same file after an edit that renames C1, `changed`: for editing
 * the text, whose Apply Changes the fake shell answers with the second, and, with the line ends of
 * Windows, for writing a file with the line ends it had.
 */
export const EDITED = {
  edit: [
    "* a netlist to edit",
    "R1 a b 1k",
    "C1 b 0 1p",
    "* @RNL { #R1 @PLACE(0, 0); #C1 @PLACE(6, 0) }",
    ".end",
  ],
  changed: [
    "* a netlist to edit",
    "R1 a b 1k",
    "C2 b 0 1p",
    "* @RNL { #R1 @PLACE(0, 0); #C2 @PLACE(6, 0) }",
    ".end",
  ],
};

/**
 * Objects without a position, and enough of them for the list of what is not drawn to load in
 * parts: a resistor, R1, is placed, as is the one in the subcircuit; a capacitor, a transistor, an
 * instance of the subcircuit and 60 resistors, Rn01 to Rn60, are not.
 */
export const UNPLACED = [
  "* objects without a position",
  ".subckt cell p q", "R1 p q 1k", ".ends",
  "R1 in a 1k", "C1 a 0 1p", "M1 d g s 0 nmos", "X1 a b cell",
  ...Array.from({ length: 60 }, (_, k) => `Rn${String(k + 1).padStart(2, "0")} n${k + 1} 0 2k`),
  "* @RNL {", "*   #R1 @PLACE(0, 0)", "* }",
  '* @RNL(scope="cell") {', "*   #R1 @PLACE(0, 0)", "* }",
  ".model nmos nmos",
  ".end",
];

/**
 * Transistors whose model card is not found, which are not drawn at all: `nch` has its card, so M1
 * has a symbol, and R1 is placed; M2 and M3 name the missing card `pch`, and Q1 the missing `qm`.
 */
export const UNDRAWN = [
  "* devices that cannot be drawn",
  ".model nch nmos",
  "R1 a b 1k",
  "M1 d g s b nch",
  "M2 d g s b pch",
  "M3 d g s b pch",
  "Q1 c b e qm",
  "* @RNL { #R1 @PLACE(0, 0) }",
  ".end",
];

/**
 * Two resistors and a capacitor for what a click and the pointer find: R2 is placed so that its box
 * overlaps R1's, and R1's name label is inside R2's box; a text is attached to R1's first terminal,
 * which is no instance or net, so that clicking it selects nothing; and a wire of net c runs
 * through C1's box, clear of its pins and of its middle.
 */
export const PICK = [
  "* hover names, labels and overlapping objects",
  "R1 a b 1k", "R2 b c 2k", "C1 c 0 1p",
  '* @RNL(type="header") { @VERSION("0.2")}',
  "* @RNL {",
  "*   #R1 @PLACE(0, 0); #R2 @PLACE(0.8, 1.5); #C1 @PLACE(10, 0);",
  '*   #R1:1 @TEXT("t1", dx=-1);',
  "*   &c @WIRE {(8.5, 1) (11.5, 1)}",
  "* }",
  ".end",
];

/**
 * Netlists whose top level has no objects, by the name of the fixture and the file it is built from,
 * each file's lines, for the subcircuits that nothing calls (apps/viewer-ui/roots.js).
 *
 * - `uncalled.sp`: what Analog Canvas writes for a cell and its testbench, both placed: the
 *   subcircuit `ota_5t`, which has the levels' index 1, and `Testbench`, 2, which instantiates it.
 * - `uncalled-spectre.scs`: the same in Spectre, as `ota` (1) and `tb` (2).
 * - `uncalled-lib.sp`: a library. `lone` (1) is not placed; `stage` (2) is, and calls `cell` (3);
 *   `ping` (4) and `pong` (5) call each other and nothing else does.
 * - `uncalled-unplaced.sp`: two subcircuits, `first` and `second`, with no position at all.
 * - `uncalled-top.sp`: a resistor at the top level without a position, and a subcircuit `spare`.
 * - `nothing.sp`: a title and no object at all.
 */
export const UNCALLED = {
  uncalled: [
    "uncalled.sp",
    [
      "* a cell and its testbench",
      ".subckt ota_5t inp inn out vdd vss",
      "M1 out inp tail vss nmos",
      "M2 x inn tail vss nmos",
      ".ends ota_5t",
      ".subckt Testbench VSS",
      "XDUT a b c d VSS ota_5t",
      "R1 a VSS 1k",
      ".ends Testbench",
      '* @RNL(scope="ota_5t") { #M1 @PLACE(0, 0); #M2 @PLACE(8, 0) }',
      '* @RNL(scope="Testbench") { #XDUT @PLACE(0, 0); #R1 @PLACE(10, 0) }',
      ".model nmos nmos",
      ".end",
    ],
  ],
  "uncalled-spectre": [
    "uncalled-spectre.scs",
    [
      "// a cell and its testbench",
      "simulator lang=spectre",
      "subckt ota (inp inn out)",
      "R1 (inp out) resistor r=1k",
      "R2 (inn out) resistor r=2k",
      "ends ota",
      "subckt tb (vss)",
      "XDUT (a b c) ota",
      "R1 (a vss) resistor r=1k",
      "ends tb",
      '// @RNL(scope="ota") { #R1 @PLACE(0, 0); #R2 @PLACE(8, 0) }',
      '// @RNL(scope="tb") { #XDUT @PLACE(0, 0); #R1 @PLACE(10, 0) }',
    ],
  ],
  "uncalled-lib": [
    "uncalled-lib.sp",
    [
      "* a library: no instance at the top level",
      ".subckt lone a b",
      "R1 a b 1k",
      ".ends lone",
      ".subckt stage a b",
      "XL a m cell",
      "R1 m b 1k",
      ".ends stage",
      ".subckt cell p q",
      "R1 p q 1k",
      ".ends cell",
      ".subckt ping a",
      "X1 a pong",
      ".ends ping",
      ".subckt pong a",
      "X1 a ping",
      ".ends pong",
      '* @RNL(scope="stage") { #XL @PLACE(0, 0); #R1 @PLACE(8, 0) }',
      ".end",
    ],
  ],
  "uncalled-unplaced": [
    "uncalled-unplaced.sp",
    [
      "* subcircuits with no position",
      ".subckt first p q",
      "R1 p q 1k",
      ".ends first",
      ".subckt second p q",
      "R1 p q 2k",
      ".ends second",
      ".end",
    ],
  ],
  "uncalled-top": [
    "uncalled-top.sp",
    ["* an object at the top level without a position", ".subckt spare p q", "R1 p q 1k", ".ends spare", "R1 a 0 1k", ".end"],
  ],
  nothing: ["nothing.sp", ["* nothing", ".end"]],
};

/** The statements over several lines: a SPICE netlist with `+` lines, and a Spectre one with `\`. */
export const CONTINUED = {
  "continued.sp": [
    "* statements over several lines",
    ".subckt cell p q",
    "R1 p q 1k",
    ".ends",
    "X1 a",
    "+ b cell",
    "R2 b 0",
    "+ r=2k $ the value",
    "* @RNL {",
    "*   #X1 @PLACE(0, 0); /* placed",
    '*   by hand */ #R2 @PLACE(6, 0) => @TEXT("a ""quoted"" text", key="note")',
    "* }",
    ".end",
  ],
  "continued.scs": [
    "// statements over several lines",
    "simulator lang=spectre",
    "subckt cell (p q)",
    "R1 (p q) resistor r=1k",
    "ends cell",
    "X1 (a b) \\",
    "    cell",
    "R2 (b 0) resistor \\",
    "    r=2k",
  ],
};

/** An emoji and a CJK character of the extension planes, both outside the BMP. */
export const EMOJI = String.fromCodePoint(0x1f600);
export const CJK = String.fromCodePoint(0x20000);

/**
 * Characters outside the BMP, an emoji and a CJK character of the extension planes, which the
 * Document counts as one character each and JavaScript as two UTF-16 units: before references of
 * records, in a name, and before a statement that fails, R9's, as the netlist has no R9.
 */
export const UNICODE = [
  "* characters outside the BMP",
  "R1 a 0 1k",
  `R${CJK}2 a b 2k`,
  `* @RNL { /* ${EMOJI} */ #R1 @PLACE(0, 0) }`,
  `* @RNL { /* ${EMOJI}${CJK} ${EMOJI} */ #"R${CJK}2" @PLACE(6, 0); /* ${CJK} */ #R9 @PLACE(0, 0) }`,
  ".end",
];

/** The comment lines of the "long" fixture: more than one call takes as its arguments in Chrome. */
export const LONG = 200000;

/**
 * Build the rnl command and return its path, in the target directory that cargo builds into, which
 * CARGO_TARGET_DIR or cargo's configuration may move.
 */
export function buildRnl() {
  if (process.env.RNL_BIN && fs.existsSync(process.env.RNL_BIN)) return process.env.RNL_BIN;
  execFileSync("cargo", ["build", "-p", "rnl-cli"], { cwd: root, stdio: "inherit" });
  const metadata = execFileSync("cargo", ["metadata", "--format-version", "1", "--no-deps"], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 1 << 26,
  });
  const target = JSON.parse(metadata).target_directory;
  return path.join(target, "debug", process.platform === "win32" ? "rnl.exe" : "rnl");
}

/** The Document of a file as `rnl view` prints it, the file named relative to `cwd`. */
function view(rnl, file, cwd) {
  return execFileSync(rnl, ["view", file], { cwd, encoding: "utf8", maxBuffer: 1 << 28 });
}

/**
 * Write the fixtures into `dir`, each as NAME.json: those of FILES; "04-hierarchy-changed", the
 * Document of 04-hierarchy.sp after a comment line was added below its title and Vin's value
 * changed, so that every statement below the title moved down a line, with the same file names;
 * "subcircuits", a netlist that instantiates each of SUBCIRCUITS once at the top level; "nested",
 * the netlist of NESTED, and "nested-changed", the same file after XB2 was named XB3;
 * "continued" and "continued-spectre", the netlists of CONTINUED; "edit", "edit-changed", the
 * same file after the edit, and "edit-crlf", the same with CRLF line ends, from EDITED; "unicode",
 * the netlist of UNICODE; the netlists of UNCALLED; "unplaced", the netlist of UNPLACED; "undrawn", the netlist of UNDRAWN; "pick", the netlist of PICK; "long", a title, LONG lines of a bare
 * `*`, R1, a record that places it and `.end`; and those of SOLVED, from the files that `rnl solve`
 * writes under `solved` in `dir`; and those of PREVIEWS.
 */
export function writeFixtures(rnl, dir) {
  const semantic = ['* large semantic file', 'R1 a 0 1k', '* @RNL { #R1 is here @PLACE(0, 0) }',
    ...Array(2600).fill('* padding'), '* @RNL { (0, 4) descriptive text @TEXT("hello") }', '.end'];
  fs.writeFileSync(path.join(dir, 'semantics-virtual.sp'), semantic.join('\n'));
  fs.writeFileSync(path.join(dir, 'semantics-virtual.json'), view(rnl, 'semantics-virtual.sp', dir));
  // No standard package stylesheet: exercise the renderer's omitted-width default itself.
  fs.writeFileSync(path.join(dir, "view-hairlines.json"), execFileSync(rnl,
    ["view", "tests/viewer/inputs/default-hairlines.sp", "--no-std"], { cwd: root, encoding: "utf8" }));
  for (const [name, file] of Object.entries(FILES)) {
    fs.writeFileSync(path.join(dir, `${name}.json`), view(rnl, file, root));
  }

  const changed = path.join(dir, "changed");
  fs.mkdirSync(changed, { recursive: true });
  const cases = path.join(root, "tests/conformance/cases");
  fs.copyFileSync(path.join(cases, "04-hierarchy-lib.inc"), path.join(changed, "04-hierarchy-lib.inc"));
  const text = fs.readFileSync(path.join(cases, "04-hierarchy.sp"), "utf8");
  const edited = text.replace(/^(.*\r?\n)/, "$1* changed after it was opened\n").replace("Vin a 0 0.9", "Vin a 0 1.2");
  if (!edited.includes("Vin a 0 1.2")) throw new Error("04-hierarchy.sp no longer has the line the changed copy edits");
  fs.writeFileSync(path.join(changed, "04-hierarchy.sp"), edited);
  fs.writeFileSync(path.join(dir, "04-hierarchy-changed.json"), view(rnl, "04-hierarchy.sp", changed));

  // Real core spans/diagnostics in files larger than a DOM window, including an included file.
  const virtual = path.join(dir, "virtual-data");
  fs.mkdirSync(virtual);
  const pad = (n, label) => Array.from({ length: n }, (_, i) => `* ${label} ${i + 1}`).join("\n") + "\n";
  const virtualText = text.replace(/^(.*\r?\n)/, "$1" + pad(1300, "prefix")).replace(/^\.end\s*$/m,
    '* @RNL { /* a long comment\n' + pad(180, "inside RNL comment") + '* end */ }\n' +
    'Runplaced spare 0 11k\n* @RNL { #missing @PLACE(1, 1) }\n' + pad(1300, "suffix") + ".end\n");
  fs.writeFileSync(path.join(virtual, "virtual.sp"), virtualText);
  const lib = fs.readFileSync(path.join(cases, "04-hierarchy-lib.inc"), "utf8");
  fs.writeFileSync(path.join(virtual, "04-hierarchy-lib.inc"), "* long include\n" + pad(1100, "include") + lib);
  fs.writeFileSync(path.join(dir, "virtual.json"), view(rnl, "virtual.sp", virtual));

  const lines = ["* a subcircuit for each initial"];
  for (const name of SUBCIRCUITS) lines.push(`.subckt ${name} p q`, "R1 p q 1k", ".ends");
  SUBCIRCUITS.forEach((name, k) => lines.push(`X${k + 1} n${k + 1} 0 ${name}`));
  lines.push(".end", "");
  fs.writeFileSync(path.join(dir, "subcircuits.sp"), lines.join("\n"));
  fs.writeFileSync(path.join(dir, "subcircuits.json"), view(rnl, "subcircuits.sp", dir));

  const nested = NESTED.concat("").join("\n");
  fs.writeFileSync(path.join(dir, "nested.sp"), nested);
  fs.writeFileSync(path.join(dir, "nested.json"), view(rnl, "nested.sp", dir));
  fs.writeFileSync(path.join(changed, "nested.sp"), nested.replace(/XB2/g, "XB3"));
  fs.writeFileSync(path.join(dir, "nested-changed.json"), view(rnl, "nested.sp", changed));

  for (const [file, text] of Object.entries(CONTINUED)) {
    fs.writeFileSync(path.join(dir, file), text.concat("").join("\n"));
    const name = file.endsWith(".scs") ? "continued-spectre" : "continued";
    fs.writeFileSync(path.join(dir, `${name}.json`), view(rnl, file, dir));
  }

  // the edited file has the same name, in a directory of its own, as 04-hierarchy-changed has
  fs.writeFileSync(path.join(dir, "edit.sp"), EDITED.edit.concat("").join("\n"));
  fs.writeFileSync(path.join(dir, "edit.json"), view(rnl, "edit.sp", dir));
  fs.writeFileSync(path.join(changed, "edit.sp"), EDITED.changed.concat("").join("\n"));
  fs.writeFileSync(path.join(dir, "edit-changed.json"), view(rnl, "edit.sp", changed));
  fs.writeFileSync(path.join(dir, "edit-crlf.sp"), EDITED.edit.concat("").join("\r\n"));
  fs.writeFileSync(path.join(dir, "edit-crlf.json"), view(rnl, "edit-crlf.sp", dir));

  fs.writeFileSync(path.join(dir, "unicode.sp"), UNICODE.concat("").join("\n"));
  fs.writeFileSync(path.join(dir, "unicode.json"), view(rnl, "unicode.sp", dir));

  for (const [name, [file, lines]] of Object.entries(UNCALLED)) {
    fs.writeFileSync(path.join(dir, file), lines.concat("").join("\n"));
    fs.writeFileSync(path.join(dir, `${name}.json`), view(rnl, file, dir));
  }

  fs.writeFileSync(path.join(dir, "unplaced.sp"), UNPLACED.concat("").join("\n"));
  fs.writeFileSync(path.join(dir, "unplaced.json"), view(rnl, "unplaced.sp", dir));
  fs.writeFileSync(path.join(dir, "undrawn.sp"), UNDRAWN.concat("").join("\n"));
  fs.writeFileSync(path.join(dir, "undrawn.json"), view(rnl, "undrawn.sp", dir));
  fs.writeFileSync(path.join(dir, "pick.sp"), PICK.concat("").join("\n"));
  fs.writeFileSync(path.join(dir, "pick.json"), view(rnl, "pick.sp", dir));

  const long = ["* a long file"].concat(new Array(LONG).fill("*"), "R1 a 0 1k", "* @RNL { #R1 @PLACE(0, 0) }", ".end", "");
  fs.writeFileSync(path.join(dir, "long.sp"), long.join("\n"));
  fs.writeFileSync(path.join(dir, "long.json"), view(rnl, "long.sp", dir));

  for (const name of PREVIEWS) {
    const out = execFileSync(rnl, ["preview", FILES[name], "--solver-options", "time_limit=none"], {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 1 << 28,
    });
    fs.writeFileSync(path.join(dir, `${name}-preview.json`), out);
  }

  const solved = path.join(dir, "solved");
  for (const [name, file] of Object.entries(SOLVED)) {
    // what the solver prints of each level is not needed; it fails, and the setup with it, on an error
    execFileSync(rnl, ["solve", file, "-o", solved], { cwd: root, encoding: "utf8", maxBuffer: 1 << 26 });
    fs.writeFileSync(path.join(dir, `${name}.json`), view(rnl, path.basename(file), solved));
  }
}
