// The Documents the fake shell serves: `rnl view` of a few files of the conformance corpus, built
// natively before the tests run; a changed copy of one of them, for reading a file again; a netlist
// with many subcircuits, for the subcircuit index; subcircuits nested four deep, for the hierarchy
// path; statements over several lines in both dialects, which the corpus has none of; characters
// outside the BMP, which JavaScript counts as two; a file of many lines; netlists of the solver's
// tests that `rnl solve` solved, for a drawing that no PLACE fixes; what `rnl preview` prints of
// a netlist, for the desktop's Solve menu; and a small netlist before and after an edit, with
// its line ends, for editing the text.

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

/** The repository's root. */
export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

/** The fixtures by name, and the files of the corpus they are built from. */
export const FILES = {
  "01-rc-lowpass": "tests/conformance/cases/01-rc-lowpass.sp",
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
 * the netlist of UNICODE; "long", a title, LONG lines of a bare `*`, R1, a record that places it and
 * `.end`; and those of SOLVED, from the files that `rnl solve` writes under `solved` in `dir`; and those of
 * PREVIEWS.
 */
export function writeFixtures(rnl, dir) {
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
