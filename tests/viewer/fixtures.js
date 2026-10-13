// The Documents the fake shell serves: `rnl view` of a few files of the conformance corpus, built
// natively before the tests run, and a changed copy of one of them, for reading a file again.

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
  "inverter-chain": "tests/conformance/equivalence/inverter-chain.scs",
};

/** Build the rnl command and return its path. */
export function buildRnl() {
  execFileSync("cargo", ["build", "-p", "rnl-cli"], { cwd: root, stdio: "inherit" });
  return path.join(root, "target", "debug", process.platform === "win32" ? "rnl.exe" : "rnl");
}

/** The Document of a file as `rnl view` prints it, the file named relative to `cwd`. */
function view(rnl, file, cwd) {
  return execFileSync(rnl, ["view", file], { cwd, encoding: "utf8", maxBuffer: 1 << 28 });
}

/**
 * Write the fixtures into `dir`, each as NAME.json: those of FILES, and "04-hierarchy-changed", the
 * Document of 04-hierarchy.sp after a comment line was added below its title and Vin's value
 * changed, so that every statement below the title moved down a line.
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
  fs.writeFileSync(path.join(dir, "04-hierarchy-changed.json"), view(rnl, "changed/04-hierarchy.sp", dir));
}
