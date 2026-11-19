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
| | either of two | `#M5 @ORIENT(["R0", "MY"]), i.e., not turned or flipped left-right` |
| | in a pipe | `… => they @ORIENT("R90")` |
| ORIENT-RELATION | two objects | `from #M1 to #M2, they have @ORIENT-RELATION("MY"), i.e., flipped left-right` |
| | either of two | `from #M3 to #M4, they have @ORIENT-RELATION(["R0", "MY"]), i.e., the same or flipped left-right` |
| | the same orientation | `from #M5 to {#M7 #M8}, they have @ORIENT-RELATION("R0"), i.e., the same orientation` |
| LAYER | | `#X is on @LAYER("bottom")` |
| | in a pipe | `… => they are on @LAYER("top")` |
| GROUP | | `{#M1 #M2 #M5} as a @GROUP` |
| | named | `%s1_set = {#M1 #M2 #M5} as a @GROUP` |
| | spacing | `{#M3 #M4} as a @GROUP(spacing_x=-1), i.e., one grid unit tighter in x` |

### B.2 Layout constraints

| Keyword | Case | Sentence |
|---|---|---|
| ALIGN | same y | `{#M1 #M2} @ALIGN("y"), i.e., same y` |
| | same x | `{#M1 #C1} @ALIGN("x"), i.e., same x` |
| | low priority | `{#M7 #M5} @ALIGN("y", priority="low"), i.e., same y` |
| | in a pipe | `… => they @ALIGN("y")` |
| ORDER | with a direction | `{#M1 #M2 #M3} are in @ORDER("left-to-right")` |
| | in a pipe | `… => they are in @ORDER("bottom-to-top")` |
| DISTRIBUTE | | `{#R1 #R2 #R3} are in @DISTRIBUTE("left-to-right"), i.e., in equal steps` |
| | in a pipe | `… => they are in @DISTRIBUTE("bottom-to-top"), i.e., in equal steps` |
| DISTANCE | with a direction | `from #M1 to #M2, they have @DISTANCE("bottom-to-top", min=4)` |
| | from a set | `from {#M6 #M7 #CL} to %pout, they have @DISTANCE("left-to-right", min=2)` |
| TABLE | | `@TABLE layout: {{#M1 #M2 #M3} {#M4 {#M5 #M6} #M7}}` |
| | across | `@TABLE(transpose=true) layout: {{#M1 #M2} {#M3 #M4}}` |
| PRIORITY | | `%c1 has @PRIORITY("disabled")` |
| | in a pipe | `… => they have @PRIORITY("low")` |
| SOLVER-CONFIG | | `@SOLVER-CONFIG(spacing_x=2, spacing_y=1)` |

- `the x of` (or `the y of`) is written only before a PLACE that gives one axis (B.1); ALIGN takes the gloss `, i.e., same x` instead.
- The x is that of the position of each member: for an instance, the point that PLACE sets (the origin of its symbol), not the centre of its outline; a set stands for each of its members (14.1).

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
| LINE | | `through {(0, 0) (4, 0)} runs a @LINE` |
| ARC | | `through {(0, 2) (2, 0) (0, -2)} runs an @ARC` |
| BEZIER | | `the control points {(0, 0) (1, 1) (2, 1) (3, 0)} make a @BEZIER` |
| CIRCLE | | `centered at (0, 0), a @CIRCLE(radius=1)` |
| RECTANGLE | centre and size | `centered at (0, 0), a @RECTANGLE(width=4, height=2)` |
| | two corners | `with corners {(0, 0) (4, 2)}, a @RECTANGLE` |
| ELLIPSE | centre and size | `centered at (0, 0), an @ELLIPSE(width=4, height=2)` |
| | corners of its box | `in the box with corners {(0, 0) (4, 2)}, an @ELLIPSE` |
| TEXT | in a symbol | `on ? there is a @TEXT("${name}", key="name") => it is at @PLACE(-2, 2.5)` |
| ATTACH | in a symbol | `$ARROW is @ATTACH to (0, -1.5) => …` |

