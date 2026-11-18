# standard

The standard packages of RNL, the series 0.4 in [`0.4/`](0.4/), written in the language 0.4 and bundled with the core. They are drafts: the language standard leaves the content of the standard package open, and names may change with it. Packages are found by the name and version their header record declares, not by file name (16.2).

A file of 0.4 uses no package implicitly and names `@USE-PACKAGE(name="rnl-std-all", version="0.4")`, or the sub-packages it wants, in its header record (16.3); a reading can also add the collection, as `rnl --include-std` does. The series 0.2 and 0.3, for the files of the languages 0.2 and 0.3, are in the cores of 0.5.x, up to 0.5.7, whose `rnl upgrade` converts such files to 0.4 by the lists below.

## The series 0.4

Every package of the series is version 0.4. Its files hold one collection package, [`rnl-std-all`](0.4/rnl-std-all.rnl) (replacing `rnl-std`, `rnl-std-core` and `rnl-std-full` of the series 0.3), [`rnl-std-semantics`](0.4/rnl-std-semantics.rnl) for circuit-structure and layout definitions (replacing `rnl-std-structures`), [`alias-zh`](0.4/alias-zh.rnl) for Chinese aliases (replacing `rnl-zh`), the optional [`rnl-std-no-labels`](0.4/rnl-std-no-labels.rnl), [`rnl-std-no-instance-labels`](0.4/rnl-std-no-instance-labels.rnl) and [`rnl-std-45deg`](0.4/rnl-std-45deg.rnl) (45-degree symbols and diagonal arrow marks), and the ten sub-packages `rnl-std-all` brings in (`rnl-std-style`, `rnl-std-basic`, `rnl-std-semiconductors`, `rnl-std-spice` or `rnl-std-spectre` for the dialect, `rnl-std-fill`, `rnl-std-semantics`, `rnl-std-functional`, `rnl-std-rf`, `rnl-std-logic` and `rnl-std-signal-flow`). It has more structures than the series 0.3, and some names of 0.3 are gone or split, such as `$CHAIN`, `$DIFFERENTIAL_PAIR` into `$DIFFERENTIAL_PAIR_N` and `$DIFFERENTIAL_PAIR_P`, and `$CURRENT_MIRROR` into `$CURRENT_MIRROR_N` and `$CURRENT_MIRROR_P`.

**Names in the series 0.4.**

- A definition whose body only places, with no assertion and no routing, says nothing about the circuit, and its name ends in `_LAYOUT`, such as `$MIRRORED_LAYOUT` and `$MIRRORED_VERTICAL_LAYOUT`. Every other structure states a fact of the circuit and may carry a default drawing as well.
- `_N` and `_P` at the end of a structure name state the polarity of its transistors, and the structure asserts it for the transistors whose type is known:
  - `_N`: NMOS, NPN, or an n-channel JFET or MESFET;
  - `_P`: PMOS, PNP or lateral PNP, or a p-channel JFET or MESFET.

  These are `$DIFFERENTIAL_PAIR_N`, `$CURRENT_MIRROR_N`, `$CASCODE_CURRENT_MIRROR_N` and their `_P` twins, which also draw the n-type with its sources or emitters down and the p-type with them up.
- The flow of a signal and the flow of power are told apart: `$SIGNAL_FLOW` and `$POWER_FLOW` each take a path, in order, with the sources before it and the loads after it. The devices that bias the rest of a circuit are marked with `$BIAS_CIRCUIT`. The drawing definition of a bias mark, `$BIAS`, is a different thing and stays.

### Renamed, merged, split and removed in 0.4

`rnl upgrade` of core 0.5.7 rewrites files of 0.3 and 0.2 by this list. Where the table says *manual*, it reports the place and leaves the text as it was.

#### Packages

