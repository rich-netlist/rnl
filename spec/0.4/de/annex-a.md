<!-- translated from en/annex-a.md sha256:2adc09eec4f4 -->
## Anhang A Schlüsselwortsignaturen (normativ)

Dieser Anhang wird aus der Wortliste `rnl-words-0.4.json` erzeugt (5.3); weichen beide voneinander ab, liegt ein Erratum vor. Er führt auf, was jedes Schlüsselwort annimmt und liefert; was es bedeutet, steht in dem in der letzten Spalte genannten Abschnitt.

- **Slots** sind in der Standardreihenfolge aufgeführt, mit ihren Namen (die als Rollen geschrieben werden dürfen, 5.3) und dem, was sie annehmen: *einzeln*: ein einzelnes Objekt; erhält er eine Menge, wird das Schlüsselwort auf jedes Mitglied angewendet (5.2.1), außer dass PRIORITY eine mit GROUP markierte Menge als die Gruppe selbst annimmt (14.8); *Menge*: eine Menge; lose für den Slot geschriebene Objekte bilden eine Menge (5.2.1); *beliebig*: ein Objekt oder eine Menge, verwendet wie vom Schlüsselwort angegeben (bei DISTANCE steht eine Menge für jedes ihrer Mitglieder, 14.5); *Form*: ein Punkt oder eine Menge von Punkten (zwei Formen einer Grafik, 9.2.5); *Rumpf*: Anweisungen in geschweiften Klammern, eingeschlossen (5.4); *Definition*: eine Definition: ein `$`-Name, ein an eine Definition gebundener `%`-Name oder eine anonyme Definition (5.6, 6.6). Absorbierte Eingaben sind strikt; eingeschlossene Eingaben sind tolerant (5.4, 7.3).
- **Arten** von Slots sind Artnamen (`instance`, `terminal`, `pin`, `net`, `point`, `wire`, `usage`, `bundle`, `text`, `annotation`, `label`) oder: *beliebig*, jedes Objekt; *Ebene*, `#` allein: die Ebene selbst; *Eigentümer*, `?` allein: in einem Symbolrumpf das mit dem Symbol gezeichnete Objekt; im Rumpf einer allgemeinen Definition die gerade expandierte Anwendung (9.4); *Position*, das Unterobjekt `::position` eines Objekts (sein Ursprung); *lokalisierbar*, ein Objekt mit Position: Instanz, Bündel, Pin, Anschluss mit Pin, Punkt, Text, der Mittelpunkt einer mit Mittelpunkt geschriebenen Grafik, eine Anwendung über ihren eigenen Punkt; eine dafür angegebene Menge steht für jedes ihrer Mitglieder (14.1); *ausrichtbar*, ein Objekt mit Ausrichtung: Instanz, Bündel, Anwendung mit Symbol; *Bedingung*, ein von einem Bedingungsschlüsselwort erzeugtes Objekt; *Menge*, eine Menge, als Name geschrieben; in ASSERT-TOPOLOGY auch ein Element, das ein ausgeschriebenes `{…}` ist (15.4); *leerer Knoten*, `&` allein: kein Netz; angenommen von Rollen der Art `net` und vom Netz-Slot von WIRE, und anderswo als leere Menge von Netzen (6.3).
- **Parameter** mit *Position 1*, *Position 2* dürfen nach Position in dieser Reihenfolge angegeben werden; alle anderen nur mit Namen (4.2). `priority` nimmt stets `"disabled"`, `"low"`, `"normal"` oder `"high"`; ohne ihn gilt die Priorität des Eintragskopfs, außer dass AS und ATTACH ausschließlich `"disabled"` daraus übernehmen (14.8).
- **Ergebnis**: das Objekt, das die Anweisung liefert (5.5), und wie ihre Fakten überschreiben (5.8). Ohne Überschreibung sammeln sich Fakten an.
- Sofern nicht vermerkt, darf ein Schlüsselwort in jedem Eintrag einer Ebene geschrieben werden; *Symbolrümpfe* bedeutet, dass es auch in einem DEFINE-SYMBOL-Rumpf geschrieben werden darf.

### A.1 Der Eintrag

`@RNL(…) {…}` schreibt seine Anweisungen auf der Ebene fest (5.4). Felder des Eintragskopfs (2.8), alle mit Namen angegeben:

| Feld | Werte | Standard | Beeinflusst Interpretation |
|---|---|---|---|
| `type` | `"authored"`, `"generated"`, `"header"` | `"authored"` | ja |
| `scope` | Zeichenkette | — | ja |
| `priority` | `"disabled"`, `"low"`, `"normal"`, `"high"` | `"normal"` | ja |
| `author` | Zeichenkette | — | nein |
| `hash` | Zeichenkette | — | nein |
| `numbering` | Zeichenkette | — | ja |
| `modified` | Zeichenkette | — | nein |
| `language` | `"spice"`, `"spectre"` | — | ja |

### A.2 Kopfeinträge und Pakete

| Schlüsselwort | Slots | Parameter | Ergebnis | Abschnitt |
|---|---|---|---|---|
| `@VERSION` (nur Kopfeinträge) | — | `version` (Position 1; erforderlich) | — | 16.1 |
| `@PACKAGE` (nur Kopfeintrag einer Paketdatei) | — | `name` (erforderlich), `version` (erforderlich) | — | 2.7 |
| `@USE-PACKAGE` (auch in Kopfeinträgen) | `body`: Rumpf, optional | `name` (erforderlich), `version`, `path` | — | 16.2 |
| `@ALIAS` (nur Kopfeinträge) | — | `alias` (Position 1; erforderlich), `original` (Position 2; erforderlich), `definition` | — | 16.5 |
| `@USE-ALIASES` (nur Kopfeinträge) | — | `name` (erforderlich), `version`, `path` | — | 16.5 |
| `@DEVICE` (nur Kopfeinträge) | — | `name`, `model`, `terminals` (Array), `type` (Zeichenkette); mindestens eines von `name`, `model`; mindestens eines von `terminals`, `type` | — | 6.12 |

### A.3 Definitionen

| Schlüsselwort | Slots | Parameter | Ergebnis | Abschnitt |
|---|---|---|---|---|
| `@DEFINE-SYMBOL` (auch in Symbolrümpfen) | `body`: Rumpf | `box` (Array) | eine Definition (Symbol) | 9.2 |
| `@DEFINE-STYLE` | `extends`: Rumpf (ein Stil) | Stileigenschaften (9.3) | eine Definition (Stil) | 9.3 |
| `@DEFINE` | `body`: Rumpf | `roles` (erforderlich; Array), `parameters` (Array) | eine Definition (allgemein) | 9.4 |

### A.4 Symbolinhalt und Grafiken

