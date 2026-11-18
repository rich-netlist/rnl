# RNL 0.4 in short

> The short guide to the language RNL 0.4, for reading and writing RNL files. It is not the standard, which prevails where the two differ: https://rich-netlist.github.io/rnl/en/spec/0.4/. Licence: CC-BY-4.0, see `LICENSE.md`.

RNL is written in the comments of a SPICE or Spectre netlist and says how the circuit is drawn and what its parts mean; it never changes the circuit. Language `"0.4"`, standard package series `"0.4"`; numbers such as (14.8) are sections of the standard. Examples are independent recipes, not one program: use existing objects in their own level and put snippets inside RNL records. `/* */` explains them. The catalogue at the end lists every name, as signatures, not RNL to copy.

## A file

```spice
* Two resistors                                   /* line 1 is the title, never read */
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
R1 a b 1k
R2 b 0 1k
* @RNL {
*   {#R1 #R2} @ALIGN("x"); /* every statement ends with ; */
*   &b @WIRE {#R1:n #R2:p};
* }
.end
```

- Records are whole-line comments, `*` in column 1 and a space; `//` too, in Spectre mode (a `.scs` file, or after `simulator lang=spectre`). The header record comes first. A record inside `.subckt … .ends` belongs to that subcircuit.
- Without the USE-PACKAGE no standard name is known. A header record you are given stays as it is; its `@DEVICE` lines give PDK devices their terminal names and types (6.12), and you do not write or change them.
- Write an ASCII space before every `@`; parameter brackets touch the keyword, role brackets the object. Positional parameters precede named ones: `@PLACE(10, y=20)`. Numbers have no units; there are no expressions. RNL names and keywords are case-sensitive; netlist names follow the host.

## References

```rnl
#M1; /* an instance of this level; # alone is the level */
&vdd; &"vdd!"; /* nets; quote other characters */
#M1:d; #M1:1; /* a terminal, by name or number: the same one */
#M1::d; /* the pin of M1's symbol that draws terminal d */
#:inp; /* a port of this subcircuit */
%dp1; $GROUND; /* a name of this level; a definition, symbol, style or shared set */
%g::pin; %dp1::left; /* a part: the pin of a ground, the left role of a pair */
(10, 20); (0, _); /* points; _ leaves an axis free */
{#M1 #M2 #M3}; /* a set, in order */
&D<3:0>; /* D<3> D<2> D<1> D<0> */
%row_set{1}; %row_set{end}; %row_set{2:end}; /* members of a written set by place */
```

- Terminal names: M `d g s b`; Q `c b e s`; J Z `d g s`; D `a k`; R C L V I B F H W `p n`; E G S `p n cp cn`; T O `p1 n1 p2 n2`; U `a b ref`; X the subcircuit's ports. Where a pin is needed, a terminal stands for its pin.
- Names bind once per level and may be used before their definition. To move `%p`, write `%p @PLACE(...)`, not another `%p = ...`. `%` names are local; `$` definitions are inherited by lower levels, where another definition may hide them.
- A word without a prefix is free text (a warning if it names something): `#M1 #M2 M3 @ALIGN("y");` aligns M1 and M2 only.

## Statements and pipes

```rnl
#M1 @PLACE(10, 20); /* objects, one keyword, its parameters */
$GROUND @ATTACH &vss => @PLACE(20, -4); /* => passes the result on: the ground, not the net */
%g = $GROUND @ATTACH &vss; /* %x = names the result */
$GROUND @ATTACH &vdd => %g2 => @PLACE(20, 4); /* => %x names the segment before it */
#M1 #M2 @ALIGN("y"); /* loose objects are one set */
{#D1 #D2} @ORIENT("MX"); /* a one-object slot given a set does each member */
{#M1 #M2} @SET-SYMBOL $NMOS; /* two slots: exactly two objects */
{#R1 {#R2 #C2} #M1} @ORDER("left-to-right"); /* a set as one item stands for each member: R2 and C2 each between */
```

What a segment passes on: PLACE, ORIENT, SET-SYMBOL, SET-STYLE, GROUP, CURRENT their input; AS, ATTACH the new usage (a set of them, one by one); ALIGN, ORDER, DISTRIBUTE, DISTANCE, TABLE, ORIENT-RELATION the new constraint; TEXT the new text; SELECT-… the set selected.

## Definitions, roles, usages

```rnl
$SIDE = @DEFINE(roles=["a", "b"], parameters=[["gap", 4]]) {
  {?a ?b} @ALIGN("y"); /* ?a: a role in the body */
  ?a ?b @DISTANCE("left-to-right", min=?gap); /* ?gap: a parameter */
};
#M3(a) #M4(b) @AS(gap=6) $SIDE; /* apply with roles marked, and a parameter */
#M3 #M4 @AS $SIDE; /* mark every role or none; without marks, use role order */
%dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N; /* every one-object role is filled: tail too */
{#M5 #M3} @AS $MATCHED; /* a set role takes the whole set: one usage */
{#M1 #M2} @AS $DIODE_CONNECTED; /* a one-object role given a set: one usage each */
$KIND = @DEFINE(roles=[["p", "net"], ["{at}", "point"]]) {}; /* common role kinds: instance terminal pin net point usage */
{%dp1::left #M9} @ALIGN("y"); /* place a structure through a member */
%g = $GROUND @ATTACH &vss; {%g::pin #M5::s} @ALIGN("x"); /* a virtual part is drawn at its own point; place it by its pin */
```

- A `{role}` takes the whole set (or one object as a set of one) and may be omitted; unmarked arguments may omit only trailing set roles. A single-object role is required. Mark each role at most once; put several members in one set: `{#R1 #R2}(members)`.
- In a body, `?` alone is the usage's own point, coordinates are offsets from it, and `%mid = (_, _);` makes a local object, `%u::mid` from outside.
- The standard structures that place tie their usage's own point: a row or a column of members has it on its line, two mirrored sides at the centre between them, so that a constraint on `%u::position` moves the structure. The column *Draws* of the catalogue says where: `$BRANCH` shares only the members' x with it, `$BRANCH_HORIZONTAL` only their y, `$PARALLEL_HORIZONTAL` and `$MIRRORED_LAYOUT` hold it at their centre. Any other structure leaves it free, and a constraint on its `::position` changes nothing drawn.
- Prefer structures that both say what the parts are and draw them. A `$BRANCH` is a series structure stacked vertically, its members from the top down; its usual use is a DC branch, such as a path from a supply to ground. Write each such path as one `$BRANCH`, and a parallel pair on it with a variant that places: `{#R1 #M1 #R2} @AS $BRANCH; {#R2 #C2} @AS $PARALLEL_HORIZONTAL;`. `$PARALLEL` alone only routes. A branch says nothing about current, which `@CURRENT` writes. A signal path through switches and capacitors, as in a switched-capacitor sampler, is drawn with atomic facts, such as an ALIGN and an ORDER on the pins of each row: `{#S1:s #S1:d #C1:p #C1:n} @ALIGN("y"); {#S1:s #S1:d #C1:p #C1:n} @ORDER("left-to-right");`. A *meaning only* structure draws nothing: write it where it is true, never in place of one that places. Mark a `$SIGNAL_FLOW` only on a clean signal path you are sure of; leave it out where the signal takes several paths, feeds back, or runs both ways through much of the circuit, however few or many members the path has.

## Selections and rules

```rnl
# @SELECT-INSTANCE(model="nch", recursive=true) => @SET-STYLE $grey; /* a rule: here and in the levels below */
# @SELECT-INSTANCE(type="pmos") => @ORIENT("MY"); /* without recursive=true: this level only */
&en @SELECT-TERMINAL(terminal="g"); /* the gate terminals on net en */
# @SELECT-PIN(wired=false); /* pins without wires */
# @SELECT-NET(port=true); # @SELECT-NET(global=true); # @SELECT-NET(wired=false);
# @SELECT-USAGE $DIFFERENTIAL_PAIR_N => @SELECT-INSTANCE(role="left"); /* the left device of every pair */
&vss @ADD-TO $GROUND_NET_SET; /* edit a shared set of the package */
&agnd @REMOVE-FROM $GROUND_NET_SET;
```

- `name`, `model`, `net` and `key` take `*`. `type` is the type of the `.model` card, or of a DEVICE.
- A selection result, and a set built from one, takes no places (`%sel{1}`) and goes to no WIRE; name the object, or write ROUTE-OBJECT.

## Layout facts

```rnl
#M1 @PLACE(10, 20); #M1 @PLACE(x=10); /* origin at (x, y); x right, y up, grid units */
#M2 @ORIENT("MY"); #M5 @ORIENT(["R0", "MY"]); /* an object takes a code; a list lets the solver choose */
#M1 #M2 @ORIENT-RELATION("MY"); /* M2 is M1 flipped left-right */
{#M1 #M2} @ALIGN("y"); /* one row; "x" is one column */
{#M1 #M2 #M3} @ORDER("left-to-right"); /* one after the other, at least 1 apart */
{#R1 #R2 #R3} @DISTRIBUTE("top-to-bottom"); /* in order, equal steps */
#M3 #M1 @DISTANCE("top-to-bottom", min=4); /* from the first to the second: min, max or exact, none negative */
{#M1 #M2 #M5} @GROUP; /* groups the set for the solver; no own origin or rigid transform */
@TABLE layout: {{#M1 #M2 #M3} {#M4 {#M5 #M6} #M7}}; /* rows top down, cells left to right, a cell column inside; one group; no alignment */
@TABLE(transpose=true) layout: {{#M1 #M2} {#M3 #M4}}; /* columns left to right */
```

