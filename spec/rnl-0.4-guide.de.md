# RNL 0.4 – Kurzleitfaden

RNL wird in den Kommentaren einer SPICE- oder Spectre-Netzliste geschrieben und legt fest, wie der Schaltplan gezeichnet wird und was die Schaltungsteile bedeuten; die Schaltung selbst bleibt dabei stets unverändert. Sprache `"0.4"`, Standardpaketserie `"0.4"`; Zahlen wie (14.8) verweisen auf Abschnitte des Standards. Die Beispiele sind eigenständige Einzelbeispiele und bilden kein zusammenhängendes Programm: Verwenden Sie jeweils vorhandene Objekte der betreffenden Ebene und setzen Sie die Ausschnitte in RNL-Einträge. `/* */` erläutert sie. Der Katalog am Ende führt jeden Namen als Signatur auf – nicht als RNL zum Kopieren.

## Eine Datei

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

- Einträge sind Ganzzeilenkommentare mit `*` in Spalte 1 gefolgt von einem Leerzeichen; ebenso `//` im Spectre-Modus (in einer `.scs`-Datei oder nach `simulator lang=spectre`). Der Kopfeintrag steht zuerst. Ein Eintrag innerhalb von `.subckt … .ends` gehört zu dieser Teilschaltung.
- Ohne das USE-PACKAGE ist kein Standardname bekannt. Ein vorgegebener Kopfeintrag bleibt unverändert; seine `@DEVICE`-Zeilen geben PDK-Bauteilen ihre Anschlussnamen und -typen (6.12), und Sie schreiben oder ändern sie nicht.
- Schreiben Sie vor jedes `@` ein ASCII-Leerzeichen; Parameterklammern berühren das Schlüsselwort unmittelbar, Rollenklammern unmittelbar das Objekt. Positionsparameter stehen vor benannten Parametern: `@PLACE(10, y=20)`. Zahlen haben keine Einheiten; Ausdrücke gibt es nicht. Bei RNL-Namen und -Schlüsselwörtern wird zwischen Groß- und Kleinschreibung unterschieden; Netzlistennamen folgen den Regeln des Hosts.

## Verweise

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

- Anschlussnamen: M `d g s b`; Q `c b e s`; J Z `d g s`; D `a k`; R C L V I B F H W `p n`; E G S `p n cp cn`; T O `p1 n1 p2 n2`; U `a b ref`; X die Ports der Teilschaltung. Wo ein Pin benötigt wird, steht ein Anschluss für seinen Pin.
- Namen werden pro Ebene einmal gebunden und dürfen bereits vor ihrer Definition verwendet werden. Um `%p` zu verschieben, schreiben Sie `%p @PLACE(...)`, nicht ein weiteres `%p = ...`. `%`-Namen sind lokal; `$`-Definitionen werden an tiefere Ebenen vererbt, wo eine andere Definition sie verdecken kann.
- Ein Wort ohne Präfix ist Freitext (eine Warnung, wenn es etwas benennt): `#M1 #M2 M3 @ALIGN("y");` richtet nur M1 und M2 aneinander aus.

## Anweisungen und Pipes

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

Was ein Segment weitergibt: PLACE, ORIENT, SET-SYMBOL, SET-STYLE, GROUP und CURRENT ihre Eingabe; AS und ATTACH die neue Anwendung (bei schrittweiser Anwendung eine Menge davon); ALIGN, ORDER, DISTRIBUTE, DISTANCE, TABLE und ORIENT-RELATION die neue Bedingung; TEXT den neuen Text; SELECT-… die ausgewählte Menge.

## Definitionen, Rollen, Anwendungen

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

- Eine `{role}` übernimmt die gesamte Menge (oder ein einzelnes Objekt als einelementige Menge) und darf weggelassen werden; ohne Rollenmarkierung dürfen nur am Ende stehende Mengenrollen ausgelassen werden. Eine Einzelrolle ist erforderlich. Markieren Sie jede Rolle höchstens einmal und fassen Sie mehrere Mitglieder in einer Menge zusammen: `{#R1 #R2}(members)`.
- In einem Rumpf steht ein alleinstehendes `?` für den eigenen Punkt der Anwendung, Koordinaten sind Versätze relativ zu ihm, und `%mid = (_, _);` erzeugt ein lokales Objekt, von außen als `%u::mid` ansprechbar.
- Die platzierenden Standardstrukturen binden den eigenen Punkt ihrer Anwendung: Eine Zeile oder Spalte von Mitgliedern hat ihn auf ihrer Linie, zwei gespiegelte Seiten im Mittelpunkt dazwischen, sodass eine Bedingung auf `%u::position` die Struktur bewegt. Die Spalte *Zeichnet* des Katalogs gibt an, wo: `$BRANCH` teilt mit ihm lediglich das x der Mitglieder, `$BRANCH_HORIZONTAL` lediglich deren y, `$PARALLEL_HORIZONTAL` und `$MIRRORED_LAYOUT` halten ihn in ihrer Mitte. Jede andere Struktur lässt ihn frei, und eine Bedingung auf ihr `::position` ändert nichts Gezeichnetes.
- Bevorzugen Sie Strukturen, die zugleich angeben, was die Teile sind, und sie zeichnen. Ein `$BRANCH` ist eine senkrecht gestapelte Serienstruktur mit Mitgliedern von oben nach unten; üblicherweise steht er für einen DC-Zweig, etwa einen Pfad von einer Versorgung zur Masse. Schreiben Sie jeden solchen Pfad als einen `$BRANCH` und ein paralleles Paar darauf mit einer platzierenden Variante: `{#R1 #M1 #R2} @AS $BRANCH; {#R2 #C2} @AS $PARALLEL_HORIZONTAL;`. `$PARALLEL` allein führt nur Leitungen. Ein Zweig macht keine Aussage über den Strom – diesen geben Sie mit `@CURRENT` an. Ein Signalpfad über Schalter und Kondensatoren, wie in einem Switched-Capacitor-Abtaster, wird mit atomaren Fakten gezeichnet, etwa mit einem ALIGN und einem ORDER für die Pins jeder Zeile: `{#S1:s #S1:d #C1:p #C1:n} @ALIGN("y"); {#S1:s #S1:d #C1:p #C1:n} @ORDER("left-to-right");`. Eine Struktur mit *nur Bedeutung* zeichnet nichts: Schreiben Sie sie dort, wo sie zutrifft, niemals anstelle einer Struktur, die platziert. Setzen Sie ein `$SIGNAL_FLOW` nur auf einen sauberen Signalpfad, dessen Sie sicher sind; lassen Sie es weg, wo sich das Signal auf mehrere Pfade aufteilt, rückkoppelt oder über weite Teile der Schaltung in beide Richtungen läuft, unabhängig davon, wie wenige oder viele Mitglieder der Pfad hat.

## Auswahlen und Regeln

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

- `name`, `model`, `net` und `key` akzeptieren `*`. `type` ist der Typ der `.model`-Karte oder eines DEVICE.
- Ein Auswahlergebnis sowie eine daraus gebildete Menge unterstützen keine Mitgliedspositionen (`%sel{1}`) und können nicht an WIRE übergeben werden; benennen Sie das Objekt oder schreiben Sie ROUTE-OBJECT.

## Layout-Fakten

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

- Richtungen: `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`. Codes: `R0`, `R90`, `R180`, `R270` drehen gegen den Uhrzeigersinn; `MY` spiegelt links-rechts, `MX` oben-unten; `MYR90`, `MXR90` spiegeln zuerst und drehen danach.
- Mitglieder sind Instanzen (über ihren Ursprung), Anschlüsse und Pins (über den Pin), Punkte, Texte sowie Anwendungen mit Symbol (über ihren eigenen Punkt). Eine Anwendung ohne Symbol steht wie in GROUP bis hinab zu den untersten Objekten für ihre Mitglieder; ihr eigener Punkt ist `%u::position`. Mengen haben keinen Ursprung: PLACE platziert jedes Mitglied einzeln, auch nach GROUP. PLACE platziert weder Pins noch Bedingungen. Ein „am weitesten links“ gibt es nicht: Geben Sie die Lage relativ zu anderen Schaltungsteilen an.
- Der Solver entscheidet über alles, was frei bleibt, und rät nicht, was ein Bauteil ist: Schreiben Sie alles auf, was gelten muss. Bevorzugen Sie Bedeutung und Bedingungen und überlassen Sie dem Solver die Platzierung; verlangt die Aufgabe Koordinaten, schreiben Sie stattdessen für alles PLACE, ORIENT und WIRE. Vermischen Sie beide Ansätze nicht: Entweder geben Sie Bedeutung und Bedingungen für den Solver mit wenigen oder gar keinen PLACE-Anweisungen an, oder Sie schreiben jede Position ohne Lösevorgang aus.
- Ordnen Sie die großen Einheiten an, nicht einzelne Bauteile: `{#V2 #C1 %bias %stage #C3 #R5} @ORDER("left-to-right");` stellt ganze Spalten nebeneinander. Eine Struktur steht für ihre Mitglieder, sodass jedes Bauteil von `%stage` zwischen jedem Bauteil von `%bias` und C3 liegt; ein Bauteil, das zwei Strukturen teilen, wird nicht gegen sich selbst geordnet. Um stattdessen den Mittelpunkt einer Struktur mit Bedingungen zu versehen, schreiben Sie ihren eigenen Punkt: `{%dp::position #X1} @ALIGN("y");` setzt X1 auf die Höhe des Mittelpunkts des Paares, während `{%dp #X1} @ALIGN("y");` die drei Transistoren auf die Höhe von X1 setzen würde (die Spalte *Zeichnet* des Katalogs gibt an, wo eine Struktur ihren Punkt bindet).
- Teile rund um die Schaltung, wie die Versorgungen und die Signalquellen, stehen meist mit etwas Abstand auf einer Seite davon; ein DISTANCE drückt das aus: `#VDD {#VIN #XU1 #CL} @DISTANCE("left-to-right", min=4);` hält die Versorgung mindestens 4 Einheiten links von jedem der übrigen Teile.
- Geben Sie der Zeichnung einen klaren Baum von Gruppen: jeder Funktionsblock eine Gruppe, Gruppen in größeren verschachtelt und etwa fünf bis sechs Elemente direkt in jedem Knoten. Viele Strukturen sind bereits Gruppen (Spalte *Gruppe*); eine Struktur, die wie ein Stromspiegel teilweise innerhalb einer anderen liegen kann, ist keine. Fassen Sie ganze Strukturen oder eine Stufe samt ihren Ports selbst zusammen: `%amp %p_inp %p_inn %p_out @GROUP;`. Gruppen überkreuzen sich nicht: Teilen zwei Gruppen Mitglieder ohne Verschachtelung, behält der Solver nur eine als Block.
- Legen Sie Ausrichtungen nur dort fest, wo der Leser eine bestimmte erwartet, die nichts anderes vorgibt – meist bei der Signalquelle, den Versorgungsquellen und der Last: `{#V1 #V2 #R5} @ORIENT("R0");`. Überlassen Sie den Rest dem Solver; die Strukturen und der Strom legen die meisten Ausrichtungen bereits fest.