- Graphics written in the scene, as annotations (13.4), read the same.

### B.5 Wires and text

| Keyword | Case | Sentence |
|---|---|---|
| WIRE | | `net &n1, through {#R1:2 %j #R2:1}, has a @WIRE` |
| | a bus | `net &D<3:0>, through {%p1 %p2}, has a @WIRE, which is a bus` |
| | a routing mode | `net &in, through {#VIN:1 #R1:1}, has a @WIRE(mode="vertical-horizontal")` |
| ROUTE-NET | | `net &n1 is drawn with @ROUTE-NET` |
| | a set | `nets {&a &b} are drawn with @ROUTE-NET` |
| | in a pipe | `… => it is drawn with @ROUTE-NET` |
| ROUTE-OBJECT | | `the pins of {#M1 #M2 #M5} are joined with @ROUTE-OBJECT` |
| UNROUTE-NET | | `net &vdd is left out with @UNROUTE-NET(priority="high")` |
| UNROUTE-OBJECT | | `the pins of #M3 are left out with @UNROUTE-OBJECT` |
| TEXT | on an object | `on #M1 there is a @TEXT("W/L = ${w}/${l}", key="wl", dy=-3)` |
| | at a point | `at (_, _) there is a @TEXT("RC low-pass") => named %title` |
| | in a pipe | `… => on them there is a @TEXT("${w}", key="size")` |

- Naming a point, such as `%j = (_, _)`, takes no added words.

### B.6 Definitions and applications