| Series 0.3 | Series 0.4 |
|---|---|
| `rnl-std` (collection) | `rnl-std-all`, the one collection; it has a header record for each host language |
| `rnl-std-core`, `rnl-std-full` (collections) | removed: use `rnl-std-all`, or name the packages one by one |
| `rnl-std-structures` | `rnl-std-semantics` (`$MATCHED`, `$BRANCH`, `$CASCADE` move with it, `$FLOW` as `$SIGNAL_FLOW` and `$MIRRORED` as `$MIRRORED_LAYOUT`) |
| — | new, optional, not in `rnl-std-all`: `rnl-std-45deg` |
| — | new, optional, not in `rnl-std-all`: `rnl-std-no-instance-labels`, which hides the default labels of instances only, where `rnl-std-no-labels` hides those of usages too |

#### Symbols

| Series 0.3 | Series 0.4 | Note |
|---|---|---|
| `$VCVS`, `$CCVS` | `$CONTROLLED_VOLTAGE_SOURCE` | merged; drawn with its output pins only. The SPICE rules give E `$VCVS_4TERM` |
| `$VCCS`, `$CCCS` | `$CONTROLLED_CURRENT_SOURCE` | merged; the SPICE rules give G `$VCCS_4TERM` |
| `$OPAMP_5TERM` | `$OPAMP` | three terminals, mapped `[1, 2, 5]`; the supply pins are on `$OPAMP_SUPPLY` |
| `$COMPARATOR_5TERM` | `$COMPARATOR` | as for `$OPAMP_5TERM`; `$COMPARATOR_SUPPLY` keeps the supply pins |
| `$CURRENT_ARROW_SYMBOL` | `$ARROW_SYMBOL` | |
| `$DELAY_CELL_SYMBOL` | `$DELAY_CELL` | `$DELAY_CELL` is now the two-terminal logic symbol (it was a definition attached to a point) |
| `$ADDER`, `$MULTIPLIER` (symbols) | `$ADDER_SYMBOL`, `$MULTIPLIER_SYMBOL` | the names `$ADDER` and `$MULTIPLIER` are now general definitions of the signal-flow blocks |

Also: net names on grounds, supplies, bias marks, port tags and net labels use `key="netname"` (was `key="name"`); the label of `$TRANSCONDUCTOR` uses `key="label"` (was `key="formula"`).

#### General definitions

| Series 0.3 | Series 0.4 | `rnl upgrade` |
|---|---|---|
| `$DIFFERENTIAL_PAIR` | split into `$DIFFERENTIAL_PAIR_N` and `$DIFFERENTIAL_PAIR_P`, which assert the polarity of the two inputs; roles `left`, `right` (were the two inputs) and an optional single `tail` | with `--approximate`, chosen by the model types of both inputs |
| `$CURRENT_MIRROR` | split into `$CURRENT_MIRROR_N` and `$CURRENT_MIRROR_P`, which assert the polarity of `ref` and `out` | with `--approximate`, chosen by the types of `ref` and `out` |
| `$CHAIN` | removed | manual: keep the ordering explicitly, or choose another definition |
| `$SUPPLY_HIGH`, `$SUPPLY_LOW` | removed | manual |
| `$CASCADE` | four roles become one set role `stages` | manual role mapping |
| `$ARROW`, `$SPDT_SWITCH`, `$FIGURE` | role `p` becomes `at` | renamed |
| `$INTEGRATOR`, `$DISCRETE_TIME_INTEGRATOR`, `$QUANTIZER`, `$UNIT_DELAY` | a block with net roles `in` and `out` (was a figure attached to a point, role `p`) | manual: `$FIGURE` with the `*_SYMBOL` for a drawing only, or bind the block roles |
| `$DELAY_CELL` (definition) | removed; the name is the logic symbol | manual: `$FIGURE` with `@SET-SYMBOL $DELAY_CELL`, or a host instance |
| `$GROUND`, `$SUPPLY`, `$IO_PORT` | role `p` takes a net only (was a net or a terminal); `$IO_PORT` asserts that its net is a port | a terminal written directly: attach its net and keep the point |
| `$NO_CONNECT` | role `t` takes a terminal or a pin | unchanged text |
| `$GROUND_STUB`, `$SUPPLY_STUB` | `gap` default 2 → 1; role `t` is a pin | writes `gap=2` where the old default was used |
| `$GROUND` on a net other than 0 | drawn with `$NAMED_GROUND_SYMBOL`, the name of its net under the bars, whether or not the net is in `$GROUND_NET_SET` (in 0.3 only the nets of `$GROUND_NET_SET` showed it); `$NAMED_GROUND_SYMBOL` has no set of its own: `$NAMED_GROUND_NET_SET` is removed, and `$GROUND_NET_SET` only decides the fill | manual: a reference to `$NAMED_GROUND_NET_SET` is reported; to draw a ground on such a net without its name, `@SET-SYMBOL $GROUND_SYMBOL` on the usage |
| `$MIRRORED` | `$MIRRORED_LAYOUT`: a layout only; `axis` is a single instance, usage or point; directed distribution | renamed; the rest with `--approximate` |
| `$FLOW` | `$SIGNAL_FLOW`, with the optional set roles `source` and `load` after `path`, which take devices, structures, ports (`#:in`), terminals and nets; places nothing and takes no direction | renamed; restores the old ORDER where it was used |