- Directions: `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`. Codes: `R0`, `R90`, `R180`, `R270` turn anticlockwise; `MY` flips left-right, `MX` upside down; `MYR90`, `MXR90` mirror, then turn.
- Members are instances by origin, terminals and pins by pin, points, texts, usages with a symbol by their own point. A usage without a symbol stands for its members, down to the lowest, as in GROUP; its own point is `%u::position`. Sets have no origin: PLACE places each member, even after GROUP. PLACE does not place pins or constraints. There is no "leftmost": write it against other parts.
- The solver decides what is left free and guesses nothing about what a part is: write what must hold. Prefer meaning and constraints and let the solver place; when the task asks for coordinates, write PLACE, ORIENT and WIRE for everything instead. Do not mix the two: either meaning and constraints for the solver with few or no PLACE, or every position written, with no solve.
- Order the large pieces, not single devices: `{#V2 #C1 %bias %stage #C3 #R5} @ORDER("left-to-right");` sets whole columns side by side. A structure stands for its members, so every device of `%stage` lies between every device of `%bias` and C3; a device that two structures share is not ordered against itself. To constrain the centre of a structure instead, write its own point: `{%dp::position #X1} @ALIGN("y");` puts X1 at the height of the centre of the pair, where `{%dp #X1} @ALIGN("y");` would put the three transistors at X1's height (column *Draws* of the catalogue says where a structure ties its point).
- Parts around the circuit, such as the supplies and the signal sources, usually stand on one side of it, a little away from the rest; a DISTANCE says so: `#VDD {#VIN #XU1 #CL} @DISTANCE("left-to-right", min=4);` keeps the supply at least 4 units left of each of the others.
- Give the drawing a clear tree of groups: each functional block a group, groups nested in larger ones, and about five or six things directly in each node. Many structures are groups already (column *Group*); one that may sit partly inside another, such as a current mirror, is not. Frame whole structures, or a stage with its ports, yourself: `%amp %p_inp %p_inn %p_out @GROUP;`. Groups do not cross: of two that share members without nesting, the solver keeps one as a block.
- Fix orientations only where the reader expects one that nothing else gives, usually the signal source, the supply sources and the load: `{#V1 #V2 #R5} @ORIENT("R0");`. Leave the rest to the solver; the structures and the current settle most of them.

## Pins and directions

```rnl
#M1::d @ORIENT("up"); /* a pin takes a direction: left down right up */
#M1::g @ORIENT("left"); /* drain up and gate left: M1 is R0, nothing else */
#M3::s @ORIENT("up"); /* a PMOS upright */
#M1::g #M2::g @ORIENT-RELATION("R180"); /* two pins face opposite ways */
{#M3::d #M1::d} @ALIGN("x"); /* stack: the drains in one column... */
&x @WIRE {#M3:d #M1:d}; /* ...and wired, or the fill labels them */
%t = $IO_PORT @ATTACH &inp; %t::pin @ORIENT("right"); {%t::pin #M1::g} @ALIGN("y"); &inp @WIRE {%t::pin #M1::g};
$IO_PORT_STUB @ATTACH #M1:g; /* the same tag, put on the pin by the package */
```

- One fixed pin leaves two orientations of eight: with only the gate, upright or upside down; with only the drain, gate left or right. Fix two pins.
- Standard pins at R0: R C L sources p up, n down; `$DIODE` a up, k down; `$NMOS` d up, g left, s down, b right (4TERM); `$PMOS` s up, g left, d down; `$NPN` c up, b left, e down; `$PNP` e up, b left, c down.

## Current

```rnl
#M1:d @CURRENT("in"); #M1:s @CURRENT("out"); /* the static current at a terminal: in, out, unspecified */
{#M1:d #M2:d #M5:d} @CURRENT("in"); /* a set does each member */
#:out @CURRENT("in"); /* at a port: current enters the subcircuit */
#M1::d @CURRENT("in"); /* a pin stands for its terminal */
```

- It draws nothing; it says what the circuit does, and the solver lays the current paths out by it. Write it on the main paths.
- Write it only where a static current flows and its direction is sure: capacitors, gates and sources without a DC value carry none, so leave them out, and leave out a terminal when unsure. Leave supply sources unmarked too: inside them the current runs up, from n to p, and a solver that prefers current running down would turn them over.
- A net whose terminals and ports all have directions moving current one way gives a violation; global nets such as 0 are not checked.

## Priority

```rnl
#M5 @PLACE(x=3, priority="low"); /* disabled, low, normal (default), high */
%dp1 = #M1(left) #M2(right) #M5(tail) @AS(priority="high") $DIFFERENTIAL_PAIR_N; /* covers every fact of the body */
%dp1 @PRIORITY("disabled"); /* turns the default drawing of the pair off */
```

- In a conflict the lower goes first, in a tie the earlier; a fact kept is hard. The standard package draws at `"low"`. A record head's `priority=` is a default, but AS and ATTACH take only `"disabled"` from it.
- A structure needs no priority: write one only to settle a conflict that a check or a solve has shown.

## Wires and routing

```rnl
%j = (_, _);
&b @WIRE {#R1:n %j #R2:p}; /* wires join only at shared point objects and terminals */
&b @WIRE(mode="manhattan") {%j #R3:p}; /* auto straight horizontal-vertical vertical-horizontal manhattan manhattan45 */
$JUNCTION @ATTACH %j; /* the dot is written */
&a @ROUTE-NET; /* the solver draws the whole net */
{#R3 #R4} @ROUTE-OBJECT; /* the connections among these objects' pins */
#R3:p @UNROUTE-OBJECT; &x @UNROUTE-NET; /* take pins out of trees; on one pin the higher priority, then the later, wins */
&vss @ROUTE-NET(priority="high"); $GROUND @ATTACH &vss; /* ground and supply nets are unrouted at high: wire them so, and attach a ground */
&D<1:0> @WIRE {%a %b}; /* several nets: a bus */
%all = # @SELECT-NET; %all @ROUTE-NET; /* every net of this level as wires; the fill keeps the ground, supply and bias nets unrouted at high, so they keep their stubs */
```

- Equal coordinates do not connect pins. Write a WIRE between them, even when its length is zero, to mark them wired and avoid extra fill.
- The structures draw the wires inside them at low. What joins two structures is a net label unless you write a ROUTE.
- Wire what reads better as a wire, not everything: the solver draws only what WIRE, ROUTE-NET and ROUTE-OBJECT ask for, and the fill gives every other unwired pin a ground, supply or net-label stub. Route the main signal path and the connections within and between neighbouring structures; leave long or crowded connections, such as bias lines and nets shared across the drawing, to labels.
- In a small circuit every net but the rails usually reads well as wires: route them as in the last line above. `# @SELECT-NET => @ROUTE-NET;` draws the same, since the fill unroutes ground, supply and bias nets at high anyway, but each rail net then gets a hint that its tree lost all its pins.

## Texts

```rnl
#M1 @TEXT("${w}/${l}", key="wl", dy=-3); /* a label that moves with M1; #M1::wl */
#M1 @TEXT("M_{in}", key="name"); /* the same key replaces the name the symbol shows */
(10, 10) @TEXT("bias", size=0.8); /* on a point: an annotation */
#M1 @TEXT("\overline{EN} \mu A \frac{W}{L}", halign="middle", valign="top", rotate=90);
```

- `${…}`: `name`, `value`, `model`, `type`, netlist parameters such as `w`; `net`; roles; paths such as `${pair::left::w}`.
- `dx`, `dy` are offsets from what the text is on. `#M1::wl @PLACE(5, 0);` puts the label at (5, 0) of the drawing; on each axis a PLACE and an offset replace each other, the later wins. In a symbol body both are in the symbol's own coordinates.

## Symbols and styles

```rnl
$TRI = @DEFINE-SYMBOL {
  %in = (-4, 0) @PIN(terminal="in", direction="left"); /* a named pin: where, which terminal, which way */
  %out = (4, 0) @PIN(terminal="out", direction="right");
  @LINE {(-2, 2) (2, 0) (-2, -2) (-2, 2)};
  (0, 0) @RECTANGLE(width=4, height=2); (0, 0) @CIRCLE(radius=1);
  ? @TEXT("${name}", key="name") => @PLACE(-2, 2.5); /* ? is the object drawn */
};
#X1 @SET-SYMBOL $TRI; /* terminal names match by name, numbers by place */
#X2 @SET-SYMBOL(terminals=["inp", "inn", "out"]) $OPAMP; /* names differ: the symbol's names in node order */
# @SELECT-INSTANCE(model="nch", recursive=true) => @SET-SYMBOL $NMOS; /* by rule; a direct SET-SYMBOL wins */
%blk = @DEFINE-SYMBOL { %i = (-3, 0) @PIN(terminal="in", direction="left"); (0, 0) @RECTANGLE(width=6, height=3); };
# @SET-SYMBOL %blk; /* inside a .subckt: its block symbol, for its instances above */
$orange = @DEFINE-STYLE(linecolor="#e07000", textcolor="#e07000") {};
# @SELECT-INSTANCE => @SET-STYLE $grey; #M1 @SET-STYLE $orange; /* M1 is orange: direct beats rule, per property */
# @SELECT-USAGE(recursive=true) $GROUND => @SET-STYLE $grey; /* virtual parts are objects of their own */
```