## Pins und Richtungen

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

- Ein einzelner festgelegter Pin lässt zwei von acht Ausrichtungen offen: Ist nur das Gate festgelegt, bleibt offen, ob der Transistor aufrecht oder auf dem Kopf steht; ist nur der Drain festgelegt, zeigt das Gate nach links oder rechts. Legen Sie zwei Pins fest.
- Standard-Pins bei R0: R, C, L und Quellen p oben, n unten; `$DIODE` a oben, k unten; `$NMOS` d oben, g links, s unten, b rechts (4TERM); `$PMOS` s oben, g links, d unten; `$NPN` c oben, b links, e unten; `$PNP` e oben, b links, c unten.

## Strom

```rnl
#M1:d @CURRENT("in"); #M1:s @CURRENT("out"); /* the static current at a terminal: in, out, unspecified */
{#M1:d #M2:d #M5:d} @CURRENT("in"); /* a set does each member */
#:out @CURRENT("in"); /* at a port: current enters the subcircuit */
#M1::d @CURRENT("in"); /* a pin stands for its terminal */
```

- Es zeichnet nichts; es beschreibt, was die Schaltung tut, und der Solver legt die Strompfade danach aus. Geben Sie es auf den Hauptpfaden an.
- Schreiben Sie es nur dort, wo ein statischer Strom fließt und seine Richtung feststeht: Kondensatoren, Gates und Quellen ohne DC-Wert führen keinen Strom – lassen Sie sie also weg, und lassen Sie ebenso jeden Anschluss weg, bei dem Sie unsicher sind. Lassen Sie auch Versorgungsquellen unmarkiert: In ihnen fließt der Strom von n nach p nach oben, sodass ein Solver, der abwärts fließenden Strom bevorzugt, sie auf den Kopf stellen würde.
- Ein Netz, dessen Anschlüsse und Ports sämtlich den Strom in dieselbe Richtung bezogen auf das Netz führen, liefert einen Verstoß; globale Netze wie 0 werden nicht geprüft.

## Priorität

```rnl
#M5 @PLACE(x=3, priority="low"); /* disabled, low, normal (default), high */
%dp1 = #M1(left) #M2(right) #M5(tail) @AS(priority="high") $DIFFERENTIAL_PAIR_N; /* covers every fact of the body */
%dp1 @PRIORITY("disabled"); /* turns the default drawing of the pair off */
```

- Bei einem Konflikt wird der Fakt mit niedrigerer Priorität zuerst verworfen, bei Gleichstand der früher geschriebene; ein behaltener Fakt ist hart. Das Standardpaket zeichnet mit `"low"`. Das `priority=` eines Eintragskopfs ist ein Standardwert, AS und ATTACH übernehmen daraus jedoch ausschließlich `"disabled"`.
- Eine Struktur benötigt keine Priorität: Geben Sie nur dann eine an, wenn ein Konflikt aufgelöst werden soll, den eine Prüfung oder ein Lösungslauf gezeigt hat.

## Leitungen und Leitungsführung

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

- Gleiche Koordinaten verbinden Pins nicht. Schreiben Sie auch bei Länge null ein WIRE zwischen ihnen, um sie als verdrahtet zu markieren und zusätzliche Ergänzung zu vermeiden.
- Die Strukturen zeichnen die Leitungen in ihrem Inneren mit Priorität low. Was zwei Strukturen verbindet, ist eine Netzbeschriftung, sofern Sie kein ROUTE schreiben.
- Verdrahten Sie das, was als Leitung besser lesbar ist, nicht alles: Der Solver zeichnet nur, was WIRE, ROUTE-NET und ROUTE-OBJECT verlangen, und die Ergänzung versieht jeden anderen unverdrahteten Pin mit einem Masse-, Versorgungs- oder Netzbeschriftungs-Stub. Führen Sie den Hauptsignalpfad sowie die Verbindungen innerhalb und zwischen benachbarten Strukturen als Leitungen aus; überlassen Sie lange oder gedrängte Verbindungen wie Bias-Leitungen und über die Zeichnung verteilte Netze den Beschriftungen.
- In einer kleinen Schaltung ist bis auf die Versorgungsschienen meist jedes Netz als Leitung gut lesbar: Führen Sie sie wie in der letzten Zeile oben als Leitungen aus, oder mit `# @SELECT-NET => @ROUTE-NET;`, was dasselbe zeichnet. In beiden Fällen nimmt die Ergänzung Masse-, Versorgungs- und Bias-Netze mit Priorität high von der Leitungsführung aus, und jedes Schienennetz erhält einen Hinweis (RNL9403), dass sein Baum alle Pins verloren hat; dieser Hinweis ist zu erwarten.
- Ein Netz, das zu einem Port seiner Ebene geführt wird, erreicht den Port nur, wenn die Port-Markierung geschrieben ist: `$IO_PORT @ATTACH &x;` oder `$IO_PORT_STUB` an dem Pin, den es treibt. Die Ergänzung fügt ihre Port-Markierungen erst hinzu, nachdem sie entschieden hat, welche Pins verdrahtet sind; ein Netz mit einem einzigen eigenen Pin, etwa ein Eingang, der ein Gate treibt, bildet daher einen Baum aus einem Punkt: Dieser Pin erhält eine Netzbeschriftung, und keine Leitung erreicht den Port.

## Texte

```rnl
#M1 @TEXT("${w}/${l}", key="wl", dy=-3); /* a label that moves with M1; #M1::wl */
#M1 @TEXT("M_{in}", key="name"); /* the same key replaces the name the symbol shows */
(10, 10) @TEXT("bias", size=0.8); /* on a point: an annotation */
#M1 @TEXT("\overline{EN} \mu A \frac{W}{L}", halign="middle", valign="top", rotate=90);
```

- `${…}`: `name`, `value`, `model`, `type`, Netzlistenparameter wie `w`; `net`; Rollen; Pfade wie `${pair::left::w}`.
- `dx` und `dy` sind Versätze relativ zu dem Objekt, an dem der Text hängt. `#M1::wl @PLACE(5, 0);` setzt die Beschriftung auf (5, 0) der Zeichnung; auf jeder Achse ersetzen ein PLACE und ein Versatz einander, wobei die spätere Angabe gewinnt. In einem Symbolrumpf beziehen sich beide auf das eigene Koordinatensystem des Symbols.

## Symbole und Stile

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

- Stileigenschaften: `linecolor linestyle linewidth fillcolor textcolor size font bold italic textvisible`; Farben `"#RRGGBB"`. Ein `%`-Name kann auch eine lokale Definition benennen. Für jede Eigenschaft schlagen direkte Einstellungen die Regeln; innerhalb derselben Stufe gewinnt die spätere Einstellung.

## Prüfungen

Die Standardstrukturen prüfen, was sie behaupten: Paarhälften mit gleichem Modell und gleicher Größe, eine Serienschaltung tatsächlich in Reihe, eine pmos-Rolle mit einem PMOS-Transistor, eine Port-Markierung an einem Port. Eine fehlgeschlagene Prüfung ist ein **Verstoß**: eine reine Meldung; nichts schlägt fehl, und die Zeichnung bleibt unverändert. Korrigieren Sie die Struktur oder die Rollen, oder wählen Sie eine Struktur, die weniger behauptet.

## Das Standardpaket

```rnl
@USE-PACKAGE(name="rnl-std-all", version="0.4"); /* everything: styles, symbols, rules for SPICE and Spectre, fill, structures, the blocks */
@USE-PACKAGE(name="rnl-std-no-labels", version="0.4"); /* optional, besides rnl-std-all: hides the labels symbols draw (name, value, model, net names); a label you write shows only with textvisible=true */
@USE-PACKAGE(name="rnl-std-45deg", version="0.4"); /* optional: symbols at 45 degrees */
@USE-PACKAGE(name="rnl-std-fill") {}; /* an empty package drops that part */
&vdd @ADD-TO $SUPPLY_NET_SET; &vss @ADD-TO $GROUND_NET_SET; /* supply and ground stubs on their pins; 0 is a ground already */
$PORT_LIST @ATTACH; /* a port tag on every port net, in one column */
$FIGURE @ATTACH %p => @SET-SYMBOL $MY_FIG; /* no netlist instance: use a symbol with unbound pins and no ${name}/${value}; not $RESISTOR */
```

- Standardsymbole: R `$RESISTOR`, C `$CAPACITOR`, L `$INDUCTOR`, V `$VOLTAGE_SOURCE`, I `$CURRENT_SOURCE`, D `$DIODE`, E `$VCVS_4TERM`, G `$VCCS_4TERM`, F `$CONTROLLED_CURRENT_SOURCE`, H `$CONTROLLED_VOLTAGE_SOURCE`, B `$BEHAVIORAL_SOURCE`, S `$VOLTAGE_CONTROLLED_SWITCH`, W `$SWITCH`, T O `$TRANSMISSION_LINE`, U `$RC_LINE`; Transistoren nach Typ: nmos `$NMOS_4TERM`, pmos `$PMOS_4TERM`, npn `$NPN`, pnp `$PNP`, njf `$NJFET`, pjf `$PJFET`; ein NMOS mit seinem Bulk auf einem Massenetz und ein PMOS mit seinem Bulk auf einem Versorgungsnetz erhalten stattdessen die dreipoligen Symbole `$NMOS` bzw. `$PMOS`, ohne eigenes SET-SYMBOL von Ihnen. Ohne Typ (kein `.model`, kein DEVICE) erhält ein Transistor kein Bauteilsymbol und wird als Kasten mit einem Pin je Knoten gezeichnet (9.2.8).
- Tragen Sie jedes Versorgungsnetz in `$SUPPLY_NET_SET` und jede andere Masse als 0 in `$GROUND_NET_SET` ein, wie oben: Ihre unverdrahteten Pins erhalten Stubs, und sie bleiben ohne Leitungsführung. Die Ergänzung überspringt Pins mit Leitungen oder Nicht-verbunden-Markierungen; jeder andere unverdrahtete Pin, eines Bauteils oder eines virtuellen Bauteils, erhält einen `$GROUND_STUB` auf einem Netz aus `$GROUND_NET_SET` (standardmäßig Netz 0), einen `$SUPPLY_STUB` auf einem Netz aus `$SUPPLY_NET_SET` (standardmäßig leer), einen `$BIAS_STUB` auf einem Netz aus `$BIAS_NET_SET` (standardmäßig leer) und sonst einen `$NET_LABEL_STUB`; der Bulk-Pin eines MOS mit vier Anschlüssen erhält auf jedem Netz einen `$NET_LABEL_STUB`. Jedes Portnetz erhält einen `$IO_PORT`, auch Versorgungs- und Massenetze, in einer Spalte mindestens 6 Einheiten links von den Instanzen der Ebene, jeder bei R0; nehmen Sie ein Netz mit `&vdd @REMOVE-FROM $PORT_FILL_SET;` heraus oder schreiben Sie `$IO_PORT @ATTACH &out;`, um eine Markierung selbst zu platzieren. Eine Markierung zeigt den Namen ihres Netzes selbst als ihren Text `netname`: Schreiben Sie kein TEXT für den Namen darauf, und verschieben Sie den Namen mit `%t::netname @PLACE(x, y);`. Verwenden Sie für einen Testbench-Namen `$NET_LABEL`. Ein Pin, der bereits einen `$IO_PORT_STUB` oder einen `$NET_LABEL_STUB` besitzt, erhält ebenfalls keine Netzbeschriftung, obwohl deren Zuleitungen vom Symbol und nicht durch ein WIRE gezeichnet werden. Die `*_FILL_SET`-Mengen sind für die eigenen Stubs und Markierungen der Ergänzung bestimmt: Ein Lösevorgang schreibt diese zurück, und die Mengen enthalten ihre Pins und Netze dann nicht mehr; bauen Sie also keine weitere Regel darauf auf, etwa ein TEXT auf ihren Pins, sondern wählen Sie die Pins selbst aus. Um zu ändern, welche Pins einen Stub erhalten, fügen Sie Pins zu diesen Mengen hinzu oder entfernen Sie sie; ein Stub, den eine eigene Regel von Ihnen anhängt, wird von der Ergänzung nicht gesehen, die dann zusätzlich ihren eigenen hinzufügt.

