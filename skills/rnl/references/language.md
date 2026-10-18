# RNL 0.3 in short

> The short guide to the language RNL 0.3, for reading and writing RNL files. It is not the standard, which prevails where the two differ: https://rich-netlist.github.io/rnl/en/spec/0.3/. Licence: CC-BY-4.0, see `LICENSE.md`.

RNL (Rich Netlist) is written in the comments of a SPICE or Spectre netlist. It says how the circuit is drawn as a schematic (symbols, positions, alignment, wires, texts, colours) and what its parts mean (a differential pair, a current mirror). The netlist alone defines the circuit; RNL never changes it. This guide covers language `"0.3"` and the standard package `rnl-std` `"0.3"`; section numbers such as (14.8) point into the standard.

## Terms

| Term | Meaning |
|---|---|
| record | One `@RNL(…) {…}` in comment lines, holding statements separated by `;` |
| header record | The record `@RNL(type="header") {…}`, the first record of the file |
| level | The top level of the file, or one subcircuit; each is drawn as a schematic of its own |
| set | An ordered list of objects, written `{…}` or found by a selection |
| definition | Made with DEFINE (applied with AS or ATTACH), DEFINE-SYMBOL or DEFINE-STYLE; named with `$` |
| role | A named slot of a definition: `?a` in its body, `#M1(a)` when applied |
| usage | The object one AS or ATTACH makes: which definition, which objects in which roles |
| virtual part | A drawn usage that is not in the netlist: a ground, a supply, a port tag |
| rule | A statement with a selection (SELECT-…); it applies in its level and every level below |
| fact | A position, an orientation or a constraint |
| solver | The tool that decides the free positions and writes them into a record with `type="generated"` |
| fill | The grounds, supplies and port tags that the standard package adds by itself |

## The shape of a file

```spice
* Two resistors
* @RNL(type="header") { @VERSION("0.3")}
R1 a b 1k
R2 b 0 1k
* @RNL {
*   {#R1 #R2} @ALIGN("x");
*   &b @WIRE {#R1:2 #R2:1}
* }
.end
```

```spectre
// Two resistors in Spectre
// @RNL(type="header") { @VERSION("0.3")}
simulator lang=spectre
R1 (a b) resistor r=1k
R2 (b 0) resistor r=1k
// @RNL {
//   {#R1 #R2} @ALIGN("x");
//   &b @WIRE {#R1:2 #R2:1}
// }
```

- A record is written in whole-line comments: `*` in column 1 and one space (in Spectre also `//`). Every line of a record is such a line; an empty one is `*` alone. End-of-line comments are never read (2.2).
- Line 1 is the title and is never read. The header record comes first, at the file level, at line 2 at the earliest. It holds `@VERSION("0.3")` and, when needed, DEVICE, ALIAS, USE-ALIASES and USE-PACKAGE (2.8). A file is read in the version it declares (16.1).
- A record between `.subckt … .ends` belongs to that subcircuit; outside, to the top level. References resolve in their own level. Text after `.end` is not read.
- Record head fields, all by name: `type` (`"authored"` by default, `"generated"`, `"header"`), `scope` (a subcircuit, from the top level), `priority`, `author`.

## Names and references

| Form | Meaning |
|---|---|
| `#M1`, `#` | An instance of this level; `#` alone is the level |
| `&vdd`, `&"vdd!"` | A net; quote names with other characters |
| `#M1:d`, `#M1:1` | A terminal, by name or by number from 1: the same terminal |
| `#:inp` | A port of this subcircuit |
| `%dp1` | A name given in this level |
| `$GROUND` | A definition, symbol, style or shared set |
| `?a` | A role or parameter inside a definition body |
| `%dp1::in_p`, `#M1::position` | A part of an object: a role, a local object, the origin, a text by its key |
| `(x, y)`, `(0, _)` | A point; `_` leaves an axis free |
| `{a b c}` | A set, in order |
| `&D<3:0>` | A subscript: D<3>, D<2>, D<1>, D<0> |