| Schlüsselwort | Slots | Parameter | Ergebnis | Abschnitt |
|---|---|---|---|---|
| `@PIN` (nur Symbolrümpfe) | `position`: einzeln (Punkt) | `terminal` (Anschluss oder nichtleeres Array von Anschlüssen; darf weggelassen werden), `label`, `direction` (`"right"`, `"up"`, `"left"`, `"down"`) | ein neuer Pin | 9.2.4 |
| `@LINE` (auch in Symbolrümpfen) | `points`: Menge (Punkt, Pin, Anschluss) | `arrow` (`"none"`, `"start"`, `"end"`, `"both"`; Standard `"none"`), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; Standard `"none"`), Linienstileigenschaften | eine Grafik des Symbols; in der Szene eine neue Anmerkung (13.4) | 9.2.5 |
| `@RECTANGLE` (auch in Symbolrümpfen) | `points`: Form (Punkt, Pin, Anschluss) | `width` (Zahl), `height` (Zahl), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; Standard `"none"`), Flächenstileigenschaften; `width` und `height` zusammen oder gar nicht | eine Grafik des Symbols; in der Szene eine neue Anmerkung (13.4) | 9.2.5 |
| `@CIRCLE` (auch in Symbolrümpfen) | `points`: Form (Punkt, Pin, Anschluss) | `radius` (Zahl), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; Standard `"none"`), Flächenstileigenschaften | eine Grafik des Symbols; in der Szene eine neue Anmerkung (13.4) | 9.2.5 |
| `@ELLIPSE` (auch in Symbolrümpfen) | `points`: Form (Punkt, Pin, Anschluss) | `width` (Zahl), `height` (Zahl), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; Standard `"none"`), Flächenstileigenschaften; `width` und `height` zusammen oder gar nicht | eine Grafik des Symbols; in der Szene eine neue Anmerkung (13.4) | 9.2.5 |
| `@ARC` (auch in Symbolrümpfen) | `points`: Menge (Punkt, Pin, Anschluss) | `arrow` (`"none"`, `"start"`, `"end"`, `"both"`; Standard `"none"`), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; Standard `"none"`), Linienstileigenschaften | eine Grafik des Symbols; in der Szene eine neue Anmerkung (13.4) | 9.2.5 |
| `@BEZIER` (auch in Symbolrümpfen) | `points`: Menge (Punkt, Pin, Anschluss) | `arrow` (`"none"`, `"start"`, `"end"`, `"both"`; Standard `"none"`), `fill` (`"none"`, `"linecolor"`, `"fillcolor"`; Standard `"none"`), Linienstileigenschaften | eine Grafik des Symbols; in der Szene eine neue Anmerkung (13.4) | 9.2.5 |
| `@ANNOTATION` | `at`: einzeln (lokalisierbar); `source`: einzeln (ein Symbol, eine allgemeine Definition oder eine Anwendung), zuletzt geschrieben | `rotate` (Zahl, Grad gegen den Uhrzeigersinn; Standard `0`), `mirror` (Wahrheitswert; Standard `false`), Stileigenschaften sowie die Parameter eines als `source` angegebenen Symbols oder einer allgemeinen Definition, mit Namen (9.4) | eine neue Anmerkung | 13.5 |
| `@TEXT` (auch in Symbolrümpfen) | `attach`: einzeln (Punkt, Instanz, Anwendung, Bündel, Pin, Anschluss, Leitung, Anmerkung, Netz, `?`) | `text` (Position 1; erforderlich), `key`, `dx` (Zahl), `dy` (Zahl), `halign` (`"start"`, `"middle"`, `"end"`; Standard `"start"`), `valign` (`"baseline"`, `"top"`, `"middle"`, `"bottom"`; Standard `"baseline"`), `rotate` (Zahl, Grad gegen den Uhrzeigersinn; Standard `0`), `fix_rotate` (Wahrheitswert; Standard `false`), `upright` (Wahrheitswert; Standard `true`), Textstileigenschaften | ein neuer Text; mit `key` ein Unterobjekt des angehängten Objekts; überschreibt nach Schlüssel (Schlüssel: angehängtes Objekt und Schlüssel, wenn ein Schlüssel angegeben ist) | 13.3 |

### A.5 Platzierung und Darstellung

| Schlüsselwort | Slots | Parameter | Ergebnis | Abschnitt |
|---|---|---|---|---|
| `@PLACE` (auch in Symbolrümpfen) | `target`: einzeln (Instanz, Anwendung, Bündel, Punkt, Text, `::position`) | `x` (Position 1; Zahl), `y` (Position 2; Zahl), `priority`; mindestens eines von `x`, `y` | das Eingabeobjekt; geometrischer Fakt (Schlüssel: Objekt und Achse) | 11.1 |
| `@ORIENT` (auch in Symbolrümpfen) | `target`: einzeln (Instanz, Anwendung, Bündel, Pin) | `code` (Position 1; erforderlich; `"R0"`, `"R90"`, `"R180"`, `"R270"`, `"MX"`, `"MY"`, `"MXR90"`, `"MYR90"` oder die Wörter `"left"`, `"down"`, `"right"`, `"up"` für die ersten vier; oder ein nichtleeres Array davon), `priority` | das Eingabeobjekt; geometrischer Fakt (Schlüssel: Objekt oder Pin) | 11.2 |
| `@LAYER` | `target`: einzeln (Instanz, Anwendung, Bündel, Leitung, Anmerkung, Text) | `layer` (Position 1; erforderlich; `"bottom"`, `"middle"`, `"top"`) | das Eingabeobjekt; überschreibt nach Schlüssel (Schlüssel: Objekt) | 11.6 |
| `@SET-SYMBOL` (auch in Symbolrümpfen) | `target`: einzeln (Instanz, Anwendung, Bündel, `#`); `symbol`: Definition (ein Symbol) | `terminals` (Array von Anschlussnamen oder von Paaren) | das Eingabeobjekt; Darstellungswarteschlange | 11.3 |
| `@SET-STYLE` | `target`: einzeln (Instanz, Anwendung, Bündel, Leitung, Anmerkung, Text); `style`: Definition (ein Stil) | — | das Eingabeobjekt; Darstellungswarteschlange | 11.3 |