- Style properties: `linecolor linestyle linewidth fillcolor textcolor size font bold italic textvisible`; colours `"#RRGGBB"`. A `%` name may also name a local definition. For each property, direct settings beat rules; within one tier, the later setting wins.

## Checks

The standard structures check what they claim: pair halves of one model and size, a series really in series, a pmos role holding a PMOS, a port tag on a port. A check that fails is a **violation**: a report; nothing fails and the drawing is the same. Fix the structure or the roles, or take a structure that claims less.

## The standard package

```rnl
@USE-PACKAGE(name="rnl-std-all", version="0.4"); /* everything: styles, symbols, rules for SPICE and Spectre, fill, structures, the blocks */
@USE-PACKAGE(name="rnl-std-no-labels", version="0.4"); /* optional, besides rnl-std-all: hides the labels symbols draw (name, value, model, net names); a label you write shows only with textvisible=true */
@USE-PACKAGE(name="rnl-std-45deg", version="0.4"); /* optional: symbols at 45 degrees */
@USE-PACKAGE(name="rnl-std-fill") {}; /* an empty package drops that part */
&vdd @ADD-TO $SUPPLY_NET_SET; &vss @ADD-TO $GROUND_NET_SET; /* supply and ground stubs on their pins; 0 is a ground already */
$PORT_LIST @ATTACH; /* a port tag on every port net, in one column */
$FIGURE @ATTACH %p => @SET-SYMBOL $MY_FIG; /* no netlist instance: use a symbol with unbound pins and no ${name}/${value}; not $RESISTOR */
```

- Default symbols: R `$RESISTOR`, C `$CAPACITOR`, L `$INDUCTOR`, V `$VOLTAGE_SOURCE`, I `$CURRENT_SOURCE`, D `$DIODE`, E `$VCVS_4TERM`, G `$VCCS_4TERM`, F `$CONTROLLED_CURRENT_SOURCE`, H `$CONTROLLED_VOLTAGE_SOURCE`, B `$BEHAVIORAL_SOURCE`, S `$VOLTAGE_CONTROLLED_SWITCH`, W `$SWITCH`, T O `$TRANSMISSION_LINE`, U `$RC_LINE`; transistors by type: nmos `$NMOS_4TERM`, pmos `$PMOS_4TERM`, npn `$NPN`, pnp `$PNP`, njf `$NJFET`, pjf `$PJFET`; an NMOS with its bulk on a ground net and a PMOS with its bulk on a supply net get the three-pin `$NMOS` and `$PMOS` instead, with no SET-SYMBOL of yours. Without a type (no `.model`, no DEVICE) a transistor gets no device symbol and is drawn as a box with a pin for each node (9.2.8).
- Put each supply net in `$SUPPLY_NET_SET` and each ground other than 0 in `$GROUND_NET_SET`, as above: their unwired pins get stubs and they stay unrouted. The fill skips pins with wires or no-connect marks; every other unwired pin, of a device or of a virtual part, gets a `$GROUND_STUB` on a net of `$GROUND_NET_SET` (net 0), a `$SUPPLY_STUB` on one of `$SUPPLY_NET_SET` (empty), a `$BIAS_STUB` on one of `$BIAS_NET_SET` (empty), a `$NET_LABEL_STUB` elsewhere; the bulk pin of a four-terminal MOS gets a `$NET_LABEL_STUB` on any net. Every port net gets an `$IO_PORT`, supply and ground nets too, in one column at least 6 units left of the level's instances, each at R0; take a net out with `&vdd @REMOVE-FROM $PORT_FILL_SET;`, or write `$IO_PORT @ATTACH &out;` to place a tag yourself. A tag shows the name of its net itself, as its text `netname`: write no TEXT of the name on it, and move the name with `%t::netname @PLACE(x, y);`. For a test bench name, use `$NET_LABEL`. A pin that already has an `$IO_PORT_STUB` or a `$NET_LABEL_STUB` gets no net label either, though their leads are drawn by the symbol, not by a WIRE. The `*_FILL_SET` sets are for the fill's own stubs and tags: a solve writes those back, and the sets then no longer hold their pins and nets, so build no other rule on them, such as a TEXT on their pins; select the pins yourself. To change which pins get a stub, add pins to these sets or remove them; a stub that a rule of yours attaches is not seen by the fill, which adds its own as well.

## Records of the solver

```spice fragment
* @RNL(type="generated", priority="high", author="rnl-solver") { #R1 @PLACE(0, 4); #R2 @PLACE(0, -4);} /* never edit; replaced on each solve */
* @RNL(author="editor") { #R2 @PLACE(y=-6, priority="high");} /* to fix a position: high, in a later record */
```

`%#3` numbers appear only there; name objects instead.

## Other keywords

Written by packages and tools; read them, you rarely write them:

```rnl
@VERSION("0.4"); @PACKAGE(name="my-pkg", version="1.0"); /* header records */
@DEVICE(model="sky130_fd_pr__nfet_01v8*", terminals=["d", "g", "s", "b"], type="nmos"); /* names and types of devices */
@ALIAS("@at", "@PLACE"); @USE-ALIASES(name="alias-zh"); /* other spellings */
@SOLVER-CONFIG(spacing_x=2, junction="$JUNCTION"); /* settings of the solver */
#XI<3:0> @BUNDLE; /* an array of instances drawn as one */
#X @LAYER("bottom"); /* drawing layer */
%u_set = %a_set %b_set @UNION; /* a set without repeats */
(0, 0) @ELLIPSE(width=4, height=2); @ARC {(0, 2) (2, 0) (0, -2)}; @BEZIER {(0, 0) (1, 1) (2, 1) (3, 0)}; /* more graphics */
&n1 @SELECT-WIRE(bus=true); #M1 @SELECT-LABEL(key="wl"); # @SELECT-ANNOTATION; #M1 @SELECT-ANCESTOR-USAGE $CURRENT_MIRROR_N;
{#M1 #M2} @ASSERT-INSTANCE(same=["model", "w", "l"]); {#M1:s #M2:s} @ASSERT-TERMINAL(same=["net"]);
{&inp &inn} @ASSERT-NET(port=true); {#R1 #R2 #R3} @ASSERT-TOPOLOGY("series"); /* the checks inside structures */
```

## Mistakes

The left column is invalid or misleading; copy only the right-hand repair, with names from your file.

|Mistake|Repair|
|---|---|
|`#M1 @PLACE (10, 20);`|`#M1 @PLACE(10, 20);`|
|`#M1 (left) #M2 (right) @AS $CROSS_COUPLED_PAIR;`|`#M1(left) #M2(right) @AS $CROSS_COUPLED_PAIR;`|
|`#M1 #M2 @SET-SYMBOL $NMOS;`|`{#M1 #M2} @SET-SYMBOL $NMOS;`|
|`#M5 #M3 @AS $MATCHED;`|`{#M5 #M3} @AS $MATCHED;`|
|`{#R1 #R2} @ORDER("x");`|`{#R1 #R2} @ORDER("left-to-right");`|
|`#R1 @PLACE(x=2u);`|`#R1 @PLACE(x=2);`|
|0.3 names `$DIFFERENTIAL_PAIR`, roles `in_p/in_n`|`#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;`|
|`#R1 @SET-SYMBOL $GROUND;`|`$GROUND @ATTACH &n;` for a ground; keep the resistor's symbol|
|`%t #M1 @ALIGN("y");` to align their pins|`{%t::pin #M1::g} @ALIGN("y");`|
|`$IO_PORT @ATTACH &n;` on a non-port net|`$NET_LABEL @ATTACH &n;`|
|`&vss @ROUTE-NET;` when vss is in the ground set|`&vss @ROUTE-NET(priority="high");`|
|`%#3 @PLACE(1, 2);` in authored text|Name that object, then use its name|
|`#M1.d; M3; &vdd!;`|`#M1:d; #M3; &"vdd!";`|
|`{@VERSION("0.4");}`|`{ @VERSION("0.4");}`|

A wire to the bulk of `$NMOS` fails: use `$NMOS_4TERM`. Unknown transistor types need a model card, DEVICE or an explicit symbol. Top-level rules need `recursive=true` to reach subcircuits.

## A complete example