- Default terminal names (6.12): M `d g s b`; Q `c b e s`; J, Z `d g s`; D `a k`; R, C, L, V, I, B, F, H, W `p n`; E, G, S `p n cp cn`; T, O `p1 n1 p2 n2`; U `a b ref`. An X instance has the ports of its subcircuit. A subcircuit the netlist does not define (a PDK device) has no names: give them with DEVICE in the header record, or use numbers.
- Netlist names follow the host's case rules; RNL's own names and keywords are case-sensitive.
- Numbers have no units (`2u` is an error); enumerations are strings such as `"left-to-right"`; truth values are `true`, `false`; arrays are `[…]`. There are no expressions.
- `%#3` is the number of an object without a name; it appears only in records of the solver (5.9).

## Statements

- A statement is objects, at most one keyword with its parameters, and the definition it uses: `#M1 @SET-SYMBOL $NMOS`. ATTACH writes the definition first: `$GROUND @ATTACH &vss`.
- Parameters by position come first, then by name: `@PLACE(10, 20, priority="high")`. The brackets touch the keyword: `@PLACE (10, 20)` is PLACE and a point. Role brackets touch the object: `#M1(in_p)`.
- `=>` passes the result of a segment to the next: `$GROUND @ATTACH &vss => @PLACE(20, -4)`.
- `%x = statement` names a result; in a pipe, `… => %x` names the segment to its left. A name is bound once per level and can be used before its statement.
- A slot for a set takes loose objects as one set: `#M1 #M2 @ALIGN("y")`. A slot for one object, given a set, works on each member: `{#D1 #D2} @ORIENT("MX")`. Two slots need exactly two objects: `{#M1 #M2} @SET-SYMBOL $NMOS`.
- A set inside `{…}` is spread one level; `{{%a_set} #M3}` keeps `%a_set` as one member, counted by its centre.
- Free text between tokens means nothing, so a statement can read like a sentence: `#M1 is at @PLACE(10, 20)`. A name without its prefix is free text: `#M1 #M2 M3 @ALIGN("y")` aligns M1 and M2 only. Explanations go in `/* */`.
- Facts add up; the order of statements matters only when they conflict or override each other.

## Definitions, roles and sets

```spice
* An input pair with its tail and a mirror load
* @RNL(type="header") { @VERSION("0.3")}
.model nch nmos level=1
.model pch pmos level=1
M1 x inp tail 0 nch
M2 out inn tail 0 nch
M3 x x vdd vdd pch
M4 out x vdd vdd pch
M5 tail vb 0 0 nch
* @RNL {
*   %dp1 = #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR;
*   %cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR;
*   {#M1 #M2 #M5} @AS $MATCHED;
*   $SIDE = @DEFINE(roles=["a", "b"], parameters=[["gap", 4]]) {
*     {?a ?b} @ALIGN("y");
*     ?a ?b @DISTANCE("left-to-right", min=?gap)
*   };
*   #M3(a) #M4(b) @AS(gap=6) $SIDE
* }
.end
```

- **Roles**: each entry is a name or `[name, kind]` (`"instance"`, `"terminal"`, `"net"`, `"point"`, `"usage"`). `"{name}"` is a set role: it takes one set, and may be left out. **Parameters** are names or `[name, default]`, given in the brackets of AS or ATTACH (9.4).
- **Applying**: objects follow the role list, or all are marked with roles. A set role takes one set: `{#M1 #M2 #M5} @AS $MATCHED`. A single role given a set applies the definition once per member.
- **Body**: `?name` refers to roles and parameters; a name bound in the body, `%mid = (_, _)`, is a local object of each usage, `%u::mid` from outside. The body cannot name netlist objects.
- **A usage is an object**: it records what the objects are, whatever happens to the facts of its body (9.5).
- **Selections** give sets: `# @SELECT-INSTANCE(type="pmos")`, `&en @SELECT-TERMINAL(terminal="g")`, `# @SELECT-NET(port=true)`, `# @SELECT-USAGE $DIFFERENTIAL_PAIR`, `#M1 @SELECT-ANCESTOR-USAGE $CURRENT_MIRROR`. Conditions are by name; `name`, `model`, `net` and `key` take `*`. `model` is the `.model` name, or the subcircuit name of an X instance; `type` is the type of the `.model`; X instances have no type (10.1).

## Layout facts

