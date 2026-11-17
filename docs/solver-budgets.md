# Solver settings and replay in 0.4

The 0.4 solver defaults to a ten-second shared budget. Fact resolution, model expansion,
structure analysis, candidate search, coordinate pivots and routing consume weighted work.
The report gives the work actually admitted, the stopping reason and a replay command.
Released input versions retain their existing solver behavior during development.

```sh
rnl solver-defaults
rnl solve circuit.sp -o solved --solver-config 'limit="time",seconds=10' --progress
rnl solve circuit.sp -o replay --solver-config 'limit="work",work=243750'
```

`rnl solver-defaults` prints the defaults a line for each layout, the default first, with the settings that layout uses; `--layout NAME` prints one layout's line. `rnl solver-defaults --json` prints the table of the settings, with their kinds, bounds, defaults, what they do and the layouts they apply to, from which a tool draws its page of them ([the table of the solver's settings](solver-settings-table.md)). `rnl solve FILE --check` says each level's layout and the settings it takes.

Under the line layout the replay command also gives `tree_work`, as in
`limit="work",work=243750,tree_work=81000` or `tree_work=false`: the work at which the
group-tree start the line layout begins with stopped refining its arrangement, which the level's
work alone does not tell. Give it with the work.

Use the reported count for the same drawing level, input, settings and solver build. For a
file with several levels, replay each level with `--level` and that level's own count. Keep
`--report` the same: requesting additional conflict explanations consumes budget too.
Progress goes to standard error and does not enter the saved file. Profiling or progress
observation can move a time cutoff; neither consumes deterministic work.

Time is checked before admitting a positive-cost operation. The reported count is the sum
of admitted operations. A work replay refuses the same next operation, including inside
resolution or a coordinate search. Stopping is latched. Already proved facts remain valid;
unfinished checks are undecided, with warnings, rather than contradictory or satisfied.
Necessary result assembly and forced route completion are deterministic finalization.

The seconds limit applies to each drawing level's solver, not the whole CLI process.
Package loading and interpretation, re-reading between levels, record validation and
writeback happen outside that budget. A ten-second search can therefore take longer
end to end. Compare the level's solver time with the CLI/process time when diagnosing
an overrun; profiling can itself slow search and move a time cutoff. Replaying the
reported work is the way to inspect the same search trajectory.

`--timeall S` gives the whole file a time as well: `solve` and `preview` take it, and a request
to the solver's WebAssembly module takes it as `timeall`. The levels of 0.4 of a time limit share its
`S` seconds, which run from the start of the solve. Each level has the least of its own limit
and an even share of what is left: the seconds left, in whole milliseconds, over the levels of a
time limit still to solve, itself among them. What a level leaves goes to those after it. Once
nothing is left, each level still to solve has its seed only. A level of a limit of work, or of
its seed, is solved as its limit says, and its time counts against the others'. The share is
given to the level as the tool's last settings, so the `level` event of the progress tells it,
and the work the level reports replays it as any other. A respace searches nothing and does not read it. A file of 0.2 that is
raised to 0.3 is solved twice, and each pass has the whole time; the second is the one written.

```sh
rnl solve circuit.sp -o solved --timeall 60
```

`SolveReport.canonical_options` is the options text to pass to `sig2` when saving a 0.4
result. It normalizes the time limit to the admitted work, so replay preserves the record's
hash as well as its placements and wires. A caller must not hash its original time settings
in place of this returned text. The CLI uses the returned text automatically.

Deterministic modes also have an emergency wall-clock ceiling, `safety_seconds=30` by default. A
`Safety` stop explicitly reports that the requested deterministic work run was interrupted.
It is not a successful completion of the requested work budget. The solver uses only the
caller's clock, including in WebAssembly; it does not read environment switches or spawn
worker threads.

At a zero-second limit, the old 0.4 resolution-only deadline still allowed placement to run.
The shared zero-second limit admits no model work: the facts remain undecided and the two
symbols stay at their initial coincident positions. This boundary example demonstrates the
scope of the limit; it is not the default ten-second setting.

