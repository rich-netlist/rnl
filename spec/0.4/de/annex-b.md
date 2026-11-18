<!-- translated from en/annex-b.md sha256:f5f5cbcacdf4 -->

## Anhang B Standardsätze (informativ)

Ein Standardsatz ist eine Anweisung in der Standardform, bei der zwischen ihren Tokens Freitext eingefügt ist, sodass sich die Anweisung wie ein Satz liest. Der Freitext ändert die Bedeutung nicht (3.3); die Regeln für den Wortlaut stehen in 4.5. In den Tabellen steht `…` am Satzanfang für die Segmente davor in einer Pipe, und `{…}` oder ein abschließendes `…` für ausgelassenen Inhalt.

### B.1 Platzierung und Ausrichtung

| Schlüsselwort | Fall | Standardsatz |
|---|---|---|
| PLACE | zwei Achsen | `#M1 is at @PLACE(10, 20)` |
| | eine Achse | `the x of #M1 is at @PLACE(x=10)` |
| | von einem Editor geschrieben | `the x of #X2 is at @PLACE(x=3, priority="high")` |
| | in einer Pipe | `… => it is at @PLACE(40, 0)` |
| ORIENT | gedreht | `#M2 @ORIENT("R90")` |
| | gespiegelt | `#M2 @ORIENT("MX"), i.e., flipped upside down` |
| | | `#M2 @ORIENT("MY"), i.e., flipped left-right` |
| | | `#M3 @ORIENT("MXR90"), i.e., flipped upside down, then rotated 90°` |
| | | `#M3 @ORIENT("MYR90"), i.e., flipped left-right, then rotated 90°` |
| | eines von beiden | `#M5 @ORIENT(["R0", "MY"]), i.e., not turned or flipped left-right` |
| | in einer Pipe | `… => they @ORIENT("R90")` |
| ORIENT-RELATION | zwei Objekte | `from #M1 to #M2, they have @ORIENT-RELATION("MY"), i.e., flipped left-right` |
| | eines von beiden | `from #M3 to #M4, they have @ORIENT-RELATION(["R0", "MY"]), i.e., the same or flipped left-right` |
| | dieselbe Ausrichtung | `from #M5 to {#M7 #M8}, they have @ORIENT-RELATION("R0"), i.e., the same orientation` |
| LAYER | | `#X is on @LAYER("bottom")` |
| | in einer Pipe | `… => they are on @LAYER("top")` |
| GROUP | | `{#M1 #M2 #M5} as a @GROUP` |
| | benannt | `%s1_set = {#M1 #M2 #M5} as a @GROUP` |
| | Abstand | `{#M3 #M4} as a @GROUP(spacing_x=-1), i.e., one grid unit tighter in x` |

### B.2 Layout-Bedingungen

| Schlüsselwort | Fall | Standardsatz |
|---|---|---|
| ALIGN | gleiches y | `{#M1 #M2} @ALIGN("y"), i.e., same y` |
| | gleiches x | `{#M1 #C1} @ALIGN("x"), i.e., same x` |
| | niedrige Priorität | `{#M7 #M5} @ALIGN("y", priority="low"), i.e., same y` |
| | in einer Pipe | `… => they @ALIGN("y")` |
| ORDER | mit einer Richtung | `{#M1 #M2 #M3} are in @ORDER("left-to-right")` |
| | in einer Pipe | `… => they are in @ORDER("bottom-to-top")` |
| DISTRIBUTE | | `{#R1 #R2 #R3} are in @DISTRIBUTE("left-to-right"), i.e., in equal steps` |
| | in einer Pipe | `… => they are in @DISTRIBUTE("bottom-to-top"), i.e., in equal steps` |
| DISTANCE | mit einer Richtung | `from #M1 to #M2, they have @DISTANCE("bottom-to-top", min=4)` |
| | von einer Menge | `from {#M6 #M7 #CL} to %pout, they have @DISTANCE("left-to-right", min=2)` |
| TABLE | | `@TABLE layout: {{#M1 #M2 #M3} {#M4 {#M5 #M6} #M7}}` |
| | quer | `@TABLE(transpose=true) layout: {{#M1 #M2} {#M3 #M4}}` |
| PRIORITY | | `%c1 has @PRIORITY("disabled")` |
| | in einer Pipe | `… => they have @PRIORITY("low")` |
| SOLVER-CONFIG | | `@SOLVER-CONFIG(spacing_x=2, spacing_y=1)` |

