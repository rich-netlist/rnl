# Solver progress events

A frontend can follow a solve while it runs, as a stream of events. Every frontend gets the
same events:

- **CLI:** `rnl solve FILE -o DIR --progress=json` writes one JSON object per line on standard
  error. The report stays on standard output. `--progress` without `=json` keeps the older
  text lines.
- **Library:** `rnl_cli::progress::solve_progress` and `rnl_cli::preview::preview_progress`
  give each event to a callback, on the thread that solves. Returning false cancels.

```sh
rnl solve circuit.sp -o solved --progress=json 2> events.ndjson
```

Observing never consumes work. A solve with a work limit writes the same files with or without
progress. With a time limit, reading the clock to report can move the cutoff slightly, as
profiling does.

## Events

Every event has `event`. `level` counts the levels of the run from 0. `elapsed` is seconds
since that level began. Fields are only ever added; a reader ignores fields it does not know,
and shows an event or a stage it does not know by its name.

| `event` | When | Fields |
| --- | --- | --- |
| `run` | Before the first level | `progress`: the format version, 1; `levels`: how many levels the run solves |
| `level` | A level begins | `level`, `title`, `again`; `limit`: `"time"` with `seconds`, `"work"` with `work`, `"seed"` or `"none"`, with the level's own settings applied |
| `stage` | A level enters a stage | `level`, `stage`, `work`, `elapsed` |
| `work` | More work | `level`, `stage`, `work`, `elapsed`, `tried`; `fraction`, the share of the limit used from 0 to 1, for a time or a work limit |
| `best` | A better candidate than any before in the level | `level`, `tried`, `work`, `elapsed`; `cost`, numbers to compare element by element, lower first, with no promised meaning |
| `level-done` | A level is solved | `level`; `status`: `"complete"`, `"time"`, `"work"`, `"safety"` or `"cancelled"`; `work`, the count to replay with; `seconds`, the solver's own time; `kept`, `discarded`, the facts; `junction_seconds`, the junction placement outside the budget, when there was one |
| `done` | The files are written (CLI only) | `seconds`, the whole run; `exit`, the exit code; `written`, the files written |
| `error` | The run stopped on an error (CLI only) | `message` |

- **How often:** the solver's `stage` and `work` events come at most every quarter of a second
  in a level. A stage left sooner is not told, and the next `work` event names the stage then.
- **`tried`:** the candidates the solver has taken up so far in the level. It never goes down.
- **`best` and layouts:** the rows layout tells each better candidate. The tree and lines
  layouts tell their result once, at the end of the layout.
- **`again`:** whether the level is solved a second time, after its file was raised to a newer
  version. It goes away with the 0.2 and 0.3 support.

## Stages

`rnl_cli::progress::STAGES` lists each stage with one line of English for a frontend to show.

| Stage | What it does |
| --- | --- |
| `resolve` | Checking the facts against each other |
| `structure` | Reading the circuit's structure and groups |
| `search` | Searching for placements |
| `coords` | Placing the candidates on the grid |
| `route` | Routing the wires |
| `score` | Comparing the candidates |
| `tree` | Laying out the groups |
| `lines` | Laying out along the signal lines |
| `junctions` | Placing the junction dots, outside the solver's budget |
| `write` | Writing the record back, outside the solver's budget |

## Cancelling

When the callback returns false, a level under a time limit stops at the solver's next check.
The solve then fails with `rnl_cli::solve::CANCELLED`, before the next level or after the last.
It gives back no partial result. A level under a work limit runs to its end first.
