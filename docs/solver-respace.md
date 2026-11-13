# Respacing a solved drawing

`rnl solve FILE -o DIR --respace` spaces each level again, starting from where the record of the
solver last drew it, without searching anew. `rnl preview --respace` does the same in memory,
and its report says `"mode": "respace"`. Use it to try another `spacing_x` or `spacing_y` on a
drawing you like.

```sh
rnl solve circuit.sp -o solved
rnl solve solved/circuit.sp -o wider --respace --solver-config 'spacing_x=3,spacing_y=1'
```

For each level, the solver:

1. Reads the level's `type="generated"` record onto the level's problem, as `rnl problem
   --previous` does.
2. Puts every object where the record places it and turns it as the record turns it. Objects the
   record does not place stay where the facts alone put them.
3. Resolves the facts and routes the wires again on that placement.
4. Spaces the drawing to the settings' spacing, the groups' own spacing included. The facts kept,
   the identities of the lines and the topology of the wires all stay as they are. It is the same
   step the line layout ends with (`docs/solver-skeleton.md`).

Nothing is searched and no object is turned. A gap that a fact holds, such as a written `PLACE`,
stays as it is; the report lists, for each axis, how many gaps were widened and which facts held
the others. The budget is shared as in a solve, and replaying with the reported work gives the
same file from the same record.

A level whose record cannot be used is not respaced and nothing is written for it. The report
lists it as not solved, with one of two reasons:

- **`no-generated`:** the level has no record of the solver, or a record that places nothing.
  Solve it first.
- **`mismatch`:** the record is not of the netlist as it is now. Its hash's input digest differs
  from the level's, as after an edit to the level, or its numbers no longer hold, or it has no hash
  that can be compared. Solve it again.
- **`unsupported`:** the level is of a version before RNL 0.4.

`--respace` cannot be combined with `--external-solver`.

The wires are routed again by the solver's router on the record's placement, not taken from the
record. With the lines layout, whose own router drew the record's wires, they can come out
otherwise. Respacing does not weigh annotation clearance before taking the spacing, as the lines
layout does.
