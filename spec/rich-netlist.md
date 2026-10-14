# Rich Netlist (RNL): principles and boundaries

Status: principles. The language that follows them is defined in the standard, [RNL 0.2](rnl-0.2.md); the principles are not tied to a language version.

Language: this English text is the authoritative version. The Chinese version is [rich-netlist.zh.md](rich-netlist.zh.md); where the two differ, this English text prevails.

Source: moved on 2026-09-24 from `docs/standards/rich-netlist.md` on the `rich-netlist` branch of the Analog Canvas repository (`cascode-ai/analog-canvas`), commit `641e9c21d36b86937d543c1698cc661eae61e007` (2026-09-19). Only this source note and the license note below were added when it was moved; the text was not changed. Since then, this repository has been the only place where it is maintained. The text was written in Chinese and translated into English in October 2026.

License: published by the copyright holder under [CC-BY-4.0](../LICENSES/CC-BY-4.0.txt); the licenses of the parts of this repository are described in the [license notes](../README.md#license). The frozen copy in Analog Canvas remains available under the AGPL-3.0 terms of that repository.

## 1. Positioning and overview

Rich Netlist is an open metadata extension attached to a native circuit netlist. It mainly describes the structural meaning of a circuit and the design intent, and it may add visual information for the schematic.

> The native netlist defines the circuit; Rich Netlist describes how to understand and present that circuit. Semantic descriptions may exist without geometry, and geometry may be refined step by step, but neither may redefine the circuit.

RNL is positioned as a description of circuit meaning. It does not aim to reproduce a drawing exactly and does not require a unique rendering. Different implementations may produce different layouts and styles from the same description; detailed visual description is an optional, supplementary capability.

The goal is that the same file can be read by the existing simulator in the existing way, and also by tools that understand RNL, which draw the schematic from it. All extension information is placed in legal comments of the host language; stripping comments must not become a step required before simulation.

**RNL is:**

- a metadata extension attached to a native netlist, carrying its information in legal comments;
- an exchange format for the structural meaning of circuits, design intent and the visual information of schematics;
- a declarative presentation specification that allows purely semantic descriptions, exact geometry and descriptions of mixed granularity.

**RNL is not:**

- a standard that unifies different netlist languages or converts simulator syntax;
- a second definition of the circuit that creates devices, parameters or connections through drawing descriptions;
- a set of rules that checks whether the electrical meaning suggested by a drawing agrees with the original netlist;
- a specification that prescribes a single layout algorithm or a single rendering;
- a visual archive format that guarantees a drawing is unchanged after an editor exports and imports it again.

## 2. Design principles

Wording conventions: "must" and "must not" state requirements; "recommended" and "encouraged" state recommended practice and are not binding; "may" and "allowed" state permission. By their roles, implementations are divided into interpreters (they read the netlist and the RNL, obtain a consistent set of descriptions and give diagnostics; an interpreter used only for checking is also called lint), renderers (they draw the schematic from that set of descriptions), generators (they produce RNL descriptions; people, LLMs and export tools are all generators) and editors (they read, modify and write back RNL); "implementation" is the common name for all of them.

### 2.1 Netlist compatibility: embed safely in comments, use one sub-tag

1. **Live safely in the netlist's own comments.** RNL information must be embedded in legal comments of the original netlist so that the existing simulator treats it as ordinary comments and simulation compatibility is not affected. The aim is to cover the mainstream netlist formats, such as SPICE, Spectre, Eldo and HSPICE.
2. **Comment prefixes may differ, the sub-tag stays the same.** Different netlist formats may use their own comment prefixes, but the sub-tag that identifies RNL inside a comment must be the same: `@RNL`.
3. **Keep the original netlist when only RNL is modified.** The original netlist text and its ordinary comments must be kept, and circuit content must not be rewritten along the way.
4. **Not bound to a particular netlist format.** RNL neither defines nor unifies the syntax of host netlists. The names of host objects, their case, the character sets available to them and so on all follow the rules and limits of the netlist format and of the simulator itself. RNL's own names are defined by RNL. Interpreters and other implementations must state which netlist formats they support; how each netlist format is parsed is outside this specification.

The sub-tag is `@RNL`. A survey of the documentation of simulators and netlist formats found no tool that gives `@RNL` in a comment a special meaning. The comment prefixes with a special meaning (such as `*#`, `*.`, `*|`) all put a character directly after the comment character, so leaving one space after the comment character avoids them. How comments are written in each netlist format is defined by the syntax specification and needs to be verified by tests (section 4, item 1).

### 2.2 One source of meaning, no checking: electrical facts are defined only by the original netlist

RNL does not duplicate the definition of the connections, device types, names or parameters of the original netlist. It may refer to this information, explain structural meaning and set display aliases. Drawing descriptions do not create or change electrical facts.

**No-checking principle: RNL is not responsible for checking whether the electrical meaning that a drawing might suggest agrees with the original netlist, and does not declare a drawing description illegal for that reason.** For example:

- Overlapping pin positions of two devices do not mean that their terminals are electrically connected, and do not make the RNL description illegal.
- A wire may be drawn between two terminals that are not electrically connected; the electrical node that the wire is associated with need not be the node of either terminal.
- Lines that touch, cross or overlap carry only geometric information; they must not be taken as an electrical connection or used to change the original netlist.

Generators and renderers decide for themselves whether to check such cases, whether to give a warning and how to present them. Such checks and warnings are not requirements of the format specification; the purpose is to lower the barrier to implementation and to leave the choice to implementations. Checks of syntax, of references and of basic semantics within a single description are still required by the standard; see 2.4 for the boundary of checking.

| Information | Boundary of authority |
| --- | --- |
| Device identity, type, parameters, terminal order, connections, subcircuit interface | Defined by the host netlist; RNL only refers to them |
| Display aliases, text positions, symbol appearance, visual paths | Described by RNL; they do not change electrical objects |
| "M1 and M2 form a differential pair" | Explains existing objects and their roles; creates no device or connection |
| "Show the parameter expression of R1" | Refers to the host value; keeps no second authoritative copy of the parameter |
| Virtual parts (supplies, grounds and other parts that exist only for display) | Defined and owned by RNL; do not exist electrically |
| Note objects (free-form text, graphics, frames and other purely drawn objects) | Defined and owned by RNL; have no electrical meaning at all |

RNL keeps no second "device–terminal–node" connection table, and it must not add parts that do not actually exist, extra nodes or zero-ohm resistors to the original netlist for the sake of the drawing; objects that exist only for display are described as virtual parts. The net associated with a visual path, the terminal that an anchor is bound to and the geometric position are expressed separately; they need not be electrically consistent on the drawing, and they must not override the connection facts of the original netlist.

**Virtual parts.** RNL may describe virtual parts: parts that do not exist electrically and exist only for display, such as supplies and grounds. A virtual part exists only in the RNL description and does not appear in the original netlist. Like a device it has terminals and may be connected to wires, and it may refer to host objects (for example, a ground associated with a net), but it is only for display, creates or changes no electrical facts, and the no-checking principle applies to it as well. The standard package defines a set of virtual parts; the set is not closed and grows with the versions of the standard package. Users may also define their own virtual parts outside the standard package; where they take effect is described in 2.5.

**Note objects.** Free-form text, graphics, frames and other annotations are note objects. A note object differs from a virtual part: it is a purely drawn object, has no terminals and has no electrical meaning at all.

### 2.3 Extensibility: restrained, but open to incremental extension

- **Define only what is necessary.** Early versions do not try to settle every meaning and detail, but the meaning of what is defined must be clear.
- **New features stay compatible with old content as far as possible.** Later versions add meanings, symbols, virtual parts or layout capabilities through extension and avoid changing the meaning of existing definitions.
- **Unknown content is handled like errors.** Keywords or extension content that an implementation does not know are treated in the same way as syntax errors, as in 2.9: the statement that contains them fails and is reported as an error; keeping it unchanged when saving is recommended. The exception is an unknown parameter field in the head of an RNL record: only a warning is given and the field is ignored, and the rest of the record is processed as usual; this leaves room for later extensions.
- **Declare versions.** Implementations must state which RNL versions they support; files must also be able to declare the RNL version they use.

Versions are defined in the standard (16.1); an extension mechanism is left to a later version (16.4).

### 2.4 High-level meaning: describe topology, intent and visual constraints explicitly

- **High-level descriptions are allowed.** Topologies such as differential pairs, RC filters and cross-coupling, design intent such as matching and biasing, and visual constraints such as symmetry and signal flow may be expressed, without first writing device coordinates and wiring.
- **Semantics are clearly defined.** Each kind of semantic description must have a clear keyword and a clear definition, saying what it means and what roles the referenced objects play, so that generators and renderers understand it the same way. The concrete vocabulary is provided by the standard package and is not part of the language standard itself.
- **Nested descriptions are allowed.** The result of a semantic description may be named and referred to by other descriptions; for example, a differential pair is declared first and then used, together with the tail current source, to form a differential input stage. A named semantic object refers to netlist objects of its own level and belongs only to that level. How this is written is defined in the standard (9.4).
- **Basic checking is required by the standard.** Syntax errors, unknown keywords, invalid references and basic semantic conflicts within a single description are basic errors defined by the standard, for example a reference to a netlist object that does not exist, or a net put into a role that requires a device. The definition of each kind of semantic description expresses these rules with its role declarations and its content. Implementations must report basic errors as errors, and the statement in error fails as in 2.9.
- **Checking conflicts between descriptions and consistency with the netlist is not required.** For example, if A and B are declared a differential pair and B and C another, RNL does not require checking such a conflict between descriptions, even if the two cannot both hold in the actual topology. Nor does RNL require verifying that semantic descriptions agree with the actual connections, types or parameters of the netlist; implementations may provide such checks and warnings themselves.

### 2.5 Scope: set by position, references limited to the level, definitions inherited downward

RNL descriptions must have a clear scope, to support subcircuits: it determines which circuit a description applies to and where the object references in it are resolved.

- **The scope is set by the place of declaration by default, and may also be specified.** A statement written inside a subcircuit belongs to that level of the circuit; the part of a file that belongs to no subcircuit is the file level of that file. The file level of the main file is the top-level circuit; the file level of an included file is not a circuit and carries only definitions and rules. Definitions given at a file level take effect for the subcircuits that the file defines and the files it includes, and do not affect the file that includes it. A statement may also specify a target subcircuit; the target must be a subcircuit that lies textually within the place of declaration, and statements inside a subcircuit must not specify one. Without a target, the scope follows the place of declaration.
- **References to netlist objects are limited to the level of the scope.** References to devices, nets, terminals and other netlist objects are resolved only within the level of the statement's scope, never across levels. For example, different subcircuits can each have a device named M1, and their descriptions do not affect each other.
- **RNL's reusable definitions are inherited downward, and lower levels may override them.** The reusable objects that RNL itself defines (such as user-defined virtual parts, symbols and possibly style definitions) take effect at their own level and every level below. The lower levels are determined by textual containment: the lower levels of a file are the subcircuits it defines and the files it includes, regardless of where a subcircuit is instantiated, so each subcircuit has only one effective set of definitions. A definition must be able to limit where it takes effect: the current level only, or inherited by the lower levels; without a limit, it is inherited by default. Rules that select objects by conditions are treated like definitions: a rule written at a level acts by default at that level and at each lower level separately, each time only on the objects of that level, and it may also be limited to the current level. A lower level may define an object with the same name again to override the definition of an upper level; the override takes effect only at that lower level and below and does not affect the upper level. Levels take precedence over order: the definition of a lower level wins over the definition with the same name at an upper level, regardless of the order of statements. Within one level a name is bound to only one definition: defining the same name again is an error and the earlier valid definition is kept; repeated descriptions of the same object still follow 2.9: the later one wins. Descriptions written directly for particular objects (for example, placing a ground somewhere) belong, like other descriptions, only to their own level and are not inherited downward.
- **An included file does not affect the file that includes it.** An included file inherits the definitions of the file that includes it; its own definitions and rules act only on the subcircuits it defines and the files it includes in turn. It is recommended that the definitions a subcircuit depends on be provided together with the subcircuit or its file, and that a description that refers to external parts not depend on external RNL definitions.
- **The block symbol of a subcircuit is visible to parent circuits.** A block symbol is the appearance of a subcircuit's interface; it is defined with the subcircuit and used by the parent circuits that instantiate it. It is the exception to definitions taking effect only downward.

This arrangement makes merging netlists easier: definitions can move safely with a subcircuit and its file, the definitions a subcircuit brings with it do not affect the parent circuit it is merged into and are not rewritten by definitions with the same name in the parent; including someone else's file does not change one's own content, and statements inside a subcircuit cannot affect anything outside it.

How scope is declared and how definitions are inherited are defined in the standard (2.6, 6.1, 6.8). Features such as visual containers may be considered as later extensions; they are not part of this principle.

### 2.6 Mixed granularity: semantic and geometric descriptions may be mixed

RNL allows high-level meaning to exist together with exact coordinates, orientations, wiring and other geometric information. For example, a differential pair and its symmetry intent may be described while the positions of some devices or the path of some wire are also specified.

A wire usually starts and ends at terminals, and coordinates, bends and so on may be added in between; such a wire does not depend on the actual graphics of the symbols, and renderers may adapt it to the symbols they use. RNL also allows a wire that connects to no terminal and starts and ends at coordinates; this does not violate the specification but is uncommon. A wire may connect several terminals: it is written as several wires that share points, and wires count as joined only when they share the same point or terminal; junction dots are marked explicitly.

The specification does not require these descriptions to be free of conflicts, nor does it require checking the conflicts between them. When conflicts are found, the default resolution follows 2.9; the actual rendering and any hints are up to the implementation. Checks of syntax, of references and of basic semantics within a single description still follow 2.4.

### 2.7 Detailed expression: visual detail may be defined where needed

RNL allows the renderer to decide the default symbols and drawing style, and also allows authors to define visual details precisely as needed, such as device symbols and the block symbols of subcircuits, including line-level graphic descriptions or references to symbol resources. Detailed description is an optional capability, not something every input must provide.

Exact coordinates that depend on symbol graphics (for example, wire end points placed right on pin positions) stay consistent between renderers only when the file defines or refers to the symbols used. When the renderer decides the symbols, it is recommended that symbol-related positions be expressed preferably by terminals rather than by absolute coordinates (see 2.6).

RNL and the symbol resources it refers to are pure data: processing them executes no code and does not implicitly access the network.

Graphics, pins and symbols are defined in the standard (9.2); symbol resources are provided as packages (16.2).

### 2.8 Friendly to people and models: clear syntax, readable helper text allowed

- **The core syntax is clear.** Object references, keywords and parameters use clear markers and definitions, so that third parties can implement them easily and LLMs can generate them directly; one declaration per statement and a fixed core structure are encouraged.
- **Readable helper text is allowed.** English words, spaces, digits and agreed punctuation may be inserted outside the formal markers as optional in-line explanations, for reading without a renderer. LLMs are encouraged to use simple, fixed explanatory sentence patterns.
- **Helper text takes no part in machine interpretation.** The machine meaning is decided only by the formal markers; negation, conditions, roles, numbers with a semantic effect and the like must be expressed in the formal syntax; ordinary text must not be relied on to change the meaning.
- **Partial generation and repair are supported.** Describing only part of a circuit is allowed, and unnecessary identifiers and coordinates are kept to a minimum; reported errors must point to the statement, field or reference concerned, to make local repair easy.
- **The reliability of generation is verified by tests.** Compare the rates of correct syntax, references and meaning generated with and without helper text, check whether explanatory text misleads, and count markers that lost a delimiter and were silently ignored as helper text — the standard does not require such errors to be detected; it is not assumed in advance that natural-language wording improves accuracy.

In the standard, the following two statements have the same machine meaning:

```text
#M1(in_p) #M2(in_n) @AS $DIFFPAIR
#M1(in_p) and #M2(in_n) are treated @AS $DIFFPAIR
```

The words between the tokens are free text, which does not change the meaning; a `not` in free text does not negate the statement (standard 3.3).

### 2.9 Processing order, tolerance and conflict resolution: netlist first, errors ignored, conflicts resolved by precedence

The host netlist is parsed first, then all RNL descriptions are collected: statements in error are ignored, descriptions without conflicts are all kept, and conflicting ones are resolved by a definite precedence, which gives a consistent set of descriptions from which the drawing is rendered. The order in which statements appear is used only to decide precedence in a conflict and does not affect how references are resolved; the place where a statement is written decides its default scope (see 2.5).

- **The host netlist comes first.** The complete host netlist is parsed first to determine the objects that can be described, then RNL is processed. RNL descriptions must not change netlist information; references to host objects are resolved against the complete netlist, and the netlist statement of a referenced object need not appear before the RNL statement.
- **RNL's own definitions need no particular order.** What RNL itself defines (such as symbol descriptions) may be used before it is defined; such dependencies are resolved against the complete RNL description, and where a definition is visible is described in 2.5.
- **A statement in error fails and is reported; everything else is processed as usual.** Syntax errors, unknown keywords and other basic errors (see 2.4) are handled in one way, without being divided further into categories: the statement in error fails and is reported as an error, and interpretation and rendering go on with the remaining descriptions. Checking and interpreting are the same process, and the standard defines only one set of diagnostics; implementations must let users see the diagnostics, and how they are presented is up to the implementation.
- **Errors are handled locally.** A statement in error fails; it does not automatically make other statements fail and does not override any existing description. Statements that depend on a failed description fail with it, whatever the order in which they appear, and are reported as errors too. For example, if a symbol description cannot be resolved, the statements that depend on that symbol description fail as well.
- **Conflicts are resolved by precedence.** Geometric descriptions such as positions, orientations and layout constraints each have a unique precedence: the priority that the description gives is compared first, and with equal priority the one that appears later wins. They are taken in from the highest precedence to the lowest, one by one; a description that has no solution together with those already taken in is discarded, so the result is unique. For example, if the position of the same device is given twice with the same priority, the later one wins. Only the conflicting part of the information is discarded: the two coordinates of a position are resolved separately, and the parts of an earlier description that do not conflict stay valid. For other information that can be overridden, the later description likewise wins. Overriding and discarding are legitimate outcomes, not errors, and whether to give a hint is up to the implementation; so an existing description may be modified either by rewriting it in place or by appending a new description after it. Deciding precedence by order of appearance happens only within the same level of the circuit; between different levels the relation of levels in 2.5 decides, not the order of appearance.
- **Saving does not change content because of diagnostics.** It is recommended that editors, when saving a file, not delete, repair or rewrite statements because of diagnostics, and only give a warning. This is a recommendation for implementations, not a constraint of the standard.

This is basic guidance for processing; implementations may adopt stricter strategies, such as stopping or rejecting the whole RNL input. This handling does not widen the range of required checks: conflicts between high-level meanings of different descriptions and the agreement of meaning and drawing with the actual netlist still need not be checked; "consistent" is limited to what the implementation actually checks.

## 3. Recommendations for implementations

RNL is fundamentally a semantic description, not a visual archive that guarantees a drawing is unchanged after export and import. The following are recommendations for editors and renderers and are not binding:

- **The granularity of export is a choice.** On export, users may be offered a choice of exact coordinate-level RNL, simple semantic RNL, or a plain netlist without RNL; all are within what the specification allows.
- **Present semantic structure in a suitable form.** For example, show a differential pair as a group rather than as devices that can be dragged separately; when the user drags one of the devices, prompt the user to choose between removing the semantic description and keeping it while overriding the device's placement with coordinates (see 2.9).
- **Keep the meaning that an edit did not break when writing in semantic form.** All semantic information not broken by the current edit is kept.
- **Distinguish "modifying only RNL" from "regenerating the netlist".** When only RNL is modified, the original netlist is kept as in 2.1; regenerating the netlist from the editor's own circuit model is another operation, also within what the specification allows.
- **Objects without any description are left to the renderer.** How they are presented is up to the renderer; a reasonable approach is to collapse them into a collection of parts, or to offer an option to hide them.
- How diagnostics are handled when saving is described in 2.9.

## 4. Further work

The following work turns the settled principles into a concrete format definition:

1. **Verifying comment compatibility.** The sub-tag is settled as `@RNL` (2.1), and the documentation of how each netlist format writes comments has been surveyed. Verify by tests the safe comment prefixes, positions, encodings and length limits of each netlist format, recording the tool versions and parsing modes the tests cover.
2. **Formal syntax.** Define the markers of objects, keywords and parameters; escapes, helper characters, statement boundaries and the multi-line form; and how versions are declared and diagnostics located.
3. **Scope and references.** Define how scope paths are written and built, how terminals are identified and how references across files work, and the concrete rules by which definitions are inherited and overridden along the levels of the circuit (including descriptors that limit where a definition takes effect, how the effective set of definitions is determined when the same subcircuit is instantiated by several parents, and the precedence between descriptions targeted by a path and the target level's own descriptions); rules such as the case of host object names follow the host netlist format and are not defined by RNL.
4. **The first semantic vocabulary and standard virtual parts.** Settle the keywords, the definitions, the object roles and the basic validity rules of single descriptions, and how semantic descriptions are named and referred to when nested. The first semantic vocabulary and the standard virtual parts belong to the standard package, not to the language standard itself, and will be built up through experiments after the prototype is complete.
5. **Optional geometric and symbol expression.** Define coordinate units, rotation and mirroring, wires (terminal end points, bends, the pure-coordinate form, and how connections of several terminals are expressed, for example virtual nodes in the middle of a wire or multi-way wire labels), note objects, the graphic format, terminal anchors and how symbol resources are referred to, and how users define their own virtual parts.
6. **Version and extension mechanisms.** Define the rules of version compatibility, extension identifiers, and how unknown content is kept when reading and writing.
7. **Ordered processing, overriding and error recovery.** Define the processing order across files, how descriptions written as a group hold, how conflicts are determined and the granularity of overriding (including which descriptions are overridden when a new description conflicts with several existing ones together), and how to recover to the next statement when a statement cannot be parsed.
8. **Test corpus and evaluation.** Build test corpora for comment compatibility, basic checking, ordered overriding and tolerance, and keeping content when reading and writing, and evaluate the correctness of LLM generation and local repair.

Version 0.2 of the standard settles items 2, 3, 5 and 7, and the part of item 6 about versions and keeping unknown content. Still open are item 1 (comments are surveyed in the documentation, but not yet verified by tests), item 4 (the content of the standard package), the extension mechanism of item 6, and item 8 (a conformance corpus).