- `the x of` (oder `the y of`) wird nur vor einem PLACE geschrieben, das eine Achse angibt (B.1); ALIGN erhält stattdessen die Erläuterung `, i.e., same x`.
- Das x ist das der Position jedes Mitglieds: bei einer Instanz der Punkt, den PLACE setzt (der Ursprung ihres Symbols), nicht der Mittelpunkt ihres Umrisses; eine Menge steht für jedes ihrer Mitglieder (14.1).

### B.3 Darstellung

| Schlüsselwort | Fall | Standardsatz |
|---|---|---|
| SET-SYMBOL | ein Objekt | `#M1 @SET-SYMBOL to $NMOS` |
| | eine Menge | `{#M1 #M2} @SET-SYMBOL to $NMOS` |
| | das Blocksymbol einer Teilschaltung | `this subcircuit # @SET-SYMBOL to $NAND2_SYM` |
| | in einer Pipe | `… => they @SET-SYMBOL to $PMOS` |
| SET-STYLE | | `{#CC #RZ} @SET-STYLE to $orange` |
| | in einer Pipe | `… => they @SET-STYLE to $grey` |
| DEFINE-STYLE | | `$orange = @DEFINE-STYLE(linecolor="#e07000", textcolor="#e07000") {}` |
| | Zusammenführen eines anderen Stils | `$wide = @DEFINE-STYLE(linewidth=2) based on {$orange}` |

### B.4 Symbole und Grafiken

| Schlüsselwort | Fall | Standardsatz |
|---|---|---|
| DEFINE-SYMBOL | | `$TRI = @DEFINE-SYMBOL {…}` |
| PIN | | `at (-4, 0) there is a @PIN(terminal="in", direction="left")` |
| LINE | | `through {(0, 0) (4, 0)} runs a @LINE` |
| ARC | | `through {(0, 2) (2, 0) (0, -2)} runs an @ARC` |
| BEZIER | | `the control points {(0, 0) (1, 1) (2, 1) (3, 0)} make a @BEZIER` |
| CIRCLE | | `centered at (0, 0), a @CIRCLE(radius=1)` |
| RECTANGLE | Mittelpunkt und Größe | `centered at (0, 0), a @RECTANGLE(width=4, height=2)` |
| | zwei Ecken | `with corners {(0, 0) (4, 2)}, a @RECTANGLE` |
| ELLIPSE | Mittelpunkt und Größe | `centered at (0, 0), an @ELLIPSE(width=4, height=2)` |
| | Ecken seines Rahmens | `in the box with corners {(0, 0) (4, 2)}, an @ELLIPSE` |
| TEXT | in einem Symbol | `on ? there is a @TEXT("${name}", key="name") => it is at @PLACE(-2, 2.5)` |
| ATTACH | in einem Symbol | `$ARROW is @ATTACH to (0, -1.5) => …` |

- In der Szene als Anmerkungen geschriebene Grafiken (13.4) lesen sich ebenso.

### B.5 Leitungen und Text

| Schlüsselwort | Fall | Standardsatz |
|---|---|---|
| WIRE | | `net &n1, through {#R1:2 %j #R2:1}, has a @WIRE` |
| | ein Bus | `net &D<3:0>, through {%p1 %p2}, has a @WIRE, which is a bus` |
| | ein Leitungsmodus | `net &in, through {#VIN:1 #R1:1}, has a @WIRE(mode="vertical-horizontal")` |
| ROUTE-NET | | `net &n1 is drawn with @ROUTE-NET` |
| | eine Menge | `nets {&a &b} are drawn with @ROUTE-NET` |
| | in einer Pipe | `… => it is drawn with @ROUTE-NET` |
| ROUTE-OBJECT | | `the pins of {#M1 #M2 #M5} are joined with @ROUTE-OBJECT` |
| UNROUTE-NET | | `net &vdd is left out with @UNROUTE-NET(priority="high")` |
| UNROUTE-OBJECT | | `the pins of #M3 are left out with @UNROUTE-OBJECT` |
| TEXT | auf einem Objekt | `on #M1 there is a @TEXT("W/L = ${w}/${l}", key="wl", dy=-3)` |
| | an einem Punkt | `at (_, _) there is a @TEXT("RC low-pass") => named %title` |
| | in einer Pipe | `… => on them there is a @TEXT("${w}", key="size")` |

- Das Benennen eines Punkts, etwa `%j = (_, _)`, nimmt keine zusätzlichen Wörter.

### B.6 Definitionen und Anwendungen

