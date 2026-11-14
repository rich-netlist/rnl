# The table of the solver's settings

A tool that lets its user set the solver draws its page from a table the solver gives, so that the page names no key, bound or default of its own. The same table comes from three places:

- the library: `rnl_solver::settings_table()` as Rust data, and `rnl_cli::solver_table::settings_table_json()` as JSON;
- the command line: `rnl solver-defaults --json` (without `--json`, the defaults as RNL named parameters, as before);
- the solver's WebAssembly module: `rnl_settings()`, or `settings()` of its loader ([bindings/solver-wasm](../bindings/solver-wasm/README.md)).

The table describes a 0.4 level. A level of an earlier version is laid out in rows and reads none of a file's settings; the check of a level (`rnl solve --check`) says what each level takes.

## Form

```json
{
  "version": 1,
  "groups": [{ "name": "layout", "what": "Layout" }],
  "settings": [
    { "key": "spacing_x", "group": "layout", "type": "integer", "min": -1, "max": 20, "default": 0,
      "in_file": true, "what": "…", "applies": { "layout": ["lines"] } }
  ],
  "objectives": [{ "name": "turns", "what": "Wires with fewer bends.", "implemented": true }],
  "default_objectives": [{ "items": [{ "name": "current_direction", "weight": 1 }], "tolerance": 0 }],
  "stages": [{ "name": "resolve", "what": "Checking the facts against each other" }]
}
```

- `groups`: the groups of the settings in the order a page shows them, each with a title.
- `settings`, one for each key the solver takes, in the order of their groups:
  - `type`: `integer`, `number`, `number-or-false`, `boolean`, `string`, `choice` or `objectives`. A page takes a type it does not know for a text box.
  - `choices`, for a choice: each value with what it means.
  - `min`, `min_exclusive` (a bound the value must be above) and `max`, where the value has them. A count is at most 2^53, the integers a number of RNL holds exactly.
  - `default`: the value when nothing gives it. It is left out where there is none, as for `work` and `junction`.
  - `in_file`: whether a file may give it in SOLVER-CONFIG. A run setting, such as `limit`, is the tool's alone.
  - `what`: one line that says what it does. `false_what`, for `number-or-false`, says what `false` means.
  - `applies`: where a solve uses it, by the values of other settings: the spacing only by the line layout, the beam and the candidates only by rows. A page may grey it out elsewhere. Respacing a drawing uses the spacing whatever the layout.
  - `shown_when`, `required_when`: where a page shows it, and where it must be given.
- `objectives`: every objective, with what it prefers, and whether this version computes it. One it does not compute is inert when given.
- `default_objectives`: the objectives a solve takes when none are given, layer by layer. The setting `objectives` has the same as its `default`, as text.
- `stages`: the stages of a solve's progress, as [`--progress=json`](solver-progress.md) names them, in the order of its table; a solve passes through those of its layout.

`applies`, `shown_when` and `required_when` guide a page; they refuse nothing. What a solve takes, a level at a time, is what the check of the settings says.

## What a level takes

The table says what may be given; the check says what each level takes. `rnl solve FILE --check --json` gives, for each level, every setting in effect with where it comes from (`default`, `package:NAME`, `header`, `level`, `command-line`) and `file:line` where it is written, then the warnings and the errors, each with the `key` it is about. A value of the file that the solver does not take is an error, with `refused`, the value left out; the solve goes on with the value before it.

A solve gives the same with its report: each level of the report of `rnl preview` has `check`, the check of that level's settings as they were before the solve, with the tool's. It is left out when the files could not be read for it. A page points at a setting by the `key` of an error there; `errors` of the report are the same as text, which a page counts.

Fields and rows are only ever added. `version` changes only if a field changes its meaning. The lines are English; their translations are kept apart from the solver.