```spice
* Five-transistor OTA
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
.model nch nmos level=1
.model pch pmos level=1
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M3 x    x     vdd  vdd pch W=4u L=0.5u
M4 out  x     vdd  vdd pch W=4u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   &vss @ADD-TO $GROUND_NET_SET; /* grounds and supplies instead of labels */
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   %dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N; /* mirrored about the tail, tail below */
*   %cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR_P;
*   {%dp1 %cm1} @ROUTE-OBJECT; /* the nets between them as wires */
*   {#M1::d #M2::d #M5::d} @ORIENT("up"); /* NMOS upright */
*   #M1::g @ORIENT("left"); /* gates outwards */
*   {#M3::s #M4::s} @ORIENT("up"); /* PMOS upright */
*   {#M3::d #M1::d} @ALIGN("x"); /* each load over its input device */
*   {#M4::d #M2::d} @ALIGN("x");
*   #M3 #M1 @DISTANCE("top-to-bottom", min=4);
*   {#M1:d #M2:d #M5:d #M3:s #M4:s} @CURRENT("in"); /* current down each branch */
*   {#M1:s #M2:s #M5:s #M3:d #M4:d} @CURRENT("out");
*   $PORT_LIST @ATTACH; /* the ports in one column */
*   {#M1 #M2} @TEXT("${w}/${l}", key="wl", dy=-3);
* }
.ends ota5t
.end
```

Before you hand a file back: header first, with VERSION and the package; names exist in their level; only 0.4 names (the catalogue); a space before every `@`, `;` after every statement; directions written; set roles get sets; grounds and supplies in their sets; orientations only where needed, and current only where sure; aligned pins wired; no edits in generated records; no violation you did not mean.

<!-- lists:begin -->
## Catalogue

A reference, not RNL to copy: keep the `$` of a name. Apply definitions with AS or ATTACH, give symbols with SET-SYMBOL and styles with SET-STYLE. Roles are in declaration order, each with the kinds it takes (a role with none takes objects of any kind); `{role}` takes a whole set and may be omitted; parameters follow `;`. So `$MATCHED({members}: instance/usage)` is used as `{#M1 #M2} @AS $MATCHED;`.

### Packages

Each is named in the header with `@USE-PACKAGE(name="...", version="0.4");`.

|Package|Content|
|---|---|
|rnl-std-all|The one package collection of the series 0.4, with no content of its own: it brings in styles, basic and semiconductor symbols and definitions, automatic fill, circuit semantics, the four domain packages, and the rule package of each host dialect through a header record for that language (2.8, 16.3).|
|rnl-std-style|The styles (9.3) and the default style rules.|
|rnl-std-basic|The symbols of passive parts, sources, switches, transmission lines, virtual parts and marks; the definitions of virtual parts, marks, figures and stubs, each with the rule that draws its usages; and the sets of ground, supply and bias nets.|
|rnl-std-semiconductors|The symbols of diodes and of MOS, bipolar, JFET and unijunction (UJT, PUT) transistors.|
|rnl-std-spice|The rule package of SPICE: assigns symbols to instances by element letter, by model type, and by bulk or control-negative net (16.3).|
|rnl-std-spectre|The rule package of Spectre: assigns symbols to instances by master, by model type, and by bulk or control-negative net (16.3).|
|rnl-std-fill|The sets and rules of the automatic fill: attaches ground, supply, bias and net-label stubs to unwired pins, including the pins of virtual parts, and a port tag to every port net in `$PORT_FILL_SET` (10.5, 12.4, 16.3).|
|rnl-std-semantics|The semantic definitions of circuit structures and of their default drawings.|
|rnl-std-functional|Behavioral models: op-amps, comparators, amplifiers, transconductors, ADC, DAC and chopper modulator, with *_SUPPLY variants carrying integer-grid vp (top) and vn (bottom) pins.|
|rnl-std-rf|RF parts: the center-tap inductor and the T-coil.|
|rnl-std-logic|Gates, buffer, inverter, delay cell and D flip-flops.|
|rnl-std-signal-flow|Ideal signal-flow virtual parts and their symbols: adder, subtractor, multiplier, continuous-time and discrete-time integrators, quantizer and unit delay.|
|rnl-std-no-labels|Hides the default labels of the standard symbols, for a file that writes its own (16.3).|
|rnl-std-no-instance-labels|Hides the default labels of instances only; the net names that grounds, supplies, bias marks, port tags and net labels draw stay (16.3).|
|rnl-std-45deg|45-degree symbols for bridges, cross-coupled pairs, diagonal windings and 45-degree wires.|

### Definitions

Draws, for the structures of rnl-std-semantics: *meaning only* places and routes nothing; *places* has position facts on its members; *routes* joins the nets its members share. Their position facts and routes are of priority low. Group, read from the body of each definition: *yes* with the roles whose members it makes one group, *via* `$X` with the roles it passes to a definition `$X` that makes the group, and nothing when it makes none; a structure that may sit partly inside another, such as a current mirror, makes none (see the tree of groups under *Layout facts*). A usage without a symbol stands for its members in ALIGN, ORDER, DISTRIBUTE and DISTANCE; its own point, `%u::position` (14.1), follows the structure only where *own point* ties it to its members: only such a usage moves as one item by its `::position`, or as a member of another structure that places; for the others, constrain their members, or a named set of them. The other packages define the parts and marks that are drawn.

