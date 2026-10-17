## Annex B Standard sentences (informative)

A standard sentence is a statement in the standard form with free text added between its tokens, so that the statement reads like a sentence. The free text does not change the meaning (3.3); the rules for the wording are in 4.5. In the tables, `…` at the start of a sentence stands for the segments before it in a pipe, and `{…}` or a final `…` for content that is left out.

### B.1 Placement and orientation

| Keyword | Case | Sentence |
|---|---|---|
| PLACE | two axes | `#M1 is at @PLACE(10, 20)` |
| | one axis | `the x of #M1 is at @PLACE(x=10)` |
| | written by an editor | `the x of #X2 is at @PLACE(x=3, priority="high")` |
| | in a pipe | `… => it is at @PLACE(40, 0)` |
| ORIENT | rotated | `#M2 @ORIENT("R90")` |
| | mirrored | `#M2 @ORIENT("MX"), i.e., flipped upside down` |
| | | `#M2 @ORIENT("MY"), i.e., flipped left-right` |
| | | `#M3 @ORIENT("MXR90"), i.e., flipped upside down, then rotated 90°` |
| | | `#M3 @ORIENT("MYR90"), i.e., flipped left-right, then rotated 90°` |
| | in a pipe | `… => they @ORIENT("R90")` |
| ORIENT-RELATION | two objects | `from #M1 to #M2, they have @ORIENT-RELATION("MY"), i.e., flipped left-right` |
| | sets | `from {#M1 #M3} to {#M2 #M4}, they have @ORIENT-RELATION("MY"), i.e., flipped left-right` |
| | the same orientation | `from #M5 to {#M7 #M8}, they have @ORIENT-RELATION("R0"), i.e., the same orientation` |
| LAYER | | `#X is on @LAYER("bottom")` |
| | in a pipe | `… => they are on @LAYER("top")` |
| GROUP | | `{#M1 #M2 #M5} as a @GROUP` |
| | named | `%s1_set = {#M1 #M2 #M5} as a @GROUP` |

### B.2 Layout constraints

| Keyword | Case | Sentence |
|---|---|---|
| ALIGN | same y | `{#M1 #M2} @ALIGN("y"), i.e., same y` |
| | same x | `{#M1 #C1} @ALIGN("x"), i.e., same x` |
| | low priority | `{#M7 #M5} @ALIGN("y", priority="low"), i.e., same y` |
| | in a pipe | `… => they @ALIGN("y")` |
| ORDER | with a direction | `{#M1 #M2 #M3} are in @ORDER("left-to-right")` |
| | either direction | `the x of {#M1 #M1:2 #M2:2 #M2} are in @ORDER("x"), either ascending or descending` |
| | in a pipe | `… => they are in @ORDER("bottom-to-top")` |
| DISTRIBUTE | | `the x of {#R1 #R2 #R3} @DISTRIBUTE("x"), i.e., in equal steps` |
| | in a pipe | `… => the x of them @DISTRIBUTE("x"), i.e., in equal steps` |
| DISTANCE | with a direction | `from #M1 to #M2, they have @DISTANCE("bottom-to-top", min=4)` |
| | from a set | `from {#M6 #M7 #CL} to %pout, they have @DISTANCE("left-to-right", min=2)` |
| | either direction | `between %a_set and %b_set, they have @DISTANCE("x", min=1)` |
| PRIORITY | | `%c1 has @PRIORITY("disabled")` |
| | in a pipe | `… => they have @PRIORITY("low")` |

- `the x of` (or `the y of`) is written only before ORDER and DISTRIBUTE whose direction or axis is `"x"` (or `"y"`), and before a PLACE that gives one axis (B.1); ALIGN takes the gloss `, i.e., same x` instead.
- The x is that of the position of each member: for an instance, the point that PLACE sets (the origin of its symbol), not the centre of its outline; a set counts by its centre (14.1).

### B.3 Appearance