| Keyword | Form | Meaning |
|---|---|---|
| PLACE | `#M1 @PLACE(10, 20)`, `@PLACE(x=10)` | The origin at (x, y); x right, y up, grid units |
| ORIENT | `#M2 @ORIENT("MY")` | `"R0"`, `"R90"`, `"R180"`, `"R270"`, `"MY"`, `"MX"`, `"MYR90"`, `"MXR90"` |
| ORIENT-RELATION | `#M1 #M2 @ORIENT-RELATION("MY")` | The second is the first turned by this transform |
| ALIGN | `{#M1 #M2} @ALIGN("y")` | One row (`"y"`) or one column (`"x"`) |
| ORDER | `{#M1 #M2} @ORDER("left-to-right")` | In order along a direction; `"x"`, `"y"` fix only the axis |
| DISTRIBUTE | `{#R1 #R2 #R3} @DISTRIBUTE("x")` | Equal spacing |
| DISTANCE | `#M3 #M1 @DISTANCE("top-to-bottom", min=4)` | From the first to the second: `min`, `max` or `exact` |
| GROUP | `{#M1 #M2 #M5} @GROUP` | One block; also where alignment, spacing and symmetry are judged (11.5) |

- Members are instances (by origin), terminals (by pin), points, texts and drawn usages; a set kept as one member counts by its centre. Nets and wires are not members.
- There is no "leftmost": write it against other parts, as `{#M6 #M7} %pout @DISTANCE("left-to-right", min=2)`.
- What RNL leaves free is decided by a solver; RNL states only what must hold.

## Priority

```spice
* Priorities
* @RNL(type="header") { @VERSION("0.3")}
.model nch nmos level=1
M1 x inp tail 0 nch
M2 out inn tail 0 nch
M5 tail vb 0 0 nch
* @RNL(priority="high") {
*   %dp1 = #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR;
*   {#M1 #M2} @ALIGN("y");
*   #M5 @PLACE(x=3, priority="low")
* }
* @RNL { %dp1 @PRIORITY("disabled")}
.end
```

- PLACE, ORIENT, ORIENT-RELATION, the four constraints, AS and ATTACH take `priority=`: `"disabled"`, `"low"`, `"normal"` (default), `"high"`. In a conflict the lower is discarded first, in a tie the earlier; a fact that is kept is hard, whatever its priority (14.8).
- The priority of a usage covers every fact of its body: `@AS(priority="high")` raises them all; PRIORITY changes it afterwards, as `%dp1 @PRIORITY("disabled")` turns the default drawing of the pair off.
- The record head's `priority` is a default for the statements that write none, but AS and ATTACH take only `"disabled"` from it: above, the pair keeps the low constraint of its body.
- The standard package writes its default drawings at `"low"`, so whatever you write wins.

## Wires and ROUTE

```spice
* Wires
* @RNL(type="header") { @VERSION("0.3")}
R1 a b 1k
R2 b 0 1k
R3 b c 1k
* @RNL {
*   %j = (0, 0);
*   &b @WIRE {#R1:2 %j #R2:1};
*   &b @WIRE {%j #R3:1};
*   $JUNCTION @ATTACH %j;
*   &a @ROUTE
* }
.end
```

- WIRE draws one wire on a net through at least two points or terminals; `mode=` is `"auto"` (default), `"straight"`, `"horizontal-vertical"`, `"vertical-horizontal"`, `"manhattan"` or `"manhattan45"` (13.1).
- Wires join only where they share a point object or a terminal; equal coordinates do not join. For a T, pass a named point and start the branch there; draw the dot with `$JUNCTION @ATTACH %j`.
- ROUTE draws a whole net as wires, a tree through every pin on it and its virtual parts (13.7).
- Several nets in the net slot make a bus: `&D<1:0> @WIRE {%a %b}`.
- Without WIRE or ROUTE a connection is not drawn as a wire.

## Texts and labels

```spice
* Labels
* @RNL(type="header") { @VERSION("0.3")}
.model nch nmos level=1
M1 x inp 0 0 nch W=2u L=0.5u
* @RNL {
*   #M1 @TEXT("${w}/${l}", key="wl", dy=-3);
*   #M1 @TEXT("Min", key="name");
*   (10, 10) @TEXT("bias", size=0.8)
* }
.end
```

- TEXT on an object is a label that moves with it; on a point it is an annotation. `key=` names it, `#M1::wl`; a text with the same key replaces the earlier one, such as `key="name"` for the name a symbol shows (13.3).
- `${…}` shows an attribute: an instance's `name`, `value`, `model`, `type` and netlist parameters such as `${w}`; the `net` of a terminal or wire; the roles of a usage; a path such as `${pair::in_p::w}` (13.6).
- Markup: `_{…}`, `^{…}`, `\overline{…}`, `\frac{…}{…}`, `\mu`, `\Omega`. Also `dx`, `dy`, `halign`, `valign`, `rotate`, and style parameters.

