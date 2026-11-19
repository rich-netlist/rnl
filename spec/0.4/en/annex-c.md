## Annex C Reserved for later versions (informative)

This annex lists what this version of the standard leaves out and says may come in a later version, with the sections that mention it. This version reserves no keywords or forms of writing for these items (5.3, 16.4), and a later version need not add them.

| Left for a later version | See |
|---|---|
| Carriers for further hosts, including block comments (such as `#com … #endcom` of Eldo), and the handling of a comment terminator of the host that this brings | 2.2, 2.5 |
| Record IDs; tools find their records by `type`, `author` and the keys of statements | 2.8 |
| Expressions | 3.7 |
| Standard sentences that say whether a definition is applied to a whole set or once for each member (`each`) | 4.5 |
| Optional slots; without them a selection always has a scope, if only `#` | 5.3, 10.1, 13.7 |
| More than one subscript expanded in one reference, and further forms of subscripts, such as `<*2>A`, `A<0:2*2>` and `D[0..7]` | 6.11 |
| Standard diagnostic codes | 7.6 |
| A way to suppress diagnostics in a file, and a strict mode, for instance as fields of the record head | 7.6 |
| Units of several facts: a definition or a usage whose facts are kept or discarded together in conflict resolution. Still open: the priority of such a unit, nested usages, and priorities written from outside through `::` | 8.5 |
| Scaling of symbols and of instances | 9.2.1, 11.7 |
| Arrows at the ends of wires | 9.2.5 |
| More line styles | 9.3 |
| Colour names and `rgb()` | 9.3 |
| Pattern fills, gradients and shadows | 9.3 |
| The intersection of sets in one statement | 10.1 |
| Choosing the symbol of a primitive by its parameters. For a few primitives of each dialect, the simulator decides the kind by the line: in SPICE, a B source written with `V=` is a voltage source and one with `I=` a current source. Subcircuits and models do not vary in this way. One way is a condition of SELECT-INSTANCE such as `param=` (the line has this parameter); another is a type that the adapter of the dialect gives these primitives (6.12), selected with `type=`. The number of terminals is selected with `terminals=` (10.1). Until then, the standard package draws both kinds by its present rules | 10.1, 6.12 |
| Rotating a whole set as one block with ORIENT | 11.2 |
| Several drawings of one host object, as separate objects with names of their own | 11.4 |
| A graphic that frames a group of objects | 13.4 |
| Annotations anchored on wires: positions along a wire (such as `t` or `offset`), and symbols or arrows anchored in the middle of a wire | 13.3, 15.2 |
| References in `${…}` to the attributes of other objects, such as `${R3:VALUE}` of KiCad | 13.6 |
| The diagonal projections `"x+y"` and `"x-y"` for constraints | 14.1 |
| A USE-PACKAGE that applies to its own level only | 16.2 |
| A package split over several files | 16.2 |
| A manifest format for a netlist and its package files | 16.2 |
| A content digest for packages, such as a `digest=` parameter of USE-PACKAGE | 16.2 |
| References that a package uses for itself only, apart from those it passes on to its users | 16.2 |
| Package-qualified names, other names for packages and level tags, such as `$acme.DIFFERENTIAL_PAIR` or `as=`, in a form other than a `.` after the name | 16.2 |
| An extension mechanism: keywords with a vendor prefix, such as `@ACME.ROUTE`, declared in the header record as required or optional extensions | 2.8, 16.4 |
| Exact RNL, that is, the conditions under which a file can be drawn without a solver, and criteria for round trips; these may instead be agreed between tools | 17 |