| Keyword | Case | Sentence |
|---|---|---|
| SET-SYMBOL | one object | `#M1 @SET-SYMBOL to $NMOS` |
| | a set | `{#M1 #M2} @SET-SYMBOL to $NMOS` |
| | the block symbol of a subcircuit | `this subcircuit # @SET-SYMBOL to $NAND2_SYM` |
| | in a pipe | `… => they @SET-SYMBOL to $PMOS` |
| SET-STYLE | | `{#CC #RZ} @SET-STYLE to $orange` |
| | in a pipe | `… => they @SET-STYLE to $grey` |
| DEFINE-STYLE | | `$orange = @DEFINE-STYLE(linecolor="#e07000", textcolor="#e07000") {}` |
| | merging another style | `$wide = @DEFINE-STYLE(linewidth=2) based on {$orange}` |

### B.4 Symbols and graphics

| Keyword | Case | Sentence |
|---|---|---|
| DEFINE-SYMBOL | | `$TRI = @DEFINE-SYMBOL {…}` |
| PIN | | `at (-4, 0) there is a @PIN(terminal="in", direction="left")` |
| LINE | | `a @LINE running through {(0, 0) (4, 0)}` |
| ARC | | `an @ARC running through {(0, 2) (2, 0) (0, -2)}` |
| BEZIER | | `a @BEZIER with control points {(0, 0) (1, 1) (2, 1) (3, 0)}` |
| CIRCLE | | `centered at (0, 0), a @CIRCLE(radius=1)` |
| RECTANGLE | centre and size | `centered at (0, 0), a @RECTANGLE(width=4, height=2)` |
| | two corners | `a @RECTANGLE with corners {(0, 0) (4, 2)}` |
| ELLIPSE | centre and size | `centered at (0, 0), an @ELLIPSE(width=4, height=2)` |
| | corners of its box | `an @ELLIPSE with box corners {(0, 0) (4, 2)}` |
| TEXT | in a symbol | `on ? there is a @TEXT("${name}", key="name") => it is at @PLACE(-2, 2.5)` |
| ATTACH | in a symbol | `$ARROW is @ATTACH to (0, -1.5) => …` |

- Graphics written in the scene, as annotations (13.4), read the same.

### B.5 Wires and text

| Keyword | Case | Sentence |
|---|---|---|
| WIRE | | `net &n1 has a @WIRE connecting {#R1:2 %j #R2:1}` |
| | a bus | `net &D<3:0> has a @WIRE connecting {%p1 %p2}, which is a bus` |
| | a routing mode | `net &in has a @WIRE(mode="vertical-horizontal") connecting {#VIN:1 #R1:1}` |
| ROUTE | | `net &n1 is fully @ROUTE` |
| | a set | `nets {&a &b} are fully @ROUTE` |
| | in a pipe | `… => it is fully @ROUTE` |
| TEXT | on an object | `on #M1 there is a @TEXT("W/L = ${w}/${l}", key="wl", dy=-3)` |
| | at a point | `at (_, _) there is a @TEXT("RC low-pass") => named %title` |
| | in a pipe | `… => on them there is a @TEXT("${w}", key="size")` |

- Naming a point, such as `%j = (_, _)`, takes no added words.

### B.6 Definitions and applications

| Keyword | Case | Sentence |
|---|---|---|
| DEFINE | | `$PAIR = a definition, @DEFINE(roles=["a", "b"], parameters=[["gap", 2]]) {…}` |
| AS | two roles | `#M1(in_p) and #M2(in_n) are treated @AS $DIFFERENTIAL_PAIR` |
| | a role that takes a set | `{#M5 #M7 #M8} are treated @AS $MATCHED` |
| | one object | `#X1:3 is treated @AS $SUPPLY_HIGH` |
| | three roles and a parameter | `#M1(a), #M2(b) and %ax(axis) are treated @AS(priority="high") $MIRRORED` |
| | a parameter of the definition | `{#R1 {%rc} #M1} are treated @AS(direction="top-to-bottom") $FLOW` |
| | a usage in a role | `%dp(pair) and %cm(load) are treated @AS $OTA` |
| | named | `%dp = #M1(in_p) and #M2(in_n) treated @AS $DIFFERENTIAL_PAIR` |
| | named in a pipe, with another object | `#M1(in_p) and #M2(in_n) are treated @AS $DIFFERENTIAL_PAIR => named %dp => it and %cm are treated @AS $OTA` |
| ATTACH | on a net | `$GROUND is @ATTACH to &vss` |
| | on a terminal | `$NO_CONNECT is @ATTACH to #M1:4` |
| | on a point | `$JUNCTION is @ATTACH to %j` |
| | two roles | `$DIFF_PORT is @ATTACH to &inp(a) and &inn(b)` |
| | followed by other segments, named | `$GROUND is @ATTACH to &vss => it is at @PLACE(24, 38) => named %g0` |
| | on each of several objects | `$SIZE_LABEL is @ATTACH to {#M1 #M2 #M3}` |
| | the fill, on a set | `$GROUND_STUB is @ATTACH to $GROUND_FILL_SET` |
| | the fill, in a pipe | `among the terminals at each level in # and below, @SELECT-TERMINAL(net="vss", wired=false) => $GROUND_STUB is @ATTACH(gap=3) to them` |

