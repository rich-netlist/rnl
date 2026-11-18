# Writing RNL that a solver draws well

This page is for files left to a solver: you write what the circuit is and how its parts relate, and `rnl solve` finds the coordinates. It assumes the short guide ([language.md](language.md)); look every name up in [packages.md](packages.md). The solver is a preview, and it says so when it runs: when the drawing must be exact, write every position yourself (PLACE, ORIENT and WIRE) instead of solving.

The points below say what to write. Complete files that follow them are in [the examples](#examples).

## Rails and current

- **Rails.** Put the supply and ground nets in their sets, so that the fill draws supply and ground symbols on them instead of labels.

  ```rnl
  &vdd @ADD-TO $SUPPLY_NET_SET; &vss @ADD-TO $GROUND_NET_SET;
  ```

- **Current only where you know it.** Write `@CURRENT` from an analysis of the circuit's actual static current paths, terminal by terminal. A terminal's name or polarity does not tell it: a netlist may use a transistor's source and drain the other way round, and the positive terminal of a part is not always where its static current enters. Not every circuit or structure has a static current: a CMOS inverter, a switch or a capacitor carries none at rest. Where your analysis is not certain, write nothing; a wrong direction misleads the solver more than a missing one.

  ```rnl
  #M5:d @CURRENT("in"); #M5:s @CURRENT("out"); /* only after checking that M5's bias current flows from d to s */
  ```

## Columns and structures

- **A `$BRANCH` is a series structure placed vertically.** Write each DC branch, such as a path from a supply to ground, as one `$BRANCH`, and also any other series structure that should stand vertically in the drawing. Members are written from the top down.

  ```rnl
  %left = {#M3 #M1} @AS $BRANCH;
  ```

- **Write the structures that hold**, with their roles: a differential pair, a mirror, an inverter. Look at the column *Draws* of the catalogue: a structure that *places* or *routes* draws something, one of *meaning only* only says what its parts are.

  ```rnl
  %dp = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
  ```

- **Mark signal flow only where it is clear.** Write `$SIGNAL_FLOW` only for a clean signal path you are sure of. Leave it out where the signal takes several paths, where there is feedback, or where much of the circuit works both ways; how many members the path has does not decide it.

- **Do not repeat what a structure draws.** A pair aligns and mirrors its halves, a mirror aligns its devices, a transmission gate stacks its two transistors. An ALIGN or ORIENT that says the same again adds nothing, and one that differs fights the structure.

## Order

- **Order structures first, not single components.** Give the columns and structures one order from left to right; constrain single components only where no structure covers them.

  ```rnl
  {%bias %left %right} @ORDER("left-to-right");
  ```

  In ORDER, ALIGN, DISTRIBUTE, DISTANCE and TABLE a structure stands for its members, the parts that fill its roles, down to the lowest: every device of `%left` lies between every device of `%bias` and every device of `%right`. A device that two structures share is not ordered against itself.

  To constrain the centre of a structure instead, write its own point, `::position`:

  ```rnl
  {%dp::position #X1} @ALIGN("y");
  ```

  This puts X1 at the height of the centre of the pair; `{%dp #X1} @ALIGN("y")` would put the three transistors of the pair at X1's height. A structure of two sides mirrored about an axis, such as a differential pair or `$MIRRORED_LAYOUT`, has its point at the centre between the sides; a row or a column, such as `$BRANCH`, a current mirror or an inverter, ties it only on its line (the column *Draws* says where). Every other structure leaves it free, and a constraint on its `::position` changes nothing drawn.

- **Inputs on the left, outputs on the right.** Input ports or input components usually go leftmost, output ports or load components rightmost. This is a habit, not a rule: change it where the drawing reads better.

  ```rnl
  $IO_PORT_STUB @ATTACH #M1:g; %po = $IO_PORT @ATTACH &out; {%right %po::pin} @ORDER("left-to-right");
  ```

  An input port can sit straight on the pin it drives with `$IO_PORT_STUB`. An output port is a `$IO_PORT` ordered after the last column, with its net routed. Ports you do not tag get a tag from the fill, at R0 in one column at least 6 units left of the instances.

- **Parts around the circuit a little apart.** The parts around the circuit, such as the supplies and the signal sources, usually stand on one side of it, a little away from the rest. A DISTANCE says so:

  ```rnl
  #VDD {#VIN #XU1 #CL} @DISTANCE("left-to-right", min=4);
  ```

  The supply stands at least 4 units left of each of the others. Like the inputs on the left, this is a habit, not a rule.

- **Align only what no structure aligns,** such as the tops of two columns or a second stage beside the first.

## Groups

A drawing reads well when it has a clear tree of groups: each functional block is a group, groups nest inside larger ones, and each node of the tree holds only a few things directly. The solver arranges the members of a group together and keeps other objects out of its region.

- **Keep each node small.** A level or a group should hold about five or six things directly: devices, ports or smaller groups. This is a habit, not a rule; where a node holds many more, look for blocks that belong together and group them.
- **Many structures are groups already.** The column *Group* of the catalogue says which, such as a differential pair, a cross-coupled pair, a transmission gate or a latch. A structure that may sit partly inside another one, such as a current mirror whose output device also belongs to an amplifier, makes no group.
- **Group the blocks yourself.** A GROUP written in the file frames several structures, or a structure with its ports:

  ```rnl
  %amp %p_inp %p_inn %p_out @GROUP;
  ```

- **Groups do not cross.** Two groups that share members while neither contains the other cannot both be blocks: the solver keeps one and a hint names the other. Group whole structures, not a device of one with a device of another.
- **Look at the tree once the constraints are written.** `rnl groups circuit.sp` prints the tree of groups of each level, with the parts each node holds directly ([tools.md](tools.md#the-rnl-command)); where a node holds too many, add a group. `rnl check` hints where a node holds more than 20 devices directly.

## Wiring

The solver draws only the wires that WIRE, ROUTE-NET and ROUTE-OBJECT ask for and those the structures route themselves; the fill gives every other pin a ground, a supply or a net label. Choose the wires by these rules:

- **Inside a structure: usually wired.** The standard structures usually route their own nets already.
- **Between structures: wire the main nets.** These are mostly the signal lines. A clock is not always wired; bias voltages and supplies are usually left to labels and symbols.

  ```rnl
  {%dp %load} @ROUTE-OBJECT; &out @ROUTE-NET;
  ```

- **Symmetric structures are wired symmetrically.** If one side of a differential pair is wired, the other side is wired too, even when that wire crosses another.

  ```rnl
  {&outp &outn} @ROUTE-NET; /* both outputs, or neither */
  ```

- **Draw by hand what the solver cannot.** A wire the solver does not draw well, or one that must take a particular path, is written with WIRE. A direct line between two pins and a run at 45° in particular usually need a WIRE written out.

  ```rnl
  &mid @WIRE(mode="straight") {#R1:2 #C1:1}; /* one straight segment, at any angle */
  &out @WIRE(mode="manhattan45") {#M2:d #R2:1}; /* across, a 45° diagonal, across */
  ```

- A ROUTE-OBJECT over structures also covers their rail pins, and the standard package keeps supplies and grounds unrouted at high, so they still get symbols; the hint RNL9403 on a rail says so and is expected. To wire a rail, write `&vss @ROUTE-NET(priority="high");`.

## Orientation

- **Orient only the key components.** Signal sources, supplies and loads are usually R0; leave the rest to the solver, since the structures and the current settle most of it. Where a component must lie sideways, such as a switch in a signal path, give it by a pin direction.

  ```rnl
  {#V1 #VDD #RL} @ORIENT("R0"); {#S1::d #S2::d} @ORIENT("left");
  ```

- **After solving, you may add orientations** for a few components where the drawing would read better, and solve again.

## Symbols and labels

- **Use the standard symbols.** Look in the catalogue before you define a symbol. A subcircuit whose ports have the names of a standard symbol's pins takes that symbol with SET-SYMBOL; a fully differential amplifier with ports inp, inn, outp and outn, for instance, is `$DIFFERENTIAL_OPAMP`.

  ```rnl
  #XOA @SET-SYMBOL $DIFFERENTIAL_OPAMP;
  ```

- **Hide labels with a package.** To leave out the names and models of the instances, use the package rnl-std-no-instance-labels; do not clear the texts one by one.

## Mistakes that cost a drawing

- Structures without any order between them: the solver may stack or scatter them.
- An ALIGN on a whole structure where its centre is meant: `{%dp #X1} @ALIGN("y")` puts every device of the pair at one height. Write `%dp::position` for the centre; a structure that ties no point of its own (the column *Draws*) has no centre to constrain.
- A structure switched off with `priority="disabled"` to silence a violation: that hides what is wrong and drops the drawing the structure gives. Fix the roles, or take a structure that claims less.
- A space before a role: `{#:vinp #:vinn} (source)` is a syntax error (RNL1130); write `{#:vinp #:vinn}(source)`.
- No routes at all: the drawing is all symbols and labels.
- A level of dozens of devices without groups: the solver has no blocks to arrange, and the drawing has none either. Group the functional blocks.
- A priority on a structure or its AS: a structure draws at low and needs none. Write one only to settle a conflict that a check or a solve has shown.
- Taking the ground, supply and bias nets out of a ROUTE-NET by hand: the fill keeps them unrouted at high anyway.
- An ALIGN, DISTANCE or ORIENT on every pair of devices: it repeats the structures, and where it contradicts them the solver drops facts (the hints RNL5100 to RNL5104 name them).

## Check, solve and look

```bash
rnl check circuit.sp
rnl solve circuit.sp -o solved --solver-config 'limit="time",seconds=30'
rnl check solved/circuit.sp
rnl render solved/circuit.sp -o png --png
```

- Fix the file until the check reports no errors, no violations and no warnings. A file without positions has a hint that objects are left to a solver; that is expected. RNL5002 and RNL5200 to RNL5203 are errors: a constraint contradicts itself.
- `-o` names a folder. In Windows PowerShell 5.1, a native command loses the inner double quotes: write `--solver-config 'limit=\"time\",seconds=30'` there.
- The check of the solved file adds geometry hints. Those that most often point at the file:
  - RNL5100 to RNL5104: facts discarded because they conflict; RNL5104 names the statement that caused it.
  - RNL7010 port-inside, RNL7009 port-enclosed: a port faces into the drawing or is boxed in; order it at the edge.
  - RNL7001 crossings, RNL7014 wire-over-text: change the order of the columns, or move the part a crossing wire serves next to it.
  - RNL7005 name-only, RNL7008 partly-named: a pin is joined only by its name; route the net if it is a main net, leave it if it is a bias or a rail.
- Look at the picture as a reader would: the signal from left to right, the current from top to bottom, matched devices side by side, nothing upside down. Add the one fact that says how something should be, and solve again.

## Fine-tuning a solved drawing

- The solver writes its result into a record of its own at the end of each level, `@RNL(type="generated", …)`. The next solve leaves these records out when it reads the level and then replaces them as a whole, so whatever is changed inside them is lost.
- To keep part of a result, copy those statements, such as a PLACE, an ORIENT or a WIRE, out of the generated record into a record of your own (`* @RNL { … }`), and change or add to them there. Your record stays and binds the next solve; what you did not copy is solved again.

  ```rnl
  #M1 @PLACE(10, 20) => @ORIENT("MY"); /* copied from the generated record, then moved */
  &out @WIRE(mode="straight") {#M2:d (30, 14) #R2:1};
  ```

- A generated record may refer to objects by number, such as `%#3`; numbers are valid only there. In your own record, refer to the object by its name.

## Examples

Complete files that follow these points; each checks without errors and solves.

| File | What it shows |
| --- | --- |
| [solver-cs-amp.sp](../examples/solver-cs-amp.sp) | A common-source stage: one column for the DC path, the load capacitor and the output port to its right, the output net routed |
| [solver-ota5t.sp](../examples/solver-ota5t.sp) | A five-transistor OTA with its bias mirror: the bias diode and the two branches in three columns, the pair and its load wired, both inputs alike |
| [solver-cascode-mirror.sp](../examples/solver-cascode-mirror.sp) | A cascode current mirror: two columns of three; only the tops of the columns aligned by hand |
| [solver-miller-ota.sp](../examples/solver-miller-ota.sp) | A two-stage Miller op-amp: a sideways compensation network between the stages, the signal nets routed, the bias net left to its name |
| [solver-pi-follower.sp](../examples/solver-pi-follower.sp) | A pi filter and a source follower: a signal path from left to right through series parts |
| [solver-sampler-inverter.sp](../examples/solver-sampler-inverter.sp) | A transmission-gate sampler and an inverter: switches turned sideways by one pin direction, no CURRENT |
