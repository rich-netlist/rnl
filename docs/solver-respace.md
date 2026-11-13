# Respacing a solved drawing

`rnl solve FILE -o DIR --respace` widens the gaps of each level to another spacing, starting from
where the record of the solver last put the objects, without searching anew. `rnl preview
--respace` does the same in memory, and its report says `"mode": "respace"`. Use it to try a wider
`spacing_x` or `spacing_y` on a drawing you like.

```sh
rnl solve circuit.sp -o solved
rnl solve solved/circuit.sp -o wider --respace --solver-config 'spacing_x=3,spacing_y=1'
```

## What it does

For each level of RNL 0.4, the solver:

1. Reads the level's `type="generated"` record onto the level's problem, as `rnl problem
   --previous` does.
2. Resolves the facts, then puts every object where the record places it and turns it as the
   record turns it. Objects the record does not place stay where the facts alone put them.
3. Routes the wires again on that placement.
4. Widens the gaps to the settings' spacing, the groups' own spacing included. The facts kept,
   the identities of the lines and the topology of the wires all stay as they are. It is the same
   step the line layout ends with (`docs/solver-skeleton.md`).

Nothing is searched and no object is turned. A gap that a fact holds, such as a written `PLACE`,
stays as it is; the report lists, for each axis, how many gaps were widened and which facts held
the others. The budget is shared as in a solve, and replaying with the reported work gives the
same file from the same record.

## Levels that are not respaced

A level whose record cannot be used is left as it is. The report lists it as not solved, with
one of three reasons, and the other levels are still respaced:

- **`no-generated`:** the level has no record of the solver, or a record that places nothing.
  Solve it first.
- **`mismatch`:** the record is not of the netlist as it is now. Its hash's input digest differs
  from the level's, as after an edit to the level, or its numbers no longer hold, or it has no hash
  that can be compared. Solve it again.
- **`unsupported`:** the level is of a version before RNL 0.4.

When no level can be respaced, nothing is written and the command still exits with 0, as a solve
does when it leaves every level as it is.

Two things stop the whole run with an error, and nothing is written:

- **The limit stops it.** A respace that its time or work limit stops has not spaced the drawing,
  so it is not written as if it had. Give it a higher limit.
- **The record does not keep a fact of the level.** That is not the solver's drawing of the level,
  as after an edit to the record itself. Solve the level again.

## Limits

- **Gaps are only widened.** A spacing smaller than the one the record was drawn with leaves the
  drawing as it is, and so does a spacing of 0. To come back to a narrower drawing, respace from
  the file of the solve again, not from a file already respaced wider.
- **The spacing comes from the tool.** Give it with `--solver-config`, or in the options of a
  library call. A `SOLVER-CONFIG` written in the file is part of the level: after an edit to it the
  record is no longer of the level, and the reason is `mismatch`.
- **Wires are routed again**, by the solver's router on the record's placement, not taken from the
  record. With the lines layout, whose own router drew the record's wires, they can come out
  otherwise.
- **Annotation clearance is not weighed** before the spacing is taken, as the lines layout does.
- `--respace` cannot be combined with `--external-solver` or `--check`.
