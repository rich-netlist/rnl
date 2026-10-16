# rnl-edit

Writing RNL into host netlists: the edit transactions of the editor interface (`notes/rnl-editor-interface/spec-draft.md` in rnl-dev, section 4). This is the first part, what a tool needs that prints a netlist and its drawing together, such as Analog Canvas's SPICE+RNL and Spectre+RNL export:

```rust
pub fn write(request: &WriteRequest) -> Result<String, String>
```

`WriteRequest` holds the netlist, its dialect (`"spice"` or `"spectre"`), the `author` of the records, and the ops, as the draft writes them in JSON. `write` reads the netlist with the host adapter of the dialect and returns it with the ops written in; the netlist's own text stays as it is, line endings included.

- **The header record** goes after line 1, which is never read (2.2, 2.8), and after the lines that continue it: VERSION (0.2, the version the toolkit reads), then a USE-PACKAGE for each `use-package` op, each name and version once. The `level` of a `use-package` is not used.
- **One record of the author's for each level** the ops name, `subckt:NAME` (case-insensitive in SPICE), or `file` or `file:PATH` for the file level, at the end of the level's own text (17): before the `.ends` of a subcircuit, before the `.end` of the file, or at the end of the file. The statements are in the order of the ops.
- **The comment carrier** is the dialect's, `*` or `//`.
- **The standard form** (4.3): the object first, positional parameters before named ones, numbers rounded to a millionth in their shortest form, the default orientation R0 and `mode="auto"` left out. A line is at most 132 characters, the comment marker included; a longer statement breaks at spaces outside strings and brackets, the further lines indented by four more spaces. A header record that does not fit on one line has one statement a line.
- **Texts**: `markup` as written, or `lines` of runs, as the Document's `Text.lines` holds them, written as markup (13.6) that reads back as the same runs. A backslash is written `\textbackslash{}`, and `_ { } $ % & # ^` with a backslash before them.

The ops written so far are `set` (symbol, position, orientation; on `#`, the level itself, the symbol of a subcircuit, 9.2.6), `add-usage`, `add-point`, `add-wire`, `add-text`, `add-to`, `remove-from`, `use-package`, and:

- `add-shape`, a graphic in the scene (13.4): `line`, `rectangle`, `circle`, `ellipse`, `arc` or `bezier`, its points, its `params` (width, height, radius, fill, arrow) and `style`, written in that order; a rectangle, circle or ellipse given by its centre and its size is written on its centre. A `layer` comes after it as `@LAYER`. The writer keeps the order of the ops, which is the order the graphics of a layer are drawn in (11.6).
- `define-symbol`, a symbol (9.2): its `box`, `pins` (terminals by number or name, position, direction, `label` as markup or lines of runs) and `prims`, graphics as `add-shape` takes them, in the symbol's coordinates. It is written one statement of its body a line.

A line that cannot break outside brackets breaks after a comma inside them, such as between the parameters of a long TEXT. An op that cannot be written refuses the whole request with the reason and the op's index: an unknown op or field, a level the netlist does not have, a graphic whose points do not fit its shape, a field of the draft not written yet (`style` and `layer` of `set` and `add-usage`, `layer` of `add-wire`; `unset` and `remove`). A netlist that has RNL records already is refused too: merging with them, `replace_own`, `base` and refusing ops that do not take effect come later.

`rnl write REQUEST.json -o OUT` ([`rnl-cli`](../rnl-cli/README.md)) and the `write` request of the [WebAssembly binding](../../bindings/runtime-wasm/README.md) call it.