|Signature|Draws|Group|Meaning|
|---|---|---|---|
|**rnl-std-basic**||||
|`$GROUND(p: net)`|—||A ground on a net, drawn at a point of its own: on the net 0 it is the three bars alone, and on any other net it shows the name of its net.|
|`$SUPPLY(p: net)`|—||A supply on a net, drawn at a point of its own.|
|`$BIAS(p: net)`|—||A bias voltage mark on a net, drawn at a point of its own.|
|`$IO_PORT(p: net)`|—||A port tag on a net, drawn at a point of its own; its net must be a port of its level.|
|`$NET_LABEL(p: net/pin)`|—||A net label: the name of a net on a short lead, drawn at a point of its own; given a bus pin, such as a pin of a bundle (11.8), it keeps the pin and is one label on all its nets, showing their names (13.6).|
|`$NO_CONNECT(t: terminal/pin)`|—||A no-connect mark on a terminal or pin left unconnected on purpose; a terminal is kept as itself so a mark on a bus bit records which bit is open (12.6).|
|`$JUNCTION(p: point/terminal)`|—||A junction dot on a point or a terminal.|
|`$CURRENT_ARROW(p: terminal/point)`|—||A current arrow on a terminal or a point.|
|`$ARROW(at: point)`|—||An arrow head drawn on a point in a symbol or in the scene (9.2.7).|
|`$SPDT_SWITCH(at: point)`|—||A single-pole double-throw switch drawn as a figure on a point, with no netlist line.|
|`$FIGURE(at: point)`|—||A figure drawn at a point with no netlist line, given a symbol of its own with SET-SYMBOL.|
|`$PORT_LIST()`|—||A port tag on every port net of a level that does not already have an `$IO_PORT`, in one column, ordered from the top down and grouped, at least 6 units left of the instances of the level, each at R0 with its pin to the right, all at priority low, as rnl-std-fill draws them; the fill then adds none in that level (12.4).|
|`$COMMENT({target}; text="")`|—||A non-drawn comment attached to any objects, or to the level itself when target is omitted.|
|`$GROUND_STUB(t: pin; gap=3)`|—||A ground near a pin, upright, with its own wire: level with the pin or below it, at most gap units away vertically (priority low).|
|`$SUPPLY_STUB(t: pin; gap=3)`|—||A supply near a pin, upright, with its own wire: level with the pin or above it, at most gap units away vertically (priority low).|
|`$BIAS_STUB(t: pin)`|—||A bias voltage mark directly on a pin, facing away from the pin like a port stub; its bar slants as / above, below and on the left of the pin and as \ on its right.|
|`$IO_PORT_STUB(t: pin)`|—||A port tag directly on a pin, its lead serving as the stub's short wire; the pin and the tag are left unrouted, as with `$NET_LABEL_STUB`.|
|`$NET_LABEL_STUB(t: pin)`|—||A net label directly on a pin, its one-unit lead serving as the stub's short wire; on a bus pin, one label for the bus; the pin and the label are left unrouted.|
|**rnl-std-semantics**||||
|`$MATCHED({members}: instance/usage)`|meaning only||Devices or sub-structures matched in their type or definition and in their layout.|
|`$MATCHED_MOS({members}: instance)`|meaning only||MOSFETs matched in type, model and channel length.|
|`$MATCHED_BJT({members}: instance)`|meaning only||Bipolar transistors matched in type, model and unit emitter geometry.|
|`$MATCHED_RESISTOR({members}: instance)`|meaning only||Resistors matched in type and model.|
|`$MATCHED_CAPACITOR({members}: instance)`|meaning only||Capacitors matched in type and model.|
|`$MIRRORED_LAYOUT(left: instance/usage, right: instance/usage, axis: point/instance/usage)`|places; own point at the centre||Two symmetric objects at one height, mirror images of each other about an axis: a layout only, which says nothing about the circuit.|
|`$MIRRORED_VERTICAL_LAYOUT(top: instance/usage, bottom: instance/usage, axis: point/instance/usage)`|places; own point at the centre||Two symmetric objects in one column, mirror images of each other about a horizontal axis: a layout only.|
|`$BRANCH({members}: instance/usage)`|places, routes; own point in x||A series structure stacked vertically: members in series in one column, from the top down; usually a DC branch, such as a path from a supply to ground.|
|`$BRANCH_HORIZONTAL({members}: instance/usage)`|places, routes; own point in y||A series structure drawn as a row: members in series at one height, from left to right.|
|`$CASCADE({stages}: instance/usage)`|meaning only||A cascade of stages along a signal path.|
|`$SERIES({members}: instance/usage)`|routes||Members connected end to end in series along one current path.|
|`$PARALLEL({members}: instance/usage)`|routes||Members connected in parallel across the same two nets.|
|`$PARALLEL_HORIZONTAL({members}: instance/usage)`|places, routes; own point at the centre||Members in parallel side by side at one height, standing as one place in a column.|
|`$PARALLEL_VERTICAL({members}: instance/usage)`|places, routes; own point at the centre||Members in parallel one above another at one x, standing as one place in a row.|
|`$SIGNAL_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|meaning only||A signal path that flows one way: from its sources through the stages of its path, in the order written, to its loads.|
|`$POWER_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|meaning only||A path of power: from its sources, such as a supply or a battery, through the stages of its path that convert or switch it, in the order written, to the loads that use it.|
|`$BIAS_CIRCUIT({members}: instance/usage)`|meaning only||A bias circuit: the devices or structures that make the bias of other parts, such as a current reference.|
|`$FEEDBACK_LOOP({forward}: instance/usage, {feedback}: instance/usage)`|meaning only||A feedback loop: a forward path and a feedback network.|
|`$DIFFERENTIAL_PAIR_N(left: instance, right: instance, tail: instance/usage)`|places, routes; own point at the centre|yes: left, right, tail|A differential pair of n-type transistors with its tail below it.|
|`$DIFFERENTIAL_PAIR_P(left: instance, right: instance, tail: instance/usage)`|places, routes; own point at the centre|yes: left, right, tail|A differential pair of p-type transistors with its tail above it.|
|`$COMPLEMENTARY_DIFFERENTIAL_PAIR(pairup: usage, pairdown: usage)`|routes|yes: pairup, pairdown|A rail-to-rail input stage: an upper and a lower differential pair on the same inputs.|
|`$PSEUDO_DIFFERENTIAL_PAIR(left: instance, right: instance)`|places; own point at the centre||A pseudo-differential pair: two matched input transistors without a tail, drawn mirrored.|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_H(left: instance, right: instance, tailleft: instance/usage, tailright: instance/usage, bridge: instance/usage)`|places, routes; own point at the centre|yes: left, right, tailleft, tailright, bridge|A differential pair degenerated by a bridge between its two tails, an H.|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_N(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|places, routes; own point at the centre|yes: left, right, degleft, degright, tail|A differential pair of n-type transistors degenerated by an element under each input, a T, with its tail below.|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_P(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|places, routes; own point at the centre|yes: left, right, degleft, degright, tail|A differential pair of p-type transistors degenerated by an element over each input, an inverted T, with its tail above.|
|`$CROSS_COUPLED_PAIR(left: instance, right: instance)`|places, routes; own point at the centre|yes: left, right|A cross-coupled pair: two matched transistors cross-coupled in positive feedback, drawn mirrored.|
|`$CURRENT_MIRROR_N({ref}: instance, {out}: instance)`|places; own point in y||A low-side current mirror of n-type transistors, with sources or emitters pointing down.|
|`$CURRENT_MIRROR_P({ref}: instance, {out}: instance)`|places; own point in y||A high-side current mirror of p-type transistors, with sources or emitters pointing up.|
|`$CASCODE_CURRENT_MIRROR_N({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|places; own point in y||A low-side cascode current mirror: a mirror row at the bottom with a cascode row above it.|
|`$CASCODE_CURRENT_MIRROR_P({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|places; own point in y||A high-side cascode current mirror: a mirror row at the top with a cascode row below it.|
|`$TRANS_LINEAR_LOOP({a}: instance/usage, {b}: instance/usage)`|places, routes; own point in x|yes: a, b|A translinear loop of two groups of transistors, drawn in one column.|
|`$TRANS_LINEAR_LOOP_HORIZONTAL({a}: instance/usage, {b}: instance/usage)`|places, routes; own point in y|yes: a, b|A translinear loop of two groups of transistors, drawn in one row.|
|`$DIODE_CONNECTED(device: instance)`|routes||A diode-connected transistor with its gate tied toward its drain, or its base toward its collector.|
|`$COMMON_SOURCE(device: instance)`|meaning only||A common-source stage of one FET.|
|`$COMMON_GATE(device: instance)`|meaning only||A common-gate stage of one FET.|
|`$COMMON_DRAIN(device: instance)`|meaning only||A source follower (a common-drain stage) of one FET.|
|`$COMMON_EMITTER(device: instance)`|meaning only||A common-emitter stage of one bipolar transistor.|
|`$COMMON_BASE(device: instance)`|meaning only||A common-base stage of one bipolar transistor.|
|`$COMMON_COLLECTOR(device: instance)`|meaning only||An emitter follower (a common-collector stage) of one bipolar transistor.|
|`$FOLLOWER(device: instance)`|meaning only||A voltage-follower stage of one transistor.|
|`$CASCODE(source: instance/usage, cascode: instance)`|routes||A cascode: a source device or sub-structure in series with a common-gate transistor.|
|`$FOLDED_CASCODE(source: instance/usage, cascode: instance, bias: instance/usage)`|routes||A folded cascode: an input device folded into a common-gate transistor with its bias.|
|`$COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage)`|routes|via `$TRANSCONDUCTOR_Z_AMPLIFIER`: input, load|A common-source amplifier: a FET input transistor and its load.|
|`$COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage)`|routes|via `$TRANSCONDUCTOR_Z_AMPLIFIER`: input, load|A common-emitter amplifier: a bipolar input transistor and its load.|
|`$DEGENERATED_COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|routes|via `$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER`: input, load, degenerator|A degenerated common-source amplifier: a FET input transistor, its load and its degeneration.|
|`$DEGENERATED_COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|routes|via `$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER`: input, load, degenerator|A degenerated common-emitter amplifier: a bipolar input transistor, its load and its degeneration.|
|`$MILLER_AMPLIFIER(stage: instance/usage, compensation: instance/usage)`|routes|yes: stage, compensation|A Miller-compensated amplifier: a gain stage and its compensation network.|
|`$PUSH_PULL_STAGE(pullup: instance/usage, pulldown: instance/usage)`|routes|yes: pullup, pulldown|A push-pull output stage: the output transistors only, a pull-up in series with a pull-down; their bias and the stage that drives them are not part of it.|
|`$TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage)`|routes|yes: transconductor, load|An amplifier stage: a transconductor and its load.|
|`$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage, degenerator: instance/usage)`|routes|yes: transconductor, load, degenerator|A degenerated amplifier stage: a transconductor, its load and its degeneration.|
|`$PASS_SWITCH(device: instance/usage)`|meaning only||A switch that passes or blocks a signal between its two sides.|
|`$RESET_SWITCH(device: instance)`|meaning only||A precharge, equalization or reset transistor.|
|`$GROUND_SWITCH(device: instance)`|meaning only||A low-side switch connected to a ground net.|
|`$POWER_SWITCH(device: instance)`|meaning only||A high-side switch connected to a supply net.|
|`$TRANSMISSION_GATE(pmos: instance/usage, nmos: instance/usage)`|places, routes; own point in x|yes: pmos, nmos|A CMOS transmission gate, the PMOS above the NMOS.|
|`$TRANSMISSION_GATE_HORIZONTAL(pmos: instance/usage, nmos: instance/usage)`|places, routes; own point in y|yes: pmos, nmos|A CMOS transmission gate drawn side by side, the PMOS on the left.|
|`$MULTI_POLE_SWITCH({throws}: instance/usage)`|routes|yes: throws|A switch of several throws that share one pole.|
|`$BOOTSTRAPPED_SWITCH(device: instance/usage)`|meaning only||A bootstrapped sampling switch.|
|`$SC_SAMPLER({switch}: instance/usage, {capacitor}: instance/usage)`|routes||A switched-capacitor sampler: sampling switches with sampling capacitors.|
|`$CHARGE_SHARING(a: instance/usage, switch: instance/usage, b: instance/usage)`|routes||A charge-sharing network: two capacitors or capacitor banks that share charge through a switch.|
|`$CHOPPER({straight}: instance/usage, {crossed}: instance/usage)`|routes||A chopper: a bridge of four switches that reverses the polarity of a signal.|
|`$FLYING_CAPACITOR(capacitor: instance/usage)`|meaning only||A flying capacitor switched between clock phases.|
|`$CMOS_INVERTER(pmos: instance, nmos: instance)`|places, routes; own point in x|via `$CMOS_GATE`: pmos, nmos|A CMOS inverter, the PMOS above the NMOS.|
|`$CMOS_INVERTER_HORIZONTAL(pmos: instance, nmos: instance)`|places, routes; own point in y|via `$CMOS_GATE`: pmos, nmos|A CMOS inverter drawn side by side, the PMOS on the left.|
|`$CMOS_GATE({pullup}: instance/usage, {pulldown}: instance/usage)`|routes|yes: pullup, pulldown|A static CMOS logic gate: a pull-up network and a pull-down network meeting in series at the output.|
|`$LATCH({members}: instance/usage)`|routes|yes: members|A latch: a bistable core with the devices around it.|
|`$INVERTER_LATCH(a: instance/usage, b: instance/usage)`|routes|via `$LATCH`: a, b|A latch of two cross-coupled inverters.|
|`$SHIFT_REGISTER({stages}: instance/usage)`|meaning only||A shift register: a chain of flip-flops or latches, often drawn overlapping, with clocks seldom wired.|
|`$DIVIDER(upper: instance/usage, lower: instance/usage)`|routes||A divider of two branches in series, upper and lower.|
|`$RESISTOR_LADDER({resistors}: instance)`|places, routes; own point in x|yes: resistors|A ladder of resistors in series in one column, from the top down.|
|`$RESISTOR_LADDER_HORIZONTAL({resistors}: instance)`|places, routes; own point in y|yes: resistors|A ladder of resistors in series drawn as a row.|
|`$PI_NETWORK(shunt1: instance/usage, series: instance/usage, shunt2: instance/usage)`|routes||A pi network: a shunt branch, a series branch and another shunt branch.|
|`$T_NETWORK(series1: instance/usage, shunt: instance/usage, series2: instance/usage)`|routes||A T network: a series branch, a shunt branch and another series branch.|
|`$LC_TANK_PARALLEL(inductor: instance/usage, capacitor: instance/usage)`|routes|yes: inductor, capacitor|A parallel LC resonant tank.|
|`$RLC_TANK_PARALLEL(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|routes|yes: resistor, inductor, capacitor|A parallel RLC resonant tank.|
|`$LC_TANK_SERIES(inductor: instance/usage, capacitor: instance/usage)`|routes|yes: inductor, capacitor|A series LC resonant tank.|
|`$RLC_TANK_SERIES(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|routes|yes: resistor, inductor, capacitor|A series RLC resonant tank.|
|`$DC_FEED(device: instance/usage)`|meaning only||An RF choke or bias resistor that feeds a DC supply or bias to a node.|
|`$DECAP(device: instance)`|places; own point in x||A decoupling capacitor on a supply or bias node.|
|`$AC_COUPLE(device: instance/usage)`|meaning only||A series coupling capacitor that passes the AC signal and blocks DC.|
|`$OSCILLATOR({members}: instance/usage)`|routes|yes: members|An oscillator made of its active and resonant or delay elements.|
|**rnl-std-signal-flow**||||
|`$ADDER(in1: net, in2: net, out: net)`|—||An ideal signal-flow adder (out = in1 + in2), drawn with `$ADDER_SYMBOL`.|
|`$SUBTRACTOR(in1: net, in2: net, out: net)`|—||An ideal signal-flow subtractor (out = in1 - in2), drawn with `$SUBTRACTOR_SYMBOL`.|
|`$MULTIPLIER(in1: net, in2: net, out: net)`|—||An ideal signal-flow multiplier (out = in1 * in2), drawn with `$MULTIPLIER_SYMBOL`.|
|`$INTEGRATOR(in: net, out: net)`|—||An ideal continuous-time integrator, drawn with `$INTEGRATOR_SYMBOL`.|
|`$DISCRETE_TIME_INTEGRATOR(in: net, out: net)`|—||An ideal discrete-time integrator, drawn with `$DISCRETE_TIME_INTEGRATOR_SYMBOL`.|
|`$QUANTIZER(in: net, out: net)`|—||An ideal quantizer, drawn with `$QUANTIZER_SYMBOL`.|
|`$UNIT_DELAY(in: net, out: net)`|—||An ideal unit delay, drawn with `$UNIT_DELAY_SYMBOL`.|
|**rnl-std-45deg**||||
|`$ARROW_45(at: point)`|—||A 45-degree arrow head drawn on a point in a symbol or on a 45-degree wire in the scene.|
|`$CURRENT_ARROW_45(p: terminal/point)`|—||A 45-degree current arrow on a terminal or a point along a 45-degree branch.|

### Symbols

Pins are listed by the terminal or role they bind; `pin` (`p`) is the pin named `pin`, bound to role `p`; `pin` (nothing) binds nothing.

|Symbol|Pins|Meaning|
|---|---|---|
|**rnl-std-basic**|||
|`$RESISTOR`|`p` `n`|A resistor.|
|`$VARIABLE_RESISTOR`|`p` `n`|A variable resistor: a resistor with an arrow across it.|
|`$CAPACITOR`|`p` `n`|A capacitor.|
|`$VARIABLE_CAPACITOR`|`p` `n`|A variable capacitor: a capacitor with an arrow across it.|
|`$INDUCTOR`|`p` `n`|An inductor.|
|`$VARIABLE_INDUCTOR`|`p` `n`|A variable inductor: an inductor with an arrow across it.|
|`$TRANSFORMER`|`p1` `n1` `p2` `n2`|A transformer: two coupled windings standing side by side, their polarity dots at the top.|
|`$POTENTIOMETER`|`n1` `nw` `n2`|A three-terminal potentiometer, the wiper in the middle.|
|`$PHOTORESISTOR`|`p` `n`|A photoresistor (light-dependent resistor).|
|`$POLARIZED_CAPACITOR`|`p` `n`|A polarized (electrolytic) capacitor, its positive plate on p.|
|`$CRYSTAL`|`n1` `n2`|A quartz crystal resonator.|
|`$SPEAKER`|`p` `n`|A loudspeaker.|
|`$VOLTAGE_SOURCE`|`p` `n`|An independent voltage source.|
|`$CURRENT_SOURCE`|`p` `n`|An independent current source, its arrow from p to n.|
|`$BATTERY`|`p` `n`|A battery.|
|`$CONTROLLED_VOLTAGE_SOURCE`|`p` `n`|A controlled voltage source drawn as a diamond with its output pins only.|
|`$CONTROLLED_CURRENT_SOURCE`|`p` `n`|A controlled current source drawn as a diamond with its output pins only.|
|`$VCVS_4TERM`|`p` `n` `cp` `cn`|A voltage-controlled voltage source (E) with its control pins on the left.|
|`$VCCS_4TERM`|`p` `n` `cp` `cn`|A voltage-controlled current source (G) with its control pins on the left.|
|`$BEHAVIORAL_SOURCE`|`p` `n`|A behavioral source (B): a circle marked B.|
|`$RF_PORT`|`p` `n`|An RF port: a sine source in series with its internal resistance, for Spectre's port.|
|`$SINE_VOLTAGE_SOURCE`|`p` `n`|A sine-wave voltage source, + on p.|
|`$PULSE_VOLTAGE_SOURCE`|`p` `n`|A pulse voltage source, + on p.|
|`$MICROPHONE`|`p` `n`|A microphone, + on p.|
|`$AMMETER`|`p` `n`|An ammeter, current measured from p to n.|
|`$SWITCH`|`p` `n`|A switch drawn open, for a current-controlled switch (W) or Spectre's switch.|
|`$CLOSED_SWITCH`|`p` `n`|A switch drawn closed.|
|`$EXTERNALLY_CONTROLLED_SWITCH`|`p` `n` `cp`|A switch with a single control pin at its side.|
|`$VOLTAGE_CONTROLLED_SWITCH`|`p` `n` `cp` `cn`|A voltage-controlled switch (S), its differential control pins below it.|
|`$SPDT_SWITCH_SYMBOL`|`com` (nothing) `a` (nothing) `b` (nothing)|The symbol of `$SPDT_SWITCH`: a single-pole double-throw switch whose pins com a b bind nothing and are wired by name (9.2.4).|
|`$TRANSMISSION_LINE`|`p1` `n1` `p2` `n2`|A transmission line (T or O), port 1 on the left and port 2 on the right.|
|`$RC_LINE`|`a` `b` `ref`|A uniform distributed RC line (U).|
|`$GROUND_SYMBOL`|`pin` (`p`)|The symbol of `$GROUND` on the net 0: a three-bar ground, its pin at the top.|
|`$NAMED_GROUND_SYMBOL`|`pin` (`p`)|The symbol of `$GROUND` on any net other than 0: a three-bar ground with the name of its net under it.|
|`$SUPPLY_SYMBOL`|`pin` (`p`)|The symbol of `$SUPPLY`: a horizontal bar with the name of its net, its pin at the bottom.|
|`$BIAS_SYMBOL`|`pin` (`p`)|The symbol of `$BIAS`: a slanted bar with the name of its net, its pin at the bottom.|
|`$IO_PORT_SYMBOL`|`pin` (`p`)|The symbol of `$IO_PORT`: a small circle with the name of its net, its pin on the right.|
|`$NET_LABEL_SYMBOL`|`pin` (`p`)|The symbol of `$NET_LABEL`: the name of its net, or the names of the nets of a bus pin, on the left of a one-unit lead, its pin facing right, as `$IO_PORT_SYMBOL`.|
|`$ARROW_SYMBOL`|—|A filled arrow head pointing right, the symbol of `$ARROW`; `$CURRENT_ARROW` draws one half its size.|
|`$NO_CONNECT_SYMBOL`|—|The symbol of `$NO_CONNECT`: a cross.|
|`$JUNCTION_SYMBOL`|—|The symbol of `$JUNCTION`: a filled dot.|
|**rnl-std-semiconductors**|||
|`$DIODE`|`a` `k`|A diode.|
|`$ZENER_DIODE`|`a` `k`|A Zener diode.|
|`$LED`|`a` `k`|A light-emitting diode (LED).|
|`$SCHOTTKY_DIODE`|`a` `k`|A Schottky barrier diode.|
|`$SHOCKLEY_DIODE`|`a` `k`|A Shockley four-layer diode.|
|`$PUT`|`a` `g` `k`|A programmable unijunction transistor (PUT).|
|`$NMOS`|`d` `g` `s`|An NMOS transistor of three pins, its bulk not drawn.|
|`$NMOS_4TERM`|`d` `g` `s` `b`|An NMOS transistor with its bulk pin aligned to drain and source and its bulk arrow at the channel.|
|`$PMOS`|`d` `g` `s`|A PMOS transistor of three pins, its bulk not drawn.|
|`$PMOS_4TERM`|`d` `g` `s` `b`|A PMOS transistor with its bulk pin aligned to drain and source.|
|`$NMOS_LOGIC`|`d` `g` `s`|A three-pin logic NMOS transistor without a source arrow.|
|`$PMOS_LOGIC`|`d` `g` `s`|A three-pin logic PMOS transistor without a source arrow, with an inversion bubble on its gate.|
|`$DEPLETION_NMOS`|`d` `g` `s`|A depletion-mode NMOS transistor of three pins, its bulk not drawn.|
|`$DEPLETION_NMOS_4TERM`|`d` `g` `s` `b`|A depletion-mode NMOS transistor with its bulk pin aligned to drain and source.|
|`$DEPLETION_PMOS`|`d` `g` `s`|A depletion-mode PMOS transistor of three pins, its bulk not drawn.|
|`$DEPLETION_PMOS_4TERM`|`d` `g` `s` `b`|A depletion-mode PMOS transistor with its bulk pin aligned to drain and source.|
|`$NDMOS`|`d` `g` `s`|An N-type DMOS transistor of three pins, its bulk not drawn.|
|`$NDMOS_4TERM`|`d` `g` `s` `b`|An N-type DMOS transistor with its bulk pin aligned to drain and source.|
|`$PDMOS`|`d` `g` `s`|A P-type DMOS transistor of three pins, its bulk not drawn.|
|`$PDMOS_4TERM`|`d` `g` `s` `b`|A P-type DMOS transistor with its bulk pin aligned to drain and source.|
|`$NPN`|`c` `b` `e`|An NPN bipolar transistor, its substrate not drawn.|
|`$PNP`|`c` `b` `e`|A PNP bipolar transistor, its substrate not drawn.|
|`$NJFET`|`d` `g` `s`|An N-channel JFET, its gate arrow pointing at the channel.|
|`$PJFET`|`d` `g` `s`|A P-channel JFET, its gate arrow pointing away from the channel.|
|`$UJT`|`e` `b1` `b2`|A unijunction transistor (UJT), b2 above and b1 below.|
|**rnl-std-functional**|||
|`$OPAMP`|`inp` `inn` `out`|An op-amp of three terminals: inp, inn, out.|
|`$OPAMP_SUPPLY`|`inp` `inn` `out` `vp` `vn`|An op-amp with supply pins: inp, inn, out, vp, vn.|
|`$OPAMP_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|An op-amp with supply pins, its inputs swapped (inn above, inp below) and its supplies kept (vp above, vn below), as `$OPAMP_SUPPLY` turned MX would not: inp, inn, out, vp, vn.|
|`$DIFFERENTIAL_OPAMP`|`inp` `inn` `outp` `outn`|A fully differential op-amp: inp, inn, outp, outn.|
|`$DIFFERENTIAL_OPAMP_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|A fully differential op-amp with supply pins: inp, inn, outp, outn, vp, vn.|
|`$DIFFERENTIAL_OPAMP_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|A fully differential op-amp with supply pins, its inputs and outputs swapped (inn and outn above, inp and outp below) and its supplies kept (vp above, vn below), as `$DIFFERENTIAL_OPAMP_SUPPLY` turned MX would not: inp, inn, outp, outn, vp, vn.|
|`$DIFFERENTIAL_OPAMP_TWISTED`|`inp` `inn` `outp` `outn`|A fully differential op-amp with its output polarity swapped (outn above, outp below): inp, inn, outp, outn.|
|`$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|A fully differential op-amp with twisted outputs and supply pins: inp, inn, outp, outn, vp, vn.|
|`$DIFFERENTIAL_OPAMP_TWISTED_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|A fully differential op-amp with supply pins, its inputs swapped (inn above, inp below), its outputs as on `$DIFFERENTIAL_OPAMP_SUPPLY` (outp above, outn below) and its supplies kept (vp above, vn below), as `$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY` turned MX would not: inp, inn, outp, outn, vp, vn.|
|`$COMPARATOR`|`inp` `inn` `out`|A comparator of three terminals: inp, inn, out.|
|`$COMPARATOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|A comparator with supply pins: inp, inn, out, vp, vn.|
|`$COMPARATOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|A comparator with supply pins, its inputs swapped (inn above, inp below) and its supplies kept (vp above, vn below), as `$COMPARATOR_SUPPLY` turned MX would not; its step reads as on `$COMPARATOR_SUPPLY`: inp, inn, out, vp, vn.|
|`$VOLTAGE_AMPLIFIER`|`in` `out`|A voltage amplifier, with an optional centered text of key label: in, out.|
|`$VOLTAGE_AMPLIFIER_SUPPLY`|`in` `out` `vp` `vn`|A voltage amplifier with supply pins: in, out, vp, vn.|
|`$TRANSCONDUCTOR`|`in` `out`|A transconductor (a gm cell), with a centered text of key label, g_m by default: in, out.|
|`$TRANSCONDUCTOR_SUPPLY`|`in` `out` `vp` `vn`|A transconductor with supply pins and a centered text of key label, g_m by default: in, out, vp, vn.|
|`$DIFFERENTIAL_TRANSCONDUCTOR`|`inp` `inn` `out`|A transconductor with a differential input and a text of key label, g_m by default: inp, inn, out.|
|`$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|A differential-input transconductor with supply pins and a text of key label, g_m by default: inp, inn, out, vp, vn.|
|`$DIFFERENTIAL_TRANSCONDUCTOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|A differential-input transconductor with supply pins and a text of key label, g_m by default, its inputs swapped (inn above, inp below) and its supplies kept (vp above, vn below), as `$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY` turned MX would not: inp, inn, out, vp, vn.|
|`$ADC`|`in` `out`|An analog-to-digital converter: in, out.|
|`$ADC_SUPPLY`|`in` `out` `vp` `vn`|An analog-to-digital converter with supply pins: in, out, vp, vn.|
|`$DAC`|`in` `out`|A digital-to-analog converter: in, out.|
|`$DAC_SUPPLY`|`in` `out` `vp` `vn`|A digital-to-analog converter with supply pins: in, out, vp, vn.|
|`$CHOPPER_SYMBOL`|`inp` `inn` `outp` `outn`|A chopper modulator block (distinct from the switch-bridge structure `$CHOPPER`): inp, inn, outp, outn.|
|**rnl-std-rf**|||
|`$CENTER_TAP_INDUCTOR`|`n1` `n2` `tap`|An inductor with a center tap.|
|`$T_COIL`|`n1` `n2` `tap`|A T-coil: two coupled coils with a center tap and a bridging capacitor.|
|**rnl-std-logic**|||
|`$AND_2INPUT`|`a` `b` `y`|An AND gate of two inputs: a, b, y.|
|`$AND_3INPUT`|`a` `b` `c` `y`|An AND gate of three inputs: a, b, c, y.|
|`$NAND_2INPUT`|`a` `b` `y`|A NAND gate of two inputs: a, b, y.|
|`$NAND_3INPUT`|`a` `b` `c` `y`|A NAND gate of three inputs: a, b, c, y.|
|`$OR_2INPUT`|`a` `b` `y`|An OR gate of two inputs: a, b, y.|
|`$OR_3INPUT`|`a` `b` `c` `y`|An OR gate of three inputs: a, b, c, y.|
|`$NOR_2INPUT`|`a` `b` `y`|A NOR gate of two inputs: a, b, y.|
|`$NOR_3INPUT`|`a` `b` `c` `y`|A NOR gate of three inputs: a, b, c, y.|
|`$XOR_2INPUT`|`a` `b` `y`|An XOR gate of two inputs: a, b, y.|
|`$XOR_3INPUT`|`a` `b` `c` `y`|An XOR gate of three inputs: a, b, c, y.|
|`$XNOR_2INPUT`|`a` `b` `y`|An XNOR gate of two inputs: a, b, y.|
|`$XNOR_3INPUT`|`a` `b` `c` `y`|An XNOR gate of three inputs: a, b, c, y.|
|`$AND_4INPUT`|`a` `b` `c` `d` `y`|An AND gate of four inputs on integer grid points: a, b, c, d, y.|
|`$NAND_4INPUT`|`a` `b` `c` `d` `y`|A NAND gate of four inputs on integer grid points: a, b, c, d, y.|
|`$OR_4INPUT`|`a` `b` `c` `d` `y`|An OR gate of four inputs on integer grid points: a, b, c, d, y.|
|`$NOR_4INPUT`|`a` `b` `c` `d` `y`|A NOR gate of four inputs on integer grid points: a, b, c, d, y.|
|`$XOR_4INPUT`|`a` `b` `c` `d` `y`|An XOR gate of four inputs on integer grid points: a, b, c, d, y.|
|`$XNOR_4INPUT`|`a` `b` `c` `d` `y`|An XNOR gate of four inputs on integer grid points: a, b, c, d, y.|
|`$BUFFER`|`a` `y`|A buffer: a, y.|
|`$INVERTER`|`a` `y`|An inverter (a NOT gate): a, y.|
|`$DELAY_CELL`|`a` `y`|A delay cell: a, y.|
|`$D_FLIP_FLOP`|`d` `ck` `q` `qb`|A D flip-flop with the output Q and its complement: d, ck, q, qb.|
|`$D_FLIP_FLOP_Q_ONLY`|`d` `ck` `q`|A D flip-flop with the output Q only: d, ck, q.|
|`$D_FLIP_FLOP_WITH_RESET`|`d` `ck` `rst` `q` `qb`|A D flip-flop with a reset: d, ck, rst, q, qb.|
|`$AND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|An AND gate of 2 inputs with supply pins: a, b, y, vp, vn.|
|`$AND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|An AND gate of 3 inputs with supply pins: a, b, c, y, vp, vn.|
|`$AND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|An AND gate of 4 inputs with supply pins: a, b, c, d, y, vp, vn.|
|`$NAND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|A NAND gate of 2 inputs with supply pins: a, b, y, vp, vn.|
|`$NAND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|A NAND gate of 3 inputs with supply pins: a, b, c, y, vp, vn.|
|`$NAND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|A NAND gate of 4 inputs with supply pins: a, b, c, d, y, vp, vn.|
|`$OR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|An OR gate of 2 inputs with supply pins: a, b, y, vp, vn.|
|`$OR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|An OR gate of 3 inputs with supply pins: a, b, c, y, vp, vn.|
|`$OR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|An OR gate of 4 inputs with supply pins: a, b, c, d, y, vp, vn.|
|`$NOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|A NOR gate of 2 inputs with supply pins: a, b, y, vp, vn.|
|`$NOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|A NOR gate of 3 inputs with supply pins: a, b, c, y, vp, vn.|
|`$NOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|A NOR gate of 4 inputs with supply pins: a, b, c, d, y, vp, vn.|
|`$XOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|An XOR gate of 2 inputs with supply pins: a, b, y, vp, vn.|
|`$XOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|An XOR gate of 3 inputs with supply pins: a, b, c, y, vp, vn.|
|`$XOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|An XOR gate of 4 inputs with supply pins: a, b, c, d, y, vp, vn.|
|`$XNOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|An XNOR gate of 2 inputs with supply pins: a, b, y, vp, vn.|
|`$XNOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|An XNOR gate of 3 inputs with supply pins: a, b, c, y, vp, vn.|
|`$XNOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|An XNOR gate of 4 inputs with supply pins: a, b, c, d, y, vp, vn.|
|`$BUFFER_SUPPLY`|`a` `y` `vp` `vn`|A buffer with supply pins: a, y, vp, vn.|
|`$INVERTER_SUPPLY`|`a` `y` `vp` `vn`|An inverter with supply pins: a, y, vp, vn.|
|`$DELAY_CELL_SUPPLY`|`a` `y` `vp` `vn`|A delay cell with supply pins: a, y, vp, vn.|
|`$D_FLIP_FLOP_SUPPLY`|`d` `ck` `q` `qb` `vp` `vn`|A D flip-flop with supply pins: d, ck, q, qb, vp, vn.|
|`$D_FLIP_FLOP_Q_ONLY_SUPPLY`|`d` `ck` `q` `vp` `vn`|A D flip-flop with Q only and supply pins: d, ck, q, vp, vn.|
|`$D_FLIP_FLOP_WITH_RESET_SUPPLY`|`d` `ck` `rst` `q` `qb` `vp` `vn`|A D flip-flop with reset and supply pins (vp and vn aligned at x = 0): d, ck, rst, q, qb, vp, vn.|
|**rnl-std-signal-flow**|||
|`$ADDER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|The symbol of `$ADDER`: a summing circle with arrowheads on its input leads a (in1) and b (in2), and output y (out).|
|`$SUBTRACTOR_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|The symbol of `$SUBTRACTOR`: a summing circle with + in the center and - outside by its negative input b (in2), inputs a (in1) and b (in2), and output y (out).|
|`$MULTIPLIER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|The symbol of `$MULTIPLIER`: a product circle with arrowheads on its input leads a (in1) and b (in2), and output y (out).|
|`$INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|The symbol of `$INTEGRATOR`: a box marked 1/s with input a (in) and output y (out).|
|`$DISCRETE_TIME_INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|The symbol of `$DISCRETE_TIME_INTEGRATOR`: a box marked with its z-domain transfer function, input a (in) and output y (out).|
|`$QUANTIZER_SYMBOL`|`a` (`in`) `y` (`out`)|The symbol of `$QUANTIZER`: a box with a staircase, input a (in) and output y (out).|
|`$UNIT_DELAY_SYMBOL`|`a` (`in`) `y` (`out`)|The symbol of `$UNIT_DELAY`: a box marked z to the minus one, input a (in) and output y (out).|
|**rnl-std-45deg**|||
|`$RESISTOR_45`|`p` `n`|A 45-degree resistor.|
|`$CAPACITOR_45`|`p` `n`|A 45-degree capacitor.|
|`$INDUCTOR_45`|`p` `n`|A 45-degree inductor (also serves diagonal transformer or balun windings).|
|`$SWITCH_45`|`p` `n`|A 45-degree open switch.|
|`$VOLTAGE_SOURCE_45`|`p` `n`|A 45-degree independent voltage source, + near p and - near n kept upright.|
|`$CURRENT_SOURCE_45`|`p` `n`|A 45-degree independent current source, arrow from p to n.|
|`$DIODE_45`|`a` `k`|A 45-degree diode.|
|`$ZENER_DIODE_45`|`a` `k`|A 45-degree Zener diode.|
|`$SCHOTTKY_DIODE_45`|`a` `k`|A 45-degree Schottky diode.|
|`$LED_45`|`a` `k`|A 45-degree light-emitting diode (LED).|
|`$NMOS_45`|`d` `g` `s`|A 45-degree three-pin NMOS transistor.|
|`$NMOS_4TERM_45`|`d` `g` `s` `b`|A 45-degree four-pin NMOS transistor, bulk at (0, 0).|
|`$PMOS_45`|`d` `g` `s`|A 45-degree three-pin PMOS transistor.|
|`$PMOS_4TERM_45`|`d` `g` `s` `b`|A 45-degree four-pin PMOS transistor, bulk at (0, 0).|
|`$NPN_45`|`c` `b` `e`|A 45-degree NPN bipolar transistor.|
|`$PNP_45`|`c` `b` `e`|A 45-degree PNP bipolar transistor.|
|`$ARROW_45_SYMBOL`|—|A filled arrow head pointing -45 degrees (down-right, along p->n / a->k of the 45-degree symbols under R90), turning to all four diagonals under R0/R90/R180/R270.|

### Sets and styles

|Name|Package|Meaning|
|---|---|---|
|`$GROUND_NET_SET`|rnl-std-basic|The ground nets, 0 by default, which a file edits with ADD-TO and REMOVE-FROM; rnl-std-fill gives their unwired pins a ground stub and leaves them unrouted. How a ground is drawn does not depend on it.|
|`$SUPPLY_NET_SET`|rnl-std-basic|The supply nets, none by default, which a file edits with ADD-TO and REMOVE-FROM.|
|`$BIAS_NET_SET`|rnl-std-basic|The bias nets, none by default, which a file edits with ADD-TO and REMOVE-FROM.|
|`$PORT_FILL_SET`|rnl-std-basic|The port nets that rnl-std-fill gives a port tag: every port net of every level, supply and ground nets too, less those with an `$IO_PORT` written and those of a level with a `$PORT_LIST`.|
|`$GROUND_FILL_SET`|rnl-std-fill|The unwired pins on ground nets, which the fill gives a ground stub.|
|`$SUPPLY_FILL_SET`|rnl-std-fill|The unwired pins on supply nets, which the fill gives a supply stub.|
|`$BIAS_FILL_SET`|rnl-std-fill|The unwired pins on bias nets, which the fill gives a bias stub.|
|`$LABEL_FILL_SET`|rnl-std-fill|The remaining unwired pins on nets, which the fill gives a net-label stub; a bus pin gets one.|
|`$base`|rnl-std-style|The base style of every object: lines, texts and fills as Analog Canvas draws them.|
|`$bus`|rnl-std-style|The style of a bus: its lines bold.|
|`$signal`|rnl-std-style|A style for signal wires: blue and bold.|
|`$note`|rnl-std-style|A style for notes: grey, smaller text.|
|`$grey`|rnl-std-style|A grey style for lines and texts.|

<!-- lists:end -->
