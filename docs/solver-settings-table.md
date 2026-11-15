# The table of the solver's settings

A tool that lets its user set the solver draws its page from a table the solver gives, so that the page names no key, bound or default of its own. The same table comes from three places:

- the library: `rnl_solver::settings_table()` as Rust data, and `rnl_cli::solver_table::settings_table_json()` as JSON;
- the command line: `rnl solver-defaults --json`. With `--layout NAME` it holds only the settings, objectives and stages that apply to that layout, with `layout`, its name, and `lines_order` only for `lines`. Without `--json`, the defaults are RNL named parameters, a line for each layout, the default first, each with the settings that layout uses (`applies`, and `shown_when` under the defaults); `--layout NAME` prints the line of that layout alone;
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
  "objectives": [{ "name": "turns", "what": "Wires with fewer bends.", "implemented": true,
                   "applies": { "layout": ["rows", "tree"] } }],
  "default_objectives": [{ "items": [{ "name": "current_direction", "weight": 1 }], "tolerance": 0 }],
  "stages": [{ "name": "search", "what": "Searching for placements", "applies": { "layout": ["rows"] } }],
  "lines_order": { "what": "The lines layout ranks the drawings it makes in a fixed order, …",
                   "ranks": [{ "name": "complete", "what": "A drawing of the whole level made." }] }
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
  - `applies`: where a solve uses it, by the values of other settings: the spacing only by the line layout, the beam and the candidates only by rows. Without it, a setting applies to every layout. A page may hide or grey it out elsewhere. Respacing a drawing uses the spacing whatever the layout.
  - `shown_when`, `required_when`: where a page shows it, and where it must be given.
- `objectives`: every objective, with what it prefers, whether this version computes it, and `applies`, the layouts that take it, left out when every layout does. One it does not compute is inert when given. The line layout takes only `overlaps`, `crowding`, `misleading` and `wire_over_symbol`, with the layers that hold them, to rank the group-tree layout it starts from; it ranks its own drawings by `lines_order`.
- `default_objectives`: the objectives a solve takes when none are given, layer by layer. The setting `objectives` has the same as its `default`, as text.
- `stages`: the stages of a solve's progress, as [`--progress=json`](solver-progress.md) names them, in the order of its table, each with `applies`, the layouts that pass through it, left out when it is not of one layout: `search`, `coords`, `route` and `score` are those of rows, `tree` of the group tree, `lines` of the line layout; `respace` is that of a respace, whatever the layout.
- `lines_order`: how the line layout ranks the drawings it makes, which no setting changes: `what`, a line to show beside it, and `ranks`, first rank first, each with a `name` and a line. It is there to be shown, not set.

`applies`, `shown_when` and `required_when` guide a page; they refuse nothing. A setting given where the layout in effect does not use it, with a value other than its default, is a warning of the check by its key, that it is ignored under that layout; so, under the line layout, are the layers of objectives other than the default that hold none of the four it takes: it takes a layer that holds one of them whole. What a solve takes, a level at a time, is what the check of the settings says.

## What a level takes

The table says what may be given; the check says what each level takes. `rnl solve FILE --check --json` gives, for each level, `layout`, the layout in effect, and every setting in effect with where it comes from (`default`, `package:NAME`, `header`, `level`, `command-line`) and `file:line` where it is written. Then come `unused`, the keys of those settings that the layout does not use or that are shown only for other values of the settings, as `applies` and `shown_when` say, then the warnings and the errors, each with the `key` it is about. Without `--json` the check prints each level's layout first, then only the settings it uses, then the warnings and the errors. A value of the file that the solver does not take is an error, with `refused`, the value left out; the solve goes on with the value before it.

A solve gives the same with its report: each level of the report of `rnl preview` has `check`, the check of that level's settings as they were before the solve, with the tool's. It is left out when the files could not be read for it. A page points at a setting by the `key` of an error there; `errors` of the report are the same as text, which a page counts.

Fields and rows are only ever added. `version` changes only if a field changes its meaning. The lines are English; their translations are kept apart from the solver.