Definitions that only gained constraints or assertions keep their names, such as `$JUNCTION` and `$MATCHED`, and the upgrade reports them as notices; `$BRANCH` also gained series routing and an order from top to bottom, which the upgrade leaves to a person.

#### Series 0.2 names

`rnl upgrade` also takes the names of the series 0.2. Symbols:

| 0.2 | 0.4 |
|---|---|
| `$RES` | `$RESISTOR` |
| `$CAP` | `$CAPACITOR` |
| `$IND` | `$INDUCTOR` |
| `$VSRC` | `$VOLTAGE_SOURCE` |
| `$ISRC` | `$CURRENT_SOURCE` |
| `$NMOS4` | `$NMOS_4TERM` |
| `$PMOS4` | `$PMOS_4TERM` |
| `$VCVS` | `$VCVS_4TERM` |
| `$VCCS` | `$VCCS_4TERM` |
| `$GROUND` (symbol) | `$GROUND_SYMBOL` |
| `$SUPPLY_BAR` | `$SUPPLY_SYMBOL` |
| `$PORT_TAG` | `$IO_PORT_SYMBOL` |
| `$NC_CROSS` | `$NO_CONNECT_SYMBOL` |
| `$JUNCTION_DOT` | `$JUNCTION_SYMBOL` |
| `$CURRENT_ARROW_SYM`, `$ARROW_HEAD` | `$ARROW_SYMBOL` |

Definitions and sets:

| 0.2 | 0.4 |
|---|---|
| `$GND` | `$GROUND` |
| `$IOPORT` | `$IO_PORT` |
| `$NC` | `$NO_CONNECT` |
| `$GND_STUB` | `$GROUND_STUB` |
| `$GND_FILL_SET` | `$GROUND_FILL_SET` |
| `$LABEL_TERM_SET` | `$LABEL_FILL_SET` |
| `$MIRROR_PAIR` | `$MIRRORED_LAYOUT` |

Every symbol writes its box (`box=`): the extent of its graphics, which a viewer uses for selection and a solver for keeping symbols apart. Pins sit on integer points, so instances on integer points keep wires on the grid.

The geometry comes from the language prototype's standard package, reviewed against the terminal order of the SPICE elements. The prototype took it from Analog Canvas's symbols; their copyright holder publishes it here under CC0-1.0, like the rest of this directory. The rules that pick symbols by Spectre master and by model type are new: Spectre instance names carry no element letter. Elements without a standard symbol yet (J, F, H, B and others) are drawn with the viewer's own stand-in, a dashed box with the terminals on its sides.

The data in this directory is licensed under CC0-1.0, so symbol definitions can be copied into netlists freely.