| Schlüsselwort | Fall | Standardsatz |
|---|---|---|
| DEFINE | | `$PAIR = a definition, @DEFINE(roles=["a", "b"], parameters=[["gap", 2]]) {…}` |
| AS | drei Rollen | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N` |
| | eine Rolle, die eine Menge annimmt | `{#M5 #M7 #M8} are treated @AS $MATCHED` |
| | ein Objekt | `#M3 is treated @AS $DIODE_CONNECTED` |
| | mit einer Priorität | `#M1(left), #M2(right) and %ax(axis) are treated @AS(priority="high") $MIRRORED_LAYOUT` |
| | ein Parameter der Definition | `{#R1 #R2} are treated @AS(gap=6) $SPACED` |
| | eine Anwendung in einer Rolle | `%dp(pair) and %cm(load) are treated @AS $OTA` |
| | benannt | `%dp = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N` |
| | in einer Pipe benannt, mit einem weiteren Objekt | `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N => named %dp => it and %cm are treated @AS $OTA` |
| ATTACH | auf einem Netz | `$GROUND is @ATTACH to &vss` |
| | auf einem Anschluss | `$NO_CONNECT is @ATTACH to #M1:4` |
| | auf einem Punkt | `$JUNCTION is @ATTACH to %j` |
| | zwei Rollen | `$DIFF_PORT is @ATTACH to &inp(a) and &inn(b)` |
| | gefolgt von weiteren Segmenten, benannt | `$GROUND is @ATTACH to &vss => it is at @PLACE(24, 38) => named %g0` |
| | auf jedem von mehreren Objekten | `$SIZE_LABEL is @ATTACH to {#M1 #M2 #M3}` |
| | auf einer Menge | `$GROUND_STUB is @ATTACH to $GROUND_FILL_SET` |
| | die Ergänzung, in einer Pipe | `among the instances at each level in # and below, @SELECT-INSTANCE(recursive=true) => among the pins of them, @SELECT-PIN(net="vss", wired=false) => $GROUND_STUB is @ATTACH(gap=3) to them` |

- AS wird verwendet, wenn die Objekte zusammen das Ding bilden, etwa eine Schaltungsstruktur, eine Platzierung oder eine lokale Zeichenweise wie `$RC_STAGE`; ATTACH, wenn das Ding zu den Objekten hinzugefügt wird, etwa eine Masse, eine Port-Markierung, eine Nicht-verbunden-Markierung, ein Knotenpunkt, eine Beschriftung für jede Instanz oder die Ergänzung. Beide Schlüsselwörter tun dasselbe; nur der Platz der Definition unterscheidet sich (9.4).
- `gap` ist ein Parameter von `$GROUND_STUB`, der Mindestabstand vom Pin hinab zum Massesymbol (standardmäßig 1). Wie die Rollennamen `left` und `axis` ist er ein in einer Definition vergebener Name, kein Standardwort.

### B.7 Auswahlen

Eine Auswahl wird als Wahl unter Kandidaten geschrieben: `among <candidates>, @SELECT-…(conditions)`. Der Wortlaut der Kandidaten hängt von der Art des Bereichs und von der ausgewählten Art ab:

| Bereich | Ausgewählt | Kandidaten |
|---|---|---|
| `#` | beliebige Art | `the <kind> in # only` |
| `#`, in einer Anweisung, die die Ebenen darunter erreicht (10.1) | beliebige Art | `the <kind> at each level in # and below` |
| eine Menge der ausgewählten Art (ein Filter) | jene Art | `{…}` |
| Netz | Instanzen | `the instances connected to &en` |
| Netz | Anschlüsse | `the terminals connected to &en` |
| Netz | Leitungen | `the wires on &en` |
| Instanz | Anschlüsse | `the terminals of #R1` |
| Instanz, Bündel, Anwendung mit Symbol | Pins | `the pins of #M1` |
| Anschluss | Pins | `the pins of #X1:vdd` |
| Netz | Pins | `the pins on &en` |
| das Ergebnis einer Pipe | Pins | `the pins of them` |
| Instanz | Netze | `the nets connected to #M1` |
| Anschluss | Instanzen | `the instances owning #M1:g` |
| Anschluss | Netze | `the nets connected to #M1:g` |
| Leitung | Netze | `the nets of %w13` |
| Instanz, Anschluss, Leitung, Netz, Grafik | Beschriftungen | `the labels attached to #M1` |
| Anwendung (keine Anwendungen unter ihren Mitgliedern) | beliebige Art | `the members of %dp` |
| Anwendung (Anwendungen unter ihren Mitgliedern, oder nicht bekannt) | beliebige Art | `all members of %st1 at any depth` |
| Anwendung | Anwendungen (SELECT-USAGE) | `%st1 itself and the usages inside it` |
| beliebiges Objekt | die Anwendungen, die es enthalten (SELECT-ANCESTOR-USAGE) | `the usages containing #M1` |
| eine Menge mehrerer Arten (Art nicht bekannt) | | stattdessen `for %mixed_set, @SELECT-…` schreiben |

- `<kind>` folgt dem Schlüsselwort: instances, terminals, pins, nets, wires, labels, annotations, usages.
- In der Mitte einer Pipe wird der Bereich in den Kandidaten `them` geschrieben, etwa `=> among all members of them at any depth,`. Ob die zuvor ausgewählten Anwendungen Anwendungen unter ihren Mitgliedern haben, ist beim Schreiben des Satzes nicht bekannt, daher wird `at any depth` verwendet.
- Für die Mitglieder von Anwendungen lautet der Wortlaut `at any depth`, nicht `at any level`: ein level ist eine Ebene der Schaltung, wie in `at each level in # and below`.
- Eine Regel in einem Paket zählt als in der Ebene ihres USE-PACKAGE geschrieben (10.4); mit `recursive=true` schreibt ein Paket `at every level that uses this package, #` anstelle von `at each level in # and below`.

| Schlüsselwort | Fall | Standardsatz |
|---|---|---|
| SELECT-INSTANCE | diese Ebene und darunter | `among the instances at each level in # and below, @SELECT-INSTANCE(model="nch", recursive=true) => they @SET-SYMBOL to $NMOS` |
| | nur diese Ebene | `among the instances in # only, @SELECT-INSTANCE(type="pmos") => …` |
| | ein Filter | `among {#M1 #M2 #R1}, @SELECT-INSTANCE(name="M*") => they @ORIENT("MY")` |
| | auf einem Netz | `among the instances connected to &en, @SELECT-INSTANCE => they @SET-STYLE to $grey` |
| | Besitzer eines Anschlusses | `among the instances owning #M1:g, @SELECT-INSTANCE => they are on @LAYER("top")` |
| | Mitglieder einer Anwendung | `among all members of %st1 at any depth, @SELECT-INSTANCE(role="pair")` |
| | Mitglieder einer Anwendung ohne geschachtelte Anwendungen | `among the members of %dp, @SELECT-INSTANCE` |
| | in der Mitte einer Pipe | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => among all members of them at any depth, @SELECT-INSTANCE => they @SET-STYLE to $signal` |
| SELECT-TERMINAL | diese Ebene und darunter | `among the terminals at each level in # and below, @SELECT-TERMINAL(net="vss", recursive=true) => they have @ASSERT-TERMINAL(same=["net"])` |
| | einer Instanz | `among the terminals of #R1, @SELECT-TERMINAL => on them there is a @TEXT("${net}")` |
| | auf einem Netz | `among the terminals connected to $GROUND_NET_SET, @SELECT-TERMINAL => among the pins of them, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| SELECT-PIN | in der Ebene | `among the pins in # only, @SELECT-PIN(wired=false) => on them there is a @TEXT("${net}", key="netname")` |
| | von Anschlüssen | `among the pins of {#M1:s #M2:s}, @SELECT-PIN(wired=false) => $GROUND_STUB is @ATTACH to them` |
| | eines Anschlusses | `among the pins of #X1:vdd, @SELECT-PIN => they @SET-STYLE to $red` |
| SELECT-NET | diese Ebene und darunter | `among the nets at each level in # and below, @SELECT-NET(port=true, recursive=true) => $IO_PORT is @ATTACH to them` |
| | einer Instanz | `among the nets connected to #M1, @SELECT-NET => named %m1_net_set` |
| | eines Anschlusses | `among the nets connected to #M1:g, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | einer Leitung | `among the nets of %w13, @SELECT-NET => they are drawn with @ROUTE-NET` |
| | ein Filter | `among {&a &b}, @SELECT-NET(wired=false) => they are drawn with @ROUTE-NET` |
| SELECT-WIRE | diese Ebene und darunter | `among the wires at each level in # and below, @SELECT-WIRE(bus=true, recursive=true) => they @SET-STYLE to $bus` |
| | auf einem Netz | `among the wires on &clk, @SELECT-WIRE => they @SET-STYLE to $red` |
| SELECT-LABEL | diese Ebene und darunter | `among the labels at each level in # and below, @SELECT-LABEL(key="wl", recursive=true) => they @SET-STYLE to $grey` |
| | an ein Objekt angehängt | `among the labels attached to #M1, @SELECT-LABEL => they @SET-STYLE to $grey` |
| SELECT-ANNOTATION | diese Ebene und darunter | `among the annotations at each level in # and below, @SELECT-ANNOTATION(recursive=true) => they @SET-STYLE to $base` |
| | ein Filter | `among {%n1 %f1 #M1}, @SELECT-ANNOTATION` |
| SELECT-USAGE | diese Ebene und darunter | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) of $DIFFERENTIAL_PAIR_N => they have @PRIORITY("disabled")` |
| | ohne Definition | `among the usages at each level in # and below, @SELECT-USAGE(recursive=true) => they @SET-STYLE to $base` |
| | eine Anwendung und die Anwendungen darin | `among %st1 itself and the usages inside it, @SELECT-USAGE of $DIFFERENTIAL_PAIR_N` |
| SELECT-ANCESTOR-USAGE | | `among the usages containing #M1, @SELECT-ANCESTOR-USAGE of $DIFFERENTIAL_PAIR_N => named %dp_set` |
| beliebige Auswahl | Art des Bereichs nicht bekannt | `for %mixed_set, @SELECT-TERMINAL => …` |

