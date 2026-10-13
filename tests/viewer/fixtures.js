// The Documents the fake shell serves: `rnl view` of a few files of the conformance corpus, built
// natively before the tests run; a changed copy of one of them, for reading a file again; a netlist
// with many subcircuits, for the subcircuit index; and statements over several lines in both
// dialects, which the corpus has none of.

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
};

/** The subcircuits of the "subcircuits" fixture: two under M, one under each other initial, and one under #. */
export const SUBCIRCUITS = [
  "adder", "buffer", "clock", "driver", "encoder", "filter", "gate", "hold", "inverter", "jitter", "keeper", "latch",
  "mixer", "mux", "nand", "oscillator", "pump", "quantizer", "regulator", "sampler", "timer", "unity", "vco", "window",
  "xor", "yield", "zero", "2stage",
];

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
 * "subcircuits", a netlist that instantiates each of SUBCIRCUITS once at the top level; and
 * "continued" and "continued-spectre", the netlists of CONTINUED.
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

  for (const [file, text] of Object.entries(CONTINUED)) {
    fs.writeFileSync(path.join(dir, file), text.concat("").join("\n"));
    const name = file.endsWith(".scs") ? "continued-spectre" : "continued";
    fs.writeFileSync(path.join(dir, `${name}.json`), view(rnl, file, dir));
  }
}