## Symbols and styles

```spice
* A symbol of its own
* @RNL(type="header") { @VERSION("0.3")}
X1 in out buf
.subckt buf in out
R1 in out 1k
.ends
* @RNL {
*   $TRI = @DEFINE-SYMBOL {
*     (-4, 0) @PIN(terminal="in", direction="left");
*     (4, 0) @PIN(terminal="out", direction="right");
*     @LINE {(-2, 2) (2, 0) (-2, -2) (-2, 2)};
*     @LINE {(-4, 0) (-2, 0)};
*     @LINE {(2, 0) (4, 0)};
*     ? @TEXT("${name}", key="name") => @PLACE(-2, 2.5)
*   };
*   #X1 @SET-SYMBOL $TRI;
*   $orange = @DEFINE-STYLE(linecolor="#e07000", textcolor="#e07000") {};
*   #X1 @SET-STYLE $orange
* }
.end
```

- A symbol body uses local coordinates about the object's origin. `@PIN(terminal=…, direction=…)` puts a terminal's pin; a terminal without a pin is not drawn, and a wire to it fails (9.2.4). `? @TEXT(…)` labels the object drawn with the symbol.
- A symbol given to an object directly beats those given by rules; among the same kind, the later wins, and the rules of the standard package come first. Rules set symbols for many objects: `# @SELECT-INSTANCE(model="nch") => @SET-SYMBOL $NMOS`.
- Styles merge by property: `linecolor`, `linestyle`, `linewidth`, `fillcolor`, `textcolor`, `size`, `font`, `bold`, `italic` and more (9.3). Colours are `"#RRGGBB"`. Virtual parts are objects of their own; style them by a rule such as `# @SELECT-USAGE $GROUND => @SET-STYLE $grey`.
- `# @SET-SYMBOL %blk` inside a subcircuit gives it a block symbol for its instances above (9.2.6).

## The standard package

### What it brings

A file uses `rnl-std` `"0.3"` implicitly: the styles, the symbols of `rnl-std-basic` and `rnl-std-semiconductors`, the symbol rules of `rnl-std-spice` or `rnl-std-spectre` by the dialect, the fill (`rnl-std-fill`) and the structures (`rnl-std-structures`). `$OPAMP` and other blocks are in `rnl-std-functional`, named in the header record: `@USE-PACKAGE(name="rnl-std-functional", version="0.3")`; `rnl-std-full` brings every package. An empty package drops one: `@USE-PACKAGE(name="rnl-std-fill") {}` (16.3).

### Default symbols and pins

R `$RESISTOR`, C `$CAPACITOR`, L `$INDUCTOR`, V `$VOLTAGE_SOURCE`, I `$CURRENT_SOURCE`, D `$DIODE`, E `$VCVS_4TERM`, G `$VCCS_4TERM`, F `$CCCS`, H `$CCVS`, B `$BEHAVIORAL_SOURCE`, S `$VOLTAGE_CONTROLLED_SWITCH`, W `$SWITCH`, J and Z `$NJFET`, T and O `$TRANSMISSION_LINE`, U `$RC_LINE`. Transistors follow the type of their `.model`: nmos `$NMOS_4TERM`, pmos `$PMOS_4TERM`, npn `$NPN`, pnp `$PNP`, njf `$NJFET`, pjf `$PJFET`; a transistor whose `.model` card is not found gets no symbol and is not drawn, so write the card, or set the symbol yourself.

| Symbols | Pins at R0 |
|---|---|
| two-terminal parts and sources | 1 at `(0, 2)`, 2 at `(0, −2)` |
| `$DIODE` | a `(−2, 0)`, k `(2, 0)` |
| `$NMOS`, `$NMOS_4TERM` | d `(0, 2)`, g `(−3, 0)`, s `(0, −2)`, b `(1, 0)` |
| `$PMOS`, `$PMOS_4TERM` | s `(0, 2)`, g `(−3, 0)`, d `(0, −2)`, b `(1, 0)` |
| `$NPN` / `$PNP` | b `(−3, 0)`; c on top for NPN, e on top for PNP |
| `$VCVS_4TERM`, `$VCCS_4TERM` | 1 `(0, 2)`, 2 `(0, −2)`, 3 `(−4, 1)`, 4 `(−4, −1)` |
| `$GROUND_SYMBOL`, `$SUPPLY_SYMBOL`, `$IO_PORT_SYMBOL` | one pin at the origin |

