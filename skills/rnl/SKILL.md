---
name: rnl
description: Read, write, check and draw RNL (Rich Netlist) 0.4, the drawing of a schematic written into a SPICE or Spectre netlist as `@RNL` comment records. Use it when a netlist has `* @RNL` or `// @RNL` records or `.rnl` package files, when asked to place, wire, annotate or draw a circuit's schematic from a netlist, to pick symbols or define structures such as a differential pair or a current mirror, to move a file of RNL 0.3 to 0.4, or to check a file with `rnl check`, `@rich-netlist/runtime` or the RNL viewers. Covers the language 0.4, the standard packages of series 0.4 and the tools. For exporting Virtuoso schematics or libraries to RNL, routes to the companion rnl-virtuoso skill.
license: See LICENSE.md (MIT, CC-BY-4.0 and CC0-1.0, by file)
compatibility: Reading and writing RNL needs no tools. Checking needs the rnl command (download a core Release executable, or build from source with Rust 1.85 or later) or Node 18 or later with the npm package @rich-netlist/runtime. Moving a file of 0.3 to 0.4 needs the last core release 0.5.x, whose rnl upgrade does it.
---

# RNL (Rich Netlist) 0.4

RNL is written in the comments of a SPICE or Spectre netlist. It says how the circuit is drawn as a schematic (which symbols, where things go, how wires run, what the parts mean) while the netlist alone defines the circuit. Simulators ignore the comments, so a netlist with RNL records still simulates as before. This skill covers the language **0.4** and the standard packages of the **series 0.4**. A file of 0.4 uses only packages of 0.4, and it names the packages it uses: nothing is used by itself.

A complete file, with the model card that a transistor needs to be drawn (a transistor is drawn by the type of its model card, so one whose card the netlist lacks is not drawn):

```spice
* Five-transistor OTA, input stage
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
.model nch nmos
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   %dp1 = #M1(left) and #M2(right) over #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N;
*   {#M1 #M2} @ALIGN("y"), i.e., same y;
*   #M5 is at @PLACE(20, 6) => it @ORIENT("R0");
*   %g0 = $GROUND is @ATTACH to &vss;
*   net &vss has a @WIRE connecting {#M5:3 %g0::pin}
* }
.ends ota5t
.end
```

The header record says that the file is written in 0.4 and uses the whole standard series. The other record says that M1 and M2 are an NMOS differential pair with M5 as its tail, that they share one y, where M5 goes and how it is turned, and that the net `vss` gets a ground wired to M5. `#M1` is an instance, `&vss` a net, `#M5:3` a terminal, `%dp1` a name of the RNL, `$GROUND` a definition of the standard package and `%g0::pin` the pin of its symbol; `@NAME(…)` are keywords, and words such as `and`, `treated` and `is at` are free text that only makes the statement read like a sentence.

## How to work