### A.6 Auswahlen

| Schlüsselwort | Slots | Parameter | Ergebnis | Abschnitt |
|---|---|---|---|---|
| `@SELECT-INSTANCE` | `scope`: beliebig (`#`, beliebiges Objekt) | `name`, `model`, `type`, `terminals` (Zahl, ganzzahlig), `symbol` (Zeichenkette: ein `$`-Name, mit `*`), `drawn` (Wahrheitswert), `role`, `recursive` (Wahrheitswert; Standard `false`) | eine neue Menge von Instanzen | 10.1 |
| `@SELECT-TERMINAL` | `scope`: beliebig (`#`, beliebiges Objekt) | `name`, `model`, `type`, `terminal` (Anschluss), `net`, `current` (`"in"`, `"out"`, `"unspecified"`), `role`, `recursive` (Wahrheitswert; Standard `false`) | eine neue Menge von Anschlüssen | 10.1 |
| `@SELECT-PIN` | `scope`: beliebig (`#`, beliebiges Objekt) | `name`, `model`, `type`, `terminal` (Anschluss), `net`, `wired` (Wahrheitswert), `role`, `recursive` (Wahrheitswert; Standard `false`) | eine neue Menge von Pins | 10.1 |
| `@SELECT-NET` | `scope`: beliebig (`#`, beliebiges Objekt) | `name`, `port` (Wahrheitswert), `wired` (Wahrheitswert), `global` (Wahrheitswert), `role`, `recursive` (Wahrheitswert; Standard `false`) | eine neue Menge von Netzen | 10.1 |
| `@SELECT-WIRE` | `scope`: beliebig (`#`, beliebiges Objekt) | `net`, `bus` (Wahrheitswert), `role`, `recursive` (Wahrheitswert; Standard `false`) | eine neue Menge von Leitungen | 10.1 |
| `@SELECT-LABEL` | `scope`: beliebig (`#`, beliebiges Objekt) | `key`, `role`, `recursive` (Wahrheitswert; Standard `false`) | eine neue Menge von Beschriftungen | 10.1 |
| `@SELECT-ANNOTATION` | `scope`: beliebig (`#`, beliebiges Objekt) | `role`, `recursive` (Wahrheitswert; Standard `false`) | eine neue Menge von Anmerkungen | 10.1 |
| `@SELECT-USAGE` | `scope`: beliebig (`#`, beliebiges Objekt); `definition`: Definition (eine allgemeine Definition), optional | `symbol` (Zeichenkette: ein `$`-Name, mit `*`), `drawn` (Wahrheitswert), `recursive` (Wahrheitswert; Standard `false`) | eine neue Menge von Anwendungen | 10.1 |
| `@SELECT-ANCESTOR-USAGE` | `scope`: beliebig (beliebiges Objekt); `definition`: Definition (eine allgemeine Definition) | `recursive` (Wahrheitswert; Standard `false`) | eine neue Menge von Anwendungen | 10.1 |

### A.7 Anwenden von Definitionen

| Schlüsselwort | Slots | Parameter | Ergebnis | Abschnitt |
|---|---|---|---|---|
| `@AS` (auch in Symbolrümpfen) | die Rollen der Definition, dann die Definition (`definition`, zuletzt geschrieben) | die Parameter der Definition (nach Position in der Reihenfolge ihrer Parameterliste oder mit Namen; 9.4), `priority` | eine neue Anwendung | 9.4 |
| `@ATTACH` (auch in Symbolrümpfen) | die Definition, dann ihre Rollen (`definition`, zuerst geschrieben) | die Parameter der Definition (nach Position in der Reihenfolge ihrer Parameterliste oder mit Namen; 9.4), `priority` | eine neue Anwendung | 9.4 |

### A.8 Leitungen

