// Solve through the WebAssembly module with js/solver.js, as a Worker would: the answer, the
// progress events, respacing, and a bad request.
//
//   node bindings/solver-wasm/tests/solve.mjs PATH/TO/rnl_solver_wasm.wasm [PATH/TO/rnl]
//
// With the native CLI as the second argument, it also checks that the module writes the files
// `rnl solve` writes, byte for byte, under a limit of work.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
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

// the table of the settings, as the CLI prints it
const table = solver.settings();
assert.equal(table.version, 1);
assert.equal(table.settings.find((s) => s.key === "layout").default, "lines");
assert.ok(table.objectives.length > 0 && table.stages.length > 0 && table.groups.length > 0);

// the settings checked, nothing solved: of each level, and of the solver_config alone
const checked = solver.check({ ...request, solver_config: 'beam=6,grid="x"' });
assert.equal(checked.ok, false, JSON.stringify(checked));
assert.equal(checked.levels[0].settings.layout.from, "header");
assert.equal(checked.levels[0].settings.layout.at, "main.sp:2");
assert.equal(checked.levels[0].settings.beam.value, 6);
assert.equal(checked.levels[0].settings.beam.from, "command-line");
assert.deepEqual(checked.levels[0].errors.map((e) => e.key), ["grid"]);
const alone = solver.check({ solver_config: "beam=0" });
assert.equal(alone.levels.length, 1);
assert.equal(alone.levels[0].title, null);
assert.deepEqual(alone.levels[0].errors.map((e) => e.key), ["beam"]);
assert.match(solver.check({ ...request, files: { "/w/main.sp": source, "\\w\\main.sp": source } }).error, /are one file$/);

const bad = solver.solve({ ...request, solver_config: "beam=0" });
assert.match(bad.error, /^solver_config:/);
console.log(`solved, respaced and refused through WebAssembly, with ${events.length} progress events`);

// a callback that answers false cancels; one that throws cancels too, and solve throws what it
// threw; the instance goes on answering after any number of either
assert.deepEqual(solver.solve(request, () => false), { cancelled: true });
for (let k = 0; k < 40; k++) {
  assert.throws(
    () =>
      solver.solve(request, () => {
        throw new TypeError("mine");
      }),
    (e) => e instanceof TypeError && e.message === "mine",
  );
}
assert.equal(solver.solve(request).files["/w/main.sp"], written);
// a callback cannot solve with the solver that calls it: the inner call throws, which cancels the
// outer one and comes out of it
assert.throws(() => solver.solve(request, () => solver.solve(request)), /cannot solve with the same solver/);
assert.equal(solver.solve(request).files["/w/main.sp"], written);

// the answer names the files by the paths of the request, and two paths of one file are refused
const windows = "C:\\w\\main.sp";
const there = solver.solve({ ...request, entry: windows, files: { [windows]: source } });
assert.deepEqual(Object.keys(there.files), [windows], JSON.stringify(there).slice(0, 400));
const twice = solver.solve({ ...request, files: { "/w/main.sp": source, "\\w\\main.sp": source } });
assert.match(twice.error, /^files: .* are one file$/);

// a time limit, on a clock that goes on by 50 ms at each reading: the level stops for the time,
// soon after its second, and says how long it took on that clock
{
  let at = 0;
  let readings = 0;
  const timed = await loadSolver(readFileSync(wasm), {
    now: () => {
      assert.ok(++readings < 100000, "the time limit does not end the solve");
      return (at += 50);
    },
  });
  const seen = [];
  const config = 'layout="rows",limit="time",seconds=1';
  const answer = timed.solve({ ...request, solver_config: config }, (e) => void seen.push(e));
  const done = seen.find((e) => e.event === "level-done");
  assert.equal(done?.status, "time", JSON.stringify(seen).slice(0, 600));
  assert.ok(done.seconds >= 1 && done.seconds < 20, `the level took ${done.seconds} s on the clock`);
  assert.match(answer.report.text.join("\n"), /\n {2}time: (?!0\.000)\d+\.\d+ s/);
  // a clock that stops being a number, or goes back, is read as the latest reading before
  let odd = 0;
  const broken = await loadSolver(readFileSync(wasm), {
    now: () => {
      odd += 1;
      assert.ok(odd < 100000, "the time limit does not end the solve");
      return odd % 3 === 0 ? NaN : odd % 3 === 1 ? odd * 50 : -5;
    },
  });
  const after = [];
  broken.solve({ ...request, solver_config: config }, (e) => void after.push(e));
  const ended = after.find((e) => e.event === "level-done");
  assert.equal(ended?.status, "time", JSON.stringify(after).slice(0, 600));
  assert.ok(after.every((e) => !("elapsed" in e) || Number.isFinite(e.elapsed)), "every elapsed is a number");
  // a clock that is not a number at first, then far from zero: the time starts at its first number
  let late = 0;
  const started = await loadSolver(readFileSync(wasm), { now: () => (++late < 4 ? NaN : 1e9 + late * 50) });
  const then = [];
  started.solve({ ...request, solver_config: config }, (e) => void then.push(e));
  const stopped = then.find((e) => e.event === "level-done");
  assert.equal(stopped?.status, "time", JSON.stringify(then).slice(0, 600));
  assert.ok(stopped.work > 0 && stopped.seconds < 20, `the level took ${stopped.seconds} s and ${stopped.work} of work`);
  // a file of 0.2 is raised and solved in two passes: its time in the report is the second's
  const old = source.replace('@VERSION("0.4"); @SOLVER-CONFIG(layout="lines")', '@VERSION("0.2")');
  const raised = timed.solve({ ...request, files: { "/w/main.sp": old }, solver_config: 'limit="work",work=3000000' });
  assert.match(raised.report.text.join("\n"), /\n {2}time: (?!0\.000)\d+\.\d+ s/, JSON.stringify(raised).slice(0, 600));
}
console.log("cancelled, thrown through, renamed and timed through WebAssembly");

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
      // No run may come near the safety limit of 30 seconds, which is on the clock and would stop
      // the native run where this module's clock stands still: so the limits of work are small,
      // and only the small circuit is solved to its seed, which takes the ladder billions of work.
      for (const config of [
        ...(text === source ? ['limit="seed"'] : []),
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
  mkdirSync(dir, { recursive: true });
  const printed = spawnSync(rnl, ["solver-defaults", "--json"], { encoding: "utf8" });
  assert.equal(printed.status, 0, printed.stderr);
  assert.deepEqual(solver.settings(), JSON.parse(printed.stdout));
  console.log(`the table of the settings is that of ${rnl} solver-defaults --json`);
  // the check is that of the CLI
  writeFileSync(join(dir, "main.sp"), source);
  const config = 'beam=6,grid="x"';
  const cli = spawnSync(rnl, ["solve", join(dir, "main.sp"), "--check", "--json", "--no-std", "--solver-config", config], { encoding: "utf8" });
  assert.equal(cli.status, 1, cli.stderr);
  assert.deepEqual(solver.check({ ...request, solver_config: config }), JSON.parse(cli.stdout));
  rmSync(dir, { recursive: true, force: true });
  console.log(`the check of the settings is that of ${rnl} solve --check --json`);
}