1. **Read [references/language.md](references/language.md) once, all of it**. It is the short guide, not the standard. For normative details, read the complete Markdown directly: prefer `spec/rnl-0.4.md` in an available RNL checkout, otherwise fetch the [English Markdown source](https://raw.githubusercontent.com/rich-netlist/rnl/main/spec/rnl-0.4.md). Search and read the relevant sections there instead of extracting the rendered website. The [Chinese Markdown](https://raw.githubusercontent.com/rich-netlist/rnl/main/spec/rnl-0.4.zh.md) is a translation; English prevails. Use the requested tag/revision when one is specified; source retrieval details are in [references/tools.md](references/tools.md#reading-the-standard-source).
2. **Look every `$NAME` up in [references/packages.md](references/packages.md)** before you write it: the definitions, symbols and sets of the standard packages, which symbol an instance gets by default, and what the alias package `alias-zh` is. A name that is not there is an error; do not guess names from other EDA tools or from 0.3 (`$DIFFERENTIAL_PAIR` and `$CURRENT_MIRROR` of 0.3 are `$DIFFERENTIAL_PAIR_N`/`_P` and `$CURRENT_MIRROR_N`/`_P` in 0.4, and the collection `rnl-std` is `rnl-std-all`).
3. **When a solver is to place the drawing**, read [references/good-drawings.md](references/good-drawings.md) before you write the records: the points that make a solved drawing read well (rails and current, columns and structures, order, wiring and wires drawn by hand, orientation), the mistakes that cost a drawing, how to check the solved result, how to fine-tune it, and complete example files. Skip it when the task gives every position.
4. **Write the records into the netlist's comments** and leave the netlist's own lines alone. A record belongs to the level it is written in: inside a `.subckt` for that subcircuit, outside any for the file's top level. Each line of a record is a whole-line comment: the comment character in column 1 (`*`; in Spectre `//` or `*`), then nothing or one space, then the text. `*@RNL` and an indented `* @RNL` are not record lines and cut the record off.
5. **Check the file** with a tool of [references/tools.md](references/tools.md) after every change, and fix what it reports until there are no errors, no violations and no warnings. A violation is a structure that does not hold what it claims, such as a pair of two different sizes: fix the roles or take a structure that claims less. A file without positions says in a hint that some objects are left to a solver and not drawn; that is not a problem.
6. **Draw it** when the user wants to see it: `rnl render FILE -o DIR` writes an SVG for each drawing level. The [web viewer](https://rich-netlist.github.io/rnl/viewer/) can open a [link to a hosted netlist or receive text from a web page](references/tools.md#links-that-open-a-specific-netlist); the [desktop viewer](https://rich-netlist.github.io/rnl/en/tools.html#desktop) opens local files on Windows, Linux or macOS. An agent can [install and launch it with a file](references/tools.md#installing-and-launching-the-desktop-viewer) when requested. Only what has coordinates is drawn.

## Rules that save time

- The netlist defines the circuit; RNL cannot change a device, a parameter or a connection. If a record contradicts the netlist, the netlist wins.
- Records with `type="generated"` are what a solver wrote back. Leave them as they are: the next solve replaces them as a whole. `rnl strip-generated FILE -o OUT` takes them out. To keep part of a result, copy those statements into a record of your own and change them there ([references/good-drawings.md](references/good-drawings.md#fine-tuning-a-solved-drawing)).
- Put the header record `* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}` first in a file you write, below the title line of a SPICE netlist. Without the USE-PACKAGE, no standard symbol, rule or fill is in effect. An included file without a header record takes the version of the file that includes it.
- **Files of 0.3.** The core from 0.6.0 reads only 0.4. To move a file of 0.3, run `rnl upgrade FILE -o DIR` of the last core release 0.5.x (see [references/tools.md](references/tools.md#moving-a-file-of-03-to-04)), read its report, and finish by hand what it leaves as a comment; then check the result with a current core. Do not rewrite a file of 0.3 by guessing.
- Do not copy a stranger's RNL from memory or from 0.2 or 0.3 material: check each name and each keyword against the guide and the packages, and each file with a tool.
- When a message is unclear, search the standard for the keyword it names; the standard word list is `spec/rnl-words-0.4.json` in the repository, and Annex A of the standard gives each keyword's signature.

## Files

| File | Content |
| --- | --- |
| [references/language.md](references/language.md) | The short guide to RNL 0.4: how to read and write a file, with examples that read without errors, and a catalogue of every standard name |
| [references/packages.md](references/packages.md) | The standard packages of series 0.4: packages, definitions with their roles, symbols with their pins, the default symbol rules, sets, and `alias-zh`. Generated from the package files, so it is in step with them |
| [references/good-drawings.md](references/good-drawings.md) | How to write a file that a solver draws well: the points to write, the wiring rules and wires drawn by hand, the common mistakes, the check of the solved file and fine-tuning a solved drawing |
| [references/tools.md](references/tools.md) | Standard Markdown retrieval; checking and rendering tools: the `rnl` command downloaded or built from source and the npm package `@rich-netlist/runtime`; moving files of 0.3 with `rnl upgrade`; the solver's settings, such as `lines_order` and `virtual_weight`; web links and page-to-viewer text transfer; and desktop viewer installation and launch |
| [examples/ota5t.sp](examples/ota5t.sp) | A five-transistor OTA in SPICE, with its structure written and nothing placed |
| [examples/ota5t.scs](examples/ota5t.scs) | The same circuit in Spectre, with `//` records |
| [examples/ota5t-placed.sp](examples/ota5t-placed.sp) | The same circuit with every object placed, so that it is drawn completely |
| [examples/solver-cs-amp.sp](examples/solver-cs-amp.sp) | A common-source stage written for a solver: one column, the load and the output port to its right |
| [examples/solver-ota5t.sp](examples/solver-ota5t.sp) | A five-transistor OTA with its bias mirror written for a solver: the bias diode and the two branches in three columns, symmetric wiring |
| [examples/solver-cascode-mirror.sp](examples/solver-cascode-mirror.sp) | A cascode current mirror written for a solver: two columns of three |
| [examples/solver-miller-ota.sp](examples/solver-miller-ota.sp) | A two-stage Miller op-amp written for a solver: a sideways compensation network between the stages |
| [examples/solver-pi-follower.sp](examples/solver-pi-follower.sp) | A pi filter and a source follower written for a solver: a signal path from left to right |
| [examples/solver-sampler-inverter.sp](examples/solver-sampler-inverter.sp) | A transmission-gate sampler and an inverter written for a solver: switches turned by a pin direction, no static current |
| [LICENSE.md](LICENSE.md) | The licences of what this package holds |

The project: <https://github.com/rich-netlist/rnl>; its site, with the standard, the gallery of the standard packages and the viewers: <https://rich-netlist.github.io/rnl/en/>.

## Related skills

For exporting an existing Virtuoso schematic or library, use the companion
[`rnl-virtuoso` skill](https://github.com/rich-netlist/rnl-virtuoso/blob/main/skills/rnl-virtuoso/SKILL.md).
Read the installed skill if available; otherwise obtain its
[complete skill directory](https://github.com/rich-netlist/rnl-virtuoso/tree/main/skills/rnl-virtuoso),
including its linked references. Its instructions are maintained with the
exporter. After export, use this RNL skill to check, edit or draw the result.

The companion uses the existing `virtuoso` bridge skill and reuses a compatible
remote exporter. If the exporter is missing, obtain the user's consent before
installing the latest [upstream release](https://github.com/rich-netlist/rnl-virtuoso/releases/latest)
in their remote home directory, unless already authorized. Neither skill bundles
the exporter or bridge.