| Previous resolution-only limit | Shared solve limit |
| --- | --- |
| ![Two symbols placed](images/shared-budget-before.png) | ![No optional placement admitted](images/shared-budget-after.png) |

`limit="seed"` finishes the selected layout's initial heuristic pass without later
refinement. Rows, group trees and line layouts use the same shared budget. This mode
does not introduce randomized ordering.

## Settings

`--solver-config` takes the named parameters inside SOLVER-CONFIG parentheses, including
quoted strings and flat arrays. Repeated flags merge by key. Precedence is solver defaults,
package headers in package order, the file header, the level's records, then explicit tool
settings. Defaults do not mask file values. `--solver-options` remains a deprecated alias;
its previous semicolon syntax is also accepted during the transition.

| Key | Value | Default | Allowed in a file |
| --- | --- | --- | --- |
| `layout` | `"rows"`, `"tree"`, `"lines"` | `"lines"` for a 0.4 level; a level of an earlier version is always laid out in rows | yes |
| `spacing_x`, `spacing_y` | integer from -1 through 20; the line layout spaces by them | 0 | yes |
| `lines_weights` | two integers from 0 through 1000, `[crossing, bend]`, not both 0; the line layout ranks by crossing × crossings + bend × bends | `[1, 4]` | yes |
| `virtual_weight` | number from 0 through 1; in the line layout's `symbol_lines`, a row or column of the symbols that only virtual symbols take counts this: the objects of usages, such as supply, ground or port symbols, whatever joins them; one an instance of the netlist or a bundle takes counts 1 | 0.5 | yes |
| `lines_order` | the layers the line layout ranks by after its facts, false joins, annotations and rules, written as `objectives` writes layers but without tolerances; items `current`, `crossings_and_bends`, `hidden_ports`, `rail_heights`, `mirror_pairs`, `dots_near_pins`, `length`, `wire_lines`, `symbol_lines`, `area`; an item left out does not rank | `current,crossings_and_bends,symbol_lines+wire_lines*2,length,area` | yes |
| `reroute_nets` | boolean; the line layout's router routes each net of three pins or more again, a wire at a time, and keeps a wire routed again where the net then has fewer crossings and bends, weighed by `lines_weights`, or as many and is shorter; it takes longer, and its searches count in the work | false | yes |
| `junction` | definition name as a string; empty disables dots | unset | yes |
| `tolerance` | finite nonnegative number | 0.000001 | yes |
| `grid` | positive number, or `false` | 1 | yes |
| `beam` | integer from 1 through 1024 | 4 | yes |
| `objectives` | layered objectives as a string | existing layered objectives | yes |
| `limit` | `"time"`, `"work"`, or `"seed"` (initial heuristic candidates only) | `"time"` | no |
| `seconds` | finite nonnegative number | 10 | no |
| `work` | exact nonnegative integer count | required for work mode | no |
| `safety_seconds` | finite nonnegative number | 30 | no |
| `candidates`, `astar_expansions` | nonnegative integer caps | 256, 200000 | no |
| `tree_work` | exact nonnegative integer count, or `false`; the line layout only | taken from the limit | no |

`tree_work` is where the line layout's group-tree start stops refining its arrangement, so that
the line search has the rest of the level's budget: at that work of the level, or, with `false`,
not before its search ends or the budget runs out. Left out, the solver fixes it before the start
from the limit, without reading the clock: three tenths of the level's limit, counted as work for
a limit of work and in its estimate of nanoseconds for a limit of time, and never past its
estimate of five seconds. A solve reports where the start stopped in its replay command. Under
a limit of `"seed"` the start refines without an allowance, and `tree_work` is not read.