| Schlüsselwort | Slots | Parameter | Ergebnis | Abschnitt |
|---|---|---|---|---|
| `@WIRE` | `net`: beliebig (Netz oder der leere Knoten); `points`: Menge (Punkt, Pin; ein Anschluss steht für seinen Pin, 5.2.4) | `mode` (`"auto"`, `"straight"`, `"horizontal-vertical"`, `"vertical-horizontal"`, `"manhattan"`, `"manhattan45"`; Standard `"auto"`), Leitungsstileigenschaften | eine neue Leitung; bei Übergabe einer Leitung wird sie neu geschrieben, überschreibend nach Schlüssel (Schlüssel: jene Leitung) | 13.1 |
| `@ROUTE-NET` | `net`: einzeln (Netz) | `mode` (`"auto"`, `"straight"`, `"horizontal-vertical"`, `"vertical-horizontal"`, `"manhattan"`, `"manhattan45"`; Standard `"auto"`), Leitungsstileigenschaften, `priority` | ein neuer Pfad | 13.7 |
| `@ROUTE-OBJECT` | `objects`: Menge (beliebiges Objekt) | `mode` (wie ROUTE-NET), Leitungsstileigenschaften, `priority` | das Eingabeobjekt | 13.7 |
| `@UNROUTE-NET` | `net`: einzeln (Netz) | `priority` | das Eingabeobjekt | 13.7 |
| `@UNROUTE-OBJECT` | `objects`: Menge (beliebiges Objekt) | `priority` | das Eingabeobjekt | 13.7 |

### A.9 Mengen, Gruppen und Bündel

| Schlüsselwort | Slots | Parameter | Ergebnis | Abschnitt |
|---|---|---|---|---|
| `@BUNDLE` | `members`: Menge (Instanz) | — | ein neues Bündel | 11.8 |
| `@GROUP` | `members`: Menge (beliebiges Objekt) | `spacing_x` (Zahl, ganzzahlig), `spacing_y` (Zahl, ganzzahlig) | das Eingabeobjekt | 11.5 |
| `@ADD-TO` | `items`: beliebig (beliebiges Objekt); `target`: beliebig (Menge) | — | die Zielmenge | 5.2.2 |
| `@REMOVE-FROM` | `items`: beliebig (beliebiges Objekt); `target`: beliebig (Menge) | — | die Zielmenge | 5.2.2 |
| `@UNION` | `sets`: Menge (beliebiges Objekt) | — | eine neue Menge | 5.2.2 |

### A.10 Bedingungen und Priorität

| Schlüsselwort | Slots | Parameter | Ergebnis | Abschnitt |
|---|---|---|---|---|
| `@ALIGN` | `members`: Menge (lokalisierbar) | `axis` (Position 1; erforderlich; `"x"`, `"y"`), `priority` | eine neue Bedingung; geometrischer Fakt | 14.2 |
| `@ORDER` | `members`: Menge (lokalisierbar) | `direction` (Position 1; erforderlich; `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`), `priority` | eine neue Bedingung; geometrischer Fakt | 14.3 |
| `@DISTRIBUTE` | `members`: Menge (lokalisierbar) | `direction` (Position 1; erforderlich; `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`), `priority` | eine neue Bedingung; geometrischer Fakt | 14.4 |
| `@DISTANCE` | `from`: beliebig (lokalisierbar); `to`: beliebig (lokalisierbar) | `direction` (Position 1; erforderlich; `"left-to-right"`, `"right-to-left"`, `"bottom-to-top"`, `"top-to-bottom"`), `min` (Zahl), `max` (Zahl), `exact` (Zahl), `priority`; mindestens eines von `min`, `max`, `exact`; `exact` nicht mit `min`; `exact` nicht mit `max` | eine neue Bedingung; geometrischer Fakt | 14.5 |
| `@ORIENT-RELATION` | `from`: einzeln (ausrichtbar oder Pin); `to`: einzeln (ausrichtbar oder Pin, wie `from`) | `relation` (Position 1; erforderlich; die acht Codes von ORIENT, nicht dessen Wörter, oder ein nichtleeres Array davon), `priority` | eine neue Bedingung; geometrischer Fakt | 14.9 |
| `@TABLE` | `rows`: Menge (lokalisierbar; geschachtelte Mengen sind die Listen der Tabelle) | `transpose` (Wahrheitswert; Standard `false`), `priority` | eine neue Bedingung; geometrischer Fakt | 14.11 |
| `@SOLVER-CONFIG` (Kopfeintrag ohne `language`, Kopfeintrag einer Paketdatei und Ebene) | — | benannte Parameter des Solvers: Zahlen, Zeichenketten, Wahrheitswerte und Arrays davon; `spacing_x`, `spacing_y` (Zahl, ganzzahlig) und `junction` (Zeichenkette: ein `$`-Name; in einem Ebeneneintrag auch ein `%`-Name der Ebene; oder leer) sind vom Standard benannt; die übrigen Schlüssel sind jene des Solvers | kein Objekt; Einstellungen des Solvers | 14.10 |
| `@PRIORITY` | `target`: einzeln (Bedingung, Anwendung, Menge) | `priority` (Position 1; erforderlich) | das Eingabeobjekt; überschreibt nach Schlüssel (Schlüssel: Objekt) | 14.8 |

