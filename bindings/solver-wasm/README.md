# solver-wasm

The solver as WebAssembly, in a module of its own beside the [core's](../runtime-wasm/README.md). A page loads it only when it solves.

It takes a solve request and gives the Document drawn as solved, the report, and the texts the solve wrote, as `rnl preview` does. It uses `rnl_cli::preview::preview_progress`, or `rnl_cli::preview::respace` when the request asks to respace (see [respacing a solved drawing](../../docs/solver-respace.md)).

## Building

```bash
node bindings/runtime-wasm/build.mjs --solver
```

This builds the core's module and this one with the workspace's `wasm` profile. It puts `rnl_solver_wasm.wasm` and [`js/solver.js`](js/solver.js) with its declarations into `dist/` beside the core's, and prints the sizes of both modules. The `package.json` there then exports them as `./solver` and `./solver.wasm`; the npm package [`@rich-netlist/runtime`](../../packages/runtime/README.md#solver) is built so. The solver's module is the larger: it compiles in the solver and the writing back as well as the core.

## The interface

The exports are those of the core's module, with `rnl_solve` in place of `rnl_build`:

1. `rnl_request(len)` gives where to write the request.
2. `rnl_solve()` solves it and returns the answer's length.
3. `rnl_response()` gives where the answer is.

`rnl_check()` checks the settings of the request written there, as `rnl solve --check --json` does, nothing solved, and returns the length of the answer. `rnl_settings()` takes no request: it puts the table of the solver's settings and objectives where `rnl_response()` says, as `rnl solver-defaults --json` prints it, and returns its length.

A panic traps; `rnl_panic_len`, `rnl_panic_message` and `rnl_alloc_failed` say why.

Unlike the core's module, it imports three functions from `env`:

| Import | |
| --- | --- |
| `rnl_now() -> f64` | Milliseconds, such as `performance.now()`. The solver's time limit is measured on it. |
| `rnl_progress(ptr, len)` | One progress event as UTF-8 JSON, as [`rnl solve --progress=json`](../../docs/solver-progress.md) writes them. |
| `rnl_cancelled() -> i32` | Not 0 to cancel. It is asked after each event. |

The clock has to go on. The time is counted from the first reading that is a number. A reading that is not a number, or is earlier than one before, counts as the latest reading before it; but on a clock that stands still a time limit never ends, and the default limit is one of time. With such a clock, give a limit of work or the seed in `solver_config`.

After a panic or another trap, an instance is not to be used again: what the call held is not released.

## The loader

[`js/solver.js`](js/solver.js) gives these imports and wraps the exports. `loadSolver(bytes)` returns an object whose `solve(request, onProgress)` gives the answer.

- **Progress:** `onProgress` gets each event as an object.
- **Cancelling from the callback:** when `onProgress` returns `false`, the solve is cancelled and the answer is `{cancelled: true}`. When it throws, the solve is cancelled too and `solve` throws what it threw. Neither harms the instance.
- **After a trap:** `solve` throws `The solver stopped: …` with the panic's message, and the loader makes a new instance for the next call.
- **One solve at a time:** `onProgress` cannot call `solve` of the same solver, which throws then.
- **The settings:** `settings()` gives the table of the solver's settings and objectives, from which a page draws its settings ([docs/solver-settings-table.md](../../docs/solver-settings-table.md)).
- **The check:** `check(request)` gives each level's settings, with where each comes from, and their warnings and errors, as `rnl solve --check --json` prints them, nothing solved. The request is a solve request; without `entry`, its `solver_config` alone is checked. Each `at` is `file:line`, the file named as the core's diagnostics name it. Neither `settings()` nor `check()` reads the clock or calls a callback.

The call is synchronous and can take the solver's whole time limit. Run it in a Worker of its own. Terminating that Worker and starting another also cancels, at once.

## The request and the answer

The request is all the solve reads: `entry`, `files`, `package_dirs`, `dialect`, `formats`, `std`, `include_std`, `solver_config` (RNL named parameters, as `--solver-config` takes them), `level` and `respace`. A Worker that runs it needs nothing from the core's Worker.

Other fields are ignored. That includes fields of the core's build request that this module does not read: `pool`, `auto_include`, `library_prefix`, `library_edits` and `std_version`.

The answer is one of three:

- `{document, report, files}`;
- `{cancelled: true}`;
- `{error}`.

`files` holds the texts the solve wrote, each under the path its file has in the request. Two paths of the request that are one file, such as `a/b.sp` and `a\b.sp`, are an error.

A level's `seconds` in the events, and its time in the report, are measured on the clock the caller gives. The time of the junction dots, `junction_seconds`, is not measured in this module and is 0.
