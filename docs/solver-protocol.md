# Independent solver protocol

The `rnl-solver-io` library encodes an independent solver's decisions as JSON and validates them against the original `LevelProblem` before writeback. Protocol version 1 is independent of the solver executable's version.

The `rnl-solver` executable has its own version, starting at `0.1.0`. It reads one input JSON document from stdin (or an explicit file) and writes one result document to stdout. Diagnostics go to stderr. Viewers continue using the solver library.

## Commands

```sh
rnl problem --solver-input --solver-config 'limit="work",work=20000' circuit.sp > input.json
rnl-solver input.json > result.json
rnl solve --external-solver rnl-solver --solver-config 'limit="time",seconds=10' -o solved circuit.sp
```

`rnl problem` without `--solver-input` retains its readable JSON format. That format is not a lossless transport. Machine export prints source diagnostics to stderr and refuses to emit a request when interpretation reports an error. `--previous` remains available only in the readable format.

`--external-solver` selects an executable explicitly; file and package settings cannot launch a process. Repeated `--external-arg` options each append one literal argument, without a shell. `--external-timeout` sets a positive process timeout in seconds per level (120 by default), independently of the solver's own time or work limit. The host stops its directly launched process on timeout; external programs are responsible for their descendants. `--profile` observes the embedded solver and cannot be combined with an external solver.

The host invokes the external program for each level as it reads after earlier in-memory writebacks, including the existing second pass when a 0.2 file needs raising. Input and output use temporary file-backed standard streams to avoid pipe deadlocks. Responses larger than 64 MiB are rejected. A failed process, malformed response or stale input aborts the run before any solved file is written. The runtime writes valid decisions using the producer's name and version as the generated record's author. The embedded solver keeps its existing author and output bytes.

## Input envelope

An input has `format: "rnl-solver-input"`, `version: 1`, and a `levels` array. Each entry contains:

- `request`: its zero-based array ordinal.
- `input`: the original problem's input digest.
- `problem`: the complete runtime `LevelProblem`.
- `options`: the caller's settings before file settings merge: `canonical`, `hints_for_low`, `safety_seconds`, `legacy_alias`, and ordered `overrides`.

Preserving explicit overrides is necessary: an explicit zero can override a nonzero file setting, while an omitted zero default cannot. Duplicate override keys are rejected. The problem includes all typed facts and nested tables, electrical and coordinate-only pin identities, source locations, usages, text markup and layout, literal values and string units, groups, and junction settings. The codec checks indices, position and usage dependencies, finite coordinates and supported nesting depths.

Every route carries a required `tree` boolean. A resolved tree retains its own members even when another route names the same net, including when newer content appears in an older drawing level. A missing flag is an error rather than an implicit whole-net request.

JSON numbers are parsed from their decimal spelling without an intermediate lossy conversion. Work counters remain integers; finite floating-point values, including negative zero, round-trip exactly. Protocol decoding has explicit nesting limits and does not change the ordinary CLI or Viewer JSON number parser.

## Result envelope

```json
{
  "format": "rnl-solver-result",
  "version": 1,
  "solver": { "name": "example-solver", "version": "1.0.0" },
  "levels": [
    {
      "request": 0,
      "level": 0,
      "input": "0123456789abcdef",
      "options": "example-settings",
      "solution": {
        "placements": [],
        "branches": [],
        "junctions": [],
        "wires": [],
        "wire_routes": []
      },
      "report": null
    }
  ]
}
```

`request` is the zero-based ordinal in the request. `level` and `input` must match that request's level number and `rnl_runtime::problem::input_digest`. The ordinal distinguishes identical levels, including those from different files. Each requested level must occur exactly once; responses may arrive in any order and the decoder restores request order. Missing, extra, duplicate or stale levels are rejected.

`options` records the effective settings used for the result's persisted hash. If the report supplies `canonical_options`, it must equal `options`. Solver-specific settings remain opaque to the transport.

## Decisions

All five solution arrays are required, including when empty.

| Array | Entry |
| --- | --- |
| `placements` | `object` index; nullable finite `x` and `y`; nullable `orient` code |
| `branches` | `at: [x, y]` and boolean `junction` |
| `junctions` | Typed point references for dots on existing points |
| `wires` | `route` index and ordered `points` |
| `wire_routes` | Authored `wire` index and ordered replacement `points` |

A wire point has one of these forms:

```json
{ "kind": "anchor", "point": { "kind": "origin", "object": 0 } }
{ "kind": "branch", "branch": 0 }
{ "kind": "bend", "at": [3.25, -7.5] }
```

Point references retain distinct `origin`, `pin`, `symbol-pin`, `pin-position` and `centre` kinds. A terminal pin carries `object`, `pin` and `terminal`; an independent pin or pin coordinate carries `object`, `pin` and `reference`. A centre carries ordered `members` recursively. The transport preserves centres even though the runtime writer cannot represent a wire through a centre; the writer reports such decisions as unsolved.

References retain `anonymous`, `name`, `number`, `generated` and `generated-number` kinds. Numbered and generated references retain each component of their `path` array, without concatenating or reparsing it. Orientations use the runtime's eight orientation names.

## Validation and writeback

Indices must refer to the original problem or to branches of this solution. A wire requires at least two points. Duplicate object placements and authored-wire rewrites are rejected. Objects without an orientation cannot receive one. Coordinates must be finite; NaN and infinity cannot become absent decisions.

An in-range pin index alone does not authorize writable text. The decoder binds terminal names and independent-pin references to that exact pin in the original problem. Electrical references and coordinate references are separate; converting an authorized electrical reference to a coordinate requires `::position`. Named symbol-pin identities are derived from the original symbol metadata with the runtime's quoting rules. Anonymous references remain unwritable and are reported by the runtime as unsolved. Result text cannot redirect an anchor to another object or inject a new RNL statement.

The runtime writes against the original problem. Result messages do not select output files or replace the original model. Callers must decode and validate the complete response before writing any files.

## Reports

`report: null` means that the solver supplied no fact report. It does not mean that all facts were satisfied or that none were satisfied. A supplied report preserves all `SolveReport` fields, including:

- Kept, failed and discarded fact indices, conflict groups, and complete/undecided flags.
- Statement hints, including source positions, usage chains and optional point identities.
- Unrouted trees and inert objectives.
- All four work counters, the shared budget limit, admitted work and stop reason.
- Canonical replay settings, errors, warnings and notes.

Integer work counters remain JSON integers and must be read without converting through floating-point values. Index checks and classification consistency are validated; conflict minimality and the solver's proof claims are the solver's responsibility. Time, work, seed and unlimited limits remain distinct, as do complete, time, work and safety stop reasons.

The independent executable prints the original budget observation to stderr. For a time or work run that does not hit its safety limit, the result on stdout uses consumed work as its limit, normalizes a time stop to a work stop, and records replay settings. Repeating that work budget produces identical result bytes and written files. Safety stops remain explicitly marked and do not promise replay. The library codec itself preserves all report fields without this normalization. When a third-party solver supplies `report: null`, `rnl solve` says the report was not supplied and omits fact and work counts.

## Verification

Tests compare complete native solutions, reports and written records after JSON byte round trips for all three currently supported language versions. Additional cases exercise nonempty branch and rewrite arrays, every reference variant, conflict reports, values above `2^53`, reordered identical requests, stale inputs, invalid indices, inconsistent settings, nonfinite coordinates and forged writable references.