The line layout spaces by a nonzero spacing. Under the row and the group-tree layouts it is
parsed and hashed, with a warning by its key that it is ignored under that layout, as for every
setting the layout in effect does not use ([the table's `applies`](solver-settings-table.md)).

Run settings in a file warn and are ignored. Unknown keys warn and are ignored. A value of
a known key that is not taken is an error for that key (14.10): in the file, it leaves the value
before it in place, or the default, and the solve goes on; invalid tool values are rejected
before solving. The problem hash includes every file value of a key. Junction names are
resolved by the runtime in the target level, including tool overrides, so an override
retains the same name and alias rules as a file setting. Tool overrides enter the options
digest, while the input digest retains the original file settings. A saved result therefore
remains solved when reopened without the command-line overrides. `JunctionConfig.effective()`
selects the resolved tool value for writing; its base definition still identifies the file.

A key's values are checked one by one, not merged first: the runtime gives every value of a
key in the order above, and the solver takes them in that order. A value it does not take
is an error for its key alone, naming the value that stays in effect, the one before it or the
default, and the other keys of the same `SOLVER-CONFIG` still apply; so of each key the last
value the solver accepts is in effect. A level's `grid="x"` over a package's `grid=2`
therefore gives `grid=2` and an error, and `rnl solve` solves the level with `grid=2` and exits
with 1, as for an error of the reading. The runtime itself judges `spacing_x`, `spacing_y`
(a finite whole number) and `junction` (a definition whose first role accepts a point in the
level): a value it does not take is an error in reading and is not passed on, so the value
before it stays in effect the same way. A value is judged in each level it applies to: a
header's `junction` that names a definition only some levels have is an error in the others
alone, and the others still use it.

`rnl solve FILE --check` checks the settings without solving or writing anything. For each
level it prints every setting in effect and where it comes from (`default`,
`package:NAME`, `header`, `level`, `command-line`), with `file:line` for a setting written in a
package, the header or a level, then the warnings and the errors. A value that is not taken
is an error, with `refused`, `{"value", "from", "at"}`, in the JSON. Without
FILE, it checks the command line's settings alone. It reports every problem, not only the
first, each with its key. `--json` prints
`{"ok", "levels": [{"title", "settings": {KEY: {"value", "from", "at"}}, "warnings": [{"key", "message"}], "errors": [...]}]}`;
without FILE, `levels` has one entry whose `title` is `null`, and a problem that is not about
one key has the `key` `null`. It exits with 1 when a level has an error, and with 2 when the
command line itself is wrong. `rnl_cli::solve::check_settings` gives the same result to a
viewer.

## Work units and checks

Weights are integers, multiplied by at least one input unit per checkpoint:

| Operation | Weight | Size multiplier |
| --- | --- | --- |
| Resolution/model encoding | 8 | objects, model reconstruction size, or one encoded pair |
| Simplex pivot | 16 | variables × tableau rows |
| Candidate estimate | 32 | elements |
| Placement | 32 | objects |
| Routing | 1 | A* expansion, grid cells, per-tree anchor pairs, or route count at a candidate boundary |
| Structure | 4 | group members or graph edges visited |

The units are a replay protocol, not a promise that every unit takes the same time. Exact
rational arithmetic and input shape affect cost. Changing the checkpoint sequence or
weights requires re-calibration and work counts from the new solver build.

The public `solver_parity` example supplies two synthetic circuits. Initial native debug
calibration ran each three times; all eight runs per repetition finished below 0.012 seconds.
With a requested million units, the two examples admitted 243750 and 997848 units; with
4096 units, they admitted 4056 and 3032. A partly affordable operation is not admitted, so
the count can be below the requested limit. These small examples calibrate the accounting
and replay checks; they do not establish a runtime bound for arbitrary circuits.

Tests cover time-to-work replay at internal cutoffs, kept facts with orientation lists and
local pins, tolerance-dependent coordinates, interrupted Cartesian TABLE expansion,
nested scopes, panic restoration, thread isolation, settings precedence and full saved-file
identity. CI also builds the same example for native and WebAssembly and compares eight
complete generated records byte for byte:

```sh
cargo build -p rnl-runtime-wasm --example solver_parity
cargo build -p rnl-runtime-wasm --example solver_parity --target wasm32-unknown-unknown
node bindings/runtime-wasm/tests/solver-equality.mjs target/debug/examples/solver_parity target/wasm32-unknown-unknown/debug/examples/solver_parity.wasm
```
