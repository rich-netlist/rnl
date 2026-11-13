// Solve through the WebAssembly module with js/solver.js, as a Worker would: the answer, the
// progress events, respacing, and a bad request.
//
//   node bindings/solver-wasm/tests/solve.mjs PATH/TO/rnl_solver_wasm.wasm [PATH/TO/rnl]
//
// With the native CLI as the second argument, it also checks that the module writes the files
// `rnl solve` writes, byte for byte, under a limit of work.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { loadSolver } from "../js/solver.js";

const [wasm, rnl] = process.argv.slice(2);
assert.ok(wasm, "usage: node solve.mjs RNL_SOLVER_WASM [RNL]");
const solver = await loadSolver(readFileSync(wasm), { now: () => 0 });

const source = `* wasm
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(layout="lines") }
R1 in a 1k
R2 a b 1k
R3 b 0 1k
* @RNL {
* $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal=1,direction="left"); (2,0) @PIN(terminal=2,direction="right"); (-1,-1) (1,-1) (1,1) (-1,1) (-1,-1) @LINE };
* {#R1 #R2 #R3} @SET-SYMBOL $S;
* &a @ROUTE-NET; &b @ROUTE-NET;
* }
.end
`;
const request = { entry: "/w/main.sp", files: { "/w/main.sp": source }, std: false, solver_config: 'limit="seed"' };

const events = [];
const solved = solver.solve(request, (e) => events.push(e));
assert.ok(solved.document && solved.report, JSON.stringify(solved).slice(0, 400));
assert.equal(solved.report.mode, "solve");
const written = solved.files["/w/main.sp"];
assert.match(written, /@RNL\(type="generated"/);
assert.equal(events[0].event, "run");
assert.ok(events.some((e) => e.event === "level-done"), JSON.stringify(events));

// the record just written, spaced again
const respaced = solver.solve(
  { ...request, files: { "/w/main.sp": written }, respace: true, solver_config: 'spacing_x=4,limit="work",work=100000000000' },
  () => {},
);
assert.equal(respaced.report.mode, "respace", JSON.stringify(respaced).slice(0, 400));
assert.notEqual(respaced.files["/w/main.sp"], written);

const bad = solver.solve({ ...request, solver_config: "beam=0" });
assert.match(bad.error, /^solver_config:/);
console.log(`solved, respaced and refused through WebAssembly, with ${events.length} progress events`);

// with the native CLI given, the same files under a limit of work: byte for byte what `rnl solve`
// writes, and `rnl solve --respace` after it
if (rnl) {
  const ladder = source
    .replace("R3 b 0 1k", "R3 b out 1k\nR4 a 0 1k\nR5 b 0 1k\nR6 in out 1k")
    .replace("{#R1 #R2 #R3}", "{#R1 #R2 #R3 #R4 #R5 #R6}")
    .replace("&b @ROUTE-NET;", "&b @ROUTE-NET; &in @ROUTE-NET; &out @ROUTE-NET;");
  const dir = mkdtempSync(join(tmpdir(), "rnl-solver-wasm-"));
  const native = (text, config, more = []) => {
    writeFileSync(join(dir, "main.sp"), text);
    rmSync(join(dir, "out"), { recursive: true, force: true });
    const args = ["solve", join(dir, "main.sp"), "-o", join(dir, "out"), "--no-std", "--solver-config", config, ...more];
    const run = spawnSync(rnl, args, { encoding: "utf8", timeout: 120000 });
    assert.equal(run.status, 0, run.stderr || String(run.error));
    return readFileSync(join(dir, "out", "main.sp"), "utf8");
  };
  let count = 0;
  try {
    for (const text of [source, ladder]) {
      // the limits are small enough that no run comes near the safety limit of 30 seconds, which
      // is on the clock and would stop the native run where this module's clock stands still
      for (const config of [
        'limit="seed"',
        'limit="work",work=200000',
        'limit="work",work=50000000',
        'limit="work",work=300000000',
        'layout="tree",limit="work",work=300000000',
        'layout="rows",limit="work",work=3000000',
      ]) {
        const here = solver.solve({ ...request, files: { "/w/main.sp": text }, solver_config: config });
        const there = native(text, config);
        assert.equal(here.files["/w/main.sp"], there, `solve with ${config}`);
        const wider = 'spacing_x=3,spacing_y=2,limit="work",work=100000000000';
        const again = solver.solve({ ...request, files: { "/w/main.sp": there }, respace: true, solver_config: wider });
        assert.equal(again.files["/w/main.sp"], native(there, wider, ["--respace"]), `respace after ${config}`);
        count += 2;
      }
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
  console.log(`${count} files written through WebAssembly are byte for byte those of ${rnl}`);
}
