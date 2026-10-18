---
name: rnl
description: Read, write, check and draw RNL (Rich Netlist) 0.3, the drawing of a schematic written into a SPICE or Spectre netlist as `@RNL` comment records. Use it when a netlist has `* @RNL` or `// @RNL` records or `.rnl` package files, when asked to place, wire, annotate or draw a circuit's schematic from a netlist, to pick symbols or define structures such as a differential pair or a current mirror, or to check a file with `rnl check`, `@rich-netlist/runtime` or the RNL viewers. Covers the language 0.3, the standard packages of series 0.3 and the tools.
license: See LICENSE.md (MIT, CC-BY-4.0 and CC0-1.0, by file)
compatibility: Reading and writing RNL needs no tools. Checking needs the rnl command (Rust 1.85 or later, built from source) or Node 18 or later with the npm package @rich-netlist/runtime.
---

# RNL (Rich Netlist) 0.3

RNL is written in the comments of a SPICE or Spectre netlist. It says how the circuit is drawn as a schematic (which symbols, where things go, how wires run, what the parts mean) while the netlist alone defines the circuit. Simulators ignore the comments, so a netlist with RNL records still simulates as before. This skill covers the language **0.3** (the standard, revision 1) and the standard packages of the **series 0.3**; files of 0.2 use other names and are read with the series 0.2.

A complete file, one of the standard's own examples, with the model card that a transistor needs to be drawn (a transistor is drawn by the type of its model card, so one whose card the netlist lacks is not drawn):

```spice
* Five-transistor OTA, input stage
* @RNL(type="header") { @VERSION("0.3")}
.model nch nmos
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   %dp1 = #M1(in_p) and #M2(in_n) treated @AS $DIFFERENTIAL_PAIR;
*   {#M1 #M2} @ALIGN("y"), i.e., same y;
*   #M5 is at @PLACE(20, 6) => it @ORIENT("R0");
*   %g0 = $GROUND is @ATTACH to &vss;
*   net &vss has a @WIRE connecting {#M5:3 %g0:p}
* }
.ends ota5t
.end
```

The record says that M1 and M2 are a differential pair sharing one y, where M5 goes and how it is turned, and that the net `vss` gets a ground wired to M5. `#M1` is an instance, `&vss` a net, `#M5:3` a terminal, `%dp1` a name of the RNL, `$GROUND` a definition of the standard package; `@NAME(…)` are keywords, and words such as `and`, `treated` and `is at` are free text that only makes the statement read like a sentence.

## How to work

1. **Read [references/language.md](references/language.md) once, all of it**. It is the short guide to the language. For anything it leaves open, the standard decides: <https://rich-netlist.github.io/rnl/en/spec/0.3/> (normative).
2. **Look every `$NAME` up in [references/packages.md](references/packages.md)** before you write it: the definitions, symbols and sets of the standard packages, which symbol an instance gets by default, and what the alias package `rnl-zh` is. A name that is not there is an error; do not guess names from other EDA tools or from 0.2 (`$RES` and `$GND` of 0.2 are `$RESISTOR` and `$GROUND` in 0.3).
3. **Write the records into the netlist's comments** and leave the netlist's own lines alone. A record belongs to the level it is written in: inside a `.subckt` for that subcircuit, outside any for the file's top level. Each line of a record is a whole-line comment: the comment character in column 1 (`*`; in Spectre `//` or `*`), then nothing or one space, then the text. `*@RNL` and an indented `* @RNL` are not record lines and cut the record off.
4. **Check the file** with a tool of [references/tools.md](references/tools.md) after every change, and fix what it reports until there are no errors and no warnings. A file without positions says in a hint that some objects are left to a solver and not drawn; that is not a problem.
5. **Draw it** when the user wants to see it: `rnl render FILE -o DIR` writes an SVG for each drawing level, the web viewer at <https://rich-netlist.github.io/rnl/viewer/> opens a file in the browser, and the desktop viewer does the same on Windows. Only what has coordinates is drawn.

## Rules that save time

- The netlist defines the circuit; RNL cannot change a device, a parameter or a connection. If a record contradicts the netlist, the netlist wins.
- Records with `type="generated"` are what a solver wrote back. Leave them as they are; `rnl strip-generated FILE -o OUT` takes them out, and a solver writes them again.
- Put the header record `* @RNL(type="header") { @VERSION("0.3")}` first in a file you write, below the title line of a SPICE netlist. A main file without one is read as 0.3 anyway; an included file without one takes the version of the file that includes it.
- Do not copy a stranger's RNL from memory or from 0.2 material: check each name and each keyword against the guide and the packages, and each file with a tool.
- When a message is unclear, search the standard for the keyword it names; the standard word list is `spec/rnl-words-0.3.json` in the repository, and Annex A of the standard gives each keyword's signature.

## Files

| File | Content |
| --- | --- |
| [references/language.md](references/language.md) | The short guide to RNL 0.3: how to read and write a file, with examples that read without errors |
| [references/packages.md](references/packages.md) | The standard packages of series 0.3: packages, definitions with their roles, symbols with their pins, the default symbol rules, sets, and `rnl-zh`. Generated from the package files, so it is in step with them |
| [references/tools.md](references/tools.md) | How to check and draw files: the `rnl` command built from source, the npm package `@rich-netlist/runtime`, the web viewer and the desktop viewer |
| [examples/ota5t.sp](examples/ota5t.sp) | The five-transistor OTA of the standard, in SPICE, with the records of the example above |
| [examples/ota5t.scs](examples/ota5t.scs) | The same circuit in Spectre, with `//` records |
| [examples/ota5t-placed.sp](examples/ota5t-placed.sp) | The same circuit with every object placed, so that it is drawn completely |
| [LICENSE.md](LICENSE.md) | The licences of what this package holds |

The project: <https://github.com/rich-netlist/rnl>; its site, with the standard, the gallery of the standard packages and the viewers: <https://rich-netlist.github.io/rnl/en/>.