### Definitions

| Name | Roles | Use |
|---|---|---|
| `$GROUND`, `$SUPPLY`, `$IO_PORT` | p (net or terminal) | `$GROUND @ATTACH &vss`; on a terminal, drawn on the pin |
| `$NO_CONNECT`, `$JUNCTION`, `$CURRENT_ARROW` | t; p; p | A no-connect mark, a junction dot, a current arrow |
| `$GROUND_STUB`, `$SUPPLY_STUB` | t; gap (2) | A ground below, a supply above the terminal, with its wire, low |
| `$DIFFERENTIAL_PAIR` | in_p, in_n, {tail} | Its body puts the tail under the middle, low |
| `$CURRENT_MIRROR` | {ref}, {out} | One usage for any number of outputs |
| `$MATCHED`, `$BRANCH` | {members} | Matched devices; one branch, in one column, low |
| `$CHAIN`, `$CASCADE` | source, {chain}, load; {first}, {second}, {feedback}, {load} | A signal chain, left to right, low; a cascade |
| `$MIRRORED` | a, b, axis | a and b at one height, mirror images about the axis point, a on the left, low |
| `$FLOW` | {path}; direction | The path in that direction, low |

### The fill

Every unwired terminal on a net of `$GROUND_NET_SET` (net 0) gets a `$GROUND_STUB`, every one on `$SUPPLY_NET_SET` (empty) a `$SUPPLY_STUB`, and every port of a subcircuit, but ground and supply nets, a port tag (10.5). Add your nets: `&vss @ADD-TO $GROUND_NET_SET`, `&vdd @ADD-TO $SUPPLY_NET_SET`; take a net out with `{&vb} @REMOVE-FROM $PORT_FILL_SET`. On request: `$LABEL_TERMINAL_SET @TEXT("${net}", key="netname")` labels unwired terminals, `$WIRE_NET_SET @ROUTE` draws the other nets as wires.

## Aliases and rnl-zh

```spice
* The same pair in Chinese
* @RNL(type="header") { @VERSION("0.3"); @USE-ALIASES(name="rnl-zh")}
.model nch nmos level=1
M1 x inp tail 0 nch
M2 out inn tail 0 nch
* @RNL {
*   #M1(正输入) #M2(负输入) @视作 $"差分对";
*   {#M1 #M2} @对齐("y")
* }
.end
```

- USE-ALIASES in the header record loads an alias package; `rnl-zh` gives Chinese spellings of the keywords and of the names of the standard package (16.5). The standard spellings keep working. ALIAS declares one alias: `@ALIAS("@并排", "@ALIGN")`. A `$` name in Chinese is quoted, as `$"差分对"`.

## Records of the solver

```spice
* Solved, then dragged
* @RNL(type="header") { @VERSION("0.3")}
R1 a b 1k
R2 b 0 1k
* @RNL { {#R1 #R2} @ALIGN("x")}
* @RNL(type="generated", priority="high", author="rnl-solver") { #R1 @PLACE(0, 4); #R2 @PLACE(0, -4)}
* @RNL(author="editor") { #R2 @PLACE(y=-6, priority="high")}
.end
```

- A solver writes what it decides into one record with `type="generated"` at the end of the level. When the level is solved again, that record is left out and replaced: do not edit it (2.8, 14.8).
- To fix a position, write a PLACE with `"high"` in a later record of the same level, as above; of the two high facts the earlier one is discarded.
- Objects without a name appear there as numbers, `%#3`, checked by the field `numbering`. Never write numbers in your own records; name the object instead.

## Common mistakes