- AS is used when the objects together make up the thing, such as a circuit structure, a placement, or a local way of drawing such as `$RC_STAGE`; ATTACH when the thing is added on the objects, such as a ground, a port tag, a no-connect mark, a junction dot, a label for each instance, or the fill. The two keywords do the same thing; only the place of the definition differs (9.4).
- `gap` is a parameter of `$GROUND_STUB`, the least distance from the terminal down to the ground symbol (2 by default). Like the role names `in_p` and `axis`, it is a name given in a definition, not a standard word.

### B.7 Selections

A selection is written as a choice among candidates: `among <candidates>, @SELECT-…(conditions)`. The wording of the candidates depends on the kind of the scope and on the kind selected:

| Scope | Selecting | Candidates |
|---|---|---|
| `#` | any kind | `the <kind> at each level in # and below` |
| `#`, with `recursive=false` | any kind | `the <kind> in # only` |
| a set of the kind selected (a filter) | that kind | `{…}` |
| net | instances | `the instances connected to &en` |
| net | terminals | `the terminals connected to &en` |
| net | wires | `the wires on &en` |
| instance | terminals | `the terminals of #R1` |
| instance | nets | `the nets connected to #M1` |
| terminal | instances | `the instances owning #M1:g` |
| terminal | nets | `the nets connected to #M1:g` |
| wire | nets | `the nets of %w13` |
| instance, terminal, wire, net, graphic | labels | `the labels attached to #M1` |
| usage (no usages among its members) | any kind | `the members of %dp` |
| usage (usages among its members, or not known) | any kind | `all members of %st1 at any depth` |
| usage | usages (SELECT-USAGE) | `%st1 itself and the usages inside it` |
| any object | the usages that contain it (SELECT-ANCESTOR-USAGE) | `the usages containing #M1` |
| a set of several kinds (kind not known) | | write `for %mixed_set, @SELECT-…` instead |

- `<kind>` follows the keyword: instances, terminals, nets, wires, labels, annotations, usages.
- In the middle of a pipe, the scope in the candidates is written `them`, such as `=> among all members of them at any depth,`. Whether the usages selected before have usages among their members is not known when the sentence is written, so `at any depth` is used.
- For the members of usages the wording is `at any depth`, not `at any level`: a level is a level of the circuit, as in `at each level in # and below`.
- A rule in a package applies at every level that uses the package, so in a package `at each level in # and below` becomes `at every level that uses this package, #`.