### B.8 Mengen und weitere Schlüsselwörter

| Schlüsselwort | Fall | Standardsatz |
|---|---|---|
| ADD-TO | | `&vdd @ADD-TO $SUPPLY_NET_SET` |
| | in einer Pipe | `… => they @ADD-TO $SUPPLY_NET_SET` |
| REMOVE-FROM | | `&agnd @REMOVE-FROM $GROUND_NET_SET` |
| UNION | | `%u_set = %a_set and %b_set @UNION` |
| BUNDLE | | `#XI<3:0> is drawn as a @BUNDLE => named %xi` |
| CURRENT | in | `#M5:1 has @CURRENT("in")` |
| | out | `#:iout has @CURRENT("out")` |
| | unspecified | `#M5:2 has @CURRENT("unspecified")` |
| ASSERT-INSTANCE | | `{#M1 #M2} have @ASSERT-INSTANCE(same=["model", "w", "l"])` |
| ASSERT-TERMINAL | | `{#M1:s #M2:s} have @ASSERT-TERMINAL(same=["net"])` |
| ASSERT-NET | | `{&inp &inn} have @ASSERT-NET(port=true)` |
| WIRE auf keinem Netz | | `between {%sub1::y %int1::a}, on no net &, there is a @WIRE` |
| ANNOTATION | | `at (40, 20) there is an @ANNOTATION(rotate=30) of $ARROW_SYMBOL` |
| ASSERT-USAGE | | `%dp1 is, by @ASSERT-USAGE, a usage of $DIFFERENTIAL_PAIR_N` |
| ASSERT-IN | | `{&vss} is, by @ASSERT-IN, in $GROUND_NET_SET` |
| ASSERT-TOPOLOGY | Reihe | `{#R1 #R2 #R3} are connected as @ASSERT-TOPOLOGY("series")` |
| | parallel | `{#M3 #M4} are connected as @ASSERT-TOPOLOGY("parallel")` |
| | Stern | `{#L1 #C1 #L2} are connected as @ASSERT-TOPOLOGY("star")` |
| | Schleife | `{#C1 #L1 #C2} are connected as @ASSERT-TOPOLOGY("loop")` |

### B.9 Kopfeintrag und Pakete

Es werden keine Wörter hinzugefügt. Der Kopfeintrag wird in der Standardschreibweise gelesen und verwendet keine Aliase (16.5).

| Schlüsselwort | Fall | Standardsatz |
|---|---|---|
| VERSION | | `@VERSION("0.4")` |
| ALIAS | | `@ALIAS("@位于", "@PLACE")` |
| USE-ALIASES | | `@USE-ALIASES(name="alias-zh", version="0.4")` |
| DEVICE | | `@DEVICE(…)` |
| PACKAGE | | `@PACKAGE(name="acme-style", version="1.2")` |
| USE-PACKAGE | in einem Kopfeintrag; auch ein Verweis im Kopfeintrag einer Paketdatei | `@USE-PACKAGE(name="rnl-std-all", version="0.4")` |
| | in einer Ebene | `@USE-PACKAGE(name="acme-style", version="1.2")` |
