// Solve through the WebAssembly module with js/solver.js, as a Worker would: the answer, the
// progress events, respacing, and a bad request.
//
//   node bindings/solver-wasm/tests/solve.mjs PATH/TO/rnl_solver_wasm.wasm
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { loadSolver } from "../js/solver.js";

const [wasm] = process.argv.slice(2);
assert.ok(wasm, "usage: node solve.mjs RNL_SOLVER_WASM");
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