| Written | Reported | Write instead |
|---|---|---|
| `#M1 @PLACE (10, 20)` | `PLACE takes x, y or both` | `#M1 @PLACE(10, 20)` |
| `#M1 (in_p) #M2 (in_n) @AS …` | `role parentheses touch the object` | `#M1(in_p) #M2(in_n) @AS …` |
| `#M1 #M2 @SET-SYMBOL $NMOS` | `@SET-SYMBOL has 2 slots, and 3 objects are written` | `{#M1 #M2} @SET-SYMBOL $NMOS` |
| `#M5 #M3 @AS $MATCHED` | `$MATCHED has 1 roles, 2 objects are written` | `{#M5 #M3} @AS $MATCHED` |
| `#M1 @PLACE(1, 2), #M2 @PLACE(3, 4)` | `at most one keyword in a segment` | `;` between statements |
| `#M1 @SET-SYMBOL($NMOS)` | `parameters hold literals only` | `#M1 @SET-SYMBOL $NMOS` |
| `#R1 @PLACE(x=2u)` | `a number takes no unit or suffix` | `@PLACE(x=2)` |
| `$GND @ATTACH &vss` | `$GND cannot be found: the standard series 0.3 calls it $GROUND` | `$GROUND @ATTACH &vss` |
| `#R1 @SET-SYMBOL $GROUND` | `must be a symbol, not $GROUND` | the symbol is `$GROUND_SYMBOL` |
| `$OPAMP` without its package | `$OPAMP cannot be found: … rnl-std-functional` | USE-PACKAGE of `rnl-std-functional` |
| `%#3 @PLACE(1, 2)` in your record | `a number is written only in the record of the solver` | give the object a name |
| a wire to the bulk of `$NMOS` | `#M1:b has no pin in its symbol: the wire fails` | `$NMOS_4TERM` |
| `M1 d g 0 0 nch` without `.model nch` | `the model card nch is not found, so the polarity of M1 is not known and it is not drawn` (a warning) | `.model nch nmos level=1`, or `#M1 @SET-SYMBOL $NMOS_4TERM` |
| `#M1.d`, `M3`, `&vdd!` | nothing: they are free text | `#M1:d`, `#M3`, `&"vdd!"` |

```spice error
* A space before the bracket
* @RNL(type="header") { @VERSION("0.3")}
R1 a 0 1k
* @RNL { #R1 @PLACE (10, 20)}
.end
```

```spice error
* A name of the series 0.2
* @RNL(type="header") { @VERSION("0.3")}
R1 a vss 1k
* @RNL { $GND @ATTACH &vss}
.end
```

## Before you hand a file back

- The header record is the first record, at line 2 or later, and declares `@VERSION("0.3")`.
- Every line of every record is a whole-line comment with the comment character in column 1.
- Every `#`, `&` and terminal name exists in the netlist of its level; terminal names fit the element (`#M1:d`, `#Q1:c`).
- Only names of the standard series `"0.3"`: `$GROUND`, `$DIFFERENTIAL_PAIR`, `$RESISTOR`, `$MIRRORED`.
- Brackets touch their keyword or object; statements end with `;`; numbers have no units.
- Every set role gets a set; two-slot keywords get exactly two objects.
- Nothing in `type="generated"` records is edited, and no `%#` number is written elsewhere.
- Nets you treat as grounds and supplies are added to `$GROUND_NET_SET` and `$SUPPLY_NET_SET`.
- Transistors have `.model` cards or explicit symbols.

## A complete example

```spice
* Five-transistor OTA
* @RNL(type="header") { @VERSION("0.3")}
.model nch nmos level=1
.model pch pmos level=1
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M3 x    x     vdd  vdd pch W=4u L=0.5u
M4 out  x     vdd  vdd pch W=4u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   &vss @ADD-TO $GROUND_NET_SET;
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   %dp1 = #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR;
*   %cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR;
*   %ax = (_, _);
*   #M1(a) #M2(b) %ax(axis) @AS $MIRRORED;
*   #M3(a) #M4(b) %ax(axis) @AS $MIRRORED;
*   #M3 #M1 @DISTANCE("top-to-bottom", min=4);
*   #M1 #M5 @DISTANCE("top-to-bottom", min=4);
*   {#M1 #M2 #M3 #M4 #M5} @GROUP;
*   {#M1 #M2} @TEXT("${w}/${l}", key="wl", dy=-3);
*   #M5:d @CURRENT("in");
*   #M5:s @CURRENT("out")
* }
.ends ota5t
.end
```

- The structures say what the circuit is; their bodies, at low priority, put the tail under the middle and each mirrored pair about the axis. The two DISTANCEs put the loads above the pair and the tail below it.
- CURRENT states the direction of the static current at a terminal; it draws nothing (15.2).
- The fill adds the grounds, supplies and port tags; what is left free is for a solver to decide.