## Einträge des Solvers

```spice fragment
* @RNL(type="generated", priority="high", author="rnl-solver") { #R1 @PLACE(0, 4); #R2 @PLACE(0, -4);} /* never edit; replaced on each solve */
* @RNL(author="editor") { #R2 @PLACE(y=-6, priority="high");} /* to fix a position: high, in a later record */
```

`%#3`-Nummern erscheinen ausschließlich dort; benennen Sie Objekte stattdessen selbst.

## Weitere Schlüsselwörter

Von Paketen und Werkzeugen geschrieben; Sie lesen sie, schreiben sie aber selten selbst:

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

## Häufige Fehler

Die linke Spalte ist ungültig oder irreführend; übernehmen Sie nur die Korrektur rechts mit den Namen aus Ihrer Datei.

|Fehler|Korrektur|
|---|---|
|`#M1 @PLACE (10, 20);`|`#M1 @PLACE(10, 20);`|
|`#M1 (left) #M2 (right) @AS $CROSS_COUPLED_PAIR;`|`#M1(left) #M2(right) @AS $CROSS_COUPLED_PAIR;`|
|`#M1 #M2 @SET-SYMBOL $NMOS;`|`{#M1 #M2} @SET-SYMBOL $NMOS;`|
|`#M5 #M3 @AS $MATCHED;`|`{#M5 #M3} @AS $MATCHED;`|
|`{#R1 #R2} @ORDER("x");`|`{#R1 #R2} @ORDER("left-to-right");`|
|`#R1 @PLACE(x=2u);`|`#R1 @PLACE(x=2);`|
|0.3-Namen `$DIFFERENTIAL_PAIR`, Rollen `in_p/in_n`|`#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;`|
|`#R1 @SET-SYMBOL $GROUND;`|`$GROUND @ATTACH &n;` für eine Masse; das Widerstandssymbol beibehalten|
|`%t #M1 @ALIGN("y");` zum Ausrichten ihrer Pins|`{%t::pin #M1::g} @ALIGN("y");`|
|`$IO_PORT @ATTACH &n;` an einem Nicht-Port-Netz|`$NET_LABEL @ATTACH &n;`|
|`&vss @ROUTE-NET;` wenn vss in der Masse-Menge liegt|`&vss @ROUTE-NET(priority="high");`|
|`%#3 @PLACE(1, 2);` in selbst verfasstem Text|Benennen Sie dieses Objekt und verwenden Sie dann seinen Namen|
|`#M1.d; M3; &vdd!;`|`#M1:d; #M3; &"vdd!";`|
|`{@VERSION("0.4");}`|`{ @VERSION("0.4");}`|

Eine Leitung zum Bulk von `$NMOS` schlägt fehl: Verwenden Sie `$NMOS_4TERM`. Unbekannte Transistortypen benötigen eine Modellkarte, ein DEVICE oder ein explizites Symbol. Regeln auf oberster Ebene benötigen `recursive=true`, um Teilschaltungen zu erreichen.

## Ein vollständiges Beispiel

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

Prüfen Sie vor der Abgabe einer Datei: Kopfeintrag zuerst, mit VERSION und dem Paket; alle Namen existieren in ihrer Ebene; nur Namen aus 0.4 (siehe Katalog); ein Leerzeichen vor jedem `@`, ein `;` nach jeder Anweisung; Richtungen sind ausgeschrieben; Mengenrollen erhalten Mengen; Massen und Versorgungen stehen in ihren Mengen; Ausrichtungen nur dort, wo nötig, und Stromrichtungen nur dort, wo sicher; ausgerichtete Pins sind verdrahtet; keine Änderungen in generierten Einträgen; kein unbeabsichtigter Verstoß.

<!-- lists:begin -->
## Katalog

Eine Referenz, kein RNL zum Kopieren: Behalten Sie das `$` am Namen bei. Wenden Sie Definitionen mit AS oder ATTACH an, weisen Sie Symbole mit SET-SYMBOL und Stile mit SET-STYLE zu. Rollen stehen in Deklarationsreihenfolge, jeweils mit den akzeptierten Arten (eine Rolle ohne Angabe nimmt Objekte jeder Art); `{role}` übernimmt eine ganze Menge und darf weggelassen werden; Parameter folgen nach `;`. So wird `$MATCHED({members}: instance/usage)` als `{#M1 #M2} @AS $MATCHED;` verwendet.

### Pakete

Jedes wird im Kopfeintrag mit `@USE-PACKAGE(name="...", version="0.4");` benannt.

|Paket|Inhalt|
|---|---|
|rnl-std-all|Die einzige Paketsammlung der Serie 0.4, ohne eigenen Inhalt: Sie bringt Stile, Basis- und Halbleitersymbole sowie -definitionen, die automatische Ergänzung, die Schaltungssemantik, die vier Fachgebietspakete sowie das Regelpaket jedes Host-Dialekts über einen Kopfeintrag für diese Sprache ein (2.8, 16.3).|
|rnl-std-style|Die Stile (9.3) und die Standard-Stilregeln.|
|rnl-std-basic|Die Symbole passiver Bauteile, Quellen, Schalter, Übertragungsleitungen, virtueller Bauteile und Markierungen; die Definitionen virtueller Bauteile, Markierungen, Figuren und Stubs, jeweils samt der Regel, die ihre Anwendungen zeichnet; sowie die Mengen der Masse-, Versorgungs- und Bias-Netze.|
|rnl-std-semiconductors|Die Symbole von Dioden sowie MOS-, Bipolar-, JFET- und Unijunction-Transistoren (UJT, PUT).|
|rnl-std-spice|Das Regelpaket von SPICE: weist Instanzen Symbole nach Elementbuchstabe, Modelltyp sowie Bulk- oder negativem Steuernetz zu (16.3).|
|rnl-std-spectre|Das Regelpaket von Spectre: weist Instanzen Symbole nach Master, Modelltyp sowie Bulk- oder negativem Steuernetz zu (16.3).|
|rnl-std-fill|Die Mengen und Regeln der automatischen Ergänzung: bringt Masse-, Versorgungs-, Bias- und Netzbeschriftungs-Stubs an unverdrahteten Pins an, einschließlich der Pins virtueller Bauteile, sowie eine Port-Markierung an jedem Portnetz in `$PORT_FILL_SET` (10.5, 12.4, 16.3).|
|rnl-std-semantics|Die semantischen Definitionen von Schaltungsstrukturen und deren Standardzeichnungen.|
|rnl-std-functional|Verhaltensmodelle: Operationsverstärker, Komparatoren, Verstärker, Transkonduktanzverstärker, ADC, DAC und Chopper-Modulator, mit *_SUPPLY-Varianten, deren vp- (oben) und vn-Pins (unten) auf dem ganzzahligen Raster liegen.|
|rnl-std-rf|HF-Bauteile: die Induktivität mit Mittelanzapfung und die T-Spule.|
|rnl-std-logic|Gatter, Puffer, Inverter, Verzögerungszelle und D-Flipflops.|
|rnl-std-signal-flow|Ideale virtuelle Signalfluss-Bauteile und ihre Symbole: Addierer, Subtrahierer, Multiplizierer, zeitkontinuierlicher und zeitdiskreter Integrator, Quantisierer und Einheitsverzögerung.|
|rnl-std-no-labels|Blendet die Standardbeschriftungen der Standardsymbole aus, für eine Datei, die eigene schreibt (16.3).|
|rnl-std-no-instance-labels|Blendet nur die Standardbeschriftungen von Instanzen aus; die Netznamen, die Massen, Versorgungen, Bias-Markierungen, Port-Markierungen und Netzbeschriftungen zeichnen, bleiben erhalten (16.3).|
|rnl-std-45deg|45-Grad-Symbole für Brücken, kreuzgekoppelte Paare, diagonale Wicklungen und 45-Grad-Leitungen.|

### Definitionen

Zeichnet, für die Strukturen aus rnl-std-semantics: *nur Bedeutung* platziert nichts und führt keine Leitungen; *platziert* enthält Positionsfakten für ihre Mitglieder; *führt Leitungen* verbindet die Netze, die ihre Mitglieder teilen. Ihre Positionsfakten und Leitungsführungen haben die Priorität low. Gruppe, aus dem Rumpf jeder Definition gelesen: *ja* mit den Rollen, deren Mitglieder sie zu einer Gruppe zusammenfasst, *über* `$X` mit den Rollen, die sie an eine die Gruppe bildende Definition `$X` weitergibt, und leer, wenn sie keine bildet; eine Struktur, die wie ein Stromspiegel teilweise innerhalb einer anderen liegen kann, bildet keine (siehe den Baum von Gruppen unter *Layout-Fakten*). Eine Anwendung ohne Symbol steht in ALIGN, ORDER, DISTRIBUTE und DISTANCE für ihre Mitglieder; ihr eigener Punkt `%u::position` (14.1) folgt der Struktur nur dort, wo *eigener Punkt* ihn an ihre Mitglieder bindet: Nur eine solche Anwendung bewegt sich über ihr `::position` als ein Element oder als Mitglied einer anderen platzierenden Struktur; bei den übrigen versehen Sie deren Mitglieder oder eine benannte Menge davon mit Bedingungen. Die übrigen Pakete definieren die gezeichneten Bauteile und Markierungen.

