# solver-wasm

The solver as WebAssembly, in a module of its own beside the [core's](../runtime-wasm/README.md). A page loads it only when it solves.

It takes a solve request and gives the Document drawn as solved, the report, and the texts the solve wrote, as `rnl preview` does. It uses `rnl_cli::preview::preview_progress`, or `rnl_cli::preview::respace` when the request asks to respace (see [respacing a solved drawing](../../docs/solver-respace.md)).

## Building

```bash
node bindings/runtime-wasm/build.mjs --solver
```

This builds the core's module and this one with the workspace's `wasm` profile. It puts `rnl_solver_wasm.wasm` and [`js/solver.js`](js/solver.js) into `dist/` beside the core's, and prints the sizes of both modules. The solver's module is the larger: it compiles in the solver and the writing back as well as the core.

## The interface

The exports are those of the core's module, with `rnl_solve` in place of `rnl_build`:

1. `rnl_request(len)` gives where to write the request.
2. `rnl_solve()` solves it and returns the answer's length.
3. `rnl_response()` gives where the answer is.

A panic traps; `rnl_panic_len`, `rnl_panic_message` and `rnl_alloc_failed` say why.

Unlike the core's module, it imports three functions from `env`:

| Import | |
| --- | --- |
| `rnl_now() -> f64` | Milliseconds, such as `performance.now()`. The solver's time limit is measured on it. |
| `rnl_progress(ptr, len)` | One progress event as UTF-8 JSON, as [`rnl solve --progress=json`](../../docs/solver-progress.md) writes them. |
| `rnl_cancelled() -> i32` | Not 0 to cancel. It is asked as the events come. |

`js/solver.js` gives these imports and wraps the exports: `loadSolver(bytes)` returns an object whose `solve(request, onProgress)` gives the answer.

The request is all the solve reads: `entry`, `files`, `package_dirs`, `dialect`, `formats`, `std`, `include_std`, `solver_config` (RNL named parameters, as `--solver-config` takes them), `level` and `respace`. A Worker that runs it needs nothing from the core's Worker.

The call is synchronous and can take the solver's whole time limit. Run it in a Worker of its own; to cancel, terminate that Worker and start another.

The answer is one of three:

- `{document, report, files}`;
- `{cancelled: true}`;
- `{error}`.