| Keyword | Case | Sentence |
|---|---|---|
| SELECT-INSTANCE | this level and below | `among the instances at each level in # and below, @SELECT-INSTANCE(model="nch") => they @SET-SYMBOL to $NMOS` |
| | this level only | `among the instances in # only, @SELECT-INSTANCE(type="pmos", recursive=false) => …` |
| | a filter | `among {#M1 #M2 #R1}, @SELECT-INSTANCE(name="M*") => they @ORIENT("MY")` |
| | on a net | `among the instances connected to &en, @SELECT-INSTANCE => they @SET-STYLE to $grey` |
| | owning a terminal | `among the instances owning #M1:g, @SELECT-INSTANCE => they are on @LAYER("top")` |
| | members of a usage | `among all members of %st1 at any depth, @SELECT-INSTANCE(role="pair")` |
| | members of a usage without nested usages | `among the members of %dp, @SELECT-INSTANCE` |
| | in the middle of a pipe | `among the usages at each level in # and below, @SELECT-USAGE of $DIFFERENTIAL_PAIR => among all members of them at any depth, @SELECT-INSTANCE => they @SET-STYLE to $signal` |
| SELECT-TERMINAL | this level and below | `among the terminals at each level in # and below, @SELECT-TERMINAL(net="vss", wired=false) => $GROUND_STUB is @ATTACH(gap=3) to them` |
| | of an instance | `among the terminals of #R1, @SELECT-TERMINAL => on them there is a @TEXT("${net}")` |
| | on a net | `among the terminals connected to $GROUND_NET_SET, @SELECT-TERMINAL(wired=false) => $GROUND_STUB is @ATTACH to them` |
| | a filter | `among {#M1:s #M2:s}, @SELECT-TERMINAL(wired=false) => $GROUND is @ATTACH to them` |
| SELECT-NET | this level and below | `among the nets at each level in # and below, @SELECT-NET(port=true) => $IO_PORT is @ATTACH to them` |
| | of an instance | `among the nets connected to #M1, @SELECT-NET => named %m1_net_set` |
| | of a terminal | `among the nets connected to #M1:g, @SELECT-NET => they are fully @ROUTE` |
| | of a wire | `among the nets of %w13, @SELECT-NET => they are fully @ROUTE` |
| | a filter | `among {&a &b}, @SELECT-NET(wired=false) => they are fully @ROUTE` |
| SELECT-WIRE | this level and below | `among the wires at each level in # and below, @SELECT-WIRE(bus=true) => they @SET-STYLE to $bus` |
| | on a net | `among the wires on &clk, @SELECT-WIRE => they @SET-STYLE to $red` |
| SELECT-LABEL | this level and below | `among the labels at each level in # and below, @SELECT-LABEL(key="wl") => they @SET-STYLE to $grey` |
| | attached to an object | `among the labels attached to #M1, @SELECT-LABEL => they @SET-STYLE to $grey` |
| SELECT-ANNOTATION | this level and below | `among the annotations at each level in # and below, @SELECT-ANNOTATION => they @SET-STYLE to $base` |
| | a filter | `among {%n1 %f1 #M1}, @SELECT-ANNOTATION` |
| SELECT-USAGE | this level and below | `among the usages at each level in # and below, @SELECT-USAGE of $DIFFERENTIAL_PAIR => they have @PRIORITY("disabled")` |
| | without a definition | `among the usages at each level in # and below, @SELECT-USAGE => they @SET-STYLE to $base` |
| | a usage and the usages inside it | `among %st1 itself and the usages inside it, @SELECT-USAGE of $DIFFERENTIAL_PAIR` |
| SELECT-ANCESTOR-USAGE | | `among the usages containing #M1, @SELECT-ANCESTOR-USAGE of $DIFFERENTIAL_PAIR => named %dp_set` |
| | a terminal | `among the usages containing #M1:3, @SELECT-ANCESTOR-USAGE of $GROUND => they @ORIENT("R180")` |
| any selection | kind of the scope not known | `for %mixed_set, @SELECT-TERMINAL => …` |

### B.8 Sets and other keywords

| Keyword | Case | Sentence |
|---|---|---|
| ADD-TO | | `&vdd @ADD-TO $SUPPLY_NET_SET` |
| | in a pipe | `… => they @ADD-TO $SUPPLY_NET_SET` |
| REMOVE-FROM | | `{&vdd &vss} @REMOVE-FROM $PORT_FILL_SET` |
| UNION | | `%u_set = %a_set and %b_set @UNION` |
| BUNDLE | | `#XI<3:0> is drawn as a @BUNDLE => named %xi` |
| CURRENT | in | `#M5:1 has @CURRENT("in")` |
| | out | `#:iout has @CURRENT("out")` |
| | unspecified | `#M5:2 has @CURRENT("unspecified")` |

### B.9 Header record and packages

No words are added. The header record is read in the standard spelling and does not use aliases (16.5).

| Keyword | Case | Sentence |
|---|---|---|
| VERSION | | `@VERSION("0.3")` |
| ALIAS | | `@ALIAS("@位于", "@PLACE")` |
| USE-ALIASES | | `@USE-ALIASES(name="rnl-zh", version="0.2")` |
| DEVICE | | `@DEVICE(…)` |
| PACKAGE | | `@PACKAGE(name="acme-style", version="1.2")` |
| USE-PACKAGE | in a header record; also a reference in the header record of a package file | `@USE-PACKAGE(name="rnl-std", version="0.2")` |
| | in a level | `@USE-PACKAGE(name="acme-style", version="1.2")` |