|Signatur|Zeichnet|Gruppe|Bedeutung|
|---|---|---|---|
|**rnl-std-basic**||||
|`$GROUND(p: net)`|—||Eine Masse auf einem Netz, gezeichnet an einem eigenen Punkt: Auf dem Netz 0 zeigt sie nur die drei Balken, auf jedem anderen Netz den Namen ihres Netzes.|
|`$SUPPLY(p: net)`|—||Eine Versorgung auf einem Netz, gezeichnet an einem eigenen Punkt.|
|`$BIAS(p: net)`|—||Eine Bias-Spannungsmarkierung auf einem Netz, gezeichnet an einem eigenen Punkt.|
|`$IO_PORT(p: net)`|—||Eine Port-Markierung auf einem Netz, gezeichnet an einem eigenen Punkt; ihr Netz muss ein Port ihrer Ebene sein.|
|`$NET_LABEL(p: net/pin)`|—||Eine Netzbeschriftung: der Name eines Netzes an einer kurzen Zuleitung, gezeichnet an einem eigenen Punkt; erhält sie einen Bus-Pin, etwa den Pin eines Bündels (11.8), behält sie den Pin und bildet eine gemeinsame Beschriftung für alle seine Netze, die deren Namen anzeigt (13.6).|
|`$NO_CONNECT(t: terminal/pin)`|—||Eine Nicht-verbunden-Markierung auf einem absichtlich unbeschalteten Anschluss oder Pin; ein Anschluss bleibt als solcher erhalten, sodass eine Markierung auf einem Bus-Bit festhält, welches Bit offen ist (12.6).|
|`$JUNCTION(p: point/terminal)`|—||Ein Knotenpunkt an einem Punkt oder einem Anschluss.|
|`$CURRENT_ARROW(p: terminal/point)`|—||Ein Strompfeil an einem Anschluss oder einem Punkt.|
|`$ARROW(at: point)`|—||Eine Pfeilspitze, gezeichnet an einem Punkt in einem Symbol oder in der Szene (9.2.7).|
|`$SPDT_SWITCH(at: point)`|—||Ein einpoliger Umschalter (SPDT), gezeichnet als Figur an einem Punkt ohne Netzlistenzeile.|
|`$FIGURE(at: point)`|—||Eine an einem Punkt gezeichnete Figur ohne Netzlistenzeile, die mit SET-SYMBOL ein eigenes Symbol erhält.|
|`$PORT_LIST()`|—||Eine Port-Markierung auf jedem Portnetz einer Ebene, das noch kein `$IO_PORT` besitzt, in einer Spalte, von oben nach unten geordnet und gruppiert, mindestens 6 Einheiten links von den Instanzen der Ebene, jede bei R0 mit ihrem Pin nach rechts, alle mit Priorität low, wie rnl-std-fill sie zeichnet; die Ergänzung fügt in dieser Ebene dann keine hinzu (12.4).|
|`$COMMENT({target}; text="")`|—||Ein nicht gezeichneter Kommentar, der an beliebige Objekte angehängt wird, oder an die Ebene selbst, wenn target weggelassen wird.|
|`$GROUND_STUB(t: pin; gap=3)`|—||Eine aufrechte Masse nahe einem Pin mit eigener Leitung: auf Höhe des Pins oder darunter, senkrecht höchstens gap Einheiten entfernt (Priorität low).|
|`$SUPPLY_STUB(t: pin; gap=3)`|—||Eine aufrechte Versorgung nahe einem Pin mit eigener Leitung: auf Höhe des Pins oder darüber, senkrecht höchstens gap Einheiten entfernt (Priorität low).|
|`$BIAS_STUB(t: pin)`|—||Eine Bias-Spannungsmarkierung direkt an einem Pin, vom Pin wegzeigend wie ein Port-Stub; ihr Balken neigt sich oberhalb, unterhalb und links vom Pin als / und rechts davon als \.|
|`$IO_PORT_STUB(t: pin)`|—||Eine Port-Markierung direkt an einem Pin, deren Zuleitung als kurze Leitung des Stubs dient; Pin und Markierung bleiben wie bei `$NET_LABEL_STUB` ohne Leitungsführung.|
|`$NET_LABEL_STUB(t: pin)`|—||Eine Netzbeschriftung direkt an einem Pin, deren eine Einheit lange Zuleitung als kurze Leitung des Stubs dient; an einem Bus-Pin eine gemeinsame Beschriftung für den Bus; der Pin und die Beschriftung bleiben ohne Leitungsführung.|
|**rnl-std-semantics**||||
|`$MATCHED({members}: instance/usage)`|nur Bedeutung||Bauteile oder Unterstrukturen, die in Typ oder Definition sowie im Layout abgeglichen sind.|
|`$MATCHED_MOS({members}: instance)`|nur Bedeutung||MOSFETs, die in Typ, Modell und Kanallänge abgeglichen sind.|
|`$MATCHED_BJT({members}: instance)`|nur Bedeutung||Bipolartransistoren, die in Typ, Modell und Einheitsemitter-Geometrie abgeglichen sind.|
|`$MATCHED_RESISTOR({members}: instance)`|nur Bedeutung||Widerstände, die in Typ und Modell abgeglichen sind.|
|`$MATCHED_CAPACITOR({members}: instance)`|nur Bedeutung||Kondensatoren, die in Typ und Modell abgeglichen sind.|
|`$MIRRORED_LAYOUT(left: instance/usage, right: instance/usage, axis: point/instance/usage)`|platziert; eigener Punkt im Mittelpunkt||Zwei symmetrische Objekte auf einer Höhe, spiegelbildlich zueinander um eine Achse: reines Layout, das nichts über die Schaltung aussagt.|
|`$MIRRORED_VERTICAL_LAYOUT(top: instance/usage, bottom: instance/usage, axis: point/instance/usage)`|platziert; eigener Punkt im Mittelpunkt||Zwei symmetrische Objekte in einer Spalte, spiegelbildlich zueinander um eine waagerechte Achse: reines Layout.|
|`$BRANCH({members}: instance/usage)`|platziert, führt Leitungen; eigener Punkt in x||Eine senkrecht gestapelte Serienstruktur: Mitglieder in Serie in einer Spalte, von oben nach unten; meist ein DC-Zweig, etwa ein Pfad von einer Versorgung zur Masse.|
|`$BRANCH_HORIZONTAL({members}: instance/usage)`|platziert, führt Leitungen; eigener Punkt in y||Eine als Zeile gezeichnete Serienstruktur: Mitglieder in Serie auf einer Höhe, von links nach rechts.|
|`$CASCADE({stages}: instance/usage)`|nur Bedeutung||Eine Kaskade von Stufen entlang eines Signalpfads.|
|`$SERIES({members}: instance/usage)`|führt Leitungen||Mitglieder, die hintereinander in Serie entlang eines Strompfads geschaltet sind.|
|`$PARALLEL({members}: instance/usage)`|führt Leitungen||Mitglieder, die parallel zwischen denselben beiden Netzen geschaltet sind.|
|`$PARALLEL_HORIZONTAL({members}: instance/usage)`|platziert, führt Leitungen; eigener Punkt im Mittelpunkt||Parallele Mitglieder nebeneinander auf einer Höhe, die in einer Spalte als ein einziger Platz stehen.|
|`$PARALLEL_VERTICAL({members}: instance/usage)`|platziert, führt Leitungen; eigener Punkt im Mittelpunkt||Parallele Mitglieder übereinander bei einem x-Wert, die in einer Zeile als ein einziger Platz stehen.|
|`$SIGNAL_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|nur Bedeutung||Ein Signalpfad, der in eine Richtung fließt: von seinen Quellen über die Stufen seines Pfads in der geschriebenen Reihenfolge zu seinen Lasten.|
|`$POWER_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|nur Bedeutung||Ein Leistungspfad: von seinen Quellen, wie einer Versorgung oder einer Batterie, über die Stufen seines Pfads, die ihn wandeln oder schalten, in der geschriebenen Reihenfolge zu den Lasten, die ihn nutzen.|
|`$BIAS_CIRCUIT({members}: instance/usage)`|nur Bedeutung||Eine Bias-Schaltung: die Bauteile oder Strukturen, die den Bias anderer Schaltungsteile erzeugen, wie eine Stromreferenz.|
|`$FEEDBACK_LOOP({forward}: instance/usage, {feedback}: instance/usage)`|nur Bedeutung||Eine Rückkopplungsschleife: ein Vorwärtspfad und ein Rückkopplungsnetzwerk.|
|`$DIFFERENTIAL_PAIR_N(left: instance, right: instance, tail: instance/usage)`|platziert, führt Leitungen; eigener Punkt im Mittelpunkt|ja: left, right, tail|Ein Differenzpaar aus Transistoren vom n-Typ mit seinem Fuß darunter.|
|`$DIFFERENTIAL_PAIR_P(left: instance, right: instance, tail: instance/usage)`|platziert, führt Leitungen; eigener Punkt im Mittelpunkt|ja: left, right, tail|Ein Differenzpaar aus Transistoren vom p-Typ mit seinem Fuß darüber.|
|`$COMPLEMENTARY_DIFFERENTIAL_PAIR(pairup: usage, pairdown: usage)`|führt Leitungen|ja: pairup, pairdown|Eine Rail-to-Rail-Eingangsstufe: ein oberes und ein unteres Differenzpaar an denselben Eingängen.|
|`$PSEUDO_DIFFERENTIAL_PAIR(left: instance, right: instance)`|platziert; eigener Punkt im Mittelpunkt||Ein Pseudo-Differenzpaar: zwei abgeglichene Eingangstransistoren ohne Fuß, gespiegelt gezeichnet.|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_H(left: instance, right: instance, tailleft: instance/usage, tailright: instance/usage, bridge: instance/usage)`|platziert, führt Leitungen; eigener Punkt im Mittelpunkt|ja: left, right, tailleft, tailright, bridge|Ein Differenzpaar mit Gegenkopplung durch eine Brücke zwischen seinen beiden Füßen in H-Form.|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_N(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|platziert, führt Leitungen; eigener Punkt im Mittelpunkt|ja: left, right, degleft, degright, tail|Ein Differenzpaar aus Transistoren vom n-Typ mit Gegenkopplung durch jeweils ein Element unter jedem Eingang in T-Form, mit seinem Fuß darunter.|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_P(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|platziert, führt Leitungen; eigener Punkt im Mittelpunkt|ja: left, right, degleft, degright, tail|Ein Differenzpaar aus Transistoren vom p-Typ mit Gegenkopplung durch jeweils ein Element über jedem Eingang in umgekehrter T-Form, mit seinem Fuß darüber.|
|`$CROSS_COUPLED_PAIR(left: instance, right: instance)`|platziert, führt Leitungen; eigener Punkt im Mittelpunkt|ja: left, right|Ein kreuzgekoppeltes Paar: zwei abgeglichene Transistoren in positiver Mitkopplung kreuzgekoppelt, gespiegelt gezeichnet.|
|`$CURRENT_MIRROR_N({ref}: instance, {out}: instance)`|platziert; eigener Punkt in y||Ein Low-Side-Stromspiegel aus Transistoren vom n-Typ, deren Sources oder Emitter nach unten zeigen.|
|`$CURRENT_MIRROR_P({ref}: instance, {out}: instance)`|platziert; eigener Punkt in y||Ein High-Side-Stromspiegel aus Transistoren vom p-Typ, deren Sources oder Emitter nach oben zeigen.|
|`$CASCODE_CURRENT_MIRROR_N({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|platziert; eigener Punkt in y||Ein Low-Side-Kaskodenstromspiegel: eine Spiegelzeile unten mit einer Kaskodenzeile darüber.|
|`$CASCODE_CURRENT_MIRROR_P({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|platziert; eigener Punkt in y||Ein High-Side-Kaskodenstromspiegel: eine Spiegelzeile oben mit einer Kaskodenzeile darunter.|
|`$TRANS_LINEAR_LOOP({a}: instance/usage, {b}: instance/usage)`|platziert, führt Leitungen; eigener Punkt in x|ja: a, b|Eine Translinear-Schleife aus zwei Gruppen von Transistoren, in einer Spalte gezeichnet.|
|`$TRANS_LINEAR_LOOP_HORIZONTAL({a}: instance/usage, {b}: instance/usage)`|platziert, führt Leitungen; eigener Punkt in y|ja: a, b|Eine Translinear-Schleife aus zwei Gruppen von Transistoren, in einer Zeile gezeichnet.|
|`$DIODE_CONNECTED(device: instance)`|führt Leitungen||Ein als Diode geschalteter Transistor mit zum Drain hin verbundenem Gate bzw. zum Kollektor hin verbundener Basis.|
|`$COMMON_SOURCE(device: instance)`|nur Bedeutung||Eine Source-Schaltung mit einem FET.|
|`$COMMON_GATE(device: instance)`|nur Bedeutung||Eine Gate-Schaltung mit einem FET.|
|`$COMMON_DRAIN(device: instance)`|nur Bedeutung||Ein Source-Folger (eine Drain-Schaltung) mit einem FET.|
|`$COMMON_EMITTER(device: instance)`|nur Bedeutung||Eine Emitterschaltung mit einem Bipolartransistor.|
|`$COMMON_BASE(device: instance)`|nur Bedeutung||Eine Basis-Schaltung mit einem Bipolartransistor.|
|`$COMMON_COLLECTOR(device: instance)`|nur Bedeutung||Ein Emitter-Folger (eine Kollektor-Schaltung) mit einem Bipolartransistor.|
|`$FOLLOWER(device: instance)`|nur Bedeutung||Eine Spannungsfolger-Stufe mit einem Transistor.|
|`$CASCODE(source: instance/usage, cascode: instance)`|führt Leitungen||Eine Kaskode: ein Quellbauteil oder eine Unterstruktur in Serie mit einem Transistor in Gate-Schaltung.|
|`$FOLDED_CASCODE(source: instance/usage, cascode: instance, bias: instance/usage)`|führt Leitungen||Eine gefaltete Kaskode: ein Eingangsbauteil, gefaltet in einen Transistor in Gate-Schaltung mit seinem Bias.|
|`$COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage)`|führt Leitungen|über `$TRANSCONDUCTOR_Z_AMPLIFIER`: input, load|Ein Source-Schaltungs-Verstärker: ein FET-Eingangstransistor und seine Last.|
|`$COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage)`|führt Leitungen|über `$TRANSCONDUCTOR_Z_AMPLIFIER`: input, load|Ein Emitterschaltungs-Verstärker: ein Bipolar-Eingangstransistor und seine Last.|
|`$DEGENERATED_COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|führt Leitungen|über `$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER`: input, load, degenerator|Ein gegengekoppelter Source-Schaltungs-Verstärker: ein FET-Eingangstransistor, seine Last und seine Gegenkopplung.|
|`$DEGENERATED_COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|führt Leitungen|über `$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER`: input, load, degenerator|Ein gegengekoppelter Emitterschaltungs-Verstärker: ein Bipolar-Eingangstransistor, seine Last und seine Gegenkopplung.|
|`$MILLER_AMPLIFIER(stage: instance/usage, compensation: instance/usage)`|führt Leitungen|ja: stage, compensation|Ein Miller-kompensierter Verstärker: eine Verstärkerstufe und ihr Kompensationsnetzwerk.|
|`$PUSH_PULL_STAGE(pullup: instance/usage, pulldown: instance/usage)`|führt Leitungen|ja: pullup, pulldown|Eine Gegentakt-Ausgangsstufe: nur die Ausgangstransistoren, ein Pull-up in Serie mit einem Pull-down; ihre Arbeitspunkteinstellung und die treibende Stufe gehören nicht dazu.|
|`$TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage)`|führt Leitungen|ja: transconductor, load|Eine Verstärkerstufe: ein Transkonduktanzverstärker und seine Last.|
|`$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage, degenerator: instance/usage)`|führt Leitungen|ja: transconductor, load, degenerator|Eine gegengekoppelte Verstärkerstufe: ein Transkonduktanzverstärker, seine Last und seine Gegenkopplung.|
|`$PASS_SWITCH(device: instance/usage)`|nur Bedeutung||Ein Schalter, der ein Signal zwischen seinen beiden Seiten durchlässt oder sperrt.|
|`$RESET_SWITCH(device: instance)`|nur Bedeutung||Ein Vorlade-, Ausgleichs- oder Reset-Transistor.|
|`$GROUND_SWITCH(device: instance)`|nur Bedeutung||Ein Low-Side-Schalter, der an ein Massenetz angeschlossen ist.|
|`$POWER_SWITCH(device: instance)`|nur Bedeutung||Ein High-Side-Schalter, der an ein Versorgungsnetz angeschlossen ist.|
|`$TRANSMISSION_GATE(pmos: instance/usage, nmos: instance/usage)`|platziert, führt Leitungen; eigener Punkt im Mittelpunkt|ja: pmos, nmos|Ein CMOS-Transfergatter mit dem PMOS oberhalb des NMOS.|
|`$TRANSMISSION_GATE_HORIZONTAL(pmos: instance/usage, nmos: instance/usage)`|platziert, führt Leitungen; eigener Punkt im Mittelpunkt|ja: pmos, nmos|Ein nebeneinander gezeichnetes CMOS-Transfergatter mit dem PMOS links.|
|`$MULTI_POLE_SWITCH({throws}: instance/usage)`|führt Leitungen|ja: throws|Ein Schalter mit mehreren Schaltkontakten (throws), die sich einen Pol teilen.|
|`$BOOTSTRAPPED_SWITCH(device: instance/usage)`|nur Bedeutung||Ein Bootstrap-Abtastschalter.|
|`$SC_SAMPLER({switch}: instance/usage, {capacitor}: instance/usage)`|führt Leitungen||Ein Switched-Capacitor-Abtaster: Abtastschalter mit Abtastkondensatoren.|
|`$CHARGE_SHARING(a: instance/usage, switch: instance/usage, b: instance/usage)`|führt Leitungen||Ein Ladungsteilungsnetzwerk: zwei Kondensatoren oder Kondensatorbänke, die über einen Schalter Ladung austauschen.|
|`$CHOPPER({straight}: instance/usage, {crossed}: instance/usage)`|führt Leitungen||Ein Chopper: eine Brücke aus vier Schaltern, die die Polarität eines Signals umkehrt.|
|`$FLYING_CAPACITOR(capacitor: instance/usage)`|nur Bedeutung||Ein zwischen Taktphasen umgeschalteter Flying-Kondensator.|
|`$CMOS_INVERTER(pmos: instance, nmos: instance)`|platziert, führt Leitungen; eigener Punkt im Mittelpunkt|über `$CMOS_GATE`: pmos, nmos|Ein CMOS-Inverter mit dem PMOS oberhalb des NMOS.|
|`$CMOS_INVERTER_HORIZONTAL(pmos: instance, nmos: instance)`|platziert, führt Leitungen; eigener Punkt im Mittelpunkt|über `$CMOS_GATE`: pmos, nmos|Ein nebeneinander gezeichneter CMOS-Inverter mit dem PMOS links.|
|`$CMOS_GATE({pullup}: instance/usage, {pulldown}: instance/usage)`|führt Leitungen|ja: pullup, pulldown|Ein statisches CMOS-Logikgatter: ein Pull-up-Netzwerk und ein Pull-down-Netzwerk, die am Ausgang in Serie zusammentreffen.|
|`$LATCH({members}: instance/usage)`|führt Leitungen|ja: members|Ein Latch: ein bistabiler Kern mit den umliegenden Bauteilen.|
|`$INVERTER_LATCH(a: instance/usage, b: instance/usage)`|führt Leitungen|über `$LATCH`: a, b|Ein Latch aus zwei kreuzgekoppelten Invertern.|
|`$SHIFT_REGISTER({stages}: instance/usage)`|nur Bedeutung||Ein Schieberegister: eine Kette aus Flipflops oder Latches, oft überlappend gezeichnet und mit selten verbundenen Takten.|
|`$DIVIDER(upper: instance/usage, lower: instance/usage)`|führt Leitungen||Ein Teiler aus zwei Zweigen in Serie, oberer und unterer.|
|`$RESISTOR_LADDER({resistors}: instance)`|platziert, führt Leitungen; eigener Punkt in x|ja: resistors|Eine Widerstandsleiter in Serie in einer Spalte, von oben nach unten.|
|`$RESISTOR_LADDER_HORIZONTAL({resistors}: instance)`|platziert, führt Leitungen; eigener Punkt in y|ja: resistors|Eine als Zeile gezeichnete Widerstandsleiter in Serie.|
|`$PI_NETWORK(shunt1: instance/usage, series: instance/usage, shunt2: instance/usage)`|führt Leitungen||Ein Pi-Netzwerk: ein Querzweig, ein Längszweig und ein weiterer Querzweig.|
|`$T_NETWORK(series1: instance/usage, shunt: instance/usage, series2: instance/usage)`|führt Leitungen||Ein T-Netzwerk: ein Längszweig, ein Querzweig und ein weiterer Längszweig.|
|`$LC_TANK_PARALLEL(inductor: instance/usage, capacitor: instance/usage)`|führt Leitungen|ja: inductor, capacitor|Ein paralleler LC-Schwingkreis.|
|`$RLC_TANK_PARALLEL(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|führt Leitungen|ja: resistor, inductor, capacitor|Ein paralleler RLC-Schwingkreis.|
|`$LC_TANK_SERIES(inductor: instance/usage, capacitor: instance/usage)`|führt Leitungen|ja: inductor, capacitor|Ein serieller LC-Schwingkreis.|
|`$RLC_TANK_SERIES(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|führt Leitungen|ja: resistor, inductor, capacitor|Ein serieller RLC-Schwingkreis.|
|`$DC_FEED(device: instance/usage)`|nur Bedeutung||Eine HF-Drossel oder ein Bias-Widerstand zur Zuführung einer Gleichspannungsversorgung oder eines Bias an einen Knoten.|
|`$DECAP(device: instance)`|platziert; eigener Punkt in x||Ein Entkoppelkondensator an einem Versorgungs- oder Bias-Knoten.|
|`$AC_COUPLE(device: instance/usage)`|nur Bedeutung||Ein serieller Koppelkondensator, der das AC-Signal durchlässt und DC sperrt.|
|`$OSCILLATOR({members}: instance/usage)`|führt Leitungen|ja: members|Ein Oszillator aus seinen aktiven sowie resonanten oder Verzögerungselementen.|
|**rnl-std-signal-flow**||||
|`$ADDER(in1: net, in2: net, out: net)`|—||Ein idealer Signalfluss-Addierer (out = in1 + in2), gezeichnet mit `$ADDER_SYMBOL`.|
|`$SUBTRACTOR(in1: net, in2: net, out: net)`|—||Ein idealer Signalfluss-Subtrahierer (out = in1 - in2), gezeichnet mit `$SUBTRACTOR_SYMBOL`.|
|`$MULTIPLIER(in1: net, in2: net, out: net)`|—||Ein idealer Signalfluss-Multiplizierer (out = in1 * in2), gezeichnet mit `$MULTIPLIER_SYMBOL`.|
|`$INTEGRATOR(in: net, out: net)`|—||Ein idealer zeitkontinuierlicher Integrator, gezeichnet mit `$INTEGRATOR_SYMBOL`.|
|`$DISCRETE_TIME_INTEGRATOR(in: net, out: net)`|—||Ein idealer zeitdiskreter Integrator, gezeichnet mit `$DISCRETE_TIME_INTEGRATOR_SYMBOL`.|
|`$QUANTIZER(in: net, out: net)`|—||Ein idealer Quantisierer, gezeichnet mit `$QUANTIZER_SYMBOL`.|
|`$UNIT_DELAY(in: net, out: net)`|—||Eine ideale Einheitsverzögerung, gezeichnet mit `$UNIT_DELAY_SYMBOL`.|
|**rnl-std-45deg**||||
|`$ARROW_45(at: point)`|—||Eine 45-Grad-Pfeilspitze, gezeichnet an einem Punkt in einem Symbol oder auf einer 45-Grad-Leitung in der Szene.|
|`$CURRENT_ARROW_45(p: terminal/point)`|—||Ein 45-Grad-Strompfeil an einem Anschluss oder einem Punkt entlang eines 45-Grad-Zweigs.|

### Symbole

Pins werden nach dem Anschluss oder der Rolle aufgeführt, die sie binden; `pin` (`p`) ist der Pin namens `pin`, gebunden an die Rolle `p`; `pin` (nothing) bindet nichts.

|Symbol|Pins|Bedeutung|
|---|---|---|
|**rnl-std-basic**|||
|`$RESISTOR`|`p` `n`|Ein Widerstand.|
|`$VARIABLE_RESISTOR`|`p` `n`|Ein veränderlicher Widerstand: ein Widerstand mit einem Pfeil hindurch.|
|`$CAPACITOR`|`p` `n`|Ein Kondensator.|
|`$VARIABLE_CAPACITOR`|`p` `n`|Ein veränderlicher Kondensator: ein Kondensator mit einem Pfeil hindurch.|
|`$INDUCTOR`|`p` `n`|Eine Induktivität.|
|`$VARIABLE_INDUCTOR`|`p` `n`|Eine veränderliche Induktivität: eine Induktivität mit einem Pfeil hindurch.|
|`$TRANSFORMER`|`p1` `n1` `p2` `n2`|Ein Transformator: zwei gekoppelte Wicklungen nebeneinander, ihre Polaritätspunkte oben.|
|`$POTENTIOMETER`|`n1` `nw` `n2`|Ein dreipoliges Potentiometer mit dem Schleifer in der Mitte.|
|`$PHOTORESISTOR`|`p` `n`|Ein Fotowiderstand (lichtabhängiger Widerstand).|
|`$POLARIZED_CAPACITOR`|`p` `n`|Ein gepolter (Elektrolyt-)Kondensator mit seiner positiven Platte an p.|
|`$CRYSTAL`|`n1` `n2`|Ein Schwingquarz-Resonator.|
|`$SPEAKER`|`p` `n`|Ein Lautsprecher.|
|`$VOLTAGE_SOURCE`|`p` `n`|Eine unabhängige Spannungsquelle.|
|`$CURRENT_SOURCE`|`p` `n`|Eine unabhängige Stromquelle, ihr Pfeil von p nach n.|
|`$BATTERY`|`p` `n`|Eine Batterie.|
|`$CONTROLLED_VOLTAGE_SOURCE`|`p` `n`|Eine gesteuerte Spannungsquelle, gezeichnet als Raute nur mit ihren Ausgangs-Pins.|
|`$CONTROLLED_CURRENT_SOURCE`|`p` `n`|Eine gesteuerte Stromquelle, gezeichnet als Raute nur mit ihren Ausgangs-Pins.|
|`$VCVS_4TERM`|`p` `n` `cp` `cn`|Eine spannungsgesteuerte Spannungsquelle (E) mit ihren Steuer-Pins links.|
|`$VCCS_4TERM`|`p` `n` `cp` `cn`|Eine spannungsgesteuerte Stromquelle (G) mit ihren Steuer-Pins links.|
|`$BEHAVIORAL_SOURCE`|`p` `n`|Eine Verhaltensquelle (B): ein Kreis mit der Beschriftung B.|
|`$RF_PORT`|`p` `n`|Ein HF-Port: eine Sinusquelle in Serie mit ihrem Innenwiderstand, für den Port von Spectre.|
|`$SINE_VOLTAGE_SOURCE`|`p` `n`|Eine Sinusspannungsquelle, + an p.|
|`$PULSE_VOLTAGE_SOURCE`|`p` `n`|Eine Pulsspannungsquelle, + an p.|
|`$MICROPHONE`|`p` `n`|Ein Mikrofon, + an p.|
|`$AMMETER`|`p` `n`|Ein Amperemeter, Strommessung von p nach n.|
|`$SWITCH`|`p` `n`|Ein offen gezeichneter Schalter, für einen stromgesteuerten Schalter (W) oder den Schalter von Spectre.|
|`$CLOSED_SWITCH`|`p` `n`|Ein geschlossen gezeichneter Schalter.|
|`$EXTERNALLY_CONTROLLED_SWITCH`|`p` `n` `cp`|Ein Schalter mit einem einzelnen Steuer-Pin an seiner Seite.|
|`$VOLTAGE_CONTROLLED_SWITCH`|`p` `n` `cp` `cn`|Ein spannungsgesteuerter Schalter (S) mit seinen differentiellen Steuer-Pins darunter.|
|`$SPDT_SWITCH_SYMBOL`|`com` (nothing) `a` (nothing) `b` (nothing)|Das Symbol von `$SPDT_SWITCH`: ein einpoliger Umschalter (SPDT), dessen Pins com, a und b nichts binden und über ihren Namen verdrahtet werden (9.2.4).|
|`$TRANSMISSION_LINE`|`p1` `n1` `p2` `n2`|Eine Übertragungsleitung (T oder O), Port 1 links und Port 2 rechts.|
|`$RC_LINE`|`a` `b` `ref`|Eine gleichförmig verteilte RC-Leitung (U).|
|`$GROUND_SYMBOL`|`pin` (`p`)|Das Symbol von `$GROUND` auf dem Netz 0: eine dreibalkige Masse mit ihrem Pin oben.|
|`$NAMED_GROUND_SYMBOL`|`pin` (`p`)|Das Symbol von `$GROUND` auf jedem anderen Netz als 0: eine dreibalkige Masse mit dem Namen ihres Netzes darunter.|
|`$SUPPLY_SYMBOL`|`pin` (`p`)|Das Symbol von `$SUPPLY`: ein waagerechter Balken mit dem Namen seines Netzes, sein Pin unten.|
|`$BIAS_SYMBOL`|`pin` (`p`)|Das Symbol von `$BIAS`: ein schräger Balken mit dem Namen seines Netzes, sein Pin unten.|
|`$IO_PORT_SYMBOL`|`pin` (`p`)|Das Symbol von `$IO_PORT`: ein kleiner Kreis mit dem Namen seines Netzes, sein Pin rechts.|
|`$NET_LABEL_SYMBOL`|`pin` (`p`)|Das Symbol von `$NET_LABEL`: der Name seines Netzes – oder die Netznamen eines Bus-Pins – links an einer eine Einheit langen Zuleitung, sein Pin nach rechts zeigend wie bei `$IO_PORT_SYMBOL`.|
|`$ARROW_SYMBOL`|—|Eine gefüllte, nach rechts zeigende Pfeilspitze, das Symbol von `$ARROW`; `$CURRENT_ARROW` zeichnet eine halb so große.|
|`$NO_CONNECT_SYMBOL`|—|Das Symbol von `$NO_CONNECT`: ein Kreuz.|
|`$JUNCTION_SYMBOL`|—|Das Symbol von `$JUNCTION`: ein gefüllter Punkt.|
|**rnl-std-semiconductors**|||
|`$DIODE`|`a` `k`|Eine Diode.|
|`$ZENER_DIODE`|`a` `k`|Eine Zener-Diode.|
|`$LED`|`a` `k`|Eine Leuchtdiode (LED).|
|`$SCHOTTKY_DIODE`|`a` `k`|Eine Schottky-Diode.|
|`$SHOCKLEY_DIODE`|`a` `k`|Eine Vierschichtdiode (Shockley-Diode).|
|`$PUT`|`a` `g` `k`|Ein programmierbarer Unijunction-Transistor (PUT).|
|`$NMOS`|`d` `g` `s`|Ein dreipoliger NMOS-Transistor ohne gezeichneten Bulk.|
|`$NMOS_4TERM`|`d` `g` `s` `b`|Ein NMOS-Transistor mit zu Drain und Source ausgerichtetem Bulk-Pin und Bulk-Pfeil am Kanal.|
|`$PMOS`|`d` `g` `s`|Ein dreipoliger PMOS-Transistor ohne gezeichneten Bulk.|
|`$PMOS_4TERM`|`d` `g` `s` `b`|Ein PMOS-Transistor mit zu Drain und Source ausgerichtetem Bulk-Pin.|
|`$NMOS_LOGIC`|`d` `g` `s`|Ein dreipoliger Logik-NMOS-Transistor ohne Source-Pfeil.|
|`$PMOS_LOGIC`|`d` `g` `s`|Ein dreipoliger Logik-PMOS-Transistor ohne Source-Pfeil, mit einem Inversionskreis am Gate.|
|`$DEPLETION_NMOS`|`d` `g` `s`|Ein dreipoliger Verarmungs-NMOS-Transistor ohne gezeichneten Bulk.|
|`$DEPLETION_NMOS_4TERM`|`d` `g` `s` `b`|Ein Verarmungs-NMOS-Transistor mit zu Drain und Source ausgerichtetem Bulk-Pin.|
|`$DEPLETION_PMOS`|`d` `g` `s`|Ein dreipoliger Verarmungs-PMOS-Transistor ohne gezeichneten Bulk.|
|`$DEPLETION_PMOS_4TERM`|`d` `g` `s` `b`|Ein Verarmungs-PMOS-Transistor mit zu Drain und Source ausgerichtetem Bulk-Pin.|
|`$NDMOS`|`d` `g` `s`|Ein dreipoliger n-Kanal-DMOS-Transistor ohne gezeichneten Bulk.|
|`$NDMOS_4TERM`|`d` `g` `s` `b`|Ein n-Kanal-DMOS-Transistor mit zu Drain und Source ausgerichtetem Bulk-Pin.|
|`$PDMOS`|`d` `g` `s`|Ein dreipoliger p-Kanal-DMOS-Transistor ohne gezeichneten Bulk.|
|`$PDMOS_4TERM`|`d` `g` `s` `b`|Ein p-Kanal-DMOS-Transistor mit zu Drain und Source ausgerichtetem Bulk-Pin.|
|`$NPN`|`c` `b` `e`|Ein NPN-Bipolartransistor ohne gezeichnetes Substrat.|
|`$PNP`|`c` `b` `e`|Ein PNP-Bipolartransistor ohne gezeichnetes Substrat.|
|`$NJFET`|`d` `g` `s`|Ein n-Kanal-JFET, dessen Gate-Pfeil auf den Kanal zeigt.|
|`$PJFET`|`d` `g` `s`|Ein p-Kanal-JFET, dessen Gate-Pfeil vom Kanal wegzeigt.|
|`$UJT`|`e` `b1` `b2`|Ein Unijunction-Transistor (UJT), b2 oben und b1 unten.|
|**rnl-std-functional**|||
|`$OPAMP`|`inp` `inn` `out`|Ein Operationsverstärker mit drei Anschlüssen: inp, inn, out.|
|`$OPAMP_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Ein Operationsverstärker mit Versorgungs-Pins: inp, inn, out, vp, vn.|
|`$OPAMP_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Ein Operationsverstärker mit Versorgungs-Pins, bei dem die Eingänge vertauscht (inn oben, inp unten) und die Versorgungen beibehalten sind (vp oben, vn unten) – anders als bei einem mit MX gespiegelten `$OPAMP_SUPPLY`: inp, inn, out, vp, vn.|
|`$DIFFERENTIAL_OPAMP`|`inp` `inn` `outp` `outn`|Ein volldifferentieller Operationsverstärker: inp, inn, outp, outn.|
|`$DIFFERENTIAL_OPAMP_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|Ein volldifferentieller Operationsverstärker mit Versorgungs-Pins: inp, inn, outp, outn, vp, vn.|
|`$DIFFERENTIAL_OPAMP_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|Ein volldifferentieller Operationsverstärker mit Versorgungs-Pins, bei dem Ein- und Ausgänge vertauscht (inn und outn oben, inp und outp unten) und die Versorgungen beibehalten sind (vp oben, vn unten) – anders als bei einem mit MX gespiegelten `$DIFFERENTIAL_OPAMP_SUPPLY`: inp, inn, outp, outn, vp, vn.|
|`$DIFFERENTIAL_OPAMP_TWISTED`|`inp` `inn` `outp` `outn`|Ein volldifferentieller Operationsverstärker mit vertauschter Ausgangspolarität (outn oben, outp unten): inp, inn, outp, outn.|
|`$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|Ein volldifferentieller Operationsverstärker mit vertauschten Ausgängen und Versorgungs-Pins: inp, inn, outp, outn, vp, vn.|
|`$DIFFERENTIAL_OPAMP_TWISTED_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|Ein volldifferentieller Operationsverstärker mit Versorgungs-Pins, bei dem die Eingänge vertauscht sind (inn oben, inp unten), die Ausgänge wie bei `$DIFFERENTIAL_OPAMP_SUPPLY` liegen (outp oben, outn unten) und die Versorgungen beibehalten sind (vp oben, vn unten) – anders als bei einem mit MX gespiegelten `$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY`: inp, inn, outp, outn, vp, vn.|
|`$COMPARATOR`|`inp` `inn` `out`|Ein Komparator mit drei Anschlüssen: inp, inn, out.|
|`$COMPARATOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Ein Komparator mit Versorgungs-Pins: inp, inn, out, vp, vn.|
|`$COMPARATOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Ein Komparator mit Versorgungs-Pins, bei dem die Eingänge vertauscht (inn oben, inp unten) und die Versorgungen beibehalten sind (vp oben, vn unten) – anders als bei einem mit MX gespiegelten `$COMPARATOR_SUPPLY`; sein Stufenzeichen verläuft wie bei `$COMPARATOR_SUPPLY`: inp, inn, out, vp, vn.|
|`$VOLTAGE_AMPLIFIER`|`in` `out`|Ein Spannungsverstärker mit optionalem zentriertem Text unter dem Schlüssel label: in, out.|
|`$VOLTAGE_AMPLIFIER_SUPPLY`|`in` `out` `vp` `vn`|Ein Spannungsverstärker mit Versorgungs-Pins: in, out, vp, vn.|
|`$TRANSCONDUCTOR`|`in` `out`|Ein Transkonduktanzverstärker (eine gm-Zelle) mit zentriertem Text unter dem Schlüssel label, standardmäßig g_m: in, out.|
|`$TRANSCONDUCTOR_SUPPLY`|`in` `out` `vp` `vn`|Ein Transkonduktanzverstärker mit Versorgungs-Pins und zentriertem Text unter dem Schlüssel label, standardmäßig g_m: in, out, vp, vn.|
|`$DIFFERENTIAL_TRANSCONDUCTOR`|`inp` `inn` `out`|Ein Transkonduktanzverstärker mit differentiellem Eingang und einem Text mit dem key label, standardmäßig g_m: inp, inn, out.|
|`$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Ein Transkonduktanzverstärker mit differentiellem Eingang, Versorgungs-Pins und einem Text mit dem key label, standardmäßig g_m: inp, inn, out, vp, vn.|
|`$DIFFERENTIAL_TRANSCONDUCTOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|Ein Transkonduktanzverstärker mit differentiellem Eingang, Versorgungs-Pins und einem Text mit dem key label, standardmäßig g_m, bei dem die Eingänge vertauscht (inn oben, inp unten) und die Versorgungen beibehalten sind (vp oben, vn unten) – anders als bei einem mit MX gespiegelten `$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY`: inp, inn, out, vp, vn.|
|`$ADC`|`in` `out`|Ein Analog-Digital-Umsetzer: in, out.|
|`$ADC_SUPPLY`|`in` `out` `vp` `vn`|Ein Analog-Digital-Umsetzer mit Versorgungs-Pins: in, out, vp, vn.|
|`$DAC`|`in` `out`|Ein Digital-Analog-Umsetzer: in, out.|
|`$DAC_SUPPLY`|`in` `out` `vp` `vn`|Ein Digital-Analog-Umsetzer mit Versorgungs-Pins: in, out, vp, vn.|
|`$CHOPPER_SYMBOL`|`inp` `inn` `outp` `outn`|Ein Chopper-Modulatorblock (abgegrenzt von der Schalterbrücken-Struktur `$CHOPPER`): inp, inn, outp, outn.|
|**rnl-std-rf**|||
|`$CENTER_TAP_INDUCTOR`|`n1` `n2` `tap`|Eine Induktivität mit Mittelanzapfung.|
|`$T_COIL`|`n1` `n2` `tap`|Eine T-Spule: zwei gekoppelte Spulen mit Mittelanzapfung und Überbrückungskondensator.|
|**rnl-std-logic**|||
|`$AND_2INPUT`|`a` `b` `y`|Ein AND-Gatter mit zwei Eingängen: a, b, y.|
|`$AND_3INPUT`|`a` `b` `c` `y`|Ein AND-Gatter mit drei Eingängen: a, b, c, y.|
|`$NAND_2INPUT`|`a` `b` `y`|Ein NAND-Gatter mit zwei Eingängen: a, b, y.|
|`$NAND_3INPUT`|`a` `b` `c` `y`|Ein NAND-Gatter mit drei Eingängen: a, b, c, y.|
|`$OR_2INPUT`|`a` `b` `y`|Ein OR-Gatter mit zwei Eingängen: a, b, y.|
|`$OR_3INPUT`|`a` `b` `c` `y`|Ein OR-Gatter mit drei Eingängen: a, b, c, y.|
|`$NOR_2INPUT`|`a` `b` `y`|Ein NOR-Gatter mit zwei Eingängen: a, b, y.|
|`$NOR_3INPUT`|`a` `b` `c` `y`|Ein NOR-Gatter mit drei Eingängen: a, b, c, y.|
|`$XOR_2INPUT`|`a` `b` `y`|Ein XOR-Gatter mit zwei Eingängen: a, b, y.|
|`$XOR_3INPUT`|`a` `b` `c` `y`|Ein XOR-Gatter mit drei Eingängen: a, b, c, y.|
|`$XNOR_2INPUT`|`a` `b` `y`|Ein XNOR-Gatter mit zwei Eingängen: a, b, y.|
|`$XNOR_3INPUT`|`a` `b` `c` `y`|Ein XNOR-Gatter mit drei Eingängen: a, b, c, y.|
|`$AND_4INPUT`|`a` `b` `c` `d` `y`|Ein AND-Gatter mit vier Eingängen auf ganzzahligen Rasterpunkten: a, b, c, d, y.|
|`$NAND_4INPUT`|`a` `b` `c` `d` `y`|Ein NAND-Gatter mit vier Eingängen auf ganzzahligen Rasterpunkten: a, b, c, d, y.|
|`$OR_4INPUT`|`a` `b` `c` `d` `y`|Ein OR-Gatter mit vier Eingängen auf ganzzahligen Rasterpunkten: a, b, c, d, y.|
|`$NOR_4INPUT`|`a` `b` `c` `d` `y`|Ein NOR-Gatter mit vier Eingängen auf ganzzahligen Rasterpunkten: a, b, c, d, y.|
|`$XOR_4INPUT`|`a` `b` `c` `d` `y`|Ein XOR-Gatter mit vier Eingängen auf ganzzahligen Rasterpunkten: a, b, c, d, y.|
|`$XNOR_4INPUT`|`a` `b` `c` `d` `y`|Ein XNOR-Gatter mit vier Eingängen auf ganzzahligen Rasterpunkten: a, b, c, d, y.|
|`$BUFFER`|`a` `y`|Ein Puffer: a, y.|
|`$INVERTER`|`a` `y`|Ein Inverter (ein NOT-Gatter): a, y.|
|`$DELAY_CELL`|`a` `y`|Eine Verzögerungszelle: a, y.|
|`$D_FLIP_FLOP`|`d` `ck` `q` `qb`|Ein D-Flipflop mit dem Ausgang Q und seinem Komplement: d, ck, q, qb.|
|`$D_FLIP_FLOP_Q_ONLY`|`d` `ck` `q`|Ein D-Flipflop nur mit dem Ausgang Q: d, ck, q.|
|`$D_FLIP_FLOP_WITH_RESET`|`d` `ck` `rst` `q` `qb`|Ein D-Flipflop mit Reset: d, ck, rst, q, qb.|
|`$AND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Ein AND-Gatter mit 2 Eingängen und Versorgungs-Pins: a, b, y, vp, vn.|
|`$AND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Ein AND-Gatter mit 3 Eingängen und Versorgungs-Pins: a, b, c, y, vp, vn.|
|`$AND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Ein AND-Gatter mit 4 Eingängen und Versorgungs-Pins: a, b, c, d, y, vp, vn.|
|`$NAND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Ein NAND-Gatter mit 2 Eingängen und Versorgungs-Pins: a, b, y, vp, vn.|
|`$NAND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Ein NAND-Gatter mit 3 Eingängen und Versorgungs-Pins: a, b, c, y, vp, vn.|
|`$NAND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Ein NAND-Gatter mit 4 Eingängen und Versorgungs-Pins: a, b, c, d, y, vp, vn.|
|`$OR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Ein OR-Gatter mit 2 Eingängen und Versorgungs-Pins: a, b, y, vp, vn.|
|`$OR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Ein OR-Gatter mit 3 Eingängen und Versorgungs-Pins: a, b, c, y, vp, vn.|
|`$OR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Ein OR-Gatter mit 4 Eingängen und Versorgungs-Pins: a, b, c, d, y, vp, vn.|
|`$NOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Ein NOR-Gatter mit 2 Eingängen und Versorgungs-Pins: a, b, y, vp, vn.|
|`$NOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Ein NOR-Gatter mit 3 Eingängen und Versorgungs-Pins: a, b, c, y, vp, vn.|
|`$NOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Ein NOR-Gatter mit 4 Eingängen und Versorgungs-Pins: a, b, c, d, y, vp, vn.|
|`$XOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Ein XOR-Gatter mit 2 Eingängen und Versorgungs-Pins: a, b, y, vp, vn.|
|`$XOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Ein XOR-Gatter mit 3 Eingängen und Versorgungs-Pins: a, b, c, y, vp, vn.|
|`$XOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Ein XOR-Gatter mit 4 Eingängen und Versorgungs-Pins: a, b, c, d, y, vp, vn.|
|`$XNOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|Ein XNOR-Gatter mit 2 Eingängen und Versorgungs-Pins: a, b, y, vp, vn.|
|`$XNOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|Ein XNOR-Gatter mit 3 Eingängen und Versorgungs-Pins: a, b, c, y, vp, vn.|
|`$XNOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|Ein XNOR-Gatter mit 4 Eingängen und Versorgungs-Pins: a, b, c, d, y, vp, vn.|
|`$BUFFER_SUPPLY`|`a` `y` `vp` `vn`|Ein Puffer mit Versorgungs-Pins: a, y, vp, vn.|
|`$INVERTER_SUPPLY`|`a` `y` `vp` `vn`|Ein Inverter mit Versorgungs-Pins: a, y, vp, vn.|
|`$DELAY_CELL_SUPPLY`|`a` `y` `vp` `vn`|Eine Verzögerungszelle mit Versorgungs-Pins: a, y, vp, vn.|
|`$D_FLIP_FLOP_SUPPLY`|`d` `ck` `q` `qb` `vp` `vn`|Ein D-Flipflop mit Versorgungs-Pins: d, ck, q, qb, vp, vn.|
|`$D_FLIP_FLOP_Q_ONLY_SUPPLY`|`d` `ck` `q` `vp` `vn`|Ein D-Flipflop nur mit Q und Versorgungs-Pins: d, ck, q, vp, vn.|
|`$D_FLIP_FLOP_WITH_RESET_SUPPLY`|`d` `ck` `rst` `q` `qb` `vp` `vn`|Ein D-Flipflop mit Reset und Versorgungs-Pins (vp und vn ausgerichtet bei x = 0): d, ck, rst, q, qb, vp, vn.|
|**rnl-std-signal-flow**|||
|`$ADDER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|Das Symbol von `$ADDER`: ein Summierkreis mit Pfeilspitzen an seinen Eingangsleitungen a (in1) und b (in2) sowie dem Ausgang y (out).|
|`$SUBTRACTOR_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|Das Symbol von `$SUBTRACTOR`: ein Summierkreis mit + in der Mitte und - außen an seinem negativen Eingang b (in2), den Eingängen a (in1) und b (in2) sowie dem Ausgang y (out).|
|`$MULTIPLIER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|Das Symbol von `$MULTIPLIER`: ein Produktkreis mit Pfeilspitzen an seinen Eingangsleitungen a (in1) und b (in2) sowie dem Ausgang y (out).|
|`$INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|Das Symbol von `$INTEGRATOR`: ein Kasten mit der Beschriftung 1/s, Eingang a (in) und Ausgang y (out).|
|`$DISCRETE_TIME_INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|Das Symbol von `$DISCRETE_TIME_INTEGRATOR`: ein Kasten mit seiner Übertragungsfunktion im z-Bereich, Eingang a (in) und Ausgang y (out).|
|`$QUANTIZER_SYMBOL`|`a` (`in`) `y` (`out`)|Das Symbol von `$QUANTIZER`: ein Kasten mit einer Treppenkennlinie, Eingang a (in) und Ausgang y (out).|
|`$UNIT_DELAY_SYMBOL`|`a` (`in`) `y` (`out`)|Das Symbol von `$UNIT_DELAY`: ein Kasten mit der Beschriftung z hoch minus eins, Eingang a (in) und Ausgang y (out).|
|**rnl-std-45deg**|||
|`$RESISTOR_45`|`p` `n`|Ein 45-Grad-Widerstand.|
|`$CAPACITOR_45`|`p` `n`|Ein 45-Grad-Kondensator.|
|`$INDUCTOR_45`|`p` `n`|Eine 45-Grad-Induktivität (dient auch für diagonale Transformator- oder Balun-Wicklungen).|
|`$SWITCH_45`|`p` `n`|Ein offen gezeichneter 45-Grad-Schalter.|
|`$VOLTAGE_SOURCE_45`|`p` `n`|Eine unabhängige 45-Grad-Spannungsquelle, + nahe p und - nahe n bleiben aufrecht.|
|`$CURRENT_SOURCE_45`|`p` `n`|Eine unabhängige 45-Grad-Stromquelle, Pfeil von p nach n.|
|`$DIODE_45`|`a` `k`|Eine 45-Grad-Diode.|
|`$ZENER_DIODE_45`|`a` `k`|Eine 45-Grad-Zener-Diode.|
|`$SCHOTTKY_DIODE_45`|`a` `k`|Eine 45-Grad-Schottky-Diode.|
|`$LED_45`|`a` `k`|Eine 45-Grad-Leuchtdiode (LED).|
|`$NMOS_45`|`d` `g` `s`|Ein dreipoliger 45-Grad-NMOS-Transistor.|
|`$NMOS_4TERM_45`|`d` `g` `s` `b`|Ein vierpoliger 45-Grad-NMOS-Transistor, Bulk bei (0, 0).|
|`$PMOS_45`|`d` `g` `s`|Ein dreipoliger 45-Grad-PMOS-Transistor.|
|`$PMOS_4TERM_45`|`d` `g` `s` `b`|Ein vierpoliger 45-Grad-PMOS-Transistor, Bulk bei (0, 0).|
|`$NPN_45`|`c` `b` `e`|Ein 45-Grad-NPN-Bipolartransistor.|
|`$PNP_45`|`c` `b` `e`|Ein 45-Grad-PNP-Bipolartransistor.|
|`$ARROW_45_SYMBOL`|—|Eine gefüllte Pfeilspitze in Richtung -45 Grad (nach rechts unten, entlang p->n bzw. a->k der 45-Grad-Symbole unter R90), die sich unter R0/R90/R180/R270 in alle vier Diagonalen dreht.|

### Mengen und Stile

|Name|Paket|Bedeutung|
|---|---|---|
|`$GROUND_NET_SET`|rnl-std-basic|Die Massenetze (standardmäßig 0), die eine Datei mit ADD-TO und REMOVE-FROM bearbeitet; rnl-std-fill versieht deren unverdrahtete Pins mit einem Masse-Stub und nimmt sie von der Leitungsführung aus. Wie eine Masse gezeichnet wird, hängt davon nicht ab.|
|`$SUPPLY_NET_SET`|rnl-std-basic|Die Versorgungsnetze (standardmäßig leer), die eine Datei mit ADD-TO und REMOVE-FROM bearbeitet.|
|`$BIAS_NET_SET`|rnl-std-basic|Die Bias-Netze (standardmäßig leer), die eine Datei mit ADD-TO und REMOVE-FROM bearbeitet.|
|`$PORT_FILL_SET`|rnl-std-basic|Die Portnetze, denen rnl-std-fill eine Port-Markierung gibt: jedes Portnetz jeder Ebene, auch Versorgungs- und Massenetze, abzüglich derer mit einem geschriebenen `$IO_PORT` sowie derer einer Ebene mit einem `$PORT_LIST`.|
|`$GROUND_FILL_SET`|rnl-std-fill|Die unverdrahteten Pins auf Massenetzen, denen die Ergänzung einen Masse-Stub gibt.|
|`$SUPPLY_FILL_SET`|rnl-std-fill|Die unverdrahteten Pins auf Versorgungsnetzen, denen die Ergänzung einen Versorgungs-Stub gibt.|
|`$BIAS_FILL_SET`|rnl-std-fill|Die unverdrahteten Pins auf Bias-Netzen, denen die Ergänzung einen Bias-Stub gibt.|
|`$LABEL_FILL_SET`|rnl-std-fill|Die verbleibenden unverdrahteten Pins auf Netzen, denen die Ergänzung einen Netzbeschriftungs-Stub gibt; ein Bus-Pin erhält einen gemeinsamen.|
|`$base`|rnl-std-style|Der Basisstil jedes Objekts: Linien, Texte und Füllungen, wie Analog Canvas sie zeichnet.|
|`$bus`|rnl-std-style|Der Stil eines Busses: seine Linien fett.|
|`$signal`|rnl-std-style|Ein Stil für Signalleitungen: blau und fett.|
|`$note`|rnl-std-style|Ein Stil für Notizen: grau, kleinere Schrift.|
|`$grey`|rnl-std-style|Ein grauer Stil für Linien und Texte.|

<!-- lists:end -->