### A.11 Elektrische Semantik

| Schlüsselwort | Slots | Parameter | Ergebnis | Abschnitt |
|---|---|---|---|---|
| `@CURRENT` | `terminal`: einzeln (Anschluss) | `direction` (Position 1; erforderlich; `"in"`, `"out"`, `"unspecified"`) | das Eingabeobjekt; überschreibt nach Schlüssel (Schlüssel: Anschluss) | 15.2 |
| `@ASSERT-INSTANCE` | `members`: Menge (Instanz) | die Bedingungen von SELECT-INSTANCE außer `role` und `recursive`, `has` (Array von Attributnamen), `same` (Array von Attributnamen), `different` (Array von Attributnamen), `message` | kein Objekt; eine Prüfung | 15.3 |
| `@ASSERT-TERMINAL` | `members`: Menge (Anschluss) | die Bedingungen von SELECT-TERMINAL außer `role` und `recursive`, `same`, `different`, `message` | kein Objekt; eine Prüfung | 15.3 |
| `@ASSERT-NET` | `members`: Menge (Netz) | die Bedingungen von SELECT-NET außer `role` und `recursive`, `same`, `different`, `message` | kein Objekt; eine Prüfung | 15.3 |
| `@ASSERT-USAGE` | `members`: Menge (Anwendung); `definition`: Definition (eine allgemeine Definition), optional | `symbol`, `drawn` (Wahrheitswert), `same`, `different` (Arrays; nur `"definition"`), `message` | kein Objekt; eine Prüfung | 15.3 |
| `@ASSERT-IN` | `members`: Menge (beliebiges Objekt); `set`: beliebig (Menge) | `message` | kein Objekt; eine Prüfung | 15.3 |
| `@ASSERT-TOPOLOGY` | `members`: Menge (Instanz, Bündel, Anwendung, Menge) | `topology` (Position 1; erforderlich; `"series"`, `"parallel"`, `"star"`, `"loop"`), `exclude` (Array von Paaren: Typ, Anschluss), `message` | kein Objekt; eine Prüfung | 15.4 |

### A.12 Weitere Standardwörter

- Arten: `instance`, `terminal`, `pin`, `net`, `point`, `wire`, `usage`, `bundle`, `text`, `annotation`, `label` (5.3, 9.4).
- Unterobjekte nach `::`: `position`, `net` (6.5).
- Von `${…}` gelesene Attribute: `name`, `value`, `model`, `type`, `net`, `x`, `y` (13.6).
- Wahrheitswerte: `true`, `false` (3.7).

Stileigenschaften (9.3):

| Eigenschaft | Typ und Werte | Standard |
|---|---|---|
| `linecolor` | Farbe | — |
| `linestyle` | `"solid"`, `"dashed"`, `"dotted"`, `"dash-dot"`, `"double"`, `"bold"`, `"none"` | `"solid"` |
| `linewidth` | Zahl | `0` |
| `cap` | `"butt"`, `"round"`, `"square"` | — |
| `join` | `"miter"`, `"round"`, `"bevel"` | — |
| `fillcolor` | Farbe | — |
| `textcolor` | Farbe | — |
| `size` | Zahl | — |
| `font` | Zeichenkette | — |
| `bold` | Wahrheitswert | — |
| `italic` | Wahrheitswert | — |
| `arrowsize` | Zahl | — |
| `textvisible` | Wahrheitswert | `true` |

Stileigenschaften, die Schlüsselwörter als Parameter annehmen (9.2.5): *Linie*: `linecolor`, `linestyle`, `linewidth`, `cap`, `join`, `fillcolor`, `arrowsize`; *Fläche*: `linecolor`, `linestyle`, `linewidth`, `cap`, `join`, `fillcolor`; *Leitung*: `linecolor`, `linestyle`, `linewidth`, `cap`, `join`; *Text*: `textcolor`, `size`, `font`, `bold`, `italic`, `textvisible`.