| Keyword | Case | Sentence |
|---|---|---|
| DEFINE | | `$PAIR = a definition, @DEFINE(roles=["a", "b"], parameters=[["gap", 2]]) {…}` |
| AS | three roles | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N` |
| | a role that takes a set | `{#M5 #M7 #M8} are treated @AS $MATCHED` |
| | one object | `#M3 is treated @AS $DIODE_CONNECTED` |
| | with a priority | `#M1(left), #M2(right) and %ax(axis) are treated @AS(priority="high") $MIRRORED_LAYOUT` |
| | a parameter of the definition | `{#R1 #R2} are treated @AS(gap=6) $SPACED` |
| | a usage in a role | `%dp(pair) and %cm(load) are treated @AS $OTA` |
| | named | `%dp = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N` |
| | named in a pipe, with another object | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N => named %dp => it and %cm are treated @AS $OTA` |
| ATTACH | on a net | `$GROUND is @ATTACH to &vss` |
| | on a terminal | `$NO_CONNECT is @ATTACH to #M1:4` |
| | on a point | `$JUNCTION is @ATTACH to %j` |
| | two roles | `$DIFF_PORT is @ATTACH to &inp(a) and &inn(b)` |
| | followed by other segments, named | `$GROUND is @ATTACH to &vss => it is at @PLACE(24, 38) => named %g0` |
| | on each of several objects | `$SIZE_LABEL is @ATTACH to {#M1 #M2 #M3}` |
| | on a set | `$GROUND_STUB is @ATTACH to $GROUND_FILL_SET` |
| | the fill, in a pipe | `among the instances at each level in # and below, @SELECT-INSTANCE(recursive=true) => among the pins of them, @SELECT-PIN(net="vss", wired=false) => $GROUND_STUB is @ATTACH(gap=3) to them` |

- AS is used when the objects together make up the thing, such as a circuit structure, a placement, or a local way of drawing such as `$RC_STAGE`; ATTACH when the thing is added on the objects, such as a ground, a port tag, a no-connect mark, a junction dot, a label for each instance, or the fill. The two keywords do the same thing; only the place of the definition differs (9.4).
- `gap` is a parameter of `$GROUND_STUB`, the least distance from the pin down to the ground symbol (1 by default). Like the role names `left` and `axis`, it is a name given in a definition, not a standard word.

### B.7 Selections

A selection is written as a choice among candidates: `among <candidates>, @SELECT-…(conditions)`. The wording of the candidates depends on the kind of the scope and on the kind selected:

| Scope | Selecting | Candidates |
|---|---|---|
| `#` | any kind | `the <kind> in # only` |
| `#`, in a statement that reaches the levels below (10.1) | any kind | `the <kind> at each level in # and below` |
| a set of the kind selected (a filter) | that kind | `{…}` |
| net | instances | `the instances connected to &en` |
| net | terminals | `the terminals connected to &en` |
| net | wires | `the wires on &en` |
| instance | terminals | `the terminals of #R1` |
| instance, bundle, usage with a symbol | pins | `the pins of #M1` |
| terminal | pins | `the pins of #X1:vdd` |
| net | pins | `the pins on &en` |
| the result of a pipe | pins | `the pins of them` |
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

- `<kind>` follows the keyword: instances, terminals, pins, nets, wires, labels, annotations, usages.
- In the middle of a pipe, the scope in the candidates is written `them`, such as `=> among all members of them at any depth,`. Whether the usages selected before have usages among their members is not known when the sentence is written, so `at any depth` is used.
- For the members of usages the wording is `at any depth`, not `at any level`: a level is a level of the circuit, as in `at each level in # and below`.
- A rule in a package counts as written in the level of its USE-PACKAGE (10.4); with `recursive=true`, a package writes `at every level that uses this package, #` in place of `at each level in # and below`.

| Keyword | Case | Sentence |
|---|---|---|
| SELECT-INSTANCE | this level and below | `among the instances at each level in # and below, @SELECT-INSTANCE(model="nch", recursive=true) => they @SET-SYMBOL to $NMOS` |
| | this level only | `among the instances in # only, @SELECT-INSTANCE(type="pmos") => …` |
| | a filter | `among {#M1 #M2 #R1}, @SELECT-INSTANCE(name="M*") => they @ORIENT("MY")` |
| | on a net | `among the instances connected to &en, @SELECT-INSTANCE => they @SET-STYLE to $grey` |
| | owning a terminal | `among the instances owning #M1:g, @SELECT-INSTANCE => they are on @LAYER("top")` |
| | members of a usage | `among all members of %st1 at any depth, @SELECT-INSTANCE(role="pair")` |
| | members of a usage without nested usages | `among the members of %dp, @SELECT-INSTANCE` |
| | in the middle of a pipe | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => among all members of them at any depth, @SELECT-INSTANCE => they @SET-STYLE to $signal` |
| SELECT-TERMINAL | this level and below | `among the terminals at each level in # and below, @SELECT-TERMINAL(net="vss", recursive=true) => they have @ASSERT-TERMINAL(same=["net"])` |
| | of an instance | `among the terminals of #R1, @SELECT-TERMINAL => on them there is a @TEXT("${net}")` |
| | on a net | `among the terminals connected to $GROUND_NET_SET, @SELECT-TERMINAL => among the pins of them, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| SELECT-PIN | in the level | `among the pins in # only, @SELECT-PIN(wired=false) => on them there is a @TEXT("${net}", key="netname")` |
| | of terminals | `among the pins of {#M1:s #M2:s}, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| | of a terminal | `among the pins of #X1:vdd, @SELECT-PIN => they @SET-STYLE to $red` |
| SELECT-NET | this level and below | `among the nets at each level in # and below, @SELECT-NET(port=true, recursive=true) => $IO_PORT is @ATTACH to them` |
| | of an instance | `among the nets connected to #M1, @SELECT-NET => named %m1_net_set` |
| | of a terminal | `among the nets connected to #M1:g, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | of a wire | `among the nets of %w13, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | a filter | `among {&a &b}, @SELECT-NET(wired=false) => they are drawn with @ROUTE-NET` |
| SELECT-WIRE | this level and below | `among the wires at each level in # and below, @SELECT-WIRE(bus=true, recursive=true) => they @SET-STYLE to $bus` |
| | on a net | `among the wires on &clk, @SELECT-WIRE => they @SET-STYLE to $red` |
| SELECT-LABEL | this level and below | `among the labels at each level in # and below, @SELECT-LABEL(key="wl", recursive=true) => they @SET-STYLE to $grey` |
| | attached to an object | `among the labels attached to #M1, @SELECT-LABEL => they @SET-STYLE to $grey` |
| SELECT-ANNOTATION | this level and below | `among the annotations at each level in # and below, @SELECT-ANNOTATION(recursive=true) => they @SET-STYLE to $base` |
| | a filter | `among {%n1 %f1 #M1}, @SELECT-ANNOTATION` |
| SELECT-USAGE | this level and below | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => they have @PRIORITY("disabled")` |
| | without a definition | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) => they @SET-STYLE to $base` |
| | a usage and the usages inside it | `among %st1 itself and the usages inside it, @SELECT-USAGE of $DIFFERENTIAL_PAIR_N` |
| SELECT-ANCESTOR-USAGE | | `among the usages containing #M1, @SELECT-ANCESTOR-USAGE of $DIFFERENTIAL_PAIR_N => named %dp_set` |
| any selection | kind of the scope not known | `for %mixed_set, @SELECT-TERMINAL => …` |

### B.8 Sets and other keywords

| Keyword | Case | Sentence |
|---|---|---|
| ADD-TO | | `&vdd @ADD-TO $SUPPLY_NET_SET` |
| | in a pipe | `… => they @ADD-TO $SUPPLY_NET_SET` |
| REMOVE-FROM | | `&agnd @REMOVE-FROM $GROUND_NET_SET` |
| UNION | | `%u_set = %a_set and %b_set @UNION` |
| BUNDLE | | `#XI<3:0> is drawn as a @BUNDLE => named %xi` |
| CURRENT | in | `#M5:1 has @CURRENT("in")` |
| | out | `#:iout has @CURRENT("out")` |
| | unspecified | `#M5:2 has @CURRENT("unspecified")` |
| ASSERT-INSTANCE | | `{#M1 #M2} have @ASSERT-INSTANCE(same=["model", "w", "l"])` |
| ASSERT-TERMINAL | | `{#M1:s #M2:s} have @ASSERT-TERMINAL(same=["net"])` |
| ASSERT-NET | | `{&inp &inn} have @ASSERT-NET(port=true)` |
| WIRE on no net | | `between {%sub1::y %int1::a}, on no net &, there is a @WIRE` |
| ANNOTATION | | `at (40, 20) there is an @ANNOTATION(rotate=30) of $ARROW_SYMBOL` |
| ASSERT-USAGE | | `%dp1 is, by @ASSERT-USAGE, a usage of $DIFFERENTIAL_PAIR_N` |
| ASSERT-IN | | `{&vss} is, by @ASSERT-IN, in $GROUND_NET_SET` |
| ASSERT-TOPOLOGY | series | `{#R1 #R2 #R3} are connected as @ASSERT-TOPOLOGY("series")` |
| | parallel | `{#M3 #M4} are connected as @ASSERT-TOPOLOGY("parallel")` |
| | star | `{#L1 #C1 #L2} are connected as @ASSERT-TOPOLOGY("star")` |
| | loop | `{#C1 #L1 #C2} are connected as @ASSERT-TOPOLOGY("loop")` |

### B.9 Header record and packages

No words are added. The header record is read in the standard spelling and does not use aliases (16.5).

| Keyword | Case | Sentence |
|---|---|---|
| VERSION | | `@VERSION("0.4")` |
| ALIAS | | `@ALIAS("@位于", "@PLACE")` |
| USE-ALIASES | | `@USE-ALIASES(name="alias-zh", version="0.4")` |
| DEVICE | | `@DEVICE(…)` |
| PACKAGE | | `@PACKAGE(name="acme-style", version="1.2")` |
| USE-PACKAGE | in a header record; also a reference in the header record of a package file | `@USE-PACKAGE(name="rnl-std-all", version="0.4")` |
| | in a level | `@USE-PACKAGE(name="acme-style", version="1.2")` |
