<!-- translated from en/front.md sha256:0cc8c8aaa953 -->

# RNL-Sprachstandard, Version 0.4

RNL (Rich Netlist) wird in den Kommentaren einer Schaltungsnetzliste geschrieben. Sie beschreibt, wie die Schaltung als Schaltplan gezeichnet wird und was ihre Teile bedeuten, während allein die Netzliste die Schaltung definiert. Dieses Dokument ist der Standard der Version 0.4 der Sprache.

- Der englische Text ist normativ. Seine Übersetzungen, benannt `rnl-0.4.<lang>.md` nach einem Sprachtag wie `zh` oder `ja`, sind informativ; bei Abweichungen gilt der englische Text.
- Die Wortliste `rnl-words-0.4.json` und ihr Schema `rnl-words-0.4.schema.json` sind Teil dieses Standards (5.3, Anhang A).
- Die Prinzipien von RNL stehen in `rich-netlist.md` (englisch, maßgeblich; Übersetzungen: `rich-netlist.<lang>.md`); dieser Standard verweist darauf als „Prinzipien“.
- Versionen 0.x geben keine Zusicherung gegenseitiger Kompatibilität (16.1).
- Dieser Text überarbeitet Version 0.3 gemäß dem Entwurf der Version 0.4 vom 2026-11-19; Anhang D führt die Änderungen auf.

## Inhalt

- [1 Überblick](#1-überblick)
  - [1.1 Anwendungsbereich](#11-anwendungsbereich)
  - [1.2 Aufbau dieses Standards](#12-aufbau-dieses-standards)
  - [1.3 Auf einen Blick](#13-auf-einen-blick)
  - [1.4 Begriffe und Konventionen](#14-begriffe-und-konventionen)
  - [1.5 Kernprinzipien](#15-kernprinzipien)
  - [1.6 Konformität](#16-konformität)
- [2 Einbettung in den Host](#2-einbettung-in-den-host)
  - [2.1 Zweistufiges Parsen](#21-zweistufiges-parsen)
  - [2.2 Träger](#22-träger)
  - [2.3 Der Einstieg](#23-der-einstieg)
  - [2.4 Ausdehnung eines Eintrags](#24-ausdehnung-eines-eintrags)
  - [2.5 Deaktivierte Einträge und Host-Text](#25-deaktivierte-einträge-und-host-text)
  - [2.6 Ebenen](#26-ebenen)
  - [2.7 Paketdateien](#27-paketdateien)
  - [2.8 Eintragskopf](#28-eintragskopf)
  - [2.9 Verilog-A-Dateien](#29-verilog-a-dateien)
- [3 Lexikalische Struktur](#3-lexikalische-struktur)
  - [3.1 Zeichen](#31-zeichen)
  - [3.2 Tokens](#32-tokens)
  - [3.3 Freitext](#33-freitext)
  - [3.4 Kommentare](#34-kommentare)
  - [3.5 Leerraum und Nachbarschaft](#35-leerraum-und-nachbarschaft)
  - [3.6 Namen](#36-namen)
  - [3.7 Literale](#37-literale)
  - [3.8 Punkte](#38-punkte)
- [4 Syntax](#4-syntax)
  - [4.1 Grammatik](#41-grammatik)
  - [4.2 Bedingungen außerhalb der Grammatik](#42-bedingungen-außerhalb-der-grammatik)
  - [4.3 Wortstellung](#43-wortstellung)
  - [4.4 Beispiele für Fehler](#44-beispiele-für-fehler)
  - [4.5 Standardsätze](#45-standardsätze)
- [5 Objekte und Anweisungen](#5-objekte-und-anweisungen)
  - [5.1 Objekte](#51-objekte)
  - [5.2 Mengen](#52-mengen)
  - [5.3 Anweisungen und Signaturen](#53-anweisungen-und-signaturen)
  - [5.4 Arten der Eingabeverarbeitung](#54-arten-der-eingabeverarbeitung)
  - [5.5 Pipes und Verschachtelung](#55-pipes-und-verschachtelung)
  - [5.6 Definitionen als Objekte](#56-definitionen-als-objekte)
  - [5.7 Handles und Namen](#57-handles-und-namen)
  - [5.8 Schlüssel von Fakten](#58-schlüssel-von-fakten)
  - [5.9 Nummern von Objekten ohne Namen](#59-nummern-von-objekten-ohne-namen)
- [6 Namen und Geltungsbereich](#6-namen-und-geltungsbereich)
  - [6.1 Hierarchie der Ebenen](#61-hierarchie-der-ebenen)
  - [6.2 Präfixe](#62-präfixe)
  - [6.3 Host-Verweise](#63-host-verweise)
  - [6.4 Anschlüsse](#64-anschlüsse)
  - [6.5 Unterobjekte](#65-unterobjekte)
  - [6.6 Benennung](#66-benennung)
  - [6.7 Geltungsbereich von Namen](#67-geltungsbereich-von-namen)
  - [6.8 Vererbung und Verdeckung](#68-vererbung-und-verdeckung)
  - [6.9 Namen in Definitionsrümpfen](#69-namen-in-definitionsrümpfen)
  - [6.10 Platzhalter](#610-platzhalter)
  - [6.11 Subskripte](#611-subskripte)
  - [6.12 Anschlussnamen: DEVICE](#612-anschlussnamen-device)
- [7 Fehlschläge und Diagnosemeldungen](#7-fehlschläge-und-diagnosemeldungen)
  - [7.1 Zwei Phasen der Prüfung](#71-zwei-phasen-der-prüfung)
  - [7.2 Fehlgeschlagene Objekte](#72-fehlgeschlagene-objekte)
  - [7.3 Strikte und tolerante Eingaben](#73-strikte-und-tolerante-eingaben)
  - [7.4 Fehlerbehebung](#74-fehlerbehebung)
  - [7.5 Konflikte sind keine Fehlschläge](#75-konflikte-sind-keine-fehlschläge)
  - [7.6 Diagnosemeldungen](#76-diagnosemeldungen)
- [8 Wirksamwerden](#8-wirksamwerden)
  - [8.1 Eine Ebene wird nach vollständigem Einlesen wirksam](#81-eine-ebene-wird-nach-vollständigem-einlesen-wirksam)
  - [8.2 Phasen](#82-phasen)
  - [8.3 Darstellungswarteschlangen](#83-darstellungswarteschlangen)
  - [8.4 Überschreibung nach Schlüssel](#84-überschreibung-nach-schlüssel)
  - [8.5 Konflikte](#85-konflikte)
- [9 Definitionen](#9-definitionen)
  - [9.1 Allgemeine Regeln](#91-allgemeine-regeln)
  - [9.2 Symbole: DEFINE-SYMBOL](#92-symbole-define-symbol)
  - [9.3 Stile: DEFINE-STYLE](#93-stile-define-style)
  - [9.4 Allgemeine Definitionen: DEFINE](#94-allgemeine-definitionen-define)
  - [9.5 Verwendungszwecke allgemeiner Definitionen](#95-verwendungszwecke-allgemeiner-definitionen)
- [10 Auswahlen, Regeln und schrittweise Generierung](#10-auswahlen-regeln-und-schrittweise-generierung)
  - [10.1 Auswahlschlüsselwörter](#101-auswahlschlüsselwörter)
  - [10.2 Die Bedingung `wired`](#102-die-bedingung-wired)
  - [10.3 Auswerten von Auswahlen](#103-auswerten-von-auswahlen)
  - [10.4 Regeln](#104-regeln)
  - [10.5 Schrittweise Generierung](#105-schrittweise-generierung)
- [11 Platzierung und Darstellung](#11-platzierung-und-darstellung)
  - [11.1 Position: PLACE](#111-position-place)
  - [11.2 Ausrichtung: ORIENT](#112-ausrichtung-orient)
  - [11.3 Symbole und Stile: SET-SYMBOL und SET-STYLE](#113-symbole-und-stile-set-symbol-und-set-style)
  - [11.4 Mehrere Zeichnungen eines Objekts](#114-mehrere-zeichnungen-eines-objekts)
  - [11.5 Gruppen: GROUP](#115-gruppen-group)
  - [11.6 Zeichenreihenfolge und Zeichenebenen: LAYER](#116-zeichenreihenfolge-und-zeichenebenen-layer)
  - [11.7 Koordinaten und Einheiten](#117-koordinaten-und-einheiten)
  - [11.8 Bündel: BUNDLE](#118-bündel-bundle)
- [12 Virtuelle Bauteile](#12-virtuelle-bauteile)
  - [12.1 Konzept](#121-konzept)
  - [12.2 Erzeugen virtueller Bauteile](#122-erzeugen-virtueller-bauteile)
  - [12.3 Positionen relativ zu einem Anschluss](#123-positionen-relativ-zu-einem-anschluss)
  - [12.4 Ports](#124-ports)
  - [12.5 Standardmäßige virtuelle Bauteile](#125-standardmäßige-virtuelle-bauteile)
  - [12.6 Nicht-verbunden-Markierungen](#126-nicht-verbunden-markierungen)
- [13 Leitungen und Text](#13-leitungen-und-text)
  - [13.1 Leitungen: WIRE](#131-leitungen-wire)
  - [13.2 Knotenpunkte](#132-knotenpunkte)
  - [13.3 Text: TEXT](#133-text-text)
  - [13.4 Anmerkungen: Grafiken in der Szene](#134-anmerkungen-grafiken-in-der-szene)
  - [13.5 Als Anmerkungen gezeichnete Symbole: ANNOTATION](#135-als-anmerkungen-gezeichnete-symbole-annotation)
  - [13.6 Textauszeichnung und Attributverweise](#136-textauszeichnung-und-attributverweise)
  - [13.7 Verbindungen als Leitungen zeichnen: ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET, UNROUTE-OBJECT](#137-verbindungen-als-leitungen-zeichnen-route-net-route-object-unroute-net-unroute-object)
- [14 Layout-Absicht](#14-layout-absicht)
  - [14.1 Gemeinsame Regeln](#141-gemeinsame-regeln)
  - [14.2 ALIGN](#142-align)
  - [14.3 ORDER](#143-order)
  - [14.4 DISTRIBUTE](#144-distribute)
  - [14.5 DISTANCE](#145-distance)
  - [14.6 Signalfluss](#146-signalfluss)
  - [14.7 Symmetrie](#147-symmetrie)
  - [14.8 Priorität: PRIORITY](#148-priorität-priority)
  - [14.9 Ausrichtungsbeziehung: ORIENT-RELATION](#149-ausrichtungsbeziehung-orient-relation)
  - [14.10 Solver-Einstellungen: SOLVER-CONFIG](#1410-solver-einstellungen-solver-config)
  - [14.11 Tabellen: TABLE](#1411-tabellen-table)
- [15 Elektrische Semantik](#15-elektrische-semantik)
  - [15.1 Anwenden semantischer Definitionen](#151-anwenden-semantischer-definitionen)
  - [15.2 Stromrichtung: CURRENT](#152-stromrichtung-current)
  - [15.3 Zusicherungen: ASSERT-INSTANCE, ASSERT-TERMINAL, ASSERT-NET, ASSERT-USAGE, ASSERT-IN](#153-zusicherungen-assert-instance-assert-terminal-assert-net-assert-usage-assert-in)
  - [15.4 Topologiezusicherungen: ASSERT-TOPOLOGY](#154-topologiezusicherungen-assert-topology)
- [16 Dateien, Versionen, Pakete und Aliase](#16-dateien-versionen-pakete-und-aliase)
  - [16.1 Versionen](#161-versionen)
  - [16.2 Pakete und USE-PACKAGE](#162-pakete-und-use-package)
  - [16.3 Das Standardpaket](#163-das-standardpaket)
  - [16.4 Erweiterungen und unbekannter Inhalt](#164-erweiterungen-und-unbekannter-inhalt)
  - [16.5 Aliase](#165-aliase)
- [17 Änderungsmodell](#17-änderungsmodell)
- [Anhang A Schlüsselwortsignaturen (normativ)](#anhang-a-schlüsselwortsignaturen-normativ)
  - [A.1 Der Eintrag](#a1-der-eintrag)
  - [A.2 Kopfeinträge und Pakete](#a2-kopfeinträge-und-pakete)
  - [A.3 Definitionen](#a3-definitionen)
  - [A.4 Symbolinhalt und Grafiken](#a4-symbolinhalt-und-grafiken)
  - [A.5 Platzierung und Darstellung](#a5-platzierung-und-darstellung)
  - [A.6 Auswahlen](#a6-auswahlen)
  - [A.7 Anwenden von Definitionen](#a7-anwenden-von-definitionen)
  - [A.8 Leitungen](#a8-leitungen)
  - [A.9 Mengen, Gruppen und Bündel](#a9-mengen-gruppen-und-bündel)
  - [A.10 Bedingungen und Priorität](#a10-bedingungen-und-priorität)
  - [A.11 Elektrische Semantik](#a11-elektrische-semantik)
  - [A.12 Weitere Standardwörter](#a12-weitere-standardwörter)
- [Anhang B Standardsätze (informativ)](#anhang-b-standardsätze-informativ)
  - [B.1 Platzierung und Ausrichtung](#b1-platzierung-und-ausrichtung)
  - [B.2 Layout-Bedingungen](#b2-layout-bedingungen)
  - [B.3 Darstellung](#b3-darstellung)
  - [B.4 Symbole und Grafiken](#b4-symbole-und-grafiken)
  - [B.5 Leitungen und Text](#b5-leitungen-und-text)
  - [B.6 Definitionen und Anwendungen](#b6-definitionen-und-anwendungen)
  - [B.7 Auswahlen](#b7-auswahlen)
  - [B.8 Mengen und weitere Schlüsselwörter](#b8-mengen-und-weitere-schlüsselwörter)
  - [B.9 Kopfeintrag und Pakete](#b9-kopfeintrag-und-pakete)
- [Anhang C Für spätere Versionen reserviert (informativ)](#anhang-c-für-spätere-versionen-reserviert-informativ)
- [Anhang D Änderungen gegenüber Version 0.3 (informativ)](#anhang-d-änderungen-gegenüber-version-03-informativ)

<!-- translated from en/ch01.md sha256:e38c6540c72f -->

## 1 Überblick

### 1.1 Anwendungsbereich

RNL (Rich Netlist) wird in den Kommentaren einer Host-Netzliste geschrieben. Sie beschreibt, wie die durch die Netzliste definierte Schaltung verstanden und als Schaltplan dargestellt wird: welche Symbole verwendet werden, wo Bauteile liegen, wie sie ausgerichtet, aneinander ausgerichtet, mit Leitungen verbunden, beschriftet und gestaltet sind und was Gruppen von Bauteilen bedeuten (Prinzipien 1).

Bauteile, Typen, Parameter, Verbindungen und Teilschaltungsschnittstellen werden ausschließlich durch die Host-Netzliste definiert. RNL verweist auf sie und kann sie nicht ändern. RNL prüft nicht die elektrische Bedeutung, die eine Zeichnung nahelegt (Prinzipien 2.2).

Dieser Standard definiert:

- wie RNL aus den Kommentaren einer Host-Netzliste und aus den von ihr eingebrachten Verilog-A-Dateien entnommen wird (Kapitel 2);
- die lexikalische Struktur und die Syntax von RNL-Text (Kapitel 3 und 4);
- welche Objekte der Text beschreibt, wie Namen aufgelöst werden, wie Fehler behandelt werden und wie die Beschreibungen zusammen wirksam werden (Kapitel 5–8);
- das Vokabular jeder Art von Beschreibung (Kapitel 9–17);
- die Standardwortliste, die zusammen mit diesem Text als `rnl-words-0.4.json` veröffentlicht wird (5.3, Anhang A).

Dieser Standard definiert weder einen Layout-Algorithmus noch eine eindeutige Darstellung (Prinzipien 1), und er definiert nicht, wie eine Host-Netzliste geparst wird (Prinzipien 2.1). Das Standardpaket `rnl-std-all` und Alias-Pakete wie `alias-zh` sind in RNL geschrieben; dieser Standard definiert, wie Pakete funktionieren (16.2, 16.3, 16.5), während ihr Inhalt gesondert veröffentlicht wird.

Dieser Text definiert Version 0.4 der Sprache. Versionen 0.x geben keine Zusicherung gegenseitiger Kompatibilität (16.1).

### 1.2 Aufbau dieses Standards

Der Standard besteht aus einem abstrakten Modell und einer Textsyntax. Das abstrakte Modell umfasst Objekte, Fakten, Arten der Eingabeverarbeitung und Phasen des Wirksamwerdens (Kapitel 5–8); die Textsyntax ist seine geschriebene Form (Kapitel 2–4). Werkzeuge tauschen den Text aus.

Anhang A listet die Signaturen der Schlüsselwörter auf; er wird aus der Wortliste erzeugt und ist normativ. Anhang B gibt die Standardsätze an, Anhang C listet das für spätere Versionen Reservierte auf, und Anhang D führt die Änderungen gegenüber Version 0.3 auf; alle drei sind informativ. Hinweise und Beispiele sind informativ. Alles Übrige ist normativ.

### 1.3 Auf einen Blick

```spice
* Five-transistor OTA, input stage
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
.model nch nmos level=1
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   %dp1 = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N;
*   {#M1 #M2} @ALIGN("y"), i.e., same y;
*   #M5 is at @PLACE(20, 6) => it @ORIENT("R0");
*   %g0 = $GROUND is @ATTACH to &vss;
*   net &vss, through {#M5:3 %g0::pin}, has a @WIRE
* }
.ends ota5t
.end
```

- `* ` leitet einen SPICE-Kommentar ein. `@RNL {…}` ist ein Eintrag; innerhalb der geschweiften Klammern stehen durch `;` getrennte Mitglieder (Kapitel 2). Der Kopfeintrag am Anfang der Datei deklariert die Sprachversion (16.1).
- `#M1`, `&vss` und `#M5:3` verweisen auf eine Instanz, ein Netz und einen Anschluss des Hosts. `%dp1` und `%g0` sind RNL-Namen dieser Ebene. `$DIFFERENTIAL_PAIR_N` und `$GROUND` sind allgemeine Definitionen (9.4) aus dem Standardpaket, das der Kopfeintrag verwendet (16.3). `%g0` ist die Anwendung, die durch das Anwenden von `$GROUND` entsteht; das Standardpaket weist ihr ein Massesymbol zu, sodass sie als Masse gezeichnet wird (Kapitel 12).
- `@AS`, `@ALIGN`, `@PLACE`, `@ORIENT`, `@ATTACH` und `@WIRE` sind Schlüsselwörter. Parameter stehen in Klammern unmittelbar am Schlüsselwort und enthalten ausschließlich Literale. AS füllt die Rollen der danach stehenden Definition mit den davor stehenden Objekten; ATTACH fügt die davor stehende Definition den danach stehenden Objekten hinzu (9.4, Kapitel 12).
- `(left)` markiert eine Rolle. `%dp1 =` benennt das Ergebnis einer Anweisung. `=>` übergibt das Ergebnis eines Segments an das nächste; folgt danach nur ein Name, benennt es das Ergebnis des vorherigen Segments (5.5).
- `and`, `treated`, `, i.e., same y`, `is at`, `it`, `is … to`, `net`, `, through`, `, has a` sind Freitext und ändern die Bedeutung nicht (3.3). Die Standardsätze fügen solche Wörter zwischen den Tokens ein, damit Anweisungen wie Sätze lesbar sind (4.5, Anhang B). Ohne sie lautet der Eintrag:

```rnl
%dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
{#M1 #M2} @ALIGN("y");
#M5 @PLACE(20, 6) => @ORIENT("R0");
%g0 = $GROUND @ATTACH &vss;
&vss {#M5:3 %g0::pin} @WIRE
```

### 1.4 Begriffe und Konventionen

#### 1.4.1 Anforderungswörter

- **muss**, **darf nicht**: eine Anforderung an Dokumente oder Implementierungen.
- **sollte**, **sollte nicht**: eine Empfehlung; es kann Gründe geben, anders zu verfahren, die zuvor verstanden sein sollten.
- **darf**: etwas Zulässiges.

Die Bedeutung der Sprache selbst wird im Präsens angegeben: „die Anweisung schlägt fehl“, „die spätere gewinnt“.

#### 1.4.2 Fehlschläge und Diagnosemeldungen

- Ein Element (ein Feld, eine Anweisung, ein Eintrag oder eine Datei; 7.6), das **fehlschlägt**, hat keine Wirkung; eine fehlgeschlagene Anweisung hat ein fehlgeschlagenes Objekt als Wert (7.2). Jeder Fehlschlag ist ein **Fehler**, unabhängig von seiner Ursache (7.6).
- Eine **Warnung** kennzeichnet ein Element, das gültig ist, aber möglicherweise nicht dem entspricht, was der Verfasser beabsichtigt hat; es wird wie üblich wirksam, sofern dieser Standard nichts anderes bestimmt, wie bei einem Fakt, über den der Solver innerhalb seines Zeitlimits nicht entscheiden konnte (8.5). Ein **Verstoß** meldet, dass eine Aussage der Datei über die Schaltung, etwa eine Zusicherung, nicht gilt; nichts schlägt fehl (15.3, 15.4). Ein **Hinweis** kennzeichnet eine häufige Überschreibung, Verdeckung oder Auflösung, die ebenfalls ein Versehen sein kann. Die Fälle sind in 7.6 aufgeführt; wo dieser Standard angibt, dass etwas eine Warnung oder einen Hinweis liefert, sollten Implementierungen dies melden.
- Wenn **die Interpretation fehlschlägt**, wird nichts gezeichnet und ein Fehler gemeldet (1.5 Punkt 3, 7.6).

#### 1.4.3 Notation

- Im Fließtext werden Schlüsselwörter in Großbuchstaben ohne `@` geschrieben: PLACE, SET-SYMBOL. In Syntax und Beispielen werden sie mit `@` geschrieben.
- Standardwörter, Namen und Literale werden in Schreibmaschinenschrift geschrieben: `priority`, `"high"`, `instance`, `$GROUND`.
- Abschnittsnummern in Klammern verweisen auf diesen Standard: (9.4). „Prinzipien 2.4“ verweist auf Abschnitt 2.4 der RNL-Prinzipien, `rich-netlist.md`.
- Beispiele sind in der Standardform (4.3) und ohne Freitext geschrieben, außer wo Freitext selbst Thema ist. Ein als Fehler gekennzeichnetes Beispiel zeigt einen Fehler.
- Beispiele verwenden das Standardpaket: Ein Beispiel ohne Kopfeintrag wird so gelesen, als stünde in seinem Kopfeintrag der Hauptdatei `@USE-PACKAGE(name="rnl-std-all", version="0.4")` (16.3).

#### 1.4.4 Begriffe

| Begriff | Englisch | Bedeutung |
|---|---|---|
| Alias | alias | Eine andere Schreibweise eines Standardworts oder eines `$`-Namens sowie der Rollen-, Parameter- und lokalen Namen einer Definition, deklariert mit ALIAS oder geladen mit USE-ALIASES (16.5) |
| Alias-Paket | alias package | Eine Paketdatei, die ausschließlich Aliase deklariert; geladen mit USE-ALIASES (2.7, 16.5) |
| Anmerkung | annotation | Eine Grafik oder ein Text, der in die Szene statt an ein Objekt gezeichnet wird (13.4) |
| anwenden, Anwendung | apply, application | Was AS und ATTACH tun: die Rollen einer Definition mit Objekten füllen. Jedes Anwenden erzeugt eine Anwendung (9.4) |
| anhängen | attach | Ein TEXT wird an den Punkt oder das Objekt angehängt, das er erhält: Er kann die Attribute des Objekts lesen und wird an oder neben dem Objekt platziert, sofern keine Position angegeben ist (13.3). Nicht zu verwechseln mit dem Schlüsselwort ATTACH, das eine allgemeine Definition anwendet (9.4) |
| Rumpf | body | Die Anweisungen in den geschweiften Klammern einer Definition oder eines Inline-Pakets (9.1, 16.2) |
| Bündel | bundle | Ein mit BUNDLE erzeugtes Objekt: mehrere Host-Instanzen, die gemeinsam an einer Stelle gezeichnet werden. Seine Anschlüsse stehen für die gleichnamigen Anschlüsse aller Mitglieder (11.8) |
| Bus | bus | Eine Leitung, deren Netz-Slot eine Menge von Netzen erhält, auch eine einelementige Menge; die Netzliste kennt keine Busse, ein Bus ist lediglich eine Darstellungsform (13.1) |
| Träger | carrier | Ein Ganzzeilenkommentar des Hosts, der RNL trägt: das Kommentarzeichen (`*`, oder `//` im Spectre-Sprachmodus) in Spalte 1, gefolgt von nichts, nur Leerraum oder genau einem Leerzeichen und dem Inhalt (2.2) |
| festschreiben | commit | Was ein Eintrag mit seinen Anweisungen tut: Jede Anweisung eines Eintrags wird Teil seiner Ebene, ob benannt oder nicht (5.4) |
| Bedingung | constraint | Eine Vorgabe für Positionen oder Ausrichtungen für den Solver: ALIGN, ORDER, DISTRIBUTE, DISTANCE, ORIENT-RELATION, TABLE (Kapitel 14) |
| Definition | definition | Was DEFINE, DEFINE-SYMBOL oder DEFINE-STYLE erzeugt, üblicherweise mit `$` benannt (Kapitel 9). Was DEFINE erzeugt, ist eine **allgemeine Definition**; Symbole (DEFINE-SYMBOL) und Stile (DEFINE-STYLE) sind **spezifische Definitionen**. Dieser Text verwendet „Definition“ für eine allgemeine Definition überall dort, wo Rollen, Parameter oder Anwendungen die Art eindeutig machen, und „allgemeine Definition“, wo sie von Symbolen und Stilen unterschieden werden muss |
| Fakt | fact | Eine von einer Anweisung getragene Aussage, etwa „M1 liegt bei (10, 20)“ (5.1) |
| fehlgeschlagenes Objekt | failed object | Der Wert einer fehlgeschlagenen Anweisung (7.2) |
| Ergänzung | fill | Die Massen, Versorgungen und Netzbeschriftungen, die Regeln des Standardpakets für Anschlüsse und Netze hinzufügen, die nicht anderweitig gezeichnet sind (10.5, 16.3) |
| Freitext | free text | Text zwischen den Tokens einer Anweisung; er ändert die Bedeutung nicht (3.3) |
| generierte Menge | generated set | Die Menge, die eine Generierungsregel erzeugt; auf ihre einzelnen Mitglieder kann nicht verwiesen werden (10.5) |
| Gruppe | group | Eine mit GROUP markierte Menge: kein eigenes Objekt; der Solver ordnet ihre Mitglieder gemeinsam an und hält andere Objekte aus ihrem Bereich heraus, als Kriterien nach allen Fakten (11.5) |
| Handle | handle | Das Ding, worauf sich ein Fakt bezieht: eine Instanz, ein Netz oder ein Anschluss des Hosts oder ein von RNL erzeugter Punkt, eine Leitung, eine Menge oder ein anderes Objekt. Die Fakten über ein Handle werden gemeinsam wirksam (5.1, 5.7) |
| Kopfeintrag | header record | Ein Eintrag mit `type="header"` am Anfang einer Datei: höchstens einer ohne `language` und höchstens einer für jede Sprache (2.8) |
| Verdeckung | hiding | Eine Definition einer unteren Ebene oder eines späteren Pakets, die anstelle einer gleichnamigen von oben verwendet wird (6.8, 16.2) |
| Host, Host-Netzliste | host, host netlist | Die Netzliste, die RNL enthält. Der Host-Adapter liest sie und stellt RNL ihre Kommentare und Objekte bereit (2.1) |
| Hausstil | house style | Die gesamte Darstellungsweise, die ein Paket oder eine Implementierung festlegt: welche Symbole, Stile und Layout-Definitionen verwendet werden, ob Verbindungen ohne Leitungen durch Beschriftungen oder Luftlinien gezeigt werden usw. Ein Paket, das einen solchen festlegt, ist ein Hausstilpaket. Er ist kein Stil im Sinne von DEFINE-STYLE |
| Element | item | Worauf sich ein Fehlschlag bezieht: ein Feld eines Eintragskopfs, eine Anweisung, ein Eintrag, eine Datei oder die gesamte Interpretation (7.6) |
| Instanz | instance | Ein Bauelement der Host-Netzliste, einschließlich Teilschaltungsinstanzen (X-Zeilen). Eine iterierte Instanz wie `XI<3:0>` steht für mehrere Instanzen (6.11) |
| key | key | Der Name eines Texts an dem Objekt, an das er angehängt ist: ein TEXT mit `key="wl"` an M1 ist `#M1::wl` (13.3) |
| Schlüsselwort | keyword | `@` gefolgt von einem Wort in Großbuchstaben wie `@PLACE`, oder ein deklarierter Alias (16.5); höchstens eines je Segment (4.1, 5.3) |
| Beschriftung | label | Ein an ein Objekt angehängter TEXT; er kann auf Attribute des Objekts wie dessen Namen oder Wert verweisen (13.3) |
| Ebene | level | Ein Textbereich: die Dateiebene der Hauptdatei (die oberste Ebene der Schaltung), die Dateiebene einer eingebundenen Datei oder eine Teilschaltungsdefinition; jedes Paket ist ebenfalls eine Ebene. Ebenen bilden durch textuelle Enthaltung einen Baum (2.6, 6.1) |
| lokales Objekt | local object | Ein benanntes Objekt im Rumpf einer allgemeinen Definition; jede Anwendung hat ihr eigenes, das von außen als `%c1::mid` angesprochen wird (9.4) |
| Hauptobjekt | main object | Das Objekt, das die erste Rolle einer Anwendung füllt; Texte verweisen über die Rolle darauf (13.6). Eine Anwendung wird an ihrem eigenen Punkt gezeichnet, nicht an ihrem Hauptobjekt (12.1) |
| Mitglied, Segment, Anweisung | member, segment, statement | Ein Mitglied ist ein durch `;` abgetrennter Teil einer Menge. Ein Segment ist ein durch `=>` abgetrennter Teil eines Mitglieds (ein Name allein nach `=>` ist kein Segment, 4.2). Eine Anweisung ist ein Segment mit einem Schlüsselwort oder ein Objekt in einem Segment ohne Schlüsselwort (4.2, 5.3) |
| Name | name | `%x =`, `$X =`, `=> %x` oder `=> $X` gibt einem Objekt einen Namen; ein Name erzeugt kein Objekt und wird in einer Ebene einmal gebunden (6.6) |
| eigener Punkt | own point | Der Punkt einer Anwendung: der Ursprung ihres Symbols oder ein freier Punkt, den der Rumpf ihrer Definition an ihre Mitglieder bindet; geschrieben `%u::position`. Eine Bedingung nimmt ihn bei einer Anwendung mit Symbol an, PLACE bei jeder Anwendung (14.1, 6.5) |
| Paket | package | Eine Menge von Definitionen und Regeln, die mit USE-PACKAGE über eine Ebene gestellt wird; geschrieben als Paketdatei (2.7) oder inline im Rumpf von USE-PACKAGE (16.2). Ein Paket, das ausschließlich auf andere Pakete verweist, ist eine **Paketsammlung (package collection)**, wie `rnl-std-all` (16.3) |
| Pakete an der Wurzel | packages at the root | Pakete vor allen Ebenen: die Pakete von USE-PACKAGE in Kopfeinträgen, mit dem Standardpaket an erster Stelle, wenn es dort angegeben oder durch eine Werkzeugeinstellung hinzugefügt wird (16.3) |
| Pin | pin | Ein Objekt eines Symbols: ein Punkt, an dem Leitungen anschließen und der Anschlüsse des mit dem Symbol gezeichneten Objekts, Rollen einer Anwendung oder nichts bindet (9.2.4) |
| Priorität | priority | Der Rang eines geometrischen Fakts: `"disabled"`, `"low"`, `"normal"` oder `"high"`. Bei einem Konflikt wird zuerst die Priorität verglichen, dann die Stelle im Text (8.5, 14.8) |
| Eintrag | record | Ein `@RNL(…) {…}` in den Kommentaren. Die Felder in seinen Klammern bilden den Eintragskopf und gelten für jede Anweisung des Eintrags (2.3, 2.8) |
| RNL-Objekt | RNL object | Was eine Anweisung mit Schlüsselwort oder ein `{…}` erzeugt: ein Handle und Fakten (5.1) |
| Rolle | role | Der Slot, den ein Objekt in einer Anweisung füllt, in Klammern unmittelbar nach dem Objekt markiert, wie `#M1(in_p)`. Die Rollen einer Definition stehen in ihrer Rollenliste und heißen im Rumpf `?name`; die Rollen eines eingebauten Schlüsselworts sind seine Slot-Namen (5.3, 9.4) |
| Regel | rule | Eine Anweisung, die das Ergebnis einer Auswahl erhält. Eine in einer Ebene geschriebene Regel wird in dieser Ebene ausgewertet, und mit `recursive=true` erneut in jeder Ebene darunter, jeweils ausschließlich mit den Objekten jener Ebene (10.4) |
| Szene | scene | Die Objekte einer Ebene aus dem Host und aus RNL mit den wirksam gewordenen Fakten (Kapitel 8) |
| Auswahl | selection | Das Ergebnis eines SELECT-…-Schlüsselworts: die Objekte einer Art in seinem Bereich, die seine Bedingungen erfüllen, ausgewertet bei Verwendung (Kapitel 10) |
| Menge | set | Eine geordnete Sammlung von Objekten, geschrieben `{…}` oder durch eine Auswahl ermittelt (5.2) |
| Slot | slot | Eine der Stellen, an denen ein Schlüsselwort Objekte annimmt, etwa `from` und `to` von DISTANCE. Jeder Slot hat einen Namen, der als Rolle markiert werden kann (5.2.1, 5.3) |
| Solver | solver | Das Werkzeug, das Positionen und Ausrichtungen aus RNL berechnet und in einen generierten Eintrag zurückschreibt (14.8, Kapitel 17) |
| Standardform | standard form | Die eine Schreibweise jedes Schlüsselworts, die Werkzeuge ausgeben: wo das Schlüsselwort steht, welche Parameter nach Position übergeben werden, ob Rollen markiert werden (4.3, 5.3) |
| Standardpaket | standard package | Die Paketsammlung `rnl-std-all` aus Definitionen, Symbolen, Stilen und Regeln, die eine Datei durch Angabe in ihrem Kopfeintrag verwendet oder die ein Werkzeug über eine Einstellung hinzufügt (16.3) |
| Standardsatz | standard sentence | Eine Anweisung in der Standardform mit dazwischen eingefügtem Freitext, sodass sie sich wie ein Satz liest (4.5, Anhang B) |
| Standardwort | standard word | Ein durch die Wortliste definiertes Wort: Schlüsselwörter, Slot-Namen, Parameternamen und -werte, Felder des Eintragskopfs, Arten, Unterobjektnamen, Attributnamen, Stileigenschaften, `true` und `false` (5.3) |
| Stil | style | Ausschließlich die mit DEFINE-STYLE definierten Eigenschaften: Farben, Linienart, Linienbreite, Textgröße usw. (9.3). Layout-Präferenzen sind keine Stile |
| Subskript | subscript | `<…>` oder `[…]` unmittelbar nach einem Host-Namen oder Anschlussnamen, wie `&D<3>` oder `&D<7:0>`, stellvertretend für einen oder mehrere Namen (6.11); nicht die Tiefstellungen der Textauszeichnung (13.6) |
| Unterobjekt | sub-object | Ein über `::` angesprochener Teil eines Objekts, etwa `#M1::position` oder `%dp::left` (6.5) |
| Symbol | symbol | Was DEFINE-SYMBOL erzeugt: Grafiken, Pins und Texte, gezeichnet für eine Instanz, ein Bündel oder eine Anwendung (9.2) |
| wirksam werden | take effect | Nachdem eine Ebene vollständig eingelesen wurde, werden ihre Fakten phasenweise zur Szene (Kapitel 8) |
| Token | token | Ein Schlüsselwort, ein Verweis, ein Punkt, Klammern, geschweifte Klammern, `;`, `=>` oder `=`; alles andere ist Freitext oder Kommentar (3.2) |
| Anschluss | terminal | Eine Verbindung einer Instanz wie in ihrer Netzlistenzeile: `#M1:d`, oder `#M1:1` nach Nummer. `#:inp` ist der Port `inp` der Ebene (6.4) |
| Anwendung | usage | Das Objekt, das beim einmaligen Anwenden entsteht. Es hält die Definition fest und welche Objekte ihre Rollen füllen; wenn es ein Symbol hat, wird es gezeichnet (9.4, 9.5). In einer Bedingung steht eine Anwendung ohne Symbol für ihre Mitglieder (14.1) |
| virtuelles Bauteil | virtual part | Eine Anwendung mit Symbol: gezeichnet, aber kein Bestandteil der Netzliste, wie Masse, Versorgung, Port-Markierung oder Knotenpunkt (Kapitel 12) |
| Wortliste | word list | Die maschinenlesbare Liste aller Standardwörter und Schlüsselwortsignaturen einer Sprachversion, `rnl-words-0.4.json` für diese Version (5.3, Anhang A) |

### 1.5 Kernprinzipien

Die Regeln dieses Standards folgen diesen Prinzipien. Die Punkte 1–6 präzisieren Prinzipien 2.9; Punkt 7 legt fest, wie das Vokabular aufgeteilt ist.

1. **Alles ist eine Beschreibung.** RNL-Text ist eine Menge von Beschreibungen. Der Interpreter sammelt alle Beschreibungen ein, bevor er ein Ergebnis liefert. Nichts wird ausgeführt, und ein Name darf verwendet werden, noch bevor die Anweisung steht, die ihn bindet.
2. **Die Stelle im Text ist nur Ordnungsdatum.** Wo eine Beschreibung geschrieben steht (ihre Ebene und ihre Stelle im Text), zählt nur in drei Fällen: wenn Beschreibungen einander widersprechen, bleibt die spätere erhalten (geometrische Fakten vergleichen zuerst die Priorität; 8.3, 8.4, 8.5); bei der Zeichenreihenfolge (11.6); und bei der Reihenfolge der Mitglieder einer Menge (5.2). Andernfalls hängt das Ergebnis nicht von der Schreibreihenfolge ab. Jedes Segment einer Pipe hat einen eigenen Platz: Ein späteres Segment gilt als später geschrieben als ein früheres (5.5).
3. **Widersprüche.**
   - Eine Beschreibung, die nicht verstanden werden kann (ein Syntaxfehler, ein nicht gefundener Name, eine falsche Art usw.) oder sich selbst widerspricht, schlägt fehl (Prinzipien 2.4, 2.9; Kapitel 7).
   - Verständliche Beschreibungen, die einander widersprechen, werden dort aufgelöst, wo dieser Standard ein eindeutiges Verfahren vorgibt, etwa Überschreibung nach Schlüssel (8.4), die Darstellungswarteschlangen (8.3) und die Auflösung geometrischer Fakten (8.5); dabei kann ein Hinweis gegeben werden.
   - Wo noch kein eindeutiges Verfahren festgelegt ist, schlägt die gesamte Interpretation fehl (7.6). In dieser Version gibt es keinen solchen Fall (7.5). Eine spätere Version kann ein Verfahren festlegen und solche Fälle tolerant machen; bestehende funktionierende Dateien bleiben davon unberührt.
4. **Jede Beschreibung sieht den Zustand vor ihrem Wirksamwerden.** Eine Beschreibung liest den Zustand zu Beginn ihrer Phase; ihre eigene Wirkung und die Wirkungen anderer Beschreibungen derselben Phase sind nicht enthalten (8.2).
5. **Objekte und Namen.** Jede Anweisung mit einem Schlüsselwort und jedes `{…}` erzeugt ein neues Objekt aus einem Handle und Fakten. Ein Verweis erzeugt kein Objekt, außer dass das Herausgreifen von Mitgliedern über ihre Positionen eine neue Menge erzeugt (5.2.3). Ein Name verweist lediglich auf ein Objekt und wird in einer Ebene nur einmal gebunden. Überschreiben gilt für Fakten, nicht für Namen (5.1, 6.6).
6. **Keine Einschränkungen zugunsten der Lesbarkeit.** Eine schwer lesbare Form mit eindeutiger Bedeutung ist zulässig; es bleibt den Verfassern überlassen, sich selbst zu mäßigen. Nur Formen ohne eindeutige Bedeutung sind Fehler.
7. **Schlüsselwörter und Definitionen.** Was eine Implementierung selbst verstehen muss und was nicht mit anderen Beschreibungen ausgedrückt werden kann, ist ein Schlüsselwort. Was sich mit bestehenden Beschreibungen im Rumpf einer allgemeinen Definition formulieren lässt, ist eine Definition, und ihr Name ist lediglich ein Hinweis (9.5).

### 1.6 Konformität

**Dokumente.** Eine Host-Netzliste ist konform zu RNL 0.4, wenn ihre Einträge, gelesen nach Kapitel 2–4 und interpretiert nach Kapitel 5–17, keine Fehler liefern (7.6), einschließlich Fehlern im Inhalt der verwendeten Pakete. Warnungen, Hinweise und Verstöße beeinträchtigen die Konformität nicht: Ein Verstoß besagt, dass die Schaltung nicht bestätigt, was die Datei aussagt, nicht dass die Datei fehlerhaft geschrieben ist. Eine Paketdatei ist in gleicher Weise konform, wenn sie für sich allein geprüft wird (16.2). Eine Datei, deren Kopfeintrag eine andere Sprachversion deklariert, unterliegt nicht dieser Version (16.1).

**Implementierungen** sind Interpreter (ein ausschließlich zur Prüfung verwendeter Interpreter wird auch Lint genannt), Viewer, Solver, Editoren und andere Werkzeuge, die RNL lesen oder schreiben. Eine konforme Implementierung:

1. entnimmt RNL den Kommentaren des Hosts gemäß Kapitel 2 und liest es gemäß Kapitel 3 und 4;
2. gibt jeder Beschreibung die in Kapitel 5–17 definierte Wirkung mit den Signaturen der Wortliste (5.3);
3. meldet jeden Fehler mit Ort und Ursache (7.6); Fehler dürfen nicht herabgestuft oder abgeschaltet werden;
4. sollte die in diesem Standard aufgeführten Warnungen und Hinweise melden und muss Verstöße melden (7.6);
5. darf eigene Prüfungen ausschließlich als Warnungen oder Hinweise hinzufügen: Sie darf kein Element fehlschlagen lassen, das dieser Standard wirksam werden lässt, und kein Element wirksam werden lassen, das dieser Standard fehlschlagen lässt;
6. darf einen strengeren Modus anbieten, der anhält oder die gesamte Eingabe zurückweist, jedoch nicht als Voreinstellung;
7. lässt den Benutzer die Diagnosemeldungen einsehen; die Art der Darstellung bleibt der Implementierung überlassen (7.6);
8. deklariert die von ihr unterstützten Sprachversionen und behandelt eine Datei einer nicht unterstützten Version gemäß 16.1;
9. behandelt unbekannte Schlüsselwörter, Parameter und Werte als Fehler (16.4);
10. stellt das Standardpaket an die Wurzel, wenn eine Datei es angibt, wie in 16.3 definiert;
11. schreibt beim Schreiben von RNL neue Anweisungen in der Standardform und ordnet von Benutzern geschriebene Anweisungen nicht um (4.3); schreibt den Kopfeintrag und die Version beim Speichern einer Datei (16.1); behält unbekannte und fehlgeschlagene Anweisungen, den Host-Text, gewöhnliche Kommentare und Dateien nicht unterstützter Versionen unverändert bei (16.4); und ändert ausschließlich die Einträge, die Kapitel 17 zu ändern erlaubt.
12. löst beim Lösen Konflikte gemäß 8.5 auf, schreibt jede getroffene Entscheidung zurück und gibt ihre Ergebnisse an andere Werkzeuge nur in einer Form weiter, die ein Eintrag des Solvers aufnehmen kann (14.8);
13. darf Grenzwerte für die genutzten Ressourcen festlegen, etwa für die Schachtelungstiefe von Symbolen (9.2.7), von Expansionen von Anwendungen (9.4), von eingebundenen Dateien und Teilschaltungen oder für die Anzahl von Objekten. Eine Eingabe jenseits einer solchen Grenze liefert einen Fehler, der angibt, dass eine Grenze der Implementierung (limit of the implementation) erreicht wurde, samt Ortsangabe; die Implementierung darf die Eingabe nicht stillschweigend abschneiden und kein anderes als das in diesem Standard definierte Ergebnis zeichnen oder lösen. Sie unterstützt mindestens 64 Ebenen der Symbolschachtelung. Das Limit für die Faktenprüfung während eines Lösevorgangs ist keine solche Grenze: Es ist eine Option des Solvers, und ein unentschieden gebliebener Fakt wird mit einer Warnung verworfen (8.5).

Dieser Standard definiert weder, wie ein Solver unter Positionen wählt, die die Bedingungen erfüllen, noch wie ein Viewer das zeichnet, was RNL offenlässt; die Stellen, an denen dies ihnen überlassen bleibt, sind im Text genannt (etwa 13.1).

**Konformitätstests** prüfen die Fehler samt ihren Ortsangaben und dem Umfang jedes Fehlschlags sowie die entstehende Szene. Warnungen dürfen als optionale Erwartungen geprüft werden; Hinweise werden nicht geprüft (7.6).

<!-- translated from en/ch02.md sha256:e9821c4aa800 -->

## 2 Einbettung in den Host

### 2.1 Zweistufiges Parsen

- Der Host-Adapter:
  - erkennt die gültigen gewöhnlichen Kommentare des Hosts und entscheidet, welche Zeilen Trägerzeilen sind (2.2);
  - bestimmt, zu welcher Ebene jeder Kommentar gehört und aus welcher Datei sowie welcher Einbindung jeder Eintrag und jede Teilschaltung stammen (6.1);
  - liefert Quellpositionen und die Textreihenfolge;
  - stellt die Host-Objekte bereit: Instanzen, Netze, Anschlüsse, Teilschaltungsports und Modelltypen.
- Der RNL-Kern sucht ausschließlich in diesen Kommentaren nach Einträgen. Er darf nicht die gesamte Netzliste ohne Rücksicht auf den Kontext nach `@RNL` durchsuchen: dieselben Zeichen in Host-Anweisungen oder Host-Zeichenketten sind kein RNL.
- Der Interpreter parst die gesamte Host-Netzliste, bevor er das RNL verarbeitet (Prinzipien 2.9).

### 2.2 Träger

Einträge werden in Ganzzeilenkommentaren des Hosts geschrieben; solche Kommentarzeilen sind Trägerzeilen. Der Host-Adapter entscheidet Zeile für Zeile, welche Zeilen Trägerzeilen sind, entfernt das Kommentarzeichen und das eine Leerzeichen danach und übergibt den Rest an den RNL-Kern (2.1).

- **Nur Ganzzeilenkommentare.** Kommentare am Zeilenende (`$`, `;`, `//`, `!` usw.) werden weder gelesen noch geschrieben, und Blockkommentare werden nicht verwendet.
  - `@RNL` in einem Kommentar am Zeilenende ist lediglich Host-Text und liefert eine Warnung.
  - Hinweis: Ob ein Zeilenende ein Kommentar ist, hängt vom Dialekt, vom Modus und von Optionen ab: in HSPICE darf `$` direkt auf eine Zahl folgen (`1k$x`), im `ps`-Modus von ngspice ist `$` ein gewöhnliches Zeichen, und Eldo kann es mit `.COMCHAR` ändern; in CDL sind `$SUB=` und dergleichen Daten. Das Schreiben am Zeilenende würde zudem die Zeile einer Host-Anweisung verändern (Prinzipien 2.1). SPICE und Spectre kennen keine Blockkommentare (Spectre nur innerhalb von `reliability{}`).
- **Kommentarzeichen der Hosts.**

  | Host | Kommentarzeichen |
  |---|---|
  | Die SPICE-Familie: SPICE3, ngspice, HSPICE, Xyce, LTspice, PSpice, WRspice, Eldo sowie CDL-Netzlisten | `*` |
  | Abschnitte im Spectre-Sprachmodus: `.scs`-Dateien oder Text nach `simulator lang=spectre` | `//` oder `*` |
  | Abschnitte im SPICE-Sprachmodus von Spectre, einschließlich eingebundener Dateien ohne `.scs`-Endung, die nicht mit `simulator lang=spectre` beginnen | `*` |

  - `*` ist in beiden Sprachmodi von Spectre ein Kommentar. `//` ist nur für den Spectre-Sprachmodus dokumentiert, nicht jedoch bei Eldo oder CDL.
  - Die Zeilen eines Eintrags dürfen `//` und `*` mischen. Ein Werkzeug, das einen Eintrag schreibt, muss den bereits in der Datei vorhandenen Einträgen folgen; gibt es noch keine, schreibt es `*` im SPICE-Modus und `//` im Spectre-Modus.
  - Die Träger gehören zur äußeren Form, die sich nicht ändert (16.1). Die Unterstützung eines neuen Hosts fügt lediglich Träger hinzu; bestehende werden nicht geändert.
- **Trägerzeilen.** Jede Zeile eines Eintrags muss beide der folgenden Bedingungen erfüllen:
  - Das Kommentarzeichen steht in Spalte 1, ohne Leerraum davor.
  - Nach dem Kommentarzeichen folgt entweder nichts, nur Leerraum, was eine leere Kommentarzeile ergibt, oder genau ein ASCII-Leerzeichen gefolgt vom Inhalt. Daher sind `*@RNL`, `*#M1` und `//@RNL` keine Trägerzeilen.

    Hinweis: HSPICE und PSpice verlangen `*` in Spalte 1, und Werkzeuge behandeln eingerückte Zeilen unterschiedlich. Direkt an das Kommentarzeichen angefügter Text hat für andere Werkzeuge eine eigene Bedeutung: ngspice und WRspice führen `*#`-Zeilen als Befehle aus, WRspice auch `*@`-Zeilen; `*.` leitet eine CDL-Direktive ein (etwa `*.PININFO`), `*|` beginnt DSPF-Daten, und `//pragma` ist der Verschlüsselungsumschlag von Spectre.
  - Eine Kommentarzeile, die diese Bedingungen nicht erfüllt (etwa `*#M1 …`, `** …` oder ein eingerücktes `* …`), bricht einen noch nicht abgeschlossenen Eintrag ab: der gesamte Eintrag schlägt fehl (2.4). Außerhalb eines Eintrags ist `@RNL` in einer solchen Zeile kein Einstieg und liefert eine Warnung.
  - Eine Zeile, die RNL enthält, sollte nicht auf `\` enden. Eine solche Zeile liefert eine Warnung, und Werkzeuge dürfen keine solche schreiben. Das RNL wird wie üblich gelesen und nichts schlägt fehl; gefährdet ist die Host-Netzliste. Hinweis: WRspice hängt die nächste Zeile an einen solchen Kommentar an, ebenso möglicherweise Spectre beim Aufruf mit `-E` (über den C-Präprozessor) und Eldo mit `SLASHCONT`; eine Host-Anweisung in der nächsten Zeile würde dann vom Kommentar geschluckt.
- **Stellen, an denen RNL nicht gelesen wird.** `@RNL` an diesen Stellen liefert eine Warnung, und Werkzeuge dürfen dort nicht schreiben:
  - Zeile 1 jeder Host-Datei sowie weitere Zeilen, die der Host als Titel auffasst (etwa die Zeile nach `.newjob` in WRspice). Zeile 1 der Hauptdatei ist ihr Titel, beim Einbinden ist sie jedoch kein Titel; sie wird nie gelesen, damit sich eine Datei als Hauptdatei und als eingebundene Datei gleich liest. Daher kann der Kopfeintrag frühestens in Zeile 2 stehen (2.8).
  - Text nach dem `.end` der Hauptdatei. Hinweis: Nach `.end` liest ngspice weiter, Xyce stoppt, und HSPICE sowie PSpice können den Text als nächste Schaltung auffassen.
  - Bereiche, die der Host nicht als Netzlistenkommentare behandelt: Steuerskripte (ngspice führt `*#`-Zeilen innerhalb von `.control … .endc` aus), Blöcke und Dateien anderer Sprachen (`.verilog` in WRspice, `reliability{}` in Spectre; Verilog-A-Dateien, die über `ahdl_include` oder `.hdl` eingebracht werden, werden nur wie in 2.9 beschrieben gelesen), verschlüsselter Text, durch Vorverarbeitung entfernte Teile sowie nicht ausgewählte `.lib`-Abschnitte. Ein Bereich, den der Adapter nicht mit Sicherheit einordnen kann, wird als nicht gelesen behandelt, mit einer Warnung.
- **Stellen, die mit einer Warnung gelesen werden:**
  - Kommentarzeilen zwischen einer Anweisung und ihren Fortsetzungszeilen (`+`-Zeilen). Ihre Bedeutung für RNL ist eindeutig, daher werden sie gelesen; Hosts behandeln sie jedoch unterschiedlich (PSpice meldet einen Fehler, SPICE 3f4 hängt die folgende `+`-Zeile an den Kommentar an). Sie liefern eine Warnung, und Werkzeuge dürfen dort nicht schreiben.
  - Unverschlüsselt geschriebene geschützte Bereiche (`.prot … .unprot`, `protect … unprotect`). Sie werden mit einer Warnung gelesen: sobald der Bereich verschlüsselt wird, gehen die Einträge verloren.
- **Die Sprache, in der eine Datei beginnt,** ist eine Eingabe an das Werkzeug. Standardmäßig entscheidet die Dateiendung: `.scs` für den Spectre-Modus und jede andere Datei für den SPICE-Modus, wie Spectre selbst es tut; der Benutzer kann dies für jede Datei ändern, auch für eingebundene Dateien. Nach dem Anfang folgt jede Zeile den obigen Regeln (`simulator lang=`). Welche Zeilen im SPICE-Modus und welche im Spectre-Modus stehen, entscheidet darüber, worauf Einträge mit `language` wirken (2.8), sodass ein Werkzeug jede Datei in der vom Benutzer gewählten Sprache liest; wie ein Werkzeug dem Benutzer diese Wahl ermöglicht, ist nicht Teil dieses Standards.
- **Empfehlungen zum Schreiben:**
  - RNL begrenzt die Zeilenlänge nicht (2.4). Generatoren sollten Zeilen an Leerraum umbrechen, mit höchstens 132 Zeichen pro Zeile: dem Limit älterer PSpice-Versionen, dem strengsten unter den gängigen Hosts (HSPICE erlaubt 1024, Eldo 2000). Ein Adapter, der das Limit seines Hosts kennt, folgt dem Host. Passt ein einzelnes Token nicht in eine Zeile, muss das Werkzeug einen Fehler melden und darf es nicht schreiben (Zeichenketten können nicht auf der nächsten Zeile fortgesetzt werden, 3.7).
  - Zur Zeichenkodierung siehe 3.1.

### 2.3 Der Einstieg

- Ein Eintrag beginnt mit `@RNL`. Der Einstieg wird nur in Großbuchstaben erkannt.
- Alle Hosts verwenden denselben Einstieg `@RNL`; er ist das Sub-Tag aus Prinzipien 2.1.
- `@RNL` darf mitten in einem Kommentar stehen; der Text davor in derselben Zeile gehört zum Host-Kommentar.
- **Das Leerzeichen vor `@`.** Das Zeichen unmittelbar vor dem `@` des Einstiegs `@RNL` und eines Schlüsselworts muss ein ASCII-Leerzeichen (U+0020) sein; ein TAB, ein Zeilenumbruch oder der Zeilenanfang zählen nicht. In einer Trägerzeile zählt das Leerzeichen nach dem Kommentarzeichen als Leerzeichen vor `@`, wie in `* @RNL`. Ein `@` innerhalb einer Zeichenkette, eines in Anführungszeichen gesetzten Namens oder eines `/* */`-Kommentars ist nicht eingeschränkt.
  - Daher benötigt auch ein Schlüsselwort nach `{`, `=>`, `=` oder `;` ein Leerzeichen davor: `#M1 @SET-SYMBOL { @DEFINE-SYMBOL {…}}`, `%x = @DEFINE(…)`, `#M2 @PLACE(30, 20) => @ORIENT("MY")`.
  - Hinweis: `@` kommt in Host-Kommentaren häufig vor, etwa in E-Mail-Adressen und `@param`; durch diese Regel wird solcher Text nicht als RNL aufgefasst. WRspice führt zudem Zeilen aus, die mit `*@` beginnen (2.2).
  - Außerhalb eines Eintrags ist `x@RNL` kein Einstieg, sondern nur Host-Kommentar. Innerhalb eines Eintrags ist ein `@` ohne Leerzeichen davor weder ein Schlüsselwort noch ein Zeichen des Freitexts: es ist ein Syntaxfehler, und das Mitglied, in dem es steht, schlägt fehl (7.4).

### 2.4 Ausdehnung eines Eintrags

- Ein Eintrag wird `@RNL(…) {…}` geschrieben. Der Eintragskopf in runden Klammern darf weggelassen werden; die geschweiften Klammern nicht, und auch eine einzelne Anweisung benötigt sie. Die Klammer des Eintragskopfs berührt `@RNL`, wie die Parameterklammern eines Schlüsselworts das Schlüsselwort berühren (3.5): `@RNL (type="header") {…}` ist ein Fehler, und der Eintrag schlägt als Ganzes fehl.
- Ein Eintrag reicht von `@RNL` bis zu der `}`, die zu seiner ersten `{` passt. Bei der Suche nach der passenden `}` werden Zeichenketten und `/* */`-Kommentare übersprungen. Runde Klammern im Eintragskopf werden ebenfalls übersprungen; nach der ersten `{` zählt jede geschweifte Klammer, auch eine innerhalb von Parameterklammern, sodass eine offen gebliebene runde Klammer nur ihr Mitglied fehlschlagen lässt (7.4). Eine Zeichenkette kann keine Zeile überschreiten (3.7): wird sie in ihrer Zeile nicht geschlossen, endet sie dort, und der Eintrag ist unvollständig (7.4).
- **Einträge über mehrere Zeilen.** Ein Eintrag darf über mehrere aufeinanderfolgende Kommentarzeilen (Trägerzeilen, 2.2) geschrieben werden. Das Kommentarpräfix jeder Zeile wird entfernt und die Inhalte werden zusammengefügt; ein Zeilenumbruch ist Leerraum (3.5).
  - Eine leere Kommentarzeile mit nur `*` (oder `*` und Leerraum) ist Leerraum, und der Eintrag wird fortgesetzt.
  - Trifft man vor der schließenden `}` auf eine Zeile, die keine Trägerzeile ist — eine Host-Anweisung, eine Zeile wie `.subckt`, `.ends` oder `.include`, eine vollständig leere Zeile oder eine Kommentarzeile, die nicht als Trägerzeile geschrieben ist (2.2) — oder endet die Datei, so ist der Eintrag unvollständig und schlägt als Ganzes fehl. Ein Eintrag überschreitet daher niemals die Grenze einer Teilschaltung oder einer Einbindung. Um innerhalb eines Eintrags eine Zeile leer zu lassen, schreibt man eine Zeile mit nur `*`.
  - Erscheint vor dem Ende des Eintrags ein Einstieg `@RNL` außerhalb von Zeichenketten und Kommentaren, ist der frühere Eintrag unvollständig und schlägt als Ganzes fehl, und das Lesen beginnt erneut beim neuen Einstieg. Eine fehlende `}` verliert somit nur einen Eintrag.
  - Ein unvollständiger Eintrag ist ein Fehler, und die Diagnosemeldung gibt an, dass der Eintrag unvollständig ist und wo er abgeschnitten wurde. Eine einzige Leerzeile zu viel lässt einen ganzen Eintrag fehlschlagen; dies geschieht leicht und darf nicht stillschweigend übergangen werden (7.6).
  - Jede Kommentarzeile innerhalb eines Eintrags ist RNL. Eine Zeile gewöhnlicher Bemerkungen dazwischen, etwa `* the next line is for M2`, wird als Freitext gelesen, und es ist ein Fehler, wenn sie Zeichen enthält, die kein Freitext sein können. Bemerkungen innerhalb eines Eintrags werden in `/* */` geschrieben (3.4).
  - RNL begrenzt die Zeilenlänge nicht; die Grenzen des Hosts sind Sache des Adapters. Eine lange Anweisung darf an Leerraum auf mehrere Zeilen umbrochen werden; Tokens und Zeichenketten dürfen nicht aufgespalten werden (3.5). Empfehlungen für Generatoren stehen in 2.2.
- Ein Token in derselben Zeile nach dem Ende eines Eintrags ist ein Fehler: diese Tokens schlagen fehl, und der Eintrag selbst wird wie üblich wirksam. Ein neuer `@RNL` bildet die Ausnahme, sodass eine Zeile mehrere Einträge enthalten darf.
- Die Kommentarträger, der Einstieg `@RNL` und der Eintragskopf hängen nicht von der Sprachversion ab, und spätere Versionen ändern sie nicht (16.1). Die geschweiften Klammern des Rumpfes werden in dieser Version wie oben beschrieben gepaart. Die Felder des Eintragskopfs stehen in 2.8.

### 2.5 Deaktivierte Einträge und Host-Text

- Eine Kommentarzeile außerhalb von Einträgen, die kein `@RNL` enthält, ist ein Host-Kommentar und wird als Ganzes ignoriert.
- **Deaktivierte Einträge.** In einer Kommentarzeile beginnt ein `/*`, vor dem nur Leerraum oder Kommentare stehen oder das unmittelbar auf einen Eintrag folgt, einen Kommentar, der bis `*/` reicht. Ein `@RNL` darin ist kein Eintrag.
- Ein `/*` mitten im Text einer Kommentarzeile, wie in `models/*.lib`, ist lediglich Host-Text.
- **Ende eines Host-Kommentars.** Träger sind Ganzzeilenkommentare (2.2): ein Host-Kommentar endet am Ende seiner Zeile, und nichts innerhalb der Zeile kann ihn früher beenden, sodass das Ende eines Host-Kommentars nicht mit RNL-Text in Konflikt gerät.

### 2.6 Ebenen

- Ebenen werden durch den Text unterteilt; ihre Hierarchie steht in 6.1:
  - Die Dateiebene der Hauptdatei: was in der Hauptdatei außerhalb aller `.subckt … .ends` steht. Sie ist die oberste Ebene der Schaltung.
  - Die Dateiebene jeder eingebundenen Datei. Die Abschnitte von `.lib` werden ebenso behandelt.
  - Jedes `.subckt … .ends`.
  - Die Dateiebene jeder Verilog-A-Datei, die die Netzliste einbringt, und jedes Modul darin (2.9).
- Ein Eintrag gehört zu der Ebene, in der er geschrieben steht (Prinzipien 2.5).
- **Reihenfolge über Dateien hinweg.** Eine eingebundene Datei ist eine eigene Ebene unterhalb der Ebene, die sie einbindet (6.1). Daher beeinflusst die Stelle der Einbindungszeile die Reihenfolge nicht, anders als bei USE-PACKAGE (16.2), und nebeneinander eingebundene Dateien beeinflussen einander nicht. Innerhalb einer Ebene gilt die Reihenfolge des Texts.
- **scope.** Ein Eintrag kann mit `@RNL(scope="name")` eine Zielebene benennen. Der Name ist der Name einer Teilschaltung und bezeichnet die Definition dieser Teilschaltung. In der folgenden Datei steht der letzte Eintrag in der Hauptdatei und wird auf die Teilschaltung `ota` angewendet:

  ```spice
  * A record written outside its subcircuit
  * @RNL(type="header") { @VERSION("0.4")}
  .model nch nmos level=1
  .subckt ota inp inn out tail vss
  M1 x inp tail vss nch
  M2 out inn tail vss nch
  .ends ota
  X1 vip vin vo vt 0 ota
  * @RNL(scope="ota") { #M1 @PLACE(10, 20)}
  .end
  ```

  - Jede Teilschaltung hat nur einen Schaltplan (6.1), daher benennt scope die Definition, nicht eine Instanz.
  - scope wirkt nur nach unten: das Ziel muss eine Teilschaltung sein, die diese Ebene in ihrem Text enthält, also eine in dieser Datei oder in einer direkt bzw. über weitere Einbindungen eingebundenen Datei definierte Teilschaltung. scope wird nicht innerhalb einer Teilschaltung geschrieben: das RNL einer Teilschaltung gilt nur für sie selbst. Ein `scope` in einem Eintrag innerhalb einer Teilschaltung ist ein Fehler und wird als nicht geschrieben behandelt (2.8). Wird das Ziel nicht gefunden oder liegt es nicht unterhalb dieser Ebene, so ist dies ein Fehler, und die Anweisungen des Eintrags, die keine Ebene zur Auflösung haben, schlagen sämtlich fehl (Kapitel 7).
  - Alle Verweise im Eintrag werden in der Zielebene aufgelöst. Seine Anweisungen werden in die Zielebene festgeschrieben, und die Namen, die sie binden, werden dort veröffentlicht und mit den eigenen Einträgen der Zielebene geteilt (6.7); einen Namen an ein anderes Objekt zu binden, den die Zielebene bereits bindet, ist ein Fehler, während erneutes Binden an dasselbe Objekt zulässig ist (6.6). scope betrifft nur seinen eigenen Eintrag und ändert die Host-Netzliste nicht.
  - Reihenfolge: Die Anweisungen, die scope in eine Teilschaltung einbringt, folgen nach deren eigenem Text; zielen mehrere Stellen per scope auf dieselbe Teilschaltung, kommen die äußeren später (8.3, 8.5). Was explizit außen geschrieben steht, überschreibt das innen Geschriebene, genau wie die Priorität einer äußeren Anwendung deren Rumpf überschreibt (14.8).
  - Die Ausgabe des Solvers und Änderungen, die Benutzer in einem Editor vornehmen, dürfen scope nicht verwenden; sie werden in den eigenen Text der Teilschaltung geschrieben (Kapitel 17). scope ist nur für Anweisungen gedacht, die Verfasser bewusst außen schreiben.

### 2.7 Paketdateien

Ein Paket kann als eigene Paketdatei geschrieben werden, die USE-PACKAGE auffindet (16.2). Eine Paketdatei ist eine reine RNL-Datei; ihre empfohlene Endung ist `.rnl`, und sie wird in UTF-8 geschrieben. Sie ist keine Netzliste und wird nicht mit `.include` geladen.

- Eine Paketdatei wird wie die Trägerzeilen einer Netzliste **gelesen** (2.2, 2.4), mit dem Unterschied, dass jede Zeile eine Trägerzeile ist und sowohl das Kommentarzeichen als auch das Leerzeichen danach entfallen:
  - ein `@` am Zeilenanfang zählt so, als stünde ein Leerzeichen davor (2.3), sodass `@RNL` und Schlüsselwörter in Spalte 1 beginnen dürfen;
  - eine Leerzeile ist eine leere Kommentarzeile, und ein Eintrag läuft über sie hinweg weiter;
  - Text außerhalb von Einträgen sind Bemerkungen und wird wie Host-Kommentare ignoriert (2.5); `/* */` funktioniert wie üblich;
  - Zeile 1 wird gelesen: eine Paketdatei ist keine Netzliste und hat keine Titelzeile (2.2).
- Sie enthält `@RNL(…) {…}`-Einträge, geparst in der Reihenfolge von 16.1. Der erste Eintrag ist der Kopfeintrag ohne `language`, der VERSION und PACKAGE enthält; eine Paketdatei darf auch einen Kopfeintrag für jede Sprache haben (2.8). Was ein Paket enthalten darf, einschließlich Verweisen auf andere Pakete, steht in 16.2; Alias-Pakete sind ebenfalls Paketdateien (16.5).

Es folgt der vollständige Text einer Paketdatei ohne Kommentarpräfixe. Die dritte Zeile liegt außerhalb der Einträge und ist eine Bemerkung. Das Paket verwendet `$NMOS`, ohne auf das Standardpaket zu verweisen; daher findet es dieses in dem Standardpaket, das an der Stelle seiner Verwendung wirksam ist, oder bei eigenständiger Prüfung in dem durch eine Werkzeugeinstellung hinzugefügten Standardpaket (16.2, 16.3):

```rnl-file
@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="acme-style", version="1.2")}

ACME house style: NMOS symbol and its rule.
@RNL {
  $NMOS_ACME = @DEFINE-SYMBOL {
    $NMOS;
    ? @TEXT("${name}", key="name") => @PLACE(-2.5, 1.5)
  };
  # @SELECT-INSTANCE(type="nmos", recursive=true) => @SET-SYMBOL $NMOS_ACME
}
```

### 2.8 Eintragskopf

Die Felder in den Klammern von `@RNL(…)` gelten für jede Anweisung des Eintrags, und alle werden mit Namen angegeben. Feldnamen und die Werte von `type` werden in der Standardschreibweise geschrieben; sie unterscheiden Groß- und Kleinschreibung und können keine Aliase haben (3.6, 16.5).

```rnl
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}    /* the first record of the file: read as version 0.4 (16.1) */
* @RNL { %dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N}    /* no type: "authored" */
* @RNL(type="generated", priority="high", author="rnl-solver 0.3", hash="sig1:9f3a62c1") {
*   #M1 @PLACE(12, 20); #M2 @PLACE(28, 20)
* }
* @RNL(author="rnl-editor 0.1") { #M2 @PLACE(x=30, priority="high")}
```

| Feld | Bedeutung | Beeinflusst die Interpretation |
|---|---|---|
| `type` | Die Art des Eintrags: `"authored"` (Standard), `"generated"`, `"header"` | ja |
| `scope` | Die Zielteilschaltung (2.6) | ja |
| `priority` | Die Standardpriorität der Anweisungen im Eintrag; bei AS und ATTACH nur `"disabled"` (14.8) | ja |
| `author` | Der Autor, das Werkzeug und seine Version, die Version eines Pakets usw.; beliebige Zeichenkette | nein |
| `hash` | Ein Hash der Eingabe des Solvers (Kapitel 17) | nein |
| `numbering` | Die Prüfsumme der Nummern, die der Eintrag verwendet (5.9); nur bei `type="generated"` | ja |
| `modified` | Der Zeitpunkt der letzten Änderung, eine Zeichenkette nach ISO 8601 | nein |
| `language` | Die Netzlistensprache, auf deren Abschnitte der Eintrag wirkt: `"spice"` oder `"spectre"` (unten) | ja |

- **type**: Die Werte sind fest, und jeder hat ein definiertes Verhalten.
  - `"authored"`, der Standardwert: von Hand, durch einen Editor oder durch einen Import geschrieben.
  - `"generated"`: das Ergebnis des Solvers. Wenn der Solver löst, wird die Ebene ohne die Einträge mit `type="generated"` interpretiert: sie fügen keine Objekte, Namen oder Fakten hinzu und schränken das Ergebnis nicht ein. Wie der Solver sie sonst nutzt, ist ihm überlassen. Danach ersetzt er sie als Ganzes, unabhängig von ihrem Autor und davon, ob er ihre Anweisungen versteht: die Regel aus 16.4, dass ein Werkzeug beim Neuschreiben eigener Einträge unverstandene Anweisungen behält, gilt für diese Ersetzung nicht. Was erhalten bleiben soll, wird in einem Eintrag anderen Typs geschrieben. Leitungen, die ein Editor beim Ziehen eines Bauteils neu zeichnet, sind ebenfalls Solver-Ergebnisse (14.8, Kapitel 17). `type` impliziert keine Priorität: der Solver schreibt `priority="high"` explizit.
  - `"header"`: die Einstellungen der Datei (16.1). Der Kopfeintrag darf weggelassen werden. Wenn er geschrieben wird, steht er auf Dateiebene, nicht innerhalb von `.subckt` oder eines Verilog-A-Moduls (2.9), und hat kein `scope`; die Kopfeinträge sind die ersten Einträge der Datei, höchstens einer ohne `language` und einer für jede Sprache (siehe `language` unten). Er enthält VERSION (16.1), die Aliase von ALIAS und die Alias-Pakete von USE-ALIASES (16.5), die Anschlussnamen und Typen von DEVICE (6.12), die Pakete an der Wurzel von USE-PACKAGE (16.3) sowie die Einstellungen von SOLVER-CONFIG (14.10); eine Paketdatei schreibt dort außerdem PACKAGE (2.7, 16.2). Der Kopfeintrag selbst wird in der Standardschreibweise ohne Aliase gelesen (16.5). Ein Kopfeintrag nach anderen Einträgen oder ein zweiter ohne `language` bzw. für dieselbe Sprache ist ein Fehler und wird als Ganzes ignoriert; bereits gelesene Einträge werden nicht erneut betrachtet. Ein gelesener Abschnitt von `.lib` zählt als Datei (2.6): sein Kopfeintrag ist der erste Eintrag im Abschnitt. Ein Werkzeug schreibt den Kopfeintrag mit einem Kommentarzeichen des Sprachmodus an der Schreibstelle (2.2). Folgt die erste Anweisung einer Datei auf eine `simulator lang=`-Zeile, darf der Kopfeintrag unmittelbar nach dieser Zeile im Modus des restlichen Dateiteils stehen oder davor, etwa in Zeile 2, in dem Modus, in dem die Datei beginnt; an beiden Stellen ist er der erste Eintrag der Datei.
- **language** lässt einen Eintrag ausschließlich auf Objekte in Abschnitten dieser Sprache wirken.
  - **Abschnitte**: Der Adapter unterteilt jede Datei anhand von `simulator lang=` nach Sprachmodus in Abschnitte (2.2). Die Werte sind `"spice"` für die SPICE-Familie, HSPICE und die übrigen SPICE-Dialekte eingeschlossen, sowie `"spectre"`; spätere Versionen können eine mit diesen nicht kompatible Sprache hinzufügen. Werte unterscheiden Groß- und Kleinschreibung.
  - **Nomineller Geltungsbereich**: Die Ebenen, die ein Eintrag erreicht, sind dieselben wie ohne `language`: ein in einer Ebene geschriebener Eintrag erreicht diese Ebene und die Ebenen darunter, und ein Eintrag in einem Paket erreicht die Ebenen, die das Paket verwenden (16.2). Von dem, was er erreicht, wirkt ein Eintrag mit `language` nur auf die Objekte in Abschnitten seiner Sprache; ein Eintrag ohne `language` wirkt auf alle Abschnitte, in einer Spectre-Datei also auch auf einen SPICE-Abschnitt.
  - **In welchem Abschnitt ein Objekt liegt**: eine Instanz und ihre Anschlüsse im Abschnitt der Instanzzeile; ein Port einer Teilschaltung (`#:port`) im Abschnitt der Zeile, die die Teilschaltung beginnt; ein Netz in jedem Abschnitt mit einer Zeile, die es nennt, sodass ein Netz, das in beiden Arten von Abschnitten vorkommt, von Einträgen beider Sprachen erreicht wird; ein von RNL erzeugtes Objekt (Anwendung, Punkt, Grafik oder Text) im Abschnitt des Host-Objekts, an das es angehängt ist oder das es füllt, wie eine an `&vss` angehängte Masse; die Ebene selbst und was an kein Host-Objekt angehängt ist in jeder Sprache, die eine Zeile in der Ebene hat.
  - **Anweisungen über Abschnittsgrenzen hinweg**: Eine Anweisung, deren Objekte teilweise außerhalb der Abschnitte der Sprache des Eintrags liegen, etwa eine Bedingung, eine Leitung oder ein GROUP, hat als Ganzes keine Wirkung. Ein Verweis auf ein Objekt außerhalb dieser Abschnitte hat keine Wirkung und ist kein Fehler, und was davon abhängt, hat ebenfalls keine Wirkung; ein überhaupt nicht gefundener Name ist wie üblich ein Fehler, wenn die Ebene Abschnitte dieser Sprache hat. Ein Eintrag, dessen Sprache in dem von ihm erreichten Bereich keinen Abschnitt hat, wirkt auf nichts und wird nicht geprüft.
  - **Definitionen werden nicht nach Abschnitten getrennt**: `$`-Namen, Symbole und Stile sind wie üblich im nominellen Geltungsbereich sichtbar (6.8); nur Anweisungen, die auf Objekte wirken, werden nach Abschnitt gefiltert.
  - **Kopfeinträge**: Eine Datei darf einen Kopfeintrag ohne `language` und für jede Sprache einen Kopfeintrag mit ihr enthalten. Sie stehen zuerst hintereinander, der ohne `language` zuerst. VERSION, PACKAGE, ALIAS und USE-ALIASES stehen nur im Kopfeintrag ohne `language`; ein Kopfeintrag mit `language` enthält nur USE-PACKAGE und DEVICE, die dann nur auf die Abschnitte seiner Sprache wirken (6.12, 16.2). Alle Kopfeinträge werden gemeinsam wirksam. Dasselbe Paket in zwei Kopfeinträgen mit verschiedenen Versionen oder verschiedenem Inline-Inhalt ist ein Fehler, da eine Ebene Abschnitte beider Sprachen enthalten kann und ein Paket eines Namens nur einmal verwendet (16.2): das USE-PACKAGE des Kopfeintrags mit `language` schlägt fehl, und das im Kopfeintrag ohne `language` wird für jeden Abschnitt wirksam; zwischen zwei Kopfeinträgen mit `language` schlägt der spätere fehl.
  - Ein Eintrag mit `type="generated"` kann kein `language` haben: der Solver löst eine ganze Ebene und ersetzt deren Einträge als Ganzes. Dort geschrieben ist `language` ein Fehler und wird als nicht geschrieben behandelt.
  - Ein Wert, den diese Version nicht auflistet, lässt den Eintrag auf nichts wirken, mit einer Warnung, die den nächstliegenden Wert nennt; dies ist eine Ausnahme von der Regel unten, dass ein falsch geschriebenes bekanntes Feld als nicht geschrieben gilt, was den Eintrag auf jeden Abschnitt wirken ließe. Werkzeuge, die `language` schreiben, schreiben `@VERSION("0.4")` in den Kopfeintrag ohne `language` (16.1). Neu von Werkzeugen geschriebene Einträge haben kein `language`; Einträge, die es tragen, bleiben unverändert erhalten (16.4).
- **Metadaten**: `author`, `hash` und `modified` beeinflussen die Interpretation nicht; die Zeichnung bleibt gleich, wenn sie gelöscht werden. Werkzeuge nutzen sie zur Entscheidung, wie ein Eintrag zu behandeln ist (Kapitel 17). Weder `type` noch `author` beeinflusst die Auflösungsreihenfolge (8.3).
- **Fehler**:
  - Ein unbekanntes Feld liefert nur eine Warnung und wird ignoriert; es beeinflusst weder den Eintrag noch die übrigen Felder. Dies lässt Raum für spätere Erweiterungen. Wird die Warnung ausgegeben, muss sie den nächstliegenden Feldnamen nennen. Hinweis: Infolgedessen erhält ein verschriebenes Feld, das die Interpretation beeinflussen würde (etwa `priorty="low"`), nur eine Warnung, und der Eintrag wird mit Priorität `"normal"` wirksam; später hinzugefügte Felder dieser Art verlassen sich auf die Version (16.1), um alten Werkzeugen mitzuteilen, dass sie veraltet sind.
  - Ein falsch geschriebenes bekanntes Feld (ein falscher Wert, `numbering` in einem Eintrag, dessen `type` nicht `"generated"` ist oder der `scope` hat, oder das doppelte Schreiben des Feldes) ist ein Fehler, und das Feld wird als nicht geschrieben behandelt: `priority` fällt auf `"normal"` zurück, `type` auf `"authored"`, und Metadaten werden verworfen. Die Ausnahme ist ein `scope`, dessen Ziel nicht gefunden wird: die Anweisungen des Eintrags haben keine Ebene zur Auflösung und schlagen sämtlich fehl (2.6). Ein nach Position angegebenes Feld ist ein Fehler und wird ignoriert.
- Einträge haben keine Bezeichner. Werkzeuge finden ihre eigenen Einträge über `type`, `author` und die Schlüssel der Anweisungen (5.8); auf ein Objekt wird aus einem anderen Eintrag über seinen Namen verwiesen oder aus dem Eintrag des Solvers über seine Nummer (5.9). Weitere Bemerkungen stehen in Kommentaren.
- Ein Solver-Ergebnis ist kein Vorschlag, der erst angenommen werden müsste: es wird unmittelbar wirksam, und ein veraltetes Ergebnis liefert lediglich einen Hinweis (Kapitel 17).

### 2.9 Verilog-A-Dateien

Eine Teilschaltung gibt ihren Instanzen ihr Blocksymbol (block symbol) (9.2.6). Ein von der Netzliste eingebrachtes Verilog-A-Modul kann dasselbe tun, mit Einträgen in der Verilog-A-Datei selbst:

```verilog
// @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
`include "disciplines.vams"
module vco(vin, out, vdd, vss);
  inout vin, out, vdd, vss;
  electrical vin, out, vdd, vss;
  parameter real f0 = 1e9;
// @RNL {
//   %vco = @DEFINE-SYMBOL(box=[-3, -3, 3, 3]) {
//     @RECTANGLE {(-2, -2) (2, 2)};
//     (-3, 0) @PIN(terminal="vin", direction="left");
//     (3, 0) @PIN(terminal="out", direction="right");
//     (0, 3) @PIN(terminal="vdd", direction="up");
//     (0, -3) @PIN(terminal="vss", direction="down");
//     ? @TEXT("${name}", key="name") => @PLACE(0, 2.5);
//     ? @TEXT("f0=${f0}", key="value") => @PLACE(0, -2.5)
//   };
//   # @SET-SYMBOL %vco
// }
  analog begin
    V(out) <+ 0;
  end
endmodule
```

Eine Spectre-Netzliste, die `ahdl_include "vco.va"` und `I1 (ctl out vdd 0) vco f0=2e9` schreibt, zeichnet I1 mit diesem Symbol, ohne eigenes RNL.

- **Welche Dateien.** Die Verilog-A-Quelldateien, die die Netzliste einbringt: mit `ahdl_include` in Spectre, in beiden Sprachmodi, und mit `.hdl` in HSPICE. Eine solche Datei wird dort gelesen, wo sie eingebracht wird, wie eine eingebundene Datei (6.1). Bei `ahdl_include … -master name` von Spectre ist das Modul unter diesem Namen in der Netzliste bekannt, und sein Blocksymbol geht an die Instanzen dieses Namens. Ein Modell, das ein Host kompiliert einbringt, wie eine `.osdi`-Datei von ngspice, hat keinen lesbaren Text; seine Instanzen haben das Standard-Blocksymbol mit einem Pin für jeden Knoten auf ihren Zeilen (9.2.6), und eine obere Ebene kann ihnen per Regel ein Symbol geben.
- **Träger.** Ganzzeilenkommentare `//`, nach den Regeln aus 2.2: `//` in Spalte 1, gefolgt von nichts, nur Leerraum oder genau einem Leerzeichen und dem Inhalt. Kommentare am Zeilenende und `/* */`-Kommentare werden nicht gelesen, und `@RNL` darin liefert eine Warnung. Zeile 1 wird gelesen: eine Verilog-A-Datei hat keine Titelzeile. Text, den der Verilog-A-Präprozessor weglassen kann, zwischen `` `ifdef `` oder `` `ifndef `` und seinem `` `endif ``, wird nicht gelesen, und `@RNL` dort liefert eine Warnung, wie bei den in 2.2 genannten Bereichen, die der Adapter nicht einordnen kann.
- **Ebenen.** Der Text außerhalb aller `module … endmodule` (und `macromodule … endmodule`) ist die Dateiebene der Verilog-A-Datei. Jedes Modul ist eine eigene Ebene, eine Modulebene (module level), unterhalb jener Dateiebene, die unterhalb der Ebene liegt, deren Text die Datei einbringt (6.1).
  - Die Dateiebene enthält ausschließlich die Kopfeinträge (2.8), geschrieben vor dem ersten Modul. Jeder andere Eintrag dort ist ein Fehler und schlägt fehl.
  - Eine Modulebene hat keine Schaltung und keinen Schaltplan: nichts darin wird gezeichnet oder gelöst, und sie hat keinen Eintrag des Solvers.
- **Was eine Modulebene enthält.** DEFINE-SYMBOL und DEFINE-STYLE sowie eine Anweisung `# @SET-SYMBOL` mit einem Symbol, wobei `#` das Modul selbst ist. Dieses SET-SYMBOL gibt das Blocksymbol des Moduls mit allem, was 9.2.6 über das Blocksymbol einer Teilschaltung sagt: es gilt überall, wo das Modul instanziiert wird, und sein Platz in der Symbol-Warteschlange ist derselbe (8.3). Alles andere in einer Modulebene ist ein Fehler und schlägt fehl: Host-Verweise, Bedingungen, PLACE, ORIENT, WIRE, ein TEXT außerhalb eines Symbolrumpfes, andere Regeln als `# @SET-SYMBOL`, AS und ATTACH, Zusicherungen, `scope`, DEVICE und SOLVER-CONFIG. USE-PACKAGE steht ausschließlich im Kopfeintrag der Datei.
- **Anschlüsse.** Die Anschlüsse einer Instanz eines Moduls sind die Knoten, die der Host in ihrer Zeile liest, in der Reihenfolge der Ports in der Portliste des Moduls, und sie sind nach den Ports benannt. Verilog-A vergleicht Namen unter Beachtung der Groß- und Kleinschreibung, daher tun dies auch die Portnamen.
  - Ein Vektorport (vector port) entspricht einem Anschluss für jedes seiner Bits, von der linken Grenze seines deklarierten Bereichs bis zu seiner rechten Grenze, wie ein Vektor nach Position verbunden wird: der Port `inout [3:0] d` nimmt die nächsten vier Knoten der Zeile auf, als `d[3]`, `d[2]`, `d[1]` und `d[0]`, und `inout [0:2] e` nimmt drei, als `e[0]`, `e[1]` und `e[2]`. Dies sind ihre Anschlussnamen, in Verweisen mit einem Subskript geschrieben (6.11), etwa `#I1:d[3:0]`, und in `terminal=` von PIN als Zeichenketten, wie ein Bus-Pin mit `terminal=["d[3]", "d[2]", "d[1]", "d[0]"]` (9.2.4).
  - Ein Modul gibt seinen Instanzen keinen Typ (6.12); DEVICE in der Netzliste kann ihnen wie anderen Instanzen einen geben.
- **Versionen und Pakete.** Der Kopfeintrag einer Verilog-A-Datei ist der einer eingebundenen Datei: ohne ihn oder ohne VERSION übernimmt die Datei die Version der Datei, die sie einbringt (16.1), und ihr USE-PACKAGE gilt nur für die Datei und ihre Module (16.2).
- **Schreiben.** Ein Werkzeug schreibt `//` in einer Verilog-A-Datei und ausschließlich den Kopfeintrag sowie die Einträge von Blocksymbolen; es schreibt dort keine Ausgabe des Solvers (Kapitel 17).

<!-- translated from en/ch03.md sha256:9148069f1090 -->

## 3 Lexikalische Struktur

### 3.1 Zeichen

- Tokens bestehen aus ASCII-Zeichen.
- Nicht-ASCII-Zeichen erscheinen nur in Zeichenketten (einschließlich Namen in Anführungszeichen), Freitext, Kommentaren und Aliasen sowie in unquotierten Rollen- und Parameternamen. Außerhalb von Freitext und Kommentaren erscheint ein unquotiertes Nicht-ASCII-Wort nur nach `@` und innerhalb von Klammern: nach `@` muss es ein deklarierter Alias sein; innerhalb von Klammern ist es ein Alias oder ein Rollen- bzw. Parametername selbst (3.6, 16.5).
- **Zeichenkodierung.** Der Host-Adapter dekodiert den Text nach den Regeln des Hosts; RNL arbeitet auf Zeichen. Ein Werkzeug, das RNL schreibt, darf weder die Zeichenkodierung noch die Zeilenenden der Host-Datei ändern und keine Byte-Order-Mark (BOM) hinzufügen; es darf kein Zeichen schreiben, das die Kodierung des Hosts nicht darstellen kann, und gibt stattdessen eine Warnung aus. Neue Dateien sollten UTF-8 verwenden.
- **Nicht-ASCII-Zeichen in Host-Kommentaren.** Adapter müssen die Hosts angeben, für die Nicht-ASCII-Zeichen in Kommentaren verifiziert wurden; bei anderen Hosts liefert ein Nicht-ASCII-Zeichen einen Hinweis.

  Hinweis: Die Unterstützung unterscheidet sich je nach Host. ngspice (ab Version 42), WRspice (außer im Titel), Xyce und neuere Versionen von LTspice akzeptieren Nicht-ASCII-Zeichen in Kommentaren; die Dokumentation von HSPICE, PSpice und Eldo macht dazu keine Angabe; das Spectre-Handbuch beschreibt Netzlisten als ASCII-Dateien; QSPICE verwendet Windows-1252. Ein weiterer Grund gegen das Hinzufügen einer BOM: ältere Versionen von ngspice fügen sie an das erste Token einer eingebundenen Datei an, und WRspice bricht das Lesen ab, wenn Zeile 1 damit beginnt.

### 3.2 Tokens

| Token | Form |
|---|---|
| Schlüsselwort | `@` gefolgt von einem Namen in Großbuchstaben wie `@PLACE`; oder ein deklarierter Alias (16.5) |
| Verweis | Ein Präfix gefolgt von einem Namen, gegebenenfalls mit Anschlüssen und Unterobjekten wie `#M1`, `#M1:d`, `%dp1::left` (Kapitel 6); oder `#`, `?` oder `&` allein (6.2). Auf einen Host-Namen oder Anschlussnamen darf direkt ein Subskript folgen wie `&D<3>`, `&D<7:0>` (6.11). Auf einen Verweis auf eine Menge dürfen direkt Mitgliedspositionen folgen wie `%a{1}`, `?ref{2:end}` (5.2.3). In einem Eintrag des Solvers außerdem `%#` gefolgt von einer vorzeichenlosen Ganzzahl, einer Nummer wie `%#3` (5.9) |
| Punkt | `(x, y)`, ohne ein Schlüsselwort oder Objekt zu berühren: nach Leerraum, einem Kommentar, `{`, `;`, `=>` oder `=` (3.8) |
| Runde Klammern | Die `(` `)` von Parametern und Rollen, unmittelbar am Schlüsselwort oder Objekt anliegend (3.5) |
| Geschweifte Klammern | Die `{` `}` von Mengen |
| Trennzeichen | `;` (trennt Mitglieder), `=>` (Pipes und Benennung), `=` (Benennung am Anfang eines Mitglieds) |

Innerhalb von Klammern (den `()` von Parametern, Rollen und Punkten sowie den `[]` von Arrays) gibt es außerdem Zeichenketten, Zahlen, `,`, `=`, `_` und Namen (Parameternamen, Rollennamen, `true`, `false`), sowie im Rumpf einer allgemeinen Definition Parameterplatzhalter `?name` (6.10). Innerhalb der `<>` und `[]` von Subskripten stehen nur vorzeichenlose Ganzzahlen, `,` und `:` (6.11); innerhalb der geschweiften Klammern von Mitgliedspositionen nur vorzeichenlose Ganzzahlen, `end`, `-`, `,` und `:` (5.2.3).

### 3.3 Freitext

- Außerhalb von Klammern kann jedes Zeichen Freitext sein, mit Ausnahme der unten aufgeführten. Dies schließt alle Nicht-ASCII-Zeichen (etwa die chinesischen Satzzeichen `：`, `、`, `；`, `（`, `）`) sowie `-`, `:`, `'`, `!`, `+`, `/`, `*` und `|` ein. Die Ausnahmen sind:
  - `@`, `#`, `&`, `%`, `$`, `?`, `"`, `;`, `=`, `{`, `}`, `(`, `)`: außerhalb von Klammern beginnen sie Tokens oder gehören zu ihnen (3.2). `&` allein, ohne nachfolgenden Namen, ist der leere Knoten (empty node) (6.3). `?` beginnt einen Platzhalter (6.10); ein alleinstehendes `?` verweist in einem Symbolrumpf auf das mit dem Symbol gezeichnete Objekt (13.3) und im Rumpf einer allgemeinen Definition auf die gerade expandierte Anwendung (9.4);
  - `[`, `]`, `<`, `>`: die Klammern von Arrays und Subskripten (3.7, 6.11);
  - `/*`: es leitet einen Kommentar ein (3.4).
- Positionsregeln halten die Bedeutung eindeutig:
  - Ein `:` oder `::` unmittelbar nach einem Verweis und gefolgt von einem Namen oder Anführungszeichen ist wie üblich ein Anschluss oder ein Unterobjekt (6.4, 6.5). Folgt kein Name, ist der `:` Freitext, wie in `Input pair: %dp1 = …` und `#M1: the left one`.
  - Ein `<` oder `[` unmittelbar nach einem Namen ist ein Subskript (6.11).
  - Ein `{` unmittelbar nach einem Verweis, ohne Leerraum, beginnt Mitgliedspositionen (5.2.3); nach Leerraum beginnt es eine Menge.
  - Im Namen eines Schlüsselworts gehört ein `-` nur dann zum Namen, wenn ein Buchstabe darauf folgt (Schlüsselwörter: 3.6). `@GROUP- the pair` ist GROUP gefolgt von Freitext.
- Freitext darf zwischen zwei beliebigen Tokens außerhalb von Klammern sowie am Anfang eines Mitglieds stehen.
- **Freitext ist Teil der Anweisung.** Ein darin geschriebener Verweis ist keine Erläuterung, sondern ein Objekt der Anweisung: `Unlike #M2, #M1 @PLACE(x=0)` setzt auch M2 auf x = 0. Text, der nur als Erläuterung dient, wird in einem Kommentar (3.4) oder als eigenes, durch `;` vom nächsten Mitglied getrenntes Mitglied geschrieben: ein Mitglied mit ausschließlich Freitext ist ein leeres Mitglied und wird ignoriert (4.1).
- Der Standard prüft Freitext nicht, und Freitext ändert die Bedeutung nicht (Prinzipien 2.8): `do not move #M2 @PLACE(x=30)` verschiebt M2 dennoch. Eine Implementierung darf von sich aus eine Warnung für Wörter wie Verneinungen ausgeben (7.6).
- Ebenso ist in einem fehlerhaften `#M1.d` das `.d` Freitext, und der Verweis bezeichnet M1 selbst, nicht dessen Drain (eine Warnung wird ausgegeben, siehe unten).
- Innerhalb von Klammern gibt es keinen Freitext.
- **Warnungen.** Die folgenden Versehen könnten unbemerkt bleiben; sie liefern eine Warnung (7.6):
  - Ein Wort im Freitext, das der Name einer Host-Instanz dieser Ebene, ein sichtbarer RNL-Name (ohne Präfix) oder der Name eines Schlüsselworts ist, etwa `M3` in `#M1 #M2 M3 @ALIGN("y")` oder `WIRE` in `WIRE {…}`: wahrscheinlich fehlt ein `#`, `%`, `$` oder `@`. Netznamen sind oft gewöhnliche Wörter (wie in und out) und werden nicht geprüft.
  - Ein Verweis, auf den unmittelbar `.` und ein Name folgen, wie `#M1.d`: Anschlüsse werden mit `:` geschrieben.
  - Ein Verweis, auf den unmittelbar `-`, `!`, `'`, `+`, `/`, `*`, `\`, `^`, `~` oder `|` folgt, wie `&vdd!` oder `#X1-a`: der Name enthält wahrscheinlich dieses Zeichen und benötigt Anführungszeichen wie `&"vdd!"` (3.6); ohne sie bezeichnet der Verweis stillschweigend `vdd` oder `X1`.
  - `|` im Freitext: eine Pipe wird `=>` geschrieben (4.2 Punkt 3).
  - `:` oder `::` unmittelbar nach einem Verweis und gefolgt von einem Nicht-ASCII-Wort, wie `%c1::中点`: ein Name mit Nicht-ASCII-Zeichen nach `::` benötigt Anführungszeichen, wie `%c1::"中点"` (3.6); ohne sie sind `::` und das Wort Freitext, und der Verweis bezeichnet `%c1`.
- Die meisten anderen Fehler dieser Art sind bereits Fehler: in Klammern gibt es keinen Freitext, sodass `@ORIENT(MY)` ein Fehler ist; `#M1 @PLACE 1 2` passt nicht zur Signatur; `#M1 (in_p)` ist ein Fehler, da eine runde Klammer nach Leerraum einen Punkt beginnt und `(in_p)` kein Punkt ist (3.5); und das `>` in `R1 -> R2` kann kein Freitext sein.
- Schreibhilfen für Verfasser werden aus der Wortliste erzeugt (5.3). Von Solvern und Editoren geschriebene Einträge dürfen keinen Freitext enthalten.

### 3.4 Kommentare

`/* … */` darf überall stehen, wo Leerraum zulässig ist, auch innerhalb von Klammern. Kommentare sind nicht schachtelbar, und jedes beliebige Zeichen darf darin stehen. Eine Erläuterung, die Verweise oder Zeichen wie `@`, `#` und `;` enthält, wird in einen Kommentar geschrieben (3.4).

### 3.5 Leerraum und Nachbarschaft

- Leerraum sind das Leerzeichen, der TAB und der Zeilenumbruch. Tokens dürfen nicht über Zeilen hinweg umbrochen werden, und Zeichenketten dürfen keine Zeilen überschreiten.
- Leerraum und Kommentare sind unzulässig: zwischen einem Präfix und seinem Namen sowie zwischen `%#` und seiner Nummer; auf beiden Seiten von `:` und `::`; zwischen einem Namen und seinem Subskript (6.11); zwischen einem Verweis und dem `{` seiner Mitgliedspositionen (5.2.3); zwischen einem Schlüsselwort und seinen Parameterklammern sowie zwischen `@RNL` und der Klammer seines Eintragskopfs (2.4); zwischen einem Objekt und seinen Rollenklammern.
- Ein `(` nach Leerraum oder einem Kommentar beginnt einen Punkt (3.8). Daher ist `#M1 @PLACE (10, 20)` PLACE gefolgt von einem Punkt, und PLACE hat keine Parameter: ein Fehler. `#M1 (in_p)` ist M1 gefolgt von einem falsch geschriebenen Punkt: ebenfalls ein Fehler. Die Diagnosemeldung schlägt vor, den Leerraum zu entfernen.
- Ein `<` oder `[` unmittelbar nach einem Namen, mit oder ohne Anführungszeichen, beginnt immer ein Subskript (6.11). Punkte werden `(x, y)` geschrieben und stehen daher nicht im Konflikt mit Subskripten; um einen Punkt nach einem Namen zu schreiben, trennt man sie mit Leerraum, wie in `&D (1, 2)`. Innerhalb eines Subskripts dürfen Leerraum und Kommentare stehen, wie in `&D<1, 2:3>` (6.11).
- Ein `.` unmittelbar nach einem Schlüsselwort ist Freitext, wie der Satzpunkt in `{#M1 #M2} @GROUP.`; `@BUNDLE.LOCK` ist BUNDLE gefolgt vom Freitext `.LOCK` (16.4).

### 3.6 Namen

- **Unquotierte Namen** bestehen aus einem oder mehreren ASCII-Buchstaben, Ziffern und Unterstrichen, bestimmt durch längste Übereinstimmung.
- **Host-Namen** (nach `#` und `&`) und Anschlussnamen folgen den Regeln des Host-Dialekts (Prinzipien 2.1): welche Namen gültig sind, ob Groß-/Kleinschreibung zählt und ob ein Name mit einer Ziffer beginnen darf, wie `&0`. SPICE unterscheidet Groß- und Kleinschreibung nicht; Spectre tut dies. Ein Name mit anderen Zeichen wird in Anführungszeichen geschrieben, etwa `#"M1<0>"`, `&"vdd!"`, `#XU1:"in+"`; der Text in Anführungszeichen wird wie in Zeichenketten geschrieben (3.7). Auf einen Namen darf unmittelbar ein Subskript folgen: `#M1<0>` ist `#"M1<0>"`, und `&D<7:0>` ist eine Liste von Netzen (6.11). Ein `<` oder `[` innerhalb von Anführungszeichen ist Teil des Namens und wird nicht geparst.
- **RNL-Namen** (die Namen nach `%`, `$` und `?`, Rollennamen sowie die Parameternamen allgemeiner Definitionen):
  - Sie unterscheiden Groß- und Kleinschreibung: `$DIFFERENTIAL_PAIR` und `$differential_pair` sind zwei Namen.
  - Nach `%`, `$` und `?` darf ein Name mit einer Ziffer beginnen, wie `%1st`. An den Stellen ohne Präfix (`(role)` und `parameter=`) besteht ein unquotierter Name aus ASCII-Buchstaben, Ziffern, Unterstrichen und beliebigen Nicht-ASCII-Zeichen und beginnt nicht mit einer ASCII-Ziffer, damit er nicht wie eine Zahl aussieht: `#M1(正输入)`, `@AS(间距=3) $X`. Beide Stellen liegen innerhalb von Klammern, wo es keinen Freitext gibt (3.3), sodass kein anderes Wort in den Namen hineingezogen wird.
  - Sie dürfen mit derselben Bedeutung in Anführungszeichen stehen: `$"a"` ist `$a`; Anführungszeichen maskieren nur. Namen mit anderen Zeichen sowie Nicht-ASCII-Namen werden in Anführungszeichen geschrieben, wie `%"差分对"`, `%"input pair"`, `?"正输入"`, `%dp::"正输入"`. An Rollen- und Parameternamen-Stellen dürfen Nicht-ASCII-Namen unquotiert stehen (Punkt oben), ein Leerzeichen oder andere ASCII-Satzzeichen erfordern jedoch weiterhin Anführungszeichen, wie `#M1("input pair")`. An Rollen- und Parameternamen-Stellen ist ein Name in Anführungszeichen stets der Name selbst, niemals ein Alias. Dies gilt nicht für `$`-Namen und Namen nach `::`, deren Nicht-ASCII-Aliase ohnehin in Anführungszeichen stehen (16.5).
  - Sie werden Zeichen für Zeichen wie geschrieben verglichen, ohne Unicode-Normalisierung. Ein leerer Name ist ein Fehler.
  - Sie nehmen kein Subskript an (6.11): ein `<` oder `[` unmittelbar nach ihnen ist ein Fehler, wie in `%p<1, 2>`. Ein Punkt wird `(1, 2)` geschrieben, durch Leerraum vom Namen getrennt.
- **Aliase** (16.5) werden im Kopfeintrag deklariert oder stammen aus Alias-Paketen. Aliase von Schlüsselwörtern und Standardwörtern dürfen Nicht-ASCII-Buchstaben enthalten; sie werden nur nach `@` und innerhalb von Klammern ohne Anführungszeichen geschrieben. Ein Alias eines `$`-Namens wird wie ein `$`-Name geschrieben, in Anführungszeichen, wenn er Nicht-ASCII ist, etwa `$"接地补位集"`. Unquotierte Namen nach `#`, `&`, `%`, `$`, `?` und `::` sind wie oben nur ASCII, sodass `#M1和#M2` `#M1`, der Freitext `和` und `#M2` ist.
- **Schlüsselwörter** stehen nur in Großbuchstaben, mit durch `-` verbundenen Wörtern wie `@AS` und `@SET-SYMBOL`; Großbuchstaben erleichtern auch das Auffinden im Freitext. `@at` ist ein unbekanntes Schlüsselwort, sofern der Kopfeintrag es nicht als Alias deklariert (16.5).
- **Standardwörter** unterscheiden Groß- und Kleinschreibung und werden genau so geschrieben, wie der Standard sie buchstabiert: Parameternamen, Feldnamen des Eintragskopfs, Unterobjektnamen, Namen von Auswahlbedingungen und Aufzählungswerte (wie `"x"`, `"low"`, `"MY"`). Werte, die Host-Dinge benennen (Namen in Auswahlbedingungen, Modellnamen, Netznamen), folgen den Regeln des Hosts. Andere Schreibweisen können über Aliase akzeptiert werden (16.5).
- **`true` und `false`** sind reserviert und nur in Kleinbuchstaben zulässig. Sie können keine Rollennamen oder Parameternamen sein.

### 3.7 Literale

- **Zahlen** sind dezimal. Sie dürfen ein Vorzeichen und führende Nullen haben, die Null vor dem Dezimalpunkt oder die Ziffern danach weglassen und einen Exponenten haben: `0`, `+1`, `01`, `.5`, `1.`, `-2.5E-4`. `01` ist 1, nicht oktal. Es gibt keine Einheiten oder technischen Suffixe (`2u` und `10k` sind Fehler), keine Hexadezimalzahlen, kein `NaN` und kein `Infinity`. Die Anzahl der Ziffern ist nicht begrenzt. Kann eine Implementierung einen Wert nicht halten, schlägt die Anweisung in dieser Implementierung fehl, mit einem Fehler, der eine Grenze der Implementierung meldet; der Wert wird nicht stillschweigend geändert. Koordinaten werden mit einer Toleranz verglichen (11.7).
- **Zeichenketten** stehen in doppelten Anführungszeichen und können keine Zeilen überschreiten. Die Zeichen zwischen den Anführungszeichen werden unverändert übernommen; nur `"` selbst wird `""` geschrieben, wie in `"say ""hi"""`. Es gibt keine Backslash-Maskierungen: `\` ist ein gewöhnliches Zeichen, und es gibt kein `\n` oder `\uXXXX`; Nicht-ASCII-Zeichen werden direkt geschrieben. Ein Zeilenumbruch kann nicht in einer Zeichenkette geschrieben werden; ein Zeilenumbruch in einem Text wird mit der Auszeichnung `\\` geschrieben (13.6). TEXT besitzt eigene Backslash-Befehle (13.6); sie legen fest, wie TEXT seinen Text liest, und sind keine Maskierungen der Zeichenkette. Zeichenketten im Eintragskopf werden genauso geschrieben; dies gehört zur äußeren Form und ändert sich nicht (16.1).
- **Arrays** werden `[value, value, …]` geschrieben. Ihre Elemente sind Literale, und Arrays dürfen geschachtelt sein.
- **Wahrheitswerte** sind `true` und `false`.
- Es gibt keine weiteren Literale. Aufzählungen werden als Zeichenketten geschrieben, etwa `axis="x"`; es gibt kein `null`, keine Map und keinen konstruierten Wert.
- Es gibt keine Ausdrücke: Operatoren sind Fehler.

### 3.8 Punkte

- Ein Punkt wird `(x, y)` geschrieben, wobei x und y Zahlen oder `_` sind (im Rumpf einer allgemeinen Definition auch ein Parameterplatzhalter, 6.10). `_` lässt diese Achse frei, sodass Bedingungen oder der Solver sie festlegen.
- Das Schreiben eines Punktes erzeugt ein Punktobjekt (5.1).
- Es gibt keine Arithmetik auf Punkten; relative Positionen werden als Bedingungen geschrieben (Kapitel 14). Im Rumpf einer allgemeinen Definition sind literale Koordinaten Versätze gegenüber dem eigenen Punkt der Anwendung (9.4).
- Vor dem `(` eines Punktes steht Leerraum oder ein Kommentar, oder es folgt unmittelbar auf `{`, `;`, `=>` oder `=`. Klammern, die ein Schlüsselwort oder Objekt berühren, enthalten dessen Parameter oder Rolle (3.5). Ein `<` unmittelbar nach einem Namen ist in jedem Fall ein Subskript (6.11).

<!-- translated from en/ch04.md sha256:86b9e3072d96 -->

## 4 Syntax

### 4.1 Grammatik

Die Grammatik eines Eintrags ist in EBNF angegeben. Jede Produktion hat die Form `name = expression ;`; `|` trennt Alternativen, `[ … ]` ist optional, `{ … }` wird null- oder mehrmals wiederholt, `( … )` gruppiert, Text in doppelten Anführungszeichen steht wörtlich, und `(* … *)` ist ein Kommentar.

```ebnf
record         = "@RNL" [ parameters ] set ;          (* the bracket touches "@RNL" *)
set            = "{" [ member ] { ";" [ member ] } "}" ;
member         = [ front-naming ] segment { naming } { "=>" segment { naming } } ;
front-naming   = ( "%" rnl-name | "$" rnl-name ) "=" ;
naming         = "=>" ( "%" rnl-name | "$" rnl-name ) ;
segment        = argument { argument } [ call { argument } ]
               | call { argument } ;
call           = keyword [ parameters ] ;          (* the bracket touches the keyword *)
keyword        = "@" keyword-name ;                (* upper case, or a declared alias *)
argument       = object [ "(" role-label ")" ] ;   (* the bracket touches the object *)
object         = reference | placeholder | point | set ;
reference      = "#" [ host-name [ subscript ] ] path
               | "&" host-name [ subscript ] path
               | "%" rnl-name [ places ] path
               | "%#" integer path                (* a number, in a record of the solver, 5.9 *)
               | "$" rnl-name [ places ] ;
placeholder    = "?" rnl-name [ places ] path
               | "?" path ;                        (* alone: in a symbol body, or in a general definition, 9.4; a path only in a general definition *)
places         = "{" place-item { "," place-item } "}" ;    (* touching the name, 5.2.3 *)
place-item     = place [ ":" place [ ":" integer ] ] ;
place          = integer | "end" [ "-" integer ] ;
path           = { ":" terminal-name [ subscript ] | "::" rnl-name } ;
subscript      = "<" subscript-item { "," subscript-item } ">"
               | "[" subscript-item { "," subscript-item } "]" ;
subscript-item = integer [ ":" integer [ ":" integer ] ] ;   (* unsigned, 6.11 *)
host-name      = name | string ;
rnl-name       = name | string ;
terminal-name  = name | string ;
role-label     = bracket-name | string ;
point          = "(" coordinate "," coordinate ")" ;   (* after white space, a comment, "{", ";", "=>" or "=", 3.8 *)
coordinate     = number | "_" | parameter-placeholder ;
parameters     = "(" [ value { "," value } [ "," named-value { "," named-value } ]
                     | named-value { "," named-value } ] ")" ;
named-value    = parameter-name "=" value ;
parameter-name = bracket-name | string ;
value          = number | string | "true" | "false"
               | "[" [ value { "," value } ] "]"
               | parameter-placeholder ;
parameter-placeholder = "?" rnl-name ;
```

- `name`, `bracket-name`, `keyword-name`, `string`, `number` und `integer` werden wie in Kapitel 3 definiert geschrieben. `name` ist ein unquotierter Name (3.6). `bracket-name` ist ein unquotierter Name in den Klammern einer Rolle oder vor dem `=` eines Parameters: ASCII-Buchstaben, Ziffern, `_` und beliebige Nicht-ASCII-Zeichen, nicht mit einer ASCII-Ziffer beginnend (3.6). `keyword-name` besteht aus Buchstaben, Ziffern und `_`, wobei Wörter durch `-` verbunden sind; ein `-` gehört nur dann dazu, wenn ein Buchstabe darauf folgt (3.3). Ein Standardschlüsselwort steht in Großbuchstaben (3.6); ein Alias besteht aus Buchstaben beliebiger Schrift, Ziffern und `_`, ohne `-` (16.5). `string` und `number` stehen in 3.7; `integer` ist eine vorzeichenlose Dezimalganzzahl.
- `rnl-name`, `host-name`, `terminal-name`, `role-label` und `parameter-name` dürfen mit derselben Bedeutung auch in Anführungszeichen stehen, außer dass ein Name in Anführungszeichen an Rollen- und Parameternamen-Stellen niemals ein Alias ist (3.6). Schlüsselwörter, Parameternamen, Slot-Namen in Rollenklammern, Unterobjektnamen, `true` und `false` dürfen als deklarierte Aliase geschrieben werden; unquotierte Nicht-ASCII-Aliase werden nur nach `@` und innerhalb von Klammern verwendet, und ein Alias nach `::` steht in Anführungszeichen (16.5).
- Ein Subskript folgt unmittelbar auf einen Host-Namen oder Anschlussnamen, ohne Leerraum dazwischen; innerhalb eines Subskripts dürfen Leerraum und Kommentare stehen (3.5, 6.11). Welche Namen ein Subskript annehmen: 4.2 Punkt 10.
- Die Parameterklammern berühren das Schlüsselwort, und die Rollenklammern berühren das Objekt; ein `(` nach Leerraum oder einem Kommentar beginnt einen Punkt (3.5).
- `=>` ist eine Pipe (pipe) oder eine Benennung, je nachdem, was darauf folgt (4.2 Punkt 3).
- Ein Platzhalter mit Namen wird nur im Rumpf einer allgemeinen Definition geschrieben (6.10). An einer Objektstelle ist er eine Rolle und darf einen Pfad haben; an einer Stelle, die ein Literal annimmt (ein Parameterwert, ein Array-Element, eine Punktkoordinate), ist er ein Parameter der Definition und hat keinen Pfad (6.10, 9.4). Das alleinstehende `?` wird in einem Symbolrumpf (13.3) und im Rumpf einer allgemeinen Definition (9.4) geschrieben; nur in einer allgemeinen Definition darf es einen Pfad haben, etwa `?::position`.
- Freitext und Kommentare dürfen zwischen zwei beliebigen Tokens außerhalb von Klammern sowie am Anfang eines Mitglieds stehen (3.3, 3.4); Leerraum und Kommentare dürfen auch innerhalb von Klammern stehen. Die Grammatik zeigt sie nicht.
- Ein leeres Mitglied (wie in `;;`) wird ignoriert; das `;` nach dem letzten Mitglied darf weggelassen werden.

### 4.2 Bedingungen außerhalb der Grammatik

1. Ein Segment ist nicht leer.
2. Ein Segment hat höchstens ein Schlüsselwort. `@RNL` erscheint nur am Anfang eines Eintrags.
3. Ein `=>`, auf das ein `%`- oder `$`-Name folgt, nach dem `=>`, `;`, `}` oder das Ende des Mitglieds kommt, ist eine Benennung. Jedes andere `=>` beginnt das nächste Segment; dieses Segment hat genau ein Schlüsselwort, und ein dort geschriebener Name ist ein Objekt des Segments. Das erste Segment einer Pipe darf ohne Schlüsselwort sein; dann hat es genau ein Objekt. Um daher ein Ergebnis zu benennen und die Pipe fortzusetzen, schreibt man `=> %g0 => @PLACE(1, 2)`. In `=> %g0 @PLACE(1, 2)` ist `%g0` ein Objekt von PLACE; üblicherweise ist es nicht gebunden, und die Anweisung schlägt fehl, weil `%g0` nicht gefunden wird. Die Diagnosemeldung darf fragen, ob ein `=>` fehlt.
4. In einem Segment ohne Schlüsselwort ist jedes Argument eine eigene Anweisung: `#M1 #M2` in `{#M1 #M2}` sind zwei Anweisungen. Ein solches Segment hat, wenn es benannt ist (`%p =` oder `=> %p`), nur ein Objekt: man schreibt `%p = {#M1 #M2}`, nicht `%p = #M1 #M2`.
5. Rollen werden nur an den Argumenten eines Segments markiert, das ein Schlüsselwort hat. Die Segmente nach einer Pipe markieren keine Rollen.
6. In einem Segment sind entweder alle Argumente mit Rollen markiert oder keines. Dieselbe Rolle wird nicht zweimal markiert.
7. Bei den Schlüsselwörtern, die eine Definition annehmen, legt das Schlüsselwort den Slot der Definition fest: bei ATTACH ist sie das erste geschriebene Objekt; bei AS, SET-SYMBOL, SET-STYLE, SELECT-USAGE und SELECT-ANCESTOR-USAGE ist sie das letzte Objekt. SELECT-USAGE darf seine Definition weglassen: sein letztes Objekt füllt den Definitions-Slot nur dann, wenn es eine Definition ist (10.1). Die Definition wird nicht mit einer Rolle markiert und zählt nicht für Regel 6 (5.6).
8. Es gibt zwei Arten der Benennung, die auch gemeinsam verwendet werden dürfen. `name =` kann nur das erste Token eines Mitglieds sein (Freitext darf davor stehen), höchstens einmal je Mitglied. `=> name` folgt auf ein Segment, am Ende des Mitglieds oder vor dem `=>` des nächsten Segments, und ein Mitglied darf mehrere haben. Regel 3 unterscheidet es von einer Pipe.
9. Die nach Position angegebenen Parameter stehen zuerst, danach die mit Namen angegebenen: nach einem `name=value` werden alle folgenden Parameter mit Namen angegeben. Nur die Parameter, für die die Signatur eine Positionsreihenfolge angibt, dürfen nach Position übergeben werden, und zwar in dieser Reihenfolge. Kein Parameter wird zweimal angegeben.
10. Ein Subskript folgt nur auf einen Host-Namen nach `#` oder `&`, oder auf einen Anschluss- oder Portnamen in einem `#`-Verweis. RNL-Namen, Rollennamen und Unterobjektnamen nehmen kein Subskript an (6.11).
11. Ein Verweis hat höchstens ein Subskript, das zu einer Liste von Namen expandiert (6.11).

### 4.3 Wortstellung

- Das Schlüsselwort mit seinen Parametern darf an beliebiger Stelle zwischen den Objekten stehen; die Reihenfolge der Objekte ist maßgeblich. `#M1 #M2 @DISTANCE(…)`, `#M1 @DISTANCE(…) #M2` und `@DISTANCE(…) #M1 #M2` sind identisch.
- Jedes Schlüsselwort hat eine einzige Standardform, und Werkzeuge dürfen ausschließlich diese Form schreiben. Die Standardform ist in der Wortliste festgehalten (5.3): wo das Schlüsselwort steht, welche Parameter nach Position und welche mit Namen angegeben werden und ob Rollen markiert werden.
- Standardmäßig lautet die Standardform: zuerst die Objekte, dann das Schlüsselwort mit seinen Parametern, dann die Definition nach dem Schlüsselwort. ATTACH schreibt seine Definition zuerst, dann das Schlüsselwort, dann die Objekte: `$GROUND @ATTACH &vss`. TABLE schreibt sein Schlüsselwort zuerst, vor der Tabelle: `@TABLE layout: {…}` (14.11). Die Parameter, für die die Signatur eine Positionsreihenfolge angibt, werden zuerst nach Position übergeben; die übrigen werden mit Namen in der Reihenfolge der Wortliste übergeben: `@PLACE(10, 20)`, `@ORIENT("R90")`, `@DISTANCE("bottom-to-top", min=4)`, `@TEXT("W/L", key="wl")`. PLACE gibt x und y nur dann nach Position an, wenn beide geschrieben sind; eine einzelne Achse wird mit Namen geschrieben, wie `@PLACE(x=10)`. Parameter, die ohne Bedeutungsänderung weggelassen werden können, werden nicht geschrieben, etwa `mode="auto"` von WIRE. Eingebaute Schlüsselwörter werden ohne Rollen geschrieben; AS und ATTACH markieren alle Rollen, wenn die allgemeine Definition zwei oder mehr Rollen hat.
- Formatierung: ein Leerzeichen nach jedem Komma in Klammern; kein Leerzeichen um das `=` eines Parameters; ein Leerzeichen auf jeder Seite des `=` einer Benennung sowie von `=>`; das Schlüsselwort berührt seine Klammer; ein Leerzeichen zwischen `{` und einem Schlüsselwort wie `{ @…` (2.3); Zahlen werden als kürzeste Dezimalzahl geschrieben, die beim Wiedereinlesen denselben Wert ergibt.
- **Standardform der Benennung.** Ein Mitglied ohne Pipe wird am Anfang benannt, `%x = statement`; ein Mitglied mit Pipe wird am Ende benannt, `statement => … => %x`, mit dem Namen hinter dem Segment, das er benennt. Dadurch benennt ein Name am Anfang stets ein ganzes Mitglied, und Leser müssen nicht raten, welches Segment einer Pipe er benennt. Die Syntax akzeptiert beide Formen.
- „Ausschließlich diese Form“ gilt für die Anweisungen, die ein Werkzeug neu schreibt. Wenn ein Werkzeug bestehende Einträge neu schreibt, darf es von Benutzern geschriebene Anweisungen nicht umordnen (16.4); es bringt sie nur dann in die Standardform, wenn der Benutzer eine Formatierung anfordert.
- Standardsätze stehen in 4.5.

### 4.4 Beispiele für Fehler

Jedes der folgenden Beispiele ist ein Fehler; der Kommentar nennt den Grund.

```rnl error
#M1 @PLACE (10, 20)    /* a bracket after white space is a point, and PLACE has no parameters (3.5) */
```

```rnl error
#M1 (left) #M2 (right) #M5 (tail) @AS $DIFFERENTIAL_PAIR_N    /* role brackets touch the object; a separate bracket is a point (3.5) */
```

```rnl error
#M2 @PLACE(62, 40) (MY)    /* the second bracket is a point, and its content is not a point */
```

```rnl error
&vss @ATTACH $GROUND    /* the definition of ATTACH is written first: $GROUND @ATTACH &vss */
```

```rnl error
R1 -> R2 {#R1 #R2} @ORDER("left-to-right")    /* ">" cannot be free text (3.3) */
```

```rnl error
#M1 @PLACE(1, 2), #M2 @PLACE(3, 4)    /* a comma does not separate statements: two keywords in one segment */
```

```rnl error
#M1 #M2 @SET-SYMBOL $NMOS    /* three objects, and SET-SYMBOL has two slots (5.2.1); write {#M1 #M2} @SET-SYMBOL $NMOS */
```

```rnl error
#M1 => #M2    /* the segment after a pipe has no keyword */
```

```rnl error
#M1 #M2 #M3 @DISTANCE("left-to-right", min=1)    /* three objects for two slots */
```

```rnl error
#M1 @PLACE(x=10, 20)    /* a parameter given by position after one given by name (4.2 item 9) */
```

```rnl error
{#M1 #M2} @ORDER(axis="x", direction="increasing")
/* ORDER has no axis: the axis and the direction are one parameter, direction, as in @ORDER("left-to-right") (14.3) */
```

```rnl error
#M1 @SET-SYMBOL($NMOS)    /* brackets hold only literals */
```

```rnl error
{#M1 #M2} @ALIGN(axis=y)    /* enumerations are written as strings */
```

```rnl error
%ax = (_, _);
#M1(left) #M2 %ax @AS $MIRRORED_LAYOUT /* roles are marked on all objects or on none */
```

```rnl error
$NMOS(symbol) #M1(target) @SET-SYMBOL    /* the definition is written last and is not marked with a role */
```

```rnl error
#R1(source) #R2(chain) #R3(chain) #R4(load) @AS $CHAIN_ROW    /* the same role is marked twice */
```

```rnl error
(0, 0) @PIN(terminal=1)    /* a pin cannot be written in the scene (13.4) */
```

```rnl error
#M1 @CURRENT("in")    /* CURRENT takes only terminals (15.2) */
```

```rnl error
&D<1:0>[1:0]    /* a name has at most one subscript (6.11) */
```

```rnl error
#XI0<1:0>:D<1:0>    /* one reference has at most one expansion (6.11) */
```

```rnl error
&D<_, 1>    /* a "<" directly after a name starts a subscript, and its content is not a subscript (3.5) */
```

```rnl error
%p<1, 2>    /* RNL names take no subscript; a point is written (1, 2) (6.11) */
```

### 4.5 Standardsätze

Ein Standardsatz ist eine Anweisung in der Standardform mit zwischen ihre Tokens eingefügtem Freitext, sodass sie sich wie ein Satz liest. Standardsätze werden in Schreibhilfen sowie von Formatierungswerkzeugen verwendet, die die Wörter zu Anweisungen hinzufügen oder daraus entfernen. Freitext beeinflusst die Bedeutung nicht (3.3): die Sätze sind nur Vorschläge, und Benutzer sowie Sprachmodelle dürfen sie schreiben, weglassen oder andere Wörter verwenden.

- **Fakten.** Eine Anweisung sagt aus, was in der Zeichnung gilt, nicht eine auszuführende Operation. Die Objekte stehen als Subjekt voran, das Schlüsselwort dient als Verb oder Substantiv, und die Parameter sind seine Werte. Es werden nur Verbindungswörter hinzugefügt, etwa `is`, `are`, `has`, `is at` und `there is`. Einige wenige Anweisungen können sich wie Operationen lesen, etwa `#M1 @SET-SYMBOL to $NMOS`.
- **Wörter folgen aus der Anweisung.** Die Wörter, die ein Werkzeug erzeugt, hängen ausschließlich vom Schlüsselwort, den Parametern, der Anzahl und den Arten der Objekte sowie der Ebene ab, in der die Anweisung steht, nicht von der Bedeutung der Schaltung. Beispielsweise folgt `is` oder `are` der Anzahl der Objekte; `the x of` wird geschrieben, wenn nur eine Achse angegeben ist; die Formulierung der Kandidaten einer Auswahl folgt dem Bereich und der ausgewählten Art (Anhang B). So werden Sätze erzeugt; Benutzer sind daran nicht gebunden.
- **Keine Wörter, die eine nicht vorhandene Bedeutung vortäuschen.** Wörter wie `must`, `required` und `only` werden nicht geschrieben, und ein Richtungswort wird nur geschrieben, wenn der Parameter diese Richtung hat. Leser glauben den Wörtern in einem Satz, und Schreibhilfen sollten sie davor warnen.
- **Erklärungen.** Leser kennen Ausrichtungscodes und Achsen möglicherweise nicht. Nach dem Schlüsselwort darf `, i.e., …` erklären, was der Code selbst bedeutet, wie in `#M2 @ORIENT("MX"), i.e., flipped upside down`.
- **ORIENT-RELATION und DISTANCE**, die zwei Slots haben, lesen `from A to B`.
- **Auswahlen** lesen sich stets als Wahl unter Kandidaten: `among <candidates>, @SELECT-…`. Ist der Bereich `#`, lauten die Kandidaten `the <kind> in # only`, und erreicht die Anweisung die unteren Ebenen, `the <kind> at each level in # and below`, was besagt, dass die Regel in jeder Ebene ausgewertet wird (10.1, 10.4). Lässt sich die Art nicht ermitteln, lautet der Satz `for X,`.
- **AS und ATTACH.** AS wird verwendet, wenn die Objekte zusammen das Gebilde ausmachen: `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N`. ATTACH wird verwendet, wenn das Gebilde zu den Objekten hinzugefügt wird: `$GROUND is @ATTACH to &vss` (9.4).
- **Pipes.** Das nächste Segment verweist auf das Ergebnis des vorherigen. Im Englischen steht `it` (nach AS, ATTACH oder einem einzelnen Objekt) oder `they` (nach einer Auswahl). Einem Namen in einer Pipe geht `named` voraus, wie in `… => named %pout`; es bleibt eine Benennung (4.2 Punkt 3).
- **Benennung.** Der Name einer einzelnen Anweisung steht am Anfang (4.3). Ist eine AS-Anweisung benannt, lässt das Englische `are` weg: `%dp = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N`.
- **Namen von Mengen.** Ein Name, der auf eine Menge verweist, sollte bei `$`-Namen auf `_SET` und bei `%`-Namen auf `_set` enden, wie `$GROUND_FILL_SET` und `%bias_set`; das Standardpaket hält sich daran.
- **Kein `each`.** Wörter wie `each` werden nicht geschrieben. Ob eine Menge als Ganzes oder Mitglied für Mitglied angewendet wird, hängt von der Rollenliste der Definition ab, und die Standardsätze geben nicht an, welches von beiden gilt.
- **Andere Sprachen.** Dieser Standard gibt die Sätze auf Englisch an. Sätze in einer anderen Sprache verwenden die Aliase eines Alias-Pakets jener Sprache, etwa `alias-zh` für Chinesisch, und folgen dessen Dokumentation (16.5); nach einem Satzzeichen jener Sprache ist vor `@` weiterhin ein Leerzeichen erforderlich (2.3).

Der Standardsatz jedes Schlüsselworts ist in Anhang B aufgeführt.

<!-- translated from en/ch05.md sha256:53b51cde1141 -->

## 5 Objekte und Anweisungen

### 5.1 Objekte

- Jede Anweisung mit einem Schlüsselwort erzeugt ein neues RNL-Objekt aus einem Handle und Fakten. Ein `{…}` erzeugt ebenfalls ein neues Objekt, eine Menge (5.2).
- Ein Verweis (`#M1`, `&vss`, `%x`, `$X`, ein Anschluss oder ein Unterobjekt) erzeugt kein Objekt; er greift ein bestehendes Handle auf. Mitgliedspositionen, die mehrere Mitglieder liefern, erzeugen eine neue Menge (5.2.3). Ein alleinstehendes `#M1` ist ebenfalls eine Anweisung: es liefert M1 ohne Fakten. Ein alleinstehendes `#` liefert diese Ebene ohne Fakten.
- Ein Handle ist das beschriebene Ding: eine Instanz, ein Netz oder ein Anschluss des Hosts oder die Ebene; oder ein von RNL erzeugter Punkt, eine Leitung, eine Menge, eine Anwendung, eine Bedingung usw. Ein Handle ist niemals ein anderes RNL-Objekt.
- Das Objekt einer Anweisung hat entweder ein neues Handle (die Anweisung erzeugt, etwa WIRE, Grafiken, `{…}`, Auswahlen, AS, ATTACH und Bedingungen) oder behält das Handle seiner Eingabe und fügt ihm neue Fakten hinzu (die Anweisung ändert, etwa PLACE, ORIENT, SET-SYMBOL, SET-STYLE, GROUP und CURRENT; ADD-TO und REMOVE-FROM behalten das Handle der Zielmenge). Die Signatur gibt an, welches zutrifft (5.3); WIRE erzeugt, wenn es ein Netz erhält, und ändert, wenn es eine Leitung erhält (13.1).
- Zwei Anweisungen gleichen Inhalts sind zwei Objekte, etwa zweimal `#M1 @PLACE(1, 2)`. Ihre Fakten widersprechen einander nicht und werden gemeinsam wirksam.
- Ein Fakt wird einmal festgeschrieben, an der Anweisung, die ihn erzeugt (5.4). Wenn die Ebene wirksam wird, werden alle Fakten zu einem Handle zusammengeführt; Widersprüche werden gemäß 1.5 Punkt 3 behandelt.
- Eigenschaften sind Fakten: Position, Ausrichtung, Symbol und Stil sind Fakten über das Objekt, und es gibt keine zu überschreibenden Eigenschaftsfelder. Wenn die Ebene wirksam wird, werden die Fakten zu einem Handle zu den endgültigen Werten zusammengeführt: x und y von PLACE als geometrische Fakten (5.8, 8.5), Symbole und Stile über die Darstellungswarteschlangen (8.3).
- Die Art eines Handles folgt aus seiner Herkunft: Host-Objekte erhalten sie vom Host, RNL-Objekte von der Anweisung, die sie erzeugt. Die Signatur gibt an, welche Arten jeder Slot annimmt (Anhang A); eine Beschreibung, die ein Objekt falscher Art erhält, schlägt fehl. Was beim Lesen des Texts geprüft werden kann, wird dann geprüft; was erst später bekannt ist (ob eine Anwendung ein Symbol hat, die Rollen einer Definition), wird geprüft, wenn die Ebene wirksam wird (7.1).

Arten von Objekten:

| Art | Erzeugt durch | Gezeichnet |
|---|---|---|
| Ebene | den Host; `#` | nein |
| Instanz | den Host; `#M1` | mit ihrem Symbol |
| Anschluss | den Host; `#M1:d`, `#:inp` (6.4). Anwendungen haben keine Anschlüsse | an den Pins, die ihn binden (9.2.4) |
| Pin | ein PIN des Symbols eines gezeichneten Objekts (9.2.4); `#X1::vdd_t`, wenn der Pin einen Namen hat | als Punkt des Symbols |
| Netz | den Host; `&vss` | nein; Leitungen und Beschriftungen sind eigene Objekte |
| Punkt | `(x, y)` | nein; ein Knotenpunkt ist `$JUNCTION` angewendet auf einen Punkt (13.2) |
| Leitung | WIRE mit einem Netz; eine Leitung, deren Netz-Slot eine Menge erhält, ist ein Bus. Mit einer Leitung schreibt WIRE sie neu (13.1) | ja |
| Beschriftung | ein an ein Objekt angehängter TEXT; mit einem Schlüssel ist sie das Unterobjekt `object::key` dieses Objekts (13.3) | ja |
| Bündel | BUNDLE (11.8) | einmal, anstelle seiner Mitglieder |
| Anmerkung | eine Grafik in der Szene (LINE, RECTANGLE, CIRCLE, ELLIPSE, ARC, BEZIER), ein an einen Punkt angehängter TEXT (13.4) oder ein durch ANNOTATION gezeichnetes Symbol (13.5) | ja |
| Anwendung | AS, ATTACH (9.4) | nur wenn sie ein Symbol hat; sie ist dann ein virtuelles Bauteil (Kapitel 12) |
| Bedingung | ALIGN, ORDER, DISTRIBUTE, DISTANCE, ORIENT-RELATION, TABLE (Kapitel 14) | nein |
| Menge | `{…}`, Auswahlen, UNION, das Ergebnis schrittweiser Generierung (5.2, Kapitel 10) | nicht selbst; ihre Mitglieder werden für sich gezeichnet |
| Definition | DEFINE-SYMBOL, DEFINE-STYLE, DEFINE (Kapitel 9) | nein; sie wirkt nur dort, wo sie verwendet wird |

Von diesen Arten haben Instanz, Anschluss, Pin, Netz, Punkt, Leitung, Beschriftung, Bündel, Anmerkung und Anwendung Artnamen (5.3), ebenso wie `text`, das jeden TEXT abdeckt, ob Beschriftung oder Anmerkung; Anhang A gibt an, wie Signaturen die übrigen benennen.

Es gibt außerdem Unterobjekte (6.5), deren Handle ein übergeordnetes Objekt und ein Name ist, sowie fehlgeschlagene Objekte (Kapitel 7).

### 5.2 Mengen

- `{…}` erzeugt ein Mengenobjekt. Seine Mitglieder sind geordnet. Eine Menge kann benannt werden (`%pair_set = {#M1 #M2}`), und andere Anweisungen können einer benannten Menge Mitglieder hinzufügen oder daraus entfernen (5.2.2).
- Eine Menge behält wiederholte Mitglieder und die Reihenfolge, in der sie geschrieben sind, etwa `{#M1 #M2 #M3 #M1}`. Ob zwei Mitglieder dasselbe Mitglied sind, entscheidet das Handle: zwei getrennt geschriebene `{…}` sind zwei Handles und nicht dasselbe Mitglied, auch wenn ihr Inhalt gleich ist.
- **Ein Verweis wird eine Stufe ausgebreitet**: ein Verweis in einer Menge, der auf eine Menge verweist, setzt deren Mitglieder an seine Stelle, sodass `{%a_set %b_set}` die Mitglieder von a_set gefolgt von denen von b_set ist. Dasselbe gilt für eine Anweisung, deren Ergebnis eine Menge ist, wie eine Auswahl, und für die Liste von Verweisen, zu der ein Subskript expandiert, wie `&D<7:0>` (6.11). Um Duplikate zu entfernen, schreibt man `%a_set %b_set @UNION` (5.2.2).
- **Ein ausgeschriebenes `{…}` bleibt erhalten**: ein direkt in einer Menge geschriebenes `{…}` bleibt als ein Mitglied erhalten, sodass `{{%a_set} {%b_set}}` eine Menge aus zwei Mengen ist.
- **Lose Objekte werden ebenfalls ausgebreitet**: erhält ein Schlüsselwort mit einem Slot mehrere ohne geschweifte Klammern geschriebene Objekte, werden sie genau so zusammengefasst, als stünden sie in `{…}` (5.2.1 Punkt 3), sodass eine Menge darunter ausgebreitet wird. Nach `%bias_set = {#R1 #R2 #R3}` und einem Text `%lbl`:
  - `%bias_set %lbl @ALIGN("x")` ist `{%bias_set %lbl} @ALIGN("x")`;
  - eine Menge als einziges Objekt wird selbst verwendet: `%bias_set @ALIGN("x")` richtet R1, R2 und R3 aneinander aus.
- **Elemente**: eine Menge merkt sich auch ihre Elemente (items), wie sie geschrieben sind, der Reihe nach: ein darin geschriebenes Objekt, ein ausgeschriebenes `{…}` oder ein Verweis auf eine Menge. Wo dieser Standard Elemente zählt (die Mitgliederlisten von ALIGN, ORDER und DISTRIBUTE, 14.1, die Mitglieder von ASSERT-TOPOLOGY, 15.4, und das Aufteilen nach 5.2.1 Punkt 2), ist ein Objekt ein Element, und ein `{…}` oder ein Verweis auf eine Menge steht als Element für jedes seiner innersten Mitglieder an diesem Element; in ASSERT-TOPOLOGY steht es für eine aus ihnen gebildete Blackbox. ADD-TO fügt ein Element am Ende hinzu, und REMOVE-FROM entfernt die Objekte aus jedem Element (5.2.2); eine durch eine Auswahl, durch UNION oder durch schrittweise Generierung erzeugte Menge hat ein Element für jedes Mitglied, ebenso wie die Liste, zu der ein Subskript expandiert (6.11), und die Menge, die Mitgliedspositionen herausgreifen (5.2.3). Überall sonst bedeutet eine Menge ihre Mitglieder, eine Stufe ausgebreitet wie oben. So hat `{%bias_set %lbl} @ALIGN("x")` zwei Elemente, R1–R3 als erstes und den Text als zweites, und gibt jedem von R1, R2 und R3 das x des Texts (14.1).
- Der Inhalt einer Menge wird in der Phase berechnet, die sie verwendet, wie bei einer Auswahl: ihr anfänglicher Inhalt und die Auswahlen in den Anweisungen, die Mitglieder hinzufügen oder entfernen, werden auf dem Zustand vor Beginn dieser Phase ausgewertet, und die Hinzufügungen und Entfernungen werden in der Reihenfolge des Texts angewendet (ihre Position, 8.5). So sieht auch eine früher geschriebene Verwendung alle Hinzufügungen und Entfernungen (1.5 Punkt 1), und eine Menge kann sich zwischen Phasen unterscheiden: die Regeln, die Objekte generieren, sehen sie ohne die generierten Objekte, Stilregeln sehen sie mit ihnen (8.2).

  Hinweis: Der Inhalt wird nicht einmalig in einem einzigen Schritt berechnet, weil das Benennen einer Auswahl sonst ihr Ergebnis verändern würde: `%g_set = # @SELECT-USAGE $GROUND; %g_set @SET-STYLE $grey` würde die generierten Massen verfehlen, die dieselbe Auswahl ohne Namen findet; ein Name ist lediglich eine andere Art, auf ein Objekt zu verweisen (1.5 Punkt 5).

#### 5.2.1 Objekte und Slots

Die in einer Anweisung geschriebenen Objekte werden den Slots der Signatur wie folgt zugeordnet.

1. **Mit Rollen markiert**: ein mit einer Rolle markiertes Objekt geht als Ganzes in diese Rolle und wird nicht aufgeteilt. In einer Anweisung sind entweder alle Objekte markiert oder keines.
2. **Zuerst eine Stufe aufteilen**: wenn keine Rollen markiert sind und die gesamte Anweisung ein einziges Objekt hat, das eine Menge ist, wird diese Menge zuerst eine Stufe aufgeteilt, in ihre Elemente (5.2). TABLE bildet die Ausnahme: sein Slot nimmt die Menge wie geschrieben als äußere Liste der Tabelle (14.11). Eine Definition und das über eine Pipe übergebene Ergebnis zählen als Objekte. So ist `{#M1 #M2} @DISTANCE(…)` gleich `#M1 #M2 @DISTANCE(…)`, während `{#M1 #M2} @SET-SYMBOL $X` zwei Objekte hat (die Menge und die Definition) und nicht aufgeteilt wird.
3. **Zuordnung über die Signatur**:
   - Ein Schlüsselwort mit mehreren Slots (eine Definition nimmt ebenfalls einen Slot ein): die Anzahl der Objekte muss gleich der Anzahl der Slots sein, und sie werden der Reihe nach eins zu eins zugeordnet; andernfalls schlägt die Anweisung fehl. Bei AS und ATTACH dürfen die Mengenrollen am Ende der Rollenliste ohne Objekte bleiben (9.4).
   - Ein Schlüsselwort mit einem Slot (etwa PLACE, ORIENT, ALIGN, ORDER, DISTRIBUTE, GROUP, die Auswahlen außer SELECT-USAGE und SELECT-ANCESTOR-USAGE, die ebenfalls eine Definition annehmen (5.6), und die Definitionsschlüsselwörter, deren Slot der Rumpf ist): ein einzelnes Objekt geht unverändert in den Slot; mehrere Objekte werden zu einer Menge zusammengefasst, in der referenzierte Mengen eine Stufe ausgebreitet werden und jedes ausgeschriebene `{…}` als ein Mitglied erhalten bleibt. Sind die Objekte die Elemente der einzigen, nach Punkt 2 aufgeteilten Menge, erhält der Slot diese Menge selbst.
   - Wenn das einzige Objekt eine leere Menge ist:
     - Ein Slot, der ein einzelnes Objekt annimmt, verrichtet seine Arbeit nullmal: nichts geschieht und nichts schlägt fehl. Ein Name wird wie üblich an die leere Menge gebunden, und Anweisungen, die den Namen verwenden, tun nichts.
     - Eine für einen Slot für einzelne Objekte ausgeschriebene leere Menge liefert eine Warnung, wie `{} @PLACE(1, 2)`. Eine leere Menge aus einer Auswahl oder Regel liefert keine: eine Regel mit `recursive=true` wird auch in jeder darunterliegenden Ebene ausgewertet, und eine Ebene ohne Objekte dieser Art ist normal. Wählt eine vom Benutzer geschriebene Auswahl in keiner Ebene etwas aus, in der sie ausgewertet wird, darf eine Implementierung eine Warnung ausgeben; wahrscheinlich ist die Bedingung falsch.
     - Ein Slot, der eine Menge annimmt, akzeptiert eine leere Menge, etwa den leeren Rumpf von `@DEFINE(roles=["a"]) {}` oder eine Bedingung ohne Mitglieder, die trivial erfüllt ist. Ausnahmen sind BUNDLE und der Netz-Slot von WIRE: sie erzeugen aus der Menge ein neues Objekt, das aus nichts nicht erzeugt werden kann, sodass die Anweisung fehlschlägt (11.8, 13.1).
     - Eine Anweisung ohne jedes Objekt, wie ein alleinstehendes `@PLACE(1, 2)`, passt nicht zur Signatur und schlägt fehl.
4. **Eine Menge in einem Slot**:
   - Ein Slot, der ein einzelnes Objekt annimmt, verrichtet seine Arbeit einmal für jedes Mitglied. Jedes Mitglied ist eine eigene Beschreibung, und das Ergebnis ist die Menge der Ergebnisse. Ein Mitglied, das selbst eine Menge ist, wird erneut aufgeteilt und Mitglied für Mitglied abgearbeitet. Die einzige Ausnahme ist PRIORITY: erhält es eine mit GROUP markierte Menge, wirkt es auf die Gruppe selbst (11.5, 14.8).
   - Ein Slot, der eine Menge annimmt, nimmt sie als Ganzes, und nimmt ein einzelnes Objekt als Menge aus diesem einen Objekt: `#M3 @ROUTE-OBJECT` ist `{#M3} @ROUTE-OBJECT`.
   - Jeder der beiden Slots von DISTANCE nimmt ein Objekt oder eine Menge; eine Menge steht für jedes ihrer Mitglieder, und DISTANCE setzt jedes Mitglied des einen Slots mit jedem Mitglied des anderen in Beziehung, eine Bedingung für jedes Paar (14.5). Dies ist das einzige Schlüsselwort, dessen beide Slots Mengen erhalten dürfen.
   - In einer Anweisung darf höchstens ein Slot, der ein einzelnes Objekt annimmt, eine Menge mit mehr als einem Mitglied erhalten. Tun dies mehrere solcher Slots, schlägt die Anweisung fehl: das Paaren der Mitglieder der Reihe nach und das Bilden aller Kombinationen wären beide sinnvoll, sodass die Bedeutung nicht eindeutig ist.

| Geschrieben | Ergebnis |
|---|---|
| `{#M1 #M2} @SET-SYMBOL $X` | Zwei Objekte, nicht aufgeteilt. Der Instanz-Slot erhält eine Menge: M1 und M2 erhalten jeweils das Symbol |
| `#M1 #M2 @SET-SYMBOL $X` | Drei Objekte für zwei Slots: die Anweisung schlägt fehl |
| `{#M1 #M2} @DISTANCE(…)`, `%pair_set @DISTANCE(…)` | Das einzige Objekt ist eine Menge und wird aufgeteilt: von M1 zu M2 |
| `%a_set %b_set @DISTANCE(…)` | Zwei Objekte, jeweils eine Menge: von jedem Mitglied von a_set zu jedem Mitglied von b_set (14.5) |
| `{#M1 #M2} @ALIGN("y")`, `#M1 #M2 @ALIGN("y")` | ALIGN hat einen Slot, der {M1 M2} erhält |
| `%bias_set %lbl @ALIGN("x")`, mit `%bias_set = {#R1 #R2 #R3}` | Die losen Objekte werden zu `{%bias_set %lbl}` zusammengefasst: zwei Elemente, R1–R3 als erstes, der Text als zweites; R1, R2 und R3 erhalten jeweils das x des Texts (14.1) |
| `{%bias_set} %lbl @ALIGN("x")` | Dieselben zwei Elemente: R1, R2 und R3 erhalten jeweils das x des Texts |
| `{%bias_set} @ALIGN("x")`, `{{#M1 #M2 #M3}} @ALIGN("y")` | Das einzige Objekt ist eine Menge und wird eine Stufe in sein einziges Element aufgeteilt (Punkt 2); dieses Element ist eine Menge und geht unverändert in den Slot: richtet R1, R2 und R3 bzw. M1, M2 und M3 aneinander aus |
| `%pair_set @PLACE(10, 20)` | Der Slot erhält pair_set selbst und platziert sie Mitglied für Mitglied: beide Mitglieder bei (10, 20). Eine Menge hat keine eigene Position (14.1) |
| `%a_set %b_set @PRIORITY("disabled")`, beide Mengen mit GROUP | Die losen Objekte werden wie in `{…}` ausgebreitet: PRIORITY erhält die Mitglieder und schlägt auf ihnen fehl (falsche Art). Um zwei Gruppen abzuschalten, schreibt man je ein PRIORITY (11.5, 14.8) |
| `{#M1 #M2} @SET-SYMBOL $X => @ALIGN("y")` | Das Ergebnis der Pipe ist das einzige Objekt und geht in ALIGN: richtet M1 und M2 aneinander aus |
| `{#M1 #M2} @SET-SYMBOL $X => #M3 @DISTANCE(…)` | Zwei Objekte, nicht aufgeteilt. Der Slot `from` erhält die Menge: von M1 zu M3 und von M2 zu M3 (14.5) |
| `# @SELECT-NET(…) => $PULL_DOWN @ATTACH &vss` | Drei Objekte für drei Slots (zwei Rollen und die Definition). Die erste Rolle ist eine Einzelrolle, daher wird die Definition einmal für jedes ausgewählte Netz angewendet, jedes Mal mit vss in der zweiten Rolle |
| `# @SELECT-INSTANCE(type="nmos") => @ORIENT("MY")` | Einzeln gespiegelt; ohne NMOS nullmal ausgeführt, und nichts geschieht |
| `# @SELECT-INSTANCE(name="Q*") => @SET-SYMBOL $NPN` | Zwei Objekte, nicht aufgeteilt; ohne BJT nullmal ausgeführt, und nichts geschieht |
| `#R1 {#R2 #R3} #R4 @AS $CHAIN_ROW` | Vier Objekte für vier Slots (drei Rollen und die Definition); die Mengenrolle `chain` erhält {R2 R3} (9.4) |
| `&D<7:0> @WIRE {…}` | Zwei Objekte, nicht aufgeteilt. Der Netz-Slot nimmt die Liste der Netze als Ganzes: ein Bus (13.1) |
| `#XI0<3:0> @ALIGN("y")` | Das einzige Objekt ist die Liste, zu der ein Subskript expandiert, und sie wird aufgeteilt: richtet die vier Instanzen aneinander aus (6.11) |

- Eine Bedingung mit zu wenigen Mitgliedern ist trivial erfüllt und schlägt nicht fehl, etwa ein ALIGN mit einem Mitglied (Kapitel 14).
- Eine Menge hat keine eigene Position (14.1); ihr `::position` ist die Menge der Positionen ihrer Mitglieder (6.5). Eine Anwendung ohne Symbol in einer Bedingung steht für ihre Mitglieder (14.1). Um eine Gruppe von Objekten als eine Stelle zu anderen in Beziehung zu setzen, macht man sie zu Mitgliedern einer Anwendung, deren Rumpf sie an den eigenen Punkt der Anwendung bindet (9.4), und versieht `%u::position` mit Bedingungen; um eine Menge auf einer Seite anderer Objekte zu halten, übergibt man sie an DISTANCE (14.5).
- Die Rollen einer Definition sind Einzelrollen oder Mengenrollen (9.4). Eine in der Rollenliste `"{name}"` geschriebene Rolle ist eine Mengenrolle und nimmt eine Menge als Ganzes an; jede andere Rolle ist eine Einzelrolle, und erhält sie eine Menge, wird die Definition einmal für jedes Mitglied angewendet.

#### 5.2.2 Mitglieder hinzufügen und entfernen sowie Duplikate

- `X @ADD-TO S` hängt X an das Ende der Menge S an (die Mitglieder von X, wenn X eine Menge ist); `X @REMOVE-FROM S` entfernt jedes Vorkommen dieser Mitglieder aus S. Das Ziel steht zuletzt, sodass eine Auswahl per Pipe übergeben werden kann: `# @SELECT-NET(name="vss") => @ADD-TO %gnd_set`.
- Das Ziel muss eine Menge sein; es wird nicht in eine verpackt. Nach `%tail = #M5` schlägt `#M3 @ADD-TO %tail` fehl, weil M5 keine Menge ist.
- Hinzufügungen und Entfernungen werden in der Reihenfolge der Position angewendet (1.5 Punkt 2). Eine in einer Ebene ausgewertete Menge enthält nur die Mitglieder jener Ebene (10.3).
- `%a_set %b_set @UNION` erzeugt eine neue Menge ohne Duplikate in der Reihenfolge des ersten Auftretens; ob zwei Mitglieder gleich sind, entscheidet das Handle.
- Um eine neue Menge zu erzeugen und die alten unverändert zu lassen: die Verkettung ist `{%a_set %b_set}`; die Vereinigungsmenge `%a_set %b_set @UNION`; die Differenz `%c_set = {%a_set}; %b_set @REMOVE-FROM %c_set`.
- Eine Menge hängt von den Mengen ab, die in der sie erzeugenden Anweisung stehen (in ihren geschweiften Klammern, in UNION oder als Bereich einer Auswahl), sowie von den Mengen, die ADD-TO und REMOVE-FROM ihr hinzufügen oder aus ihr entfernen. Abhängigkeiten, die einen Zyklus bilden, sind ein Fehler, mit oder ohne Entfernung: betrachtet man die Anweisungen in der Reihenfolge ihrer Position (8.5), schlägt diejenige fehl, deren Abhängigkeit den Zyklus schließt, und ihre Diagnosemeldung führt den gesamten Zyklus auf; die übrigen werden wie üblich wirksam. So schlägt `%a_set @ADD-TO %b_set; %b_set @ADD-TO %a_set` bei der zweiten Anweisung fehl, und `%b_set = {%a_set}; %b_set @REMOVE-FROM %a_set`, dessen Entfernung kein eindeutiges Ergebnis hätte, schlägt bei der Entfernung fehl.

#### 5.2.3 Mitgliedspositionen

```rnl
%row_set = {#M1 #M2 #M3 #M4};
%row_set{1} @PLACE(0, 0);                                         /* M1 */
%row_set{1} %row_set{end} @DISTANCE("left-to-right", exact=12);   /* from M1 to M4 */
%row_set{2:end} @ALIGN("y")                                       /* M2, M3 and M4 */
```

- Ein Verweis auf eine Menge, auf den unmittelbar `{…}` folgt, greift Mitglieder der Menge über ihre Positionen (member places) heraus, wie `%a{1}`, `%a{end}`, `%a{end-1}` und `%a{2:end}`. Dies wird nach an Mengen gebundenen `%`- und `$`-Namen sowie nach dem Platzhalter einer Mengenrolle im Rumpf einer allgemeinen Definition geschrieben, wie `?ref{1}` (9.4).
- **Form**: innerhalb der geschweiften Klammern durch `,` getrennte Elemente; jedes Element ist `i`, `a:b` oder `a:b:s`, und die Formen dürfen gemischt werden wie bei Subskripten (6.11). Eine Position ist eine positive Ganzzahl, `end` oder `end-k`, wobei k eine vorzeichenlose Ganzzahl ist; s ist eine vorzeichenlose Ganzzahl von mindestens 1. Innerhalb der geschweiften Klammern dürfen Leerraum und Kommentare stehen.
- **Zählung**: Positionen zählen ab 1 in der Reihenfolge der Mitglieder der Menge nach allen ihren Hinzufügungen und Entfernungen (5.2), auch den nach dem Verweis geschriebenen, sodass `%a{2}` nach `%a = {#M1}; #M2 @ADD-TO %a` M2 ist; die Mitglieder werden wie in 5.2 beschrieben eine Stufe ausgebreitet, nicht nach Elementen gezählt: nach `%a = {%b_set %lbl}` ist `%a{2}` das zweite Mitglied von b_set, oder der Text, wenn b_set ein Mitglied hat. `end` ist das letzte Mitglied. Die Elemente werden in der geschriebenen Reihenfolge expandiert und zusammengefügt; Wiederholungen bleiben erhalten. `a:b` läuft von a bis b, rückwärts wenn a größer als b ist, und `a:b:s` in Schritten von s, ohne b zu überschreiten.
- **Eines oder eine Menge**: ein Element, das eine einzelne Position ist, liefert das Mitglied selbst. Alles andere liefert eine neue Menge der Mitglieder an diesen Positionen in jener Reihenfolge: ein neues Mengenobjekt, der einzige Fall, in dem ein Verweis ein Objekt erzeugt (5.1). Ein Mitglied, das selbst eine Menge ist (ein ausgeschriebenes `{…}`, 5.2), wird übernommen, wie es ist.
- **Jenseits der Mitglieder**: eine Position, die die Menge nicht hat, liefert nichts: eine einzelne solche Position liefert eine leere Menge, und in einer Liste wird sie weggelassen. Dies ist kein Fehler, da beim Schreiben der Anweisung möglicherweise nicht bekannt ist, wie viele Mitglieder eine Menge hat (wie bei einem ALIGN mit einem Mitglied, 14.2).
- **Nur Positionen**: die geschweiften Klammern unmittelbar nach einem Verweis enthalten nur Positionen. `%a{#b}` ist ein Syntaxfehler; um einen Verweis und danach eine Menge zu schreiben, trennt man sie mit Leerraum wie `%a {#b}` (3.5).
- **Fehler**:
  - Positionen eines Objekts, das keine Menge ist, etwa einer Instanz, schlagen fehl (falsche Art).
  - Eine Anwendung ist keine Menge: ihre Mitglieder werden über die Rolle angesprochen, wie `%dp1::left` (6.5), und Positionen nach einer Anwendung sind ein Fehler.
  - Positionen des Ergebnisses einer Auswahl sind ein Fehler, ob die Auswahl dort steht oder an den Namen gebunden ist, ebenso Positionen jeder Menge, die über ihre Elemente, UNION, ADD-TO oder REMOVE-FROM von einer Auswahl abhängt (10.4), wie `%a{1}` nach `%a = {%s}` mit `%s` als Auswahl: ihre Mitglieder sind erst bekannt, wenn die sie verwendende Phase sie auswertet (5.2, 8.2).

#### 5.2.4 Objekte, die für eine andere Art angegeben werden

Nimmt ein Slot eine Art an und erhält ein Objekt einer anderen Art, wird das Objekt nur so umgewandelt, wie es die Tabelle angibt; jede andere Nichtübereinstimmung ist ein Artfehler bereits beim Lesen des Texts (5.1, 7.1), was immer auch gemeint sein mag. Ein Objekt wird nur in das umgewandelt, wofür es ohne Wahlmöglichkeit steht: zur Zeichnung hin von einem Anschluss zu seinem Pin und von dort zu einem Punkt sowie von einem gezeichneten Objekt zu seiner Position; zur Schaltung hin von einem Pin zu dem von ihm gebundenen Anschluss sowie von einem Pin oder Anschluss zu seinem Netz. Für alles andere schreibt man das Unterobjekt (`::position`, 6.5) oder eine Auswahl (10.1).

| Gegeben \ Slot nimmt an | Punkt (Grafiken) | Punkt oder Pin (Leitungen) | Position (Mitglieder von Bedingungen) | Pin | Anschluss | Netz |
|---|---|---|---|---|---|---|
| Punkt | = | = | = | nein | nein | nein |
| Pin | 1 | = | 1 | = | 8 | 9 |
| Anschluss | 2 | 3 | 2 | 3 | = | 10 |
| Instanz, Bündel | nein | nein | 4 | nein | nein | nein |
| Anwendung | nein | nein | 5 | nein | nein | nein |
| Text | nein | nein | 6 | nein | nein | nein |
| mit Mittelpunkt geschriebene Grafik | nein | nein | 7 | nein | nein | nein |
| andere Grafik, Leitung | nein | nein | nein | nein | nein | nein |
| Netz | nein | nein | nein | nein | nein | = |

„=“ bedeutet dieselbe Art; „nein“ ist ein Artfehler; eine Zahl ist eine der Umwandlungen unten. Eine Umwandlung von oder zu einem Pin benötigt das Symbol des Objekts, wird also erst vorgenommen, sobald die Symbole zugewiesen sind (8.2); die Schritte davor sehen das Objekt wie geschrieben. Wo ein Slot oder eine Rolle ein einzelnes Objekt annimmt, lässt eine solche Umwandlung, die eine Menge ergibt, wie aus einem Bus-Pin, die Anweisung bei dieser Prüfung fehlschlagen (7.1): die Anweisung wurde mit einem Objekt gebildet und wird nachträglich nicht aufgeteilt. Eine Leitung, die einen Pin erhält, endet an diesem Pin, was sie verbindet (10.2, 13.1); sie übernimmt nicht bloß die Koordinaten des Pins.

1. **Pin zu Punkt oder Position**: die Position des Pins.
2. **Anschluss zu Punkt oder Position**: die Position seines Pins, wie in 3.
3. **Anschluss zu Pin**: der Pin, der den Anschluss im Symbol seines Objekts bindet (9.2.4). Gibt es keinen, schlägt die Anweisung fehl (9.2.4); gibt es mehrere, ist es der im Symbol zuletzt geschriebene. Bei einem Anschluss eines Bündels der Pin des Bündelsymbols (11.8).
4. **Instanz oder Bündel zu Position**: ihr Ursprung, `::position` (6.5). Eine Instanz wird nicht in einen Punkt einer Leitung oder Grafik umgewandelt: um eine Leitung am Ursprung enden zu lassen oder dort eine Grafik zu zeichnen, schreibt man `#M1::position`.
5. **Anwendung zu Position**: ihr eigener Punkt (14.1). Eine Anwendung wird nicht in einen Punkt einer Leitung oder Grafik umgewandelt: um eine Leitung an ihrem Punkt enden zu lassen oder dort eine Grafik zu zeichnen, schreibt man `%u::position` (6.5).
6. **Text zu Position**: die Position des Texts selbst (13.3).
7. **Mit Mittelpunkt geschriebene Grafik zu Position**: der Mittelpunkt (13.4). Eine mit zwei Punkten geschriebene Grafik, LINE, ARC und BEZIER haben mehrere Punkte und werden nicht umgewandelt.
8. **Pin zu Anschluss**: der Anschluss, den der Pin bindet (9.2.4); bei einem Bus-Pin die Menge seiner Anschlüsse. Ein Pin, der keinen Anschluss bindet, etwa ein Pin einer Anwendung oder ein nur gezeichneter Pin, hat keinen, und die Anweisung schlägt fehl.
9. **Pin zu Netz**: sein `::net` (6.5), das Netz des Anschlusses, den er bindet, oder das Netz, das die von ihm gebundene Rolle füllt; bei einem Bus-Pin die Menge dieser Netze. Ein Pin, der nichts bindet, hat keines, und die Anweisung schlägt fehl.
10. **Anschluss zu Netz**: sein `::net` (6.5); bei einem Anschluss eines Bündels, dessen Mitglieder auf verschiedenen Netzen liegen, die Menge dieser Netze (11.8).

- PLACE nimmt nur die in 11.1 aufgeführten Objekte an: ein Anschluss oder Pin wird nur mit seinem Objekt bewegt und nicht für sich platziert (14.1).
- Eine Menge wird nicht umgewandelt: ein Slot, der ein einzelnes Objekt annimmt, verrichtet seine Arbeit für jedes Mitglied (5.2.1), und in Bedingungen steht eine Stelle, die eine Menge hält, für jedes ihrer Mitglieder (14.1).
- Der leere Knoten, `&` allein (6.3), wird ebenfalls nicht umgewandelt: eine Rolle der Art `"net"` und der Netz-Slot von WIRE nehmen ihn, wie er ist, und jede andere Stelle, die ein Netz annimmt, nimmt ihn als leere Menge von Netzen.
- Ein Netz wird in nichts umgewandelt: es hat keine Position und keinen Pin und steht für keinen einzelnen Anschluss. Ein Objekt wird auch nicht in mehrere Objekte einer anderen Art umgewandelt: eine Instanz wird nicht zu ihren Anschlüssen. Dafür schreibt man eine Auswahl, etwa `#M1 @SELECT-TERMINAL` oder `&en @SELECT-PIN` (10.1).
- Keine Umwandlungen und anderwärts behandelt: woran ein TEXT angehängt wird (13.3), wie der Bereich einer Auswahl zur ausgewählten Art führt (10.1), und WIRE mit einer Leitung (13.1).

### 5.3 Anweisungen und Signaturen

- Eine Anweisung besteht aus höchstens einem Schlüsselwort, einigen Objekten und einer Parameterliste.
- Jedes Schlüsselwort hat eine Signatur, die festlegt:
  - die Anzahl der Objekte sowie den Namen jedes Slots und die Arten von Objekten, die er annimmt. Bei AS und ATTACH ist die Anzahl die Länge der Rollenliste der Definition plus eins für die Definition selbst (bei AS zuletzt geschrieben, bei ATTACH zuerst), und die Arten sind die für die Rollen deklarierten (9.4);
  - die Parameter: ihre Namen, Typen, ob sie erforderlich sind, ihre zulässigen Werte sowie die Reihenfolge, in der sie nach Position angegeben werden dürfen;
  - die Ausgabe: ein neues Objekt, das Handle der Eingabe, ein Unterobjekt, eine Definition oder kein Handle; WIRE liefert eine neue Leitung, wenn es ein Netz erhält, und die übergebene Leitung, wenn es eine Leitung erhält (13.1);
  - wie jede Eingabe verarbeitet wird (5.4) und ob sie gegenüber Fehlschlägen strikt oder tolerant ist (7.3);
  - wo das Schlüsselwort geschrieben werden darf (siehe die Wortliste unten).
- Die Anzahl der Objekte ist fest; eine veränderliche Anzahl von Objekten wird als Menge übergeben. Es gibt keine optionalen Slots, mit folgenden Ausnahmen: eine nicht gefüllte Mengenrolle von AS und ATTACH ist eine leere Menge (9.4); die Definition von SELECT-USAGE darf weggelassen werden (10.1); der Rumpf von USE-PACKAGE darf weggelassen werden (16.2). Eine Definition belegt ebenfalls einen Slot.
- Objekte werden Slots nach Rolle oder nach Reihenfolge zugeordnet, alle markiert oder keines; eine Rolle darf nur einmal markiert werden. Das über eine Pipe übergebene Objekt ist das erste Objekt; AS und ATTACH ordnen es nach Rolle zu, und es belegt nicht den Slot der Definition (5.5). Wie Objekte Slots zugeordnet werden, steht in 5.2.1.
- Die in einer Anweisung geschriebenen Objekte dürfen unabhängig vom Schlüsselwort mit Rollen markiert werden. Der Rollenname ist der Slot-Name in der Signatur: `#M5:3(to) #M1:3(from) @DISTANCE("left-to-right", min=1)` entspricht `#M1:3 #M5:3 @DISTANCE("left-to-right", min=1)` in Reihenfolge. Ein Rollenname, der kein Slot-Name des Schlüsselworts ist, lässt die Anweisung fehlschlagen; er wird nicht ignoriert (Prinzipien 2.3, 2.8). Die Objekte eines Schlüsselworts mit einem Slot dürfen ebenfalls markiert werden, doch die Markierung fügt nichts hinzu.
- Die Parameter von AS und ATTACH sind die der Parameterliste der Definition (9.4), zuzüglich eines Standardparameters, `priority` (14.8); kein Parameter einer Definition darf `priority` heißen.
- **Standardwortliste**: jede Sprachversion veröffentlicht eine maschinenlesbare Liste ihrer Standardwörter, die Teil des Standards ist. Was jedes Schlüsselwort annimmt und welche Parameter es hat, steht ausschließlich in der Liste; der Haupttext gibt ihre Bedeutung an. Anhang A wird aus der Liste erzeugt, und ein Widerspruch zwischen beiden ist ein Erratum. Die Prüfungen eines Interpreters, Dokumentation, Vervollständigung in Editoren, das Grundgerüst von Konformitätstests und Schreibreferenzen für LLMs werden aus der Liste erzeugt. Für Version 0.4 wird die Liste als `rnl-words-0.4.json` veröffentlicht.
  - Die Liste enthält alle Standardwörter: die Schlüsselwörter mit ihren Signaturen, Slot-Namen, Parameternamen und -werten, die Felder des Eintragskopfs, Artnamen, Unterobjektnamen, die Attributnamen, die `${…}` liest, Stileigenschaften sowie `true` und `false`. Aliase (16.5) werden für Wörter der Liste deklariert, und Versionsprüfungen sowie Vorschläge des nächstliegenden Namens schlagen Wörter darin nach.
  - Die Liste ist eine JSON-Datei, und das JSON Schema `rnl-words-0.4.schema.json` beschreibt die Struktur der Liste selbst. Werkzeuge und die Betreuer des Standards lesen und schreiben sie, nicht diejenigen, die Schaltungen beschreiben. Jedes Wort trägt `since`, die Version, die es hinzugefügt hat; reservierte Wörter sind markiert (Version 0.4 reserviert keine Schlüsselwörter). Unter `removed` nennt die Liste die Standardwörter früherer Versionen, die diese Version nicht mehr besitzt, samt dem Abschnitt, der den Ersatz angibt, sodass eine Diagnosemeldung darauf hinweisen kann. Ein Wort, das in der von einer Datei deklarierten Version nicht vorkommt, wird als unbekanntes Wort behandelt, und die Diagnosemeldung gibt an, welche Version benötigt wird (16.1).
  - Die Liste hält für jedes Schlüsselwort fest:
    - jeden Slot: seinen Namen, die Arten, die er annimmt, ob er ein einzelnes Objekt oder eine Menge annimmt (5.2.1), und wie er verarbeitet wird (5.4), was entscheidet, ob er strikt oder tolerant ist (7.3); den Slot der Definition; bei AS und ATTACH folgen die Slots der Rollenliste der Definition;
    - jeden Parameter: seinen Namen; seinen Typ (Zahl, Zeichenkette, Wahrheitswert oder Array mit dem Typ seiner Elemente; bei festen Werten die Werte); ob er erforderlich ist; seinen Standardwert; die Reihenfolge der Parameter, die nach Position angegeben werden dürfen (die übrigen werden nur mit Namen angegeben); sowie Bedingungen an Kombinationen, etwa mindestens eines von `x` und `y` bei PLACE oder nur eine der Formen (Mittelpunkt- oder Eckenform) bei RECTANGLE;
    - die Ausgabe: ein neues Objekt und seine Art, das weitergegebene Eingabeobjekt, das weitergegebene Ziel (ADD-TO), eine Definition, ein Unterobjekt (TEXT mit `key=`) oder keines; bei WIRE auch die Art des Objekts, das es umschreibt, wenn es eines erhält (13.1);
    - wo es geschrieben werden darf: in einer Ebene; nur in einem Symbolrumpf (PIN); nur im Kopfeintrag (etwa VERSION, ALIAS, DEVICE); nur im Kopfeintrag einer Paketdatei (PACKAGE);
    - wie seine Fakten überschrieben werden (5.8): nach Schlüssel mit dem Schlüssel; als geometrische Fakten (8.5); über die Darstellungswarteschlangen (8.3); oder aufsummiert;
    - seine Standardform und seinen Standardsatz (4.3, 4.5, Anhang B);
    - eine einzeilige Beschreibung und den Abschnitt des Haupttexts.
  - Nicht in der Liste: die Bedeutung (welche Fakten ein Schlüsselwort erzeugt, in welcher Phase, wie sie in Konflikt geraten) steht im Haupttext. Die Signatur einer allgemeinen Definition in einem Paket ist ihre Rollenliste und Parameterliste (9.4).

  <!-- JSON excerpt: added by the integrator -->

- **Slot-Namen eingebauter Schlüsselwörter**: jeder Slot hat einen Namen, nämlich den Rollennamen, der markiert werden darf. Slot-Namen sind Standardwörter und werden wie Schlüsselwörter versioniert: einen umzubenennen ist eine inkompatible Änderung; die Slots eines Schlüsselworts haben unterschiedliche Namen. Auch der Slot einer Definition hat einen Namen, der in der Liste und in Diagnosemeldungen verwendet wird, aber nicht markiert werden kann (4.2 Punkt 7). Es gibt einen einheitlichen Satz von Artnamen, der sowohl in Signaturen als auch in den Rollendeklarationen von Definitionen verwendet wird (9.4): `instance`, `terminal`, `pin`, `net`, `point`, `wire`, `usage`, `bundle`, `text`, `annotation` und `label` (Anhang A).

### 5.4 Arten der Eingabeverarbeitung

| Art | Schlüsselwörter | Fakten der Eingabe | Fehlschläge (Standard) | Namen |
|---|---|---|---|---|
| absorbieren | die meisten Schlüsselwörter | an die Ausgabe weitergeführt | strikt | Namen in einer verschachtelten Menge gelten nur in jener Menge |
| einschließen | DEFINE, DEFINE-SYMBOL, DEFINE-STYLE | werden Inhalt der Definition und werden nicht weitergegeben | tolerant | lokal; bei jeder Anwendung umbenannt, sodass jede Anwendung ihre eigenen hat (9.4) |
| festschreiben | der Eintrag, `@RNL` | der Reihe nach in die Ebene eingefügt | tolerant | in der Ebene veröffentlicht und von allen ihren Einträgen geteilt; dürfen vor ihrer Bindung verwendet werden |

- Was ein Mitglied hervorbringt, sind alle Fakten, die von seinen Segmenten und den darin verschachtelten Mengen erzeugt werden, nicht nur das Handle an seinem Ende. Sie werden mit dem obersten Mitglied festgeschrieben, das sie enthält.
- Jede Anweisung eines Eintrags wird festgeschrieben, ob benannt oder nicht. Was sie dann bewirkt, hängt vom Objekt ab: eine Beschreibung in der Szene wird gezeichnet oder nimmt am Lösen teil; eine Definition wird verfügbar und zeichnet von sich aus nichts; Verweise sowie nur aus Verweisen bestehende Mengen haben keine Wirkung. Die Anweisungen in einem einfachen `{…}` werden mit dem Mitglied festgeschrieben, das es enthält, sodass die Linie von `{ @LINE {…}}` gezeichnet wird (13.4).
- Die Anweisungen im Rumpf einer Definition werden nicht festgeschrieben; sie werden Inhalt der Definition. Eine allgemeine Definition wird erst expandiert, wenn sie angewendet wird (9.4).
- Ein USE-PACKAGE mit Rumpf verarbeitet den Rumpf ebenfalls durch Einschließen: die Anweisungen des Rumpfes sind Inhalt des Pakets und werden nicht in die Ebene festgeschrieben. Wie die Namen eines Pakets gesehen werden, steht in 16.2.

### 5.5 Pipes und Verschachtelung

- `a => @K(…)` macht das Ergebnis von `a` zum ersten Objekt von K; AS und ATTACH legen es in die erste Rolle, nicht in den Slot der Definition. Ein Ergebnis, das eine Menge ist, bleibt ein einzelnes Objekt (5.2.1). Das Segment nach `=>` nimmt dieses Objekt ausschließlich nach Position und kann es nicht mit einer Rolle markieren.
- Eine Pipe wird nicht Schritt für Schritt ausgeführt. Sie expandiert zu mehreren durch Handles verknüpften Beschreibungen: jedes Segment beschreibt das Handle, das aus dem vorherigen Segment hervorgeht, und ein Segment, das Mitglied für Mitglied arbeitet, expandiert zu parallelen Beschreibungen (1.5 Punkt 1).
- **Die Reihenfolge der Segmente.** Jedes Segment hat eine eigene Stelle, und ein späteres Segment gilt überall dort als später geschrieben als ein früheres, wo die Stelle im Text zählt (1.5 Punkt 2): bei der Überschreibung nach Schlüssel (8.4), in den Darstellungswarteschlangen (8.3), in der Auflösungsreihenfolge (8.5), in der Zeichenreihenfolge (11.6) und bei den Neuschreibungen einer Leitung (13.1). So wird M1 in `#M1 @SET-SYMBOL $A => @SET-SYMBOL $B` mit `$B` gezeichnet.
- **Ein Fehlschlag erreicht nur die Beschreibungen, die davon abhängen.** Eine Beschreibung schlägt nur aus einem von zwei Gründen fehl: sie kann nicht verstanden werden oder widerspricht sich selbst; oder ein Handle, das sie verwendet, hätte von einer fehlgeschlagenden Beschreibung erzeugt werden sollen.
  - In `$GROUND @ATTACH &vss => @PLACE("20", 38) => @ORIENT("R90") => %g1` schlägt nur PLACE fehl (eine Zeichenkette, wo PLACE eine Zahl annimmt, 5.3); die Masse wird dennoch erzeugt, und ORIENT wird dennoch wirksam.
  - In `#XI0<3:0> @BUNDLE(bad=1) => @PLACE(10, 20) => %s` wird das Bündel nicht erzeugt (BUNDLE hat keinen Parameter `bad`), sodass PLACE kein Objekt zu beschreiben hat und ebenfalls fehlschlägt.
- Was ein Name bindet: `%x =` am Anfang bindet das letzte Handle der gesamten Pipe; `=> %x` bindet das Ergebnis des Segments links davon, also das letzte Handle, wenn es am Ende steht, und das Ergebnis jenes Segments, wenn es in der Mitte steht, etwa `#M1 @AS $A => %a => @AS $B => %b`. PLACE, ORIENT, SET-SYMBOL, SET-STYLE und GROUP behalten das Handle ihrer Eingabe, sodass in `$GROUND @ATTACH &vss => @PLACE(24, 38) => %g0` der Name `%g0` die Anwendung dieses Anwendens ist, also diese Masse. AS und ATTACH erzeugen neue Handles: das zweite Segment von `#M1 @AS $A => @AS $B` wendet `$B` auf die vom ersten Segment erzeugte Anwendung an (Verschachtelung), nicht auf M1. Um zwei Definitionen auf M1 anzuwenden, schreibt man zwei Anweisungen. Wenn das Segment links eines Namens fehlschlägt: hätte es das Handle seiner Eingabe behalten, verweist der Name weiterhin auf dieses Handle (`%g1` oben ist die Masse); hätte es ein neues Objekt erzeugt, verweist der Name auf das fehlgeschlagene Objekt (`%s` oben).
- Ein Objekt darf als Menge geschrieben werden, die Anweisungen enthält (Verschachtelung). In einer verschachtelten Menge gebundene Namen gelten nur in jener Menge. Jede verschachtelte Anweisung ist eine eigene Beschreibung: in `{%a = #M1 @PLACE(1, 2); #M9} @ALIGN("y")` schlägt ALIGN fehl, wenn der Host kein M9 hat, während die Koordinaten von M1 dennoch wirksam werden.
- Die Standardform schreibt Anweisungen flach: ein Objekt benennen und dann in anderen Anweisungen darauf verweisen. Eine einzelne Anweisung wird vorn mit `%x =` benannt, eine Pipe mit `=> %x` (4.3).

### 5.6 Definitionen als Objekte

Ein Schlüsselwort, das eine Definition verwendet, nimmt sie als eines seiner Objekte, und das Schlüsselwort legt fest, welcher Slot sie aufnimmt: bei ATTACH ist sie das erste geschriebene Objekt; bei SET-SYMBOL, SET-STYLE, AS, SELECT-USAGE und SELECT-ANCESTOR-USAGE ist sie das letzte Objekt, und SELECT-USAGE darf sie weglassen (10.1).

```rnl
%ax = (_, _);
%w13 = &x @WIRE {#M1:1 #M3:1};
#M1 @SET-SYMBOL $NMOS;
%w13 @SET-STYLE $signal;
#M1(left) #M2(right) %ax(axis) @AS $MIRRORED_LAYOUT;
$GROUND @ATTACH &vss;
# @SELECT-USAGE $GROUND => @SET-SYMBOL $GROUND_SYMBOL
```

- Eine Definition wird nicht mit einer Rolle markiert und zählt nicht für „alle markiert oder keines“. Sie ist ein Objekt und belegt einen Slot, sodass `{#M1 #M2} @SET-SYMBOL $X` zwei Objekte sind (5.2.1).
- Da das Schlüsselwort den Slot der Definition festlegt, kann eine Anweisung geparst werden, ohne zu wissen, ob ein `$`-Name auf eine Definition oder eine Menge verweist: in `$GROUND_STUB @ATTACH $GROUND_FILL_SET` ist das erste eine Definition und das zweite eine Menge.
- Eine Definition kann nicht per Pipe übergeben werden: das übergebene Objekt füllt den ersten Slot, und AS und ATTACH ordnen es nach Rolle außerhalb des Slots der Definition zu (5.5). Daher kann `$GROUND @ATTACH &vss` nicht `$GROUND => @ATTACH &vss` geschrieben werden.
- Eine Definition darf eine verschachtelte anonyme Definition sein: `#M1 @SET-SYMBOL { @DEFINE-SYMBOL {…}}`.
- Eine allgemeine Definition kann später oder in einer anderen Ebene stehen, daher werden die Anzahl der Objekte, die Rollennamen und die Parameter von AS und ATTACH geprüft, wenn die Ebene wirksam wird (7.1).

### 5.7 Handles und Namen

- Der Host liefert die Handles von Host-Objekten: Instanznamen und Netznamen. Ein Anschluss ist eine Instanz und eine Anschlussnummer, sodass `#M1:1` und `#M1:d` derselbe Anschluss sind.
- Bei dem, was RNL erzeugt, stammt das Handle von der Anweisung, die es erzeugt. Eine andere Anweisung kann nur über einen Namen darauf verweisen: der Name ist der einzige Weg, es zu finden, außer im Eintrag des Solvers, der auf ein Objekt ohne Namen über seine Nummer verweist (5.9). Ein Objekt ohne Namen kann andernfalls nur innerhalb seiner eigenen Anweisung verwendet werden (in einer Pipe oder durch Verschachtelung).
- Ein Name verweist niemals auf zwei Dinge: er wird in einer Ebene nur einmal gebunden (6.6).
- Das Handle eines Unterobjekts ist sein übergeordnetes Objekt und sein Name; es benötigt keinen eigenen Namen.
- Das Handle eines durch Expansion einer allgemeinen Definition erzeugten lokalen Objekts ist seine Anwendung und sein lokaler Name, von außen `%c1::mid` geschrieben (9.4). Die lokalen Objekte einer Anwendung ohne Namen werden in Diagnosemeldungen nur nach ihrer Position angezeigt; nur der Eintrag des Solvers kann über die Nummer der Anwendung auf sie verweisen, wie `%#5::mid` (5.9).
- Ein einzelnes Objekt einer generierten Menge hat keinen Namen und kann nicht für sich allein angesprochen werden: Objekte werden nicht über ihre Art und die von ihnen verknüpften Objekte identifiziert (es gibt keine natürlichen Schlüssel), und Mengen bieten keinen Zugriff per Index.

### 5.8 Schlüssel von Fakten

- Fakten mit einem Schlüssel werden nach Schlüssel überschrieben: beim selben Schlüssel wird der spätere Fakt wirksam; schlägt der spätere fehl, bleibt der frühere gültige Wert erhalten.
- Geometrische Fakten bilden die Ausnahme: x und y von PLACE (ein Schlüssel für jede Koordinate) und ORIENT (ein Schlüssel für jedes Objekt und einer für jeden Pin, 11.2). Zwei unterschiedliche Werte für denselben Schlüssel geraten in Konflikt und werden nach 8.5 aufgelöst: bei gleicher Priorität bleibt der spätere erhalten, was dasselbe Ergebnis wie Überschreiben nach Schlüssel liefert; bei unterschiedlichen Prioritäten bleibt der höhere erhalten. PLACE ändert nur die angegebenen Koordinaten; eine nie angegebene Koordinate bleibt frei.
- Das Erzeugen eines Objekts unter einem bereits gebundenen Namen ist kein Überschreiben nach Schlüssel: ein Name wird in einer Ebene nur einmal gebunden (6.6). Um ein von RNL erzeugtes Objekt zu ändern, schreibt man eine Anweisung, die es modifiziert, etwa `%g0 @PLACE(40, 38)`.
- Fakten ohne Schlüssel summieren sich auf: jene, die nicht in Konflikt stehen, gelten gemeinsam, und Konflikte werden nach 8.5 aufgelöst.
- SET-SYMBOL und SET-STYLE werden nicht nach Schlüssel überschrieben; sie werden über die Darstellungswarteschlangen wirksam (8.3).
- VERSION und PACKAGE werden nur einmal im Kopfeintrag geschrieben (16.1, 16.2), werden also niemals überschrieben: ein zweites ist ein Fehler, es schlägt fehl, und das erste wird wirksam.
- CURRENT hat einen Schlüssel für jeden Anschluss (15.2) und LAYER einen für jedes Objekt (11.6).
- Anwendungen haben keinen Schlüssel: eine mehrfach angewendete Definition erzeugt ebenso viele Anwendungen, die einander nicht überschreiben. Zwei identische Anwendungen erzeugen zwei Anwendungen, sodass zweimal geschriebenes `$GROUND @ATTACH &vss` zwei Massen zeichnet. Fakten, die modifizierende Anweisungen wie PLACE auf eine Anwendung schreiben, haben Schlüssel und folgen den obigen Regeln.
- Die Priorität eines Objekts ist ein Fakt mit einem Schlüssel; die Reihenfolge, in der Prioritäten einander überschreiben, steht in 14.8.
- Ein TEXT mit Schlüssel (`key=`) ist ein Text je angehängtem Objekt und Schlüssel, und ein späterer überschreibt ihn Parameter für Parameter (13.3). Ein TEXT ohne Schlüssel wird nicht nach Schlüssel überschrieben: jeder geschriebene ist ein weiterer Text (13.3).
- Eine Leitung hat keinen Schlüssel: zweimal geschrieben sind es zwei Leitungen (13.1). WIRE mit einer Leitung schreibt diese neu; das Neuschreiben hat einen Schlüssel, die Leitung, sodass ein späteres Neuschreiben ein früheres ersetzt (13.1).
- Bedingungen haben keinen Schlüssel. Um eine Bedingung zu ändern, schaltet man sie ab (`"disabled"`, 14.8) und schreibt eine neue, oder man bearbeitet den Text.

### 5.9 Nummern von Objekten ohne Namen

```spice fragment
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
.model nch nmos level=1
.subckt amp in out vdd vss
M1 out in vss vss nch
R1 vdd out 10k
* @RNL {
*   #M1 #R1 @ALIGN("x");
*   &out @WIRE {#M1:1 (_, _) #R1:2};         /* %#1: the wire; %#2: the free point */
*   {#M1 #R1} @TEXT("${name}")               /* %#3: the label of M1; %#4: that of R1 */
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig1:…", numbering="…") {
*   #M1 @PLACE(10, 0); #R1 @PLACE(10, 8);
*   %#2 @PLACE(12, 4);
*   %#1 @WIRE(mode="straight") {#M1:1 (12, 2) %#2 (12, 6) #R1:2};
*   %#3 @PLACE(13, 1); %#4 @PLACE(13, 9)
* }
.ends amp
```

- Der Eintrag des Solvers verweist per Nummer auf die Objekte, die über keinen Namen erreichbar sind, sodass er jede Entscheidung zurückschreiben kann (14.8). Eine Nummer wird `%#` gefolgt von einer vorzeichenlosen Ganzzahl geschrieben, wie `%#3`, und wie ein Name verwendet: `%#5::pin` ist ein Pin des Symbols der Anwendung `%#5`, und `%#5::mid` eines ihrer lokalen Objekte. Eine Nummer verweist lediglich auf ihr Objekt; sie wird nicht wie ein Name gebunden, sodass `%#3 = …` und `=> %#3` Syntaxfehler sind (4.1).
- **Welche Objekte.** Die Punkte, Leitungen, Beschriftungen, Bündel, Anmerkungen und Anwendungen, die keinen eigenen Namen haben und nicht über `::` von einem Objekt mit Namen oder Nummer erreichbar sind (6.5, 9.4). Ein nur in einer verschachtelten Menge gebundener Name (6.7) zählt nicht mit, da der Eintrag des Solvers ihn nicht verwenden kann. Für einen Eintrag des Solvers sind dies die Objekte in folgender Reihenfolge:
  1. jene, die von den Mitgliedern der von Verfassern stammenden Einträge der Ebene (`type="authored"`, 2.8) und in den Rümpfen der von diesen Mitgliedern erzeugten Anwendungen erstellt werden. Sie werden auf der Ebene so gezählt, wie der Solver sie sieht, ohne die Einträge mit `type="generated"` (2.8), und behalten ihre Nummern, wenn die Ebene mit diesen Einträgen interpretiert wird;
  2. jene, die von den Mitgliedern des Eintrags selbst und in den Rümpfen der von ihnen erzeugten Anwendungen erstellt werden. Sie werden auf dem Eintrag wie geschrieben gezählt, als ob jede Nummer darin aufgelöst werden könnte; ob die unten stehende Prüfsumme übereinstimmt, ändert sie nicht.

  Bedingungen und Mengen werden nicht nummeriert. Objekte, die Regeln schrittweise generieren (10.5), werden ebenfalls nicht nummeriert: der Solver schreibt sie als explizite Anweisungen, die unter Punkt 2 fallen. Jeder Eintrag des Solvers hat seine eigenen Nummern: Punkt 1 ist für alle gleich, und Punkt 2 zählt nur die eigenen Mitglieder des Eintrags.
- **Reihenfolge.** Die von Verfassern stammenden Einträge werden in der Reihenfolge von 2.6 genommen: der eigene Text der Ebene, dann die mit `scope` von außen in die Ebene geschriebenen Einträge. Innerhalb eines Mitglieds werden Objekte in der Reihenfolge des Tokens nummeriert, das sie erzeugt: dem `(` eines Punkts, dem Schlüsselwort der übrigen. Erzeugt ein Token ein Objekt für jedes Mitglied einer Menge (5.2.1), erhält jedes Objekt eine eigene Nummer in der Reihenfolge der Mitglieder der Menge. Die Objekte im Rumpf einer Anwendung werden unmittelbar nach der Anwendung nummeriert, in der Reihenfolge des Rumpfes, und so weiter hinab durch geschachtelte Anwendungen. Die Nummerierung beginnt bei 1.
- **Wo.** Nummern dürfen nur in Einträgen mit `type="generated"` stehen, die das Feld `numbering` und kein `scope` haben (2.6, 2.8). Überall sonst ist eine Nummer ein Fehler: ein von einem Verfasser stammender Eintrag verweist per Namen auf Objekte. Innerhalb seines Eintrags darf eine Nummer vor dem Mitglied verwendet werden, das ihr Objekt erzeugt, wie ein Name (6.7).
- **Prüfsumme.** `numbering` enthält eine Prüfsumme dafür, wie die nummerierten Objekte erzeugt werden. Für jede Nummer gibt es der Reihe nach eine Zeile:
  - die Tokens des Mitglieds des Eintrags oder des Definitionsrumpfes, das das Objekt erzeugt, mit all seinen Segmenten (4.1), ohne Freitext und Kommentare, getrennt durch ein Leerzeichen. Ein Token ist hier eine lexikalische Einheit: ein Schlüsselwort; ein Verweis mit seinen Anschlüssen, Unterobjekten und seinem Subskript ohne Leerraum; eine Zeichenkette oder Zahl wie geschrieben; ein Name; `_`; oder eines von `{ } ( ) [ ] , = ; =>` bzw. ein Teil von Mitgliedspositionen (`:`, `end`, `-`, 5.2.3). So ergibt `&out @WIRE {#M1:1 (_, _) #R1:2}` die Zeile `&out @WIRE { #M1:1 ( _ , _ ) #R1:2 }`;
  - für ein Objekt im Rumpf einer Anwendung steht die Zeile der Anwendung zuerst, auf dieselbe Weise gebildet, unabhängig davon, ob die Anwendung eine Nummer hat, dann ein Tabulator und die Tokens des Mitglieds des Rumpfes, wie die Definition es schreibt;
  - erzeugt das Token ein Objekt für jedes Mitglied einer Menge, folgen ein Tabulator und das Handle des Mitglieds, geschrieben als Verweis: eine Host-Instanz oder ein Netz durch seinen Namen; ein Anschluss durch seine Instanz und Anschlussnummer wie `#M1:1`; ein Pin durch sein Objekt und den Namen des Pins wie `#X1::vdd_t`, oder ohne Namen durch seinen Anschluss; die Position eines Objekts oder ein Text auf einem Schlüssel durch das Objekt gefolgt von `::position` oder `::` und dem Schlüssel; ein RNL-Objekt durch einen `%`-Namen, den die Ebene daran bindet, den in der Reihenfolge von 2.6 zuerst gebundenen bei mehreren, unabhängig davon, mit welchem Namen die Menge geschrieben wurde; ein Objekt ohne solchen Namen durch seine Nummer oder, als lokales Objekt einer Anwendung, durch den nach denselben Regeln gebildeten Verweis der Anwendung, `::` und seinen lokalen Namen: den im Rumpf der Definition zuerst gebundenen Namen bei mehreren, auch innerhalb eines Mitglieds (in `%p = (0, 0) => %q` ist es `p`), unabhängig vom Namen, mit dem die Menge geschrieben wurde, und über die zuerst angewendete Anwendung, wenn mehrere Anwendungen es als lokales Objekt haben. Ein Name wird unverändert geschrieben, wenn er nur aus ASCII-Buchstaben, Ziffern und `_` besteht, sonst in doppelten Anführungszeichen mit verdoppeltem `"` (3.6). Beispielsweise hat nach `%p = (0, 0); %q = %p; %s = {%q}` die durch `%s @TEXT("x")` erzeugte Anmerkung die Zeile `%s @TEXT ( "x" )`, einen Tabulator und `%p`, und die Prüfsumme ist `675df5171032726d`. Mit `$D = @DEFINE(roles=["x"]) { %p = (0, 0) => %q }; %u = #M1 @AS $D; %s = {%u::q}` endet die Zeile von `%s @TEXT("x")` mit einem Tabulator und `%u::p`, und die Prüfsumme ist `a8c9571193175903`.

  Die Prüfsumme sind die ersten 16 Hexadezimalziffern in Kleinbuchstaben des SHA-256 dieser Zeilen in UTF-8, jeweils abgeschlossen durch einen Zeilenvorschub (LF).
- Stimmt die Prüfsumme eines Eintrags nicht mit der Ebene überein, haben die Anweisungen des Eintrags, die eine Nummer verwenden, keine Wirkung, und falls es solche gibt, liefert der Eintrag einen Hinweis: die Lösung ist für die Objekte ohne Namen veraltet. Ein Eintrag, der keine Nummer verwendet, braucht kein `numbering`. Die übrigen Anweisungen des Eintrags werden wie üblich wirksam (Kapitel 17). Eine Nummer jenseits des letzten nummerierten Objekts ist ein Fehler.
- Im Beispiel hat die Prüfsumme je eine Zeile für sechs Nummern: das WIRE-Mitglied zweimal (für die Leitung, dann für den Punkt), das TEXT-Mitglied zweimal, gefolgt von `#M1` und `#R1`, sowie das neuschreibende WIRE noch zweimal, da die beiden darin als Koordinaten geschriebenen Punkte Objekte des Eintrags selbst sind mit den Nummern 5 und 6. Ein Viewer, der nicht löst, zeichnet den freien Punkt, den Pfad der Leitung und die beiden Beschriftungen so, wie der Solver sie gewählt hat.
- Schreibt man stattdessen `{#R1 #M1}`, werden die Beschriftungen 3 und 4 vertauscht: die Prüfsumme stimmt nicht mehr, und die Anweisungen mit Nummern haben keine Wirkung, bis die Ebene erneut gelöst wird, während M1 und R1 ihre Positionen behalten. Das Ändern des ALIGN oder das Verschieben von M1 lässt die Nummern unverändert.
- Hinweis: Die Prüfsumme erfasst nur, wie die nummerierten Objekte erzeugt werden. Das Verschieben eines Bauteils oder das Ändern einer Anweisung, die kein Objekt ohne Namen erzeugt, lässt die Nummern unverändert; das Einfügen, Entfernen, Ändern oder Umordnen eines Mitglieds, das solche Objekte erzeugt, macht sie veraltet, bis die Ebene erneut gelöst wird. Ein Eintrag des Solvers enthält normalerweise keine Anweisungen, die Mengen oder Definitionen ändern; Punkt 1 wird ohne solche Einträge in jedem Fall gezählt.

<!-- translated from en/ch06.md sha256:01f4618ca3e4 -->

## 6 Namen und Geltungsbereich

### 6.1 Hierarchie der Ebenen

- **Ebenen folgen dem Text.** Das RNL einer Ebene gilt für diese Ebene und für die Ebenen, die sie in ihrem Text enthält; welche Teilschaltung welche instanziiert, spielt keine Rolle.
  - Eine Datei enthält die Teilschaltungen, die sie definiert, und die Dateien, die sie einbindet.
  - Ein in einer Teilschaltung geschriebenes `.include` und eine in einer Teilschaltung geschachtelte `.subckt`-Definition (eine lokale Definition in SPICE3 und ngspice) liegen unterhalb dieser Teilschaltung.
- Von oben nach unten lautet die Kette der Ebenen bis zu einer Teilschaltung: die Pakete an der Wurzel (die Pakete von USE-PACKAGE in Kopfeinträgen, mit dem Standardpaket an erster Stelle, 16.3) → die Dateiebene der Hauptdatei → … (die eingebundenen Dateien, eine innerhalb der anderen) → die Datei, die die Teilschaltung definiert → die Teilschaltung selbst. Zum Beispiel:

  ```text
  packages at the root (such as the standard package)
  └─ file level of tb.sp (the top level)
     ├─ load                    a subcircuit defined in tb.sp
     ├─ file level of ota.sp    a file included by tb.sp
     │  ├─ ota                  ota instantiates cell
     │  └─ cell
     └─ file level of bias.sp
        └─ bias                 also instantiates cell, but neither the RNL of bias.sp nor that of bias reaches cell
  ```

- **Das RNL einer Teilschaltung gilt nur für die Teilschaltung.** Bei der Konfliktauflösung stehen ihre Anweisungen nach dem Inhalt der Dateiebenen und haben daher Vorrang davor (8.5); sie werden jedoch nicht an die Teilschaltungen weitergegeben, die sie instanziiert, außer an jene, die in ihr selbst definiert sind. Wenn amp cell instanziiert, erreichen die Definitionen und Regeln von amp cell nicht.
- **Eine eingebundene Datei beeinflusst die sie einbindende Ebene nicht.** Die Definitionen, Regeln und USE-PACKAGE ihrer Dateiebene gelten nur für ihre Teilschaltungen und für die Dateien, die sie ihrerseits einbindet. In der Netzliste gehören die außerhalb von Teilschaltungen geschriebenen Bauteile zur einbindenden Ebene, doch ihr RNL kann sie nicht ändern. So wird eine Datei niemals von den Dateien geändert, die sie einbindet.
- **Die Dateiebene einer eingebundenen Datei hat keine eigene Schaltung.** Nur die Dateiebene der Hauptdatei ist eine Schaltung. Host-Verweise (`#M1`, `&vss`) auf der Dateiebene einer eingebundenen Datei schlagen fehl (6.3); dort haben nur Definitionen, Regeln, USE-PACKAGE und `scope` Wirkung. Eine für sich geöffnete Datei ist die Hauptdatei.
- **Eine eingebundene Datei erbt von der sie einbindenden Ebene.** Der Hausstil der Hauptdatei (ihre Regeln und die Pakete ihrer USE-PACKAGE) vererbt sich nach unten in die Teilschaltungen von Bibliotheksdateien. So kann eine Bibliotheksdatei unter verschiedenen Hauptdateien oder beim Öffnen für sich unterschiedlich gezeichnet werden, während ihr Solver-Ergebnis nur einmal gespeichert wird (Kapitel 17). Um die Symbole einer Bibliothek frei von den Darstellungsregeln oberer Ebenen zu halten, verwendet man das Standardpaket in der Bibliotheksdatei oder Teilschaltung erneut mit USE-PACKAGE (16.2). Um nur die Version des Standardpakets festzulegen und dennoch den Hausstil von oben überschreiben zu lassen, schreibt man das USE-PACKAGE in den Kopfeintrag der Bibliotheksdatei (16.3). Regeln der Hauptdatei gelten nur für deren oberste Ebene, sofern ihre Auswahl nicht `recursive=true` angibt (10.1); die Regeln eines Hausstils, die Bibliotheksdateien erreichen sollen, geben dies an.
- **Verilog-A-Dateien.** Eine Verilog-A-Datei, die die Netzliste mit `ahdl_include` oder `.hdl` einbringt, steht dort, wo sie eingebracht wird, wie eine eingebundene Datei, und jedes ihrer Module ist eine Ebene unterhalb ihrer Dateiebene, die keine Schaltung und keinen Schaltplan hat und dem Modul nur sein Blocksymbol gibt (2.9).
- **Ein Schaltplan pro Teilschaltung.** Ein Viewer löst Teilschaltungen nicht auf: eine Instanz einer Teilschaltung wird stets mit ihrem Symbol gezeichnet, und das Innere der Teilschaltung wird gesondert gezeichnet und gelöst. Jede Teilschaltung hat genau eine Kette oberer Ebenen, sodass in einem Dokument jede Teilschaltung genau einen Schaltplan besitzt.
- **Pakete.** Die `$`-Definitionen eines Pakets werden in der Ebene wirksam, in der `@USE-PACKAGE` steht, sowie in den Ebenen darunter; seine Regeln gelten als in jener Ebene geschrieben und erreichen mit `recursive=true` die Ebenen darunter (16.2, 10.4). Die Pakete von USE-PACKAGE in einem Kopfeintrag stehen an der Wurzel, mit dem Standardpaket darunter, wenn es dort angegeben oder durch eine Werkzeugeinstellung hinzugefügt wird (16.3). Ein Include stellt eine Datei unterhalb der Ebene, und die Datei erbt von der Ebene; ein USE-PACKAGE stellt ein Paket über die Ebene, und die Ebene erbt vom Paket.
- **Regeln gelten standardmäßig für ihre eigene Ebene.** Eine in einer Ebene geschriebene Regel wird nur in dieser Ebene ausgewertet. Erreicht sie die Ebenen darunter — über `recursive=true` in einer in ihr geschriebenen Auswahl oder, falls sie keine enthält, über die von ihr verwendeten Mengen (10.1) —, wird sie auch einmal in jeder Ebene darunter ausgewertet, wobei jeweils nur jene Ebene betrachtet wird (10.4). Dies entspricht dem, was Prinzipien 2.5 von Regeln verlangt: eine Regel kann ihre Wirkung auf die aktuelle Ebene beschränken oder sie an die unteren Ebenen vererben lassen und gilt standardmäßig für ihre eigene Ebene, während `$`-Definitionen standardmäßig vererbt werden (6.8).

### 6.2 Präfixe

| Präfix | Verweist auf | Aufgelöst in |
|---|---|---|
| `#` | Eine Instanz des Hosts; `#` allein ist diese Ebene | Dem Host dieser Ebene |
| `&` | Ein Netz des Hosts; `&` allein ist der leere Knoten, gar kein Netz (6.3) | Dem Host dieser Ebene |
| `%` | Einen in dieser Ebene vergebenen Namen: eines Szenenobjekts, eines bestehenden Objekts wie `%tail = #M5`, oder einer nur in dieser Ebene verwendeten Definition | Dieser Ebene |
| `$` | Eine Definition oder eine Menge (6.6) | Der Ebene, in der sie gebunden ist, und den Ebenen darunter |
| `?` | Eine Rolle oder einen Parameter einer Definition; `?` allein ist in einem Symbolrumpf das mit dem Symbol gezeichnete Objekt (13.3), und im Rumpf einer allgemeinen Definition die expandierte Anwendung (9.4) | Dem Rumpf einer allgemeinen Definition oder eines Symbols |

`$X` und `%X` sind zwei verschiedene Namen.

### 6.3 Host-Verweise

- `#M1` und `&vss` werden ausschließlich im Host dieser Ebene aufgelöst, niemals in einer anderen Ebene (Prinzipien 2.5). Gleichnamige Instanzen in verschiedenen Teilschaltungen stehen in keiner Beziehung zueinander.
- Ein nicht gefundener Host-Verweis schlägt fehl (Prinzipien 2.4). Die Dateiebene einer eingebundenen Datei hat keine eigene Schaltung, sodass dort jeder Host-Verweis fehlschlägt (6.1).
- Auf einen Namen darf unmittelbar ein Subskript folgen, das für einen oder mehrere Host-Namen steht, etwa `&D<3>` und `#XI0<3:0>` (6.11).
- Der Host-Adapter entscheidet gemäß der Semantik des Hosts, welche Netze eine Ebene besitzt:
  - Lokale Netze: die Knoten, die die Netzlistenzeilen der Ebene verwenden, Ports eingeschlossen.
  - Globale Netze: `0` ist die globale Masse in jedem SPICE-Dialekt. Die Masse-Aliase, die ein Simulator selbst kennt (wie `gnd` in ngspice und `GND`, `GND!`, `!GND` sowie `GROUND` in HSPICE), werden vom Adapter des Dialekts mit `0` zusammengeführt; nach dem Zusammenführen sind `&gnd` und `&0` dasselbe Handle. Ein mit `.GLOBAL vdd` (`global vdd` in Spectre) deklarierter Name ist in jeder Teilschaltung derselbe Knoten. Ein auf `!` endender Name wie `vdd!` ist eine Konvention von Virtuoso-Schaltplänen; maßgeblich ist die Deklaration.
  - Ein globales Netz hat in jeder Ebene, die es verwendet, genau ein Handle: die Kopie in jener Ebene. Eine Ebene, deren Netzlistenzeilen es nicht verwenden, hat keine Kopie. Daher verweist `&0` auf die Kopie in der Ebene, in der es geschrieben steht: jeder Schaltplan zeichnet seine eigene, die Fakten gehören zu jener Ebene, und Beschreibungen in einer unteren Ebene können eine obere Ebene nicht ändern.
  - Um das globale Netz in jeder Ebene zu meinen, wählt man es in einer oberen Ebene nach Namen mit der Bedingung `global=true` aus (10.1).
- **Der leere Knoten.** Alleinstehendes `&` ohne Namen besagt, dass kein Netz vorhanden ist: was ihm übergeben wird, ist mit keinem elektrischen Knoten der Schaltung verbunden. Es wird dort geschrieben, wo ein Netz angenommen wird und keines existiert, etwa für die Pins virtueller Bauteile, die ein Blockschaltbild ohne Netzlistenzeilen zeichnen: `$INTEGRATOR @ATTACH &(in) &y(out)`, `& @WIRE {%sub1::y %int1::a}`.
  - Er ist kein Objekt: er hat kein Handle, kann nicht an einen Namen gebunden werden, kann kein Mitglied einer Menge sein und wird nie ausgewählt. Zwei Stellen, die `&` schreiben, haben nichts gemeinsam; ein Viewer, der einen Pin oder eine Leitung auf dem leeren Knoten hervorhebt, hebt nur dieses eine Element hervor.
  - Zwei Stellen nehmen ihn unverändert an: eine Rolle der Art `"net"` (9.4) und der Netz-Slot von WIRE (13.1). Ein Pin, der an eine von ihm gefüllte Rolle gebunden ist, liegt auf keinem Netz, und sein `::net` ist der leere Knoten (12.1); `${net}` und `${p}` von ihm sind der leere Text (13.6).
  - Überall sonst, wo ein Netz angenommen wird, zählt er als leere Menge von Netzen: ROUTE-NET verbindet nichts, ASSERT-NET gilt trivial, eine Auswahl mit ihm als Bereich wählt nichts aus, und ADD-TO fügt nichts hinzu (5.2.1). Wird er dort ausgeschrieben, liefert er eine Warnung, wie eine ausgeschriebene leere Menge; wird er über eine Rolle erreicht, wie im Rumpf einer Definition, liefert er keine. Ein Rumpf, der das Netz einer Rolle verwendet, braucht dafür also keinen eigenen Fall.
  - Er ist kein Netz des Hosts, sodass `&` gefolgt von einem Namen, den der Host nicht hat, weiterhin fehlschlägt wie zuvor.

### 6.4 Anschlüsse

- `object:terminal` ist ein Anschluss: `#M1:1`, `#M1:d`, `#XU1:inp` sowie ein Anschluss eines Bündels wie `%x:in` (11.8). `#:inp` ist der Port inp dieser Ebene. Nur Host-Instanzen, Bündel und die Ports einer Ebene haben Anschlüsse; eine Anwendung hat keine (12.1).
- Eine Zahl ist die Anschlussnummer des Hosts, ab 1 gezählt. Ein nur aus Ziffern geschriebener Anschluss ist eine Zahl, auch wenn er in Anführungszeichen steht (4.1), überall wo ein Anschluss benannt wird: in `:`-Verweisen, in `terminal=` von PIN, in `terminals=` von SET-SYMBOL und in der Bedingung `terminal` von Auswahlen. Daher wird ein nur aus Ziffern bestehender Portname wie in `.subckt amp 3 1 2` über seine Position angesprochen, und `#X1:1` ist der Port namens 3; ein nur aus Ziffern bestehender Name bei DEVICE ist ein Fehler (6.12).
- Ein Name ist ein Anschlussname des Hosts: ein durch `@DEVICE` im Kopfeintrag vergebener Name (6.12) oder ein Portname der Teilschaltung. Ein Name, der nicht unquotiert geschrieben werden kann (3.6), steht in Anführungszeichen, etwa `#XU1:"in+"`. Auf einen Anschluss- oder Portnamen darf unmittelbar ein Subskript folgen, wie `#X1:D<7:0>` und `#:D<7:0>` (6.11).
- Groß-/Kleinschreibung: die Portnamen von Teilschaltungen und die durch DEVICE vergebenen Anschlussnamen folgen den Regeln des Hosts: SPICE ignoriert Groß- und Kleinschreibung, sodass auch `#M1:D` funktioniert; Spectre tut dies nicht.
- Das `label=` eines Pins dient nur der Anzeige und kann nicht verwendet werden, um auf einen Anschluss zu verweisen: Anschlüsse gehören zur Netzliste, und ein Verweis darauf hängt nicht vom verwendeten Symbol ab.
- **Anschlüsse und Pins**: ein Anschluss gehört zur Netzliste; ein Pin gehört zum Symbol eines gezeichneten Objekts (9.2.4). Ein Anschluss wird an den Pins gezeichnet, die ihn binden, und wo ein Pin oder Punkt benötigt wird, steht ein Anschluss für seinen Pin (5.2.4). Ein Pin wird über seinen Anschluss geschrieben, wie `#M1:d`, oder, wenn er im Symbol einen Namen hat, über diesen Namen als Unterobjekt, wie `#X1::vdd_t` (6.5); eine Zahl ist niemals ein Pin-Name.
- Ein nicht gefundener Anschluss schlägt fehl. Ob ein Anschluss im Symbol einen Pin hat, wird nach Zuweisung der Symbole geprüft (7.1).
- Abgesehen von den Portnamen von Teilschaltungen werden Anschlussnamen durch DEVICE vergeben; 6.12 listet die Standardnamen der SPICE-Familie auf.
- Eine in der Netzliste nicht definierte Teilschaltung (wie das PDK-Bauteil in `XM1 … sky130_fd_pr__nfet_01v8`) hat nur dann Anschlussnamen, wenn der Benutzer sie mit DEVICE im Kopfeintrag vergibt (6.12) oder ein Paket verwendet, das DEVICE-Anweisungen enthält (16.2); andernfalls können nur Nummern verwendet werden.

### 6.5 Unterobjekte

- `object::name` ist ein Unterobjekt. Es ist immer ein Objekt, niemals ein Wert. Sein Handle ist das übergeordnete Objekt und der Name, sodass zwei Namen überall dort zwei Mitglieder sind, wo Mitglieder vor der Zuweisung der Symbole gezählt werden (5.2.4): `{#R1::p #R1::q} @UNION` hat zwei Mitglieder, auch wenn beide Namen sich als Namen desselben Pins erweisen.
- Die Unterobjekte sind:
  - Ein Schlüssel eines Texts: das Unterobjekt, das ein TEXT mit `key=` an dem Objekt erzeugt, an das er angehängt ist, etwa `#M1::wl`. Ein Text mit Schlüssel in einem Symbol ist ein Unterobjekt jedes mit dem Symbol gezeichneten Objekts (13.3). `::name` und `::value` sind keine Standardunterobjekte, sondern nur gebräuchliche Schlüssel (13.3).
  - `::position`: der Ursprung des Objekts. `#M1::position @PLACE(…)` ist `#M1 @PLACE(…)`. Bei einer Anwendung ist es ihr eigener Punkt (14.1), der Ursprung ihres Symbols, wenn sie eines hat (12.1). Bei einer Menge ist es die Menge der `::position` jedes Mitglieds, wobei verschachtelte Mengen nach innen verfolgt werden: `{%dp1 %dp2}::position` ist `{%dp1::position %dp2::position}`, die beiden eigenen Punkte, während `{%dp1 %dp2}` in einer Bedingung für die sechs Transistoren steht (14.1). Bei einem TEXT ist es die Position des Texts selbst (13.3); bei einem in Mittelpunktform geschriebenen RECTANGLE, CIRCLE oder ELLIPSE der Mittelpunkt (13.4).
  - `::net` eines Anschlusses: das Netz, auf dem er liegt, etwa `#M5:1::net`, das `&tail` ist. Zu den Anschlüssen eines Bündels siehe 11.8: liegen die Mitglieder auf verschiedenen Netzen, ist es die Menge dieser Netze. `::net` eines Pins: das Netz des Anschlusses, den er bindet, oder dessen, was die Rolle füllt, die er bindet (12.1); bei einem Bus-Pin die Menge dieser Netze; ein Pin, der nichts bindet, hat keines.
  - Ein benannter Pin des Symbols des Objekts: `#X1::vdd_t` ist der Pin namens vdd_t im Symbol von X1, `%g0::pin` der Pin namens pin im Symbol der Anwendung `%g0` (9.2.4).
  - Ein Rollenname einer Anwendung: was diese Rolle füllt, etwa `%dp1::left`, das M1 ist, und `%g0::p`, das das Netz vss ist. Bei einer Mengenrolle ist es jene Menge. Es ist jenes Objekt selbst und kein zweites: `{%dp1::left #M1} @UNION` hat ein Mitglied, und ein für `%dp1::left` geschriebenes PLACE hat den Schlüssel von M1.
  - Ein benanntes lokales Objekt im Rumpf einer Anwendung, wie `%c1::mid` (9.4).
- Die Standardunterobjektnamen sind reserviert: kein Rollenname, Parametername oder lokaler Name einer allgemeinen Definition und kein Schlüssel eines Texts darf mit einem von ihnen übereinstimmen (verglichen wie geschrieben, 3.6). Ein lokaler Name darf kein Rollenname sein, und der Schlüssel eines Texts darf kein Rollenname oder lokaler Name der Anwendung sein, an die er angehängt ist. Ein lokaler Name darf ein Parametername sein: ein Parameter ist ein Wert, und `::` nimmt keine Werte. Kein Parameter einer Definition darf `priority` heißen (14.8).
- **Der Name nach `::` wird in dieser Reihenfolge nachgeschlagen**: die Rollennamen und lokalen Namen der Anwendung selbst; deren `definition=`-Aliase; die Standardunterobjektnamen und ihre Aliase; die Namen der Pins des Symbols des Objekts; die Schlüssel von Texten (Aliase: 16.5). Ein Pin-Name, der gleich einem Rollennamen, einem lokalen Namen der Anwendung oder einem Standardunterobjektnamen ist, ist ein Fehler, sobald das Symbol dem Objekt zugewiesen wird; ebenso ist der Schlüssel eines Texts ein Fehler, der gleich einem Pin-Namen ist.
  - Schlüssel kommen zuletzt, sodass ein Schlüssel nicht so buchstabiert sein darf wie ein Name davor: ein Standardunterobjektname oder einer seiner in dieser Datei deklarierten Aliase (etwa `"位置"` von `alias-zh`), ein Rollenname oder lokaler Name der Anwendung oder ein `definition=`-Alias dieser Namen. Ein solcher Schlüssel ist ein Fehler wie die obigen Kollisionen (13.3).
- `::` greift nur auf Teile des Objekts selbst zu; das Auffinden von Objekten in der Szene anhand von Bedingungen geschieht immer über Auswahlschlüsselwörter. Daher wird „alle Anwendungen einer Definition“ als Auswahl geschrieben (10.1), nicht als Unterobjekt der Definition.
- Anschlüsse verwenden `:` und Unterobjekte `::`, damit in späteren Versionen hinzugefügte Unterobjektnamen nicht mit Portnamen einer Netzliste kollidieren können.
- **Unterobjektnamen**: es gibt zwei Standardunterobjektnamen, `position` und `net`, und beide sind reserviert. Alle anderen Unterobjektnamen sind dynamisch: Schlüssel von Texten, Rollennamen von Anwendungen, lokale Namen in Rümpfen von Definitionen und die Namen von Pins. In späteren Versionen hinzugefügte Standardunterobjektnamen gelten nach Version (16.1): in einer Datei einer älteren Version bleibt ein gleichnamiger Schlüssel gültig.

### 6.6 Benennung

- `%name = statement` und `$name = statement` benennen das Ergebnis einer Anweisung; in einer Pipe benennen `=> %name` und `=> $name` das Ergebnis des Segments links davon (5.5). Ein Name ist nur eine andere Art, auf ein Objekt zu verweisen, und erzeugt kein Objekt.
- `$name` bindet eine Definition oder eine Menge, einschließlich des Ergebnisses einer Auswahl. Eine in einer Ebene ausgewertete `$`-Menge enthält nur die Mitglieder jener Ebene (10.3). Andere Szenenobjekte gehören zu einer Ebene und können keinen `$`-Namen haben.
- `%name` kann jedes Objekt dieser Ebene binden: ein von der Anweisung erzeugtes Objekt, ein bestehendes Objekt (`%tail = #M5`), eine Menge (`%pair_set = {#M1 #M2}`) oder eine nur in dieser Ebene verwendete Definition.
- **Ein Name wird in einer Ebene nur einmal gebunden.** Ihn erneut an dasselbe Objekt zu binden, ist zulässig, etwa zweimal geschriebenes `%x = #M1`; ihn an ein anderes Objekt zu binden, ist ein Fehler, und der Name behält seine erste gültige Bindung. Was eine Bindung bindet, folgt 5.5. Ein Segment, das das Handle seiner Eingabe behält, etwa PLACE oder ORIENT, bindet dieses Handle auch dann, wenn das Segment fehlschlägt, und eine solche Bindung ist genau dann gültig, wenn dieses Handle gültig ist: ist die Eingabe selbst ein fehlgeschlagenes Objekt, wie nach einem fehlgeschlagenen BUNDLE, ist auch die Bindung nicht gültig (5.5). Ein Segment, das ein neues Objekt erzeugt, bindet es, und die Bindung ist gültig, wenn jenes Segment beim Einlesen und Erzeugen der Ebene nicht fehlschlägt (Schritte 1 bis 3 von 8.2); schlägt es später fehl, bei einer Prüfung nach der Zuweisung der Symbole (8.2), behält es den Namen, eine spätere Bindung übernimmt ihn nicht, und der Name verweist fortan auf das fehlgeschlagene Objekt. Nur wenn keine seiner Bindungen gültig ist, ist der Name an ein fehlgeschlagenes Objekt gebunden (7.2). Eine nicht gültige Bindung belegt den Namen nicht, sodass eine spätere Bindung an ein anderes Objekt nicht der Fehler des doppelten Bindens ist: nach dem fehlgeschlagenen Bündel aus 5.5 bindet `%s = #XI1` `%s` an XI1. Dies gilt gleichermaßen für Anweisungen, die Objekte, Definitionen, Mengen oder Ergebnisse von Auswahlen erzeugen.
- Überschreiben gilt für Fakten, nicht für Namen: um ein von RNL erzeugtes Objekt zu ändern, schreibt man eine Anweisung, die es modifiziert, etwa `%g0 @PLACE(40, 38)` (5.8).

### 6.7 Geltungsbereich von Namen

- Die durch `@RNL` festgeschriebenen Namen werden in der Ebene veröffentlicht: alle Einträge der Ebene teilen sie, und ein Name darf vor der Anweisung verwendet werden, die ihn bindet (Prinzipien 2.9).
- Ein in einer verschachtelten Menge gebundener Name gilt nur in jener Menge.
- Ein im Rumpf einer Definition gebundener Name ist lokal zu diesem Rumpf (6.9).

### 6.8 Vererbung und Verdeckung

- `$`-Definitionen werden in der Ebenenhierarchie nach unten vererbt (6.1), niemals nach oben: in die Teilschaltungen der Datei und in eingebundene Dateien. Eine Ebene sieht die `$`-Definitionen der von ihr eingebundenen Dateien nicht; um oberen Ebenen Definitionen bereitzustellen, erstellt man ein Paket und verwendet es mit USE-PACKAGE (16.2). Das Blocksymbol einer Teilschaltung ist die einzige Ausnahme, die nach oben wirkt (9.2.6).
- Ein in einer unteren Ebene unter demselben Namen wie eine geerbte Definition definiertes `$X` verdeckt die geerbte Definition in dieser Ebene und den Ebenen darunter und liefert einen Hinweis; die obere Ebene sieht weiterhin ihre eigene.
- Die `$`-Definitionen eines mit USE-PACKAGE in einer Ebene verwendeten Pakets zählen als eine Ebene oberhalb dieser Ebene (16.2). Verdecken sie Definitionen der oberen Ebenen oder der Pakete an der Wurzel, wird ein Hinweis gegeben wie bei jeder Verdeckung. Ein Hinweis wird auch gegeben, wenn ein in einem Kopfeintrag verwendetes Paket einen Namen des Standardpakets oder eines zuvor an der Wurzel platzierten Pakets definiert (16.3).
- `%`-Namen gelten nur in ihrer eigenen Ebene und werden nicht vererbt: nicht in die Teilschaltungen der Datei und nicht über Include hinweg. Die `%`-Definitionen eines Pakets sind private Helfer des Pakets.

### 6.9 Namen in Definitionsrümpfen

- Die Namen im Rumpf einer Definition werden dort aufgelöst, wo die Definition geschrieben steht, nicht erneut dort, wo sie angewendet wird; die einzige Ausnahme ist ein alleinstehendes `#` im Rumpf einer allgemeinen Definition (unten).
- Daher ändert das Verdecken einer Definition nur, worauf der Name verweist: die bereits innerhalb anderer Definitionen verwendeten Definitionen bleiben unverändert. Beispielsweise ändert das Verdecken von `$DIFFERENTIAL_PAIR_N` in einer unteren Ebene nicht das Differenzpaar im Rumpf einer in einer oberen Ebene geschriebenen Definition.
- Ein `%name` in einem Rumpf wird zuerst unter den lokalen Namen des Rumpfes nachgeschlagen, dann unter den Definitionen der Ebene. Auf die Szenenobjekte der Ebene (Punkte, Leitungen usw.) kann aus einem Rumpf nicht verwiesen werden; ein solcher Verweis ist ein Fehler. Eine Menge oder das Ergebnis einer Auswahl, die die Ebene mit `%` benennt, ist ebenfalls ein solches Objekt, gleichgültig welche Mitglieder sie bei der Auswertung hat: um eine mit Rümpfen zu teilen, benennt man sie mit `$`, worauf ein Rumpf verweisen darf und was in der Ebene jeder Anwendung ausgewertet wird (6.6, 10.3).
- **Notation in Rümpfen**: Rollen und Parameter werden `?name` geschrieben. Lokale Namen werden mit `%name =` (oder `=> %name` in einer Pipe) gebunden und als `%name` angesprochen. Von außen werden lokale Namen über die Anwendung erreicht (9.4).
- Ein Definitionsrumpf sieht den gerade definierten Namen nicht: in `$NMOS = @DEFINE-SYMBOL {$NMOS; …}` ist das `$NMOS` im Rumpf die Definition dieses Namens außerhalb (in einer oberen Ebene oder einem Paket). Auf diese Weise verdeckt und erweitert eine untere Ebene eine obere Definition; ohne eine solche äußere Definition ist dies ein Fehler.
- Host-Verweise in den Rümpfen von DEFINE-SYMBOL und DEFINE sind Fehler: `#M1`, `&vss`, `#:inp` und `#M1:d` dürfen dort nicht stehen, ebenso wenig ein alleinstehendes `#` in einem Symbolrumpf. Im Rumpf einer allgemeinen Definition bezeichnet ein alleinstehendes `#` die Ebene, in der die Anwendung steht, was erst bekannt ist, wenn die Definition angewendet wird (9.4); es steht dort als Bereich einer Auswahl. Ein Blocksymbol kann nicht aus einem Rumpf vergeben werden: `# @SET-SYMBOL` dort ist ein Fehler, da das Blocksymbol einer Ebene vor den Anwendungen der Ebene in die Symbol-Warteschlange geht und eine Anwendung erst danach generiert werden kann (8.3, 9.2.6). Namen in einem Rumpf werden dort aufgelöst, wo die Definition geschrieben ist, aber eine allgemeine Definition wird in anderen Ebenen und anderen Dateien angewendet, wo ein Host-Verweis noch in die Ebene der Definition zeigen würde, sodass seine Bedeutung nicht eindeutig wäre. Eine allgemeine Definition, die Host-Objekte benötigt, erhält sie über ihre Rollen.
- **Wann der Rumpf einer allgemeinen Definition geprüft wird**:
  - Einmal für jede Definition: alles, was nicht von den Argumenten abhängt, wird dann geprüft, einschließlich Schlüsselwörtern, Signaturen (Anzahl der Objekte, Parameternamen und -typen), `$`- und `%`-Namen (aufgelöst, wo die Definition geschrieben steht), Host-Verweisen, Kollisionen lokaler Namen (9.4) sowie ob mit einer Art deklarierte Rollen korrekt verwendet werden: ist die Rolle t als `"instance"` deklariert, ist `?t::net` im Rumpf ein Fehler. Signaturen werden beim Lesen des Texts geprüft (7.1); Namen können erst nach dem Einlesen der gesamten Ebene aufgelöst werden, daher erfolgen Prüfungen, die Namen benötigen, wenn die Ebene wirksam wird. Beides geschieht für jede Definition nur einmal, wie oft sie auch angewendet wird.
  - Für jede Anwendung: was von den Argumenten abhängt, etwa die Objekte, die Rollen ohne deklarierte Art erhalten, oder ob das Symbol einen Pin hat, wird bei jeder Anwendung geprüft und an der Anwendung gemeldet, unter Verweis auch auf die Anweisung in der allgemeinen Definition.
  - Umfang der Fehler: eine als fehlerhaft erkannte Anweisung des Rumpfes (einschließlich eines kollidierenden lokalen Namens) wird einmal bei der Definition gemeldet und aus jeder Expansion weggelassen; der Rest des Rumpfes wirkt wie üblich (der Rumpf ist tolerant, 9.1). Eine fehlerhafte Rollen- oder Parameterliste (wiederholte Namen, Kollisionen mit Standardunterobjektnamen, unbekannte Artnamen) lässt die gesamte allgemeine Definition fehlschlagen, und mit ihr jede Anwendung, die sie verwendet.
  - Auch eine niemals angewendete allgemeine Definition wird geprüft. Diese Prüfungen hängen nicht von Anwendungen ab, finden also Fehler auch in ungenutzten Definitionen eines Pakets.

### 6.10 Platzhalter

- `?name` erscheint nur im Rumpf einer allgemeinen Definition, und der Name muss in der Rollenliste oder der Parameterliste stehen; andernfalls ist es ein Fehler beim Lesen des Texts.
- Wo ein Objekt steht, ist es eine Rolle, wie `?t` oder `?t::net`; wo ein Literal steht, ist es ein Parameter, wie `min=?gap`, `[?w, 2]` und `(?x, 0)` (9.4). Eine Rolle, wo ein Literal steht, oder ein Parameter, wo ein Objekt steht, ist ein Fehler beim Lesen des Texts. Rollennamen und Parameternamen stehen in einer gemeinsamen Tabelle und dürfen sich nicht wiederholen.
- Ein Rollenplatzhalter darf einen Pfad haben, wie `?t::net`, und der Platzhalter einer Mengenrolle darf Mitgliedspositionen annehmen, wie `?ref{1}` (5.2.3); ein Parameterplatzhalter hat keines von beiden.

### 6.11 Subskripte

| Geschrieben | Steht für |
|---|---|
| `&D<3>` | Dasselbe wie `&"D<3>"` |
| `&D<7:0>` | Eine Liste: `D<7>`, `D<6>`, …, `D<0>` |
| `&D<1,2:3>` | `D<1>`, `D<2>`, `D<3>` |
| `&D<0:6:2, 9>` | `D<0>`, `D<2>`, `D<4>`, `D<6>`, `D<9>` |
| `&D[3:0]` | `D[3]`, `D[2]`, `D[1]`, `D[0]` |
| `#XI0<3:0>` | Vier Instanzen |
| `#X1:D<7:0>` | Acht Anschlüsse von X1 |
| `#:D<7:0>` | Acht Ports dieser Ebene |
| `&"D"<1:0>` | `D<1>`, `D<0>` |
| `&"D<7:0>"` | Der Knoten mit dem Namen `D<7:0>`: nichts in Anführungszeichen wird geparst |

- Eine Netzliste kennt keine Busse: wenn Werkzeuge wie Virtuoso eine Netzliste schreiben, zerlegen sie einen Bus in einzelne Bits, etwa `D\<3\>` in einer Spectre-Netzliste; Bus-Ports von Teilschaltungen und iterierte Instanzen werden ebenfalls zerlegt. Ein Subskript ist eine Kurzschreibweise für einen oder mehrere solcher Namen. Es baut lediglich Namen und setzt nicht voraus, dass der Host Busse kennt, sodass seine Bedeutung eindeutig ist.
- **Form**: ein Host-Name, mit oder ohne Anführungszeichen, unmittelbar gefolgt von `<…>` oder `[…]` mit durch `,` getrennten Elementen. Jedes Element ist `i`, `a:b` oder `a:b:s`, und die Formen dürfen gemischt werden. Die Zahlen i, a, b und s sind vorzeichenlose Dezimalganzzahlen, und s ist mindestens 1. Zwischen dem Namen und der Klammer steht kein Leerraum; innerhalb der Klammern sind Leerraum und Kommentare zulässig (3.5).
- **Expansion**: die Elemente werden in der geschriebenen Reihenfolge expandiert und zusammengefügt; Wiederholungen bleiben erhalten. `a:b` läuft von a bis b, und `a:b:s` in Schritten von s, ohne b zu überschreiten: `<7:0>` ist 7, 6, …, 0, und `<0:7:2>` ist 0, 2, 4, 6. Die Schrittweite steht zuletzt, wie in Virtuoso und Xschem, nicht wie bei `a:s:b` in MATLAB. Ein erzeugter Name ist der Name vor der Klammer, die öffnende Klammer, die Dezimalzahl ohne führende Nullen und die schließende Klammer in der geschriebenen Klammerart: `&D<07>` ist `D<7>`, und `&D[3]` ist `D[3]`.
- **Eines oder eine Liste**: ein Element aus einer einzelnen Zahl ergibt einen Verweis: `&D<3>` ist `&"D<3>"`. Alles andere ergibt eine Liste, die ein Verweis auf eine Menge ist, wie `&D<3:3>` und `&D<3,5>`. Eine Liste in einer Menge wird eine Stufe ausgebreitet (5.2); als einziges Objekt einer Anweisung wird sie zuerst eine Stufe aufgeteilt (5.2.1), sodass `#XI0<3:0> @ALIGN("y")` vier Instanzen aneinander ausrichtet; in einem Slot für einzelne Objekte wirkt sie Mitglied für Mitglied; im Netz-Slot von WIRE wird sie als Ganzes genommen und erzeugt einen Bus (13.1).
- **Wo**: Host-Namen nach `#` und `&` sowie Anschlussnamen und Portnamen in `#`-Verweisen. RNL-Namen (nach `%`, `$` und `?`), Rollennamen und Unterobjektnamen nehmen kein Subskript an; ein `<` oder `[` unmittelbar nach ihnen ist ein Fehler. Mitglieder einer Menge werden stattdessen über ihre Positionen mit geschweiften Klammern herausgegriffen (5.2.3).
- **Kein Punkt direkt nach einem Namen**: ein `<` oder `[` unmittelbar nach einem Namen beginnt stets ein Subskript, und passt das Folgende nicht zur Form, ist es ein Syntaxfehler; es wird niemals als Punkt gelesen. Um einen Punkt nach einem Namen zu schreiben, trennt man sie, etwa `&D (1, 2)`.
- **Eine Expansion**: ein Name hat höchstens ein Subskript, sodass ein Name wie `A<1>[2]` in Anführungszeichen geschrieben wird, `&"A<1>[2]"`. Höchstens ein Subskript in einem Verweis expandiert zu einer Liste: `#XI0<3:0>:a` und `#X1:D<7:0>` sind zulässig, `#XI0<1:0>:D<1:0>` ist ein Fehler, und zwei Subskripte, die jeweils eine einzelne Zahl sind, sind zulässig, wie `#XI0<3>:D<1>`. Folgt auf einen zu einer Liste expandierenden Verweis ein `::`, wird das Unterobjekt für jedes Mitglied genommen, wie `#XI0<3:0>::name`.
- **Nichts in Anführungszeichen wird geparst**: `&"D<7:0>"` verweist auf den Knoten namens `D<7:0>`; `&"D"<7:0>` expandiert. Enthält ein Name in der Netzliste `<` oder `[`, die keine Bus-Bits markieren, setzt man ihn in Anführungszeichen, um auf ihn unverändert zu verweisen.
- **Verglichen werden die Namen, die der Adapter liest** (3.6): `D\<7\>` in einer Spectre-Netzliste, wo der Backslash Spectres Maskierung ist, wird vom Adapter als `D<7>` gelesen, sodass `&D<7>` darauf verweist.
- Jedes Mitglied wird für sich aufgelöst. Ein nicht gefundenes Mitglied ist ein fehlgeschlagenes Objekt, das je nach Angabe des empfangenden Slots strikt oder tolerant behandelt wird (7.3).
- Zeichenketten in Parametern werden nicht expandiert, etwa das `terminal=` von PIN (9.2.4) und die Bedingung `name=` einer Auswahl.
- Nicht akzeptiert: die Wiederholung von Virtuoso, `<*2>A` und `A<0:2*2>`, sowie dessen Nebeneinanderstellung `A,B` (Mengen können bereits geschrieben werden); `D[0..7]` von KiCad und Xschem, deren Mitglieder D0 … D7 ohne Klammern sind.
- Iterierte Instanzen (wie `I0<3:0>` von Virtuoso: mehrere gemeinsam als eine gezeichnete Host-Instanzen) werden als Bündel geschrieben: `#XI0<3:0> @BUNDLE` (11.8).

### 6.12 Anschlussnamen: DEVICE

```spice
* sky130 transistors, defined in the PDK and not in this netlist
* @RNL(type="header") {
*   @VERSION("0.4");
*   @DEVICE(name="X*", model="sky130_fd_pr__nfet_*", terminals=["d", "g", "s", "b"], type="nmos");
*   @DEVICE(name="X*", model="sky130_fd_pr__pfet_*", terminals=["d", "g", "s", "b"]);
*   @DEVICE(model="sky130_fd_pr__pfet_*", type="pmos")    /* adds the type to the names given above */
* }
XM1 out in 0 0 sky130_fd_pr__nfet_01v8 w=1 l=0.15
XM2 out in vdd vdd sky130_fd_pr__pfet_01v8 w=2 l=0.15
* @RNL { #XM1:d @CURRENT("in") /* XM1 is a sky130 NMOS; its terminal 1 is named d, and its type is nmos */}
.end
```

- **RNL benennt die Anschlüsse und vergibt den Typ.** `@DEVICE` beschreibt eine Bauteilklasse: `terminals` listet die Namen ihrer Anschlüsse in der Reihenfolge der Knoten in der Netzlistenzeile auf, und `type` gibt ihren Typ an (unten). Es vergibt lediglich Namen an die Anschlussnummern des Hosts und ändert weder die Reihenfolge der Anschlüsse noch die Schnittstelle einer Teilschaltung; es ist ein Anzeige-Alias, was Prinzipien 2.2 zulässt. Die Namen in einer `terminals`-Liste müssen sich nach den Groß-/Kleinschreibungsregeln des Hosts unterscheiden und dürfen nicht nur aus Ziffern bestehen (6.4); beispielsweise gelten `"d"` und `"D"` in SPICE als gleich. Im Kopfeintrag einer Paketdatei, die von Dateien beider Dialekte verwendet werden kann, müssen sie sich ohne Rücksicht auf Groß- und Kleinschreibung unterscheiden. Eine Liste mit zwei gleichen Namen oder einem nur aus Ziffern bestehenden Namen ist ein Fehler, jene DEVICE-Anweisung schlägt fehl, und die Instanzen behalten die Namen, die frühere DEVICE-Anweisungen und der Adapter ihnen geben.
- **Nur im Kopfeintrag** (2.8): anderswo geschriebenes DEVICE ist ein Fehler. Der Kopfeintrag einer Paketdatei zählt mit (16.2). Ein Inline-Paket kann kein DEVICE enthalten; man schreibt es in den Kopfeintrag der Datei.
- **Abgleich**: `name` wird mit dem Instanznamen verglichen und `model` mit dem in der Instanzzeile geschriebenen Modellnamen oder Teilschaltungsnamen (in Spectre dem Master der Instanz, etwa `resistor` für ein Grundelement); beide dürfen den Platzhalter `*` (beliebig viele Zeichen) verwenden und werden nach den Groß-/Kleinschreibungsregeln des Hosts verglichen. Eines von beiden darf weggelassen werden, aber nicht beide, und ein DEVICE passt auf eine Instanz, wenn die geschriebenen Angaben passen. Ein DEVICE gibt `terminals`, `type` oder beides an; eines ohne `name` und `model` oder ohne `terminals` und `type` ist ein Fehler und schlägt fehl. Die Auswahlbedingungen, die Namen vergleichen, nehmen `*` nach denselben Regeln (10.1).
- **Typ**: `type` ist eine Zeichenkette wie `"nmos"`; die Wörter der Tabelle unten sind die Standardwörter. Es ist der Typ, den die Bedingung `type` (10.1), `${type}` (13.6) und die Zusicherungen (15.3) für eine Instanz lesen, der der Host keinen gibt, etwa eine X-Instanz, einschließlich in der Netzliste nicht definierter PDK-Bauteile. Eine Instanz einer Modellkarte behält den Typ ihres Modells (unten): ein DEVICE mit abweichendem Typ liefert eine Warnung und ändert ihn nicht, wie bei den Portnamen von Teilschaltungen.
- **Passen mehrere, zählt das spätere Eintragspunkt für Punkt**: für die Anschlussnamen und für den Typ getrennt zählt das zuletzt passende DEVICE, das den Punkt angibt, sodass ein DEVICE die Namen und ein späteres den Typ vergeben kann und ein späteres DEVICE, das einen Punkt weglässt, diesen nicht ändert. Die Vorgaben des Adapters kommen zuerst, dann die DEVICE-Anweisungen der von der Ebene verwendeten Pakete (in der Reihenfolge der Positionen der Pakete, 16.2), dann die in den Kopfeinträgen der äußeren Dateien, dann die der eingebundenen Dateien; innerhalb eines Kopfeintrags in der geschriebenen Reihenfolge, und von den Kopfeinträgen einer Datei zuerst der ohne `language`, dann jene mit `language` in der geschriebenen Reihenfolge, die jeweils nur auf die Instanzen in Abschnitten ihrer Sprache wirken (2.8). So überschreibt das in einer Datei Geschriebene ihre Pakete, Pakete überschreiben den Adapter, und spezifische Anweisungen sollten nach allgemeinen stehen. Die DEVICE-Anweisungen aller Pakete stehen vor denen der Dateien, wo immer ihr USE-PACKAGE steht: der Kopfeintrag steht immer am Dateianfang, sodass ein Platzieren eines Pakets bei seinem USE-PACKAGE es das in der Datei selbst Geschriebene überschreiben ließe. Ein DEVICE mit `model` wird nicht als spezifischer eingestuft; es gibt keine gesonderte Spezifitätsregel.
- **Geltungsbereich**: DEVICE im Kopfeintrag einer Datei gilt für diese Datei und die von ihr eingebundenen Dateien; eine eingebundene Datei kann nur sich selbst und die wiederum von ihr eingebundenen Dateien ändern (6.1). DEVICE im Kopfeintrag einer Paketdatei gilt für die Ebenen, die das Paket verwenden (16.2). Beispielsweise kann ein Paket für sky130 DEVICE-Anweisungen, Symbole und die Regeln zur Symbolwahl enthalten, und ein Benutzer, der `@USE-PACKAGE(name="sky130-rnl")` schreibt, erhält Anschlussnamen, Typen und Symbole.
- **Vorgaben des Adapters**: die Standard-Elementdefinitionen jedes Dialekts, die als auf der äußersten Ebene geschrieben gelten. Sie müssen nicht in der Datei stehen, und ein Werkzeug darf sie ausschreiben. Werkzeuge für denselben Dialekt müssen dieselben Definitionen liefern. Es sind die Vorgaben der Version der Datei, in der die Instanzzeile steht (16.1), so als stünde ein DEVICE im Kopfeintrag dieser Datei. Für die SPICE-Familie:

  ```rnl header
  @DEVICE(name="M*", terminals=["d", "g", "s", "b"]);
  @DEVICE(name="Q*", terminals=["c", "b", "e", "s"]);
  @DEVICE(name="J*", terminals=["d", "g", "s"]);
  @DEVICE(name="Z*", terminals=["d", "g", "s"]);
  @DEVICE(name="D*", terminals=["a", "k"]);
  @DEVICE(name="R*", terminals=["p", "n"]);             /* and one each like it for C, L, V, I, B, F, H, W */
  @DEVICE(name="E*", terminals=["p", "n", "cp", "cn"]);  /* and one each like it for G, S */
  @DEVICE(name="T*", terminals=["p1", "n1", "p2", "n2"]); /* and one like it for O */
  @DEVICE(name="U*", terminals=["a", "b", "ref"])
  ```

  Instanznamen in Spectre beginnen nicht mit einem Elementbuchstaben, daher werden die Vorgaben über `model`, den Master der Instanz, abgeglichen. Für die Grundelemente von Spectre:

  ```rnl header
  @DEVICE(name="*", model="resistor", terminals=["p", "n"]);      /* and one each like it for capacitor, inductor, vsource, isource, ccvs, cccs, bsource, port */
  @DEVICE(name="*", model="diode", terminals=["a", "k"]);
  @DEVICE(name="*", model="vcvs", terminals=["p", "n", "cp", "cn"]); /* and one each like it for vccs, relay */
  @DEVICE(name="*", model="tline", terminals=["p1", "n1", "p2", "n2"]);       /* as T in SPICE */
  @DEVICE(name="*", model="transformer", terminals=["p1", "n1", "p2", "n2"])   /* as T in SPICE */
  ```

  - `switch` hat einen Anschluss für jeden seiner Knoten, benannt wie Spectre seine Positionen nummeriert: `t0`, der gemeinsame Anschluss, dann `t1` bis `tn`, die Umschaltkontakte.
  - Eine Instanz einer Modellkarte, deren Master der Name einer `model`-Anweisung ist, übernimmt die Namen der Familie des Masters dieses Modells. Ihr Typ für die Bedingung `type` (10.1) wird mit den Wörtern von SPICE geschrieben:

  | Familie | Master des Modells | Anschlüsse | Typ |
  |---|---|---|---|
  | MOS | `bsim1`, `bsim2`, `bsim3`, `bsim3v3`, `bsim4`, `bsim6`, `bsimbulk`, `bsimsoi`, `bsimcmg`, `bsimimg`, `b3soipd`, `mos0`, `mos1`, `mos2`, `mos3`, `mos9`, `mos11`, `mos705`, `mos902`, `mos903`, `psp102`, `psp103`, `psp104`, `pspnqs102`, `pspnqs103`, `ekv`, `hisim`, `hisim2`, `hisimhv`, `hvmos`, `bta_soi`, `ucsd_hvmos`, `hisim_hv`, `utsoi`, `letisoi`, `bsimcmg_va` | `d g s b` | `nmos`, oder `pmos`, wenn `type` des Modells `p` oder `pmos` ist |
  | Bipolar | `bjt`, `bjt500`, `bjt503`, `bjt504`, `bjt504t`, `bjt3500`, `bjt301`, `vbic`, `hicum`, `hicum0`, `hicum2`, `mextram` | `c b e s` | `npn`, oder `pnp`, wenn `type` des Modells `pnp` ist |
  | Diode | `diode`, `juncap`, `juncap200`, `dio500`, `junb` | `a k` | `d` |
  | JFET | `jfet` | `d g s` | `njf`, oder `pjf`, wenn `type` des Modells `p` oder `pjf` ist |
  | MESFET | `gaas`, `angelov`, `tom2`, `tom3` | `d g s` | `nmf`, oder `pmf`, wenn `type` des Modells `p` oder `pmf` ist |
  | Widerstand | `resistor`, `rdiff`, `r3` | `p n` | `r` |
  | Kondensator | `capacitor` | `p n` | `c` |
  | Induktivität | `inductor` | `p n` | `l` |
  | Relais | `relay` | `p n cp cn` | `sw` |
- **In der Netzliste definierte Teilschaltungen** (einschließlich jener in über include oder lib eingebrachten Bibliotheksdateien): die Portnamen sind die der Definition, was ein Fakt des Hosts ist (Prinzipien 2.2). DEVICE ändert sie nicht; ein DEVICE, das mit abweichenden Namen auf eine solche Instanz passt, liefert eine Warnung. DEVICE benennt ausschließlich Bauteile ohne Anschlussnamen: Grundelemente und in der Netzliste nicht definierte Teilschaltungen (PDK-Bauteile), für die der Benutzer oder ein LLM das DEVICE schreibt. Ein DEVICE ohne `terminals` kann einer in der Netzliste definierten Teilschaltung dennoch ihren Typ geben.
- **Anzahl der Namen**: hat eine Instanz mehr Knoten als `terminals` Namen, haben die übrigen Anschlüsse nur Nummern; hat sie weniger, schlägt ein Verweis auf einen überzähligen Namen fehl.
- **Groß-/Kleinschreibung**: die von DEVICE vergebenen Anschlussnamen werden nach den Regeln des Hosts verglichen (6.4).
- Nummern funktionieren immer, ebenso Portnamen von Teilschaltungen, die nicht nur aus Ziffern bestehen, und `#:port`.

<!-- translated from en/ch07.md sha256:7d1065e7e655 -->

## 7 Fehlschläge und Diagnosemeldungen

### 7.1 Zwei Phasen der Prüfung

- **Lesen**: die lexikalische Struktur; die Grammatik; die Signaturen der eingebauten Schlüsselwörter, also die Anzahl der Objekte, ihre Arten und die Parameter (einschließlich der Form von Arrays); sowie ob jedes `?name` im Rumpf einer allgemeinen Definition in deren Rollenliste oder Parameterliste steht und an einer dazu passenden Stelle steht (6.10). Signaturfehler im Rumpf einer allgemeinen Definition, die nicht von den Argumenten abhängen, werden ebenfalls zu diesem Zeitpunkt gemeldet (6.9).
- **Wirksamwerden**:
  - nachdem die gesamte Ebene eingelesen wurde, werden Namen und Host-Verweise aufgelöst und die Arten der Objekte geprüft, auf die Namen verweisen;
  - die Namen im Rumpf einer allgemeinen Definition werden für jede Definition einmal aufgelöst, nicht erneut für jede Anwendung (6.9);
  - die Anzahl der Objekte, die Rollennamen, die für die Rollen deklarierten Arten und die Parameter von AS und ATTACH werden geprüft (9.4);
  - jede Anwendung prüft die Teile ihrer Expansion, die von den Argumenten abhängen;
  - sobald Symbole zugewiesen sind, wird geprüft, ob die Anschlüsse, an die Leitungen und Bedingungen anschließen, in ihrem Symbol einen Pin besitzen, ob die Anwendungen, auf die ORIENT wirkt, ein Symbol besitzen, ob die über eine Instanz oder Anwendung angesprochenen Pin-Namen wie `#R1::p` oder `%g::pin` Pins ihres Symbols sind, und ob die Umwandlungen von und zu Pins vorgenommen werden können (5.2.4).

### 7.2 Fehlgeschlagene Objekte

- Eine fehlgeschlagene Anweisung erzeugt ein fehlgeschlagenes Objekt, das eine Diagnosemeldung trägt: die Ursache, den Ort und die vorgelagerte Ursache.
- Ein fehlgeschlagenes Objekt behält seinen Platz unter den Mitgliedern, und die anderen Mitglieder behalten ihren. Hat die Host-Netzliste kein M9, schlägt `{#M1 #M2 #M9} @ALIGN("y")` als Ganzes fehl; es wird nicht zu einer Ausrichtung von nur M1 und M2.
- Ist ein Name an ein fehlgeschlagenes Objekt gebunden (einschließlich einer beim Lesen fehlgeschlagenen Definition), erhalten die Anweisungen, die auf den Namen verweisen, ebenfalls das fehlgeschlagene Objekt. Anweisungen, die von einer fehlgeschlagenen Beschreibung abhängen, schlagen mit ihr fehl, unabhängig von ihrer Reihenfolge (Prinzipien 2.9), außer dass eine Anweisung, die bei einer Prüfung nach Zuweisung der Symbole fehlschlägt, die bereits durchlaufenen Schritte nicht rückgängig macht (8.2); auch dies sind Fehler, und ihre Diagnosemeldungen nennen die vorgelagerte Ursache (7.6).
- Ein zweites Binden eines Namens an ein anderes Objekt ist ein Fehler, und der Name behält seine erste gültige Bindung; nur wenn keine seiner Bindungen gültig ist, wird der Name an ein fehlgeschlagenes Objekt gebunden (6.6, 5.5).

### 7.3 Strikte und tolerante Eingaben

- Die Signatur deklariert für jede Eingabe, ob sie strikt oder tolerant ist:
  - strikt: enthält die Eingabe ein fehlgeschlagenes Objekt, schlägt die gesamte Anweisung fehl;
  - tolerant: die fehlgeschlagenen Mitglieder werden entfernt und eine Diagnosemeldung wird festgehalten; der Rest wird wie üblich wirksam.
- Standardmäßig sind absorbierte Eingaben strikt; eingeschlossene Eingaben (Definitionsrümpfe) und festgeschriebene Eingaben (`@RNL`) sind tolerant (5.4).
- Die Argumente von AS und ATTACH sind strikt; die Anweisungen der Expansion sind tolerant: jede, die fehlschlägt, wird für sich verworfen.
- Bei Pipes und Verschachtelung betrifft ein Fehlschlag nur die Beschreibungen, die davon abhängen (5.5): enthält eine strikte Eingabe ein fehlgeschlagenes Objekt, schlägt die Anweisung fehl, doch darin verschachtelte Anweisungen, die nicht vom Fehlschlag abhängen, werden wie üblich wirksam; arbeitet eine Anweisung einmal für jedes Mitglied, ist jedes Mitglied eine eigene Beschreibung und schlägt für sich allein fehl.

### 7.4 Fehlerbehebung

- Ein Syntaxfehler lässt das gesamte Mitglied fehlschlagen, in dem er auftritt, bis zum nächsten `;` oder dem Ende der umgebenden Menge.
- Ist eine Zeichenkette oder ein `/* */`-Kommentar nicht geschlossen oder passen die geschweiften Klammern nicht zusammen, lässt sich das Ende des Eintrags nicht finden (2.4): der Eintrag ist unvollständig, und der gesamte Eintrag schlägt fehl. Eine innerhalb eines Mitglieds nicht geschlossene Klammer — etwa die von Parametern, eines Arrays, einer Rolle, eines Punkts oder eines Subskripts — ist nur ein Syntaxfehler dieses Mitglieds: die übrigen Mitglieder des Eintrags werden wie üblich gelesen.
- Wird ein Eintrag vor seiner Vollendung abgeschnitten, schlägt der gesamte Eintrag fehl, und die Diagnosemeldung gibt an, dass der Eintrag unvollständig ist (2.4).
- Die Erholung von einem Fehler reicht niemals über seinen Eintrag hinaus: die übrigen Einträge werden wie üblich gelesen (Prinzipien 2.9).

### 7.5 Konflikte sind keine Fehlschläge

- Konflikte werden nach dem Einlesen der gesamten Ebene durch die definierten Verfahren behandelt: Überschreibung nach Schlüssel (8.4), die Darstellungswarteschlangen (8.3) und die Auflösung geometrischer Fakten (8.5).
- Ein Konflikt, für den noch kein Verfahren definiert ist, lässt die gesamte Interpretation fehlschlagen (1.5 Punkt 3; 7.6). In dieser Version gibt es keinen solchen Fall: ein Zyklus von Mengen schlägt bei der Anweisung fehl, die ihn schließt (5.2.2).

### 7.6 Diagnosemeldungen

- **Prüfen und Interpretieren sind ein Vorgang.** Ein Interpreter liest die Eingabe und liefert die entstehende Szene zusammen mit einer einzigen Menge von Diagnosemeldungen; ausschließlich zur Prüfung eingesetzt, wird er auch Lint genannt. Dieser Standard definiert ausschließlich diese eine Menge von Diagnosemeldungen. Die lokalen Prüfungen, die ein Editor für sofortige Rückmeldung vornimmt, etwa Syntaxhervorhebung, sind keine Diagnosemeldungen in diesem Sinne: dieser Abschnitt gilt dafür nicht, und sie können die Diagnosemeldungen nicht ersetzen.
- **Elemente.** Ein Fehlschlag bezieht sich auf ein Element: ein Feld eines Eintragskopfs, eine Anweisung, einen Eintrag, eine Datei oder die gesamte Interpretation. Welches gemeint ist, wird jeweils dort angegeben, wo der Fehlschlag definiert ist (siehe Umfang eines Fehlschlags unten).
- **Vier Arten von Meldungen**:
  - **Fehler**: ein Element schlägt fehl, ob die Ursache im Element selbst, in einem anderen Element oder vorgelagert in etwas liegt, wovon es abhängt. Zu den Fehlern gehören die Grundfehler aus Prinzipien 2.4 (Syntaxfehler, unbekannte Wörter, nicht passende Signatur, nicht gefundener Verweis, falsche Art, Selbstwiderspruch, nicht unterstützte Version), die weiteren in diesem Standard definierten Fehlschläge (etwa ORIENT auf einer Anwendung ohne Symbol oder ein Anschluss ohne Pin in seinem Symbol; 12.1, 9.2.4) sowie die daraus folgenden Fehlschläge (7.2).
  - **Warnung**: das Element ist gültig und wird wie üblich wirksam, sofern dieser Standard nichts anderes bestimmt (wie bei einem Fakt, der verworfen wird, weil seine Prüfung nicht abgeschlossen wurde, 8.5), entspricht aber möglicherweise nicht der Absicht des Verfassers, etwa eine für einen Slot für einzelne Objekte ausgeschriebene leere Menge wie `{} @PLACE(1, 2)` (5.2.1); eine unbenannte Definition direkt als Mitglied eines Eintrags, nicht in einer Anweisung geschachtelt (9.1); eine Rolle einer Generierungsregel, die die unteren Ebenen erreicht (10.1), gefüllt mit einem Objekt der eigenen Ebene, das jene Ebenen nicht haben (10.5); ein unbekanntes Feld in einem Eintragskopf (2.8); eine durch eine andere ersetzte Paketversion (16.2); eine Leitung an einen Anschluss, der nicht auf dem von der Leitung benannten Netz liegt (13.1); ein geometrischer Fakt, der verworfen wird, weil seine Prüfung innerhalb des Limits des Solvers nicht abgeschlossen wurde (8.5).
  - **Hinweis**: eine häufige Überschreibung, Verdeckung oder Auflösung, die ebenfalls ein Versehen sein kann, etwa eine untere Ebene oder ein darin verwendetes Paket, das eine Definition einer oberen Ebene oder der Pakete an der Wurzel verdeckt (6.8); ein Paket eines späteren USE-PACKAGE, das eines eines früheren USE-PACKAGE verdeckt (16.2); ein bei der Konfliktauflösung verworfener geometrischer Fakt (8.5); die mangels Angabe verwendete neueste Version eines Pakets (16.2); eine Hauptdatei ohne deklarierte Version, die mit der Standardversion des Werkzeugs gelesen wird (16.1); nicht gefundene Namen des Standardpakets, wo das Standardpaket nicht verwendet wird (16.3); ein Eintrag des Solvers, dessen Nummern veraltet sind (5.9).
  - **Verstoß**: eine Aussage der Datei über die Schaltung gilt nicht: eine Zusicherung, deren Prüfung nicht hält (15.3, 15.4), oder die CURRENT-Anweisungen eines Netzes, die seinen gesamten Strom in eine Richtung lenken (15.2). Nichts schlägt fehl, und die Prüfung sowie ihre Meldung ändern nichts an der Zeichnung: eine Zusicherung zeichnet so, als stünde sie nicht dort, und die von der Prüfung gelesenen CURRENT-Anweisungen bleiben wirksam. Die Meldung richtet sich an den Verfasser und an Werkzeuge zur Dateiprüfung. Ein Viewer darf sie unter einer eigenen Bezeichnung anzeigen.
- **Der Umfang eines Fehlschlags** wird jeweils dort angegeben, wo ein Fehlschlag definiert ist: ein Feld (ein falsch geschriebenes bekanntes Feld eines Eintragskopfs gilt als nicht geschrieben, 2.8); eine Anweisung (die meisten Fälle); ein Eintrag (nicht geschlossene Zeichenkette oder Kommentar, nicht passende geschweifte Klammern, abgeschnittener Eintrag, nicht gefundenes `scope`-Ziel; 7.4, 2.4, 2.6); eine Datei (nicht unterstützte Version, 16.1); die gesamte Interpretation (ein Konflikt, für den noch kein Verfahren definiert ist, 1.5 Punkt 3: die Interpretation bricht ab und nichts wird gezeichnet; eine spätere Version könnte einen solchen Fall tolerant machen; diese Version hat keinen).
- **Anforderungen**:
  - Fehler müssen gemeldet werden, und ebenso Verstöße: die Meldung ist der Zweck einer Zusicherung (15.3). Warnungen und Hinweise sind die in diesem Standard aufgeführten, und Implementierungen sollten sie melden.
  - Eine Implementierung darf eigene Prüfungen hinzufügen, jedoch nur als Warnungen oder Hinweise; sie dürfen kein Element fehlschlagen lassen, oder verschiedene Implementierungen würden unterschiedliche Ergebnisse zeichnen.
  - Eine Implementierung muss dem Benutzer die Einsicht in die Diagnosemeldungen ermöglichen; wie sie dargestellt werden, bleibt der Implementierung überlassen (Prinzipien 2.9).
  - Eine Implementierung darf strenger sein, etwa anhalten oder die gesamte Eingabe zurückweisen (Prinzipien 2.9), doch darf dies nicht ihre Voreinstellung sein. Sie darf den Benutzer den Schweregrad von Warnungen, Hinweisen und Verstößen anheben oder absenken lassen; Fehler dürfen nicht herabgestuft werden, da ein Fehler ein Fehlschlag ist.
  - Konformitätstests prüfen Fehler (ihren Ort und den Umfang des Fehlschlags), Verstöße (welche Prüfung nicht hält und für welche Mitglieder) sowie die entstehende Szene; Warnungen dürfen optionale Erwartungen sein; Hinweise werden nicht geprüft.
  - Hinweis: Warnungen raten, was der Verfasser gemeint hat, und werden sich mit der Erfahrung ändern; sie zu Anforderungen zu machen, würde diese Vermutungen im Standard einfrieren. Prinzipien 2.4 fordert ebenfalls nur die Grundfehler.
- **Inhalt**: eine Diagnosemeldung nennt
  - den Ort: die Datei, die Zeile und Spalte bzw. den Bereich; welchen Eintrag, welche Anweisung, welches Feld oder welchen Verweis (Prinzipien 2.8);
  - die Ursache: was erwartet wurde und was empfangen wurde, etwa „in_p takes an instance, got a net“ (9.4); wird ein Name nicht gefunden, den nächstliegenden Namen;
  - die vorgelagerte Ursache: jede infolge eines Fehlers fehlgeschlagene Anweisung nennt ihre vorgelagerte Ursache, und eine Darstellung darf solche Anweisungen unter der Grundursache bündeln, damit ein einziger Tippfehler nicht Dutzende Fehler über die Zeichnung erzeugt. Schlägt eine Regel auf vielen Objekten fehl, dürfen die Fehlschläge zu einer Diagnosemeldung zusammengefasst werden, die die Objekte auflistet, etwa wenn ein Paket `W/L=${w}/${l}` für jeden MOS schreibt und einige Instanzen kein `w` haben;
  - die Herkunft: bei einer aus einer allgemeinen Definition expandierten Anweisung sowohl die Anwendung als auch die ursprüngliche Anweisung in der Definition; bei einer durch eine Regel erzeugten die Regel und die Ebene, in der sie ausgewertet wurde; bei einer in einem Paket den Namen, die Version und die Datei des Pakets (16.2).
- Wortlaut und Sprache der Diagnosemeldungen sind nicht vorgeschrieben. Diese Version hat keine Standardcodes für Diagnosemeldungen; Implementierungen dürfen ihre Diagnosemeldungen nummerieren, und die Nummern definieren keine Toleranzklassen.
- **Pakete**: in einem Dokument, das ein Paket verwendet, werden Fehler im Inhalt des Pakets wie üblich gemeldet und dem Paket zugeschrieben; Warnungen und Hinweise zum Inhalt des Pakets werden nur ausgegeben, wenn das Paket selbst geprüft wird. Verstöße gegen Zusicherungen, die ein Paket schreibt, werden in dem Dokument gemeldet, das es verwendet, da sie dessen Schaltung betreffen, und nennen die Zusicherung im Paket sowie die Anwendung oder Regel, die sie angewendet hat. Hinweis: Benutzer können ein Paket nicht ändern, und breite Regeln, die sein Autor absichtlich geschrieben hat, würden sie sonst mit Warnungen überfluten.
- Eine Datei kann Diagnosemeldungen nicht unterdrücken oder einen strengeren Modus anfordern.
- Unbekannte Schlüsselwörter, Parameter und Werte sind Fehler wie Syntaxfehler (Prinzipien 2.3): die Anweisung schlägt fehl (16.4), außer in SOLVER-CONFIG, wo ein Wert, der nicht angenommen wird, nur seinen Schlüssel fehlschlagen lässt (14.10). In dieser Version ist der Inhalt, der erkannt, aber nicht interpretiert wird und nur eine Warnung liefert, ein unbekanntes Feld eines Eintragskopfs (2.8) und ein Schlüssel von SOLVER-CONFIG, den der Solver nicht kennt (14.10).

<!-- translated from en/ch08.md sha256:bf7d27b5693d -->

## 8 Wirksamwerden

### 8.1 Eine Ebene wird nach vollständigem Einlesen wirksam

- Der Interpreter parst die gesamte Host-Netzliste und liest alle Einträge; danach wird jede Ebene in den Phasen aus 8.2 wirksam.
- Innerhalb einer Ebene ist die Reihenfolge der Anweisungen nur so maßgeblich, wie 1.5 Punkt 2 angibt; sie beeinflusst nicht, wie Namen und Verweise aufgelöst werden (Prinzipien 2.9).
- Welche von zwei Ebenen die andere überschreibt, entscheidet die Hierarchie der Ebenen, nicht die Reihenfolge im Text (Prinzipien 2.9; 8.3).

### 8.2 Phasen

1. **Lesen**: die lexikalische Struktur, die Grammatik und die Signaturen.
2. **Namen und Definitionen**: `$`-Namen, `%`-Namen und Host-Verweise werden aufgelöst; die Rümpfe von DEFINE-SYMBOL und DEFINE-STYLE werden ausgewertet.
3. **Objekte erzeugen**: die explizit geschriebenen Szenenobjekte (Punkte, Leitungen, Beschriftungen, Bündel, Grafiken, Bedingungen, Anwendungen) und die Mengen werden erzeugt, und die Rümpfe allgemeiner Definitionen werden expandiert; die Regeln in einem Rumpf treten den Regeln der Ebene der Anwendung bei (9.4). Auswahlen werden hier nicht expandiert.
4. **Symbol-Warteschlange**: SET-SYMBOL wird wirksam und weist den Objekten aus Schritt 3 Symbole zu. Erst jetzt werden die Auswahlen der Regeln in dieser Warteschlange in jeder Ebene ausgewertet.
5. **Bäume**: die Regeln von ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET und UNROUTE-OBJECT (10.4, 13.7) werden in jeder Ebene einmal ausgewertet, in der Reihenfolge: Pakete, die oberen Ebenen, diese Ebene. Ihre Bedingungen lesen die Bäume der Anweisungen aus Schritt 3, und sie sehen einander nicht; die von ihnen erzeugten Bäume sind ab Schritt 6 sichtbar (10.2).
6. **Schrittweise Generierung**: AS und ATTACH, deren Objekte das Ergebnis einer Auswahl sind, an das Ergebnis einer Auswahl angehängtes TEXT (10.5, 13.3), RECTANGLE, CIRCLE und ELLIPSE mit Größenangabe, deren Punkt das Ergebnis einer Auswahl ist (9.2.5), die Regeln von LINE, ARC und BEZIER sowie die Regeln von CURRENT (10.4) werden in jeder Ebene einmal in derselben Reihenfolge ausgewertet.
7. **Symbole für generierte Objekte**: dieselbe Symbol-Warteschlange wird noch einmal für die neuen Objekte durchlaufen, mit den SET-SYMBOL-Anweisungen und -Regeln des generierten Inhalts (10.5). Die Symbole der Objekte aus Schritt 3 stehen nach Schritt 4 fest, da Bäume und `wired` daraus berechnet werden (10.2): ein SET-SYMBOL des generierten Inhalts, das ein solches Objekt erreicht, hat darauf keine Wirkung, und es wird eine Warnung ausgegeben. Die Generierung sieht die Symbole so, wie sie vor der Generierung waren.
   Zusicherungen (15.3) werden hier geprüft, sobald die Symbole bekannt sind.
8. **Geometrie**: PLACE, ORIENT und Bedingungen werden einzeln in der Auflösungsreihenfolge aufgenommen, und eine, die mit den bereits aufgenommenen in Konflikt steht, wird verworfen (8.5, 14.8); danach wird das Layout gelöst. Dieser Schritt benötigt die Positionen der Pins und die Rahmen der Symbole (9.2.3), kommt also nach der Symbolzuweisung; ORIENT auf einer Anwendung ohne Symbol schlägt hier fehl, während PLACE und Bedingungen den eigenen Punkt der Anwendung nehmen (14.1). Objekte ohne Position oder Bedingung werden vom Solver platziert, der ihre Positionen zurückschreibt (14.8).
9. **Stil-Warteschlange**: SET-STYLE wird wirksam, mit den zwei Stufen aus 8.3. Erst jetzt erhalten die Stileigenschaften, die die Grafiken eines Symbols offenlassen, und die Farben von `fill=` konkrete Werte aus dem Stil des besitzenden Objekts.
10. **Darstellung**.

- Diese Phasen bilden die innere Ordnung aus 1.5 Punkt 4: eine Beschreibung liest den Zustand vor Beginn ihrer Phase; weder ihre eigenen Wirkungen noch die der anderen Beschreibungen derselben Phase sind enthalten.
- Eine Menge wird in der Phase ausgewertet, die sie verwendet (5.2): dieselbe Menge enthält bei Verwendung in Schritt 6 keine generierten Objekte und enthält sie bei Verwendung in Schritt 9.
- Auswahlbedingungen lesen ausschließlich Informationen, die in früheren Phasen festgelegt wurden:
  - Host-Informationen, Objektarten, Anwendungen, die als Anweisungen auf Anschlüssen geschriebenen Stromrichtungen sowie explizite Leitungen stehen in Schritt 3 fest; die Richtungen, die CURRENT-Regeln (Schritt 6) und CURRENT auf Pins (sobald die Symbole zugewiesen sind) setzen, sind ab Schritt 7 sichtbar, etwa für die Stil-Warteschlange und die Prüfung aus 15.2;
  - eine Auswahl in der Symbol-Warteschlange kann nicht nach dem Symbol auswählen, das ein Objekt verwendet;
  - Generierungsregeln können Symbole sehen.
- Ob ein Anschluss in seinem Symbol einen Pin hat, wird nach Schritt 7 geprüft (7.1). Eine Anweisung, die bei einer Prüfung nach Zuweisung der Symbole fehlschlägt, etwa eine Anwendung, deren Rolle der Art `"pin"` einen Anschluss ohne Pin erhält, hat an den vorherigen Schritten teilgenommen, wie sie geschrieben war: diese Schritte werden nicht neu berechnet, und ab dann hat sie keine Wirkung. Leitungen werden ebenso gezählt (10.2).
- Die Verfahren zum Lösen und Zeichnen werden von diesem Standard nicht vorgeschrieben; nur ihre Eingaben sind es (Prinzipien 1).

### 8.3 Darstellungswarteschlangen

- Es gibt zwei Warteschlangen: die Symbol-Warteschlange nimmt SET-SYMBOL auf, und die Stil-Warteschlange nimmt SET-STYLE auf.
- Für eine Ebene besteht jede Warteschlange aus diesen Gruppen in dieser Reihenfolge:
  1. die Regeln der Wurzelpakete: das Standardpaket zuerst, dann die weiteren Pakete von USE-PACKAGE in Kopfeinträgen (16.3); wird ein Paket in dieser Ebene oder einer oberen Ebene (in einer Dateiebene oder einer Teilschaltung) erneut mit USE-PACKAGE verwendet, tritt es stattdessen an die Stelle jenes USE-PACKAGE (16.2);
  2. die Symbole, die Teilschaltungen ihrem eigenen `#` zuweisen, also ihre Blocksymbole (nur Symbol-Warteschlange; 9.2.6);
  3. die Regeln der oberen Ebenen, von der äußersten nach innen;
  4. alle in dieser Ebene geschriebenen SET-SYMBOL- oder SET-STYLE-Anweisungen, sowohl Regeln als auch Anweisungen auf einzelnen Objekten;
  5. von außen mit `scope` in diese Ebene geschriebene SET-SYMBOL oder SET-STYLE, die aus äußeren Ebenen später (2.6).
- Innerhalb jeder Gruppe gilt die Textreihenfolge. Der Inhalt eines anderen Pakets tritt an die Stelle seines USE-PACKAGE (16.2): ein in einer oberen Ebene verwendetes Paket gehört zu jener Ebene, und ein in dieser Ebene verwendetes an die Stelle des USE-PACKAGE im Text dieser Ebene. Durch schrittweise Generierung erzeugte SET-SYMBOL und SET-STYLE gehören zur Gruppe der Regel, die sie erzeugt, an der Stelle jener Regel.
- **Zwei Stufen**:
  - Direkt: SET-SYMBOL und SET-STYLE, die dieses Objekt direkt benennen, also deren Eingabe sein Handle, sein Name oder ein `::`-Pfad zu ihm ist, oder eine in `{…}` geschriebene Menge, die es als Mitglied aufführt (auch wenn jene Menge benannt ist). Bei der Definition des Objekts geschriebene Stilparameter (9.2.5, 9.3) zählen ebenfalls mit, als wäre ein SET-STYLE für das Objekt an der Anweisung geschrieben worden, die es erzeugt.
  - Weitergegeben oder ausgewählt: was ein Mitglied durch eine Auswahl oder durch schrittweise Generierung erhält; was ein besitzendes Objekt an die Grafiken seines Symbols weitergibt (9.2.5); was ein Objekt an die an es angehängten Texte weitergibt (11.3); was eine Anwendung an ihr Symbol und an die in ihrem Rumpf erzeugten zeichenbaren Objekte weitergibt, und so weiter hinab durch geschachtelte Anwendungen, nicht jedoch an die Mitglieder, die ihre Rollen füllen (12.1 Punkt 4). Ein weitergegebener Fakt behält die Position seiner ursprünglichen Anweisung.
  - Für jede Eigenschaft (in der Symbol-Warteschlange das Symbol) werden zuerst die direkten Fakten betrachtet und der nach Position letzte genommen; gibt es keinen, werden die weitergegebenen oder ausgewählten Fakten betrachtet und der nach Position letzte genommen; gibt es von beiden keinen, nimmt eine Instanz das Standard-Blocksymbol (9.2.6), eine Anwendung wird nicht gezeichnet (12.1), und eine Stileigenschaft nimmt die Vorgabe des Renderers (9.3). Positionen folgen der obigen Reihenfolge der Warteschlange.
  - So wird ein Symbol oder Stil, den ein Editor für ein einzelnes Objekt ändert, nicht von später geschriebenen Regeln überschrieben; Regeln, Pakete und Themen bestimmen weiterhin die nicht einzeln geänderten Objekte.
- `type` und `author` eines Eintragskopfs beeinflussen die Reihenfolge nicht (2.8).
- Ein Symbol wird als Ganzes ersetzt; Stile verschmelzen eigenschaftsweise. Das Anwenden eines Stils wirkt so, als würde jede von ihm definierte Eigenschaft einmal geschrieben, sodass ein Stil, der nur `linewidth` definiert, auch nur die Linienbreite ändert.
- Regeln gelten für die Ebene, in der sie geschrieben stehen, und wenn sie diese erreichen — über `recursive=true` in einer in ihnen geschriebenen Auswahl oder, falls sie keine enthalten, über die von ihnen verwendeten Mengen (10.1) — für die Ebenen darunter, niemals für die Ebenen darüber; die Regeln eines Pakets gelten als in der Ebene seines USE-PACKAGE geschrieben, und die eines Wurzelpakets in der obersten Ebene seiner Datei (10.4, 16.2). Sie werden einmal in jeder Ebene ausgewertet und betrachten ausschließlich die Objekte jener Ebene.

### 8.4 Überschreibung nach Schlüssel

Fakten mit einem Schlüssel (5.8) überschreiben einander nach Schlüssel: der spätere wird wirksam (Prinzipien 2.9). Das Überschreiben ist eine zulässige Beschreibungsweise, kein Fehler; ob ein Hinweis gegeben wird, liegt bei der Implementierung. PLACE und ORIENT sind geometrische Fakten und werden nach 8.5 aufgelöst (5.8).

### 8.5 Konflikte

Geometrische Fakten können zusammen keine Lösung haben, auch wenn keine zwei von ihnen einander widersprechen, wie `{#M1 #M2} @ALIGN("y")`, `#M1 @PLACE(y=0)` und `#M2 @PLACE(y=10)`. Das Folgende legt fest, welche Fakten verworfen werden.

- **Geometrische Fakten** sind die vier atomaren Bedingungen (ALIGN, ORDER, DISTRIBUTE, DISTANCE), die Ausrichtungsrelation ORIENT-RELATION (14.9), TABLE (14.11), jede Koordinate von PLACE, `dx` und `dy` von TEXT (13.3; auf derselben Achse ersetzen diese und PLACE einander, was nicht hier aufgelöst wird) sowie ORIENT. Symbole (SET-SYMBOL) gehören nicht dazu; sie stehen vor der Geometrie fest (8.2). Ebenso wenig gehören die eigenen Kriterien der Implementierung dazu (Bauteile überlappen nicht, ein Raster usw.): dies sind eigene Abwägungen des Solvers und rangieren nach allen Beschreibungen der Sprache. Erzwingen die Beschreibungen eine Überlappung von Bauteilen, überlappen sie.
- **Einheiten der Auflösung**:
  - Eine atomare Bedingungsanweisung ist eine Einheit, die bei einem Konflikt als Ganzes verworfen wird (ORIENT-RELATION ebenfalls); sie wird niemals nur für einen Teil ihrer Mitglieder behalten. Dies stimmt mit Fehlschlägen überein: die Mitglieder einer atomaren Bedingung sind strikte Eingaben (7.3). Erhält ein Slot für einzelne Objekte eine Menge und arbeitet die Anweisung einmal für jedes Mitglied, ist jede dieser Beschreibungen eine Einheit (5.2.1); ebenso jede Bedingung, für die eine Mitgliederliste steht, eine für jede Kombination (14.1) oder jedes Paar von DISTANCE (14.5). Ein TABLE ist eine Einheit (14.11).
  - Ein PLACE besteht aus zwei Einheiten, eine für x und eine für y, die getrennt verworfen werden können. `dx` und `dy` von TEXT sind ebenfalls je eine Einheit; sie ersetzen nach Position (13.3) zuerst ein PLACE desselben Texts auf derselben Achse oder werden davon ersetzt, und die verbleibenden nehmen an der Auflösung teil.
  - ORIENT ist eine Einheit: die Ausrichtung dieses Objekts ist gleich einem Wert oder einem der Werte seiner Liste (11.2); ein ORIENT eines Pins besagt, dass die Ausrichtung seines Objekts eine derjenigen ist, die den Pin in die geschriebene Richtung oder in eine der geschriebenen Richtungen drehen. Ohne ORIENT gibt es keine Bedingung: der Solver wählt und schreibt die Ausrichtung zurück (14.8); ein Renderer ohne Lösung dafür zeichnet `"R0"`. Eine Ausrichtung ist eine Variable mit acht Werten, aus denen die Positionen der Anschlüsse berechnet werden; die Konfliktprüfung kann die Ausrichtungen direkt durchprobieren und muss sie nicht als relative Positionen von Anschlüssen umschreiben.
  - Eine allgemeine Definition ist keine Bedingung: jeder aus einer Anwendung expandierte Fakt ist eine eigene Einheit, die für sich geordnet und verworfen wird.
- **Sich selbst widersprechende Fakten**: jeder Fakt wird zuerst für sich allein geprüft. Ein Fakt, der allein keine Lösung hat, wie ein DISTANCE von einem Objekt zu sich selbst mit `min=4` (9.5), widerspricht sich selbst: er schlägt fehl (7.6) und nimmt an der Auflösung nicht teil. Nur die Fakten, die für sich eine Lösung haben, werden geordnet und aufgenommen.
- **Reihenfolge der Auflösung**: die geometrischen Fakten werden in eine einzige Reihenfolge gebracht. Sie werden zuerst nach `priority` geordnet, `"high"` vor `"normal"` vor `"low"` (14.8); bei gleicher Priorität nach Position, der spätere zuerst. Positionen:
  - nach Ebene und Textreihenfolge: die Wurzelpakete (16.3), der Inhalt der oberen Ebenen, die Anweisungen dieser Ebene, die mit `scope` von außen in diese Ebene geschriebenen Anweisungen (die aus äußeren Ebenen später, 2.6); der Inhalt eines anderen Pakets steht an der Stelle des zu dieser Ebene nächstgelegenen USE-PACKAGE (16.2);
  - aus einer allgemeinen Definition expandierte Fakten nehmen die Position der AS- oder ATTACH-Anweisung ein, und schrittweise generierte Fakten nehmen die Position der Generierungsregel ein (10.5);
  - erzeugt eine Anweisung mehrere Fakten, folgen die einer Expansion der Reihenfolge des Definitionsrumpfes, die einer Anweisung für jedes Mitglied der Reihenfolge der Mitglieder, und bei PLACE steht x vor y; in jedem Fall kommt der spätere zuerst. Die Segmente einer Pipe sind eigene Beschreibungen, und das spätere Segment gilt als später geschrieben (5.5): die Fakten von `#R1 @PLACE(0, 0) => @PLACE(5, 5)` kommen in der Reihenfolge y = 5, x = 5, y = 0, x = 0, und in `#M1 @PLACE(0, 0) => @ORIENT("R90")` kommt ORIENT vor PLACE.
- **Auflösung**: die Fakten werden einzeln in der Auflösungsreihenfolge aufgenommen; nach jedem Fakt werden die bisher aufgenommenen Fakten auf eine Lösung geprüft, und gibt es keine, wird dieser Fakt verworfen. Das Ergebnis ist eindeutig: es ist die konsistente Teilmenge, die in dieser Ordnung lexikografisch am größten ist. Fakten haben eine Lösung, wenn reelle Koordinaten existieren — und für jede freie Ausrichtung einer ihrer acht Werte —, die sie alle innerhalb der Toleranz erfüllen (11.7); ganzzahlige Punkte sind lediglich ein Ziel des Solvers.
- **Limit**: die Prüfung auf eine Lösung kann lange dauern, da manche Fakten auf eine von mehreren Weisen gelten, die der Reihe nach probiert werden müssen: eine Liste von Ausrichtungen (11.2, 14.9) und eine freie Ausrichtung. Eine Implementierung sollte daher ein geeignetes Limit für die auf die Prüfung verwendete Zeit oder Arbeit setzen, damit eine große Schaltung nicht zu lange dauert. Das Limit ist eine vom Benutzer einstellbare Option des Solvers (14.8), vom Werkzeug gegeben, nicht von der Datei (14.10); dieser Standard legt weder seine Messgröße noch seinen Wert fest. Wird die Prüfung für einen Fakt nicht innerhalb des Limits fertig, wird der Fakt mit einer Warnung verworfen, die angibt, dass er nicht entschieden wurde (7.6); das Ergebnis kann dann von dem oben definierten sowie von Lauf zu Lauf abweichen. Ohne Limit und immer dann, wenn keine Prüfung es erreicht, ist das Ergebnis das oben definierte.
- Die aufgenommenen Fakten sind für den Solver sämtlich hart; die verworfenen haben keine weitere Wirkung. Fakten mit der Priorität `"disabled"` werden vor dem Ordnen entfernt (14.8).
- Hinweis: Dieser Abschnitt legt fest, welche Fakten behalten werden, nicht wie sie gefunden werden. Eine Implementierung darf eine ganze Folge von Fakten auf einmal prüfen und erst nach dem zu verwerfenden Fakt suchen, wenn die Folge keine Lösung hat, oder nur jene Fakten prüfen, die Objekte mit dem neuen teilen, solange sie dieselben Fakten behält.
- Eine lösende Implementierung muss Konflikte gemäß diesem Abschnitt auflösen, und ihr Ergebnis muss alle aufgenommenen Fakten erfüllen. Eine Implementierung, die nicht löst, zeichnet ausschließlich die explizite Geometrie und muss Konflikte zwischen Anweisungen nicht prüfen (Prinzipien 2.4).
- Das Verwerfen ist eine zulässige Auflösung und liefert einen Hinweis, den eine Implementierung zu einer Warnung hochstufen darf. Wird er gegeben, nennt der Hinweis jede Menge aufgenommener Fakten, die zusammen mit dem verworfenen Fakt keine Lösung hat und aus der kein Fakt weggelassen werden kann, jeden Fakt über die Anweisung, die ihn geschrieben hat (7.6). Jede Anweisung, deren Fakten zu einer solchen Menge gehören, erhält ebenfalls einen Hinweis, der die ihretwegen verworfenen Fakten nennt, sodass der Verfasser sieht, welche Anweisungen durch sie ihre Wirkung verloren haben. Wird das Auffinden all dieser Mengen innerhalb des Zeitlimits nicht fertig, besagt der Hinweis, dass seine Liste unvollständig ist. Das Verwerfen eines `"low"`-Fakts ist zu erwarten, und seine Hinweise dürfen entfallen (7.6): eine Implementierung muss weder die Mengen von Fakten aufführen, mit denen er in Konflikt steht, noch den Anweisungen darin einen Hinweis geben.
- Ein Konflikt ist kein Fehlschlag (7.5). Konflikte höherstufiger Bedeutung zwischen Anweisungen sowie die Übereinstimmung mit der Netzliste müssen nicht geprüft werden (Prinzipien 2.4).

<!-- translated from en/ch09.md sha256:6683c3ad47d6 -->

## 9 Definitionen

### 9.1 Allgemeine Regeln

- Es gibt drei Arten von Definitionen: DEFINE-SYMBOL erzeugt ein Symbol, DEFINE-STYLE erzeugt einen Stil, und DEFINE erzeugt eine allgemeine Definition. Symbole und Stile sind die spezifischen Definitionen (1.4.4). Jede von ihnen nimmt eine Menge als Rumpf; die Anweisungen eines Rumpfes sind einzeln tolerant (7.3).
- Eine Definition ist ein Objekt. Sie wird mit `$name =` oder `%name =` benannt (6.6) und als Objekt eines Schlüsselworts in dem Slot verwendet, den das Schlüsselwort dafür festlegt (5.6).
- Das Festschreiben einer Definition veröffentlicht lediglich ihren Namen. Eine Definition zeichnet selbst nichts und wird erst bei ihrer Verwendung wirksam: ein Symbol, wenn SET-SYMBOL es zuweist, ein Stil, wenn SET-STYLE ihn zuweist, eine allgemeine Definition, wenn AS oder ATTACH sie anwendet.
- Eine direkt als Mitglied eines Eintrags geschriebene Definition, die nicht in einer Anweisung verschachtelt ist und keinen Namen hat, wird niemals verwendet und liefert eine Warnung. Eine in einer Anweisung verschachtelte anonyme Definition wird wie üblich verwendet.
- **Einbindung**: ein Verweis auf eine Definition derselben Art im Rumpf von DEFINE-SYMBOL oder DEFINE-STYLE bindet deren Inhalt an jener Stelle ein. Später geschriebene Mitglieder mit Schlüssel überschreiben früher geschriebene; Mitglieder ohne Schlüssel summieren sich auf. Es gibt keinen Parameter `base=`. Eine Einbindung, die unmittelbar oder über andere Einbindungen bzw. über die Symbole von in einem Symbolrumpf gezeichneten Anwendungen (9.2.7) die gerade erstellte Definition zurückbringt, schließt einen Zyklus: diese Einbindung schlägt fehl, ihre Diagnosemeldung nennt die Kette, und der Rest des Rumpfes wird wirksam. Der Inhalt einer Definition wird ausgehend von dieser Definition gebildet, sodass bei `$A` mit Einbindung von `$B` und `$B` mit Einbindung von `$A` `$A` aus seinem eigenen Inhalt und dem von `$B` ohne `$A` besteht und `$B` umgekehrt.
- Allgemeine Definitionen binden einander nicht ein; sie werden durch AS und ATTACH in ihren Rümpfen kombiniert. Ableitung und Erweiterung funktionieren ebenso; es gibt keine Vererbungsbeziehung (9.4).

### 9.2 Symbole: DEFINE-SYMBOL

#### 9.2.1 Auswertung

- Ein Symbol ist ein Wert: sein Rumpf wird dort ausgewertet, wo es definiert ist, und die Anweisungen darin erzeugen Pins und Grafiken in den lokalen Koordinaten des Symbols. Diese werden zum Inhalt der Definition und werden nicht in die Szene festgeschrieben.
- Die Reihenfolge der Mitglieder ist die Zeichenreihenfolge.
- Lokale Koordinaten haben dieselbe Einheit und dieselben Achsen wie die Szene (x nach rechts, y nach oben), und (0, 0) ist der Ursprung des Symbols (6.5); diese Version kennt keine Skalierung (11.7). Ein Punkt im Rumpf muss auf beiden Achsen Zahlen angeben; `_` kann nicht verwendet werden.

#### 9.2.2 Anweisungen im Rumpf

- Der Rumpf darf ausschließlich Grafiken, Pins, benannte lokale Punkte (wie `%tip = (0, 0.7)`), Einbindungen anderer Symbole, nur zum Zeichnen angewendete Anwendungen (mit ATTACH oder AS, 9.4) samt den darauf geschriebenen SET-SYMBOL und ORIENT (9.2.7) sowie an das besitzende Objekt `?`, also das mit diesem Symbol gezeichnete Objekt, angehängtes TEXT samt dem einen solchen Text positionierenden PLACE enthalten (13.3).
- Jede andere Anweisung schlägt fehl und wird verworfen (der Rumpf ist tolerant); Host-Verweise schlagen ebenfalls fehl.
- Pins dürfen nur im Rumpf von DEFINE-SYMBOL stehen. Grafiken dürfen auch in der Szene stehen, in Szenenkoordinaten; dort sind sie Anmerkungen (13.4).

#### 9.2.3 Parameter

- `box=[x1, y1, x2, y2]`: der Rahmen des Symbols, in lokalen Koordinaten und achsenparallel; er wird mit der Ausrichtung der Instanz transformiert. Die Sprache selbst verwendet ihn nicht; Viewer verwenden ihn üblicherweise als Auswahlrahmen und Solver als Grenze zur Trennung von Symbolen, und seine Verwendung bleibt der Implementierung überlassen. Wird er nicht angegeben, ist er der umschließende Rahmen aller Grafiken im Symbol außer Text, da die Größe von Text von Schriftart und Renderer abhängt.

#### 9.2.4 Pins

```rnl
$RES_BOX = @DEFINE-SYMBOL {
  %p = (0, 3) @PIN(terminal=1, label="P", direction="up");
  %n = (0, -3) @PIN(terminal=2, label="N", direction="down");
  @LINE {%p (0, 1.5)};
  @LINE {(0, -1.5) %n};
  (0, 0) @RECTANGLE(width=1.2, height=3)
}
```

- **Ein Pin ist ein Objekt des Symbols**: jede PIN-Anweisung erzeugt einen Pin, einen Punkt, der Anschlüsse binden darf. Im Rumpf kann ein benannter Pin als Punkt verwendet werden, wie die obigen Zuleitungen bei `%p` beginnen und bei `%n` enden. Sein lokaler Name ist auch sein Name als Unterobjekt jedes mit dem Symbol gezeichneten Objekts, wie `#R1::p` (6.5); ein Pin ohne lokalen Namen hat keinen Namen und wird über den von ihm gebundenen Anschluss angesprochen (5.2.4).
- **Namen werden einmal gebunden**: ein Pin-Name wird in einem Symbol einmal gebunden, die Pins eingebundener Symbole mitgezählt (9.2.7, 6.6); ein an zwei Pins gebundener Name ist ein Fehler. Pins ersetzen einander nicht: jede PIN-Anweisung erzeugt einen eigenen Pin.
- Pins sollten auf ganzzahligen Punkten liegen (11.7): liegt die Instanz auf einem ganzzahligen Punkt, fallen auch ihre Pins und die daran angeschlossenen Leitungen auf ganzzahlige Punkte. Andere Positionen sind kein Fehler; eine Implementierung darf einen Hinweis ausgeben. Grafiken in einem Symbol unterliegen dem nicht.
- `terminal=`: der Anschluss, den der Pin bindet, ein Name oder eine Nummer; dies sind die Anschlussnamen des Symbols. SET-SYMBOL gleicht sie mit den Anschlüssen des Objekts ab (11.3): einen Namen mit dem gleichnamigen Anschluss, der durch DEVICE (6.12) oder den Portnamen der Teilschaltung gegeben ist, eine Nummer mit dem Anschluss an jener Stelle. Bei einer Anwendung, die keine Anschlüsse hat, wird ein Name mit der gleichnamigen Rolle in ihrer Definition abgeglichen, und eine Nummer bindet keine Rolle (11.3, 12.1). `terminal=` darf weggelassen werden: der Pin bindet nichts und wird nur gezeichnet, etwa die Umschaltkontakte eines als Figur gezeichneten Schalters.
- **Mehrere Pins eines Anschlusses**: ein Anschluss darf an mehreren Pins gezeichnet werden, etwa eine Versorgung auf beiden Seiten eines großen Blocks. Jeder ist ein eigener Pin und wird über seinen Namen angesprochen; wo der Anschluss für einen Pin steht, steht er für den zuletzt geschriebenen Pin (5.2.4). Jeder Pin ist für sich verdrahtet oder nicht (10.2).
- **Bus-Pins**: `terminal=` kann ein Array annehmen, was einen Pin für mehrere Anschlüsse ergibt, etwa einen Bus-Port einer Teilschaltung: `(-4, 0) @PIN(terminal=["D<3>", "D<2>", "D<1>", "D<0>"], label="D<3:0>", direction="left")`. Das Array hat mindestens ein Element, und jedes Element ist eine Nummer oder ein Name. Zeichenketten in Parametern expandieren keine Subskripte (6.11), daher werden die Namen einzeln geschrieben.
  - Alle diese Anschlüsse werden an diesem Pin gezeichnet: eine an einen von ihnen angeschlossene Leitung oder Bedingung ist an diesen Pin angeschlossen, und der Pin ist verdrahtet (10.2).
  - Mehrere Pins am selben Punkt sind kein Ersatz: gleiche Koordinaten verbinden nicht (13.1), eine Leitung schließt nur an einen der Pins an, die anderen bleiben `wired=false`, und die Ergänzungsregeln des Standardpakets würden ihnen Beschriftungen und Massen hinzufügen.
- `label=`: der am Pin angezeigte Text, nur zur Darstellung. Er ist kein Name: der Pin wird über den Namen angesprochen, an den er im Symbol gebunden ist, etwa `p` in `%p = (0, 3) @PIN(…)`, geschrieben `#R1::p`, und der Anschluss über seinen Anschlussnamen wie `#R1:1`; `label="P"` liefert weder `#R1::P` noch `#R1:P` (6.4, 6.5). Ob und wo er angezeigt wird, liegt beim Viewer. Er darf Textauszeichnung verwenden, wie `\overline{CS}` (13.6).
- **Ein nicht gezeichneter Anschluss hat keinen Pin.** Ein Anschluss ohne Pin hat für das Zeichnen keine Bedeutung: eine daran angeschlossene Leitung oder Bedingung schlägt fehl („das Symbol hat keinen Pin für diesen Anschluss“); und die Ergänzung wählt ihn nicht aus (10.1). Es gibt keine Notation, um einen Anschluss mit Pin auszublenden; um das Bulk wegzulassen, verwendet man ein dreianschlüssiges Symbol ohne Bulk-Pin.
- `direction=`: die Richtung, in der eine Leitung den Pin verlässt: `"right"`, `"up"`, `"left"` oder `"down"`. Sie ist in den lokalen Koordinaten des Symbols angegeben und dreht sich mit der Ausrichtung (11.2). Sie ist ein Hinweis für Solver und Viewer, etwa die Richtung, in der freie Leitungsführung den Pin verlässt, die Stromrichtung am Anschluss (15.2) oder eine Referenz bei der Wahl von Ausrichtungen; für sich allein ist sie keine Bedingung und nimmt nicht an Konflikten teil, aber auf den Pin geschriebene ORIENT und ORIENT-RELATION schränken ihn ein (11.2, 14.9). Sie darf weggelassen werden; dann hat der Pin keine Richtung: eine Implementierung darf eine aus den Grafiken als Hinweis ableiten, aber ORIENT und ORIENT-RELATION auf jenem Pin schlagen fehl.
- Es gibt keine Zuleitungslänge: Zuleitungen werden im Symbol mit LINE gezeichnet.
- Es gibt keinen elektrischen Typ: Pins und Anschlüsse haben keinen Parameter wie Eingang oder Ausgang, und die Netzliste hat ebenfalls keine solche Angabe. Wo dies benötigt wird, drücken Benutzer es über Anwendungen semantischer Definitionen aus; ein Exporter für Werkzeuge wie Virtuoso bildet allgemeine Definitionen nach eigener Konfiguration auf elektrische Typen ab.
- Ein Pin ist nur ein Punkt: ob eine Zuleitung gezeichnet wird, liegt beim Symbol, und wie der Punkt des Pins markiert wird (ein kleines Quadrat, ein Punkt oder gar nicht), liegt beim Viewer. Ein Pin ohne Zuleitung braucht also keine besondere Notation: man schreibt PIN und kein LINE. Beispielsweise wird eine Zeichnung, deren Leitungen an einen vom Editor verborgenen Bulk-Pin anschließen, als Symbol importiert, das den Bulk-PIN behält und keine Bulk-Zuleitung zeichnet. Gewöhnliche Symbole, einschließlich der des Standardpakets, zeichnen Zuleitungen.

#### 9.2.5 Grafiken

Positionen sind Punktobjekte; Skalare sind Parameter. In der Tabelle steht *point* für ein beliebiges Punktobjekt.

| Form | Bedeutung |
|---|---|
| `@LINE {point point …}` | Zwei Punkte ergeben ein Liniensegment, mehr einen Linienzug; liegt der letzte Punkt an derselben Position wie der erste, ist die Figur geschlossen und die Stoßstelle wird als Ecke gezeichnet |
| `@RECTANGLE {point point}` | Ein Rechteck; die beiden Punkte sind gegenüberliegende Ecken in beliebiger Reihenfolge |
| `(0, 0) @RECTANGLE(width=4, height=2)` | Ein Rechteck; der Punkt ist der Mittelpunkt, und `width` und `height` sind Breite und Höhe |
| `@CIRCLE {point point}` | Ein Kreis; der erste Punkt ist der Mittelpunkt und der zweite liegt auf dem Kreis |
| `(0, 0) @CIRCLE(radius=1)` | Ein Kreis; der Punkt ist der Mittelpunkt |
| `@ELLIPSE {point point}` | Eine Ellipse; die beiden Punkte sind gegenüberliegende Ecken ihres umschließenden Rahmens |
| `(0, 0) @ELLIPSE(width=4, height=2)` | Eine Ellipse; der Punkt ist der Mittelpunkt, und `width` und `height` sind die ihres umschließenden Rahmens |
| `@ARC {point point point}` | Ein Kreisbogen vom ersten Punkt durch den zweiten zum dritten |
| `@BEZIER {point point point point …}` | Eine kubische Bézier-Kurve aus 3n+1 Punkten: der Startpunkt, dann für jedes Segment zwei Kontrollpunkte und ein Endpunkt; benachbarte Segmente teilen ihre Endpunkte |
| `(1, 0) @TEXT("…")` | Ein an einen Punkt angehängter Text: fester Text des Symbols, der sich von Instanz zu Instanz nicht ändern kann |
| `? @TEXT("${name}", key="name") => @PLACE(1, 0)` | Ein an das besitzende Objekt (das mit diesem Symbol gezeichnete Objekt) angehängter Text, der dessen Attribute liest; sein Schlüssel ist `name`, sodass er für M1 `#M1::name` ist, und ein Text mit demselben Schlüssel in der Szene überschreibt ihn Parameter für Parameter (13.3) |

- **Zwei Formen**: RECTANGLE, CIRCLE und ELLIPSE werden danach unterschieden, ob ein Größenparameter angegeben ist. Mit einer Größe (`width` und `height` für RECTANGLE und ELLIPSE, `radius` für CIRCLE) nimmt der Objekt-Slot einen Punkt, den Mittelpunkt; erhält er eine Menge, zeichnet er für jeden Punkt eine Figur (5.2.1). Erhält er das Ergebnis einer Auswahl, wie `# @SELECT-PIN => @RECTANGLE(width=0.4, height=0.4, fill="linecolor")`, wird er wie TEXT in Schritt 6 von 8.2 schrittweise generiert und sieht somit die Symbole; eine mit zwei Punkten geschriebene Grafik nimmt keine Auswahl (10.4): wie viele Punkte eine Auswahl liefert, ist beim Lesen nicht bekannt, und die Form verlangt genau zwei. Ohne Größe nimmt der Objekt-Slot eine Menge aus zwei Punkten, und eine andere Punktzahl lässt die Grafik fehlschlagen. Eine nur teilweise angegebene Größe ist ein fehlerhafter Parameter, und die Grafik schlägt fehl. In der Zweipunktform sind die Punkte Objekte: sie können benannt oder die Positionen von Pins sein, und die Figur folgt ihnen.
- Rechtecke und Ellipsen sind achsenparallel; ein schräges Rechteck wird mit einem geschlossenen LINE gezeichnet.
- **Füllung**: jede Grafik außer TEXT nimmt `fill=`. Ein offenes LINE, ARC oder BEZIER wird als das Gebiet gefüllt, das von einer geraden Linie vom letzten Punkt zum ersten geschlossen wird, wobei diese Schlusslinie nicht gezeichnet wird; ein gefülltes ARC ist das von Sehne und Bogen begrenzte Kreissegment. Eine gefüllte, aber nicht umrandete Figur (das Gate eines MOS, eine Pfeilspitze, ein Knotenpunkt) wird `fill="linecolor", linestyle="none"` geschrieben (9.3).
- **Pfeilspitzen**: LINE, ARC und BEZIER nehmen `arrow=`: `"none"` (Standard), `"start"`, `"end"` oder `"both"`, wie `@LINE(arrow="end") {%from %to}`. Die Spitze liegt am Start- oder Endpunkt und zeigt in die Richtung der Linie an jenem Ende: bei LINE das erste oder letzte Segment; bei ARC die Tangente des Bogens am Endpunkt; bei BEZIER die Gerade vom benachbarten Kontrollpunkt zum Endpunkt (fallen beide zusammen, der nächste Punkt, der nicht zusammenfällt). Die Länge der Pfeilspitze gibt `arrowsize` vor (9.3), und ihre Form bleibt der Implementierung überlassen. Die Endpunkte sind Punktobjekte; verschieben Bedingungen sie, folgt die Pfeilspitze in beliebigem Winkel, während eine an einem Punkt gezeichnete Pfeil-Anwendung nur acht Ausrichtungen hat. WIRE nimmt kein `arrow=`.
- **Stilparameter**: Grafiken dürfen Stileigenschaften direkt als Parameter annehmen (9.3): LINE, ARC und BEZIER nehmen `linecolor`, `linestyle`, `linewidth`, `cap`, `join`, `fillcolor` und `arrowsize`; RECTANGLE, CIRCLE und ELLIPSE nehmen die ersten sechs; TEXT nimmt `textcolor`, `size`, `font`, `bold`, `italic` und `textvisible`. Die angegebenen sind der direkte Stil der Grafik, der über dem vom besitzenden Objekt weitergegebenen Stil rangiert (8.3), sodass sich eine im Symbol festgelegte Farbe mit dem Thema nicht ändert; die nicht angegebenen stammen aus dem Stil des besitzenden Objekts, sodass ein Themenwechsel keine Änderung am Symbol erfordert. Die Füllung wird mit `fill=` geschrieben: `"none"` (Standard), `"linecolor"` (die Farbe von `linecolor`) oder `"fillcolor"` (die Farbe von `fillcolor`).
- **Nicht vorgesehen**: SVG-Pfadzeichenketten, POLYGON (geschlossenes LINE verwenden), eine eigene Pfeilgrafik (für Pfeilspitzen an Linienenden siehe oben; sonst ein gefülltes geschlossenes LINE oder eine an einem Punkt gezeichnete Pfeil-Anwendung verwenden, 9.2.7), abgerundete Rechtecke, elliptische Bögen (mit BEZIER annähern) sowie Bilder.
- **Text**: TEXT hat außerdem `size=` (Schriftgröße, 11.7), `halign=` (waagerecht: `"start"` (Standard), `"middle"`, `"end"`), `valign=` (senkrecht: `"baseline"` (Standard), `"top"`, `"middle"`, `"bottom"`) und `rotate=` (0, 90, 180 oder 270; Standard 0). In einem Symbolrumpf verweist ein an `?` angehängtes TEXT auf die Attribute des besitzenden Objekts, und ein an einen Punkt angehängtes ist fester Text; in der Szene ist ein TEXT je nachdem, woran er angehängt ist, eine Anmerkung oder eine Beschriftung (13.3, 13.4). Textauszeichnung wird in 13.6 beschrieben.
- **Grundlinie**: bei `valign="baseline"` liegt der Anker (anchor) auf der Grundlinie der ersten Zeile, und weitere Zeilen gehen nach unten, wie in SVG, PostScript und den meisten Editoren; `"top"`, `"middle"` und `"bottom"` richten das gesamte Textfeld aus. Die Grundlinie ist der Standard.
- **Textrichtung**: der Anker ist die eigene Position des Texts (13.3). Das Textfeld wird gemäß `halign` und `valign` neben dem Anker platziert und dreht sich um den Anker um `rotate`, einen Winkel in Grad gegen den Uhrzeigersinn, eine beliebige Zahl wie `30`; ein Text in einem Symbol wird dann mit der Ausrichtung der Instanz transformiert, sowohl Drehungen als auch Spiegelungen. Bei `upright=true`, dem Standard, stehen die Zeichen im Feld aufrecht lesbar: waagerechter Text von links nach rechts, senkrechter Text von unten nach oben, und Glyphen werden niemals gespiegelt; ein Text, dessen Endwinkel nach der Ausrichtung größer als 90 Grad und höchstens 270 ist, wird um 180 Grad um seinen Anker gedreht. So wird `rotate=180` als 0 und 270 als 90 dargestellt, mit dem Textfeld auf der anderen Seite des Ankers; wird die Instanz gespiegelt, wird der Anker wie üblich gespiegelt und das Textfeld wandert in Spiegelungsrichtung auf die andere Seite des Ankers, die Zeichen werden jedoch nicht gespiegelt. Dies gilt für jede Ausrichtung: `start` und `end` tauschen die Plätze, ebenso `top` und `bottom`. Ein Text mit `valign="baseline"` wird auf dieselbe Weise gedreht: er hängt dann um so viel unter seinem Anker, wie er darüber stand. Ein Text mit `fix_rotate=true` behält die Richtung seiner Zeichen bei, und nur die Seite seines Feldes folgt der Ausrichtung (13.3). Bei `upright=false` wird ein Text niemals für die Lesbarkeit gedreht: seine Zeichen folgen der Drehung und Spiegelung exakt, einschließlich gespiegelter Glyphen, was sich für Pfeile und andere als Text geschriebene Zeichen wie `→` eignet. Beide wirken zusammen: `fix_rotate` bestimmt den Endwinkel (`rotate` in der Szene, oder `rotate` mit dem Objekt transformiert, wobei eine Spiegelung das Vorzeichen umkehrt), und `upright` bestimmt, ob dieser Winkel dann zum Lesen um 180 Grad gedreht wird; ein Text mit `fix_rotate=true` wird niemals gespiegelt. Beispielsweise hat der Name eines NMOS seinen Anker bei (1.5, 1.5) und erstreckt sich mit `halign="start"` nach rechts; bei `"MY"` wandert der Anker nach (−1.5, 1.5) und der Text erstreckt sich nach links, vom Rumpf weg; bei `"R90"` wandert der Anker nach (−1.5, 1.5) und der Text verläuft senkrecht und liest sich von unten nach oben. Texte in der Szene drehen sich nicht mit Instanzen.
- **Beschriftungen in einer Spalte**: die Texte, die ein Symbol bei demselben x, mit demselben `halign`, `valign`, `rotate`, `fix_rotate` und `upright` und mit nichtleerem Inhalt an `?` anhängt, bilden eine Spalte, von oben nach unten gelesen, wie der Name über dem Wert eines Widerstands. Würde die Ausrichtung des Objekts eine Spalte umkehren — indem sie sie auf den Kopf stellt (`"MX"`, `"R180"`) oder auf die Seite legt, sodass sie von rechts nach links liest (`"R270"`, `"MXR90"`) —, tauschen ihre Texte die Plätze, der erste nimmt den Platz des letzten ein usw., sodass die Spalte dort, wo sie steht, in ihrer Reihenfolge lesbar bleibt. Texte mit unterschiedlichen Ausrichtungen, wie ein Name über einem Rahmen und eine Formel in dessen Mitte, sind keine Spalte, und ein Text, dessen Position die Szene angibt (13.3), nimmt nicht teil.

#### 9.2.6 Blocksymbole

Geschrieben innerhalb der Teilschaltung `.subckt ota5t … .ends`:

```rnl subckt
%ota5t = @DEFINE-SYMBOL(box=[-4, -4, 4, 4]) {
  (-4, 1) @PIN(terminal="inp");
  (-4, -1) @PIN(terminal="inn");
  (4, 0) @PIN(terminal="out");
  @LINE {(-4, -4) (-4, 4) (4, 0) (-4, -4)};
  ? @TEXT("${name}", key="name") => @PLACE(-1, 0)
};
# @SET-SYMBOL %ota5t
```

- Das Symbol, das eine Teilschaltung ihrem eigenen `#` zuweist, ist das Standardaussehen der Instanzen dieser Teilschaltung, wo immer sie stehen: ihr Blocksymbol. Die Pins entsprechen den Ports der Teilschaltung nach Namen.
- Es ist die Ausnahme von der Regel aus Prinzipien 2.5, dass Definitionen nur nach unten wirken: es gilt für die Ebenen, die die Teilschaltung instanziieren.
- Die Bus-Ports einer Teilschaltung sind in der Netzliste bereits in einzelne Ports aufgespalten (6.11); ein Blocksymbol zeichnet sie mit einem Bus-Pin gemeinsam als einen (9.2.4).
- In der Symbol-Warteschlange steht es nach den Regeln der Pakete an der Wurzel und vor den Regeln der Ebenen, einschließlich der in einer Ebene verwendeten Pakete, sodass eine obere Ebene es durch eine Regel oder ein SET-SYMBOL für eine einzelne Instanz ersetzen kann (8.3).
- Keine Instanz instanziiert eine Dateiebene, daher hat in einer Dateiebene geschriebenes `# @SET-SYMBOL` keine Wirkung.
- Ein Verilog-A-Modul gibt sein Blocksymbol auf dieselbe Weise, mit `# @SET-SYMBOL` innerhalb des Moduls in seiner Verilog-A-Datei; seine Pins entsprechen den Ports des Moduls nach Namen, und ein Vektorport entspricht einem Anschluss für jedes Bit (2.9).
- **Das Standard-Blocksymbol.** Eine Instanz, der keine Regel, keine Anweisung und kein Blocksymbol ein Symbol zuweist, wird mit dem Standard-Blocksymbol gezeichnet, das aus ihren Anschlüssen gebildet wird:
  - **Pins.** Es hat einen Pin für jeden Anschluss. Bei einer Instanz einer Teilschaltung oder eines Moduls, das die Datei liest (2.9), sind es die Ports in der Reihenfolge der Definition, jeder Pin nach seinem Port benannt, etwa `terminal="inp"`. Bei jeder anderen Instanz, etwa der einer Zelle, deren Definition die Datei nicht liest, oder eines Bauteils, dessen Art keine Regel angibt, sind es die Knoten auf ihrer Zeile, jeder Pin nach seiner Nummer benannt, etwa `terminal=1`. Die erste Hälfte der Pins steht auf der linken Seite und der Rest auf der rechten; ist ihre Zahl ungerade, hat die linke Seite einen mehr. Jeder Pin ist im Rahmen mit dem Namen seines Ports beschriftet, sonst mit dem Namen seines Anschlusses (6.12), sonst mit seiner Nummer.
  - **Rahmen.** Der Rahmen ist ein Quadrat, dessen Seite doppelt so lang ist wie die Zahl der Pins auf der linken Seite, mindestens aber 2; sein Mittelpunkt ist der Ursprung des Symbols. Die Pins haben den Abstand 2 und zeigen aus dem Rahmen heraus: die linken laufen von 1 unterhalb der oberen Ecke nach unten, und die rechten sind auf ihrer Seite zentriert. So hat eine Instanz mit einem Anschluss einen Rahmen der Seitenlänge 2 mit ihrem Pin in der Mitte der linken Seite, und eine mit drei einen Rahmen der Seitenlänge 4 mit Pins bei (−2, 1), (−2, −1) und (2, 0); jeder Pin liegt auf ganzen Einheiten.
  - **Texte.** Über dem Rahmen steht der Name der Instanz (`key="name"`) und darunter `${model}` (`key="model"`), der Name der Teilschaltung oder des Modells (13.6).
  - **Platz in der Warteschlange.** Es kommt nach der gesamten Symbol-Warteschlange (8.3), außerhalb der Gruppe der Blocksymbole, sodass jede Regel oder Anweisung, die der Instanz ein Symbol zuweist, es ersetzt, ebenso das Blocksymbol ihrer Teilschaltung. Es zählt als Symbol für `drawn` und passt auf kein `symbol=` (10.1).
  - Damit hat jede Instanz ein Symbol, mit einem Pin für jeden ihrer Anschlüsse, sofern ein ihr zugewiesenes Symbol nicht einen auslässt (9.2.4).
- Hat eine Teilschaltung mehrere `# @SET-SYMBOL`-Anweisungen, stehen sie in derselben Gruppe der Symbol-Warteschlange in Textreihenfolge, und die spätere wird wirksam (8.3); eine Implementierung darf eine Warnung ausgeben.

#### 9.2.7 Varianten und Fragmente

```rnl
$NMOS_BULK = @DEFINE-SYMBOL(box=[-3, -3, 2, 3]) {
  $NMOS;
  %b = (1, 0) @PIN(terminal=4, label="B", direction="right");
  @LINE {(0, 0) %b}
}
```

- Das Einbinden eines anderen Symbols in einem Rumpf und anschließendes Hinzufügen erzeugt eine Variante: Pins und Grafiken summieren sich auf, und die Pin-Namen des eingebundenen Symbols zählen als Namen der Variante (9.2.4). Um einen Pin zu verschieben, teilt man den gemeinsamen Teil in Fragmente auf und schreibt die Pins in jedem Symbol, wie es das Standardpaket tut.
- Ein nur in der eigenen Ebene verwendetes Fragment wird mit `%` benannt, wie `%MOS_CHANNEL = @DEFINE-SYMBOL {…}`, und ein anderer Symbolrumpf bindet es mit `%MOS_CHANNEL;` ein.
- Grafiken haben keinen Schlüssel: Einbinden fügt nur hinzu, und eine eingebundene Grafik kann nicht entfernt werden. Um weniger zu zeichnen, definiert man ein anderes Symbol: man teilt den gemeinsamen Teil in Fragmente auf und bindet in jedem Symbol die benötigten Teile ein, etwa `$NMOS3 = @DEFINE-SYMBOL {%MOS_BODY}` und `$NMOS_4TERM = @DEFINE-SYMBOL {%MOS_BODY; %BULK}`.
- **Anwendungen im Rumpf**: eine Figur, die an verschiedenen Positionen oder in verschiedenen Ausrichtungen benötigt wird, etwa eine Pfeilspitze, wird als an einem Punkt gezeichnete Anwendung geschrieben:

  ```rnl
  $ARROWHEAD = @DEFINE(roles=[["p", "point"]]) {};
  $ARROWHEAD_SYM = @DEFINE-SYMBOL {
    @LINE(fill="linecolor", linestyle="none") {(0.4, 0) (-0.4, 0.3) (-0.4, -0.3) (0.4, 0)}
  };
  $ISRC_DOWN = @DEFINE-SYMBOL {
    (0, 3) @PIN(terminal=1, direction="up");
    (0, -3) @PIN(terminal=2, direction="down");
    @LINE {(0, 3) (0, 1.5)};
    @LINE {(0, -1.5) (0, -3)};
    (0, 0) @CIRCLE(radius=1.5);
    @LINE {(0, 0.9) (0, -0.5)};
    $ARROWHEAD @ATTACH (0, -0.6) => @SET-SYMBOL $ARROWHEAD_SYM => @ORIENT("R270")
  }
  ```

  - Die erste Rolle wird mit einem Punkt des Rumpfes in lokalen Koordinaten gefüllt. Die Anwendung wird mit ihrem Symbol an diesem Punkt gezeichnet, ihre Ausrichtung dreht sich um diesen Punkt, und sie wird dann gemeinsam mit dem umgebenden Symbol transformiert; die Transformationen geschachtelter Anwendungen setzen sich nach außen der Reihe nach zusammen, sodass sich die Pfeilspitze mit dem Bauteil dreht.
  - Nur zum Zeichnen: äußere Regeln können nicht in einen Symbolrumpf hineinsehen, daher wird ihr Symbol mit SET-SYMBOL im Rumpf zugewiesen; eine Anwendung ohne eines wird nicht gezeichnet, und eine Warnung wird ausgegeben. Die Rümpfe allgemeiner Definitionen werden innerhalb eines Symbols nicht expandiert, und Bedingungen sowie Leitungen haben dort keine Bedeutung; ORIENT ist hier internes Datum des Symbols, nimmt nicht an Konflikten teil und hat keine Priorität.
  - Ein SET-SYMBOL hier, das unmittelbar oder über Einbindungen und andere solche Anwendungen das gerade erstellte Symbol zurückbringt, schließt einen Zyklus wie eine Einbindung (9.1): dieses SET-SYMBOL schlägt fehl, die Anwendung wird nicht gezeichnet, und der Rest des Rumpfes wird wirksam.
  - Das Zeichnen einer Anwendung an dem Punkt, der ihre erste Rolle füllt, gilt nur in Symbolrümpfen, die der Solver nicht platziert. In der Szene erzeugt dieselbe Notation eine Anwendung mit einem eigenen Punkt (12.1 Punkt 5), und Regeln können ihr ein Symbol zuweisen; die Regeln eines Hausstilpakets reichen nicht in Symbole hinein.
- Pin-Namen und Formeln: ob Pin-Namen angezeigt werden, liegt beim Viewer (9.2.4); eine Formel ist einfach ein TEXT, der auf Attribute verweist (13.6), ohne besondere Regeln. Textauszeichnung wird in 13.6 beschrieben.

#### 9.2.8 Zuordnung von Symbolen zu Objekten

Symbole werden nicht in Symbole für Bauteile und Symbole für Anwendungen unterteilt. Ob ein Symbol für ein Objekt verwendet werden kann, hängt davon ab, ob seine Pins zu den Anschlüssen des Objekts passen.

- **Instanzen ohne Symbol.** Keine Instanz bleibt ohne eines: weist keine Anweisung oder Regel einer Instanz ein Symbol zu, hat sie das Standard-Blocksymbol (9.2.6) mit einem Pin für jeden Anschluss und wird wie jede andere gezeichnet und gelöst. Ein Paket darf ein Bauteil ohne Symbol lassen, wenn es seine Art nicht bestimmen kann, statt es als ein Bauteil zu zeichnen, das es möglicherweise nicht ist, wie es das Standardpaket bei einem Transistor tut, dessen Modellkarte nicht gelesen wurde; das Bauteil wird dann als Rahmen mit einem Pin für jeden Knoten gezeichnet.

### 9.3 Stile: DEFINE-STYLE

```rnl
$ink = @DEFINE-STYLE(linecolor="#202020", textcolor="#202020", fillcolor="#ffffff", size=1.2) {};
$accent = @DEFINE-STYLE(linecolor="#1f4fd8", linestyle="bold") {$ink};
$shade = @DEFINE-STYLE(fillcolor="#1f4fd833") {}
```

- Die Parameter in den Klammern sind die eigenen Eigenschaften des Stils, und sie werden zuletzt wirksam. Die Menge darf nur auf andere Stile verweisen, deren Eigenschaften an jener Stelle eingebunden werden, wobei spätere frühere überschreiben. Ein Stil ohne Basisstil wird mit `{}` geschrieben.
- Ein Stil wird erst wirksam, wenn SET-STYLE ihn zuweist (11.3). Dieselben Eigenschaften dürfen auch direkt als Parameter von Grafiken, TEXT und WIRE geschrieben werden (9.2.5, 13.1), wobei die Werte über die zwei Stufen aus 8.3 übernommen werden.
- Ein Stil bestimmt nur Konturen und Farben: Position, Ausrichtung, Form (die Punkte eines Symbols oder einer Grafik) und Textinhalt gehören nicht zu einem Stil.
- **Eigenschaften**:

| Gruppe | Eigenschaft | Werte |
|---|---|---|
| Linie | `linecolor` | Eine Farbe; auch die Farbe von `fill="linecolor"` |
| | `linestyle` | `"solid"` (Standard), `"dashed"`, `"dotted"`, `"dash-dot"`, `"double"` (zwei parallele durchgezogene Linien), `"bold"`, `"none"` (keine Linie) |
| | `linewidth` | Die Linienbreite in Koordinateneinheiten (11.7); `0` ist eine Haarlinie, bei jeder Vergrößerung gleich dünn, und ist der Standard |
| | `cap` | Linienenden: `"butt"`, `"round"`, `"square"` |
| | `join` | Ecken: `"miter"`, `"round"`, `"bevel"` |
| Füllung | `fillcolor` | Die Farbe von `fill="fillcolor"` |
| Text | `textcolor` | Die Farbe von Text; wenn nicht angegeben, `linecolor` |
| | `size` | Die Schriftgröße in Koordinateneinheiten (11.7) |
| | `font` | Ein Schriftname, nur ein Hinweis; ein Viewer ohne diese Schrift verwendet seine eigene |
| | `bold`, `italic` | Fett oder kursiv für den gesamten Text, `true` oder `false`; die Auszeichnungen `\textbf` und `\textit` können sie weiterhin auf Teile anwenden (13.6) |
| | `textvisible` | Ob der Text angezeigt wird (Textsichtbarkeit): `true`, der Standard, oder `false`. Ein Text mit `false` wird nicht gezeichnet, beansprucht keinen Platz und kann nicht ausgewählt werden: er wird bei Kollisionen, beim Rahmen eines Symbols und bei den Spalten aus 9.2.5 ausgelassen. Sein Inhalt, Schlüssel, seine Position und Attributverweise bleiben bestehen und werden wie üblich ausgewertet; ein Viewer darf anbieten, verborgene Texte anzuzeigen. Ein Paket, das Beschriftungen verbirgt, definiert einen Stil und weist ihn ihnen zu: `$hidden = @DEFINE-STYLE(textvisible=false) {};` und `# @SELECT-LABEL(recursive=true) => @SET-STYLE $hidden`; ein einzelner Text kann ihn auch direkt annehmen, wie `#M1 @TEXT("note", key="memo", textvisible=false)` |
| Sonstiges | `arrowsize` | Die Länge von Pfeilspitzen an Linienenden in Koordinateneinheiten (9.2.5) |

- Eine Linie hat nur fünf Eigenschaften: Farbe, Stil, Breite, Linienende und Ecke. `"bold"` zeichnet die doppelte Linienbreite, und bei Breite 0 die doppelte Haarlinie; Strichlängen und Lücken gestrichelter Linien sowie der Abstand von Doppellinien bleiben der Implementierung überlassen, proportional zur Linienbreite. Weitere Werte von `linestyle` dürfen in späteren Sprachversionen hinzukommen (16.1).
- `"none"` zeichnet keine Linie und auch keine Pfeilspitzen an Linienenden (sie gehören zur Linie, 9.2.5); `fill="linecolor"` übernimmt dennoch die Farbe von `linecolor`, sodass eine gefüllte, aber nicht umrandete Figur `fill="linecolor", linestyle="none"` geschrieben wird und ihre Farbe weiterhin einem Themenwechsel folgt. Sowohl Leitungen als auch Grafiken nehmen es an; eine Leitung damit ist eine unsichtbare Leitung, aber weiterhin eine Leitung (13.1).
- **Farben** werden als hexadezimale CSS-Farben geschrieben: `"#RRGGBB"` oder `"#RRGGBBAA"`, oder in den Kurzformen `"#RGB"` und `"#RGBA"`. Der Alpha-Wert steht zuletzt: `00` ist vollständig transparent und `FF` deckend; ohne ihn ist die Farbe deckend. Linien und Füllungen haben jeweils ihre eigene Farbe, sodass ihre Transparenz getrennt festgelegt wird, wie `$shade` oben nur die Füllung halbtransparent macht. Diese Version akzeptiert keine Farbnamen oder `rgb()`. Farben sind Zeichenketten.
- Diese Version bietet keine Musterfüllungen, Farbverläufe oder Schatten; keine Sichtbarkeit anderer Objekte als Texte (diese ein- und auszublenden ist Sache des Viewers, 16.2; `textvisible` betrifft nur Texte, sodass eine solche Sichtbarkeit später unter eigenen Namen hinzugefügt werden kann und ein Text dann angezeigt wird, wenn beides wahr ist); und keine Objekte wie Themen und Profile (ein Thema ist ein Paket aus Stilen und Regeln, eingebracht mit USE-PACKAGE, 16.2).
- Gibt weder ein Stil noch ein Parameter einen Wert für eine Eigenschaft vor, wird die eigene Vorgabe des Renderers verwendet; dies ist kein Fehler (Prinzipien 2.7). Das Standardpaket stellt ein portables Standardaussehen bereit (16.3), das vom Benutzer geschriebene Regeln und Anweisungen überschreiben.
- Unbekannte Eigenschaften werden wie Syntaxfehler behandelt (Prinzipien 2.3).

### 9.4 Allgemeine Definitionen: DEFINE

```rnl id=gdef
$TAIL_MID = @DEFINE(roles=["a", "b", "t"]) {
  %mid = (_, _);
  {?a %mid ?b} @DISTRIBUTE("left-to-right");
  {%mid ?t} @ALIGN("x")
};
$CHAIN_ROW = @DEFINE(roles=["source", "{chain}", "load"]) {
  {?source ?chain ?load} @ORDER("left-to-right")
};
$GND_BELOW = @DEFINE(roles=[["t", "pin"]], parameters=[["gap", 2]]) {
  %g = $GROUND @ATTACH ?t::net;
  ?t::net @WIRE {?t %g::pin};
  {?t %g::pin} @ALIGN("x");
  %g::pin ?t @DISTANCE("bottom-to-top", exact=?gap)
};
%c1 = #M1:3(a) #M2:3(b) #M5:1(t) @AS $TAIL_MID;
#R1(source) {#R2 #R3}(chain) #R4(load) @AS $CHAIN_ROW;
# @SELECT-INSTANCE => @SELECT-PIN(net="vss", wired=false) => $GND_BELOW @ATTACH(gap=3)
```

- Eine allgemeine Definition besteht aus einer Rollenliste, einer Parameterliste und einem Rumpf. Im Rumpf wird auf Rollen und Parameter mit `?name` verwiesen (6.10). Was eine Definition bewirkt, entscheidet ihr Inhalt; siehe 9.5.
- **Anwenden**: AS und ATTACH füllen die Rollen einer Definition mit Objekten und erzeugen eine Anwendung. Die beiden Schlüsselwörter tun dasselbe und unterscheiden sich nur darin, wo die Definition steht:
  - AS hat die Definition zuletzt und liest sich „diese Objekte, als soundso“; es wird für Bedeutung und Layout verwendet, etwa `#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N`;
  - ATTACH hat die Definition zuerst, vor den ausgeschriebenen Objekten, und die Standardform setzt das Schlüsselwort hinter die Definition, wie `$GROUND @ATTACH &vss`; es liest sich „soundso, hinzugefügt zu diesen Objekten“ und wird für virtuelle Bauteile und die Ergänzung verwendet (Kapitel 12, 10.5).
  - Das jeweils andere Schlüsselwort zu wählen ist kein Fehler, nur holprig zu lesen; die Definition an die falsche Stelle zu schreiben ist ein Fehler (4.2 Punkt 7). Per Pipe übergebene Objekte werden den Rollen direkt zugeordnet, sodass beide Schlüsselwörter dann gleich aussehen; man wählt nach dem Zweck der Definition.
  - Entscheidungshilfe: man verwendet AS, wenn die Objekte zusammen das Gebilde ausmachen, etwa eine Schaltungsstruktur, ein Layout oder eine lokale Darstellungsweise aus diesen Objekten; man verwendet ATTACH, wenn das Gebilde zu den Objekten hinzugefügt wird, etwa eine Masse, eine Port-Markierung, eine Nicht-verbunden-Markierung, ein Knotenpunkt, eine Beschriftung für jede Instanz oder die Ergänzung. Die Standardsätze lauten `#M1(left), #M2(right) and #M5(tail) are treated @AS $DIFFERENTIAL_PAIR_N` und `$GROUND is @ATTACH to &vss` (4.5).
- **Rollenliste**: der Parameter `roles`, ein Array, dessen Elemente Rollennamen oder `[name, kind]`-Paare sind. Beide Parameter von DEFINE können nur mit Namen angegeben werden.
  - Die Liste darf leer sein, `roles=[]`: die Definition wird allein angewendet, wie `$PORT_LIST @ATTACH`, und die Anwendung hat kein Hauptobjekt; sie hat dennoch ihren eigenen Punkt (14.1).
  - Ein Element der Form `"{name}"` ist eine Mengenrolle, die eine ganze Menge annimmt; die übrigen sind Einzelrollen (5.2.1 Punkt 4).
  - Eine Mengenrolle, die ein einzelnes Objekt erhält, nimmt es als eine Menge nur mit diesem Objekt; eine leere Menge ist ebenfalls gültig, und Bedingungen im Rumpf, die zu wenige Mitglieder erhalten, gelten trivial (Kapitel 14).
  - Eine Einzelrolle, die eine Menge erhält, wendet die Definition einmal für jedes Mitglied an, und nullmal für eine leere Menge; in einer Anwendung darf höchstens eine Einzelrolle eine Menge mit mehr als einem Mitglied erhalten (andernfalls schlägt die Anweisung fehl, 5.2.1).
  - Eine Regel, die eine Definition anwendet, deren Rollen alle Mengenrollen sind, erzeugt in einer Ebene, in der alle leer sind, keine Anwendung: Die Regel wird in jeder Ebene ausgewertet, die sie erreicht (10.4), und eine Anwendung entsteht nur dort, wo ihre Mengenrollen etwas enthalten, wie eine Einzelrolle, die eine leere Menge erhält, die Definition null Mal anwendet. Eine Anweisung, die keine Regel ist, erzeugt ihre Anwendung auch bei leeren Mengenrollen. So erzeugt die Ergänzung der Standardserie 0.4 die Spalte der Port-Markierungen einer Ebene nur in den Ebenen, die Port-Markierungen bekommen (12.4).
  - Steht das `?name` einer Mengenrolle innerhalb von `{…}`, ist es ein Element jener Menge (5.2): in einer Bedingung steht es für jedes seiner Mitglieder an jenem Element (14.1), sodass das ORDER im obigen Beispiel R2 und R3 jeweils zwischen R1 und R4 setzt.
  - Eine Mengenrolle, die eine Menge erhält, die Mengen als Mitglieder hat, nimmt sie unverändert an; der Rumpf entscheidet, wie sie verwendet wird.
  - Eine Mengenrolle entspricht keinem Anschluss (6.4).
  - In einer Anwendung darf jede Rolle nur einmal markiert werden; um einer Mengenrolle mehrere Objekte zu übergeben, schreibt man sie als Menge, wie `{#R2 #R3}(chain)` oben.
  - **Arten**: ein Element der Form `[name, kind]` deklariert, welche Art von Objekt die Rolle annimmt; ein Element mit nur einem Namen nimmt jede Art an. Die Artnamen sind die der Wortliste (5.3): `"instance"`, `"terminal"`, `"pin"`, `"net"`, `"point"`, `"wire"`, `"usage"`, `"bundle"`, `"text"`, `"annotation"` und `"label"`. In einer Rollendeklaration nimmt `"instance"` auch Bündel an und `"terminal"` die Anschlüsse von Bündeln (11.8); ein Array bedeutet eine beliebige aus mehreren Arten, etwa `["p", ["net", "terminal"]]`. Ein Objekt einer dieser Arten wird unverändert angenommen; ein Objekt keiner dieser Arten, das sich in eine davon umwandeln lässt (5.2.4), wird in die erste davon in der Reihenfolge des Arrays umgewandelt, sodass `#R1::p` für `["net", "terminal"]` zum Netz des Pins wird. Eine Rolle, die `"net"` annimmt, nimmt auch den leeren Knoten, `&` allein, an, was besagt, dass die Rolle auf keinem Netz liegt (6.3); eine Mengenrolle nicht, da der leere Knoten kein Mitglied einer Menge ist. Die Art einer Mengenrolle gilt für jedes ihrer Mitglieder, wie `["{members}", "instance"]`. Es gibt keine Art für Anwendungen einer bestimmten Definition.
  - Erhält eine Rolle ein Objekt der falschen Art, schlägt die gesamte Anwendung fehl, und die Diagnosemeldung nennt die Rolle, was sie annimmt und was sie erhalten hat; dasselbe gilt, wenn ein Mitglied einer Mengenrolle die falsche Art hat. Erzeugt eine Auswahl die Anwendungen schrittweise, schlägt nur die Anwendung fehl, die das falsche Objekt erhalten hat.
  - Hinweis: In einer Definition mit leerem Rumpf prüft nichts, was die Rollen füllt, sodass nur eine deklarierte Art eine falsch gefüllte Rolle erkennt; in einem nichtleeren Rumpf wird auch jede Anweisung der Expansion für sich geprüft (9.5). Deklarierte Arten ermöglichen zudem die Prüfung der Rollenverwendungen im Rumpf bereits an der Definition (6.9).
  - **Optionale Rollen und Anzahlen**: eine Einzelrolle muss gefüllt werden; eine Mengenrolle darf ungefüllt bleiben, was sie zur leeren Menge macht. Werden Objekte in Reihenfolge übergeben und sind es weniger Objekte als Rollen, müssen die restlichen Rollen sämtlich Mengenrollen sein; sind Objekte mit Rollen markiert, ist eine nicht geschriebene Mengenrolle eine leere Menge. Es gibt keine Mindest- oder Höchstzahl: Bedingungen mit zu wenigen Mitgliedern gelten trivial (Kapitel 14). Eine Variante anderer Gestalt wird als weitere Definition geschrieben (siehe Ableitung und Erweiterung unten). Gültigkeitsregeln wie dasselbe Objekt in zwei Rollen oder erwartete Verbindungen werden in 9.5 behandelt.
- **Parameterliste**: der Parameter `parameters`, der weggelassen werden darf. Jedes Element ist ein Parametername oder `[name, default]`.
  - Parameter werden in den Klammern von AS und ATTACH übergeben, wie `$GND_BELOW @ATTACH(gap=3)`, nach Position in der Reihenfolge der Parameterliste oder mit Namen; werden beide gemischt, stehen die nach Position zuerst (4.2 Punkt 9). Die Standardform schreibt sie mit Namen (4.3). Ein Name, der nicht in der Parameterliste steht, lässt die Anweisung fehlschlagen. Erzeugt eine Auswahl die Anwendungen schrittweise, verwendet jede Anwendung dieselben Parameter.
  - Überall, wo der Rumpf ein Literal enthalten darf, kann stattdessen ein Parameterplatzhalter stehen: als Wert eines Parameters, als Element eines Arrays, als Koordinate eines Punkts. Innerhalb eines Definitionsrumpfes lautet die Regel, dass Parameter nur Literale halten (4.1), also „Literale oder Parameterplatzhalter“; nach der Expansion sind es wieder Literale. Die Parameter von AS und ATTACH selbst können nur Literale sein und keine Objekte übergeben; Objekte kommen über Rollen herein.
  - Ein Parameter, der nicht übergeben wird und keinen Standardwert hat, zählt dort, wo er verwendet wird, als nicht geschrieben: als benannter Parameter ist er nicht geschrieben, etwa `size=?size` in `@TEXT(?label, size=?size)`, wonach der Text die eigene Standardgröße von TEXT oder die Größe seines Stils erhält (11.3); als Koordinate eines Punkts ist er `_`; in einem nach Position übergebenen Parameter oder einem Array-Element hat er keine definierte Bedeutung, und diese Anweisung schlägt fehl. Daher ist keine Markierung für erforderliche Parameter nötig.
  - Parameter sind für Größen wie Zahlen und Text gedacht; eine strukturell abweichende Variante sollte als gesonderte Definition geschrieben werden; die Sprache erzwingt dies nicht. Parameter werden nur in den Rumpf hinein gereicht; Anwendungen können nicht nach den Werten ihrer Parameter ausgewählt werden.
- Rollennamen und Parameternamen stehen in einer gemeinsamen Tabelle und müssen alle verschieden sein; keiner von ihnen darf ein Standardunterobjektname sein (6.5). Ein Parameter darf nicht `priority` heißen, da dies ein Standardparameter von AS und ATTACH ist (14.8).
- **Die Anwendung selbst**: im Rumpf einer allgemeinen Definition ist ein alleinstehendes `?` die gerade expandierte Anwendung über ihren eigenen Punkt (14.1). `{?members::position ?} @ALIGN("y")` setzt den Punkt der Anwendung auf die Höhe ihrer Mitglieder, sodass eine Bedingung auf diesem Punkt, etwa ein DISTRIBUTE um eine Achse in einer anderen Struktur, sie mit sich bewegt; `::position` hält ein Mitglied, das eine Anwendung ohne Symbol ist, als einen Punkt (14.1). Ein alleinstehendes `?` ist ein Objekt, keine Rolle. Es nimmt wie eine Anwendung einen Pfad an (6.5): `?::position` ist ihr eigener Punkt, an dem eine Leitung oder Grafik im Rumpf enden darf (5.2.4), und `?::` gefolgt vom Namen eines Pins ihres Symbols ist jener Pin, etwa `?::pin` für einen Pin namens pin.
- **Regeln in einem Rumpf**: ein Rumpf darf Regeln enthalten, eine Auswahl und die daraus weitergeleiteten Anweisungen (10.1). Sie wirken als Regeln der Ebene, in der die Anwendung steht, geschrieben an der Stelle ihrer AS- oder ATTACH-Anweisung (8.5, 10.4), und werden in den Schritten von 8.2 wirksam wie dort geschriebene Regeln: SET-SYMBOL in der Symbol-Warteschlange, die Regeln der Route-Schlüsselwörter in Schritt 5, Generierung in Schritt 6, SET-STYLE in der Stil-Warteschlange. In einem Rumpf ist `#` jene Ebene (10.3). Im Rumpf einer Anwendung, die selbst in Schritt 6 generiert wird, schlägt eine Regel von Schritt 5 oder 6 (10.4) fehl, und die übrigen Regeln werden in den späteren Schritten wirksam, wie 10.5 angibt.
- Die im Rumpf gebundenen `%`-Namen sind lokale Namen: jede Anwendung erzeugt neue Objekte für sie, auf die von außen über die Anwendung verwiesen wird (siehe unten). Ein von mehreren Anwendungen geteiltes Objekt, etwa eine Symmetrieachse, wird über eine Rolle hereingereicht.
- Eine Anwendung darf die Objekte mit Rollen markieren (`#M1(a) #M2(b) … @AS $T`, `$T @ATTACH #M1(a) #M2(b) …`) oder sie in der Reihenfolge der Rollenliste übergeben (`#M1 #M2 … @AS $T`). Bleibt eine Einzelrolle ungefüllt oder werden mehr Objekte als Rollen übergeben, schlägt die gesamte Anweisung fehl; eine ungefüllte Mengenrolle wird eine leere Menge.
- Eine Anwendung erzeugt eine Anwendung, die die Definition und die Füllung jeder Rolle festhält und benannt werden kann (wie `%c1`). Das Löschen der Anweisung entfernt auch die daraus expandierten Beschreibungen. Eine Anwendung hat keinen Schlüssel: jede Anwendung ist ein eigenes Objekt (5.8).
- **Expansion**:
  - Platzhalter werden durch die Argumente und Parameterwerte ersetzt;
  - lokale Namen werden durch neue Handles ersetzt, „Anwendung + lokaler Name“, geschrieben `%c1::mid`; eine unbenannte Anwendung wird in Diagnosemeldungen über die Position ihrer Anweisung angezeigt;
  - die Namen von außerhalb des Rumpfes werden am Anwendungsort nicht erneut aufgelöst (6.9);
  - das Ergebnis der Expansion wird an der Stelle der AS- oder ATTACH-Anweisung festgeschrieben und nimmt diese Position bei der Konfliktauflösung ein (8.5).
- **Koordinaten in einem Rumpf**: eine literale Koordinate im Rumpf einer allgemeinen Definition, in einem Punkt wie `(1, 0)` oder in PLACE, ist ein Versatz gegenüber dem eigenen Punkt der Anwendung (14.1), in den Achsen der Ebene: `(1, 0)` liegt eine Einheit rechts von diesem Punkt, und `(0, 0)` ist der Punkt selbst, `?::position`. Ein `_` bleibt frei, und ein Parameterplatzhalter in einer Koordinate ist nach der Ersetzung ein Versatz. Nur der Punkt verschiebt den Rumpf: ORIENT auf der Anwendung dreht ihr Symbol, nicht was ihr Rumpf zeichnet oder platziert (11.2). So kann eine Definition eine Markierung um ihren eigenen Punkt zeichnen, etwa zwei sich bei `(0, 0)` kreuzende LINEs, und ihren Punkt an einen Pin binden (12.1 Punkt 5). PLACE auf dem alleinstehenden `?` in seinem eigenen Rumpf schlägt fehl, da sein Versatz von sich selbst aus wäre. Außerhalb des Rumpfes, etwa im Eintrag des Solvers, sind Positionen Koordinaten der Ebene (14.8).
- Geschachtelte Anwendungen werden Schritt für Schritt angesprochen (`%c1::inner::mid`). Eine Anwendung kann auch eine Rolle einer anderen Definition füllen: mit einer eigenen Definition `$OTA = @DEFINE(roles=[["pair", "usage"], ["load", "usage"]]) {}` ist `#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N => %cm1 @AS $OTA` ein Verstärker, dessen Rolle pair mit der Anwendung des Differenzpaars und dessen Rolle load mit dem Stromspiegel `%cm1` gefüllt ist (5.5).
- **Zyklische Anwendung**: eine Anwendung innerhalb einer Expansion, die eine bereits in einer umgebenden Anwendung expandierte Definition direkt oder über andere Definitionen erneut anwendet, schließt einen Zyklus. Diese Anwendung schlägt fehl, und ihre Diagnosemeldung führt die gesamte Kette auf; der Rest wird wie üblich wirksam. Unten expandiert das Anwenden von `$A` auf M1 `$B`, dessen Anwendung von `$A` den Zyklus `$A → $B → $A` schließt und fehlschlägt; das ORIENT in `$B` spiegelt M1 dennoch:

  ```rnl error
  $A = @DEFINE(roles=["x"]) {?x @AS $B};
  $B = @DEFINE(roles=["x"]) {
    ?x @AS $A;               /* closes the cycle $A → $B → $A: this application fails */
    ?x @ORIENT("MY")
  };
  #M1 @AS $A
  ```

- **Von außen auf lokale Objekte verweisen**:

  ```rnl use=gdef
  $DP_BLOCK = @DEFINE(roles=["in_p", "in_n"]) {
    %pair = {?in_p ?in_n} @GROUP
  };
  %dp1 = #M1(in_p) #M2(in_n) @AS $DP_BLOCK;
  %c1::mid @PLACE(x=20);              /* moves the midpoint of c1 */
  {%c1::mid #M9:1} @ALIGN("x");       /* aligns the drain of M9 with the midpoint of c1 */
  %dp1::pair @PRIORITY("disabled")    /* turns off only the group expanded for dp1 */
  ```

  - Auf jedes benannte lokale Objekt eines Rumpfes kann von außen über die Anwendung verwiesen werden, geschrieben `usage::local_name`; unbenannte sind nicht erreichbar, außer per Nummer aus dem Eintrag des Solvers (5.9). Ob eines benannt wird, entscheidet der Autor der Definition, gemäß „um auf ein Objekt zu verweisen, benenne es“ (5.7); es gibt keine gesonderte Exportliste.
  - Pfade können verkettet werden: `%st1::dp::mid` ist das mid der lokalen Anwendung dp im Rumpf von st1. Das Durchlaufen einer Rolle funktioniert genauso: `%st1::pair::mid` ist das mid der Anwendung, die die Rolle pair füllt.
  - Dies sind gewöhnliche Verweise: Fakten können ihnen hinzugefügt werden (PLACE, SET-STYLE, TEXT, PRIORITY usw.), sie können in anderen Anweisungen verwendet werden, und sie können Rollen anderer Definitionen füllen; Konflikte mit den Fakten der Expansion werden nach 8.5 aufgelöst. Ein lokaler Name kann von außen nicht erneut gebunden werden: `%c1::mid = …` ist ein Fehler.
  - Namen: die über `::` an einer Anwendung erreichbaren Namen müssen eindeutig sein. Daher darf ein lokaler Name nicht gleich einem Rollennamen oder Standardunterobjektnamen sein, was an der Definition geprüft wird, und der Schlüssel eines Texts an der Anwendung darf nicht gleich einem dieser Namen sein (6.5, 13.3). Ein lokaler Name darf gleich einem Parameternamen sein: Parameter sind Werte, und `::` greift nicht auf Werte zu.
  - Priorität: eine von außen direkt auf ein lokales Objekt geschriebene PRIORITY-Anweisung überschreibt die Priorität der Anwendung (14.8).
  - Unerreichbar: die lokalen Objekte unbenannter Anwendungen, außer per Nummer aus dem Eintrag des Solvers (5.9), sowie die von Anwendungen, die schrittweise durch eine Auswahl generiert wurden. Eine Auswahl mit einer Anwendung als Bereich wählt weiterhin nur die Mitglieder ihrer Rollen aus, nicht ihre lokalen Objekte (10.1): über eine Anwendung werden lokale Objekte nur per Namen angesprochen. Eine Auswahl mit der Ebene als Bereich wählt sie als Objekte jener Ebene aus, wie jedes andere Objekt, etwa `# @SELECT-USAGE $FILTER` unten; was eine Regel nicht kann, ist ein lokales Objekt einer Definition benennen, die sie nicht selbst geschrieben hat.
  - Lokale Namen sind daher Teil der Schnittstelle einer Definition: ändert eine Definition einen lokalen Namen, finden Verweise von außen ihn nicht mehr und schlagen fehl (Kapitel 7), das Solver-Ergebnis veraltet mit ihnen, und ein Hinweis empfiehlt ein erneutes Lösen (Kapitel 17). Das Standardpaket gibt an, auf welche lokalen Namen seiner Definitionen Verlass ist.
- **Notation im Rumpf**: Rollen und Parameter werden `?name` geschrieben, lokale Namen `%name` (6.9, 6.10).
- **Ableitung und Erweiterung**: nur durch Komposition; es gibt keine Vererbungsbeziehung (9.1). Der Rumpf einer abgeleiteten Definition wendet die Basisdefinition an und fügt seine eigenen Anweisungen hinzu:

  ```rnl
  $FILTER = @DEFINE(roles=["in", "out"]) {};
  $LOWPASS = @DEFINE(roles=["in", "out", "r", "c"]) {
    %f = ?in(in) ?out(out) @AS $FILTER;
    {?r ?c} @ORDER("left-to-right")
  }
  ```

  - Inhalt wird über Anweisungen im Rumpf hinzugefügt, numerische Varianten über Parameter; um eine Definition in einer unteren Ebene zu verdecken und zu erweitern, verweist der Rumpf auf die gleichnamige äußere Definition (6.9).
  - Die Auswahl nach der Basisdefinition erfordert keine Sonderregel: die aus der abgeleiteten Definition expandierte FILTER-Anwendung befindet sich in der Szene, sodass `# @SELECT-USAGE $FILTER` sie auswählt, und für `$FILTER` geschriebene Regeln wirken auch auf Tiefpassfilter, wobei sie nur auf die FILTER-Anwendung darin wirken und nicht auf die von der abgeleiteten Definition hinzugefügten Anweisungen.
  - Es gibt keinen gesonderten Mechanismus zum Weglassen eines Teils der Basisdefinition: der Rumpf einer abgeleiteten Definition kann `%f::x @PRIORITY("disabled")` schreiben, um einen Teil davon abzuschalten, aber eine Priorität auf der äußeren Anwendung überschreibt dies (14.8); um einen Teil zuverlässig wegzulassen, teilt man die Basisdefinition in kleinere auf oder schreibt eine neue.

### 9.5 Verwendungszwecke allgemeiner Definitionen

Was eine Definition tut, entscheidet ihr Inhalt, nicht ein Schlüsselwort:

```rnl
$PAIR = @DEFINE(roles=["in_p", "in_n"]) {};                /* meaning: roles only */
$SIDE_BY_SIDE = @DEFINE(roles=["a", "b"]) {               /* layout: constraints in the body */
  {?a ?b} @ALIGN("y");
  {?a ?b} @ORDER("left-to-right")
};
$PROBE = @DEFINE(roles=[["p", "net"]]) {};                 /* virtual part: a rule gives its usages a symbol (chapter 12) */
$PROBE_SYM = @DEFINE-SYMBOL {(0, 0) @PIN(terminal="p"); (0, 0) @CIRCLE(radius=0.5)};
# @SELECT-USAGE $PROBE => @SET-SYMBOL $PROBE_SYM
```

Im Standardpaket sind `$CURRENT_MIRROR_N`, `$MIRRORED_LAYOUT` und `$GROUND` von diesen drei Arten; `$DIFFERENTIAL_PAIR_N` hat seine Rollen und in seinem Rumpf ein Standardlayout (unten).

- Sie sind alle dieselbe Art von Ding: eine Anwendung (die festhält, welche Definition angewendet wurde und was jede Rolle füllt), zuzüglich der aus dem Rumpf expandierten Beschreibungen. Die beiden Teile haben unterschiedliche Nutzer:
  - die aus dem Rumpf expandierten Bedingungen werden vom Solver verarbeitet, müssen erfüllt werden und werden bei einem Konflikt nach 8.5 aufgelöst;
  - die Anwendung dient Menschen und LLMs zum Lesen sowie den Auswahlen und Regeln von RNL selbst (10.1).

  So tragen auch die Anwendungen von Layout-Definitionen Bedeutung; eine Definition mit leerem Rumpf wie `$PAIR` hält Bedeutung allein über ihre Rollen fest, und ihre Bedeutung wird vom Standardpaket oder ihrem Autor festgelegt. Eine Definition, die Bedeutung festhält, darf in ihrem Rumpf auch eine Darstellung mitbringen (unten).
- Eine semantische Anwendung ist kein Kommentar: ihre Verweise müssen gefunden werden, ihre Rollen müssen passen, ihre grundlegende Gültigkeit wird geprüft (Prinzipien 2.4), und Auswahlen sowie Regeln können sie nutzen; nur wie sie gezeichnet wird, ist nicht vorgeschrieben.
- „Was es ist“ und „wie es gezeichnet wird“ dürfen in einer Definition oder in zweien stehen. Eine Definition darf in ihrem Rumpf eine Standarddarstellung mit Priorität `"low"` mitbringen: die eigenen Fakten des Verfassers stehen davor, und die Priorität der Anwendung schaltet sie ab oder hebt sie an, wie `%dp1 @PRIORITY("disabled")`, oder `# @SELECT-USAGE $DIFFERENTIAL_PAIR_N => @PRIORITY("disabled")` für jede Anwendung der Definition, während die Anwendung weiterhin festhält, was die Objekte sind (14.8). Der Schritt von der Bedeutung zur Darstellung kann auch als Regel in einem Paket geschrieben werden, und ein Paketwechsel ändert die Zeichnung: `# @SELECT-USAGE(recursive=true) $DIFFERENTIAL_PAIR_N => @AS $DP_LAYOUT`; das Löschen der Layout-Anwendung berührt die semantische Anwendung nicht. Das Standardpaket schreibt seine Standarddarstellungen in die Rümpfe seiner Definitionen, jede Bedingung davon mit Priorität `"low"`, etwa den Fuß eines Differenzpaars unter der Mitte des Paars, die Mitglieder von `$BRANCH` in einer Spalte von oben nach unten (16.3).
- **Darstellung wird immer in RNL geschrieben**: eine Bedeutung wird ausschließlich durch das auf bestimmte Weise gezeichnet, was RNL schreibt — im Rumpf ihrer Definition oder in einer durch eine Regel angewendeten Layout-Definition wie oben, gegebenenfalls in einem Paket; man verlasse sich für eine Bedeutung nicht auf Standardkonventionen eines Renderers. Ob eine Definition eine Darstellung mitbringt, liegt bei ihrem Paket; dieser Standard macht dazu keine weitere Vorschrift. Wie die von RNL nicht beschriebenen Teile platziert werden, liegt bei der Implementierung (Prinzipien 1), ist aber keine Zeichenkonvention für eine Bedeutung und unzuverlässig. Dass ein Renderer den Standardstil von Objekten ohne Beschreibungen festlegt (Prinzipien 2.7) und dass eine Instanz ohne Symbol das Standard-Blocksymbol hat (9.2.6), ist etwas anderes.
- **Vokabular gehört zum Standardpaket**: die ersten semantischen Definitionen und die standardmäßigen virtuellen Bauteile sind Inhalt des Standardpakets, das nicht Teil des Sprachstandards ist (16.3). Wie die Definitionen des Standardpakets ihre Bedeutung und Rollen festlegen, ist ebenfalls Sache des Standardpakets. Die Bedeutung der Definition eines Benutzers ist ihr Rumpf; ihr Name ist nur für Menschen zum Lesen.
- **Grundlegende Gültigkeit** (Prinzipien 2.4): die Syntax von Definitionen prüft nicht, ob zwei Rollen mit demselben Objekt gefüllt sind. Die grundlegende Gültigkeit einer Anwendung ist: die Verweise werden gefunden, die Rollen erhalten die richtigen Arten, die Einzelrollen sind gefüllt (9.4), und jede Anweisung der Expansion ist für sich gültig.

  ```rnl error
  $DP_LAYOUT = @DEFINE(roles=[["in_p", "instance"], ["in_n", "instance"]]) {
    {?in_p ?in_n} @ALIGN("y");
    ?in_p ?in_n @DISTANCE("left-to-right", min=4)
  };
  #M1(in_p) #M1(in_n) @AS $DP_LAYOUT      /* the expanded DISTANCE asks M1 to be at least 4 from itself */
  ```

  - In diesem Beispiel widerspricht sich das expandierte DISTANCE selbst: es schlägt mit einer Diagnosemeldung fehl, und nur es wird verworfen (5.5, 7.3); das ALIGN gilt wie üblich. Eine Definition mit leerem Rumpf wie `$PAIR` oben kann eine solche Füllung nicht erkennen: `#M1(in_p) #M1(in_n) @AS $PAIR` meldet keinen Fehler.
  - Ein virtuelles Bauteil mit beiden Enden auf demselben Netz hat eine eindeutige Bedeutung, daher kann „dasselbe Objekt füllt zwei Rollen“ nicht allgemein ein Fehler sein.
  - Weitere Prüfungen, etwa die Forderung, dass zwei Rollen verschieden sind, werden als Zusicherungen geschrieben (15.3).
- **Schlüsselwörter und allgemeine Definitionen** (1.5 Punkt 7): was eine Implementierung selbst verstehen muss und was nicht mit anderen Beschreibungen geschrieben werden kann, ist ein Schlüsselwort, wie die atomaren Bedingungen, GROUP, WIRE, TEXT und CURRENT; was in den Rumpf einer allgemeinen Definition geschrieben werden kann, ist eine allgemeine Definition, wie Differenzpaare, Stromspiegel, Massen und Versorgungen sowie Signalfluss (14.6). DEFINE-SYMBOL, DEFINE-STYLE, SET-SYMBOL und SET-STYLE sind ebenfalls Schlüsselwörter: ein Symbolrumpf steht in lokalen Koordinaten und wird dort ausgewertet, wo er definiert ist; die Eigenschaften eines Stils sind Literale; SET-SYMBOL und SET-STYLE setzen eine Eigenschaft bestehender Objekte und werden in Warteschlangen entschieden, sobald alle Objekte existieren, während AS und ATTACH jedes Mal ein neues Objekt erzeugen.
- Eine Definition wird über die Definition identifiziert, zu der ihr Name auflöst, nicht über die Namenszeichenkette: definiert eine untere Ebene ihr eigenes `$DIFFERENTIAL_PAIR_N`, ist dies eine andere Definition, und die für das Standard-`$DIFFERENTIAL_PAIR_N` geschriebenen Regeln eines Pakets erkennen sie nicht.

<!-- translated from en/ch10.md sha256:bff47dee5256 -->

## 10 Auswahlen, Regeln und schrittweise Generierung

Dieses Kapitel behandelt die Auswahlschlüsselwörter, die Regeln, die sie verwenden, sowie die schrittweise Generierung.

### 10.1 Auswahlschlüsselwörter

Es gibt für jede ausgewählte Objektart ein Auswahlschlüsselwort, jeweils mit einer festen Menge an Bedingungen. Das Ergebnis einer Auswahl ist eine Menge (5.2).

Jedes Auswahlschlüsselwort nimmt ein Bereichsobjekt an. `#` ist die Ebene, in der die Auswahl ausgewertet wird (10.3). Erhält sie ein Objekt oder eine Menge, sucht die Auswahl unter diesen Objekten: erhält sie etwa eine Menge von Netzen, wählt SELECT-TERMINAL die Anschlüsse auf jenen Netzen aus (wie der Bereich zur ausgewählten Art führt, wird nach der Tabelle beschrieben). Der Bereich kann nicht weggelassen werden, daher wird auch eine einfache Auswahl mit `#` geschrieben: `# @SELECT-INSTANCE(name="R*") => @SET-SYMBOL $RESISTOR`.

| Schlüsselwort | Wählt aus | Bedingungen |
|---|---|---|
| SELECT-INSTANCE | Host-Instanzen. Mit Anschlüssen im Bereich die Instanzen, zu denen sie gehören; mit Netzen die Instanzen, die einen Anschluss darauf haben | `name` (der Instanzname, wie `name="R*"`); `model` (der Name an der Modellposition der Instanzzeile: ein `.model`-Name oder ein Teilschaltungsname); `type` (der Typ der Instanz, aus ihrer `.model` oder einem DEVICE, wie `"nmos"`; nimmt `*`); `terminals` (die Anzahl der Anschlüsse der Instanz, wie `terminals=3`); `symbol` und `drawn` (unten) |
| SELECT-TERMINAL | Anschlüsse von Host-Instanzen, mit oder ohne Pins: eine Auswahl der Schaltung | `name`, `model` und `type` der Instanz; `terminal` (ein Anschlussname oder eine Nummer); `net` (der Name des Netzes, auf dem er liegt); `current` (die Stromrichtung, 15.2: `"in"`, `"out"` oder `"unspecified"`) |
| SELECT-PIN | Pins gezeichneter Objekte: eine Auswahl der Zeichnung (9.2.4) | `name`, `model` und `type` des Objekts, dem der Pin gehört; `terminal` (ein Anschluss- oder Rollenname, den der Pin bindet); `net` (der Name seines `::net`); `wired` (10.2) |
| SELECT-NET | Netze. Mit Instanzen, Anschlüssen oder Leitungen im Bereich die Netze, auf denen sie liegen | `name`; `port` (ob es ein Port dieser Ebene ist); `wired` (10.2); `global` (ob es ein globales Netz ist, 6.3) |
| SELECT-WIRE | Leitungen. Mit Netzen im Bereich die auf ihnen gezeichneten Leitungen | `net` (der Name des Netzes der Leitung; ein Bus passt, wenn eines seiner Netze passt); `bus` (`true`: nur Busse, 13.1; `false`: nur Leitungen, die keine Busse sind, deren Netz-Slot ein Netz statt einer Menge erhält) |
| SELECT-LABEL | Beschriftungen, also an Objekte angehängte TEXTe (13.3). Mit Objekten im Bereich die an sie angehängten Beschriftungen | `key` (der Schlüssel des Texts; nimmt `*` und unterscheidet Groß-/Kleinschreibung; eine Beschriftung ohne Schlüssel passt auf kein `key=`) |
| SELECT-ANNOTATION | Anmerkungen, also Grafiken in der Szene und an Punkte angehängte TEXTe (13.4) | Keine Bedingung außer `role` und `recursive` |
| SELECT-USAGE | `scope @SELECT-USAGE $D`: die Anwendungen der Definition `$D` im Bereich, nach innen blickend. Mit `#` die Anwendungen der Ebene; mit Anwendungen diese Anwendungen selbst und die Anwendungen darin. Ohne `$D` die Anwendungen einer beliebigen Definition | Die Definition ist das letzte Objekt (5.6) und darf weggelassen werden. `symbol` und `drawn` (unten) |
| SELECT-ANCESTOR-USAGE | `objects @SELECT-ANCESTOR-USAGE $D`: die Anwendungen von `$D`, die diese Objekte als Mitglieder haben, nach außen blickend; mittelbare Mitglieder zählen mit | Die Definition ist das letzte Objekt (5.6) und ist erforderlich. Keine Bedingung außer `recursive` |

Darüber hinaus nimmt jedes Auswahlschlüsselwort die Bedingung `recursive`, und die ersten sieben nehmen `role`, was nur dann eine Bedeutung hat, wenn der Bereich Anwendungen enthält (beide unten).

- **Ein Schlüsselwort für jede Art.** Jedes Auswahlschlüsselwort ist nach der Art benannt, die es auswählt, wie `&clk @SELECT-WIRE => @SET-STYLE $signal` für die Leitungen eines Netzes oder `#M1 @SELECT-LABEL` für die Beschriftungen von M1. Keine Auswahl nimmt Objekte aller Arten: um jede Art abzudecken, schreibt man für jede Art eine Regel, wie es das Standardpaket tut, um jedem Objekt seinen Basisstil zu geben.
- **Wie der Bereich zur ausgewählten Art führt.** Jedes Objekt im Bereich macht einen Schritt hin zur ausgewählten Art; ein Objekt ohne einen solchen Schritt wählt nichts aus, was kein Fehler ist:
  - SELECT-INSTANCE: eine Instanz ist sie selbst; ein Anschluss führt zu seiner Instanz; ein Netz führt zu den Instanzen, die einen Anschluss darauf haben.
  - SELECT-TERMINAL: eine Instanz führt zu ihren Anschlüssen; ein Netz führt zu den Anschlüssen darauf; ein Anschluss ist er selbst; ein Pin führt zu den Anschlüssen, die er bindet.
  - SELECT-PIN: ein gezeichnetes Objekt (eine Instanz, ein Bündel, eine Anwendung mit Symbol) führt zu den Pins seines eigenen Symbols, und mit `role` führt eine Anwendung stattdessen zu den Pins, die diese Rolle füllen, wie die anderen Auswahlen Mitglieder nehmen (unten); eine Anwendung ohne Symbol führt zu den Pins unter den Objekten, die ihre Rollen füllen; ein Anschluss führt zu den Pins, die ihn binden; ein Netz führt zu den Pins, deren `::net` jenes Netz ist, einschließlich der an durch es gefüllte Rollen gebundenen Pins; ein Pin ist er selbst. Mit `#` jeder Pin der Ebene.
  - SELECT-NET: ein Netz ist es selbst; eine Instanz führt zu den Netzen ihrer Anschlüsse; ein Anschluss führt zu seinem Netz; eine Leitung führt zu den Netzen, auf denen sie gezeichnet ist.
  - SELECT-WIRE: ein Netz führt zu den darauf gezeichneten Leitungen; eine Leitung ist sie selbst. SELECT-LABEL: ein Objekt führt zu den an es angehängten Beschriftungen; eine Beschriftung ist sie selbst. SELECT-ANNOTATION nimmt nur die Anmerkungen im Bereich.
  - Ein Schritt über Anschlüsse zählt jeden Anschluss, ob mit Pin oder ohne: Anschlüsse gehören zur Schaltung. Was gezeichnet ist, wird über Pins ausgewählt. So umfasst `&vss @SELECT-INSTANCE` einen NMOS, dessen einziger Anschluss auf vss ein Bulk ist, das sein Symbol nicht zeichnet. Ein Port dieser Ebene wie `#:inp` führt zu seinem Netz.
  - Mit Anwendungen als Bereich werden nur die Mitglieder der ausgewählten Art genommen; Mitglieder werden nicht erneut als Bereich verwendet (siehe unten).
  - Beispielsweise sind die vom Takt gesteuerten Transistoren `&clk @SELECT-INSTANCE`; nur jene, deren Gate auf clk liegt, sind `&clk @SELECT-TERMINAL(terminal="g") => @SELECT-INSTANCE`; die Netze von M1 sind `#M1 @SELECT-NET`.
  - Hinweis: Eine Regel, die Massen, Leitungen, Beschriftungen oder Stile zur Zeichnung hinzufügt, wählt Pins aus (SELECT-PIN), damit das Hinzugefügte zum Gezeichneten passt; eine Regel, die die Schaltung prüft, wählt Anschlüsse aus.
- **Nach Symbol: `symbol` und `drawn`.** SELECT-INSTANCE und SELECT-USAGE nehmen `symbol="$X"`, eine Zeichenkette: das Objekt wird mit einem über diesen `$`-Namen gegebenen Symbol gezeichnet, dem Namen, der im SET-SYMBOL stand, das es zugewiesen hat, verglichen wie geschrieben und mit dem Platzhalter `*`, etwa `symbol="$IO_PORT*"`; ein über einen `%`-Namen oder anonym zugewiesenes Symbol passt auf kein `symbol=`. Ein Alias eines `$`-Namens in jenem SET-SYMBOL wird als der ursprüngliche Name gelesen (16.5), sodass der ursprüngliche verglichen wird; die Zeichenkette von `symbol=` wird nicht übersetzt (16.5) und wird mit dem ursprünglichen Namen geschrieben. Anders als die Definition von SELECT-USAGE, die über das erkannt wird, wozu ihr Name auflöst (9.5), ist dies ein Namensvergleich; Parameter halten nur Literale (3.7), daher wird der Name wie bei anderen Bedingungen als Zeichenkette geschrieben. Sie nehmen außerdem `drawn` (`true`: das Objekt ist gezeichnet, mit einem durch eine Anweisung, eine Regel, ein Blocksymbol oder das Standard-Blocksymbol (9.2.6) zugewiesenen Symbol; `false`: ist es nicht, wie eine Anwendung ohne Symbol). Jede Instanz wird gezeichnet. Diese Bedingungen lesen die Symbole, funktionieren also in Generierungsregeln und in der Stil-Warteschlange, die nach der Symbolzuweisung kommen (8.2); in einer Auswahl von SET-SYMBOL geschrieben, sind sie ein Fehler, da die Symbol-Warteschlange nicht nach dem Symbol auswählen kann, das sie gerade zuweist. SELECT-PIN liest ebenfalls Symbole, mit derselben Einschränkung, ebenso jede Auswahl, deren Bereich ein Pin ist oder über Pins geht, etwa `#R1::p @SELECT-TERMINAL`: in einer Auswahl von SET-SYMBOL ist dies ein Fehler. Dasselbe gilt für einen Bereich mit `::` und einem Namen, der kein Rollenname, lokaler Name oder Standardunterobjektname des Objekts ist, etwa `#X1::k`: ob er einen Pin oder einen Schlüssel eines Texts benennt, den das Symbol mitbringt, ist erst bekannt, sobald die Symbole zugewiesen sind (6.5).
- **Diese Ebene, oder auch die Ebenen darunter.** Jedes Auswahlschlüsselwort hat die Bedingung `recursive`, standardmäßig `false`. Eine Regel wird in der Ebene ausgewertet, in der sie geschrieben steht, und wie weit sie nach unten reicht, wird wie folgt bestimmt; bei jeder Auswertung werden jede Auswahl und jede Menge darin in jener Ebene ausgewertet und enthalten nur die Mitglieder jener Ebene (10.3).
  - **Eine Regel mit einer darin geschriebenen Auswahl**, in ihrer Pipe oder in einem darin geschriebenen `{…}`, erreicht die Ebenen darunter, wenn eine dieser Auswahlen `recursive=true` angibt, und nur dann; die von ihr verwendeten Mengen ändern dies nicht. So fügt `# @SELECT-INSTANCE(recursive=true) => @SELECT-PIN(net="vss", wired=false) => $GROUND_STUB @ATTACH` Massen in dieser Ebene und in jeder Ebene darunter hinzu, während `# @SELECT-NET(name="vdd") => @REMOVE-FROM $PORT_FILL_SET` nur in seiner eigenen Ebene wirkt.
  - **Eine Regel ohne darin geschriebene Auswahl**, die von Auswahlen abhängende Mengen nimmt (10.4), erreicht jede Ebene unterhalb ihrer eigenen, die eine jener Mengen erreicht. Eine Menge erreicht ihre eigene Ebene und die Ebenen, die die ihr Ergebnisse von Auswahlen hinzufügenden Anweisungen erreichen: die Anweisung, an die sie gebunden ist, die Mengen unter ihren Elementen sowie die ADD-TO-Anweisungen, die ihr hinzufügen, wobei ein ADD-TO ohne darin geschriebene Auswahl so weit reichend zählt wie die Menge, die es hinzufügt, nicht die Menge, der es hinzufügt. In einer Ebene, die eine Menge nicht erreicht, hat sie keine Mitglieder, auch dann, wenn eine dorthin reichende Regel sie verwendet. So enthält mit `$GROUND_NET_SET = # @SELECT-NET(name="0", recursive=true)` die Menge das Netz 0 jeder Ebene, `%g_set = {$GROUND_NET_SET}` reicht ebenso weit, und `$IO_PORT @ATTACH $PORT_FILL_SET` oder `$SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET` wirkt in jeder Ebene, die diese Mengen erreichen.
  - Auf ein einzelnes Host-Objekt wird einfach verwiesen, wie `&vss`: Host-Verweise werden nur in ihrer eigenen Ebene aufgelöst, und eine Menge davon, wie `{&vss}`, erreicht keine Ebene darunter.
- **SELECT-USAGE und SELECT-ANCESTOR-USAGE.** Beide erkennen eine Definition über das, wozu ihr Name auflöst (9.5), wählen stets Anwendungen aus und haben kein `role=`.
  - SELECT-USAGE blickt nach innen, wie die anderen Auswahlschlüsselwörter. Mit `#` als Bereich wählt es die Anwendungen der ausgewerteten Ebene aus; mit einer Anwendung oder einer Menge von Anwendungen diese Anwendungen selbst und die Anwendungen darin bis zu den innersten. Instanzen, Netze und Anschlüsse im Bereich sind keine Anwendungen und wählen nichts aus; eine Warnung verweist auf SELECT-ANCESTOR-USAGE für die Anwendungen, in denen sie stehen.
  - SELECT-ANCESTOR-USAGE blickt nach außen: es wählt die Anwendungen aus, die ein Objekt des Bereichs direkt oder indirekt als Mitglied haben (siehe das Beispiel unten). Die Anwendungen im Bereich selbst sind nicht enthalten. Mit `#` als Bereich ist es ein Fehler: eine Ebene ist kein Mitglied von irgendetwas.
- **Anwendungen als Bereich.** Mit Anwendungen als Bereich wählen die anderen Auswahlschlüsselwörter die Mitglieder ihrer Art in diesen Anwendungen aus, und `role=` nimmt nur die Mitglieder jener Rolle. SELECT-PIN weicht nur bei einer Anwendung mit Symbol und ohne `role` ab, was die Pins ihres eigenen Symbols liefert (oben). Mitglieder, die Anwendungen sind, werden nach innen verfolgt bis zu den innersten:

  ```rnl
  $OTA = @DEFINE(roles=[["pair", "usage"], ["load", "usage"]]) {};
  %dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
  %cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR_P;
  %ota1 = %dp1(pair) %cm1(load) @AS $OTA;
  %all_set = %ota1 @SELECT-INSTANCE;                 /* {#M1 #M2 #M5 #M3 #M4} */
  %pair_set = %ota1 @SELECT-INSTANCE(role="pair");   /* {#M1 #M2 #M5} */
  %up_set = #M1 @SELECT-ANCESTOR-USAGE $OTA          /* {%ota1} */
  ```

  - Der Wert von `role=` akzeptiert auch die `definition=`-Aliase der Definition der Anwendung (16.5). Für die direkten Mitglieder allein verwendet man die Rolle als Unterobjekt, etwa `%dp1::tail`. Der Durchlauf hält bei einer bereits besuchten Anwendung an. Mitglieder sind die Objekte, die Rollen füllen, nicht die lokalen Objekte des Rumpfes (9.4).
  - **Nur Mitglieder der ausgewählten Art.** Mitglieder werden nicht als Bereich zum Weitergehen verwendet, und Mitglieder anderer Arten zählen nicht. SELECT-TERMINAL nimmt nur Anschlussmitglieder: nach `$GROUND @ATTACH &0; $CURRENT_ARROW @ATTACH #M5:1` ist `# @SELECT-USAGE => @SELECT-TERMINAL` nur {#M5:1}, nicht die Anschlüsse auf Netz 0. Die Mitglieder des Differenzpaars %dp1 sind Instanzen, daher ist `%dp1 @SELECT-TERMINAL` leer. Für die Anschlüsse der Mitgliedsinstanzen schreibt man `%dp1 @SELECT-INSTANCE => @SELECT-TERMINAL(terminal=2)` (die Gates aller drei Transistoren, einschließlich des Fußtransistors); für die Anschlüsse auf Mitgliedsnetzen `… => @SELECT-NET => @SELECT-TERMINAL`. Jedes Segment tut eine Sache; um weiterzugehen, fügt man ein Segment an. Direkt im Bereich angegebene Objekte folgen den obigen Regeln: ein Netz liefert die Anschlüsse darauf, eine Instanz ihre Anschlüsse.
  - **Eine durch Umwandlung gefüllte Rolle** (5.2.4), etwa eine Rolle der Art `"pin"`, die einen Anschluss erhält, hält das Objekt wie geschrieben, bis die Symbole zugewiesen sind, und ab dann das umgewandelte Objekt. Daher wählt `@SELECT-PIN(role="t")` in den Generierungsregeln und danach den Pin aus, ob die Rolle nun den Pin über seinen Namen oder einen dafür stehenden Anschluss erhalten hat.
- Um Objekte in der Szene anhand einer Bedingung zu finden, verwendet man stets ein Auswahlschlüsselwort; `::` greift nur auf Teile des Objekts selbst zu (6.5).
- **Bündel.** Die Mitglieder eines Bündels können nicht ausgewählt werden; stattdessen wird das Bündel ausgewählt (11.8). SELECT-INSTANCE wählt das Bündel selbst nach den Bedingungen seines ersten Mitglieds aus, einmal je Bündel. SELECT-TERMINAL wählt die Anschlüsse des Bündels aus, und `net=` passt, wenn es zum Netz irgendeines Mitglieds passt.
- **Bedingungen** werden mit Namen geschrieben, als `name=value`; ein Objekt wird nur ausgewählt, wenn alle geschriebenen Bedingungen gelten. Für „oder“ schreibt man mehrere Regeln. Die Bedingungen, die Namen vergleichen (`name`, `model`, `net`), nehmen den Platzhalter `*`, der auf beliebig viele Zeichen passt, und vergleichen nach den Groß-/Kleinschreibungsregeln des Hosts, wie es DEVICE tut (6.12). Um nach Elementbuchstaben auszuwählen, schreibt man `name="R*"`. Spectre-Instanznamen beginnen nicht mit einem Elementbuchstaben; man wählt sie über `model` aus, etwa `model="resistor"` (6.12).
- **`model`** vergleicht den an der Modellposition der Instanzzeile geschriebenen Namen: bei M-, Q-, D- und anderen Bauelementen den `.model`-Namen; bei X-Instanzen den Teilschaltungsnamen, einschließlich in der Netzliste nicht definierter PDK-Bauteile wie `# @SELECT-INSTANCE(model="sky130_fd_pr__nfet_01v8")`; in Spectre den Master. Es ist dasselbe wie `model=` von DEVICE (6.12). `type` vergleicht den Typ der Instanz: den Typ ihrer `.model` oder, bei einer Instanz, der der Host keinen gibt, etwa einer X-Instanz, den Typ, den ein DEVICE vergibt (6.12); ohne beides hat die Instanz keinen Typ und passt auf kein `type=`. `type` nimmt den Platzhalter `*` wie die Bedingungen, die Namen vergleichen: `type="*mos"` passt auf die Typen nmos und pmos, und `type="*"` wählt die Instanzen aus, die irgendeinen Typ haben, wie auch immer er lautet, womit eine Regel oder Zusicherung Bauteile mit unbekanntem Typ auslässt.
- **`terminals`** vergleicht die Anzahl der Anschlüsse der Instanz, eine ganze Zahl: die Knoten, die der Host in ihrer Zeile liest und die die Nummern 1, 2, … tragen (6.12), gleichgültig wie viele Namen DEVICE ihnen gibt. So wählt `terminals=3` `Q1 c b e qn` aus und nicht `Q2 c b e s qn`. Bei einer X-Instanz zählt es die Knoten ihrer Zeile, also die Ports der Teilschaltung; in Spectre die Knoten in den Klammern, sodass `S1 (com t1 t2) switch` 3 hat. Es kombiniert wie die übrigen Bedingungen mit ihnen, etwa `# @SELECT-INSTANCE(type="nmos", terminals=4) => @SET-SYMBOL $NMOS_4TERM`; ein Bündel wird nach seinem ersten Mitglied ausgewählt (11.8).
- Der Vergleich folgt 3.6: Bedingungsnamen und Standardaufzählungswerte unterscheiden Groß- und Kleinschreibung; Werte, die Host-Dinge benennen (Namen, Modellnamen, Netznamen), folgen den Regeln des Hosts.
- Ein Netz kann nach Namen ausgewählt werden: `# @SELECT-NET(name="0")` wählt es aus, und `# @SELECT-TERMINAL(net="0")` die Anschlüsse darauf. Objekte können auch nach Bedeutung ausgewählt werden: `# @SELECT-USAGE $DIFFERENTIAL_PAIR_N => @SELECT-INSTANCE(role="left")` wählt den linken Transistor jedes Differenzpaars aus.
- Es gibt kein Schlüsselwort für eine Schnittmenge, etwa die Mitglieder einer Menge, die zu einer Anwendung gehören; man schreibt sie mit REMOVE-FROM zweimal (5.2.2):

  ```rnl
  %s_set = {#M1 #M3 #M5};
  %s_set @SELECT-ANCESTOR-USAGE $DIFFERENTIAL_PAIR_N => @SELECT-INSTANCE => %dp_m_set;
  %out_set = {%s_set};
  %dp_m_set @REMOVE-FROM %out_set;    /* the members of %s_set in no differential pair */
  %in_set = {%s_set};
  %out_set @REMOVE-FROM %in_set       /* the members of %s_set in a differential pair */
  ```

- **Masse- und Versorgungsnetze** sind durch Mengen gegeben, die das Standardpaket bereitstellt, `$GROUND_NET_SET` (standardmäßig Netz 0) und `$SUPPLY_NET_SET` (standardmäßig leer). Es gibt keine Netzklassen.
  - Ein Paket darf keine Host-Verweise enthalten (16.2), daher kann der Standardinhalt einer solchen Menge nur als Auswahl geschrieben werden, etwa `# @SELECT-NET(name="0", recursive=true)`.
  - Der Benutzer fügt Netze mit ADD-TO hinzu und entfernt sie mit REMOVE-FROM:

    ```rnl
    &vss @ADD-TO $GROUND_NET_SET;                                       /* the vss of this level */
    # @SELECT-NET(name="vss", recursive=true) => @ADD-TO $GROUND_NET_SET;   /* the nets named vss of every level */
    # @SELECT-NET(name="vdd!", global=true, recursive=true) => @ADD-TO $SUPPLY_NET_SET  /* only the global vdd! */
    ```

    Die zweite Anweisung fügt auch lokale Netze namens vss in Teilschaltungen hinzu. Um ein Netz nur an einer Stelle auszulassen, kopiert man die Menge und entfernt das Netz aus der Kopie: `%g_set = {$GROUND_NET_SET}; &vss2 @REMOVE-FROM %g_set`.
  - Eine Ergänzungsregel verwendet eine solche Menge als ihren Bereich, wie `$GROUND_NET_SET @SELECT-PIN(wired=false, recursive=true) => $GROUND_STUB @ATTACH`, was auch die Pins erfasst, die virtuelle Bauteile auf diesen Netzen haben; das Standardpaket nimmt dann die Pins der Bauteile heraus, die es selbst platziert, etwa die der Massen.
  - Schreibt man in seiner eigenen Ebene `$GROUND_NET_SET = …`, verdeckt das die Menge des Pakets, und die Regeln des Pakets sehen die neue nicht (6.8).
  - Massen und Versorgungen werden nicht aus Netznamen erraten. Ein Importer schreibt die Spannungsdomäne eines Netzes aus einer Editordatei (etwa `powerDomain`) als ADD-TO.
  - Ein Solver darf die Standardmengen als Hinweise lesen (etwa Versorgungen oben und Massen unten), wie er die Definitionen des Standardpakets lesen darf; dies ist nicht vorgeschrieben.
  - Hinweis: Es gibt kein Schlüsselwort für Masse- und Versorgungsnetze (1.5 Punkt 7): die Verwendungen dieser Mengen, etwa das Auswählen von Anschlüssen zum Hinzufügen von Massen oder zum Symbolwechsel, lassen sich sämtlich als Regeln in einem Paket schreiben.
- **Der leere Knoten** (6.3) ist kein Netz: SELECT-NET liefert ihn niemals, ein Pin darauf wird von SELECT-PIN nur über sein Objekt oder `#` ausgewählt, nicht über ein Netz oder `net=`, und eine Leitung darauf von SELECT-WIRE ohne `net=`.
- **Globale Netze.** Die Bedingung `global=true` von SELECT-NET wählt nur globale Netze aus (6.3). Gleichnamige lokale Netze in verschiedenen Ebenen sind unterschiedliche Knoten, und die Auswahl nach Namen wählt alle davon aus; um nur das globale zu nehmen, fügt man diese Bedingung hinzu.

### 10.2 Die Bedingung `wired`

- Ein Pin ist `wired=false`, wenn er keiner der Punkte einer expliziten Leitung ist (ein Ende oder ein Punkt dazwischen; eine an einen Anschluss geschriebene Leitung endet am Pin des Anschlusses, 5.2.4) und in keinem Baum von ROUTE-NET oder ROUTE-OBJECT liegt (13.7), da der Solver die Leitungen eines Baums zeichnet. Ein Bus-Pin ist ein Pin, verdrahtet wenn eine Leitung über irgendeinen seiner Anschlüsse an ihm endet (9.2.4). Busleitungen sind ebenfalls explizite Leitungen (13.1). Eine Leitung zu einem Anschluss eines Bündels verbindet diesen Anschluss jedes Mitglieds (11.8).
- Ein Netz ist `wired=false`, wenn kein Anschluss einer Host-Instanz darauf von einer expliziten Leitung berührt wird und kein Baum von ROUTE-NET oder ROUTE-OBJECT darauf liegt; die Pins virtueller Bauteile zählen nicht mit.
- Welche Pins in Bäumen liegen, wird in zwei Stufen von 8.2 festgelegt. Nach Schritt 4, sobald die Symbole zugewiesen sind, aus den Anweisungen von Schritt 3: die Regeln von ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET und UNROUTE-OBJECT, die in Schritt 5 ausgewertet werden (10.4), lesen diese Bäume, etwa `{&a &b} @SELECT-NET(wired=false) => @ROUTE-NET`, und sehen einander nicht, sodass eine Regel niemals die Bäume liest, die sie selbst erzeugt. Nach Schritt 5 mit den Bäumen, die jene Regeln erzeugen und ändern: die Bedingungen von Schritt 6 und der späteren Schritte lesen diese. So zählt eine Generierungsregel, wie die Ergänzung des Standardpakets (16.3), die Pins, die eine ROUTE-Regel verbindet, als verdrahtet, und die Pins, die eine UNROUTE-Regel herausnimmt, als nicht verdrahtet, sofern keine geschriebene Leitung sie berührt. Die Route-Anweisungen in einem in Schritt 6 generierten Inhalt, etwa dem Rumpf einer generierten Anwendung, erzeugen und ändern ebenfalls Bäume, aber keine Bedingung sieht sie.
- Eine auf dem leeren Knoten geschriebene Leitung (13.1) zählt wie jede andere: die Pins, an denen sie endet, sind verdrahtet.
- Nur explizit geschriebene Leitungen zählen, nicht generierte. Ob virtuelle Bauteile vorhanden sind, macht keinen Unterschied.
- Leitungen zählen so, wie sie geschrieben stehen, ab Schritt 3 von 8.2, auch eine Leitung, die später fehlschlägt, weil ein Anschluss keinen Pin hat (7.1, 9.2.4). Bei einer umgeschriebenen Leitung zählt die letzte Neuschreibung (13.1).

### 10.3 Auswerten von Auswahlen

- Eine Auswahl wird nicht dort expandiert, wo sie geschrieben steht, und jede Auswertung betrachtet ausschließlich eine Ebene. Eine Regel (10.4) wird einmal in der Ebene ausgewertet, in der sie geschrieben steht, und einmal in jeder Ebene darunter, die sie erreicht (10.1); bei jeder Auswertung ist `#` jene Ebene, und die Auswahl wählt nur Objekte jener Ebene aus. So richtet `# @SELECT-INSTANCE(model="nch", recursive=true) => @ALIGN("y")` auf der obersten Ebene geschrieben einmal in jeder Zeichnung aus und richtet nicht Instanzen verschiedener Zeichnungen aneinander aus.
- Eine Auswahl wird in der Phase ausgewertet, die sie verwendet: Darstellungsregeln in der Symbol-Warteschlange und der Stil-Warteschlange, die Regeln der Route-Schlüsselwörter in Schritt 5, Generierungsregeln in der Generierungsphase (8.2). Dasselbe gilt für Mengen (5.2).
- Die Art, die eine Auswahl auswählt, ist beim Einlesen bekannt. Sie einem Slot zu übergeben, der diese Art nicht annimmt, ist sofort ein Fehler, auch innerhalb einer Menge, etwa `# @SELECT-TERMINAL(net="vss") => @SET-SYMBOL $NMOS`.
- SELECT-USAGE und SELECT-ANCESTOR-USAGE wählen stets Anwendungen aus; geht eine Auswahl von Anwendungen zu deren Mitgliedern über, ist die ausgewählte Art durch ihr Schlüsselwort gegeben. Daher ist die Art jeder Auswahl beim Einlesen bekannt.
- Das Ergebnis einer Auswahl ist eine Menge und kann mit `%` oder `$` benannt werden. Eine in einer Ebene ausgewertete Menge nimmt nur die Mitglieder jener Ebene an: `&vss @ADD-TO $GROUND_NET_SET` auf der obersten Ebene geschrieben fügt nur das vss der obersten Ebene hinzu, das bei der Auswertung in einer Teilschaltung nicht in der Menge ist (6.6).
- **Die Reihenfolge des Ergebnisses.** Die Mitglieder des Ergebnisses einer Auswahl stehen in dieser Reihenfolge, die überall dort zählt, wo die Reihenfolge einer Menge zählt, etwa bei ORDER, DISTRIBUTE, TABLE und LINE sowie bei den Ergebnissen einer schrittweise angewendeten Regel (5.2.1):
  - Mit `#` als Bereich die Objekte der Ebene in der Reihenfolge der Szene:
    - Instanzen in der Reihenfolge ihrer Zeilen in der Netzliste, ein Bündel an der Zeile seines ersten Mitglieds; Anschlüsse nach Instanz oder Bündel, dann nach Nummer;
    - Netze: die Ports der Ebene in der Reihenfolge der Teilschaltungszeile, dann die übrigen Netze in der Reihenfolge ihres ersten Auftretens auf den Instanzzeilen;
    - Pins nach dem Objekt, zu dem sie gehören, zuerst Instanzen und Bündel in der Reihenfolge der Zeilen (ein Bündel an der Zeile seines ersten Mitglieds), dann Anwendungen in ihrer Reihenfolge; innerhalb eines Objekts in der Reihenfolge der PIN-Anweisungen seines Symbols;
    - Anwendungen, Leitungen, Texte und Grafiken: zuerst die in Schritt 3 von 8.2 erzeugten in der Reihenfolge ihrer Positionen (8.5), wobei das vom Rumpf einer Anwendung Erzeugte unmittelbar nach jener Anwendung in der Reihenfolge des Rumpfes folgt; dann die in späteren Schritten erzeugten, Schritt für Schritt. Innerhalb eines Schritts stehen sie in der Reihenfolge der Positionen der erzeugenden Anweisungen oder Regeln, die Ergebnisse einer Regel in der Reihenfolge ihrer Eingabe (5.2.1), und die von Symbolen mitgebrachten Texte nach dem Objekt, zu dem sie gehören, wobei die Objekte in der für Pins angegebenen Reihenfolge genommen werden, dann in der Reihenfolge des Symbols. Ein Text der Szene, der einen Text des Symbols nach Schlüssel überschreibt (13.3), ist ein Text an der Position der ersten Anweisung, die ihn schreibt.
  - Mit jedem anderen Bereich liefert jedes Mitglied des Bereichs der Reihe nach seine Ergebnisse, jeweils in der obigen Reihenfolge, und ein bereits geliefertes Objekt wird nicht noch einmal geliefert. Dies gilt für jedes Auswahlschlüsselwort, SELECT-ANCESTOR-USAGE eingeschlossen.
  - Mengen behalten die Reihenfolge, in der ihre Mitglieder geschrieben und hinzugefügt werden (5.2, 5.2.2): ADD-TO hängt an und behält Wiederholungen, und UNION entfernt sie.
- Ist der Bereich einer Auswahl mit Host-Objekten ihrer Ebene geschrieben, wie `&Q<0:3:2, 1> @SELECT-TERMINAL(name="R*", recursive=true) => @TEXT("${net}", key="netname")` auf der obersten Ebene, existieren diese Objekte bei der Auswertung der Regel in einer unteren Ebene nicht: die Auswahl ist dort leer und tut nichts, ohne Warnung, genau wie eine Menge nur die Mitglieder der Ebene aufnimmt. Die Warnung aus 10.5 wird nur für die anderen Rollen von Generierungsregeln gegeben.

### 10.4 Regeln

- Eine Anweisung, die das Ergebnis einer Auswahl direkt oder über eine davon abhängende Menge annimmt, ist eine Regel. Eine Menge hängt von einer Auswahl ab, wenn sie an eine gebunden ist, wenn sie eine oder eine solche Menge unter ihren Elementen hat, wenn sie durch UNION solcher Mengen entsteht oder wenn ADD-TO oder REMOVE-FROM sie mit dem Ergebnis einer Auswahl oder einer solchen Menge ändert (5.2.2): ihre Mitglieder sind dann erst in der Phase bekannt, die sie verwendet. Im Rumpf einer Anwendung hängt eine Mengenrolle von einer Auswahl ab, wenn das, was sie füllt, davon abhängt.
- Jedes Schlüsselwort außer BUNDLE (11.8), WIRE (13.1) und mit zwei Punkten geschriebenen RECTANGLE, CIRCLE und ELLIPSE (9.2.5) darf das Ergebnis einer Auswahl annehmen, einzeln oder als Ganzes wie in 5.2.1 beschrieben: `# @SELECT-INSTANCE(type="nmos") => @ORIENT("MY")`, `# @SELECT-INSTANCE(model="nch") => @ALIGN("y")`.
- SET-SYMBOL und SET-STYLE mit dem Ergebnis einer Auswahl als Objekt sind Darstellungsregeln und gehen in die Darstellungswarteschlangen (8.3). AS und ATTACH sind Generierungsregeln (10.5), ebenso an das Ergebnis einer Auswahl angehängtes TEXT (13.3), mit Größe geschriebene RECTANGLE, CIRCLE und ELLIPSE, deren Punkt jenes Ergebnis ist (9.2.5), und die Regeln von LINE, ARC und BEZIER, die gezeichnete Objekte erzeugen; ihr Slot nimmt eine Menge, sodass eine solche Regel eine Figur durch das gesamte Ergebnis in dessen Reihenfolge zeichnet (5.2.1, 10.3). Die Regeln von ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET und UNROUTE-OBJECT werden davor in Schritt 5 ausgewertet: ihre Bedingungen lesen die Bäume der Anweisungen aus Schritt 3, und die Generierungsregeln sehen die von ihnen erzeugten Bäume (10.2). Die Regeln von CURRENT werden zusammen mit den Generierungsregeln in Schritt 6 ausgewertet; zusammen sind sie die Regeln von Schritt 6: Stromrichtungen stehen aus den Anweisungen von Schritt 3 fest, und die Bedingungen von Schritt 6 sehen nicht, was diese Regeln ändern (15.2). BUNDLE nimmt das Ergebnis einer Auswahl nicht an: ein Bündel muss bestehen, bevor die Symbole zugewiesen werden (11.8). Regeln anderer Schlüsselwörter, wie der Bedingungen, PLACE und ORIENT, werden im Schritt ihres Schlüsselworts wirksam und sehen die generierten Objekte. Bei jedem anderen Schlüsselwort entspricht eine Regel einer für jedes ausgewählte Objekt oder für alle zusammen geschriebenen Anweisung und wird in der Phase ihres Schlüsselworts wirksam.
- Eine Regel gilt für die Ebene, in der sie geschrieben steht, und für die Ebenen darunter, die sie erreicht (10.1), über `recursive=true` in einer in ihr geschriebenen Auswahl oder, falls sie keine enthält, über die von ihr verwendeten Mengen, einmal in jeder Ebene ausgewertet und nur jene Ebene betrachtend (10.3). Eine Regel in einem Paket gilt als in der Ebene des USE-PACKAGE des Pakets geschrieben, und ein Wurzelpaket als in der obersten Ebene seiner Datei verwendet (16.2), sodass sie die Ebenen darunter nur gemäß 10.1 erreicht, weshalb das Standardpaket in seinen Auswahlen `recursive=true` schreibt. Eine Regel im Rumpf einer allgemeinen Definition gilt als in der Ebene der Anwendung geschrieben (9.4).
- **Host-Objekte einer Ebene in einer Regel, die die Ebenen darunter erreicht**: eine Regel, die die Ebenen darunter erreicht und deren andere Slots Host-Objekte ihrer Ebene benennen, wie `%clk_set %rest_set @DISTANCE("bottom-to-top", min=3)` mit `%clk_set = {#VP1 #VP2}` und `%rest_set` gebunden an eine Auswahl mit `recursive=true`, findet in einer unteren Ebene keine solchen Objekte: dort hat die Anweisung keine Wirkung, und es wird keine Warnung ausgegeben. Nur Generierungsregeln warnen, wie 10.5 angibt.

### 10.5 Schrittweise Generierung

„Für jeden Anschluss auf vss ohne Leitung platziere eine Masse und verbinde sie mit einer Leitung“:

```rnl
$GROUND_STUB = @DEFINE(roles=[["t", "pin"]], parameters=[["gap", 2]]) {
  %g = $GROUND @ATTACH ?t::net;
  ?t::net @WIRE {?t %g::pin};
  {?t %g::pin} @ALIGN("x");
  %g::pin ?t @DISTANCE("bottom-to-top", exact=?gap)
};
# @SELECT-INSTANCE => @SELECT-PIN(net="vss", wired=false) => $GROUND_STUB @ATTACH => %stubs_set
```

Das Standardpaket stellt ein solches `$GROUND_STUB` bereit, nur dass sein Abstand standardmäßig 1 ist und es lediglich den Mindestabstand vorgibt (`min=?gap`), damit ein Solver die Leitung länger zeichnen kann, etwa wenn er die Massen verschieden langer Bauteile auf eine Linie setzt, dass es die Masse nach unten richtet (`@ORIENT("R0")`) und dass seine Bedingungen die Priorität `"low"` haben; die Definition hier verdeckt es (6.8).

- Erhält eine Einzelrolle von AS oder ATTACH das Ergebnis einer Auswahl, wird die Definition einmal für jedes ausgewählte Objekt angewendet, und die übrigen Rollen sind bei jeder Anwendung dieselben (5.2.1, 9.4). Eine Mengenrolle nimmt das Ergebnis einer Auswahl als Ganzes an, und die Definition wird einmal angewendet; sind alle ihre Rollen Mengenrollen, nur in den Ebenen, in denen eine davon etwas enthält (9.4).
- Virtuelle Bauteile werden auf dieselbe Weise schrittweise erzeugt: `# @SELECT-NET(port=true) => $IO_PORT @ATTACH` gibt jedem Portnetz eine Port-Markierung; mit `$RLOAD`, einer als Widerstand gezeichneten Definition mit zwei Rollen, hängt `# @SELECT-NET(name="out*", recursive=true) => $RLOAD @ATTACH &vss` einen zwischen jedes ausgewählte Netz und vss; da `&vss` zu dieser Ebene gehört, wird die Regel nur hier wirksam, mit der unten beschriebenen Warnung.
- Die Ergänzung des Standardpakets fügt Massen und Versorgungen mit einer Leitung hinzu (`$GROUND_STUB`, `$SUPPLY_STUB`): sobald sie zurückgeschrieben sind, macht die Leitung den Pin nicht mehr `wired=false`, und die Ergänzung überspringt ihn. Eine allein durch Bedingungen ohne Leitung an einen Pin gesetzte Masse lässt den Pin `wired=false`, sodass die Ergänzung dort weiterhin eine hinzufügt; eine Leitung der Länge 0 zwischen den beiden Pins (13.1) macht den Pin verdrahtet.
- Die Zuordnung gilt innerhalb jeder Anwendung: die lokalen Namen einer Definition sind in jeder Anwendung neu, sodass das `%g` einer Anwendung die in jener Anwendung erzeugte Masse ist.
- Das Ergebnis ist eine generierte Menge. Sie kann benannt und als Ganzes an SET-SYMBOL oder SET-STYLE übergeben werden (etwa `%stubs_set @SET-STYLE $grey`), aber auf kein einzelnes Mitglied davon kann verwiesen werden (5.7).
- **Von Hand schreiben.** Um eines davon für sich zu beschreiben, erzeugt man es von Hand und verdrahtet es. Sein Pin ist dann nicht mehr `wired=false`, und die Ergänzung überspringt es. Es gibt keine gesonderte Möglichkeit, eine Generierung abzubrechen.
- **Momentaufnahme.** Alle Generierungsregeln sehen dieselbe Momentaufnahme, das Ergebnis der Schritte 3 bis 5 von 8.2 (nach Überschreibung nach Schlüssel, mit den Symbolen aller Objekte und den Bäumen aus Schritt 5), und sehen die Ergebnisse der jeweils anderen nicht. Daher erzeugt dieselbe Regel zweimal geschrieben zwei Exemplare.
- Generierte Fakten erhalten in der Konfliktauflösung die Position ihrer Generierungsregel (8.5).
- Generierte Objekte ohne Position werden vom Solver platziert; um vorzugeben, wie sie platziert werden, verwendet man eine Definition mit Bedingungen.
- **Zurückschreiben.** Auf ein einzelnes generiertes Objekt kann nicht verwiesen werden (5.7). Wenn der Solver sie zurückschreibt, schreibt er sie als explizite Anweisungen, etwa `$IO_PORT @ATTACH &out => @PLACE(40, 20) => %p_out` samt der Leitung dorthin. Das explizite Objekt tritt nur dann an die Stelle des generierten, wenn die Regel auslässt, was bereits geschrieben ist, wie es die Ergänzungen des Standardpakets durch Auswahl nur unverdrahteter Pins (10.1) und eine Ergänzung nach Netz durch Bilden der Differenz zweier Mengen tun (unten); eine anders geschriebene Regel erzeugt das Objekt neben dem expliziten erneut, ohne Position. Eine Regel, deren Objekte der Solver platziert, sollte daher auf eine dieser beiden Arten das bereits Geschriebene auslassen. Die Objekte im Rumpf einer auf diese Weise explizit geschriebenen Anwendung ohne lokalen Namen werden per Nummer angesprochen (5.9). Die Anwendung wird mit der Priorität geschrieben, die die Regel auf ihre Anwendung schreibt, wie geschrieben, oder mit `"disabled"`, wenn sie diese aus dem Kopf des Eintrags der Regel hat (14.8); andernfalls wird sie ohne Priorität geschrieben, sodass ihr Rumpf seine Prioritäten behält, und ein Eintragskopf mit einer anderen Priorität wird nicht geschrieben.
- Generierung rekursiert nicht: enthält der von einer Generierungsregel expandierte Inhalt Regeln von Schritt 5 oder 6 (10.4), schlagen diese fehl, da jeder dieser Schritte einmal abläuft. Seine übrigen Regeln werden in den späteren Schritten wie üblich wirksam: SET-SYMBOL in der Symbol-Warteschlange von Schritt 7; PLACE, ORIENT, LAYER, PRIORITY und Bedingungen in Schritt 8, sodass der Rumpf einer generierten Anwendung platzieren und ausrichten kann, was seine Rollen erhalten; SET-STYLE in der Stil-Warteschlange; Zusicherungen bei ihrer Prüfung (8.2, 15.3).
- Wird eine andere Rolle einer Generierungsregel, die die Ebenen darunter erreicht (10.1), mit einem Objekt der Ebene (einem Szenenobjekt oder Host-Objekt) gefüllt, wird die Regel nur in der Ebene wirksam, in der sie geschrieben steht, mit einer Warnung: diese Objekte gehören zu jener Ebene, und die Ebenen darunter haben sie nicht. Eine Rolle, die eine Definition erhält, unterliegt keiner solchen Einschränkung.
- Ein von einer Generierung erzeugtes SET-SYMBOL oder SET-STYLE gehört zur Gruppe der Regel, die es erzeugt hat, an der Stelle jener Regel (8.3).
- Ein an eine Auswahl oder an eine generierte Menge gebundener Name kann nicht erneut gebunden werden (6.6).
- **Ergänzung nach Netz.** Es gibt keine eingebaute Regel „keines hinzufügen, wo eines existiert“. Eine Ergänzung schreibt die Differenz zweier Mengen; ein Paket, das Port-Markierungen hinzufügt, schreibt:

  ```rnl
  $PORT_FILL_SET = # @SELECT-NET(port=true, recursive=true);                                /* the nets that get a port tag */
  # @SELECT-USAGE(recursive=true) $IO_PORT => @SELECT-NET => @REMOVE-FROM $PORT_FILL_SET;   /* less those that have one */
  $IO_PORT @ATTACH $PORT_FILL_SET
  ```

  - Die Ergänzung der Standardserie 0.4 wird auf diese Weise geschrieben, mit der Menge in `rnl-std-basic`, und sie setzt außerdem die Markierungen jeder Ebene in eine Spalte (12.4). Sie lässt die Versorgungs- und Massenetze in der Menge; eine Datei, die dort, wo ihre Stubs stehen, keine Markierung wünscht, schreibt `$SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET; $GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET`.
  - Eine von Hand geschriebene Port-Markierung tritt an die Stelle der generierten: nach `$IO_PORT @ATTACH &inp => @PLACE(0, 20) => %p` erhält inp keine generierte Port-Markierung. Der Benutzer kann auch Netze aus `$PORT_FILL_SET` herausnehmen, wie `# @SELECT-NET(name="vdd") => @REMOVE-FROM $PORT_FILL_SET`, oder die Anwendungen seiner eigenen Port-Definition zählen: `# @SELECT-USAGE $MY_PORT => @SELECT-NET => @REMOVE-FROM $PORT_FILL_SET`.
  - Generierte Port-Markierungen zählen nicht mit: die Generierungsregeln verwenden `$PORT_FILL_SET` in Schritt 6, und die Auswahlen in der Differenz werden auf dem Zustand vor Schritt 6 ausgewertet (5.2).
  - Hinweis: „Eines existiert bereits“ ist nicht eingebaut, weil es bei einer Definition mit mehreren Rollen unklar ist, die eigenen Port-Definitionen des Benutzers nicht mitgezählt würden und eine wiederholte Layout-Definition lediglich Bedingungen wiederholt, was Konfliktauflösung und Prioritäten behandeln (8.5, 14.8).
- Bei einer Ergänzung nach Anschluss ist es weiterhin die Leitung eines von Hand geschriebenen Objekts, die an die Stelle des generierten tritt: ein Pin mit einer Leitung ist nicht mehr `wired=false`. Es gibt weiterhin keine Möglichkeit, eine Generierung abzubrechen: ein Netz aus der Menge zu nehmen ändert nur die Menge.

<!-- translated from en/ch11.md sha256:693deb0f50aa -->

## 11 Platzierung und Darstellung

### 11.1 Position: PLACE

```rnl
#M1 @PLACE(10, 20);
#M1 @PLACE(x=10)
```

- PLACE gibt einem Objekt eine Position; mindestens eines von `x` und `y` muss geschrieben werden. PLACE setzt nur die geschriebenen Koordinaten. Jede Koordinate ist zunächst frei, und eine nie geschriebene Koordinate bleibt frei, um durch Bedingungen und den Solver entschieden zu werden.
- PLACE setzt nur Koordinaten, keine Drehung, Spiegelung, Skalierung oder Richtungen von Anschlüssen; die Ausrichtung wird mit ORIENT gesetzt (11.2).
- Ein PLACE sind zwei geometrische Fakten, eines für x und eines für y, und in einem Konflikt kann jedes für sich verworfen werden (8.5). Der Parameter `priority=` gibt seine Priorität an (14.8), etwa `#M1 @PLACE(12, 20, priority="high")`: x und y nach Position stehen zuerst, die Parameter nach Namen danach (4.2 Punkt 9).
- Objekte: Instanzen, Anwendungen (über ihren eigenen Punkt, 14.1), Bündel (11.8), Punkte, TEXTe (Anmerkungen und Beschriftungen; PLACE setzt die Position des Texts selbst, 13.3) und der Ursprung `::position` (einschließlich des Mittelpunkts eines in Mittelpunktform geschriebenen RECTANGLE, CIRCLE oder ELLIPSE). Andere Grafiken nimmt PLACE nicht an; um sie zu verschieben, verschiebt man ihre Punkte (13.4). Im Rumpf einer allgemeinen Definition sind die Koordinaten Versätze gegenüber dem eigenen Punkt der Anwendung (9.4). In einem Symbolrumpf positioniert PLACE ausschließlich die an `?` angehängten TEXTe in lokalen Koordinaten (9.2.2).
- Die Ausgabe behält das Handle der Eingabe (5.1).
- Eine Menge als Objekt wird Mitglied für Mitglied platziert (5.2.1): `%pair_set @PLACE(10, 20)` setzt beide Mitglieder auf (10, 20). Eine Menge hat keine eigene Position (14.1).
- Zu Achsen, Einheit, Dezimalzahlen und Toleranz von Koordinaten siehe 11.7.

### 11.2 Ausrichtung: ORIENT

```rnl
%pair_set = {#M3 #M4};
#M2 @ORIENT("MY");          /* flipped left-right */
%pair_set @ORIENT("R90");   /* each member turns about its own origin and stays where it is */
#M5 @ORIENT(["R0", "MY"])   /* not turned, or flipped left-right: the solver chooses */
#M6::g @ORIENT("left")      /* M6 is oriented so that its gate faces left */
```

- Objekte: Instanzen, Anwendungen mit einem Symbol und Bündel; sowie Pins (unten). ORIENT auf einer Anwendung ohne Symbol schlägt fehl (12.1); Punkte, Beschriftungen, Grafiken, Leitungen und Netze werden nicht angenommen.
- Eine Menge als Objekt wird Mitglied für Mitglied ausgerichtet (5.2.1): jedes Mitglied dreht sich um seinen eigenen Ursprung und bleibt dort, wo es ist, und jedes Mitglied ist eine Beschreibung. Die Menge wird nicht als ein Block gedreht. Eine ganze Menge gemeinsam zu drehen ist nicht vorgesehen: eine Menge hat keine Position (14.1).
- Eine Anwendung mit Symbol dreht sich um ihren eigenen Punkt; sie hat eine eigene Ausrichtung und dreht sich nicht mit den Objekten, die ihre Rollen füllen (12.1). Was der Rumpf ihrer Definition zeichnet oder platziert, dreht sich ebenfalls nicht mit ihr (9.4).
- **Eine Liste von Codes**, wie `["R0", "MY"]`, bildet einen Fakt, der gilt, wenn das Objekt eine der aufgeführten Ausrichtungen hat. Er wird als Ganzes behalten oder verworfen (8.5), und der Solver wählt unter den Codes. Die Liste enthält mindestens einen Code; eine Liste mit einem Code ist dasselbe wie dieser Code. Der Solver schreibt die gewählte Ausrichtung als einzelnen Code in seinen Eintrag zurück (14.8); die Liste im Eintrag des Verfassers bleibt erhalten, und der nächste Lösevorgang wählt erneut. Ein Renderer, der nicht löst und kein Ergebnis für das Objekt hat, zeichnet den ersten Code der Liste.
- Ohne ORIENT gibt es keine Bedingung, und der Solver wählt; ein Renderer, der nicht löst, zeichnet `"R0"` (8.5). Beziehungen zwischen den Ausrichtungen mehrerer Objekte (dieselbe Ausrichtung, Spiegelbilder) werden mit ORIENT-RELATION geschrieben (14.9).
- ORIENT ist ein geometrischer Fakt und nimmt an der Konfliktauflösung teil (8.5); der Parameter `priority=` gibt seine Priorität an (14.8).
- **Ausrichtungscodes.** Jeder Code steht für eine Ausrichtung. Nur die acht Schreibweisen der Tabelle und die vier Wörter unten werden akzeptiert, und sie unterscheiden Groß- und Kleinschreibung (3.6). Die Namen stammen aus Virtuoso und, bei nach oben gerichtetem y (11.7), ebenso ihre Bedeutungen. Gleichwertige Schreibweisen (wie `"MXR180"`, was MY ist) und beliebige Winkel werden nicht akzeptiert, weder einzeln noch in einer Liste: sie sind ungültige Parameterwerte, und das ORIENT schlägt fehl. Werkzeuge schreiben ausschließlich diese acht einzeln zurück. Es gibt keine relative Form, die ein Objekt ausgehend von seiner aktuellen Ausrichtung weiterdreht (Kapitel 17).

  | Code | (x, y) wird zu | Bedeutung |
  |---|---|---|
  | `"R0"` | (x, y) | Nicht gedreht |
  | `"R90"` | (−y, x) | Um 90° gegen den Uhrzeigersinn gedreht |
  | `"R180"` | (−x, −y) | Um 180° gedreht |
  | `"R270"` | (y, −x) | Um 270° gegen den Uhrzeigersinn gedreht, also 90° im Uhrzeigersinn |
  | `"MY"` | (−x, y) | An der y-Achse gespiegelt: links-rechts vertauscht |
  | `"MX"` | (x, −y) | An der x-Achse gespiegelt: oben-unten vertauscht |
  | `"MYR90"` | (−y, −x) | MY, dann R90 |
  | `"MXR90"` | (y, x) | MX, dann R90 |

- **Wörter für die vier Drehungen.** Eine Drehung darf auch als Richtung geschrieben werden, die denselben Code bedeutet: `"left"` ist `"R0"`, `"down"` ist `"R90"`, `"right"` ist `"R180"` und `"up"` ist `"R270"`, auch in Listen. ORIENT-RELATION nimmt nur die Codes (14.9). Für ein Objekt ist das Wort lediglich eine andere Schreibweise: `#M2 @ORIENT("left")` ist `#M2 @ORIENT("R0")`. Für einen Pin liest es sich wörtlich: die Richtung, in die der Pin zeigt, gemessen von links aus.
- **Pins.** ORIENT auf einem Pin oder einem Anschluss, der für seinen Pin steht (5.2.4), schränkt die Richtung des Pins in der Zeichnung ein: die mit `direction=` im Symbol geschriebene Richtung (9.2.4), gedreht um die Ausrichtung seines Objekts, eine der vier. Es gilt, wenn das Objekt eine der Ausrichtungen hat, die den Pin in diese Richtung drehen, zwei der acht, legt also die Ausrichtung des Objekts nicht eindeutig fest; eine Liste erlaubt jede aufgeführte Richtung. Ein gespiegelter Code auf einem Pin steht für die Richtung, in die er links überführt: `"MY"` ist `"right"`, `"MX"` ist `"left"`. Ein solches ORIENT ist ein eigener geometrischer Fakt mit dem Pin als Schlüssel, neben einem etwaigen ORIENT des Objekts; die beiden werden wie beliebige zwei Fakten aufgelöst (8.5). Ein Pin ohne geschriebenes `direction=` hat nichts einzuschränken: das ORIENT dieses Pins schlägt fehl, während die übrigen Mitglieder derselben Anweisung weiterhin wirksam werden (5.2.1). Ein Renderer, der nicht löst und kein Ergebnis für das Objekt hat, zeichnet es so, als wäre das ORIENT seiner Pins nicht geschrieben.
- **Reihenfolge der Verknüpfung.** MXR90 und MYR90 spiegeln zuerst und drehen dann; die Namen lesen sich in der Reihenfolge der Operationen, wie in Cadence und in OpenDB von OpenROAD.
- **Wirkungsweise.** Die Ausrichtung wirkt auf die lokalen Koordinaten des Symbols um dessen Ursprung, und der Ursprung wird anschließend an die Position des Objekts gesetzt: ein Punkt des Symbols wird an der Position des Objekts zuzüglich der auf seine lokalen Koordinaten angewendeten Ausrichtung gezeichnet. Die Grafiken und Pin-Positionen des Symbols werden starr mittransformiert, und die Richtungen, in denen Leitungen die Pins verlassen, drehen sich mit (9.2.4). Texte werden gemäß 9.2.5 dargestellt; Glyphen werden nicht gespiegelt.
- **Nur dort, wo Dinge gezeichnet werden.** Eine Ausrichtung ändert nur, wo Pins gezeichnet werden, nicht zu welchen Anschlüssen sie gehören. Wird ein Blocksymbol oben-unten gespiegelt, wird der Pin von inp unten gezeichnet, und die Leitung zu inp folgt ihm. Die beiden Pins werden nicht vertauscht, um inp oben zu halten: das würde ändern, welche Leitung an welchen Anschluss anschließt, eine Angelegenheit der Netzliste (Prinzipien 2.2). Um inp anderswo zu zeichnen, verwendet man ein anderes Symbol (11.3).

### 11.3 Symbole und Stile: SET-SYMBOL und SET-STYLE

```rnl
%w13 = &x @WIRE {#M1:1 #M3:1};
#M1 @SET-SYMBOL $NMOS;
# @SELECT-INSTANCE(model="nch") => @SET-SYMBOL $NMOS;
%w13 @SET-STYLE $signal;
# @SELECT-INSTANCE => @SET-STYLE $base
```

- SET-SYMBOL gibt einem Objekt ein Symbol. Das Objekt kann eine Instanz, eine Anwendung oder ein Bündel sein, oder eine Menge davon (einschließlich der Ergebnisse von Auswahlen und schrittweiser Generierung); jedes Mitglied erhält das Symbol einmal (5.2.1). In einer Teilschaltung gibt `# @SET-SYMBOL` das Blocksymbol an (9.2.6).
- Eine Anwendung, die ein Symbol erhält, wird gezeichnet: sie ist ein virtuelles Bauteil (Kapitel 12).
- **Zuordnung von Anschlüssen zu Pins.** Die Anschlussnamen eines Symbols sind jene, die seine Pins binden (9.2.4). Wenn SET-SYMBOL einer Instanz ein Symbol gibt, wird jeder Anschlussname des Symbols dem gleichnamigen Anschluss der Instanz zugeordnet, wie er durch DEVICE oder die Ports der Teilschaltung gegeben ist (6.12); eine Zahl wird dem Anschluss an dieser Stelle zugeordnet. Ein Anschluss der Instanz, den kein Pin bindet, hat keinen Pin, etwa ein Bulk in einem dreipoligen Symbol. Ein Anschlussname des Symbols, den die Instanz nicht hat, lässt seinen Pin ungebunden, mit einer Warnung.
  - `terminals=` ändert die Zuordnung für dieses SET-SYMBOL. Seine Standardform listet in der Reihenfolge der Knoten der Instanzzeile (wie `terminals` von DEVICE, 6.12) die Anschlussnamen des Symbols auf: bei einem Symbol, dessen Pins `"a"`, `"b"` und `"c"` binden, bindet `#X1 @SET-SYMBOL(terminals=["b", "a", "c"]) $GATE` Knoten 1 an den Pin von b, Knoten 2 an den von a und Knoten 3 an den von c. Es darf auch Paare auflisten, zuerst den Anschluss der Instanz und als Zweites den des Symbols: `terminals=[["d", "b"], ["g", "a"], ["s", "c"]]`; nicht in Paaren aufgeführte Anschlüsse werden nach Namen zugeordnet. Eine Liste, die keines von beiden ist oder einen Anschluss doppelt nennt, ist ein ungültiger Parameter, und das SET-SYMBOL schlägt fehl.
  - Bei einer Anwendung werden die Anschlussnamen des Symbols den Rollennamen ihrer Definition zugeordnet. Eine Zahl oder ein Name, den keine Rolle trägt, bindet keine Rolle, und es wird eine Warnung ausgegeben. `terminals=` listet Symbolanschlussnamen in der Reihenfolge der Rollenliste auf oder Paare aus einem Rollennamen und einem Symbolanschlussnamen (12.1).
- SET-STYLE gibt einem Objekt einen Stil. Das Objekt kann eine Instanz, eine Anwendung, ein Bündel, eine Leitung oder eine Grafik (Texte eingeschlossen) sein, oder eine Menge davon (einschließlich der Ergebnisse von Auswahlen und schrittweiser Generierung). Netze, Ebenen, Bedingungen, Definitionen und Punkte werden nicht angenommen. Anschlüsse werden mit dem Symbol ihres Objekts gezeichnet und übernehmen den Stil ihres Objekts. Ein einer Anwendung gegebener Stil geht auf ihr Symbol und auf die in ihrem Rumpf erzeugten zeichenbaren Objekte über (8.3), sodass ein Stil auf einer semantischen Anwendung ohne Symbol kein Fehler ist; wird im Rumpf nichts gezeichnet, hat er keine sichtbare Wirkung.
  - Hinweis: Andernfalls würde die Regel `# @SELECT-USAGE => @SET-STYLE $base` des Standardpakets für jede semantische Anwendung eine Warnung auslösen.
- Die Ausgabe behält das Handle der Eingabe; wenn das Objekt eine Menge ist, ist die Ausgabe diese Menge.
- Beide werden über die Darstellungswarteschlangen wirksam, nicht durch Überschreibung nach Schlüssel. Es gibt zwei Stufen: ein SET-SYMBOL oder SET-STYLE, das das Objekt direkt benennt, steht über jenen, die das Objekt über eine Auswahl erreichen oder an es weitergereicht werden (8.3).
- Wo Standardregeln existieren (das Standardpaket, 16.3), müssen sie nicht geschrieben werden.
- **Parameter und Stile.** Auf ein Objekt geschriebene Stilparameter, wie `size=` von TEXT, bilden den direkten Stil dieses Objekts und stehen über den Stilen aus Auswahlen oder Weitergaben. Ein SET-STYLE, das das Objekt direkt benennt und später geschrieben steht, überschreibt sie nach Position (8.3); so ändert ein Editor ein einzelnes Objekt. Ein nicht übergebener Parameter einer Definition zählt als nicht geschrieben, sodass der Stil wirksam wird (9.4).
- **Beschriftungen und Knotenpunkte.** Der Stil eines Objekts wird an die an es angehängten Texte weitergegeben. In der Stufe der ausgewählten oder weitergegebenen Stile wird er nach Position mit den Regeln verglichen, die den Text auswählen (etwa SELECT-LABEL); die Parameter des Texts selbst und ein SET-STYLE, das den Text direkt benennt, liegen in der direkten Stufe. Steht `#M1 @SET-STYLE $signal` nach den Beschriftungsregeln, übernimmt auch der Name von M1 diese Farbe, und das Ziehen des Namens ändert seine Farbe nicht. An `?` in einem Symbol angehängte Texte sind ebenfalls Beschriftungen, und SELECT-LABEL wählt sie aus; an Punkte angehängte feste Texte sind Grafiken des Symbols und übernehmen den Stil des Objekts. An Netze oder Punkte angehängte Texte erhalten keinen Stil von dem, woran sie hängen, und verwenden nur ihren eigenen. Ein Knotenpunkt ist eine Anwendung von `$JUNCTION` (13.2); der Punkt ist sein Symbol, und seine Farbe folgt dem Stil der Anwendung.

### 11.4 Mehrere Zeichnungen eines Objekts

Es gibt keine Möglichkeit, ein Host-Objekt mehrfach zu zeichnen: ein Host-Handle wie `#M1` meint stets die eine Zeichnung dieses Objekts. Sollte eine spätere Version mehrere Zeichnungen eines Objekts hinzufügen, wären dies eigene Objekte mit eigenen Namen, und Host-Handles behielten ihre Bedeutung (Anhang C).

Hinweis: Mehrere gemeinsam als ein Bauteil gezeichnete Instanzen sind ein Bündel (11.8).

### 11.5 Gruppen: GROUP

```rnl
%stage1_set = {#M1 #M2 #M5} @GROUP;
%stage2_set = {#M3 #M4} @GROUP(spacing_x=-1);
{%stage1_set %stage2_set} @GROUP;
%stage1_set %stage2_set @DISTANCE("left-to-right", min=1)
```

- GROUP fügt einer Menge einen Fakt hinzu: der Solver behandelt ihre Mitglieder als einen Block. Es behält das Handle der Menge, sodass `%stage1_set` weiterhin diese Menge ist.
- **Eine Gruppe ist kein Objekt.** Sie hat keine Position und keinen Rahmen, und auf eine Gruppe kann keine Bedingung geschrieben werden: wird sie in einer Bedingung angegeben, ist die gruppierte Menge eine Menge, und eine Stelle, die sie aufnimmt, steht für jedes ihrer Mitglieder (14.1). GROUP ist auch keine fünfte atomare Bedingung. Strukturen des Standardpakets, die Gruppen sind, haben anonyme Gruppen.
- **Mitglieder.** Die Mitglieder einer Gruppe sind die Objekte in ihrer Menge, wobei verschachtelte Mengen bis zu den innersten Objekten verfolgt werden. Eine Anwendung ohne Symbol steht für die Objekte, die ihre Rollen füllen, und so weiter bis hinab zu den untersten Objekten; eine mit Symbol gezeichnete Anwendung ist selbst ein Mitglied. ROUTE-OBJECT folgt den Rollen von Anwendungen auf dieselbe Weise und nimmt auch die eigenen Pins einer mit Symbol gezeichneten Anwendung mit auf (13.7).
- **Wirkung.** Eine Gruppe leitet den Solver auf genau zwei Arten: sie ordnet die Mitglieder einer Gruppe gemeinsam vor oder getrennt vom Rest an, und sie hält andere Objekte aus dem Bereich der Gruppe heraus, soweit die Fakten dies zulassen. Beides sind Kriterien des Solvers, die nach allen Beschreibungen der Sprache eingeordnet sind (8.5): eine Gruppe garantiert keine geometrische Relation, und ein Objekt, das durch Fakten in den Bereich einer Gruppe gezwungen wird, wird dort gezeichnet. Anforderungen an Positionen, etwa kein Überlappen bzw. Ineinandergreifen (das DISTANCE im Beispiel) oder Zentrierung, werden als atomare Bedingungen geschrieben.
- **Der Baum der Gruppen gehört zum Solver.** Gruppen schachteln sich durch Enthaltung: die Mitglieder von `{%stage1_set %stage2_set}` sind die fünf Instanzen, sodass beide Stufen darin liegen. Zwei Gruppen, die Mitglieder teilen, ohne dass eine die andere enthält, können nicht beide Blöcke sein; der Solver behält eine davon nach seiner eigenen Regel, und ein Hinweis nennt die andere. Wie Gruppen überdies Suche und Bewertung beeinflussen, liegt bei der Implementierung.
- **Abstand.** `spacing_x` und `spacing_y` sind ganze Zahlen von Rastereinheiten (11.7), die der Solver in x bzw. y zu seinem Standardabstand innerhalb der Gruppe addiert; jeder kann weggelassen werden, und ein negativer Wert zieht die Gruppe enger zusammen. Zwischen zwei Objekten verwendet der Solver den Abstand der innersten Gruppe, die beide enthält; zwischen Objekten ohne gemeinsame Gruppe den Abstand der Ebene, den `@SOLVER-CONFIG` mit gleichnamigen Schlüsseln setzt (14.10); zwischen einer Gruppe und dem, was außerhalb liegt, den Abstand der umschließenden Gruppe oder der Ebene. Ein Solver darf den Wertebereich begrenzen und gibt dies an. Der Abstand ist ein Kriterium des Solvers, kein Fakt: er weicht jedem behaltenen Fakt.
- GROUP ist ein Schlüsselwort, keine Definition des Standardpakets: seine Bedeutung kennt nur der Solver, und sie lässt sich nicht im Rumpf einer Definition schreiben (1.5 Punkt 7).
- Ein Viewer darf eine Gruppe zeichnen, etwa mit einem Rahmen; dies liegt beim Viewer, und dieser Standard legt es nicht fest.
- GROUP ist keine atomare Bedingung: es nimmt an der Konfliktauflösung nicht teil (8.5) und hat keine Priorität, sondern nur Ein und Aus. `%stage1_set @PRIORITY("disabled")` hält die Gruppe vom Solver fern, was die Gruppierung aufhebt. Ist eine äußere Anwendung deaktiviert, sind die Gruppen in ihrer Expansion ebenfalls aus, sofern nicht ein von außen über `::` direkt auf eine Gruppe geschriebenes PRIORITY sie wieder einschaltet (14.8). Eine Gruppe in der Expansion einer Definition kann nur dann für sich ausgeschaltet werden, wenn sie ein benanntes lokales Objekt ist, auf das von außen verwiesen wird (9.4).

### 11.6 Zeichenreihenfolge und Zeichenebenen: LAYER

```rnl
@RECTANGLE(fill="fillcolor") {(0, 0) (40, 30)} => @LAYER("bottom") => %f1;    /* a frame, drawn below everything */
(2, 28) @TEXT("input stage") => @LAYER("top") => %n1
```

- Es gibt drei Zeichenebenen: `"bottom"`, `"middle"` und `"top"`. Die untere Zeichenebene wird zuerst gezeichnet, dann die mittlere, dann die obere. Innerhalb einer Zeichenebene werden Objekte nach ihrer Position gezeichnet: ein späteres Objekt wird über ein früheres gezeichnet und verdeckt es.
- `@LAYER(…)` setzt die Zeichenebene eines Objekts; ohne sie liegt das Objekt auf `"middle"`. Es gibt einen Schlüssel für jedes Objekt, und das spätere LAYER überschreibt (5.8). LAYER ändert die Position des Objekts innerhalb der Zeichenebene nicht.
- **Position** ist die Reihenfolge im Text (1.5 Punkt 2), dieselbe Ordnung wie in 8.3 und 8.5: innerhalb einer Ebene die Reihenfolge des Texts; der Inhalt eines Pakets an der Stelle seines USE-PACKAGE, mit den Paketen an der Wurzel zuerst (16.3); mit `scope=` hineingeschriebene Anweisungen nach dem eigenen Text der Zielteilschaltung. Jede Objektart übernimmt die Position von:
  - Leitungen, Grafiken und Texten (Anmerkungen und Beschriftungen): der Anweisung, die es erzeugt;
  - Anwendungen: dem SET-SYMBOL, das wirksam wird, oder, bei einem durch eine Regel gegebenen Symbol, der Regel (8.3);
  - durch Regeln, durch die Expansion von Definitionen und durch schrittweise Generierung erzeugten Objekten: der Anweisung, die sie erzeugt, und innerhalb einer Expansion der Reihenfolge im Rumpf;
  - Instanzen: ganz unten in ihrer Zeichenebene, untereinander in der Reihenfolge der Netzliste; eine auf eine andere Zeichenebene gesetzte Instanz liegt ganz unten in jener Zeichenebene. Ein Bündel zählt als Instanz, mit der Netzlistenreihenfolge seines ersten Mitglieds.
- Anweisungen, die ein Objekt verändern, wie PLACE, ORIENT, SET-STYLE und LAYER, verschieben seine Position nicht.
- Objekte: Instanzen, Anwendungen mit Symbol, Bündel, Leitungen, Grafiken und Beschriftungen; eine Menge Mitglied für Mitglied (5.2.1). LAYER auf einer Anwendung ohne Symbol ist kein Fehler, hat aber keine Wirkung (wie bei SET-STYLE, 11.3); auf Punkten, Netzen, Bedingungen und Definitionen schlägt es fehl.
- Die Mitglieder eines Symbols haben keine Zeichenebenen: sie werden in Mitgliederreihenfolge (9.2.1) gemeinsam mit ihrem Objekt gezeichnet.
- Es gibt kein numerisches `z=`. Überschreiben (über den Schlüssel eines Fakts) und Verdecken (durch Zeichenebene und Position) sind getrennt.
- „Zeichenebene“ meint hier die Zeichenreihenfolge; es hat nichts mit den Ebenen des Hosts zu tun (2.6).

### 11.7 Koordinaten und Einheiten

- **Achsen.** x verläuft nach rechts und y nach oben; ein größeres y liegt höher. Daher verlaufen `"bottom-to-top"` von ORDER und DISTANCE nach oben (14.3, 14.5), und R90 dreht gegen den Uhrzeigersinn (11.2). In SVG, anderen Formaten und manchen Editoren verläuft y nach unten; die Implementierung kehrt y beim Zeichnen, Importieren und Exportieren um.
- **Einheit.** Eine Einheit hat keine physikalische Größe. Wie groß eine Einheit gezeichnet oder gedruckt wird, liegt beim Viewer.
- **Dezimalzahlen.** Koordinaten und Längen dürfen mit Dezimalstellen geschrieben werden (3.7); sie müssen keine ganzen Zahlen sein, und es gibt keine Begrenzung der Stellenzahl. Koordinaten werden wie geschrieben gezeichnet und nicht auf ganze Zahlen gerundet.
- **Raster.** Pins sollten auf ganzzahligen Punkten liegen (9.2.4); Grafiken in Symbolen sind nicht eingeschränkt. Wird eine Instanz mit einer der acht Ausrichtungen auf einen ganzzahligen Punkt platziert, bleiben ganzzahlige Pin-Positionen nach der Transformation ganzzahlig, sodass Leitungen ebenfalls auf ganzzahligen Punkten enden: die ganzzahligen Punkte bilden das übliche Raster. Die Sprache hat kein eigenes Raster. Ob der Solver freie Koordinaten auf ganze Zahlen rundet, ist seine eigene Regel, nach allen Sprachbeschreibungen eingeordnet (8.5); erzwingen Bedingungen eine Dezimalzahl, wird die Dezimalzahl verwendet. Die Knickpunkte, die ein Leitungsmodus berechnet, bleiben wie berechnet und werden nicht gerundet (13.1).
- **Toleranz.** Bei der Prüfung, ob eine Bedingung gilt, und bei der Entscheidung von Konflikten (8.5) gilt eine Abweichung innerhalb der Toleranz als erfüllt. Die Toleranz ist eine Option des Solvers, die der Benutzer einstellen kann; dieser Standard legt ihren Wert nicht fest, doch ist sie größer als 0, damit ein Wert ohne endliche Dezimaldarstellung zurückgeschrieben werden kann (unten). Ein Viewer, der nicht löst, verwendet bei der Prüfung seine eigene Einstellung. Zwei Werkzeuge können daher bei einem Fall genau an der Toleranzgrenze voneinander abweichen. Ein Solver schreibt Koordinaten als Dezimalzahlen zurück, die als derselbe Wert wiedereingelesen werden, ohne weiteres Abschneiden; ein Wert ohne endliche Dezimaldarstellung, wie 1/3, wird mit so vielen Stellen geschrieben, dass die von ihm erfüllten Fakten beim Wiedereinlesen des Eintrags innerhalb der Toleranz weiterhin gelten (Kapitel 17).
- **Symbolkoordinaten.** Die lokalen Koordinaten in einem DEFINE-SYMBOL-Rumpf haben die Einheit und Achsen der Szene; ihr Ursprung ist der Ursprung des Symbols (9.2.1, 6.5). Es gibt keine Skalierung: Symbole nehmen keinen Skalierungsparameter an, und Instanzen können nicht skaliert werden. Ein Importer rechnet die Symbole anderer Werkzeuge in RNL-Einheiten um.
- **Andere Längen.** Alle Längen der Sprache stehen in Koordinateneinheiten und skalieren mit der Zeichnung: `min`, `max` und `exact` von DISTANCE, `radius=` von CIRCLE, `width=` und `height=` von RECTANGLE und ELLIPSE, die Textgröße `size` (wie `font-size` in SVG), die Linienbreite `linewidth`, die Pfeillänge `arrowsize` (9.3) sowie als Längen verwendete Parameter von Definitionen (wie `gap=3`). Eine Linienbreite von 0 ist die Ausnahme: eine Haarlinie, bei jeder Vergrößerung gleich dünn, die zugleich der Standard ist (9.3).
- **Ursprung und Papier.** Der Ursprung der Szene hat keine besondere Bedeutung. Jede Teilschaltung ist eine Zeichnung (6.1) mit eigenen Koordinaten; der Viewer wählt den Ausschnitt nach der Ausdehnung des Inhalts. RNL speichert weder ein Papierformat noch das Fangraster eines Editors.

### 11.8 Bündel: BUNDLE

Eine iterierte Instanz aus Virtuoso, etwa `I0<3:0>`, ist ein Symbol, aber vier Zeilen in der Netzliste:

```spice
* A unit cell repeated four times
* @RNL(type="header") { @VERSION("0.4")}
.subckt unit in out vdd vss
R1 in out 1k
.ends unit
XI0<3> D<3> out vdd vss unit
XI0<2> D<2> out vdd vss unit
XI0<1> D<1> out vdd vss unit
XI0<0> D<0> out vdd vss unit
* @RNL {
*   #XI0<3:0> @BUNDLE => @PLACE(30, 10) => %x;   /* four instances drawn as one symbol */
*   &D<3:0> @WIRE {(20, 10) %x:in};              /* a bus to the in pin of the bundle */
*   &out @WIRE {%x:out (40, 10)};                /* all four members are on out: an ordinary pin */
*   %x @TEXT("I0<3:0>", key="name")              /* the name, written out */
* }
.end
```

- **Ein neues Objekt.** BUNDLE nimmt eine Menge von Host-Instanzen und erzeugt ein Bündel, das einmal anstelle dieser Instanzen gezeichnet wird. Die Mitglieder stehen in der geschriebenen Reihenfolge, und das erste Mitglied bestimmt die Pins und das Symbol.
- **Pins und Symbol.** Die Anschlussnamen und die Anzahl der Anschlüsse sind die des ersten Mitglieds. Die Darstellungswarteschlangen behandeln das Bündel wie sein erstes Mitglied: Auswahlen erreichen das Bündel anhand der Bedingungen seines ersten Mitglieds (10.1), und das Blocksymbol der Teilschaltung wird angewendet. SET-SYMBOL, Stile, PLACE, ORIENT, LAYER, Bedingungen und direkt auf das Bündel geschriebene Texte gehören dem Bündel.
- **Anschlüsse.** Ein Anschluss des Bündels, wie `%x:in`, steht für die gleichnamigen Anschlüsse aller Mitglieder: er ist ein Bus-Pin.
  - **Nach Namen**: der Anschluss des Bündels steht für die Anschlüsse jenes Namens der Mitglieder, wo auch immer sie in der Reihenfolge jedes Mitglieds stehen; ein Mitglied ohne einen Anschluss dieses Namens gehört nicht dazu.
  - `::net`: wenn alle Mitglieder auf demselben Netz liegen, jenes Netz, etwa `&out` für `%x:out::net`; andernfalls die Menge ihrer Netze in Mitgliederreihenfolge, etwa D<3>, D<2>, D<1>, D<0> für `%x:in::net`. Daher zeichnet `?t::net @WIRE {…}` in einer Definition einen Bus, wenn es eine Menge erhält, und eine gewöhnliche Leitung, wenn es ein einzelnes Netz erhält (13.1).
  - Eine Leitung zu einem Anschluss des Bündels verbindet diesen Anschluss jedes Mitglieds (10.2).
- **Attribute** (13.6): hat ein Wert bei allen Mitgliedern denselben Wert, dieser Wert; andernfalls die Werte verbunden mit `,`. `${w}` zeigt 1u, wenn alle 1u sind; `${name}` ist `XI0<3>,XI0<2>,XI0<1>,XI0<0>`, und um `I0<3:0>` anzuzeigen, schreibt man einen Text mit demselben Schlüssel, wie im Beispiel. `${net}` eines Anschlusses funktioniert ebenso, etwa `D<3>,D<2>,D<1>,D<0>`.
- **Auswahlen.** Mitglieder können nicht ausgewählt werden; stattdessen wird das Bündel ausgewählt (10.1). So gibt eine Regel wie „ein Netzname für jeden unverdrahteten Pin“ einem Bündel eine Beschriftung, die den ganzen Bus zeigt. Eine Rolle einer Definition, die Instanzen annimmt, nimmt auch Bündel an, und eine, die Anschlüsse annimmt, nimmt auch die Anschlüsse von Bündeln an (9.4).
- **Mitglieder werden nicht für sich gezeichnet.** Eine das Zeichnen betreffende Beschreibung, die direkt auf ein Mitglied oder auf einen Anschluss eines Mitglieds geschrieben ist (Position, Ausrichtung, Symbol, Stil, Zeichenebene, Bedingung, Leitung oder Text), schlägt fehl: das Mitglied wird ausschließlich über das Bündel gezeichnet. Beschreibungen, die nicht das Zeichnen betreffen, funktionieren wie gewohnt, etwa CURRENT und die Rollen semantischer Anwendungen.
- **Alle von einer Sorte.** Die Mitglieder sollten denselben Modell- oder Teilschaltungsnamen und dieselben Anschlussnamen haben, in Anzahl, Namen und Reihenfolge. Ist dies nicht der Fall, wird eine Warnung ausgegeben; sie werden dennoch gebündelt, mit den Pins und dem Symbol des ersten Mitglieds. Mitglieder, die sich nur in ihren Parametern unterscheiden, geben keine Warnung: ihre Attribute zeigen die verbundenen Werte (oben).
- **Ein Bündel für jede Instanz.** Eine Instanz, die in mehreren BUNDLEs steht, gehört zu dem später geschriebenen (1.5 Punkt 2); das frühere behält seine übrigen Mitglieder, und ein Hinweis darf ausgegeben werden. Ein früheres Bündel, dessen Mitglieder alle übernommen wurden, ist leer und wird mit einer Warnung nicht gezeichnet.
- BUNDLE nimmt ausgeschriebene Objekte an: erhält es das Ergebnis einer Auswahl oder eine davon abhängige Menge (10.4), schlägt es fehl, da ein Bündel bestehen muss, bevor die Symbole zugewiesen werden (8.2, 10.4). Aus demselben Grund schlägt ein BUNDLE in Inhalt fehl, den Schritt 6 generiert. BUNDLE mit einer leeren Menge schlägt fehl (5.2.1). Ein Mitglied, das keine Host-Instanz ist, schlägt fehl (falsche Art). Ein Mitglied, das nicht gefunden werden kann, lässt die gesamte Anweisung fehlschlagen (streng, 7.3).

<!-- translated from en/ch12.md sha256:5da84c59c21c -->

## 12 Virtuelle Bauteile

### 12.1 Konzept

- Ein virtuelles Bauteil ist ein Bauteil, das ausschließlich in RNL existiert, etwa eine Masse, eine Versorgung, eine Port-Markierung oder eine Nicht-verbunden-Markierung. Elektrisch existiert es nicht: es wird nur gezeichnet und erzeugt oder verändert keine elektrischen Fakten.
- Ein virtuelles Bauteil ist eine Anwendung, die ein Symbol hat; es gibt kein eigenes Schlüsselwort und keine eigene Objektart dafür. `$GROUND @ATTACH &vss` erzeugt eine Anwendung; das Standardpaket gibt den Anwendungen von `$GROUND` ein Massesymbol, sodass sie gezeichnet wird.
- Regeln:
  1. **Pins und Rollen.** Eine Anwendung hat keine Anschlüsse. Die Pins ihres Symbols werden über Namen angesprochen, wie `%g0::pin` (6.5), und `%g0::p` ist weiterhin das Mitglied, das Rolle p füllt, das Netz vss. Ein Pin, dessen `terminal=` eine Rolle der Definition benennt, zugeordnet nach Namen oder über `terminals=` (11.3), ist an diese Rolle gebunden: er wird auf dem gezeichnet, was die Rolle füllt, sodass eine Leitung auf diesem Netz dort enden darf und ROUTE-NET ihn einbezieht (13.7), und `::net` des Pins ist dieses Netz. Ein Pin, der eine Rolle bindet, wird auf dem Netz dessen gezeichnet, was sie füllt: ein Netz, oder das Netz eines Anschlusses oder eines Pins (5.2.4). Daher setzt `$GROUND @ATTACH #M5:3` eine Masse auf das Netz von M5:3 an einer eigenen Position, nicht auf jenen Pin; um eine Masse am Pin zu zeichnen, verwendet man eine Definition, deren Rumpf dies tut, wie `$GROUND_STUB` (10.5). Eine gebundene Rolle, die mit dem leeren Knoten gefüllt ist (6.3), setzt den Pin auf kein Netz, was gültig ist: er wird dann nur durch Leitungen verbunden, die auf den leeren Knoten geschrieben sind (13.1), und die Ergänzung des Standardpakets gibt ihm nichts. Eine gebundene Rolle, die mit einer anderen Art gefüllt ist, etwa einer Instanz oder einem Punkt, gibt dem Pin kein Netz: ein Artfehler, dessentwegen SET-SYMBOL für diese Anwendung fehlschlägt, die nicht gezeichnet wird, und die Diagnosemeldung verweist auch auf die Anwendung. Eine mit der Art `"net"` deklarierte Rolle wandelt einen Anschluss beim Anwenden der Definition in sein Netz um (9.4), und einen Pin, sobald die Symbole zugewiesen sind (5.2.4). Eine Rolle, die kein Pin bindet, nimmt die Arten an, die ihre Definition deklariert, etwa den Anschluss von `$NO_CONNECT` (12.6). Sie ist kein elektrischer Anschluss: SELECT-TERMINAL wählt sie nicht aus und ASSERT-TERMINAL prüft sie nicht; SELECT-PIN tut es (10.1). Ein Pin, der keine Rolle bindet, wird nur gezeichnet; er wird von Hand über seinen Namen verdrahtet. Ein Pin-Name darf nicht gleich einem Rollennamen sein (6.5).
  2. Eine Anwendung wird nur gezeichnet, wenn sie ein Symbol erhalten hat; eine Anwendung ohne Symbol wird nicht gezeichnet. Das Standardpaket gibt nur den Definitionen Symbole, die gezeichnet werden sollen, sodass semantische Anwendungen wie Differenzpaare nicht gezeichnet werden.
  3. Eine Anwendung ohne Symbol zeichnet selbst nichts: ORIENT darauf schlägt fehl, ebenso eine Leitung zu einem Pin-Namen von ihr, da sie keine Pins hat (9.2.4); dies wird nach Schritt 7 von 8.2 geprüft. Sie hat dennoch einen eigenen Punkt, `%u::position`: PLACE nimmt diesen Punkt an, ebenso auf `%u::position` geschriebene Bedingungen, während eine Bedingung auf der Anwendung selbst ihre Mitglieder annimmt (14.1); der Punkt zeigt sich in der Zeichnung nur über das, was der Rumpf ihrer Definition an ihn bindet (14.1, 9.4). Eine Leitung oder eine Grafik darf an diesem Punkt nur über `::position` enden, etwa `&n {%u::position %pt} @WIRE`: es ist ein Punkt, kein Pin, sodass die Leitung nichts verbindet (5.2.4, 6.5). Ein ihr gegebener Stil ist kein Fehler: der Stil geht auf die in ihrem Rumpf erzeugten zeichenbaren Objekte über (8.3) und hat keine sichtbare Wirkung, wenn im Rumpf nichts gezeichnet wird (11.3).
  4. Eine gezeichnete Anwendung zeichnet nur ihr eigenes Symbol. Sie enthält ihre Mitglieder nicht, verschiebt sie nicht und ist kein visueller Behälter.
  5. **Wo sie gezeichnet wird.** Der Ursprung ihres Symbols ist der eigene Punkt der Anwendung (14.1): PLACE und Bedingungen verschieben ihn, ORIENT dreht das Symbol um ihn (11.2), und er folgt den Objekten, die die Rollen füllen, nicht, außer über Fakten, wie Leitungen zu seinen Pins und die Fakten im Rumpf ihrer Definition. Eine Definition, die eine Markierung auf einem Pin oder an einem Punkt zeichnet, bindet den Punkt der Anwendung in ihrem Rumpf daran, wie `{? ?t} @ALIGN("x"); {? ?t} @ALIGN("y")` (9.4). Ohne ORIENT wählt der Solver ihre Ausrichtung und schreibt sie zurück (14.8), und ein Renderer, der keine Lösung dafür hat, zeichnet `"R0"` (8.5); er folgt nicht der Richtung, in der Leitungen einen Pin verlassen. Das Objekt, das die erste Rolle füllt, ist das Hauptobjekt der Anwendung; eine Anwendung hat keinen eigenen Namen, und Texte verweisen über die Rolle auf das Hauptobjekt, wie `${p}` (13.6).
  6. Der Rumpf einer Definition wird stets expandiert, daher darf die Definition eines virtuellen Bauteils einen Rumpf haben.
- In den Begriffen von Prinzipien 2.2 ist ein virtuelles Bauteil eine Anwendung mit Symbol, deren Rollen mit Host-Objekten gefüllt sind; ein standardmäßiges virtuelles Bauteil ist eine Definition des Standardpakets mit einer Regel, die ihm ein Standardsymbol gibt; ein benutzerdefiniertes virtuelles Bauteil ist eine eigene Definition des Benutzers zusammen mit einer Symbolregel oder einem in ihrem Rumpf vergebenen Symbol.
- **Definieren eines virtuellen Bauteils.** Der übliche Weg ist eine allgemeine Definition, unmittelbar gefolgt von der Regel, die ihren Anwendungen ihr Symbol gibt: `$GROUND = @DEFINE(roles=[["p", "net"]]) {}; # @SELECT-USAGE(recursive=true) $GROUND => @SET-SYMBOL $GROUND_SYMBOL`. Die Regel nimmt nach Position ihren Platz in der Symbol-Warteschlange ein (8.3), sodass eine spätere Regel eines Hausstilpakets oder der Datei dieselben Anwendungen anders zeichnet, etwa `# @SELECT-USAGE(recursive=true) $GROUND => @SET-SYMBOL $EARTH`, auch jene, die in anderen Definitionen oder durch die Ergänzung erzeugt wurden; eine einzelne Anwendung wird durch direktes Benennen anders gezeichnet, `%g1 @SET-SYMBOL $EARTH`. Der Rumpf darf das Symbol ebenfalls vergeben, `? @SET-SYMBOL $X`, wobei das alleinstehende `?` die Anwendung ist (9.4): das benennt die Anwendung direkt (8.3), sodass keine Regel es ersetzt, was zu einem Bauteil passt, das nicht anders gezeichnet werden soll. Das Ändern der Zeichnung ändert weder die Anschlüsse noch die Leitungen daran.
- Ob eine Anwendung ein virtuelles Bauteil ist, hängt allein davon ab, ob sie ein Symbol hat.
- Hinweis: Eine Definition, die gezeichnet werden sollte, aber kein Symbol erhält, wird erst gemeldet, wenn eine ihrer Anwendungen ausgerichtet oder verdrahtet wird oder wenn eine Bedingung darauf keine sichtbare Wirkung hat (14.1).

### 12.2 Erzeugen virtueller Bauteile

```rnl id=g0
$GROUND @ATTACH &vss => @PLACE(24, 38) => %g0;
&vss @WIRE {#M5:3 %g0::pin}
```

- In `$D @ATTACH &net` werden die Objekte nach der Definition der Rollenliste der Definition zugeordnet. Dies wird mit ATTACH geschrieben und als „dieses auf dem Netz hinzufügen“ gelesen (9.4). Ein virtuelles Bauteil mit mehreren Rollen wird wie eine Instanzzeile der Netzliste geschrieben: bei einer Definition `$DIFF_PORT` mit den Rollen a und b schreibt man `$DIFF_PORT @ATTACH &inp(a) &inn(b)` oder nach Reihenfolge `$DIFF_PORT @ATTACH &inp &inn`.
- Die Rollen werden üblicherweise mit Netzen gefüllt: die Pins eines virtuellen Bauteils, die seine Rollen binden, werden auf diesen Netzen gezeichnet (12.1 Punkt 1), und seine Beziehung zu einem Pin eines Bauteils (wo es gezeichnet wird, womit es verbunden ist) wird durch Leitungen und Bedingungen angegeben (12.3). Ein Pin, der eine Rolle bindet, wird auf dem Netz dessen gezeichnet, was sie füllt (12.1 Punkt 1): `$GROUND @ATTACH #M5:3` hängt eine Masse an das Netz von M5:3 an einer eigenen Position, genau wie `$GROUND @ATTACH #M5:3::net`. Die Objektarten, die eine Rolle annimmt, können in der Definition deklariert werden (9.4), etwa `roles=[["p", "net"]]`.
- Diese Zuordnung ist nur beschreibend: sie stellt keine elektrische Verbindung her und wird nicht geprüft (Prinzipien 2.2).
- Ein Netz kann beliebig viele virtuelle Bauteile haben, etwa fünf Massen und zwei Messpunkte. Anwendungen haben keinen Schlüssel: jedes Anwenden erzeugt eine weitere Anwendung (5.8).
- Einem einzelnen virtuellen Bauteil kann ein anderes Symbol gegeben werden, etwa `%g1 @SET-SYMBOL $EARTH` mit einem eigenen Symbol `$EARTH`.
- ATTACH hat keine eigenen Parameter außer `priority` (14.8). Die Richtung eines Ports gehört zum Inhalt des Standardpakets (12.4); der angezeigte Name ist der des Hauptobjekts, und um einen anderen Text anzuzeigen, schreibt man eine Beschriftung (13.3). Die Parameter der Definition werden wie üblich über ATTACH übergeben (9.4).
- `%g0` wird in einer Ebene nur einmal gebunden (6.6); um seine Position zu ändern, schreibt man `%g0 @PLACE(…)`.

### 12.3 Positionen relativ zu einem Anschluss

Parameter können keine Host-Verweise halten, daher wird eine Position relativ zu einem Anschluss als Bedingung geschrieben:

```rnl use=g0
{%g0::pin #M5:3} @ALIGN("x");
%g0::pin #M5:3 @DISTANCE("bottom-to-top", exact=2)    /* the pin of the ground 2 below the source of M5 */
```

Mit `@ALIGN("y")` anstelle von DISTANCE liegt der Pin der Masse auf dem Pin von M5. Eine Leitung der Länge 0 zwischen den beiden Pins, `&vss @WIRE {#M5:3 %g0::pin}` (13.1), macht den Pin ebenfalls verdrahtet, sodass die Ergänzung dort keine zweite Masse hinzufügt (10.2, 10.5).

### 12.4 Ports

```rnl
$IO_PORT @ATTACH &inp => @PLACE(0, 20) => %p_inp
```

- Der Name eines Ports einer Teilschaltung ist der Name des Netzes in ihr, daher ist eine Port-Markierung dem Netz `&inp` zugeordnet.
- Ein Port selbst (`#:inp`) wird nicht gezeichnet; er wird ausschließlich durch ein virtuelles Bauteil dargestellt. Die Ergänzung der Standardserie 0.4 gibt über die Menge `$PORT_FILL_SET` jedem Portnetz eine Port-Markierung, auch Versorgungs- und Massenetzen, und lässt die Netze aus, die eine geschriebene Port-Markierung haben (10.5); die Markierungen einer Ebene stehen in einer Spalte, jede bei R0 mit ihrem Pin nach rechts, und die Spalte bleibt, von ihrem eigenen Punkt aus gemessen, mindestens 6 Einheiten links von jeder Instanz der Ebene, durch Bedingungen der Priorität `"low"`; den Rest entscheidet der Solver. Eine Datei nimmt ein Netz aus der Menge heraus, damit es ohne Markierung bleibt, etwa `# @SELECT-NET(name="vdd") => @REMOVE-FROM $PORT_FILL_SET`; schreibt `$IO_PORT @ATTACH` für eine Markierung, die sie selbst platziert; oder schreibt einmal in einer Ebene `$PORT_LIST @ATTACH` für dieselbe Spalte, ausgeschrieben, und die Ergänzung fügt dann in dieser Ebene keine hinzu. Der eigene Punkt einer `$PORT_LIST` steht auf ihrer Spalte (14.1), sodass eine Bedingung an der Anwendung die Spalte verschiebt.
- Die Richtung eines Ports ist nur eine Frage der Darstellung: die Netzliste hat keine Richtungen, und der Benutzer entscheidet, welches Symbol gezeichnet wird. Welche Port-Definitionen und -Symbole es gibt, gehört zum Inhalt des Standardpakets.
- **Ausrichtung von Port-Markierungen.** Eine Port-Markierung hat eine Richtung: der Pin von `$IO_PORT_SYMBOL` im Standardpaket liegt etwa an einem Ende, und die Markierung erstreckt sich in die andere Richtung. Ohne ORIENT wählt der Solver die Ausrichtung aus den Richtungen der Leitungen, mit dem Ziel möglichst weniger Knicke. Dies ist ein Ziel des Solvers, nicht Teil der Sprache (14.8), daher wird das ORIENT einer Port-Markierung üblicherweise nicht von Hand geschrieben.

### 12.5 Standardmäßige virtuelle Bauteile

Die standardmäßigen virtuellen Bauteile gehören zum Standardpaket, das nicht Teil dieses Sprachstandards ist (16.3). Das Standardpaket stellt beispielsweise eine Masse `$GROUND`, eine Versorgung `$SUPPLY`, eine Port-Markierung `$IO_PORT` und eine Nicht-verbunden-Markierung `$NO_CONNECT` (12.6) bereit, samt ihren Symbolen, den Regeln, die diese Symbole vergeben, und der Ergänzung (10.5).

Für ein Netz vss liefern drei Schreibweisen drei verschiedene Zeichnungen:

1. `&vss @ADD-TO $GROUND_NET_SET`: jeder Anschluss auf vss ohne Leitung erhält darunter eine Masse mit einer kurzen Leitung (die Ergänzung des Standardpakets, 10.5). So zeichnen es Lehrbücher üblicherweise: ein Massesymbol unter jedem geerdeten Anschluss.
2. `$GROUND @ATTACH &vss => %g`: eine Masse für das ganze Netz, vom Solver platziert. Die Anschlüsse werden durch Leitungen mit ihrem Pin verbunden, etwa `&vss @WIRE {#M5:3 %g::pin}`, oder durch ROUTE-NET für das ganze Netz (13.7).
3. `$GROUND_STUB @ATTACH #M5:3`: eine Masse mit einer kurzen Leitung nur unter dem Source-Anschluss von M5, so wie die Ergänzung sie für jeden Pin zeichnet (10.5).

- Eine Versorgung wird ebenso geschrieben, mit der Menge `$SUPPLY_NET_SET` und der Definition `$SUPPLY`. Netz 0 ist standardmäßig in `$GROUND_NET_SET` (10.1).
- Bei der dritten Schreibweise macht die Leitung im Rumpf des Stubs den Pin verdrahtet, sodass die Ergänzung dort keine zweite Masse hinzufügt (10.2, 10.5).
- Ports: die Ergänzung gibt jedem Portnetz einer Teilschaltung eine Port-Markierung (12.4). Um eine zu platzieren, mit Bedingungen zu versehen oder den angezeigten Namen zu ändern, schreibt man sie aus: `$IO_PORT @ATTACH &out => %pout`, mit Leitungen zu `%pout::pin` und `%pout @TEXT("V_{OUT}", key="name")` für den Namen (13.3). Ein Netz, das keine Port-Markierung haben soll, wird aus `$PORT_FILL_SET` herausgenommen.

### 12.6 Nicht-verbunden-Markierungen

```rnl
$NO_CONNECT @ATTACH #M1:4    /* terminal 4 of M1 is left unconnected on purpose */
```

- Eine Nicht-verbunden-Markierung ist die Definition `$NO_CONNECT` des Standardpakets, deren Rolle einen Anschluss oder einen Pin annimmt; es gibt kein Schlüsselwort dafür. Ein Anschluss wird als er selbst aufgenommen, nicht umgewandelt (5.2.4), sodass eine Markierung auf einem Mitglied eines Bus-Pins (9.2.4), wie `$NO_CONNECT @ATTACH #U1:d<3>`, angibt, welcher Anschluss offen bleibt; mehrere Markierungen auf einem Bus-Pin werden an diesem Pin übereinander gezeichnet. Kein Pin des Markierungssymbols bindet die Rolle (12.1 Punkt 1); die Definition bindet den Punkt der Anwendung an den Pin dessen, was die Rolle füllt (12.1 Punkt 5), und das Standardpaket gibt ihr ein Kreuzsymbol mit dem Ursprung im Mittelpunkt des Kreuzes und ohne Pin.
- Die Anwendung hält fest, dass der Anschluss absichtlich unverbunden bleibt: `# @SELECT-USAGE $NO_CONNECT => @SELECT-TERMINAL` wählt diese Anschlüsse aus.
- Prüfungen (weitere Host-Anschlüsse auf dem Netz, eine Leitung, die den Anschluss berührt) sind nicht erforderlich (Prinzipien 2.4); eine Implementierung, die das standardmäßige `$NO_CONNECT` kennt, darf eine Warnung ausgeben.
- Eine Nicht-verbunden-Markierung zählt nicht als Leitung (10.2): ein Pin, der nur die Markierung hat, ist weiterhin `wired=false`. Damit eine Ergänzung nach Pin diese Pins überspringt, zieht man ihre Pins ab, `# @SELECT-USAGE $NO_CONNECT => @SELECT-TERMINAL => @SELECT-PIN`, wie bei der Differenz in 10.5.
- Hinweis: Eine Nicht-verbunden-Markierung ist kein Schlüsselwort, weil sie für eine Implementierung außer zum Zeichnen keinen Nutzen hat: der Solver liest sie nicht, und wie sie gezeichnet wird, lässt sich in einer Definition und einem Symbol schreiben (1.5 Punkt 7).
- Netzbeschriftungen sind keine virtuellen Bauteile; siehe 13.3.

<!-- translated from en/ch13.md sha256:492780434318 -->

## 13 Leitungen und Text

### 13.1 Leitungen: WIRE

```rnl id=w13
&x @WIRE(mode="straight") {#M1:1 (10, 37) #M3:1} => @SET-STYLE $signal => %w13;
&out @WIRE(mode="manhattan") {#M2:1 #M4:1};
&tail @WIRE {#M1:3 #M5:1}                    /* no mode: "auto", routed freely */
```

- WIRE nimmt ein Netz (oder eine Menge von Netzen, was einen Bus ergibt; siehe unten) und eine geordnete Liste von mindestens zwei Punkten. Ein Punkt ist ein Punktobjekt oder ein Pin, einschließlich eines Pins eines virtuellen Bauteils; ein Anschluss steht für seinen Pin (5.2.4). Punkte dürfen sich wiederholen; sind der erste und der letzte Punkt gleich, bildet die Leitung eine Schleife.
- WIRE nimmt nicht das Ergebnis einer Auswahl an, und auch keine davon abhängige Menge (10.4): erhält es eine solche, schlägt es fehl. Eine Leitung wird vom Solver als diese Leitung samt ihren Knicken zurückgeschrieben (14.8), und eine aus einer Auswahl erzeugte Leitung hätte nichts, als das sie zurückgeschrieben werden könnte; um die Pins einer Auswahl als Leitungen zu zeichnen, verwendet man ROUTE-OBJECT (13.7).
- **Leitungsmodus**: die Leitung verläuft durch jeden Punkt der Liste; wie sie zwischen zwei benachbarten Punkten verläuft, legt der Parameter `mode=` fest. Für benachbarte Punkte A(x1, y1) und B(x2, y2) mit dx = x2 − x1 und dy = y2 − y1:

  | `mode` | Von A nach B |
  |---|---|
  | `"auto"` (Standard) | Vom Solver entschieden, der sie für die jeweilige Situation optimieren darf und den Pfad zurückschreibt (siehe unten, 14.8); bevor die Leitung gelöst ist, wird sie nach dem unten stehenden Standardpfad gezeichnet, den ihr auch der Solver gibt, wenn nichts im Weg steht |
  | `"straight"` | Ein gerades Segment |
  | `"horizontal-vertical"` | A → (x2, y1) → B |
  | `"vertical-horizontal"` | A → (x1, y2) → B |
  | `"manhattan"` | Wenn \|dx\| ≥ \|dy\|: die Hälfte von dx waagerecht, ganz dy nach oben oder unten, dann die andere Hälfte waagerecht: A → (x1 + dx/2, y1) → (x1 + dx/2, y2) → B; andernfalls ebenso mit vertauschtem x und y |
  | `"manhattan45"` | Wenn \|dx\| ≥ \|dy\|: (\|dx\| − \|dy\|)/2 waagerecht, dann eine 45°-Diagonale (\|dy\| entlang jeder Achse), dann der Rest waagerecht; andernfalls ebenso mit vertauschtem x und y |

  - Jeder Modus außer `"auto"` ist eine strikte Regel, und der Viewer zeichnet ihn wie definiert. Ist dx oder dy gleich 0, liefert jeder Modus ein gerades Segment. Bei |dx| = |dy| wählt `"manhattan"` den Fall |dx| ≥ |dy|, und `"manhattan45"` ist ein einzelnes diagonales Segment. Die von einem Modus berechneten Knicke behalten ihre exakten Werte und werden nicht gerundet; sie können auf halbe Einheiten fallen (11.7).
  - **Der Standardpfad von `"auto"`**: jedes Segment zwischen zwei benachbarten Punkten verläuft waagerecht und senkrecht. Ein Ende, das ein Pin mit einer Richtung ist (9.2.4), genommen nach der Ausrichtung seines Objekts, ist gerichtet: das Segment verlässt ihn entlang dieser Richtung oder im rechten Winkel dazu und trifft entgegen dieser Richtung oder im rechten Winkel dazu auf ihn, nie andersherum; das Verlassen oder Eintreffen im rechten Winkel zählt als Knick. Ein Pin ohne Richtung und ein Punkt sind ungerichtet und schränken nichts ein. Das Segment verläuft durch keines seiner Enden und überlappt sich nicht selbst. Unter den Pfaden, die dies erfüllen, ist der Standardpfad jener mit den wenigsten Knicken; dann der kürzeste, wobei ein Abschnitt, der nur hinausgeht, um zurückzukommen, 1 lang ist; dann jener, der an weniger seiner gerichteten Enden abbiegt, wobei die Richtung des ersten Endes vor der des letzten beibehalten wird; dann jener, dessen mittlerer Abschnitt in der Mitte zwischen den Enden liegt, wie bei `"manhattan"`; zwischen zwei ungerichteten Enden jener, der zuerst waagerecht verläuft, ein L mit einem Knick; und ein Umweg verläuft links vom ersten gerichteten Ende (dessen Richtung um 90 Grad gegen den Uhrzeigersinn gedreht). Eine Leitung mit Punkten zwischen ihren Enden wird als Ganzes gewählt: ihre Zwischenpunkte sind ungerichtet, eine Richtungsänderung an einem Zwischenpunkt zählt als Knick, und die Leitung darf an einem Zwischenpunkt nicht umkehren, verläuft also gerade durch einen solchen hindurch, wo sie kann. Auch die Knicke des Standardpfads behalten ihre exakten Werte und können auf halbe Einheiten fallen (11.7), etwa wenn sein mittlerer Abschnitt in der Mitte zwischen zwei Enden im Abstand einer Einheit liegt.
  - Die von einem Modus hinzugefügten Knicke sind keine Objekte: auf sie kann nicht verwiesen werden, und auf ihnen kann kein Knotenpunkt gesetzt werden. Um an einem Knick abzuzweigen oder dort einen Punkt zu zeichnen, schreibt man den Knick als Punkt. Ein freier Punkt `(_, _)` erhält zuerst seine Koordinaten vom Solver und wird dann gemäß dem Modus verbunden.
  - Ein Wert von `mode`, der nicht in der Tabelle steht, ist ein ungültiger Parameterwert: die Leitung schlägt fehl.
- **Stilparameter**: WIRE nimmt außerdem die Linienstileigenschaften als Parameter an: `linecolor`, `linestyle`, `linewidth`, `cap`, `join` (9.3). Sie bilden den direkten Stil der Leitung (8.3), etwa `@WIRE(linestyle="bold")` für eine Versorgungsschiene oder `@WIRE(linestyle="dashed")` für einen Substratanschluss.
- **Neuschreiben einer Leitung.** Erhält WIRE anstelle eines Netzes eine Leitung, erzeugt es keine neue Leitung: es schreibt diese Leitung neu. Die Anweisung wird so gelesen wie ein neues WIRE, und ihre Punkteliste sowie ihr Modus ersetzen jene der Leitung; das Netz der Leitung bleibt erhalten. Stilparameter sind direkter Stil der Leitung (8.3).
  - Das Neuschreiben hat einen Schlüssel, die Leitung: ein späteres ersetzt die Liste und den Modus eines früheren (8.4); bei zwei Segmenten einer Pipe ist das spätere Segment das spätere (5.5). Die Stilparameter jedes Neuschreibens bleiben direkter Stil der Leitung und werden gemäß 8.3 Eigenschaft für Eigenschaft aufgelöst. Ein Neuschreiben ist kein geometrischer Fakt (8.5).
  - Ein fehlschlagendes Neuschreiben, etwa eines, das einen Anschluss ohne Pin aufführt (9.2.4), ist ein Fehler und hat keine Wirkung, seine Stilparameter eingeschlossen; die Leitung behält ihre frühere Liste und ihren früheren Modus: jene eines früheren Neuschreibens oder jene, mit denen sie erzeugt wurde (5.8). Jedes fehlschlagende Neuschreiben wird gemeldet, auch wenn ein späteres es ersetzt.
  - Wie eine neue Leitung zählt ein Neuschreiben ab Schritt 3 von 8.2: `wired=` (10.2) liest die Punkte des letzten Neuschreibens, auch wenn dieses Neuschreiben später, nach Schritt 7, fehlschlägt (7.1).
  - Der Netz-Slot nimmt ein Netz, eine Menge von Netzen oder eine Leitung; eine Menge, die eine Leitung enthält, hat die falsche Art.
  - Der Solver schreibt den Pfad, den er für jede `"auto"`-Leitung gewählt hat, auf diese Weise zurück (14.8). Er sollte die Punkte, die die Leitung in den verfassten Einträgen nach ihrem letzten Neuschreiben besitzt, in ihrer Reihenfolge behalten, nur als Koordinaten geschriebene Knicke dazwischen einfügen und `mode="straight"` schreiben: die Verbindungen einer Leitung werden von ihrem Verfasser geschrieben, und der Solver entscheidet nur ihre Geometrie.
  - **Aufteilen einer Leitung.** Auf einem Netz, das ein Baum von ROUTE-NET oder ROUTE-OBJECT verbindet (13.7), darf der Solver aus der Mitte einer vom Verfasser geschriebenen Leitung abzweigen. Er schreibt dann diese Leitung in seinem Eintrag nach Namen oder Nummer neu (5.9), behält die Punkte des Verfassers in ihrer Reihenfolge und fügt den Verzweigungspunkt als eigenen Punkt dazwischen ein und schließt dort die Verzweigung an. Andere Leitungen ändert er nur wie im obigen Punkt angegeben. Dies ist ein Neuschreiben, keine Löschung: die verfasste Anweisung bleibt erhalten.
  - Ein Editor, in dem der Benutzer eine Leitung umformt, schreibt dasselbe in seinen eigenen Eintrag; der Solver behält die Leitung dann wie neu geschrieben.
  - Um den Pfad in der Anweisung selbst festzulegen, schreibt man die Knicke als Punkte und verwendet einen anderen Modus als `"auto"`; ein Exporter, der die genaue Form bewahrt, schreibt `mode="straight"` und listet jeden Knick auf.
- **Mehrere Anschlüsse auf einem Netz** (Prinzipien 2.6) werden durch mehrere Leitungen verbunden, die gemeinsame Punkte teilen. Leitungen sind nur dann verbunden, wenn sie dasselbe Punktobjekt (einen benannten Punkt) oder denselben Anschluss teilen; gleiche Koordinaten verbinden sie nicht (Prinzipien 2.2). Es gibt keine Anweisung, die einen Baum schreibt, und kein Graphobjekt.
- **T-Verzweigung**: die durchgehende Leitung führt den Verzweigungspunkt als einen ihrer Zwischenpunkte auf und muss nicht in zwei geteilt werden; die Verzweigung beginnt an diesem Punkt (siehe das Beispiel in 13.2). Es gibt keinen „Punkt auf einem Leitungssegment“.
- **Sich berührende Pins**: werden zwei Pins ohne Leitung dazwischen an demselben Punkt gezeichnet, wird dennoch eine Leitung geschrieben, um die beiden Anschlüsse zu verbinden, etwa bei einem direkt auf dem Pin eines Kondensators gezeichneten Massesymbol: `&0 @WIRE(mode="straight") {#C1:2 %g3::pin}`. Ihre beiden Enden fallen zusammen und ihre Länge ist 0, sodass nichts gezeichnet wird, aber sie gibt die Verbindung an: beide Pins sind verdrahtet (10.2), und das Standardpaket fügt keine weitere Masse hinzu. Gleiche Koordinaten allein verbinden nicht (Prinzipien 2.2). Ein Exporter, der einen Schaltplan konvertiert, in dem Pins durch Position verbinden, schreibt solche Leitungen; wo eine Leitung gerade durch einen Pin oder einen Knotenpunkt verläuft, führt er diesen Punkt als Zwischenpunkt auf (T-Verzweigung); wo ein Knotenpunkt nur eine Leitung hat und direkt auf einem Pin liegt, verbindet er die Leitung direkt mit diesem Anschluss.
- **Das Netz wird lediglich angegeben**, und es muss geschrieben werden. Die Netze, auf denen die Endpunkte tatsächlich liegen, müssen nicht damit übereinstimmen, und dies wird nicht geprüft (Prinzipien 2.2); das Netz wird auch nicht aus den Endpunkten hergeleitet, und die Endpunkte dürfen auf verschiedenen Netzen liegen.
  - Liegt ein Anschluss oder Pin in der Liste nicht auf dem Netz der Leitung (bei einer Busleitung: nicht in ihrer Netzmenge), liefert dies eine Warnung: höchstwahrscheinlich ist der Anschluss vertippt, etwa `#R1:1` statt `#R1:2`. Das Netz eines Anschlusses oder Pins ist bekannt (ein Pin eines virtuellen Bauteils liegt auf dem Netz, das die von ihm gebundene Rolle füllt, 12.1); ein Punkt und ein Pin, der nichts bindet, haben kein Netz und werden nicht geprüft. Die Leitung ist gültig und wird gezeichnet (Prinzipien 2.2, 7.6).
- **Eine Leitung auf keinem Netz**: der Netz-Slot nimmt auch den leeren Knoten an, wie in `& @WIRE {%sub1::y %int1::a}` (6.3). Die Leitung wird wie jede andere gezeichnet und gelöst, und die Pins, an denen sie endet, sind verdrahtet (10.2), aber sie gehört zu keinem Netz: zwei solche Leitungen, die einen Punkt teilen, werden verbunden gezeichnet und sind dennoch kein Netz, kein Baum von ROUTE-NET oder ROUTE-OBJECT nimmt sie auf, und `${net}` der Leitung ist der leere Text. Ein Anschluss oder Pin in ihrer Liste, der auf einem Netz liegt, liefert die obige Warnung, ebenso wie ein Pin auf dem leeren Knoten in der Liste einer Leitung eines Netzes: die Zeichnung würde eine Verbindung zeigen, die die Schaltung nicht besitzt. Der leere Knoten kann kein Element der Menge eines Busses sein.
- **Busse**: der Netz-Slot nimmt ein Netz oder eine Menge von Netzen als Ganzes. Erhält er eine Menge, ist die Leitung eine Busleitung, auch wenn die Menge ein Mitglied hat. Eine Netzliste kennt keine Busse (6.11); ein Bus ist nur eine Darstellungsform, mehrere Netze als eine Linie zu zeichnen.

  ```spice
  * A 4-bit bus between two blocks, with two taps
  * @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
  .subckt blk D<3> D<2> D<1> D<0>
  RL D<3> D<0> 1k
  * @RNL {
  *   $BLK_SYM = @DEFINE-SYMBOL {
  *     (-3, 0) @PIN(terminal=["D<3>", "D<2>", "D<1>", "D<0>"], label="D<3:0>", direction="left");
  *     @RECTANGLE {(-3, -2) (3, 2)}
  *   };
  *   # @SET-SYMBOL $BLK_SYM                                             /* one bus pin for four ports (9.2.4) */
  * }
  .ends blk
  XA D<3> D<2> D<1> D<0> blk
  XB D<3> D<2> D<1> D<0> blk
  RT3 D<3> 0 1k
  RT2 D<2> 0 1k
  * @RNL {
  *   %t3 = (20, 10); %t2 = (24, 10);
  *   %bus = &D<3:0> @WIRE(linestyle="bold") {#XA:D<3> %t3 %t2 #XB:D<3>};   /* a bus between the bus pins of XA and XB */
  *   %bus @TEXT("D<3:0>");                                                /* the name of the bus, written out */
  *   &D<3> @WIRE {%t3 #RT3:1};                                            /* a tap: a single wire that shares %t3 with the bus */
  *   &D<2> @WIRE(mode="straight") {#RT2:1 (23, 9) %t2}                    /* its last stretch enters the bus at 45° */
  * }
  .end
  ```

  - Die Reihenfolge der Menge ist die Reihenfolge der Bits, und Duplikate bleiben erhalten; direkt innerhalb der Menge geschriebene Mengen werden der Reihe nach bis zu ihren innersten Mitgliedern aufgelöst. Ein Mitglied, das kein Netz ist, lässt die Leitung fehlschlagen (falsche Art). Eine leere Menge schlägt ebenfalls fehl: eine Leitung trägt mindestens ein Netz (5.2.1).
  - Der Netz-Slot ist streng (7.3): kann ein Bit nicht gefunden werden, schlägt die gesamte Leitung fehl, ebenso wie wenn ein einzelnes Netz nicht gefunden werden kann.
  - Punkte, `mode=`, Stilparameter, Knotenpunkte und T-Verzweigungen sind wie bei anderen Leitungen. Es gibt kein Schlüsselwort BUS: eine Implementierung zeichnet einen Bus so, wie sie eine Leitung zeichnet, und ihn fett zu zeichnen ist eine Frage des Stils (1.5 Punkt 7). Ein Importer darf `linestyle="bold"` direkt schreiben; das Standardpaket stellt Busleitungen standardmäßig über die Bedingung `bus=true` von SELECT-WIRE fett dar (10.1, 16.3).
  - **Abzweige** benötigen keinen eigenen Mechanismus: eine Einzelleitung, die einen Punkt mit einem Bus teilt, ist ein Abzweig, und der Bus führt den Abzweigpunkt als Zwischenpunkt auf (T-Verzweigung). Ein kurzes 45°-Stück in den Bus hinein ist nur der Verlauf des letzten Abschnitts der Einzelleitung. Ein Abzweigsymbol ist eine am Abzweigpunkt angewendete Definition und gehört zum Standardpaket (16.3). Der Name des Bits ist eine Beschriftung `${net}` auf der Einzelleitung (13.3). Ob das Netz der Einzelleitung im Bus enthalten ist, wird nicht geprüft (Prinzipien 2.2); eine Implementierung darf einen Hinweis geben.
  - Ein Abzweig kann auch am Ende eines Busses beginnen: ein an einem Punkt endender Bus darf sich als mehrere Busse oder Einzelleitungen fortsetzen, die an diesem Punkt beginnen, etwa eine Stammleitung `BOT<10:0>`, die dort endet, wo `BOT<10:3>` und `BOT<2:0>` abgehen, mit einem Knotenpunkt dort (13.2). Die Stammleitung führt diesen Punkt als Ende auf, und jeder Teil nimmt seine eigenen Bits. Leitungen, die an einem Punkt beginnen, an dem auch eine Leitung verläuft oder endet, die alle ihre Netze trägt – hier die Stammleitung –, sind Abzweige dieser Leitung und nicht miteinander verbunden: eine Implementierung, die warnt, wenn eine Leitung auf einer Leitung eines anderen Netzes endet, als wären beide verbunden, warnt bei ihnen nicht.
  - Das Attribut `net` einer Busleitung sind die Namen ihrer Netze der Reihe nach, verbunden mit `,` ohne Leerzeichen, etwa `D<3>,D<2>,D<1>,D<0>`; um `D<3:0>` anzuzeigen, schreibt man diesen Text. Implementierungen kürzen die Liste nicht ab: eine Abkürzung hängt davon ab, wie der Host Namen schreibt, und hat kein eindeutiges Ergebnis.
  - Somit ist `{&a &b} @WIRE {…}` eine Busleitung und nicht zwei übereinander gezeichnete Leitungen.
  - Mehrere als ein Bauteil gezeichnete Host-Instanzen (eine iterierte Instanz) werden als Bündel geschrieben (11.8).
- Ein Anschluss, an den eine Leitung anschließt, muss einen Pin in seinem Symbol haben; andernfalls schlägt die Leitung fehl (9.2.4).
- Linien, die sich berühren, kreuzen oder überlappen, sind nur Geometrie und stellen keine Verbindung her (Prinzipien 2.2). Leitungen, die sich ohne gemeinsamen Punkt kreuzen, erhalten keinen Knotenpunkt; kleine Bögen an Kreuzungen und dergleichen sind eine Frage des Stils und der Implementierung.
- Jedes WIRE, das ein Netz erhält, ist ein neues Objekt ohne Schlüssel: zweimal geschrieben, ergibt es zwei Leitungen (5.8). Um aus einer anderen Anweisung auf eine Leitung zu verweisen, benennt man sie; der Eintrag des Solvers verweist auf eine unbenannte Leitung über ihre Nummer (5.9). Ein Endpunkt, der ein Anschluss ist, bewegt sich mit seinem Bauteil; ein mit Koordinaten geschriebener Punkt bewegt sich nicht; ein benannter Punkt kann mit Bedingungen versehen werden.
- **Nicht gezeichnete Verbindungen**: wie eine Verbindung dargestellt wird, die keine Leitung im RNL hat (vom Solver verlegt und als WIRE zurückgeschrieben, eine Luftlinie, eine Beschriftung), entscheiden der Hausstil des Solvers und des Viewers sowie die verwendeten Hausstilpakete; die Sprache legt dies nicht fest. Ein Hausstilpaket kann mit Regeln Beschriftungen oder Leitungen erzeugen, und diese sind RNL. Um ein Netz als Leitungen zeichnen zu lassen und die Leitungsführung dem Solver zu überlassen, schreibt man ROUTE-NET oder ROUTE-OBJECT (13.7).

### 13.2 Knotenpunkte

```rnl
%j1 = (10, 30);
&x @WIRE {#M1:1 %j1 #M3:1};          /* the through wire passes %j1 */
&x @WIRE {%j1 (30, 30) #M4:2};       /* the branch starts at %j1 */
$JUNCTION @ATTACH %j1                /* draws the dot */
```

- Eine Verzweigung sind mehrere Leitungen, die einen Punkt teilen (13.1).
- Knotenpunkte werden explizit geschrieben: ein Punkt wird nur dort gezeichnet, wo eine Definition angewendet wird, die einen zeichnet, etwa `$JUNCTION` des Standardpakets; ein Renderer fügt von sich aus keine Punkte hinzu.
- Ein Knotenpunkt ist eine Anwendung, kein Schlüsselwort. `$JUNCTION` ist eine Definition des Standardpakets mit einer Rolle, die einen Punkt oder einen Anschluss annimmt; ihre Definition bindet den Punkt der Anwendung an diesen Punkt oder an den Pin des Anschlusses (12.1 Punkt 5), wie bei `$NO_CONNECT` (12.6) und dem Strompfeil (15.2). Sie kann auf einen Anschluss angewendet werden, wenn zwei Leitungen an seinem Pin zusammentreffen. Das Standardpaket gibt diesen Anwendungen einen gefüllten Punkt als Symbol: die Größe des Punkts kommt vom Symbol und seine Farbe aus `linecolor` des Stils der Anwendung; es gibt keine eigene Einstellung für die Punktgröße.
  - Zweimaliges Anwenden von `$JUNCTION` auf denselben Punkt erzeugt zwei Punkte übereinander, die wie einer aussehen.
  - Alle Knotenpunkte werden mit `# @SELECT-USAGE $JUNCTION` ausgewählt.
  - Die Zeichenreihenfolge folgt der Position der Regel, die das Symbol zuweist (11.6). Die Regeln des Standardpakets stehen zuerst, sodass die Punkte vor den Leitungen gezeichnet werden; ein Punkt hat die Farbe der Leitungen und ist breiter als diese, sodass dies keinen sichtbaren Unterschied macht und kein LAYER nötig ist.
- Knotenpunkte werden nicht aus der Anzahl der an einem gemeinsamen Punkt zusammentreffenden Leitungen abgeleitet: reale Schaltpläne haben Verzweigungspunkte mit nur zwei Leitungen, die mit einem Punkt gezeichnet werden, und ein abgeleiteter Punkt bräuchte wiederum eine Möglichkeit, ihn zu entfernen. Ein Exporter schreibt die Punkte, die das Original zeigt. Eine Implementierung darf einen Hinweis ausgeben, wenn drei oder mehr Leitungen an einem gemeinsamen Punkt ohne Knotenpunkt zusammentreffen; dieser Standard verlangt dies nicht.
- **Vom Solver hinzugefügte Knotenpunkte**: die Ausgabe des Solvers ist ebenfalls RNL und darf Punkte enthalten. Die verwendete Definition gibt der Schlüssel `junction` von SOLVER-CONFIG an (14.10), ein als Zeichenkette geschriebener `$`-Name wie `junction="$JUNCTION"`, oder im SOLVER-CONFIG eines Ebeneneintrags ein `%`-Name jener Ebene, wie `junction="%BIG_DOT"` für einen allein für diese Teilschaltung definierten größeren Punkt; das Standardpaket setzt `$JUNCTION` für die Ebenen, die es verwenden (16.3). Mit diesem Schlüssel wendet der Solver jene Definition auf jeden Verzweigungspunkt der Bäume von ROUTE-NET und ROUTE-OBJECT an (13.7), und wo vom Benutzer geschriebene Leitungen zu dritt oder mehr an einem gemeinsamen Punkt ohne eine Anwendung dieser Definition zusammentreffen, darf er in seiner Ausgabe ebenfalls eine hinzufügen. Ohne den Schlüssel schreibt er keine Punkte, und `junction=""` schaltet sie aus, wo ein Paket ihn gesetzt hat. Der Name wird nachgeschlagen wie ein im Eintrag des Solvers in dieser Ebene geschriebener Name (6.8); jeder andere Wert, der dort keine Definition benennt oder eine, deren erste Rolle keinen Punkt annimmt, ist ein Wert, der in dieser Ebene nicht angenommen wird (14.10): ein Fehler, und der Schlüssel gilt dort als nicht geschrieben, sodass der Wert von vorher gilt oder, wenn es keinen gibt, der Solver keine Punkte schreibt. Ein gemeinsamer Punkt ist stets ein benannter Punkt oder ein Anschluss (gleiche Koordinaten zählen nicht), sodass der Punkt geschrieben werden kann. Andere Werkzeuge, die Knotenpunkte hinzufügen, etwa ein Editor, entnehmen die Definition demselben Schlüssel.
- Ein Knotenpunkt ist nur Zeichnung: er stellt keine Verbindung her und wird nicht geprüft (Prinzipien 2.2). Ein Punkt mit `$JUNCTION` und ohne Leitung erhält dennoch seinen Punkt.

### 13.3 Text: TEXT

```rnl use=w13
(50, 18) @TEXT("5T OTA testbench");                         /* an annotation: attached to a point */
(1, 1) @TEXT("see here") => @PLACE(2, 2);                   /* attached to (1, 1); the text itself is at (2, 2) */
#M1 @TEXT("W/L=${w}/${l}", key="wl") => @PLACE(12, 18);     /* a label: attached to M1, with the key wl, so it is #M1::wl */
#R1 @TEXT("R_{D}", key="name");                             /* a key the symbol has: new content, the position still from the symbol */
#M1::name @PLACE(13, 18);                                   /* the name of M1, dragged in an editor */
%w13 @TEXT("${net}");                                       /* the net name on a wire; no key; placed by the implementation */
&out @TEXT("${name}");                                      /* shows the name of the net out; placed by the implementation */
# @SELECT-INSTANCE => @SELECT-PIN(wired=false) => @TEXT("${net}", key="netname")    /* net names on pins without wires */
```

- **Leerer Text**: ein TEXT, dessen Inhalt die leere Zeichenkette ist, zeichnet nichts. Mit einem Schlüssel ersetzt er die Beschriftung des Symbols mit diesem Schlüssel und blendet sie so aus. Das Standardpaket `rnl-std-no-labels` blendet Standardbeschriftungen stattdessen mit dem Stil `textvisible=false` aus (9.3, 16.3), wodurch ihr Inhalt erhalten bleibt, sodass ein TEXT mit demselben Schlüssel und `textvisible=true` eine wieder anzeigt.
- **Anhängen**: TEXT nimmt einen Punkt oder ein gezeichnetes Objekt an, und der Text wird daran angehängt. Woran er angehängt ist, entscheidet dreierlei: wessen Attribute `${…}` liest (13.6); wohin der Text kommt, wenn seine Position nicht angegeben ist; und, wenn er einen Schlüssel hat, welches Objekts Unterobjekt der Text ist.
  - Ein Punkt (`(x, y)`, ein benannter Punkt, ein `::position`): der Text ist eine Anmerkung (13.4). Er kann nur auf die Attribute `x` und `y` des Punkts verweisen.
  - Ein Objekt: der Text ist eine Beschriftung. Das Objekt kann eine Instanz, eine gezeichnete Anwendung, ein Bündel, ein Pin, ein Anschluss (an seinem Pin, 5.2.4), eine Leitung oder eine Grafik sein. Es kann auch ein Netz sein: ein Netz selbst wird nicht gezeichnet, und die Implementierung setzt einen an ein Netz angehängten Text neben eine seiner Leitungen oder einen seiner Anschlüsse. So gibt eine Datei an, welche Netze ihre Namen anzeigen.
  - Eine Anwendung ohne Symbol oder ein Anschluss ohne Pin hat keine Bedeutung fürs Zeichnen: erhält TEXT eines davon, schlägt es fehl (12.1, 9.2.4).
  - Erhält TEXT eine Menge, hängt es an jedes Mitglied einen Text an (5.2.1), wie in der letzten Zeile des Beispiels.
  - In einem Symbolrumpf ist ein an einen Punkt angehängter Text fester Text des Symbols; ein an ein alleinstehendes `?` angehängter Text ist an das mit dem Symbol gezeichnete Objekt angehängt (siehe unten).
- **Inhalt**: `text` ist erforderlich, auch wenn ein Schlüssel angegeben ist; ohne ihn schlägt das TEXT fehl. Er darf nach Position als Erstes angegeben werden, wie in `@TEXT("${name}", key="name")`. Es gibt keinen Standardinhalt: um den Namen anzuzeigen, schreibt man `"${name}"`. Auszeichnung und Attributverweise stehen in 13.6.
- **Position**: ein Text hat eine eigene Position, die nicht der Punkt oder das Objekt ist, an das er angehängt ist.
  - Die Position eines Texts beginnt frei, wie `(_, _)`. Wenn kein PLACE, `dx`, `dy` oder eine Bedingung sie festlegt, entscheidet der Solver und schreibt die Position zurück (14.8); ohne Lösung entscheidet der Viewer: ein an einen Punkt angehängter Text kommt üblicherweise an diesen Punkt, und ein an ein Objekt angehängter Text neben das Objekt; beides darf verschoben werden, um Überlappungen zu vermeiden.
  - PLACE setzt die Position des Texts selbst. Daher ist `(1, 1) @TEXT(…) => @PLACE(2, 2)` kein Widerspruch: der erste Teil hängt den Text an (1, 1) an, der zweite setzt den Text auf (2, 2). Um eine Anmerkung exakt irgendwohin zu setzen, schreibt man PLACE.
  - `dx`, `dy`: Versätze, die nur TEXT annimmt. Wenn das, woran der Text hängt, einen Koordinatenpunkt hat (ein Punkt, der Ursprung einer Instanz, der Punkt, an dem eine Anwendung gezeichnet wird, der Pin eines Anschlusses), liegt der Text an diesem Punkt plus (dx, dy). Leitungen und Netze haben keinen Koordinatenpunkt: ein Versatz bei einem an eines von ihnen angehängten Text lässt das TEXT fehlschlagen.
    - Der Text hat weiterhin eine einzige Position, jene mit dem Versatz; Bedingungen und der Solver verwenden sie, und es gibt keine Position „vor dem Versatz“. Wird das angehängte Objekt verschoben oder vom Solver platziert, bewegt sich der Text mit; verschieben andere Bedingungen den Text, bewegt sich das angehängte Objekt mit. Ein Versatz ist dasselbe wie ein exaktes DISTANCE für x und eines für y, geschrieben auf dem Text: ein positiver Versatz in steigender Richtung (`"left-to-right"`, `"bottom-to-top"`), ein negativer in fallender Richtung, mit seinem Betrag als `exact` (14.5).
    - Versätze verlaufen entlang der Achsen der Szene und drehen sich nicht mit der Ausrichtung des angehängten Objekts, da sich Texte in der Szene nicht mit Objekten drehen. In einem Symbolrumpf dürfen sie ebenfalls geschrieben werden; dort stehen sie in lokalen Koordinaten und transformieren sich mit der Instanz, wie eine mit `=> @PLACE(…)` geschriebene lokale Position.
    - **Versätze und PLACE ersetzen einander**: auf jeder Achse ist die Position eines Texts entweder absolut (das x oder y von PLACE) oder ein Versatz (`dx` oder `dy`), niemals beides. Auf derselben Achse desselben Texts wird die später geschriebene Art (nach der Position aus 8.5) wirksam, und die früher geschriebene Art hat überhaupt keine Wirkung: nach einem Versatz ist die absolute Position auf dieser Achse frei und folgt aus dem Versatz; nach einem PLACE gilt der Versatz auf dieser Achse nicht mehr. Dies ist eine Ersetzung, kein Konflikt: die Priorität spielt keine Rolle, und es wird kein Hinweis ausgegeben. Nach `dx` und `dy` macht etwa ein späteres `#M1::name @PLACE(x=13)` x zum absoluten Wert 13, `dx` gilt nicht mehr, und y folgt weiterhin `dy`; ein Editor schreibt dies, wenn ein per Versatz platzierter Text gezogen wird. Eine im Symbol angegebene Position kommt zuerst (siehe unten), und ein PLACE oder ein Versatz in der Szene ersetzt sie.
    - Was jeweils wirksam wird, ist ein gewöhnlicher geometrischer Fakt: das x und das y eines Versatzes sind jeweils eine Einheit, ihre Position ist die der TEXT-Anweisung, die sie schreibt, und ihre Priorität ist der Standardwert des Eintragskopfs (14.8); sie werden mit anderen Fakten gemäß 8.5 aufgelöst. Mehrere PLACEs auf derselben Achse werden untereinander gemäß 8.5 aufgelöst. Bei einem Text mit Schlüssel werden `dx` und `dy` getrennt überschrieben (8.4).
  - Ein Text kann über seine eigene Position Mitglied von Bedingungen sein (14.1). Das `::position` eines Texts ist seine eigene Position: `%n1::position @PLACE(…)` ist dasselbe wie `%n1 @PLACE(…)`.
  - Texte in der Szene drehen sich nicht mit der Ausrichtung dessen, woran sie hängen; um einen zu drehen, schreibt man `rotate=`. Liefert die Szene beide Achsen der Position eines Texts durch PLACE oder Versätze, verwendet keine Achse die lokale Position des Symbols: der Text erbt die Ausrichtung des angehängten Objekts nicht, und `halign` und `valign` werden in den Richtungen der Szene gedeutet.
  - Der Solver schreibt die von ihm gewählte Position zurück (14.8): über den Schlüssel (unten) oder den Namen des Texts, oder andernfalls über seine Nummer (5.9). Auf einen Text ohne Schlüssel, den eine Generierungsregel direkt erzeugt (13.3, 10.5), kann nicht verwiesen werden: der Solver überlässt seine Position dem Viewer. Auf einen Text im Rumpf einer von einer Regel erzeugten Anwendung wird über seine Nummer verwiesen, sobald der Solver die Anwendung als explizite Anweisung ausschreibt (10.5, 5.9).
- **Schlüssel `key=`**:
  - Ohne Schlüssel ist der Text ein Text des angehängten Objekts ohne Schlüssel; wird er erneut geschrieben, ist er ein weiterer Text, wie bei Leitungen (13.1). Er kann mit `%x =` benannt und in seiner Ebene referenziert und geändert werden. Texte, die Regeln ohne Schlüssel erzeugen, haben kein Handle, sodass der Solver ihre Positionen nicht zurückschreiben kann.
  - Mit einem Schlüssel ist der Text das Unterobjekt `object::key` des angehängten Objekts, etwa `#M1::wl`. Dasselbe Objekt mit demselben Schlüssel ist derselbe Text, und spätere Anweisungen überschreiben ihn Parameter für Parameter: Größe, Ausrichtung, Drehung usw. werden jeweils für sich überschrieben, und nicht geschriebene Parameter bleiben wie sie waren (8.4). Der Inhalt wird in jeder Anweisung geschrieben (`text` ist erforderlich), und der spätere Inhalt ersetzt stets den früheren; um nur die Ausrichtung oder die Drehung zu ändern, schreibt man den Inhalt erneut, wie in `#M1 @TEXT("${name}", key="name", halign="end")`. Die Position folgt den Regeln von PLACE (8.5). Andere Einträge, Editoren und der Solver finden den Text über dieses Handle.
  - Ein an einen Punkt angehängter Text nimmt kein `key=` an, da ein Punkt keine Unterobjekte hat: mit einem Schlüssel schlägt das TEXT fehl.
  - Die Namen nach `::` werden in fester Reihenfolge nachgeschlagen, und Schlüssel stehen zuletzt (6.5): die Rollennamen und lokalen Namen einer Anwendung, dann deren `definition=`-Aliase, dann die Standardunterobjektnamen und ihre Aliase, dann die Namen der Pins des Symbols des Objekts, dann die Schlüssel von Texten. Daher darf ein Schlüssel nicht genauso geschrieben sein wie irgendein Name, der davor kommt: ein Standardunterobjektname (`position`, `net`) oder ein für die Datei deklarierter Alias eines solchen (wie `位置` aus `alias-zh`); ein Rollenname oder lokaler Name der Anwendung, an die der Text angehängt ist; oder ein `definition=`-Alias eines solchen. Ein solcher Schlüssel ist ein Fehler, wie die Namenskollisionen aus 6.5. Bei einem in einem Symbol geschriebenen Text sind die Aliase jene der Datei, in der das Symbol geschrieben steht. `name` und `value` sind keine Standardunterobjektnamen, sondern nur übliche Schlüssel.
- **Texte in Symbolen**:
  - In einem Symbolrumpf ist ein alleinstehendes `?` das mit dem Symbol gezeichnete Objekt: M1, wenn M1 gezeichnet wird, `%g0`, wenn die Anwendung `%g0` gezeichnet wird. Es kann nur das Objekt von TEXT sein; überall sonst schlägt es fehl, etwa `? @PLACE(…)` in einem Symbol, was keine Bedeutung hat.
  - `? @TEXT("${name}", key="name") => @PLACE(1.5, 1.5)`: der Text wird an das mit dem Symbol gezeichnete Objekt angehängt, und `${…}` liest dessen Attribute. Dieses PLACE steht in lokalen Koordinaten und transformiert sich mit der Instanz (siehe die Textrichtung in 9.2.5). Ohne PLACE setzt die Implementierung den Text neben das Objekt.
  - Ein Text mit Schlüssel ist ein Unterobjekt des mit dem Symbol gezeichneten Objekts, etwa `#M1::name`. Die Texte im Symbol kommen zuerst, und ein Text mit demselben Schlüssel in der Szene überschreibt ihn Parameter für Parameter: `#R1 @TEXT("R_{D}", key="name")` ändert nur den Inhalt und behält die Position aus dem Symbol; `#M1::name @PLACE(13, 18)` ändert nur die Position und behält den Inhalt aus dem Symbol. Eine in der Szene angegebene Position steht in Szenenkoordinaten und dreht sich nicht mehr mit der Instanz.
  - Die Schlüssel, die ein Symbol mitbringt, existieren erst, sobald Symbole zugewiesen sind (8.2 Schritte 4 und 7). Hat das Symbol von M1 keinen Text mit dem Schlüssel `name` und die Szene schreibt keinen, schlägt `#M1::name` fehl. Um vom Symbol unabhängig zu sein, schreibt man ein TEXT mit diesem Schlüssel in der Szene.
  - Ein an einen Punkt im Symbol angehängter Text ist fester Text, wie `(-2, 2) @TEXT("+")` in einem Operationsverstärker-Symbol, und kann nicht für eine einzelne Instanz geändert werden.
- **Regeln**: eine Regel wie `# @SELECT-INSTANCE => @SELECT-PIN(wired=false) => @TEXT(…)` wählt Pins aus, die aus den Symbolen stammen, sodass sie in Schritt 6 mit der schrittweisen Generierung ausgewertet wird (8.2). Fügt eine Implementierung selbst Luftlinien oder Beschriftungen hinzu, überspringt sie Pins, die bereits eine Beschriftung haben.
- Welche Netznamen wo angezeigt werden und ob Verbindungen ohne Leitungen mit Beschriftungen oder Luftlinien gezeigt werden, entscheidet standardmäßig die Implementierung: dies gehört zu ihrem Hausstil (Prinzipien 2.7). Beschriftungen statt Leitungen zu zeigen verändert das Layout, sodass Solver und Renderer dieselbe Einstellung verwenden. RNL schreibt dies nur bei Bedarf: um eine Absicht auszudrücken (ohne Positionen) oder um bereits entschiedene Positionen festzuhalten.
- Es gibt keine Positionen entlang einer Leitung: die Koordinaten einer Leitung sind selbst nicht festgelegt.
- **`fix_rotate`**: ein Wahrheitswert, standardmäßig `false`. Mit `true` wird das `rotate` des Texts in der Szene genommen: die Zeichen behalten diese Richtung bei, unabhängig von der Ausrichtung des Objekts, an dem der Text hängt. Sein Anker folgt weiterhin dem Objekt, und ebenso die Seite des Ankers, auf der das Textfeld liegt. Eine Ausrichtung gibt an, wo das Feld vom Anker aus liegt: rechts bei `start`, links bei `end`, darüber bei `bottom` und `baseline`, darunter bei `top`, und zu keiner Seite bei `middle`. Diese Richtung wird mit der Ausrichtung des Objekts transformiert, und der Text wird gemäß dem Ergebnis ausgerichtet. So liegt ein über einem Symbol mit `halign="middle"` und `valign="bottom"` geschriebener Name links von seinem Anker, ausgerichtet nach `end` und `middle`, wenn das Symbol um `"R90"` gedreht wird, und darunter, ausgerichtet nach `top`, bei `"MX"`; er liest sich in jeder Ausrichtung waagerecht und bleibt außerhalb des Symbols. Ein Feld, das über seinem Anker bleibt, behält das `bottom` oder `baseline`, mit dem es geschrieben wurde.
- **`upright`**: ein Wahrheitswert, standardmäßig `true`: der Text wird um 180 Grad gedreht, wenn er sich sonst auf dem Kopf lesen würde, und seine Glyphen werden nicht gespiegelt. Mit `false` folgen seine Zeichen der Drehung und Spiegelung unverändert (9.2.5).
- Parameter: `text` (erforderlich; darf nach Position angegeben werden), `key`, `dx`, `dy`, `halign`, `valign`, `rotate`, `fix_rotate`, `upright` sowie die Textstileigenschaften `textcolor`, `size`, `font`, `bold`, `italic`, `textvisible` (9.3); ein an einen Punkt angehängter Text hat kein `key`. Zu `halign`, `valign` und `rotate` siehe 9.2.5. Die Stilparameter sind der direkte Stil des Texts (8.3); wie ein Text seinen Stil erhält, steht in 11.3.

### 13.4 Anmerkungen: Grafiken in der Szene

```rnl
(50, 18) @TEXT("5T OTA testbench") => @SET-STYLE $note => %n1;
%f1 = @RECTANGLE {(0, 0) (40, 30)};
%n1 @PLACE(60, 18)                            /* moves the text; may also be written in another record */
```

- LINE, RECTANGLE, CIRCLE, ELLIPSE, ARC, BEZIER und TEXT können direkt in der Szene geschrieben werden, und ebenso im Rumpf einer allgemeinen Definition. In einem Symbolrumpf sind ihre Koordinaten lokal; in der Szene sind es Szenenkoordinaten. Wo sie geschrieben stehen, entscheidet dies; das Schlüsselwort ist dasselbe.
- Die Notizobjekte aus Prinzipien 2.2 (Freiformtext, Grafiken, Rahmen und andere Objekte, die nur gezeichnet werden, ohne Anschlüsse und ohne elektrische Bedeutung) sind die Anmerkungen dieses Standards: in der Szene geschriebene Grafiken, an Punkte angehängte TEXTe und die von ANNOTATION gezeichneten Symbole (13.5).
- PIN darf ausschließlich im Rumpf von DEFINE-SYMBOL geschrieben werden: ein Pin bindet einen Anschluss des mit dem Symbol gezeichneten Objekts, und die Szene hat kein solches Objekt.
- Ein an einen Punkt angehängter TEXT ist eine Anmerkung (13.3): er kann nur auf die Attribute `x` und `y` des Punkts verweisen, und er nimmt kein `key=` an, da ein Punkt keine Unterobjekte hat (mit einem Schlüssel schlägt das TEXT fehl). Um den Namen oder Wert eines Objekts anzuzeigen, hängt man den TEXT an das Objekt an, was ihn zu einer Beschriftung macht.
- Die Position einer Grafik ist durch ihre Punkte gegeben. Ein Punkt kann ein beliebiges Punktobjekt sein: ein benannter Punkt, ein freier Punkt mit Bedingungen oder `#M1::position`, das dem Bauteil folgt. Im Rumpf einer allgemeinen Definition ist ein literaler Punkt ein Versatz vom eigenen Punkt der Anwendung (9.4). PLACE nimmt keine Grafik selbst an: um aus einem anderen Eintrag ein mit Mittelpunkt geschriebenes RECTANGLE, CIRCLE oder ELLIPSE zu verschieben, verschiebt man sein `::position`; LINE, ARC, BEZIER und die mit zwei Punkten geschriebenen Grafiken haben mehrere Punkte, daher benennt man die Punkte und verschiebt sie. TEXT ist die Ausnahme: er hat eine eigene Position, der Punkt, an dem er hängt, gibt nur seinen Standardort an, und PLACE setzt die Position des Texts direkt (13.3).
- Aussehen: Grafiken können Stilparameter direkt annehmen (9.2.5) und nehmen auch SET-STYLE an, mit den beiden Stufen aus 8.3; eine Eigenschaft, die von keinem vorgegeben ist, erhält die Vorgabe des Renderers.
- Ein Rahmen ist ein RECTANGLE oder ein LINE, dessen letzter Punkt der erste ist. Es gibt keine Grafik, die eine Gruppe von Objekten umschließt: man zeichnet ein RECTANGLE und erzwingt, dass seine Ecken außerhalb der Gruppe liegen, etwa `%tl %stage1_set @DISTANCE("left-to-right", min=1)`, was die Ecke mindestens 1 links von jedem Mitglied hält (14.5); umschlossen werden die Punkte der Mitglieder zuzüglich des Rands. Ein Viewer darf ein GROUP als Rahmen zeichnen; das liegt beim Viewer (11.5). Es gibt kein `::box`.
- Die Zeichenreihenfolge folgt Zeichenebenen und Positionen (11.6).
- **Klasse**: ob etwas eine Anmerkung ist, folgt aus seiner Objektart: Grafiken in der Szene sind Anmerkungen, sodass ein Renderer sie als Klasse ein- oder ausblenden kann. SELECT-ANNOTATION wählt alle Grafiken in der Szene und das von ANNOTATION Gezeichnete aus (13.5), einschließlich jener, die aus den Rümpfen allgemeiner Definitionen expandiert wurden (etwa von einem Hausstilpaket gezeichnete Rahmen); Grafiken in Symbolen, Beschriftungen, Leitungen und Anwendungen (einschließlich Knotenpunkten) sind keine Anmerkungen. Feinere Klassen (Erläuterungen, Schriftfelder (title blocks), von einem Paket gezeichnete Verzierungen) sind benannte Mengen, und eine Grafik kann in mehreren Mengen stehen. Ein- und Ausblenden liegt beim Viewer; die Sprache hat kein `visible`.
- Ein Text auf einem Bauteil ist ein an das Bauteil angehängter TEXT, also eine Beschriftung (13.3). Ein an `#M1::position` angehängter TEXT ist eine Anmerkung; standardmäßig folgt er ebenfalls dem Bauteil, kann aber nur auf die Koordinaten jenes Punkts verweisen und hat keinen Schlüssel.

### 13.5 Als Anmerkungen gezeichnete Symbole: ANNOTATION

```rnl
%a1 = (40, 20) @ANNOTATION(rotate=30) $ARROW_SYMBOL;     /* an arrow at any angle */
#M1 @ANNOTATION(rotate=-45, mirror=true) $ARROW_SYMBOL;   /* follows M1 */
%a1::position @PLACE(44, 20)                             /* moves the first one */
```

- ANNOTATION sammelt die Grafiken dessen ein, was es erhält, und zeichnet sie als eine Anmerkung (13.4), um einen beliebigen Winkel gedreht. Zwei Slots: `at`, zuerst geschrieben, ein lokalisierbares Objekt: ein Punkt oder ein Objekt, dessen Position die Anmerkung folgt; und `source`, zuletzt geschrieben, eines von:
  - einem Symbol (DEFINE-SYMBOL): die Grafiken und die TEXTe seines Rumpfes;
  - einer allgemeinen Definition (DEFINE), mit oder ohne Rollen: die direkt in ihrem Rumpf geschriebenen Grafiken, deren sämtliche Punkte literale Punkte sind, die Versätze vom Punkt einer Anwendung sind (9.4), sowie die an solche Punkte oder an `?` angehängten TEXTe. Die Definition wird nicht angewendet: es wird keine Anwendung erzeugt, sodass nichts doppelt gezeichnet wird;
  - einer Anwendung: die Grafiken ihres Symbols, wenn sie eines hat, sobald Symbole zugewiesen sind (8.2), sowie die Grafiken, die ihr Rumpf nur mit literalen Punkten gezeichnet hat, wie bei einer Definition, um den Punkt der Anwendung herum. Die Anwendung selbst bleibt dort gezeichnet, wo sie ist.
- Alles Übrige wird weggelassen: PINs, `box=`, Bedingungen, die in einem Rumpf erzeugten Anwendungen und die Grafiken eines Rumpfes, die einen Punkt auf einem anderen Objekt haben, etwa eine zu einer Rolle gezeichnete Linie, die nach dem Verschieben und Drehen keine Bedeutung mehr hätte. Eine aus diesem Grund weggelassene Grafik liefert einen Hinweis; der Rest wird ohne Hinweis weggelassen, sodass ein Symbol mit Pins für ein Zeichen wiederverwendet werden kann. Nichts füllt eine Rolle; die Anmerkung hat keine Pins, kann kein Ende einer Leitung sein und liegt in keinem Baum von 13.7.
- **Parameter**: `rotate`, ein Winkel in Grad, gegen den Uhrzeigersinn, eine beliebige Zahl, standardmäßig `0`; `mirror`, ein Wahrheitswert, standardmäßig `false`, der das Symbol vor dem Drehen an seiner senkrechten Achse spiegelt; und die Stileigenschaften, die an seine Grafiken und Texte weitergegeben werden wie bei SET-STYLE (11.3). Es nimmt außerdem nach Namen die Parameter eines als `source` angegebenen Symbols oder einer allgemeinen Definition an, wie AS es tut (9.4), etwa `(0, 0) @ANNOTATION(rotate=30, w=5) $D`: ein nicht angegebener Parameter nimmt seinen Standardwert, und einer ohne Standardwert zählt als nicht geschrieben (9.4). Trägt ein Parameter der Quelle den Namen eines Parameters von ANNOTATION selbst, wie `rotate` oder `mirror`, ist der Parameter von ANNOTATION gemeint, und ein Hinweis wird ausgegeben. Eine als `source` angegebene Anwendung behält die Werte ihrer eigenen Parameter, und ein an ANNOTATION geschriebener Parameter ihrer Definition ist dann ein ungültiger Parameter. Die Texte des Symbols werden mit ihm gedreht wie die Texte eines Symbols mit einer Instanz, gemäß ihrem eigenen `rotate`, `fix_rotate` und `upright` (9.2.5).
- In den Texten des Symbols ist `?` die Anmerkung, die wie ein Punkt nur die Attribute `x` und `y` hat (13.4); ein anderer Attributverweis dort wird als leerer Text gezeichnet.
- **Ergebnis**: eine neue Anmerkung mit dem Unterobjekt `::position`, ihrem Anker. Erhält sie einen geschriebenen Punkt, beginnt der Anker dort und kann über sein `::position` verschoben werden, wie eine mit Mittelpunkt geschriebene Grafik (13.4); erhält sie ein Objekt, folgt er der Position des Objekts. SELECT-ANNOTATION wählt sie aus, SET-STYLE gestaltet sie, und `textvisible` blendet ihre Texte aus.
- Was es ist, ist dort bekannt, wo es geschrieben steht: alles andere als ein Symbol, eine allgemeine Definition oder eine Anwendung im Slot `source` ist ein Fehler; eine Quelle ohne zu zeichnende Grafik liefert eine Warnung und zeichnet nichts.
- **Im Vergleich zum Übrigen**: die in der Szene geschriebenen Grafiken (13.4) haben jeweils ihre eigenen Punkte; eine Anwendung mit Symbol, wie eine Figur des Standardpakets, wird in den acht Ausrichtungen aus 11.2 gezeichnet, kann Pins haben und kann verdrahtet werden. ANNOTATION zeichnet ein Symbol unter beliebigem Winkel und hat keine elektrische Bedeutung.

### 13.6 Textauszeichnung und Attributverweise

```rnl
#R1 @TEXT("R_{D}", key="name");                              /* subscript */
(50, 18) @TEXT("\textbf{5T OTA} testbench\\VDD = 1.8 V");    /* bold, two lines */
#M1 @TEXT("W/L=${w}/${l}", key="wl");                        /* host parameters */
$NMOS_M = @DEFINE-SYMBOL {
  $NMOS;
  ? @TEXT("${model}", key="model") => @PLACE(1.5, -1.5)      /* the model name, in a symbol */
}
```

- **Auszeichnung** ist eine Teilmenge des Textmodus von LaTeX, kein vollständiges TeX:

  | Auszeichnung | Bedeutung |
  |---|---|
  | `_{…}`, `^{…}` | Tiefstellung, Hochstellung |
  | `\textbf{…}`, `\textit{…}` | Fett, kursiv |
  | `\overline{…}` | Überstrich, etwa das aktiv-niedrige `\overline{RST}` |
  | `\subrest{…}` | Das erste Zeichen wie geschrieben, der Rest tiefgestellt: `\subrest{${name}}` zeigt M1 als M₁ und VDD als V mit tiefgestelltem DD |
  | `\subdigits{…}` | Die Ziffern am Ende tiefgestellt: R12 als R₁₂, Vout1 als Vout₁; Inhalt ohne Ziffern am Ende wird unverändert angezeigt |
  | `\(…\)` | Mathematikmodus (unten) |
  | `\frac{…}{…}` | Bruch: Zähler oben, Nenner unten, ein Bruchstrich dazwischen, etwa `\frac{${w}}{${l}}`; ein Viewer, der sie nicht stapeln kann, zeigt „Zähler/Nenner“ |
  | `\alpha` … `\omega`, `\Gamma` … `\Omega` | Griechische Buchstaben mit den Namen aus LaTeX |
  | `\pm`, `\times`, `\cdot`, `\infty`, `\approx`, `\neq`, `\leq`, `\geq`, `\ll`, `\gg`, `\propto`, `\parallel`, `\angle`, `\circ`, `\to`, `\uparrow`, `\downarrow` | Gebräuchliche Symbole |
  | `\\` | Zeilenumbruch |
  | `\_`, `\{`, `\}`, `\$`, `\%`, `\&`, `\#`, `\textbackslash` | Die Zeichen `_`, `{`, `}`, `$`, `%`, `&`, `#`, `\` selbst |

- Griechische Buchstaben: kleingeschrieben `\alpha` bis `\omega` (LaTeX hat kein `\omicron`; man schreibt o) und die Varianten `\varepsilon`, `\vartheta`, `\varpi`, `\varrho`, `\varsigma`, `\varphi`; großgeschrieben nur die Buchstaben, die sich von lateinischen unterscheiden: `\Gamma`, `\Delta`, `\Theta`, `\Lambda`, `\Xi`, `\Pi`, `\Sigma`, `\Upsilon`, `\Phi`, `\Psi`, `\Omega`. Ein Symbol, das nicht in der Tabelle steht, wird direkt als Unicode-Zeichen geschrieben (Zeichenketten dürfen Nicht-ASCII-Zeichen enthalten, 3.7).
- Auszeichnungen dürfen geschachtelt werden, etwa `\textit{V}_{\textit{DD}}` und `I_{D_{1}}`. Wie die Skripte von `V_{in}^{+}` gesetzt werden (übereinander oder nacheinander), liegt beim Viewer.
- **Tief- und Hochstellungen benötigen geschweifte Klammern**: ein alleinstehendes `_` oder `^` ist ein gewöhnliches Zeichen. Dies unterscheidet sich von LaTeX, wo das m in `g_m` tiefgestellt ist: Namen und Beschreibungen enthalten oft `vin_1` oder `bias_ctrl`, was jene Regel verderben würde. Dafür wird `g_m` so angezeigt, wie es geschrieben ist.
- Andere Zeichen werden so angezeigt, wie sie geschrieben sind, einschließlich eines alleinstehenden `{`, `}`, `$`, `~`, `%`, `&` oder `#`, die in LaTeX andere Bedeutungen haben.
- **Backslash**: `\\` ist ein Zeilenumbruch wie in LaTeX; ein Backslash selbst wird `\textbackslash` geschrieben. Ein Backslash, dem kein bekannter Befehl folgt, wird wie geschrieben angezeigt und liefert eine Warnung, etwa `C:\temp` (`\temp` ist ein unbekannter Befehl; siehe den nächsten Punkt); um die Warnung zu vermeiden, schreibt man `C:\textbackslash{}temp`. `\textbackslash` wird nur dort benötigt, wo auf den Backslash unmittelbar `\`, `{`, `}`, `_`, `^`, `$`, `%`, `&`, `#` oder der Name eines bekannten Befehls folgt. RNL-Zeichenketten haben keine Backslash-Maskierungen (3.7): `"\textbf{V}"` wird genau so geschrieben, nicht als `\\textbf`.
- Ein unbekannter Befehl (`\` gefolgt von Buchstaben, die nicht in der Tabelle stehen, wie `\sqrt` oder `\alpah`) und unausgeglichene geschweifte Klammern werden wie geschrieben angezeigt und liefern eine Warnung; der Text bleibt gültig.
- **Nach einem Befehlsnamen**: ein Befehlsname ist die Folge von Buchstaben direkt nach `\`. Leerzeichen nach dem Namen eines bekannten Befehls werden verworfen und nicht angezeigt, und ein leeres `{}` direkt danach trennt nur und wird ebenfalls nicht angezeigt. So zeigen `10 \mu A` und `10 \mu{}A` beide 10 μA; um das Leerzeichen zu behalten, schreibt man `\mu{} A`; `\textbackslash{}temp` zeigt \temp. Leerzeichen nach den maskierten Zeichen wie `\_` und `\{` bleiben erhalten.

  Hinweis: Unterstützung für mehr von TeX in einer späteren Version erlaubt lediglich mehr. Der Mathematikmodus wird `\(…\)` geschrieben, nicht `$…$`, da `$` von Attributverweisen verwendet wird.
- **Formate von Namen**: `\subrest{…}` und `\subdigits{…}` ändern nur, wie ihr Inhalt angezeigt wird: die angezeigten Zeichen werden in einen normalen Teil und eine Tiefstellung aufgeteilt. Der Inhalt darf referenziert sein, etwa `\subrest{${name}}`; er wird weiterhin nicht als Auszeichnung gelesen (unten). Sie dürfen in andere Auszeichnungen geschachtelt werden, etwa `\textit{\subrest{${name}}}`.
- **Mathematikmodus**: `\(…\)` enthält eine Formel in einer Teilmenge des Mathematikmodus von LaTeX:
  - Buchstaben sind Variablen und werden kursiv gesetzt; Ziffern und die Symbole der Tabelle stehen aufrecht, und Leerzeichen werden nicht angezeigt;
  - `_` und `^` nehmen ein Zeichen oder eine Gruppe in geschweiften Klammern an, wie in LaTeX, sodass `g_m` innerhalb `\(…\)` ein g mit tiefgestelltem m ist;
  - `\frac{…}{…}`, `\sqrt{…}`, die griechischen Buchstaben und gebräuchlichen Symbole der Tabelle sowie die Relationen `=`, `<`, `>`;
  - `\mathrm{…}` setzt seinen Inhalt aufrecht, wie für Einheiten und Namen, etwa `\(V_\mathrm{DD}\)`;
  - ein Attributverweis darin wird wie geschrieben aufrecht angezeigt.

  Ein Befehl außerhalb dieser Teilmenge wird wie geschrieben angezeigt und liefert eine Warnung.
- **Brüche**: `\frac` nimmt zwei Gruppen in geschweiften Klammern direkt danach an; fehlt eine, ist es unbekannte Auszeichnung, die mit einer Warnung wie geschrieben angezeigt wird. Zähler und Nenner dürfen Auszeichnung und `${…}` enthalten.
- Wie weit Tief- und Hochstellungen sowie Brüche nach oben oder unten verschoben und wie stark sie verkleinert werden, liegt beim Viewer. Die Schriftart sowie Fett- und Kursivschrift für den gesamten Text stammen aus dem Stil oder aus den Parametern `font`, `bold`, `italic` (9.3).
- Auszeichnung wird im `text` von TEXT und im `label` von PIN gelesen (wie `\overline{CS}`). Andere Zeichenkettenparameter (`key`, Aufzählungswerte, Auswahlbedingungen) lesen keine Auszeichnung.
- Hinweis: Bei Import und Export sind `_{…}` und `^{…}` von KiCad dasselbe, und sein `~{…}` ist `\overline{…}`; ein Überstrich, den Altium mit `\` nach jedem Zeichen schreibt, wird als `\overline{…}` neu geschrieben.
- **Attributverweise** `${name}` zeigen ein Attribut des Objekts an, an das der Text angehängt ist. Um `${` selbst zu schreiben, schreibt man `\${`; alles von `\${` bis zu seiner passenden `}` wird wie geschrieben angezeigt, und diese `}` zählt nicht als unausgeglichene geschweifte Klammer. Die Attribute sind:
  - Instanzen: `name`; `value`, der auf der Host-Zeile geschriebene Wert wie geschrieben, leer, wenn keiner vorhanden ist (er fällt nicht auf den Modellnamen zurück); `model` und `type`, wie in Auswahlbedingungen (10.1), sodass `${model}` einer X-Instanz den Teilschaltungsnamen anzeigt; sowie die Host-Parameter, wie `w`. Ein Host-Parameter mit dem Namen eines Standardattributs wird `${#…}` geschrieben (siehe unten).
  - Anschlüsse, Pins und Leitungen: `net`, das Netz, auf dem sie liegen, angezeigt über seinen Namen; bei einem Pin das Netz von `::net` (6.5), und ein Pin, der nichts bindet, hat kein `net`. Auf dem leeren Knoten (6.3) ist `net` der leere Text, und ebenso eine damit gefüllte Rolle, wie `${p}`: es wird nichts gezeichnet (13.3). Das `net` einer Busleitung sind die Namen ihrer Netze der Reihe nach, verbunden mit `,` ohne Leerzeichen (13.1). Bei einem Anschluss eines Bündels ist es der Name des Netzes, wenn alle Mitglieder auf demselben Netz liegen, und andernfalls die Namen verbunden mit `,` (11.8).
  - Netze: `name`.
  - Anwendungen: die Rollen und die Parameter ihrer Definition. `${p}` ist das Objekt, das Rolle p füllt, angezeigt nach den folgenden Regeln. Eine Anwendung hat keinen eigenen Namen: die Symbole von Massen, Versorgungen und Ports schreiben `${p}`, was den Namen des Netzes anzeigt, wenn p ein Netz ist; eine mit einem Anschluss oder Pin gefüllte Rolle, wie t von `$GROUND_STUB`, zeigt ebenfalls den Namen seines Netzes an.
  - Bündel: ein Attribut, das bei allen Mitgliedern gleich ist, hat diesen Wert; andernfalls werden die Werte mit `,` verbunden (11.8).
  - Punkte: `x`, `y`, die endgültigen Koordinaten, also nach dem Lösen.
  - Grafiken: keine Attribute.
- **Attributpfade**: in `${a::b::c}` nimmt der erste Name ein Attribut des angehängten Objekts, und jedes folgende `::name` nimmt ein Attribut des Werts davor, wie `${p::x}` (das x der Rolle p, wenn sie ein Punkt ist) und `${pair::left::w}`.
  - Der Wert eines Attributs kann ein Objekt sein (eine Rolle einer Anwendung; das `net` eines Anschlusses oder einer Leitung) oder Text (`name`, `value`, ein Host-Parameter usw.). Text hat keine Attribute, daher darf ihm kein `::` folgen.
  - Ein Wert auf dem Pfad kann auch eine Menge sein, etwa eine Mengenrolle. Der Rest des Pfades wird dann von jedem ihrer Mitglieder genommen, eine Menge darunter von ihren eigenen Mitgliedern, in der Reihenfolge der Menge, und die Werte werden angezeigt wie eine Menge (unten); fehlt einem Mitglied das Attribut, schlägt der Text fehl. So zeigt bei einer Anwendung, deren Mengenrolle `pairs` zwei Differenzpaare hält, `${pairs::left::w}` das `w` für beide `left`-Transistoren, etwa `1u,2u`.
  - In `${…}` nimmt `::` ausschließlich Attribute, anders als `::` in Anweisungen, das Unterobjekte nimmt (6.5): `${m::name}` ist der Name der Instanz m, während `#M1::name` in einer Anweisung der Text mit dem Schlüssel `name` auf M1 ist (13.3).
  - `${m::#type}` sucht nur nach einem Host-Parameter, wie es `${#type}` tut (siehe unten).
- **Anzeige**: ist der letzte Wert des Pfades Text, wird er wie geschrieben angezeigt. Ist er ein Objekt: Instanzen und Netze zeigen ihre Namen; Anschlüsse, Pins und Leitungen zeigen den Namen ihres Netzes, und eine Busleitung die Namen ihrer Netze verbunden mit `,`; Bündel wie oben beschrieben; eine Menge (etwa eine Mengenrolle) zeigt ihre Mitglieder, jedes auf diese Weise, verbunden mit `,`. Punkte, Anwendungen und Grafiken können nicht angezeigt werden: der Text schlägt fehl. Wenn also die Rolle p ein Punkt ist, schreibt man `${p::x}`, nicht `${p}`.
- Verweise geben den Text wie in der Netzliste geschrieben wieder, ohne ihn auszuwerten: `w=1u` zeigt 1u. Was jeder Dialekt bereitstellt, definiert sein Host-Adapter; in Spectre beispielsweise ist der Wert eines Widerstands der Parameter `r`.
- Die oben aufgeführten Attribute existieren stets; eines ohne Wert ist leer. Ein Verweis auf einen nicht existierenden Attributnamen lässt den Text fehlschlagen.
- Der referenzierte Inhalt wird wie geschrieben angezeigt und nicht als Auszeichnung gelesen: Host-Namen und -Werte werden nicht in dieser Auszeichnung geschrieben.
- Verweise gehen nur von den Attributen des angehängten Objekts abwärts; sie können kein beliebiges Objekt erreichen. Es gibt keinen Verweis auf ein anderes Objekt wie `${R3:VALUE}` aus KiCad.
- **Aliase**: Attributnamen in `${…}` akzeptieren auch die für die Datei deklarierten Aliase, nachgeschlagen zuerst als Standardattribut, dann als Alias, dann als Host-Parameter; `${#name}` sucht ausschließlich nach einem Host-Parameter (16.5). Die Rollen- und Parameternamen einer Anwendung sind RNL-Namen: sie akzeptieren ausschließlich die `definition=`-Aliase ihrer Definition, bei jedem Schritt in der Definition der Anwendung jenes Schritts nachgeschlagen, etwa `${输入对::左管::w}` (16.5).
- **Host-Parameter mit Standardnamen**: ein Standardattribut geht vor, und ein gleichnamiger Host-Parameter wird `${#type}` geschrieben (`#` steht in RNL für den Host). Jeder Host-Parameter darf so geschrieben werden; `${w}` steht kurz für `${#w}`, wo kein Standardattribut und kein Alias diesen Namen trägt. Standardattributnamen werden genau wie geschrieben verglichen, Host-Parameternamen nach den Regeln des Hosts: in einem SPICE-Host ist `${Type}` kein Standardattribut, also der Host-Parameter type.
- **Formatieren von Namen**: referenzierter Inhalt wird nur als Ganzes mit Auszeichnung darum herum formatiert, etwa `\textit{${name}}`, sowie durch die beiden Befehle `\subrest{…}` und `\subdigits{…}`, die den Rest eines Namens oder seine Ziffern am Ende tiefgestellt anzeigen. Andere Formatierungen einzelner Zeichen werden als Text geschrieben, etwa `#M1 @TEXT("M_{1}", key="name")`, was einer Umbenennung nicht folgt; eine weitergehende Konvention eines Viewers ist dessen Hausstil.

### 13.7 Verbindungen als Leitungen zeichnen: ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET, UNROUTE-OBJECT

```rnl
&x @ROUTE-NET;                                /* x is drawn as wires; the solver decides the branches and paths */
%jo = (_, _);
&out @WIRE {#M2:1 %jo};
&out @ROUTE-NET;                              /* this wire is drawn as written; the solver adds the rest */
{#M1 #M2 #M5} @ROUTE-OBJECT;                  /* on each net that two or more of their pins share, those pins are joined */
&vdd @UNROUTE-NET(priority="high");           /* the pins of vdd are not joined, whatever asks for it at a lower priority */
# @SELECT-NET(name="clk*", recursive=true) => @ROUTE-NET   /* as a house-style package writes it, with nets chosen by a rule */
```

- Diese Schlüsselwörter geben an, welche Verbindungen als Leitungen gezeichnet werden, ohne anzugeben, wo sie abzweigen und wohin die Knotenpunkte kommen; das entscheidet der Solver.
- **ROUTE-NET** verbindet die Pins eines Netzes zu einem Baum: die Pins der Bauteile auf dem Netz und die Pins virtueller Bauteile, deren `::net` das Netz ist, also der Pins, die an mit ihm gefüllte Rollen gebunden sind (12.1). Ein Slot, der Netze annimmt; eine Menge wird Mitglied für Mitglied bearbeitet (5.2.1) und ist kein Bus.
- **ROUTE-OBJECT** verbindet die Pins einiger Objekte: für jedes Netz, auf dem zwei oder mehr ihrer Pins liegen, werden diese Pins zu einem Baum verbunden. Ein Slot, der eine Menge von Objekten annimmt; er reicht seine Eingabe weiter. Eine Instanz oder ein Bündel steht für seine Pins, ein Anschluss für seinen Pin und eine Anwendung für die Objekte, die ihre Rollen füllen, hinab bis zu den untersten Objekten, wie bei den Mitgliedern einer Gruppe (11.5); eine Anwendung mit Symbol steht ebenso für ihre eigenen Pins, jeden auf dem Netz der Rolle, die er bindet (12.1). Objekte ohne Pins, wie ein Netz oder ein eine Rolle füllender Punkt, fügen keine hinzu: ein ganzes Netz wird mit ROUTE-NET gezeichnet. So kann eine Definition des Standardpakets die Verbindungen innerhalb ihrer Struktur zeichnen, etwa `{?left ?right ?tail} @ROUTE-OBJECT(priority="low")`, ohne den Rest der Netze zu verbinden.
- **UNROUTE-NET** und **UNROUTE-OBJECT** nehmen Pins heraus: die Pins eines Netzes oder einiger Objekte (gezählt wie bei ROUTE-OBJECT) werden durch die Bäume von ROUTE-NET und ROUTE-OBJECT nicht verbunden.
- **Jeder Pin und jeder Baum für sich**: ein von einem ROUTE-NET oder ROUTE-OBJECT erfasster Pin liegt im Baum dieser Anweisung, sofern kein UNROUTE-NET oder UNROUTE-OBJECT, das den Pin erfasst, gegen diese Anweisung gewinnt: es hat eine höhere Priorität (14.8) oder dieselbe Priorität und steht später in der Positionsreihenfolge aus 8.5. Der Vergleich erfolgt mit jeder ROUTE-Anweisung für sich, sodass ein Pin durch ein UNROUTE der Priorität `"normal"` aus dem Baum eines ROUTE-OBJECT der Priorität `"low"` herausgenommen werden kann und im Baum eines ROUTE-NET der Priorität `"high"` verbleibt. Eine Anweisung mit `"disabled"` wird weggelassen. Diese Anweisungen sind keine geometrischen Fakten und nehmen nicht an Konflikten teil (8.5); sie entscheiden nur, welche Pins der Solver verbindet.
- **Wann**: die Anweisungen dieser Schlüsselwörter werden ab Schritt 3 von 8.2 wirksam. Ihre Regeln, wie die letzte Zeile des Beispiels, werden in Schritt 5 vor der Generierung ausgewertet: ihre Bedingungen lesen die Bäume der Anweisungen, und die Generierung, wie die Ergänzung des Standardpakets, sieht die Bäume, die die Regeln erzeugen (10.2, 10.4).
- **Geschriebene Leitungen bleiben unberührt**: ein explizites WIRE wird wie geschrieben gezeichnet, auch auf einem Netz oder an einem Pin, die durch UNROUTE herausgenommen wurden. Bereits geschriebene Leitungen sind Teil der Bäume, die sie berühren, und der Solver ergänzt den Rest.
- **Bäume**: jedes ROUTE-NET erzeugt einen Baum auf seinem Netz, und jedes ROUTE-OBJECT einen Baum auf jedem Netz, das es verbindet. Bäume auf einem Netz, die einen Pin teilen oder die geschriebene Leitungen verbinden, sind ein Baum; er übernimmt den Stil der in der Positionsreihenfolge ersten Anweisung, und ein Hinweis wird ausgegeben, wenn ihre Stile abweichen. Ein Netz kann mehrere Bäume haben, die nichts verbindet, etwa zwei ROUTE-OBJECT auf weit auseinanderliegenden Bauteilen.
- Nur Pins werden verbunden: Anschlüsse ohne Pin liegen in keinem Baum (9.2.4). Ein Pin auf dem leeren Knoten liegt auf keinem Netz (6.3), sodass ROUTE-OBJECT ihn nicht verbindet; er wird nur durch eine auf den leeren Knoten geschriebene Leitung verbunden (13.1). Ein Baum mit weniger als zwei zu verbindenden Punkten tut nichts. Ob ein Pin verdrahtet ist (10.2), wird vor der Generierung entschieden, aus den Bäumen ohne die virtuellen Bauteile, die die Generierung hinzufügt: Ein Netz mit einem einzigen eigenen Pin, etwa ein Eingang, der ein Gate treibt, bildet einen Baum aus einem Punkt; dieser Pin ist also nicht verdrahtet, und die Ergänzung gibt ihm, was sie einem Pin ohne Leitung gibt, auch wenn das Netz zum Port seiner Ebene geführt wird. Um ein solches Netz zu seinem Port zu führen, schreibt man die Port-Markierung in die Datei, etwa `$IO_PORT @ATTACH &x` oder `$IO_PORT_STUB` des Standardpakets (12.4, 16.3): Eine Port-Markierung, die die Ergänzung hinzufügt, zählt nicht.
- **Parameter**: ROUTE-NET und ROUTE-OBJECT nehmen `mode` (einen Hinweis für den Solver, mit den Werten von WIRE, standardmäßig `"auto"`), die Stilparameter von Leitungen (13.1) und `priority`; UNROUTE-NET und UNROUTE-OBJECT nehmen `priority`.
- Hinweis: Ein WIRE mit einer leeren Punkteliste bedeutet nicht ROUTE-NET. Eine Punkteliste kann aus einer Menge stammen, die zufällig leer ist, und eine leere Liste würde dann stillschweigend alles verbinden; daher sind dies eigene Schlüsselwörter; diese Version hat auch keine optionalen Slots (5.3).
- **Vom Solver zurückgeschrieben**: konkrete WIREs (mit ihren Knicken und `mode="straight"`), die benannten Punkte, an denen sie abzweigen, und die Knotenpunkte, wenn `junction` gesetzt ist (13.2), mit den Stilparametern des Baums, kommen in den eigenen Eintrag des Solvers. Die ROUTE-NET- und ROUTE-OBJECT-Anweisungen sind die Absicht des Benutzers und bleiben erhalten; der nächste Lösevorgang berechnet sie erneut (Kapitel 17). Um aus der Mitte einer vom Verfasser geschriebenen Leitung abzuzweigen, teilt der Solver diese Leitung auf (13.1).
- **Viewer, die nicht lösen**, zeichnen Luftlinien oder nichts, wie der Viewer entscheidet (Prinzipien 2.7).
- Ein Hausstilpaket wählt mit Mengen und Regeln aus, welche Netze als Leitungen gezeichnet werden, wie im Beispiel; es gibt kein Schlüsselwort dafür, wie ein Netz gezeigt wird.

<!-- translated from en/ch14.md sha256:76f571ca1b76 -->

## 14 Layout-Absicht

### 14.1 Gemeinsame Regeln

- Eine Bedingung erzeugt ein Bedingungsobjekt. Auf eine benannte Bedingung kann verwiesen werden, was beim Finden und Bearbeiten hilft, wie `%row1 = {#M1 #M2} @ALIGN("y")`; eine Bedingung kann auch anonym sein.
- **Vokabular**: die atomaren Bedingungen für Positionen sind ausschließlich ALIGN, ORDER, DISTRIBUTE und DISTANCE, neben PLACE, ORIENT, GROUP und TABLE (14.11), das als Tabelle geschriebene Ordnungen ist. Andere Relationen werden als allgemeine Definitionen geschrieben: Symmetrie (14.7), Zentrierung (ein DISTRIBUTE aus drei Mitgliedern), Zeilen und Spalten (ALIGN mit ORDER), Signalfluss (14.6), gespiegelte Paare, Abstände im Verhältnis (mehrere exakte DISTANCEs). Es gibt kein LEFT oder ABOVE (sie sind ein ORDER oder ein DISTANCE mit einer Richtung) und kein RATIO, NEAR oder INSIDE. Relative Ausrichtung (dieselbe Ausrichtung, Spiegelbilder) ist ORIENT-RELATION (14.9); ob die Gates eines Paars einander zugewandt oder nach außen gerichtet sind, wird weiterhin mit einem ORIENT festgelegt, oder mit einem ORDER auf Anschlüssen (14.3). Dass Symbole sich nicht überlappen, Abstandskonstanten und Raster gehören zum Solver (11.7) und sind nicht Teil des Vokabulars der Bedingungen.
- **Keine Positionen relativ zur gesamten Zeichnung**: es gibt kein Schlüsselwort für „ganz links“, „ganz rechts“ oder „ganz oben“, und keine Ausdehnung der gesamten Zeichnung oder von Symbolumrissen. Man schreibt ein ORDER oder ein DISTANCE relativ zu einer Gruppe von Objekten, und der Solver findet das Layout daraus. Für einen Port ganz rechts: `{#M6 #M7 #CL} %pout @DISTANCE("left-to-right", min=2)`, oder `{#M7 %pout} @ORDER("left-to-right")`, wenn M7 bereits das am weitesten rechts liegende Bauteil ist.
- **Mitglieder sind die untersten Objekte**: Instanzen, Bündel, Anschlüsse und Pins, Punkte, TEXT (über seine eigene Position, 13.3), der Mittelpunkt eines mit Mittelpunkt geschriebenen RECTANGLE, CIRCLE oder ELLIPSE sowie mit Symbol gezeichnete Anwendungen, jeweils über ihren eigenen Punkt. Eine Anwendung ohne Symbol ist selbst kein Mitglied, sondern steht für ihre Mitglieder: die Objekte, die ihre Rollen füllen, wobei ein Mitglied, das eine Anwendung ohne Symbol ist, wie bei GROUP (11.5) nach innen bis zu den untersten Objekten verfolgt wird und die Netze darunter weggelassen werden. Ihr eigener Punkt wird `%u::position` geschrieben (6.5). Leitungen, Netze, Bedingungen und Definitionen können keine Mitglieder sein: erhält die Bedingung eines davon, schlägt sie fehl. Eine Position auf einer Leitung muss zuerst als Punkt geschrieben werden (13.1). Eine Menge ist kein Mitglied: sie hat keine eigene Position (unten).
- **Welcher Punkt zählt**: eine Instanz zählt über ihren Ursprung, den Ursprung ihres Symbols, der auch das ist, was PLACE platziert; ein Anschluss oder Pin über die Position des Pins, also die Position des Bauteils zuzüglich der starren Transformation der Pin-Position im Symbol, die nicht für sich bewegt werden kann; ein Punkt über sich selbst; eine Anwendung mit Symbol oder `::position` einer beliebigen Anwendung über ihren eigenen Punkt (unten); ein TEXT über seine eigene Position; ein mit Mittelpunkt geschriebenes RECTANGLE, CIRCLE oder ELLIPSE über seinen Mittelpunkt. Der Mittelpunkt des Rahmens eines Symbols wird nicht verwendet: die Rahmen dreipoliger und vierpoliger MOS-Symbole unterscheiden sich, und eine Reihe davon würde nicht fluchten. Um nach Pins auszurichten, schreibt man Anschlüsse. Visuelle Symmetrie erfordert, dass der Ursprung auf der Symmetrieachse des Symbols liegt, was Sache der Symbolbibliothek ist.
- **Der eigene Punkt einer Anwendung**: jede Anwendung hat einen eigenen Punkt, auch eine Anwendung ohne Symbol. Der Punkt beginnt frei, wie `(_, _)`, und wird durch Fakten gebunden: PLACE, Bedingungen, Leitungen zu den Pins ihres Symbols und die Fakten, die der Rumpf ihrer Definition auf ihn schreibt; was frei bleibt, platziert der Solver. Eine Anwendung mit Symbol wird mit dem Ursprung ihres Symbols an diesem Punkt gezeichnet (12.1), und eine Bedingung auf ihr wirkt auf diesen Punkt. Eine Bedingung auf einer Anwendung ohne Symbol wirkt auf ihre Mitglieder (oben); um ihren Punkt mit Bedingungen zu versehen, schreibt man `%u::position`: `{%dp::position #X1} @ALIGN("y")` setzt X1 auf die Höhe des Mittelpunkts des Paares, während `{%dp #X1} @ALIGN("y")` X1 und die drei Transistoren des Paares auf eine Höhe setzen würde. Der Rumpf ihrer Definition bindet den Punkt an ihre Mitglieder, wo er Fakten auf das alleinstehende `?` schreibt (9.4), etwa `{?members::position ?} @ALIGN("y")`. Ist der Punkt einer Anwendung durch keinen Fakt an etwas Gezeichnetes gebunden, hat eine Bedingung auf `%u::position` keine sichtbare Wirkung, und ein Hinweis weist darauf hin.
- **Mengen haben keine Position**: eine Menge ist nur eine Liste von Mitgliedern, ohne Mittelpunkt und ohne Ausdehnung. In der Mitgliederliste von ALIGN, ORDER oder DISTRIBUTE steht ein Element, das eine Menge ist – ein ausgeschriebenes `{…}` oder ein Verweis auf eine Menge (5.2) –, für jedes seiner innersten Mitglieder an diesem Element: die Anweisung ist eine Bedingung für jede Kombination, die aus jedem solchen Element ein Mitglied nimmt. `{#M1 {#M2 #M3} #M4} @ORDER("top-to-bottom")` ist `{#M1 #M2 #M4}` und `{#M1 #M3 #M4}`, sodass M2 und M3 jeweils zwischen M1 und M4 liegen und untereinander nicht geordnet sind. Ein Element, das eine Anwendung ohne Symbol ist, steht auf dieselbe Weise für ihre Mitglieder (oben). Eine Menge in einem Slot von DISTANCE steht für jedes ihrer Mitglieder, eine Bedingung für jedes Paar (14.5). Ein Element, das eine leere Menge ist, wird weggelassen. Eine Kombination, die dasselbe Objekt zweimal nimmt, wird weggelassen: zwei Strukturen, die ein Bauteil teilen und als `{%stage1 %stage2} @ORDER("left-to-right")` geordnet werden, ordnen jedes andere Paar ihrer Mitglieder und ordnen dieses Bauteil nicht gegen sich selbst.
  - Die so erzeugten Bedingungen haben die Priorität, den Namen und die Position der geschriebenen Anweisung (8.5, 14.8); ein Name bindet deren Menge. Eine Kombination mit zu wenigen Mitgliedern, um etwas auszusagen, liefert einen Hinweis.
  - Eine Implementierung darf begrenzen, für wie viele Bedingungen eine Anweisung steht; jenseits ihrer Grenze schlägt die Anweisung fehl, mit einem Fehler, der angibt, dass dies eine Grenze der Implementierung ist (1.6).
  - Eine Menge, die das einzige Objekt ist, wird zuerst eine Stufe aufgeteilt (5.2.1): `%row_set @ORDER("left-to-right")` und `{%row_set} @ORDER("left-to-right")` ordnen beide die Mitglieder der Menge.
- **Richtungen werden ausgeschrieben.** ALIGN nimmt eine Projektion, `"x"` oder `"y"` (14.2); ORDER, DISTRIBUTE und DISTANCE nehmen eine Richtung, eine von vieren (14.3). Keine Form lässt die Richtung offen: eine Relation, die auf beide Arten gezeichnet werden darf, wird als ein Fakt für eine Art geschrieben, und eine Definition, die beide Zeichnungen erlauben soll, kommt als zwei Definitionen, wie `$BRANCH` und `$BRANCH_HORIZONTAL` des Standardpakets. Die einzigen Fakten, die eine Wahl lassen, sind die Listen von Ausrichtungen von ORIENT (11.2) und ORIENT-RELATION (14.9).
- Bedingungen ändern die Netzliste nicht. Bei einem Konflikt wird eine atomare Bedingung ganz behalten oder ganz verworfen (8.5).
- Die vier atomaren Bedingungen, ORIENT-RELATION und TABLE nehmen den Parameter `priority=` (14.8).

### 14.2 ALIGN

`{…} @ALIGN("y")`: die Mitglieder stimmen in der Projektion `axis` überein: `"y"` gibt ihnen dasselbe y (eine Zeile), `"x"` dasselbe x (eine Spalte). `axis` ist erforderlich, darf nach Position angegeben werden und ist eine Projektion: eine Liste von Projektionen ist ein ungültiger Parameterwert, und die Bedingung schlägt fehl. Mit weniger als zwei Mitgliedern gilt die Bedingung trivial und schlägt nicht fehl: wie viele Mitglieder eine Auswahl oder eine Menge hat, ist beim Schreiben der Anweisung nicht bekannt.

### 14.3 ORDER

```rnl
%rc_set = {#R2 #C2};
{#M1 #M1:2 #M2:2 #M2} @ORDER("left-to-right");  /* M1 on the left, the two gates face each other */
{#R1 {%rc_set} #M1} @ORDER("left-to-right")     /* R2 and C2 each between R1 and M1 */
```

- `{…} @ORDER(direction)`: die Mitglieder sind in der Projektion der Richtung streng monoton. Mit weniger als zwei Mitgliedern gilt dies trivial.
- **Richtung**: der Parameter `direction`, erforderlich, zuerst nach Position angegeben. Seine Werte sind:
  - `"left-to-right"`, `"right-to-left"`: steigendes x, fallendes x;
  - `"bottom-to-top"`, `"top-to-bottom"`: steigendes y, fallendes y (y verläuft nach oben, 11.7).

  DISTRIBUTE und DISTANCE nehmen dieselben Werte. Es gibt keinen Wert, der nur die Achse festlegt: `"x"` und `"y"` sind ungültige Parameterwerte, und die Bedingung schlägt fehl.
- Ein Element, das eine Menge ist, steht für jedes seiner Mitglieder (14.1): das zweite Beispiel ordnet R1, R2, M1 und R1, C2, M1; R2 und C2 sind untereinander nicht geordnet.
- ORDER ist ein Fakt über Punkte: er vergleicht die Punkte der Mitglieder, nicht ihre Rahmen. Die Rahmen von Symbolen entlang eines ORDER auseinanderzuhalten ist ein Kriterium des Solvers, nach jedem Fakt eingeordnet (8.5); erzwingen die Fakten ein Überlappen von Rahmen, überlappen sie, und ein Hinweis darf dies melden.
- ORDER ist streng: benachbarte Mitglieder unterscheiden sich um mindestens eine Koordinateneinheit (11.7). Es steht kurz für ein DISTANCE in seiner Richtung mit `min=1` zwischen jedem Paar benachbarter Mitglieder (14.5). Für gleiche Koordinaten verwendet man ALIGN.

### 14.4 DISTRIBUTE

```rnl
{#R1 #R2 #R3} @DISTRIBUTE("left-to-right")     /* R1, R2 and R3 from left to right, in equal steps */
```

- `{…} @DISTRIBUTE(direction)`: die Mitglieder folgen einander streng in der Richtung wie bei ORDER (14.3), und die Abstände zwischen benachbarten Mitgliedern in ihrer Projektion sind gleich. Die Schritte liegen zwischen den Punkten der Mitglieder, nicht den Lücken zwischen Rahmen; ein Element, das eine Menge ist, steht für jedes seiner Mitglieder (14.1).
- **Richtung**: der Parameter `direction`, erforderlich, zuerst nach Position angegeben, mit den Werten von ORDER (14.3).
- Bei zwei Mitgliedern ist DISTRIBUTE dasselbe wie ORDER; bei weniger als zweien gilt es trivial.

### 14.5 DISTANCE

```rnl
#M1:2 #M1 @DISTANCE("left-to-right", min=1);      /* the gate is on the left */
#R1 #R2 @DISTANCE("left-to-right", exact=8)       /* R2 exactly 8 units to the right of R1 */
```

```rnl
%stage1_set = {#M1 #M2 #M5};
%stage2_set = {#M3 #M4};
%stage1_set %stage2_set @DISTANCE("left-to-right", min=1)     /* all of stage 2 is to the right of all of stage 1 */
```

- `A B @DISTANCE(direction, min=…, max=…)`: A füllt den Slot `from` und B den Slot `to`; jedes ist ein Objekt oder eine Menge.
- **Richtung**: der Parameter `direction`, erforderlich, zuerst nach Position angegeben, mit den Werten von ORDER (14.3). Nur diese Achse wird eingeschränkt; dies ist nicht der euklidische Abstand.
  - Eine steigende Richtung (`"left-to-right"`, `"bottom-to-top"`) schränkt B − A ein;
  - eine fallende Richtung (`"right-to-left"`, `"top-to-bottom"`) schränkt A − B ein.
- **Mengen**: eine Menge in einem Slot steht für jedes ihrer Mitglieder (14.1), ebenso eine Anwendung ohne Symbol, und das DISTANCE gilt von jedem Mitglied von A zu jedem Mitglied von B, eine Bedingung für jedes Paar; ein Paar eines Objekts mit sich selbst wird weggelassen. So setzt `%stage1_set %stage2_set @DISTANCE("left-to-right", min=1)` ganz Stufe 2 rechts von ganz Stufe 1, und zwei Mengen, die sich nicht durchmischen sollen, werden auf diese Weise geschrieben. Ist eine Seite eine leere Menge, gibt es kein Paar, und es geschieht nichts.
- Mindestens eines von `min`, `max` und `exact` muss geschrieben werden; `min` darf nicht größer sein als `max`. Ein DISTANCE gibt zugleich die Ordnung an: B liegt in der Richtung niemals vor A. Daher sind die Schranken nicht negativ, und ein negatives `min`, `max` oder `exact` ist ein ungültiger Parameterwert, dessentwegen die Bedingung fehlschlägt; ein nicht geschriebenes `min` ist 0, sodass `max=10` für einen Abstand von 0 bis 10 gilt.
- `exact=` ist ein exakter Abstand, kurz für gleiches `min` und `max`: B − A = exact für eine steigende Richtung, A − B = exact für eine fallende. Es kann nicht zusammen mit `min` oder `max` geschrieben werden: das ist ein ungültiger Parameter, und die Anweisung schlägt fehl. Werkzeuge schreiben exakte Abstände mit `exact` (4.3).

### 14.6 Signalfluss

Signalfluss (Prinzipien 2.4) wird als Anwendung einer allgemeinen Definition geschrieben; es gibt kein Schlüsselwort dafür. Ob das Standardpaket eine solche Definition bereitstellt und unter welchem Namen, liegt beim Standardpaket (16.3); die Beispiele verwenden `$SIGNAL_FLOW` der Standardserie 0.4, das einen als Menge angegebenen Pfad benennt und vor ihm die Quellen sowie nach ihm die Lasten nennen kann. Es ist eine semantische Definition: sie platziert nichts.

```rnl
%rc_set = {#R2 #C2} @GROUP;
{#R1 {%rc_set} #M1} @AS $SIGNAL_FLOW                                    /* the signal goes from R1 through the block of R2 and C2 to M1 */
{#:inp #:inn}(source) {#R1 {%rc_set} #M1}(path) #CL(load) @AS $SIGNAL_FLOW   /* from the two inputs of the level to the load CL */
```

- Signalfluss ist ein weicher Begriff: nicht jede Schaltung hat eine strenge Richtung. Ein Solver darf ihn als Präferenz lesen; um die Stufen in der Zeichnung in eine Reihenfolge zu bringen, schreibt man ORDER oder TABLE (14.3, 14.11).
- Eine Stufe des Pfades, die aus mehreren Objekten besteht, wird als eigenes `{…}` geschrieben, etwa `{%rc_set}`. Ein GROUP (11.5) bewirkt nur, dass der Solver sie als einen Block behandelt, und darf weggelassen werden.
- Die Quellen und die Lasten sind jeweils eine Gruppe ohne Reihenfolge, wie etwa die beiden Eingänge eines Differenzsignals. Sie können Bauteile, Strukturen, Ports der Ebene wie `#:inp` (6.4), andere Anschlüsse oder Netze sein. Ohne markierte Rollen sind die Objekte der Pfad, wie in der ersten Zeile.
- Der Leistungsfluss, von einer Versorgung durch die Stufen, die sie wandeln oder schalten, zu den Teilen, die sie nutzen, wird auf dieselbe Weise mit `$POWER_FLOW` geschrieben.

### 14.7 Symmetrie

Symmetrie wird als Anwendung einer allgemeinen Definition geschrieben. Die Symmetrieachse ist lediglich ein freier Punkt, und kein Schlüsselwort ist nötig. Die Standardserie 0.4 stellt `$MIRRORED_LAYOUT` bereit, mit den Rollen `left`, `right` und `axis`: links und rechts auf derselben Höhe, die Achse in der Mitte dazwischen, und rechts ausgerichtet wie links links-rechts gespiegelt; sowie `$MIRRORED_VERTICAL_LAYOUT`, mit `top`, `bottom` und `axis`, dasselbe aufrecht gedreht. Der eigene Punkt beider Anwendungen liegt im Mittelpunkt zwischen den beiden Seiten (14.1), sodass eine Bedingung auf der Anwendung das Paar bewegt. Wie in jeder Definition des Standardpakets haben diese Bedingungen die Priorität `"low"` (9.5). Die Achse ist ein Objekt: stehen mehrere Bauteile auf der Achse, macht man sie zuerst zu einer Anwendung, die über ihren eigenen Punkt zählt (14.1), oder gibt eines davon an.

```rnl
%ax = (_, _);
%sym12 = #M1(left) #M2(right) %ax(axis) @AS $MIRRORED_LAYOUT
```

- `$MIRRORED_LAYOUT` legt die tatsächliche Ausrichtung nicht fest: ob die Gates nach innen oder nach außen zeigen, erfordert noch ein ORIENT auf einer Seite (14.9). Für ein symmetrisches Paar mit derselben Ausrichtung auf beiden Seiten schreibt man eine andere allgemeine Definition ohne sein ORIENT-RELATION.

### 14.8 Priorität: PRIORITY

```rnl id=dpl
$DP_LAYOUT = @DEFINE(roles=["in_p", "in_n"]) {
  {?in_p ?in_n} @ALIGN("y", priority="low")                    /* a default layout, the first to give way */
};
%dp2 = #M1(in_p) #M2(in_n) @AS(priority="high") $DP_LAYOUT;     /* this pair keeps the default layout strictly */
#M1 @PLACE(12, 20, priority="high")                             /* fixes the position of M1 */
```

```rnl use=dpl
# @SELECT-USAGE(recursive=true) $DP_LAYOUT => @PRIORITY("disabled")    /* a house-style package turns the default layout off, to apply its own */
```

- **Vier Werte**: `"disabled"`, `"low"`, `"normal"`, `"high"`. Ein Fakt ohne Priorität ist `"normal"`, ob er in einem Eintrag des Benutzers, in einem Paket oder im Rumpf einer Definition geschrieben steht.
- **Geordnet nach Rang, nicht gewichtet**: bei einem Konflikt werden zuerst Prioritäten verglichen, und die höhere wird behalten; bei gleichen Prioritäten wird die später geschriebene behalten (8.5). Bedingungen werden nicht mit Gewichten summiert.

  Hinweis: Mit Gewichten könnten zehn Bedingungen niedriger Priorität eine hoher Priorität aufwiegen, und Solver unterscheiden sich darin, wie sie messen, wie stark eine Bedingung verletzt ist.
- **Jeder behaltene Fakt ist hart**: `"low"` bedeutet „wird im Konflikt zuerst verworfen“, nicht „so nah wie möglich“. Der Solver muss jeden behaltenen Fakt erfüllen, gleich welcher Priorität; die eigenen Ziele der Implementierung für eine ansprechende Zeichnung werden erst innerhalb der behaltenen Fakten optimiert, nach allen Beschreibungen der Sprache.
- **`"disabled"`**: der Fakt wird vor der Ordnung entfernt und dem Solver nicht übergeben. Dies ist kein Fehlschlag: das Objekt existiert weiterhin, davon abhängige Beschreibungen sind nicht betroffen, es wird keine Warnung ausgegeben, und eine später geschriebene Priorität kann es wieder einschalten.
- **Wie sie geschrieben wird**:
  - PLACE, ORIENT, die vier atomaren Bedingungen, ORIENT-RELATION, TABLE, ROUTE-NET, ROUTE-OBJECT, UNROUTE-NET, UNROUTE-OBJECT, AS und ATTACH nehmen den Parameter `priority=` an, und die Priorität gehört zum Fakt. `priority` ist der einzige Standardparameter von AS und ATTACH; ein Parameter einer Definition darf nicht diesen Namen haben.
  - Die PRIORITY-Anweisung ändert eine Priorität nachträglich. Sie nimmt nur Objekte mit einem eigenen Handle an: Bedingungen, Anwendungen und Mengen mit GROUP (nur ein und aus, 11.5). Eine Menge mit GROUP wird als die Gruppe selbst genommen; jede andere Menge wird Mitglied für Mitglied genommen (5.2.1). Erhält sie eine Instanz oder ein anderes Objekt, schlägt sie fehl (falsche Art): `#M1 @PLACE(…) => @PRIORITY(…)` ist ein Fehler, weil PLACE das Handle von M1 weiterreicht; man schreibt die Priorität als Parameter von PLACE.
  - Der Eintragskopf `@RNL(priority=…)` gibt den Anweisungen des Eintrags, die diesen Parameter annehmen und ihn nicht schreiben, sowie `dx` und `dy` von TEXT (13.3) einen Standardwert. AS und ATTACH gibt er ausschließlich `"disabled"`, sodass ein deaktivierter Eintrag seine Anwendungen samt ihren Rümpfen ausschaltet wie seine anderen Anweisungen. Unter einem Kopf mit `"high"`, `"normal"` oder `"low"` hat eine Anwendung nur dann eine Priorität, wenn ihre eigene Anweisung oder eine PRIORITY-Anweisung ihr eine gibt, und behält andernfalls die Prioritäten ihres Rumpfes, wie wenn eine Regel sie erzeugt, sodass eine vom Solver ausgeschriebene Anwendung (10.5) dasselbe bedeutet wie die generierte. Eine Priorität, die eine Anwendung schreibt, überschreibt ihren Rumpf weiterhin (unten).
- **Reihenfolge des Überschreibens**, von niedrig nach hoch: der Standardwert des Eintragskopfs; der eigene Parameter der Anweisung; PRIORITY-Anweisungen (unter mehreren gewinnt die spätere; bei jenen in Paketen nach der Position des USE-PACKAGE); die Priorität einer umschließenden Anwendung; PRIORITY-Anweisungen, die von außen über `::` direkt auf eines ihrer lokalen Objekte geschrieben sind (unter mehreren gewinnt die spätere). Ist keines davon angegeben, ist die Priorität `"normal"`.
- **Die Priorität einer Anwendung wirkt nach unten**: die Priorität einer Anwendung (ihr eigener Parameter oder eine PRIORITY-Anweisung auf ihr) gilt für alle Fakten, zu denen sie expandiert, einschließlich jener geschachtelter Anwendungen, und überschreibt deren eigene Prioritäten; bei Schachtelung entscheidet die äußerste Anwendung mit einer Priorität. Um nur einen Teil darin anzupassen, schreibt man von außen über `::` direkt auf jenes lokale Objekt (9.4): dies ist spezifischer als die Priorität der Anwendung und überschreibt sie: die spezifischere gewinnt. Eine solche im Rumpf einer umschließenden Definition geschriebene Anweisung unterliegt weiterhin der Priorität der umschließenden Anwendung. Jede Anweisung eines Eintrags kann direkt geändert werden, sodass der Eintragskopf nur einen Standardwert vorgibt.
  - Im Einzelnen: die Priorität einer Anwendung bestimmt über jede im Rumpf ihrer Definition geschriebene PRIORITY-Anweisung, die dann keine Wirkung hat. Ein von außen über `::` direkt auf eine lokale Anwendung geschriebenes PRIORITY bestimmt ebenso: jene Anwendung zählt als die äußerste mit einer Priorität, und die Anweisung entscheidet für alles, wozu diese Anwendung expandiert. Unter den Anweisungen, über die nichts bestimmt, gewinnt die auf dem innersten Objekt geschriebene; auf demselben Objekt gewinnt eine von außerhalb des Rumpfes, der das Objekt erzeugt, geschriebene gegenüber einer in jenem Rumpf geschriebenen; danach gewinnt die spätere.
  - Beispielsweise sind bei deaktiviertem `%o1`, `%o1::in @PRIORITY("high")` oben und `%in::row @PRIORITY("low")` im Rumpf der Definition von `%o1` die Fakten von `row` auf `"high"`: über die Anweisung im Rumpf bestimmt die Priorität von `%o1`, welche die Anweisung oben für `in` überschreibt. Hätte `%o1` keine Priorität, wären sie `"low"`, geschrieben auf dem innereren Objekt. Um in beiden Fällen `"low"` zu haben, schreibt man `%o1::in::row @PRIORITY("low")` von außen.
- **Die Ausgabe des Solvers**: der Solver schreibt seine Ergebnisse als PLACE, ORIENT usw. mit `"high"` in den letzten Eintrag des eigenen Texts der Ebene, etwa `@RNL(type="generated", priority="high") {…}`. Bei einer Teilschaltung steht dies innerhalb ihres `.subckt … .ends`, auch wenn es in einer eingebundenen Datei liegt; bei der obersten Ebene steht es auf der Dateiebene der Hauptdatei (Kapitel 17). Wenn der Solver löst, wird die Ebene ohne die Einträge mit `type="generated"` interpretiert (2.8), sodass alte Ergebnisse den nächsten Lösevorgang nicht einschränken; nach dem Lösen entfernt er sie und schreibt die neuen Ergebnisse am Ende hinein.
  - Zwischen Lösevorgängen wird eine neue Anweisung normaler Priorität, die mit den Ergebnissen des Solvers in Konflikt steht, verworfen und erst nach dem nächsten Lösevorgang wirksam. Ein Viewer darf anhand des Hashes im Eintragskopf ein erneutes Lösen vorschlagen; solche Verwerfungen müssen nicht einzeln gemeldet werden.
  - Ein Ziehen in einem Editor wird als PLACE mit `"high"` geschrieben, ebenfalls im eigenen Text der Ebene, nach dem Eintrag des Solvers. Dies legt die Position fest: jeder spätere Lösevorgang nimmt sie als harte Bedingung. Um sie freizugeben, löscht man diese Anweisung.
- **Was der Solver schreibt.** Der Eintrag des Solvers muss jede Position, Ausrichtung und jeden Pfad enthalten, die der Solver entschieden hat und die der Text offenlässt: nach Namen oder Schlüssel für die Objekte, die einen haben, nach Nummer für die anderen (5.9), mit WIRE für den Pfad einer Leitung (13.1) und als explizite Anweisungen für die Objekte, die Regeln generieren (10.5). Eine Entscheidung, die dort nicht geschrieben steht, ist kein Ergebnis: das Objekt bleibt frei, als wäre die Ebene nicht gelöst worden. So zeichnet ein Viewer, der nicht löst, die Ebene exakt so, wie der Solver sie gezeichnet hat. Der Solver entscheidet nichts, worauf er nicht verweisen kann, etwa die Position eines Texts ohne Schlüssel, den eine Generierungsregel direkt erzeugt; ein solcher Text wird vom Viewer platziert (13.3). Der eigene Punkt einer von einer Regel erzeugten Anwendung wird ebenfalls nicht geschrieben, wenn diese Anwendung nicht ausgeschrieben werden kann, etwa wenn ihre Definition paketprivat ist (16.2), weder Symbol noch Text hat und kein anderes Objekt und keine Leitung über ihren Punkt platziert wird: ein solcher Punkt hilft nur dem Solver, nichts Gezeichnetes steht auf ihm, und ihn frei zu lassen ändert keine Zeichnung; er zählt nicht als ungeschrieben gebliebene Entscheidung.
- **Die Ergebnisse eines Solvers sind RNL.** Gibt ein Solver seine Ergebnisse in einer anderen Form an andere Werkzeuge weiter, etwa in den Nachrichten zwischen einem Solver und einem Viewer, darf diese Form nur das enthalten, was ein Eintrag des Solvers enthalten kann, sodass jedes Ergebnis als ein solcher Eintrag gespeichert werden kann.
- **Die Ziele eines Solvers** wählen lediglich unter den Layouts, die jeden behaltenen Fakt erfüllen; ein behaltener Fakt ist hart, gleich welcher Priorität. Die Ziele sind nicht Teil der Sprache: die Arten der Ziele und ihre Reihenfolge unterscheiden sich zwischen Solvern, und dieser Standard kann kein gemeinsames Vokabular dafür festlegen. Ein Standard ist festgelegt: eine `"auto"`-Leitung, der nichts im Weg steht und die nicht mit anderen Leitungen zusammengeführt wird, wird zu ihrem Standardpfad gelöst (13.1), der zuerst Knicke und als Zweites Länge zählt. Optionen des Solvers gehören zu der Eingabe, die der Hash abdeckt (Kapitel 17); eine Datei hält sie mit SOLVER-CONFIG fest, damit ein Ergebnis reproduziert werden kann (14.10).

### 14.9 Ausrichtungsbeziehung: ORIENT-RELATION

```rnl
#M1 #M2 @ORIENT-RELATION("MY");                  /* M2 is drawn as M1 flipped left-right */
#M5 {#M7 #M8} @ORIENT-RELATION("R0");            /* two facts: M7 and M8 each have the orientation of M5 */
#M3 #M4 @ORIENT-RELATION(["R0", "MY"])           /* M4 as M3, or as M3 flipped left-right */
#M1::g #M2::g @ORIENT-RELATION("R180")           /* the gates of M1 and M2 face opposite ways */
```

- `A B @ORIENT-RELATION(relation)`: zwei Slots, `from` und `to`, jeweils ein Objekt. Die Relation ist einer der acht Ausrichtungscodes von ORIENT (11.2) oder eine Liste davon, nach Position angegeben. Die Ausrichtung von B ist die Ausrichtung von A gefolgt von der Transformation der Relation. Die Transformation wird in den Koordinaten der Zeichnung genommen, sodass `"MY"` stets links-rechts an einer senkrechten Linie spiegelt, ob A selbst gedreht ist oder nicht.
- **Eine Liste von Relationen**, wie `["R0", "MY"]`, bildet einen Fakt, der gilt, wenn eine der aufgeführten Relationen zwischen A und B gilt. Er wird als Ganzes behalten oder verworfen (8.5), und der Solver wählt, welche Relation er zeichnet. Die Liste hat mindestens einen Code; eine Liste mit einem Code ist dasselbe wie dieser Code.
- **Ein Objekt zu einem Objekt.** Eine Menge in einem Slot wird Mitglied für Mitglied abgearbeitet (5.2.1), ein Fakt für jedes Mitglied: `#M5 {#M7 #M8} @ORIENT-RELATION("R0")` gibt M7 und M8 jeweils die Ausrichtung von M5 und somit allen dreien dieselbe. Eine Menge mit mehr als einem Mitglied in beiden Slots ist ein Fehler: das paarweise Zuordnen in Reihenfolge und das In-Beziehung-Setzen jedes Paars wären beide sinnvoll (5.2.1).
- Es gibt keine Form mit einer einzigen Menge, deren Mitglieder alle dieselbe Ausrichtung haben: das Schlüsselwort benennt eine Beziehung zwischen zwei Dingen, und bei einer Menge ist nicht klar, worauf sich die Beziehung bezieht. Ein einzelnes Objekt passt nicht zur Signatur, was ein Fehler ist. Eine einzelne Menge wird zuerst eine Stufe aufgeteilt, wie 5.2.1 sagt, sodass `{#M1 #M2} @ORIENT-RELATION("MY")` gleich `#M1 #M2 @ORIENT-RELATION("MY")` ist; liefert die Aufteilung nicht zwei Objekte, ist dies ein Fehler.
- Mitglieder sind Objekte, die eine Ausrichtung haben, wie bei ORIENT (11.2): Instanzen, Bündel und Anwendungen mit Symbol; oder beide sind Pins (unten), wobei ein Anschluss für seinen Pin steht (5.2.4). Ein Punkt, ein Netz, eine Leitung oder eine Anwendung ohne Symbol ist ein Fehler, und ebenso ein auf ein Objekt bezogener Pin: die Bedingung schlägt fehl. Erhält ein Slot eine leere Menge, gibt es nichts in Beziehung zu setzen, und es geschieht nichts (5.2.1).
- **Pins.** Mit einem Pin in jedem Slot gilt die Relation zwischen den Richtungen der beiden Pins in der Zeichnung (11.2): die Richtung von B ist die um die Relation gedrehte Richtung von A, etwa `"R180"` für in entgegengesetzte Richtungen zeigende Pins, und ein gespiegelter Code dreht eine Richtung so, wie er einen Punkt dreht. Die Relation wird ausschließlich mit den Codes geschrieben: die Wörter aus 11.2 benennen Richtungen, keine Drehungen, und hier geschrieben sind sie ungültige Parameterwerte, sodass die Bedingung fehlschlägt. Ein Pin ohne geschriebenes `direction=` lässt diesen Fakt fehlschlagen (9.2.4).
- Sie ist eine Bedingung: sie erzeugt ein Bedingungsobjekt, kann benannt werden, nimmt `priority=` an und kann mit PRIORITY geändert werden (14.8); in Konflikten wird sie als eine Einheit behalten oder verworfen (8.5). Mitglied für Mitglied ausgeführt, erzeugt sie eine Bedingung für jedes Mitglied, und ein Name bindet deren Menge (5.2.1). Ein Mitglied mit einem ORIENT aus einem Code hat seine Ausrichtung bereits festgelegt; für die anderen Mitglieder wählt der Solver unter den Ausrichtungen, die die Relation und ihre ORIENT-Listen erfüllen (11.2).
- Sie legt die tatsächliche Ausrichtung nicht fest: wenn zwei MOS-Transistoren Spiegelbilder sind, erfüllen sowohl nach innen gerichtete Gates als auch nach außen gerichtete Gates `"MY"`. Um zu entscheiden, schreibt man zusätzlich ein ORIENT oder ein ORDER auf Anschlüssen (14.3); um sie einzugrenzen, ohne festzulegen, schreibt man ORIENT mit einer Liste (11.2).

### 14.10 Solver-Einstellungen: SOLVER-CONFIG

```rnl header
@SOLVER-CONFIG(spacing_x=2, spacing_y=1, layout="lines", lines_weights=[1, 4])    /* the default of the file */
```

```rnl
@SOLVER-CONFIG(spacing_x=1)                                                       /* this level: overrides the key spacing_x */
```

- SOLVER-CONFIG gibt dem Solver Einstellungen vor, damit die Datei erneut mit denselben Einstellungen gelöst werden kann. Es erzeugt keine Fakten: der Solver nutzt die Einstellungen nur zur Auswahl unter den Layouts, die jeden behaltenen Fakt erfüllen (14.8).
- **Form**: benannte Parameter in RNL-Literalen: Zahlen, Zeichenketten, `true` und `false` sowie Arrays davon (3.7). Es gibt keine geschachtelten Einstellungen. Abgesehen von den Schlüsseln, die dieser Standard benennt (unten), sind die Schlüssel keine Standardwörter: jeder Solver definiert seine eigenen. Der Kern liest die Parameter als RNL-Literale, reicht sie wie geschrieben in der unten genannten Reihenfolge an den Solver weiter und führt keine davon zusammen; von den Werten prüft er nur die der Schlüssel, die dieser Standard benennt. Die übrigen Schlüssel und Werte beurteilt der Solver: ein ihm unbekannter Schlüssel liefert eine Warnung und wird ignoriert. Ein Solver kann einen Schlüssel auch nur unter einigen seiner eigenen Einstellungen annehmen, etwa ein Optimierungsziel, das eines seiner Layouts verwendet und ein anderes nicht; unter einer Einstellung, die den Schlüssel nicht verwendet, wird er ebenso mit einer Warnung ignoriert.
- **Von diesem Standard benannte Schlüssel**: `spacing_x` und `spacing_y`, der Abstand der Ebene in Rastereinheiten, addiert zum Standardabstand des Solvers, wie bei Gruppen (11.5); sowie `junction`, die Definition, mit der der Solver die Punkte an Verzweigungspunkten zeichnet, ein als Zeichenkette geschriebener `$`-Name wie `"$JUNCTION"`; im SOLVER-CONFIG eines Ebeneneintrags auch ein `%`-Name, der in dieser Ebene nachgeschlagen wird (6.6), damit eine für eine Teilschaltung definierte Variante in ihr bleibt; oder `""` für keine Punkte (13.2). Jeder Solver nimmt `junction` an. Sie sind die einzigen Schlüssel, die diese Version benennt, nicht die einzigen Schlüssel, die ein Solver annehmen darf: welche weiteren Schlüssel ein Solver hat, wie `layout` und `lines_weights` im Beispiel, und was sie bedeuten, liegt bei diesem Solver, der sie dokumentiert. Ein Solver, der einen Abstand der Ebene hat, nimmt ihn unter diesen beiden Namen an.
- **Wo**: im Kopfeintrag ohne `language`, für die Ebenen jener Datei und der von ihr eingebundenen Dateien, wie bei DEVICE (2.8, 6.12); im Kopfeintrag einer Paketdatei, für die Ebenen, die das Paket verwenden (16.2); und in den Einträgen einer Ebene, für jene Ebene. Es hat keine Objekte. Geschrieben im Inhalt eines Pakets, in einem Inline-Paket, in einem Eintrag mit `language` oder in einem Eintrag des Solvers hat es keine Wirkung, und es wird eine Warnung ausgegeben; somit liefert ein nur über Kopfeinträge mit `language` verwendetes Paket keine Einstellungen.
- **Reihenfolge**: Einstellungen überschreiben Schlüssel für Schlüssel, die spätere über die frühere: die Standardwerte des Solvers, die Kopfeinträge der von der Ebene verwendeten Pakete in der Reihenfolge ihrer Positionen (wie bei DEVICE, 6.12), die Kopfeinträge der äußeren Dateien, dann jene der eingebundenen Dateien, die Einträge der Ebene, dann die dem Werkzeug gegebenen Einstellungen, etwa auf der Befehlszeile oder in einer Seite eines Viewers. Ein Werkzeug übernimmt diese in demselben Text wie die Klammern, sodass sie in beide Richtungen kopiert werden können.
- **Ein Wert, der nicht angenommen wird**, etwa eine Zeichenkette, wo eine Zahl angenommen wird, ist ein Fehler nur für diesen Schlüssel (16.4): der Schlüssel gilt an dieser Stelle als nicht geschrieben, sodass der Wert von vorher gültig bleibt, und die anderen Schlüssel derselben Anweisung gelten wie gewohnt. Ein Wert wird in jeder Ebene beurteilt, für die er gilt: ein `junction` in einem Kopfeintrag, das in einer Ebene keine Definition benennt, wird nur in dieser Ebene nicht angenommen, und die anderen Ebenen verwenden es weiter. Der Kern beurteilt die Werte der Schlüssel, die dieser Standard benennt, und lässt die aus, die er nicht annimmt; der Solver beurteilt die Werte seiner eigenen Schlüssel. Die Einstellungen, die ein Solver erhält, sind also eine Liste in der obigen Reihenfolge, in der ein Schlüssel mehrmals vorkommen kann, und für jeden Schlüssel nimmt der Solver den letzten Wert, den er akzeptiert.
- **Zeit- oder Arbeitsbudgets** (8.5) gibt nur das Werkzeug vor, etwa auf der Befehlszeile oder in einer Seite eines Viewers, nie die Datei: Ein Budget hängt vom Rechner ab, der löst. Die Schlüssel eines Solvers, die ein Budget setzen, etwa `limit`, `seconds` und `work` des eingebauten Solvers, sind seine werkzeugeigenen Schlüssel: In SOLVER-CONFIG geschrieben, in jedem Eintrag, wird ein solcher Schlüssel wie ein unbekannter Schlüssel mit einer Warnung ignoriert, und das vom Werkzeug gegebene Budget oder der Standardwert des Solvers bleibt in Kraft.
- Die für eine Ebene wirksamen Einstellungen, jene der Pakete eingeschlossen, gehören zu der Eingabe, die der Hash des Solvers abdeckt (Kapitel 17).

### 14.11 Tabellen: TABLE

```rnl
@TABLE layout: {
  {#M1 #M2 #M3}
  {#M4 {#M5 #M6} #M7}
}
```

- TABLE ordnet Objekte als Tabelle an, geschrieben mit geschachtelten `{…}`: das Beispiel hat zwei Zeilen von oben nach unten; in der ersten M1, M2 und M3 von links nach rechts; in der zweiten M4, die Zelle aus M5 und M6 sowie M7 von links nach rechts; in dieser Zelle M5 über M6. Eine Tabelle darf über mehrere Zeilen geschrieben werden. Die Standardform schreibt das Schlüsselwort zuerst, gelesen als „@TABLE layout: {…}“ (4.3); das Wort `layout:` dort ist Freitext (3.3), und der Slot von TABLE, der die Tabelle annimmt, ist `rows` (Anhang A).
- **Lesen**: die äußere Liste verläuft von oben nach unten, ein Element für jede Zeile; jede darin geschachtelte Liste verläuft im rechten Winkel zu der Liste, die sie enthält, von links nach rechts in einer von oben nach unten verlaufenden Liste, und von oben nach unten in einer von links nach rechts verlaufenden. `transpose=true` lässt die äußere Liste von links nach rechts verlaufen. Die an TABLE gegebene Menge ist seine äußere Liste wie geschrieben und wird nicht zuerst aufgeteilt (5.2.1 Punkt 2), sodass `@TABLE layout: {{#M1 #M2 #M3}}` eine Zeile ist und `@TABLE layout: %port_set` jedes Element der Menge in eine eigene Zeile setzt.
- **Elemente**: ein Element, das ein Objekt ist, ist eine Zelle; ein Element, das eine Menge ist – ein in der Tabelle geschriebenes `{…}` oder ein Verweis auf eine Menge –, ist eine Liste eine Ebene tiefer. Dies ist die Regel von TABLE: eine Menge darin ist kein Element, das für jedes seiner Mitglieder steht (14.1). Eine mit Symbol gezeichnete Anwendung ist über ihren eigenen Punkt eine Zelle; eine Anwendung ohne Symbol ist eine Zelle, die ihre Mitglieder aufnimmt (14.1), jedes davon vor dem nächsten Element; `%u::position` macht ihren eigenen Punkt zu einer Zelle.
- **Der Fakt**: ein TABLE ist ein geometrischer Fakt, der in Konflikten als Ganzes behalten oder verworfen wird (8.5). Er gilt, wenn in jeder Liste der Tabelle jeder Punkt eines Elements streng vor jedem Punkt des nächsten Elements in der Richtung dieser Liste liegt: mindestens eine Koordinateneinheit davor, wie bei ORDER (14.3). Ein Prüfer darf ihn in solche Ordnungen expandieren; dem Solver wird die Tabelle als ein Element übergeben, samt der Richtung ihrer äußeren Liste und der darin geschachtelten Listen.
- **Eine Gruppe**: die Objekte der gesamten Tabelle bilden eine Gruppe, wie mit `@GROUP` auf derselben Menge (11.5); ihre Zeilen und Spalten sind keine eigenen Gruppen.
- TABLE richtet nicht aus: Zeilen müssen nicht dieselbe Höhe haben und Spalten nicht dasselbe x. Um sie auszurichten, schreibt man ALIGN. Eine Zelle erstreckt sich nicht über mehrere Zeilen oder Spalten.
- Eine leere Liste in einer Tabelle wird weggelassen, und ihre Nachbarn werden Nachbarn (14.1). Eine Tabelle mit einer Zelle gilt trivial und erzeugt keine Gruppe. Ein Objekt in zwei Zellen derselben Tabelle müsste vor sich selbst kommen, sodass das TABLE fehlschlägt, ebenso wenn ein Mitglied nicht lokalisiert werden kann, wie ein Netz oder ein Anschluss ohne Pin (14.1).
- TABLE nimmt `priority=` an (14.8) und erzeugt ein Bedingungsobjekt, das benannt werden kann.

<!-- translated from en/ch15.md sha256:7d292a08ebfb -->

## 15 Elektrische Semantik

### 15.1 Anwenden semantischer Definitionen

```rnl
%dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
%cm1 = {#M3 #M9}(ref) {#M4 #M6}(out) @AS $CURRENT_MIRROR_P;
%in = {#M8 #M5} @AS $MATCHED
```

- Was Objekte bedeuten, wird durch Anwenden einer semantischen Definition mit AS markiert (9.5): das Anwenden erzeugt eine Anwendung, und die Objekte füllen die Rollen der Rollenliste der Definition. Eine Bedeutung zu markieren und eine Layout-Relation anzugeben sind dieselbe Art von Anweisung.
- Eine Anwendung kann Mitglied einer anderen Anwendung sein (Schachtelung), wie in den Beispielen von 9.4 und 10.1.
- Eine Mengenrolle, in der Rollenliste `"{name}"` geschrieben (wie `"{members}"` von `$MATCHED` sowie `"{ref}"` und `"{out}"` von `$CURRENT_MIRROR_P` im Beispiel), nimmt eine Menge für eine nicht festgelegte Anzahl von Mitgliedern an (9.4).
- Eine Rolle darf die Objektart deklarieren, die sie annimmt, etwa `roles=[["in_p", "instance"], ["in_n", "instance"]]`; ein Objekt der falschen Art lässt die gesamte Anwendung fehlschlagen (9.4). Eine Definition mit leerem Rumpf prüft nichts an dem, was die Rollen füllt, sodass ein falsches Objekt dort ohne solche Deklarationen nicht erkannt werden kann.
- Die Rollennamen einer Anwendung sind ihre Unterobjekte, etwa `%dp1::left`.
- Das Anwenden einer Definition expandiert ihren Rumpf, und der Rumpf darf Bedingungen enthalten, etwa eine Standardzeichnung mit Priorität `"low"`, welche die Priorität der Anwendung ausschaltet (9.5, 14.8).
- Nicht geprüft: ob die Bedeutung mit den Verbindungen, Typen oder Parametern der Netzliste übereinstimmt, wird nicht geprüft (Prinzipien 2.4). Beispielsweise prüft die Sprache bei `#M3(ref) #M4(out) @AS $MY_MIRROR` nicht, ob Gate und Drain von M3 verbunden sind; eine Definition prüft das, was sie behauptet, mit den Zusicherungen ihres Rumpfes (15.3, 15.4).
- Die Anwendungen einer bestimmten Bedeutung werden mit SELECT-USAGE ausgewählt, und die Anwendungen, die ein bestimmtes Objekt enthalten, mit SELECT-ANCESTOR-USAGE; die Mitglieder von Anwendungen werden über eine Auswahl mit den Anwendungen als Bereich erreicht (10.1).
- Das Vokabular semantischer Definitionen gehört zum Standardpaket (9.5), das Definitionen wie `$DIFFERENTIAL_PAIR_N` bereitstellt.
- Die grundlegende Gültigkeit einer Beschreibung (Prinzipien 2.4) besteht darin: ihre Verweise können gefunden werden, ihre Rollen erhalten Objekte der richtigen Arten, ihre Einzelrollen sind gefüllt, und jede Anweisung, zu der sie expandiert, ist selbst gültig (9.5). Ob zwei Rollen mit demselben Objekt gefüllt sind, wird nicht geprüft.

### 15.2 Stromrichtung: CURRENT

```rnl
#M5:1 @CURRENT("in");                 /* drain: the current flows in */
#M5:3 @CURRENT("out")                 /* source: the current flows out */
```

- CURRENT setzt die Richtung des statischen Stroms an einem Anschluss: `"in"` (er fließt hinein), `"out"` (er fließt heraus) oder `"unspecified"` (um einen von einer Regel eines Pakets gesetzten Wert zu überschreiben). Der Parameter ist `direction`. Eine CURRENT-Regel wird in Schritt 6 ausgewertet (10.4). Ein CURRENT, dessen Objekt ein Pin ist – so geschrieben oder über eine Rolle erreicht –, benötigt das Symbol, um seinen Anschluss zu finden (5.2.4): es ist keine Regel und wird wirksam, sobald die Symbole seines Objekts gegeben sind, in Schritt 4 oder in Schritt 7 für ein in Schritt 6 generiertes Objekt. Bis Schritt 6 liest die Bedingung `current` von Auswahlen nur die auf Anschlüsse geschriebenen CURRENT-Anweisungen; ab Schritt 7, wie in der Stil-Warteschlange, liest sie alle Richtungen, und ebenso die Geometrie und die unten stehende Prüfung.
- Es ist ein Fakt über den Anschluss, kein geometrischer Fakt: es nimmt nicht an Konflikten teil (8.5), und ein Solver nutzt es nur als Hinweis, etwa für die Reihenfolge, in der er Ausrichtungen ausprobiert. Wie ORIENT ist es eine ändernde Anweisung und reicht ihr Eingabe-Handle weiter. Es hat einen Schlüssel, einen je Anschluss, und das spätere gewinnt. Ein Anschluss, für den es nie geschrieben wird, ist nicht angegeben.
- Es wird ausschließlich auf Anschlüssen angegeben. Der Strom innerhalb eines Bauteils mit mehreren Anschlüssen lässt sich nicht eindeutig aufteilen: bei einem dreipoligen Bauteil lässt „a fließt nach b, b fließt nach c“ offen, ob der Strom bei b hinein- oder herausfließt, während die Richtung des statischen Stroms an einem Anschluss stets eine eindeutige Bedeutung hat.
- Es nimmt nur Anschlüsse des Hosts an, einschließlich der Ports dieser Ebene: `#:inp @CURRENT("in")` bedeutet, dass der Strom dort in die Teilschaltung hineinfließt. Ein Pin eines Bauteils wird in seinen Anschluss umgewandelt, etwa `#M1::d @CURRENT("in")`; virtuelle Bauteile haben keine Anschlüsse, daher werden ihre Pins nicht umgewandelt (5.2.4). Ein Anschluss ohne Pin ist zulässig (anders als bei Leitungen).
- Nicht gegen die Netzliste geprüft (Prinzipien 2.4). Die CURRENT-Anweisungen eines Netzes bilden zusammen eine Zusicherung (15.3): wenn jeder Anschluss auf dem Netz, die Ports dieser Ebene eingeschlossen, ein CURRENT von `"in"` oder `"out"` hat und alle den Strom in Bezug auf das Netz in dieselbe Richtung bewegen, liefert dies einen Verstoß (7.6), der das Netz und seine Anschlüsse nennt, da der Strom nirgendwohin abfließen kann. Ein Bauteilanschluss mit `"in"` entnimmt dem Netz Strom, und ein Port dieser Ebene mit `"in"` führt dem Netz Strom zu. Ein Anschluss auf dem Netz ohne CURRENT oder mit `"unspecified"` lässt das Netz ungeprüft. Ein globales Netz (6.3), wie `0`, wird ebenfalls nicht geprüft: es ist in jeder Ebene derselbe Knoten, sodass der Strom über Anschlüsse anderer Ebenen abfließen kann. Ein CURRENT in einem Eintrag oder im Rumpf einer Anwendung, deren Priorität `"disabled"` ist, zählt nicht. Es wird zusammen mit den Zusicherungen nach Schritt 7 von 8.2 geprüft und ist wie diese nur ein Bericht, der das Netz, seine Anschlüsse und deren CURRENT-Anweisungen nennt; die CURRENT-Fakten bleiben wirksam.
- Verwendung: der Strom am Anschluss verläuft in der Richtung, in der eine Leitung den Pin verlässt (9.2.4), gedreht mit der Ausrichtung des Symbols, wenn er herausfließt, und entgegengesetzt, wenn er hineinfließt. Ein Solver kann dies nutzen, um unter den Layouts zu wählen, etwa indem er abwärts fließenden Strom bevorzugt (14.8); aufwärts oder abwärts ist eine Frage des Hausstils. Ein Anschluss ohne CURRENT, mit `"unspecified"` oder dessen Pin kein `direction=` schreibt (9.2.4), liefert keine Richtung; eine Richtung, die eine Implementierung aus den Grafiken ableitet, zählt hier nicht. Regeln können Anschlüsse nach der Richtung ihres Stroms auswählen, mit der Bedingung `current` von SELECT-TERMINAL (10.1).
- **Pfeile**: CURRENT zeichnet nichts. Ob und wie Pfeile aus CURRENT gezeichnet werden, ist eine Präferenz des Viewers (Teil seines Hausstils), wie welche Netznamen angezeigt werden (13.3). Ein Viewer, der sie selbst zeichnet, überspringt Anschlüsse, die bereits eine Anwendung des Standard-Strompfeils haben, wie er auch das standardmäßige `$NO_CONNECT` erkennt (12.6). Pfeile, die behalten werden sollen (aus einem importierten Schaltplan oder vom Verfasser gesetzt), werden als explizite Objekte geschrieben:

  ```rnl
  $CURRENT_ARROW @ATTACH #M5:1 => @ORIENT("R270") => @TEXT("I_{SS}", dx=1);   /* on the drain pin, with I_SS beside it */
  $CURRENT_ARROW @ATTACH (12, 20) => @ORIENT("R270")                           /* in the middle of a wire, at absolute coordinates */
  ```

  - Ein auf einem Anschluss gezeichneter Pfeil ist eine Anwendung, deren Definition ihren Punkt an den Pin des Anschlusses bindet (12.1 Punkt 5). Er dreht sich nicht mit dem Pin, daher wird seine Ausrichtung geschrieben; der Text daneben wird an die Anwendung angehängt (13.3).
  - Ein Pfeil in der Mitte einer Leitung ist nicht an der Leitung verankert: er wird an einem Punkt mit absoluten Koordinaten gezeichnet.
  - Wie ein nach innen zeigender Pfeil von einem nach außen zeigenden unterschieden wird und wie Definition und Symbol des Pfeils gezeichnet werden, gehört zum Standardpaket. Das Symbol kann einfach eine Linie mit einer Pfeilspitze an ihrem Ende sein (9.2.5).
  - Gezeichnete Pfeile werden von CURRENT getrennt gehalten: wären sie ein und dasselbe, würde das Exportieren eines Pfeils ändern, worauf das Layout abzielt.

### 15.3 Zusicherungen: ASSERT-INSTANCE, ASSERT-TERMINAL, ASSERT-NET, ASSERT-USAGE, ASSERT-IN

```rnl
$MATCHED_PAIR = @DEFINE(roles=[["left", "instance"], ["right", "instance"]]) {
  {?left ?right} @ASSERT-INSTANCE(same=["model", "w", "l"], different=["name"]);
  {?left ?right} @SELECT-TERMINAL(terminal="s") => @ASSERT-TERMINAL(same=["net"], message="the sources of a pair share a net")
};
# @SELECT-INSTANCE(name="MD*") => @ASSERT-INSTANCE(type="nmos");
%dp1 @ASSERT-USAGE $DIFFERENTIAL_PAIR_N;                      /* %dp1 is a usage of this definition */
#M5:s @SELECT-NET => @ASSERT-IN $GROUND_NET_SET              /* the source of M5 is on a ground net */
```

- Eine Zusicherung prüft, ob das, was eine Datei oder eine Definition aussagt, mit den Fakten der Schaltung übereinstimmt, etwa dass die beiden Hälften eines Paars dasselbe Modell und dieselbe Größe haben oder dass Dummies von einem Typ sind. Sie meldet nur: sie erzeugt keinen Fakt, ändert nichts in der Zeichnung und nimmt nicht an der Auflösung teil (8.5).
- **Ein Schlüsselwort für jede Art**, wie bei Auswahlen (10.1): ASSERT-INSTANCE prüft Instanzen (und Bündel, anhand ihres ersten Mitglieds, 11.8), ASSERT-TERMINAL Anschlüsse, ASSERT-NET Netze, ASSERT-USAGE Anwendungen. Der eine Slot nimmt eine Menge von Objekten dieser Art an; ein einzelnes Objekt ist eine einelementige Menge. Die Art eines Objekts ist beim Lesen des Texts bekannt, und eine falsche Art ist dort ein Fehler (5.2.4), sodass keine Zusicherung Arten prüft.
- **Bedingungen** haben die Namen und Bedeutungen der Bedingungen des Auswahlschlüsselworts derselben Art (10.1), wie `type="nmos"`, `terminals=4`, `net="vss"` oder `port=true`: jedes Mitglied muss jede geschriebene Bedingung erfüllen.
- **`same=[…]`**: für jedes Attribut in der Liste haben alle Mitglieder denselben Wert. **`different=[…]`**: für jedes Attribut in der Liste haben je zwei Mitglieder verschiedene Werte. Die Attributnamen sind jene von `${…}` (13.6): bei Instanzen `name`, `value`, `model`, `type` sowie die Namen ihrer Parameter; bei Anschlüssen `net`; bei Netzen `name`; bei Anwendungen `definition`. Für Anschlüsse, Netze und Anwendungen sind dies alle: jeder andere Name ist ein ungültiger Parameterwert, und die Zusicherung schlägt fehl (7.1). Bei Instanzen sind die Namen der Parameter offen, sodass jeder Name angenommen wird, und ein Attribut, das ein Mitglied nicht besitzt, zählt als eigener Wert „nicht vorhanden“: Mitglieder, denen es fehlt, sind einander gleich und unterscheiden sich von jedem geschriebenen Wert. Daher gilt `same=["nf"]`, wenn kein Mitglied `nf` schreibt, und liefert einen Verstoß, wenn einige es schreiben und andere nicht; ein Prozess, dessen Bauteile keinen solchen Parameter haben, benötigt keine eigene Zusicherung. Eine Instanz ohne Typ (10.1) hat kein `type`. Werte werden so verglichen, wie der Host sie schreibt, ohne Auswertung (es gibt keine Ausdrücke, 3.7).
- **`has=[…]`**, für ASSERT-INSTANCE: jedes Mitglied besitzt jedes Attribut in der Liste, etwa `has=["w", "l"]`; ein Mitglied, dem eines fehlt, liefert einen Verstoß. `same` verlangt nicht, dass ein Attribut geschrieben ist; `has` tut es.
- **ASSERT-USAGE** nimmt als letztes Objekt eine Definition an, wie es SELECT-USAGE tut (5.6, 10.1); sie darf weggelassen werden. Mit ihr muss jedes Mitglied eine Anwendung dieser Definition sein, erkannt anhand dessen, wozu ihr Name auflöst (9.5), nicht anhand der Schreibweise des Namens. Ihre Bedingungen sind `symbol` und `drawn` (10.1). `same=["definition"]` gilt, wenn alle Mitglieder Anwendungen einer Definition sind, und `different=["definition"]`, wenn keine zwei es sind.
- **ASSERT-IN**: `X @ASSERT-IN S` prüft, ob jedes Mitglied von X ein Mitglied der Menge S ist, die zuletzt geschrieben wird, wie bei ADD-TO (5.2.2); S muss eine Menge sein und wird nicht in eine verpackt. Ob zwei Mitglieder gleich sind, wird anhand des Handles entschieden (5.2). Die Mitglieder dürfen beliebiger Art sein, und S darf von Auswahlen abhängen (10.4). Es hat keine Bedingungen, nur `message`, und ein Verstoß nennt die Mitglieder von X, die S nicht enthält. Mit einer Mengendifferenz (5.2.2) sagt es auch aus, dass eine Menge leer ist.
- `different=["name"]` prüft, dass Instanzen oder Netze verschiedene Objekte sind, etwa wenn zwei Rollen nicht mit demselben Objekt gefüllt werden dürfen; Anschlüsse haben kein `name`, sodass `name` für sie ein ungültiger Parameterwert ist, in `different` ebenso wie in `same`.
- **Der Bericht**: eine Prüfung, die nicht gilt, liefert einen Verstoß (7.6), der die Mitglieder und das Attribut nennt; **`message`** fügt ihm einen Text hinzu. Nichts schlägt fehl, und die Zeichnung ist dieselbe wie ohne die Zusicherung: eine Zusicherung ist nur ein Bericht für den Verfasser und für Werkzeuge wie `rnl check`.
- **Wo und wann**: in jeder Ebene und im Rumpf einer allgemeinen Definition, wo sie für jede Anwendung geprüft wird (9.4). Zusicherungen werden nach Schritt 7 von 8.2 geprüft, wenn jede Bedingung gelesen werden kann, auch `drawn` und `symbol` (10.1). Eine Menge ohne Mitglieder gilt trivial, und ebenso eine Zusicherung, die den leeren Knoten erhält (6.3).
- Eine Zusicherung hat keine Priorität; in einem Eintrag mit `priority="disabled"` und im Rumpf einer Anwendung, deren Priorität `"disabled"` ist (14.8), wird sie nicht geprüft.

### 15.4 Topologiezusicherungen: ASSERT-TOPOLOGY

```rnl
{#R1 #R2 #R3} @ASSERT-TOPOLOGY("series");                  /* a chain: R1 to R2 to R3 */
{#M3 #M4} @ASSERT-TOPOLOGY("parallel", exclude=[["nmos", "g"], ["nmos", "b"]]);   /* drains on one net, sources on one net */
{#L1 #C1 #L2} @ASSERT-TOPOLOGY("star");                    /* a T network: three branches meet at one net */
{#C1 #L1 #C2} @ASSERT-TOPOLOGY("loop");                    /* a pi network: C1, L1 and C2 go round */
{%branch1 %branch2} @ASSERT-TOPOLOGY("parallel")           /* two branches, each a usage or a set taken as a black box, between the same nets */
```

- ASSERT-TOPOLOGY prüft, ob einige Mitglieder auf eine bestimmte Weise verbunden sind: in Reihe, parallel, als Stern oder als einzelne Schleife. Es ist eine Zusicherung wie jene aus 15.3, und was 15.3 über sie sagt, gilt auch hier: es meldet nur, eine nicht geltende Prüfung liefert einen Verstoß (7.6), dem `message` einen Text hinzufügt, und wo, wann und unter welcher Priorität sie geprüft wird, ist dasselbe.
- **Die Mitglieder**: der eine Slot nimmt eine Menge von Mitgliedern in Reihenfolge an, gezählt nach Elementen (5.2): ein Objekt ist ein Mitglied, und ein Element, das ein `{…}` oder ein Verweis auf eine Menge ist, ist ein Mitglied, eine Blackbox. Eine Menge, die das einzige Objekt ist, wird zuerst eine Stufe aufgeteilt (5.2.1), sodass `%chain @ASSERT-TOPOLOGY("series")` die Elemente der Kette prüft und `{%branch1 %branch2}` zwei Mitglieder hat. Ein Mitglied ist somit eine Instanz (oder ein Bündel, anhand seines ersten Mitglieds, 11.8), eine Anwendung oder eine Menge. Erhält es das Ergebnis einer Auswahl, das für jedes Mitglied ein Element hat, stehen die Mitglieder in der Reihenfolge des Ergebnisses (10.3). Mit weniger als zwei Mitgliedern gilt die Prüfung trivial.
- **Die Netze eines Mitglieds**:
  - eine Instanz: die Netze, auf denen jene ihrer Anschlüsse liegen, die `exclude` nicht herausnimmt;
  - eine Anwendung oder eine Menge: eine Blackbox, ein aus den von ihr eingebrachten Anschlüssen gebildetes Element. Eine Anwendung, ob mit Symbol oder nicht, steht für das, was ihre Rollen füllt, und eine Menge für ihre Mitglieder, nach innen verfolgt bis zu den innersten; eine Instanz bringt alle ihre Anschlüsse ein, ein Anschluss nur sich selbst, ein Pin die von ihm gebundenen Anschlüsse, und Netze und Punkte bringen nichts ein; ein Bündel zählt nach seinem ersten Mitglied. Die Netze der Blackbox sind jene, auf denen die von ihr eingebrachten, durch `exclude` nicht herausgenommenen Anschlüsse liegen und die auch außerhalb ihrer verwendet werden: durch einen Anschluss, den sie nicht einbringt, einschließlich jener der anderen Mitglieder eines Bündels, oder als Port der Ebene. Ein Netz, das nur von den eingebrachten Anschlüssen verwendet wird, zählt nicht. Hinweis: Ein Netz, das zwei Mitglieder teilen, wird außerhalb jedes von ihnen vom anderen verwendet, sodass die Frage, ob ein Netz ein Port ist, das Prüfergebnis nur bei Mitgliedern ändert, die denselben Anschluss einbringen; sonst ändert sie nur, welche Netze die der Blackbox sind, wie ein Werkzeug sie anzeigen darf.
- **`exclude=[[type, terminal], …]`**: Anschlüsse, die nicht zählen, als Paare aus einem Typ, verglichen wie die Bedingung `type` (10.1), und einem Anschlussnamen oder einer Anschlussnummer, etwa `exclude=[["nmos", "g"], ["nmos", "b"]]`. Der Anschluss eines Paars wird verglichen wie die Bedingung `terminal` von SELECT-TERMINAL (10.1): eine Nummer nach Stelle, ein Name nach den Groß-/Kleinschreibungsregeln des Hosts; ein Name, den die Instanz nicht hat, passt auf nichts. Ein Anschluss einer Instanz ohne Typ oder eines Typs, den die Liste nicht nennt, zählt. Ohne `exclude` zählt jeder Anschluss. Ein Paar, das kein Array aus zwei Zeichenketten oder einer Zeichenkette und einer ganzen Zahl ist, ist ein ungültiger Parameterwert, und die Zusicherung schlägt fehl (7.1).
- **Die Topologien**, der erste Parameter, nach Position angegeben, eine von vieren:
  - `"series"`: je zwei in der Reihenfolge benachbarte Mitglieder teilen genau ein Netz, und zwei nicht benachbarte Mitglieder teilen keines. Was sonst auf einem gemeinsamen Netz liegt, wird nicht geprüft: die Mitte eines Teilers darf anderswohin führen.
  - `"parallel"`: mindestens zwei Netze, die Enden, werden von allen Mitgliedern geteilt. Weitere Netze eines Mitglieds, etwa ein Abzweig, den ein Zweig herausführt, werden nicht geprüft, wie auch bei `"series"` nichts anderes auf einem gemeinsamen Netz geprüft wird.
  - `"star"`: ein Netz wird von allen Mitgliedern geteilt, und keine zwei Mitglieder teilen ein anderes Netz.
  - `"loop"`: mindestens drei Mitglieder. Im Kreis der Reihe nach, das letzte und das erste eingeschlossen, teilen je zwei benachbarte genau ein Netz, diese Netze sind alle verschieden, und zwei nicht benachbarte Mitglieder teilen keines. Bei zwei Mitgliedern liefert `"loop"` einen Verstoß; man schreibt `"parallel"`.

<!-- translated from en/ch16.md sha256:fa0479c7afe6 -->

## 16 Dateien, Versionen, Pakete und Aliase

Dieses Kapitel behandelt die Sprachversion einer Datei, Pakete und Paketdateien, das Standardpaket, unbekannten Inhalt und Aliase.

### 16.1 Versionen

```rnl
* @RNL(type="header") { @VERSION("0.4")}
```

- **Eine Sprachversion je Datei.** RNL verschiedener Versionen wird in verschiedene Dateien geschrieben, die über include verbunden werden (6.1).
- **Deklariert im Kopfeintrag.** VERSION darf ausschließlich im Kopfeintrag (`type="header"`) geschrieben werden, der der erste Eintrag der Datei ist (2.8). Die Version darf nur einmal deklariert werden, sodass eine Datei niemals zwei Versionswerte hat. Ein anderswo geschriebenes VERSION ist ein Fehler und wird ignoriert.
- **Lesereihenfolge:**
  1. der Host-Adapter liefert die Kommentare;
  2. alle Einträge werden anhand ihrer äußeren Form herausgeschnitten, und ihre Eintragsköpfe werden gelesen; dieser Schritt hängt nicht von der Version ab;
  3. wenn der erste Eintrag der Kopfeintrag ist, wird sein VERSION gelesen;
  4. die übrigen Einträge werden in dieser Version geparst.
- **Teile, die nicht von der Version abhängen.** Die Kommentar-Träger, der `@RNL`-Eintrag, die Form des Eintragskopfs und die Form von VERSION im Kopfeintrag ändern sich in keiner späteren Version; andernfalls könnte ein Parser die Version nicht finden. Unterstützung für einen neuen Host fügt nur Träger hinzu und ändert bestehende nicht (2.2). Der Rest des Kopfeintrags wird in der deklarierten Version gelesen.
- **Kein Kopfeintrag.** Der Kopfeintrag darf weggelassen werden, ebenso VERSION darin. Eine eingebundene Datei übernimmt dann die Version der Datei, die sie einbindet; eine Hauptdatei übernimmt die Standardversion des Werkzeugs, mit einem Hinweis. Werkzeuge müssen den Kopfeintrag und die Version schreiben, wenn sie eine Datei speichern, sodass von Hand geschriebene Dateien sie weglassen dürfen und von Werkzeugen geschriebene Dateien sie besitzen. Eine Paketdatei muss einen Kopfeintrag haben (16.2); ohne VERSION übernimmt sie die Standardversion des Werkzeugs, wie es eine Hauptdatei tut. Ein Inline-Paket hat die Version der Datei, in der es geschrieben steht (16.2).
- **Nicht unterstützte Version.** Unterstützt ein Werkzeug die Version einer Datei nicht, wird nichts vom RNL dieser Datei interpretiert, ein Fehler wird gemeldet, und die Datei wird beim Speichern unverändert beibehalten; andere Dateien werden wie gewohnt interpretiert.
- Implementierungen müssen die von ihnen unterstützten Versionen deklarieren (Prinzipien 2.3). Die Sprachversion, die Version eines Pakets (`version=` von USE-PACKAGE) und die Version des Hosts sind drei verschiedene Dinge. Wie Paketversionen geschrieben und abgeglichen werden, steht in 16.2. Eine Datei dieser Version verwendet nur Pakete dieser Version: eine Paketdatei oder ein Alias-Paket, das eine andere Sprachversion deklariert, oder keines deklariert und daher in einer früheren gelesen wird, ist ein Fehler an dem USE-PACKAGE oder USE-ALIASES, das sie direkt oder über andere Pakete erreicht (16.2, 16.5).
- **Versionsnummern.** Eine Version wird `"major.minor"` geschrieben: zwei vorzeichenlose Dezimalzahlen, etwa `"1.0"` oder `"1.2"`. Es gibt keine Patch-Nummern, Daten oder Profile. Versionen vor `"1.0"` sind Entwürfe, geschrieben `"0.minor"`; Versionen 0.x geben keine Zusicherung gegenseitiger Kompatibilität, und jede zählt als eigene Hauptversion. Eine fehlerhaft gebildete Version, wie `"1"` oder `"v1.2"`, wird als nicht unterstützte Version behandelt, nicht als fehlende, damit die Datei nicht in einer anderen Version falsch gelesen wird.
- **Eine Datei wird in der Version gelesen, die sie deklariert.** Werkzeuge müssen wissen, in welcher Version jedes Standardwort hinzugekommen ist: in einer Datei, die eine frühere Version deklariert, ist ein später hinzugekommenes Wort kein Standardwort. So beeinträchtigt ein später hinzugekommener Standardunterobjektname keinen gleich geschriebenen Schlüssel in einer älteren Datei (6.5), ein neuer Attributname beeinträchtigt keinen gleichnamigen Host-Parameter (13.6), und ein neuer Standardparameter von AS und ATTACH beeinträchtigt keinen gleichnamigen Definitionsparameter (9.4).
- **Innerhalb einer Hauptversion fügt eine neue Nebenversion nur hinzu und ändert keine Bedeutung.**
  - Erlaubt: Hinzufügen von Schlüsselwörtern, Parametern, Werten, Standardwörtern, Unterobjektnamen und Attributnamen; Verleihen einer Bedeutung an eine Form, die zuvor fehlschlug. Die Punkte in Anhang C und alles, was dieser Standard eine reine Lockerung nennt, sind von dieser Art, und ebenso das Tolerantmachen der Fälle, in denen die gesamte Interpretation fehlschlägt (1.5 Punkt 3).
  - Ohne neue Hauptversion nicht erlaubt: Ändern der Bedeutung einer gültigen Form; Entfernen von etwas; Ändern eines Standardwerts; Verwandeln einer gültigen Form in einen Fehler.
  - Eine alte Datei behält ihre Bedeutung in einem neueren Werkzeug. In einem älteren Werkzeug werden die Teile einer neueren Datei, die die ältere Version kennt, wie gewohnt gelesen, die neuen Teile sind unbekannter Inhalt (16.4), und eine Warnung besagt, dass die Version der Datei neuer ist, als das Werkzeug unterstützt. Das Werkzeug darf sich auch dafür entscheiden, die Datei nicht zu zeichnen, aber es darf nicht vorgeben, sie zu verstehen.
- **Eine andere Hauptversion.** Eine Datei einer Hauptversion, die das Werkzeug nicht unterstützt, wird nicht interpretiert und unverändert beibehalten (siehe oben). Ihre Pakete werden ebenfalls nicht verwendet: kein RNL der Datei wird interpretiert, und ein Viewer zeichnet die Netzliste mit seiner eigenen Standarddarstellung und meldet, dass die Version nicht unterstützt wird. Ein Werkzeug darf mehrere Hauptversionen unterstützen. Der Übergang einer Datei auf eine neue Hauptversion ist eine explizite Migration, bei der das Werkzeug die Datei und ihren Kopfeintrag umschreibt; ein Werkzeug darf eine alte Datei nicht stillschweigend nach den Regeln einer neuen Hauptversion lesen. Jede neue Hauptversion dieses Standards kommt mit Migrationshinweisen.
- **Unterstützte Versionen.** Ein Werkzeug gibt für jede Hauptversion an, bis zu welcher Nebenversion es sie unterstützt, etwa `"1.0"`–`"1.3"`. `"1.3"` zu unterstützen bedeutet, auch `"1.0"` bis `"1.2"` lesen zu können.
- **Erhöhen der Version.** Schreibt ein Werkzeug eine Form in eine Datei, die nur eine neuere Version kennt, erhöht es die Version im Kopfeintrag. Zuvor prüft es, ob die dazwischen hinzugekommenen Standardwörter genauso geschrieben sind wie bereits in der Datei vorhandene Namen (Schlüssel von Texten, Rollennamen, Definitionsparameter, Host-Parameter), und gibt für jeden solchen Namen eine Warnung aus, da das Erhöhen der Version seine Bedeutung ändern kann; die Version wird dennoch erhöht.
- **Versionen 0.3 und 0.2.** Diese Version ersetzt Version 0.3. Dateien der Versionen 0.3 und 0.2 sind Dateien anderer Hauptversionen (oben), die dieser Standard nicht beschreibt. Sie werden mit einem Upgrade-Werkzeug nach 0.4 überführt, das Version 0.3 liest: es erhöht die Version, überführt die Datei in die Standardserie 0.4 und schreibt `@USE-PACKAGE(name="rnl-std-all", version="0.4")` in ihren Kopfeintrag anstelle einer dort festgelegten Version des Standardpakets, schreibt die Datei gemäß Anhang D um und meldet, was es nicht umschreiben kann, etwa eine Anwendung einer Definition, die die Serie 0.4 nicht mehr hat.

### 16.2 Pakete und USE-PACKAGE

```rnl fragment
@USE-PACKAGE(name="acme-style", version="1.2");                               /* looked up in the package directories of the tool */
@USE-PACKAGE(name="acme-style", version="1.2", path="lib/acme-style.rnl");    /* reads this package file */
@USE-PACKAGE(name="acme-style", version="1.2") {…}                            /* an inline package */
```

**Wo ein Paket wirksam wird.**

- Die `$`-Definitionen eines Pakets werden in der Ebene wirksam, in der sein USE-PACKAGE geschrieben steht, sowie in den Ebenen darunter. Seine Regeln zählen als in jener Ebene geschrieben: sie werden dort angewendet, und in den Ebenen darunter nur, wenn sie diese über `recursive=true` in einer darin geschriebenen Auswahl erreichen oder, falls sie keine haben, über die von ihnen verwendeten Mengen (10.1, 10.4). Diese Version hat kein USE-PACKAGE, das nur für seine eigene Ebene gilt. In einer Teilschaltung geschrieben, gilt ein Paket nur für diese Teilschaltung; auf Dateiebene geschrieben, für die Teilschaltungen der Datei und die von ihr eingebundenen Dateien (6.1); im Kopfeintrag geschrieben, für dieselben Ebenen wie auf Dateiebene, jedoch an die Wurzel gestellt (16.3).
- Die Pakete an der Wurzel, ob im Kopfeintrag geschrieben oder durch eine Einstellung des Werkzeugs hinzugefügt (16.3), zählen als auf der obersten Ebene der Datei verwendet: in der Hauptdatei auf deren oberster Ebene. Daher erreichen die Regeln eines im Kopfeintrag verwendeten Hausstilpakets die Teilschaltungen nur mit `recursive=true`, so wie das Standardpaket sie schreibt.
- **Platz in der Reihenfolge.** Der Inhalt eines Pakets wird an die Stelle seines USE-PACKAGE gesetzt, und innerhalb des Pakets in der Reihenfolge seines Texts, genau wie die aus einer Definition expandierten Fakten die Position der AS- oder ATTACH-Anweisung einnehmen. Konfliktauflösung (8.5) und die Darstellungswarteschlangen (8.3) verwenden diese Position. Ein vor den Anweisungen einer Ebene verwendetes Paket wirkt als Satz von Vorgaben; danach verwendet, können seine Regeln frühere Anweisungen überschreiben. Ein USE-PACKAGE im Kopfeintrag wird an die Wurzel gestellt (16.3).

**Namen in einem Paket.** Nur der Platz folgt dem USE-PACKAGE; die Namen eines Pakets bleiben seine eigenen.

- Ein Name in einem Paket wird zuerst unter den Definitionen des Pakets selbst nachgeschlagen, dann in den Paketen, auf die es verweist (siehe „Pakete, die auf Pakete verweisen“ unten), dann in dem auf der Ebene des USE-PACKAGE wirksamen Standardpaket. Er wird nicht unter den eigenen Definitionen des Benutzers dieser Ebene oder der Ebenen darüber nachgeschlagen, und auch nicht in anderen Paketen; wie in einem Definitionsrumpf (6.9) fängt eine gleichnamige Definition auf der Ebene des USE-PACKAGE ihn nicht ein. So werden die Namen eines Inline-Pakets genauso gefunden wie die einer Paketdatei gleichen Inhalts, und sein Text kann in eine solche Datei ausgelagert werden; und eine Paketdatei kann für sich allein geprüft werden, mit dem Standardpaket, das eine Einstellung des Werkzeugs hinzufügt (16.3). Das Standardpaket und die Pakete, auf die es direkt oder indirekt verweist, sehen nur sich selbst und die Pakete, auf die sie verweisen, nicht das Standardpaket, sodass eine Suche niemals zu ihnen zurückkehrt.
- Die `$`-Definitionen eines Pakets sind öffentlich, und sie zählen als zur Ebene unmittelbar oberhalb der Ebene des USE-PACKAGE gehörig: eine gleichnamige Definition in jener Ebene verdeckt die des Pakets und liefert einen Hinweis (6.8); es ist keine zweite Bindung in derselben Ebene. Wenn die `$`-Definitionen eines Pakets Definitionen der Ebenen darüber oder der Pakete an der Wurzel verdecken, wird ebenfalls ein Hinweis gegeben (6.8).
- Die `%`-Definitionen eines Pakets sind privat für das Paket.

**Was ein Paket enthält.**

- Ein Paket enthält Definitionen, `$`-Mengen und Regeln. Der Kopfeintrag einer Paketdatei enthält VERSION und PACKAGE und darf außerdem USE-PACKAGE, DEVICE (6.12) und SOLVER-CONFIG (14.10) enthalten, die für die Ebenen gelten, die das Paket verwenden, sowie ALIAS und USE-ALIASES, die nur für diese Datei gelten (16.5). USE-PACKAGE verweist auf andere Pakete (unten) und darf auch in einem Kopfeintrag für eine Sprache stehen (2.8). Ein Alias-Paket ist eine Paketdatei nur mit Kopfeintrag, geladen mit USE-ALIASES (16.5).
- Ein Paket hat keine eigene Schaltung, wie die Dateiebene einer eingebundenen Datei (6.1):
  - Anweisungen, die bestimmte Host-Objekte benennen (`#M1`, `&vss`, `#:inp`), schlagen fehl; ein alleinstehendes `#` ist weiterhin die Ebene, in der eine Regel ausgewertet wird, auch im Rumpf einer allgemeinen Definition (9.4);
  - eine Anweisung, die keine Regel ist und ein einzelnes Szenenobjekt zeichnet, wie `(0, 0) @RECTANGLE …`, hat keine Ebene, in der sie gezeichnet werden könnte, und ist ein Fehler;
  - `scope=` und `type="generated"` in einem Eintragskopf sind Fehler (2.8);
  - USE-PACKAGE in einem anderen Eintrag eines Pakets als seinem Kopfeintrag ist ein Fehler: Verweise auf andere Pakete werden ausschließlich im Kopfeintrag geschrieben.
- Der Rumpf eines Inline-Pakets folgt denselben Regeln, hat aber keinen Kopfeintrag: Name und Version des Pakets sind die Parameter seines USE-PACKAGE, und DEVICE sowie SOLVER-CONFIG werden im Kopfeintrag der Datei selbst geschrieben. Ein Inline-Paket kann nicht auf andere Pakete verweisen; um eine Paketsammlung in einer Datei einzufrieren, wird ihr expandierter Inhalt der Reihe nach in den Rumpf geschrieben, mit den Änderungen, die seine Bedeutung bewahren (16.3).

**Paketdateien.** Wie eine Paketdatei gelesen wird, steht in 2.7 mit einem Beispiel. Ihr Kopfeintrag deklariert mit `@PACKAGE(name=…, version=…)`, welches Paket und welche Version die Datei ist; eine Datei ohne PACKAGE ist kein Paket, und ein USE-PACKAGE darauf ist ein Fehler. Ein Paket ist eine Datei: ein über mehrere Dateien verteiltes Paket bräuchte eine Reihenfolge zwischen ihnen, die diese Version nicht vorsieht. Eine Paketdatei deklariert ihre Sprachversion mit VERSION, die die Sprachversion der Dateien sein muss, die sie verwenden (16.1); andernfalls ist das USE-PACKAGE ein Fehler, und die Anweisungen, die Namen des Pakets verwenden, schlagen als nicht gefundene Namen fehl.

**Suche.**

- Ein USE-PACKAGE mit einem Rumpf ist ein Inline-Paket und wird nicht gesucht.
- Mit `path=` wird diese Datei gelesen. Der Pfad ist relativ zum Verzeichnis der Datei, in der das USE-PACKAGE geschrieben steht, mit `/` als Trennzeichen; er benennt eine lokale Datei, keine URL. Existiert die Datei nicht oder nennt ihr PACKAGE ein anderes Paket oder eine Version, die die folgende Regel nicht akzeptiert, wird das Paket mit einer Warnung nach Namen gesucht. Existiert die Datei, ist aber kein Paket – ohne PACKAGE oder mit einem falsch geschriebenen `version=` in PACKAGE –, schlägt das USE-PACKAGE fehl, wie bei jeder Datei, die kein Paket ist (oben).
- Andernfalls wird das Paket nach Name und Version seines PACKAGE in den im Werkzeug eingestellten Paketverzeichnissen gesucht. Das Standardpaket wird mit dem Werkzeug ausgeliefert oder in einem Paketverzeichnis installiert. Die Paketverzeichnisse sind eine Einstellung des Werkzeugs und stehen nicht in der Datei; gibt es mehrere Kopien derselben Version, entscheidet die Reihenfolge der Verzeichnisse in dieser Einstellung.
- Diagnosemeldungen geben an, welche Datei und welche Version verwendet wurden.
- Wird das Paket nicht gefunden, schlägt das USE-PACKAGE fehl (ein Fehler), und die Anweisungen, die Namen aus dem Paket verwenden, schlagen mit ihm fehl (7.3); die Diagnosemeldung nennt das fehlende Paket als Ursache.
- `name=` ist erforderlich. Er wird als ganze Zeichenkette verglichen und unterscheidet Groß- und Kleinschreibung; Paketnamen sollten Kleinbuchstaben, Ziffern und `-` verwenden.
- Mit `path=` können eine Netzliste und ihre Paketdateien zusammengehalten und als Ganzes verschoben werden; diese Version hat kein Manifestformat für sie.

**Paketversionen.**

- Eine Paketversion wird wie eine Sprachversion `"major.minor"` geschrieben; jedes 0.x zählt als eigene Hauptversion (16.1).
- Existiert die geschriebene Version, wird sie verwendet. Andernfalls wird die kleinste der neueren Versionen innerhalb derselben Hauptversion verwendet, mit einer Warnung, dass die Version des Pakets abweicht und die Zeichnung abweichen kann; hat die Hauptversion nur ältere Versionen oder keine, wird das Paket nicht gefunden. Eine ältere Version wird nicht verwendet, weil ihr Namen fehlen können. Eine über `path=` benannte Datei wird nach derselben Regel geprüft.
- Ohne `version=` wird die neueste gefundene Version verwendet, mit einem Hinweis. Ein `version=`, das nicht wie in 16.1 angegeben geschrieben ist, in USE-PACKAGE, USE-ALIASES oder PACKAGE, ist ein Fehler und wird nicht gesucht: dieses USE-PACKAGE oder USE-ALIASES schlägt fehl, und eine Paketdatei, deren PACKAGE es enthält, ist kein Paket (oben). Das `version=` eines Inline-Pakets benennt nur die Version und wird nicht gesucht; falsch geschrieben, ist es ebenfalls ein Fehler, und dieses Inline-Paket schlägt fehl.
- Ein nach Name und Version gefundenes Paket wird in einem Dokument nur einmal gelesen: dieselbe Version ist derselbe Satz von Definitionen, wo immer sie verwendet wird. Ein Paket findet die Namen der Pakete, auf die es verweist, und des wirksamen Standardpakets dort, wo es verwendet wird: deren Version und Inline-Inhalt werden wie unten entschieden: durch das der Ebene nächste USE-PACKAGE bei einem Paket, das eine Datei verwendet, und durch die neueste Version auf der Kette der Ebenen bei einem, das nur über andere Pakete eingebracht wird. Weichen diese zwischen zwei Stellen ab, an denen das Paket verwendet wird, wird es für jede einmal gelesen, und die Lesungen sind verschiedene Sätze von Definitionen, wie es zwei Versionen sind (9.5). Jedes Inline-Paket ist eine eigene Kopie: zwei Inline-Pakete sind auch bei gleichem Text verschieden, sodass ein Paket, das sie an zwei Stellen vorfindet, für jede einmal gelesen wird. Gleichnamige Definitionen in verschiedenen Versionen sind zwei Definitionen (9.5); siehe das Beispiel in 16.3.
- Paketautoren sollten der Regel der Sprache folgen (16.1): innerhalb einer Hauptversion bestehende Namen, Rollen, Parameter oder Pins weder entfernen noch ändern; die Zeichnung darf sich ändern. Dies ist eine Konvention, die Werkzeuge nicht prüfen können.
- Diese Version hat keine Inhaltsprüfsumme für Pakete (wie einen Parameter `digest=` von USE-PACKAGE). Um eine Zeichnung exakt gleich zu halten, legt man die Version fest oder schreibt das Paket inline in die Datei. Wie der `hash` eines Eintragskopfs berechnet wird, liegt beim Werkzeug (Kapitel 17).

**Pakete, die auf Pakete verweisen.** Ein Paket kann auf andere Pakete verweisen, sodass ein großes Paket in Teile zerlegt und in mehreren Größen wieder zusammengesetzt werden kann; das Standardpaket ist so aufgebaut (16.3). Ein Paket, dessen Datei nur einen Kopfeintrag hat und nur auf andere Pakete verweist, heißt *Paketsammlung* (*package collection*); dieser Standard nennt es üblicherweise einfach Paket.

- **Form.** Im Kopfeintrag einer Paketdatei schreibt man `@USE-PACKAGE(name=…, version=…)` für jedes referenzierte Paket. Ein `path=` ist relativ zum Verzeichnis jener Paketdatei; Suche und Versionen sind wie oben. Ein Verweis hat keinen Rumpf; einer mit Rumpf ist ein Fehler, weil Inline-Inhalt keine Version hat und nicht an der Wahl der neuesten Version teilnehmen kann (unten).
- **Reihenfolge.** Wird ein Paket verwendet, werden die von seinem Kopfeintrag referenzierten Pakete zuerst in der Reihenfolge der Verweise expandiert und davor gestellt; ein referenziertes Paket, das auf weitere verweist, wird ebenso zuerst expandiert. Dann kommt der Inhalt des Pakets selbst. Innerhalb einer Expansion wird jedes Paket einmal platziert, an seiner ersten Stelle. So ist der Inhalt einer Paketsammlung der Inhalt der Pakete, auf die sie verweist, eines nach dem anderen.
- **Namen.** Die `$`-Namen eines referenzierten Pakets sind in dem Paket sichtbar, das darauf verweist, und werden an dessen Benutzer weitergereicht; wie die `$`-Namen des Pakets selbst zählen sie als zur Ebene unmittelbar oberhalb der Ebene des USE-PACKAGE gehörig. Definieren mehrere referenzierte Pakete `$X`, verdeckt der spätere Verweis den früheren, und eine Definition des Pakets selbst verdeckt sie alle; jedes solche Verdecken liefert einen Hinweis, wie bei mehreren in einer Ebene verwendeten Paketen. Diese Version unterscheidet nicht zwischen Verweisen für den eigenen Gebrauch des Pakets und Verweisen, die an seine Benutzer weitergereicht werden. `%`-Namen bleiben für jedes Paket privat.
- **Nur eines wirksam.** Pakete werden nach Namen gezählt, wie in „Welches USE-PACKAGE wirksam wird“ unten, einschließlich jener, die über andere Pakete eingebracht werden, an der Stelle, an der sie expandiert werden.
  - Bei einem Paket, das eine Datei direkt verwendet, wird das in einer Datei geschriebene USE-PACKAGE wirksam, das der Ebene am nächsten steht, wie in „Welches USE-PACKAGE wirksam wird“ unten, mit seiner Version und seinem Inline-Inhalt, auch dort, wo andere Pakete auf das Paket verweisen. Weicht seine Hauptversion von der ab, die ein darauf verweisendes Paket verlangt, wird eine Warnung ausgegeben: das verweisende Paket könnte Namen verwenden, die es nicht hat.
  - Bei einem Paket, das nur über andere Pakete eingebracht wird, wird die neueste Version innerhalb einer Hauptversion unter allen Verweisen auf einer Kette von Ebenen verwendet, mit einem Hinweis, wenn die Versionen abweichen; innerhalb einer Hauptversion fügt eine neuere Version nur hinzu, sodass sie auch den älteren Verweisen dient. Verweise verschiedener Hauptversionen sind ein Fehler: der später expandierte Verweis schlägt fehl, und die Anweisungen, die Namen daraus verwenden, schlagen mit ihm fehl (7.3).
- **Ersetzen oder Weglassen eines Pakets** folgt aus „nur eines wirksam“; es gibt keine weitere Regel. Ein leeres Inline-Paket desselben Namens lässt es weg, etwa `@USE-PACKAGE(name="rnl-std-fill") {}`, was die automatische Ergänzung ausschaltet; eine andere Version oder anderer Inline-Inhalt unter demselben Namen ersetzt es. Wird ein weggelassenes Paket von anderen Paketen verwendet, schlagen die Anweisungen darin fehl, die seine Namen verwenden.
- **Zyklen.** Verweise, die einen Zyklus bilden, sind ein Fehler, und der Verweis, der den Zyklus schließt, schlägt fehl. Welcher Verweis das ist, hängt von der Stelle ab, an der die Expansion beginnt; werden mehrere Pakete eines Zyklus direkt verwendet, wird der Zyklus für jedes von ihnen einmal gemeldet.
- **Wo Diagnosemeldungen gemeldet werden.** Der Hinweis über abweichende Versionen und die Warnung über eine andere Hauptversion werden am in der Datei geschriebenen USE-PACKAGE gemeldet (Hinweise und Warnungen innerhalb von Paketen werden dem Benutzer nicht gezeigt, 7.6). Der Fehler eines fehlschlagenden Verweises wird an diesem Verweis gemeldet und gibt an, welches USE-PACKAGE in der Datei ihn eingebracht hat.
- Ein Alias-Paket kann nicht referenziert werden (16.5). USE-ALIASES im Kopfeintrag einer Paketdatei gilt wie gewohnt nur für diese Datei.

**Derselbe `$`-Name in mehreren Paketen.**

- Definieren mehrere in einer Ebene verwendete Pakete `$X`, verdeckt das später verwendete das frühere, mit einem Hinweis.
- Ein Name wird in dieser Reihenfolge nachgeschlagen: diese Ebene → die in dieser Ebene verwendeten Pakete (das spätere zuerst) → die Ebene darüber → die in der Ebene darüber verwendeten Pakete → … → die Pakete an der Wurzel (das später platzierte zuerst).
- Diese Version hat keine paketqualifizierten Namen, andere Namen für Pakete oder Ebenen-Tags (wie `$acme.DIFFERENTIAL_PAIR` oder `as=`). Hinweis: eine spätere Version, die sie hinzufügt, benötigt eine andere Form als ein `.` nach dem Namen, der am Ende eines Satzes üblich ist (`align #M1 and #M2.`).

**Welches USE-PACKAGE wirksam wird.** Auf der Kette der Ebenen von der Wurzel hinab bis zu einer Ebene ist ein Paket nur einmal wirksam. Das USE-PACKAGE, das in der dieser Ebene nächsten Ebene geschrieben steht, wird wirksam, mit seiner Version und seinem Inline-Inhalt; innerhalb einer Ebene zählt das spätere als näher, und eines im Kopfeintrag zählt als ganz am Anfang der Dateiebene geschrieben. Ein über andere Pakete eingebrachtes Paket zählt als an dem USE-PACKAGE geschrieben, das es einbringt, mit seiner Version wie in „Pakete, die auf Pakete verweisen“ oben gewählt. Das Paket wird an der Position dieses USE-PACKAGE platziert, oder an der Wurzel, wenn dieses USE-PACKAGE im Kopfeintrag steht.

Hinweis: Andernfalls würden die Generierungsregeln eines Pakets, wie die Masse-Ergänzung, in einer Ebene darunter erneut laufen und ihre Objekte doppelt erzeugen.

**Erneutes Verwenden eines Pakets.** Eine Teilschaltung oder eine Bibliotheksdatei darf das Standardpaket auf ihrer Dateiebene oder in einer Teilschaltung erneut verwenden. Die Darstellungsregeln des Standardpakets werden dann nach den von den oberen Ebenen herabfließenden Regeln (etwa denen der Hauptdatei) platziert und überschreiben diese erneut, und die `$`-Namen des Pakets, die als zur Ebene unmittelbar über diesem USE-PACKAGE gehörig zählen, verdecken die gleichnamigen Definitionen aus den Ebenen darüber. Die in dieser Ebene verwendeten Symbole werden dann von den Darstellungsregeln der Ebenen darüber nicht beeinflusst (6.1).

- Dies betrifft nur Symbole und Namen. Die von den Ebenen darüber herabfließenden geometrischen und generierenden Regeln gelten weiterhin; veraltet eine Lösung ihretwegen, wird ein Hinweis ausgegeben, erneut zu lösen (Kapitel 17).
- Um nur die Version festzulegen und dennoch den Stil der Ebenen darüber das Paket überschreiben zu lassen, schreibt man das USE-PACKAGE in den Kopfeintrag (16.3).

**Inline-Pakete.** USE-PACKAGE darf einen Rumpf haben, der der Inhalt des Pakets ist. Die Datei trägt das Paket dann mit sich, und es wird nichts gesucht:

```rnl
@USE-PACKAGE(name="acme-style", version="1.2") {
  $NMOS_ACME = @DEFINE-SYMBOL {$NMOS; (-1, 0) @CIRCLE(radius=2.5)};
  # @SELECT-INSTANCE(type="nmos", recursive=true) => @SET-SYMBOL $NMOS_ACME
}
```

- Ein Inline-Paket bedeutet dasselbe wie ein USE-PACKAGE ohne Rumpf: Name und Version des Pakets, sein Platz am USE-PACKAGE, das Nachschlagen seiner Namen wie in „Namen in einem Paket“, das Zählen seiner `$`-Namen als Ebene darüber, seine privaten `%`-Namen und „nur eines wirksam“ nach Paketname gezählt gelten wie oben. Der Rumpf wird durch Umschließung aufgenommen (5.4). Ein Inline-Paket hat die Sprachversion der Datei, in der es geschrieben steht (16.1).
- Ein Inline-Paket unterscheidet sich von einer Paketdatei nur darin, dass sein Inhalt in der Netzliste geschrieben steht und dass es keinen Kopfeintrag hat: seine Sprachversion ist die der Datei, sein `version=` benennt nur die Version, es kann kein DEVICE, SOLVER-CONFIG oder ALIAS deklarieren, und es kann nicht auf andere Pakete verweisen. Jedes Inline-Paket ist eine eigene Kopie. Das Auslagern eines Inline-Pakets bewahrt daher, wie seine Namen gefunden werden, aber nicht immer, welche Definitionen welche sind: wo bereits eine Paketdatei desselben Namens und derselben Version verwendet wird, werden beide zu einem Satz von Definitionen (oben), und Regeln, die nach Definition auswählen, wie SELECT-USAGE (9.5), können dann mehr Anwendungen erreichen.
- Inline-Pakete werden normalerweise nur für zweierlei verwendet: um ein Paket mit einem leeren Inline-Paket desselben Namens wegzulassen (16.3), und um eine ganze Menge von Paketen in der Datei einzufrieren, mit einem Rumpf, der ihren vollständigen expandierten Inhalt enthält und in sich abgeschlossen ist.
- Um das Standardpaket zu erweitern oder einen Teil davon zu ersetzen, schreibt man eine Paketdatei, deren Kopfeintrag auf die verwendeten Standardpakete verweist (siehe „Pakete, die auf Pakete verweisen“ oben), und verweist mit `path=` aus der Netzliste darauf. Ein Inline-Paket mit Inhalt, der ein Unterpaket des Standardpakets ersetzt, zählt als Teil des Standardpakets und sieht nur sich selbst: es kann keine Namen aus den anderen Unterpaketen des Standardpakets verwenden.

**Weitere Regeln.**

- Das Verarbeiten eines Pakets darf weder Code ausführen noch implizit auf das Netzwerk zugreifen (Prinzipien 2.7).
- Wie ein Viewer eine Zeichnung darstellt und ob er die Darstellung nach den Voreinstellungen des Benutzers ändern kann (etwa dunkler Hintergrund oder Drucken), liegt außerhalb dieses Standards. Eine Eigenschaft, der weder ein Stil noch ein Parameter einen Wert gibt, übernimmt die Vorgabe des Renderers (9.3, Prinzipien 2.7).

### 16.3 Das Standardpaket

- Das Standardpaket ist nicht Teil dieses Standards. Es ist wie jedes andere Paket in RNL geschrieben und wird gesondert veröffentlicht; seine semantischen Definitionen, virtuellen Bauteile und sein übriger Inhalt werden durch das Paket selbst definiert.
- Die Namen im Standardpaket haben kein vorgeschriebenes Präfix, etwa `$DIFFERENTIAL_PAIR_N`. Ein Benutzer darf eine gleichnamige Definition schreiben, die die standardmäßige durch Verdecken ersetzt (6.8).
- **Eine Paketsammlung.** Das Standardpaket `rnl-std-all` hat nur Kopfeinträge: es ist die einzige Sammlung der Serie und verweist auf jedes andere Paket derselben außer den beiden unten genannten optionalen. Eine Datei verwendet die gesamte Sammlung oder schreibt die gewünschten Pakete einzeln.
  - `rnl-std-style` (Stile und die Standard-Stilregeln), `rnl-std-basic` und `rnl-std-semiconductors` (die Symbole und Definitionen, einschließlich der virtuellen Bauteile wie Massen, Versorgungen und Port-Markierungen sowie der Mengen der Masse-, Versorgungs- und Bias-Netze) und die Regelpakete der beiden Dialekte, die Instanzen ihre Symbole geben: `rnl-std-spice` nach Elementbuchstabe und Modelltyp, `rnl-std-spectre` nach Master und Modelltyp (siehe die Dialekte unten). Diese zeichnen die Netzliste und fügen nichts hinzu. Der Kopfeintrag von `rnl-std-basic` enthält `@SOLVER-CONFIG(junction="$JUNCTION")`, damit der Solver seine Knotenpunkte mit der Standarddefinition zeichnet (13.2, 14.10).
  - `rnl-std-fill` enthält die Regeln, die unverdrahteten Pins Massen, Versorgungen, Bias-Markierungen und Netzbeschriftungen hinzufügen, jeweils auf einem Stub (10.5); es verweist auf `rnl-std-basic`. Es wählt Pins aus, sodass auch die Pins ergänzt werden, die virtuelle Bauteile auf einem Netz haben, außer jenen der Bauteile, die es selbst platziert. In der Serie 0.4 gibt es jedem Portnetz in `$PORT_FILL_SET` eine Port-Markierung, in einer Spalte für jede Ebene (12.4); kein Netz wird als Leitungen gezeichnet, sofern nicht eine Datei oder ein Paket dies mit ROUTE-NET oder ROUTE-OBJECT angibt, die Masse-, Versorgungs- und Bias-Netze werden mit `@UNROUTE-NET(priority="high")` ausgelassen, und eine Datei, die sie als Leitungen gezeichnet haben möchte, schreibt `@ROUTE-NET(priority="high")` oder WIREs (13.7). Pins mit Leitungen, von Hand geschrieben oder durch ROUTE-NET oder ROUTE-OBJECT gezeichnet, als Anweisungen oder als Regeln geschrieben, erhalten von der Ergänzung kein Masse- oder Versorgungssymbol; eine Datei, die eines wünscht, hängt es an (10.5). Die Regeln der Route-Schlüsselwörter werden vor der Ergänzung ausgewertet, die die von ihnen erzeugten Bäume sieht (8.2, 10.2). Die Mengen `$GROUND_FILL_SET`, `$SUPPLY_FILL_SET`, `$BIAS_FILL_SET`, `$LABEL_FILL_SET` und `$PORT_FILL_SET` enthalten, was die Ergänzung noch hinzufügen soll: sobald ein Lösevorgang seine Stubs und Markierungen zurückschreibt (14.8), enthalten sie diese Pins und Netze nicht mehr, daher baut eine Datei keine andere Regel darauf auf, etwa ein TEXT auf ihren Pins.
  - `rnl-std-semantics` enthält die Definitionen von Schaltungsstrukturen, etwa des Differenzpaars, des Stromspiegels sowie von Signal- und Leistungsflüssen, und von Layouts, wie `$MIRRORED_LAYOUT`; die Bedingungen in ihren Rümpfen haben alle die Priorität `"low"` (9.5, 14.8). Eine Definition, die nur platziert, ohne Zusicherung und ohne Routing, sagt nichts über die Schaltung aus, und ihr Name endet auf `_LAYOUT`, wie `$MIRRORED_LAYOUT`. `_N` und `_P` am Ende eines Strukturnamens, wie in `$DIFFERENTIAL_PAIR_N` und `$CURRENT_MIRROR_P`, geben die Polarität seiner Transistoren an, die die Struktur für jene mit bekanntem Typ zusichert (15.3), sowie die zugehörige Zeichnung: Sources oder Emitter nach unten beim n-Typ und nach oben beim p-Typ.
  - Die Pakete einzelner Fachgebiete sind `rnl-std-functional` (Verstärker, Komparatoren, Wandler und Ähnliches), `rnl-std-rf`, `rnl-std-logic` und `rnl-std-signal-flow`.
  - Die Namen in den Paketen wiederholen sich nicht, sodass ein Benutzer nicht wissen muss, aus welchem Paket ein Name stammt.
  - Das optionale `rnl-std-45deg` enthält unter 45 Grad gezeichnete Symbole, etwa für Brücken, kreuzgekoppelte Paare und diagonale Wicklungen; es ist nicht in der Sammlung enthalten, und eine Datei, die es wünscht, schreibt sein USE-PACKAGE.
  - Das optionale `rnl-std-no-labels` schaltet die Standardbeschriftungen der Standardsymbole aus, für Dateien, die alle ihre Beschriftungen selbst schreiben; es ist nicht in der Sammlung enthalten. In der Serie 0.4 wählt es ausschließlich gezeichnete Objekte aus (`drawn=true`, 10.1) und gibt ihren Namens-, Wert- und Modellbeschriftungen sowie den Netznamen, die die Symbole von Anwendungen zeichnen, den Stil `textvisible=false` (9.3); eine Datei zeigt eine davon wieder an, indem sie `textvisible=true` darauf schreibt (13.3). Das optionale `rnl-std-no-instance-labels` tut dasselbe nur für gezeichnete Instanzen, sodass die Beschriftungen von Anwendungen bleiben, etwa die Netznamen von Massen, Versorgungen, Port-Markierungen und Netzbeschriftungen; es ist ebenfalls nicht in der Sammlung enthalten. Weitere Pakete dürfen später hinzukommen. Die gesamte Standardserie wird gemeinsam unter einer Versionsnummer veröffentlicht, und die Sammlung legt die Versionen ihrer Pakete fest.
  - Dies ist der Aufbau der Standardserie 0.4; ihr `rnl-std-all` hat einen Kopfeintrag für jede Sprache (2.8).
- **Kein implizites Laden.** Eine Datei der Version 0.4 verwendet das Standardpaket `rnl-std-all` nur dann, wenn ein USE-PACKAGE es schreibt, wie jedes andere Paket (16.2). Ein Werkzeug darf eine Einstellung anbieten, die es hinzufügt, wenn der Kopfeintrag der Hauptdatei `rnl-std-all` in keiner Form schreibt, etwa eine Option `--include-std` auf der Befehlszeile oder eine Einstellung eines Viewers, die standardmäßig eingeschaltet sein darf. Ist die Einstellung eingeschaltet, liest das Werkzeug die Datei so, als stünde in ihrem Kopfeintrag ohne `language` zuerst `@USE-PACKAGE(name="rnl-std-all", version="…")` mit der neuesten im Werkzeug vorhandenen Version der Standardserie in der Sprachversion der Datei, und wenn es in die Hauptdatei schreibt, schreibt es dieses USE-PACKAGE in deren Kopfeintrag, sodass die Datei von jedem Werkzeug gleich gezeichnet wird. Eine im Kopfeintrag einer Datei festgelegte Version wird für diese Datei und die von ihr eingebundenen Dateien verwendet (unten).
  - Hinweis: Das Standardpaket ist ein Paket wie jedes andere. Eine Netzliste, die nicht für RNL geschrieben wurde, wird nur durch eine solche Einstellung mit Standardsymbolen gezeichnet, und eine Datei, die Definitionen des Standardpakets verwendet, wie `$GROUND`, schreibt das USE-PACKAGE, oder ihre Namen werden nicht gefunden. Jeder nicht gefundene Name ist wie üblich ein Fehler; definiert ein nicht verwendetes Paket der Standardserie einen solchen Namen, weist ein Hinweis einmal für die Datei darauf hin, nennt jenes Paket und verweist auf USE-PACKAGE und auf die Einstellung des Werkzeugs.
- **Das Regelpaket des anderen Dialekts.** Die Namen von Instanzen bedeuten in den beiden Dialekten Verschiedenes: in SPICE ist `I0` eine Stromquelle; in Spectre sagt der Name einer Instanz nichts aus, und Virtuoso benennt die Instanz einer Teilschaltung `I0`. Die Standardserie 0.4 drückt dies mit `language` aus: ihre Sammlung `rnl-std-all` hat einen Kopfeintrag ohne `language` für das beiden Sprachen Gemeinsame, einen mit `language="spice"`, der `rnl-std-spice` verwendet, und einen mit `language="spectre"`, der `rnl-std-spectre` verwendet (2.8). So findet in einer Spectre-Datei ein SPICE-Abschnitt seine Symbole nach den Regeln von SPICE und die Spectre-Abschnitte nach jenen von Spectre.
  - Ein Werkzeug, das die Serie 0.4 einfriert, schreibt den Teil jeder Sprache als Inline-Paket in den Kopfeintrag jener Sprache und den gemeinsamen Teil in den Kopfeintrag ohne `language`: ein Inline-Paket enthält Anweisungen, keine Einträge, und kann selbst kein `language` tragen.
- **Pakete an der Wurzel.** Der Kopfeintrag einer Hauptdatei oder einer eingebundenen Datei, darunter auch eine Verilog-A-Datei (2.9), darf USE-PACKAGE enthalten:

  ```rnl
  * @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
  ```

  - Sein Geltungsbereich ist der eines USE-PACKAGE auf Dateiebene: die Datei, die es schreibt, und die Dateien, die sie einbindet. Eines in einer eingebundenen Datei gilt für diese Datei und die Dateien, die diese ihrerseits einbindet, nicht für die Datei, die sie einbindet (6.1).
  - Es wird an die Wurzel gestellt: vor die Regeln aller Ebenen und auch vor Blocksymbole (8.3, Gruppe 1). Seine `$`-Namen stehen ebenfalls an der Wurzel, sodass eine gleichnamige Definition in einer Ebene, etwa in der Hauptdatei, sie weiterhin verdeckt. Ein USE-PACKAGE auf Dateiebene wird stattdessen an seiner eigenen Position platziert: es verwendet das Paket erneut (16.2), sodass es die herabfließenden Regeln überschreibt und seine `$`-Namen als zur Ebene darüber gehörig zählen. Im Kopfeintrag geschrieben, besagt ein Paket: „diese Datei hängt von dieser Version ab, und der Stil der Ebenen darüber darf sie weiterhin überschreiben“; auf Dateiebene geschrieben, besagt es: „diese Datei verwendet das Paket, um ihre eigene Zeichnung festzulegen“.
  - Welches USE-PACKAGE wirksam wird, richtet sich nach 16.2: das in der dieser Ebene nächsten Ebene geschriebene, wobei eines im Kopfeintrag als ganz am Anfang der Dateiebene geschrieben zählt.
  - Die Reihenfolge der Pakete an der Wurzel: zuerst das Standardpaket `rnl-std-all`, wo immer ein Kopfeintrag es schreibt und auch wenn eine Einstellung des Werkzeugs es hinzufügt, dann die übrigen in der Reihenfolge der ersten Datei, die jedes schreibt (von der äußersten Datei nach innen), und ihres Kopfeintrags. Schreibt der Kopfeintrag einer inneren Datei dasselbe Paket erneut, ändert dies nur die in den Ebenen dieser Datei verwendete Version oder den Inline-Inhalt; der Platz bleibt erhalten, sodass das Festlegen einer Version die Reihenfolge nicht ändert.
  - Ein Paket in einem Kopfeintrag, das einen `$`-Namen des Standardpakets oder eines vor ihm an die Wurzel gestellten Pakets definiert, verdeckt diese Definition und liefert einen Hinweis (6.8).
  - Das Festlegen einer Version ist Entscheidung des Verfassers. Eine Hauptdatei sollte sie festlegen. Eine eingebundene Datei, die sie nicht festlegt, folgt der Datei, die sie einbindet; sie sollte eine Version nur festlegen, wenn sie von einer abhängt. Werkzeuge dürfen eingebundenen Dateien von sich aus keine Versionen hinzufügen, außer der Festlegung, die das Hochstufen einer Datei älterer Version schreibt (16.1).
  - Eine Version sollte nicht mit einem USE-PACKAGE auf Dateiebene der Hauptdatei festgelegt werden: dort wird das Paket nach den Blocksymbolen platziert, und wenn die Datei anderswo eingebunden wird, verwendet das USE-PACKAGE das Paket erneut (16.2) und ändert die Bedeutung der Datei.
- **Beispiel.** Der Kopfeintrag von `tb.sp` schreibt `@USE-PACKAGE(name="rnl-std-all", version="1.2")`, und der von `ota.sp`, das `tb.sp` einbindet, schreibt Version `"1.0"`.
  - Die oberste Ebene von `tb.sp` und die in `tb.sp` definierten Teilschaltungen verwenden `"1.2"`; die Teilschaltungen in `ota.sp` verwenden `"1.0"`. Die Stilregeln von `tb.sp` fließen weiterhin nach `ota.sp` und überschreiben die Vorgaben von `"1.0"`.
  - Standardwahlen treffen Regeln im Standardpaket, etwa die Regel, die den Anwendungen von `$GROUND` ein Massesymbol gibt. Jede Version wählt in den Ebenen, die sie verwenden, ihr eigenes `$GROUND` aus, sodass `ota.sp` wie gewohnt gezeichnet wird.
  - Gleichnamige Definitionen in den beiden Versionen sind zwei Definitionen (16.2). Daher findet eine Regel aus `tb.sp`, die nach Definition auswählt, etwa `# @SELECT-USAGE(recursive=true) $GROUND => @SET-SYMBOL $MY_GND`, nur die Anwendungen des `$GROUND` von `"1.2"`; `ota.sp` folgt ihr nur, wenn es keine eigene Version festlegt. Regeln, die nach Eigenschaften des Hosts auswählen, etwa `# @SELECT-INSTANCE(type="nmos", recursive=true) => …`, sind nicht betroffen.
  - Damit seine Symbole frei von den Regeln von `tb.sp` sind, verwendet `ota.sp` das Paket auf seiner Dateiebene (16.2).
- **Weniger vom Standardpaket.** Eine Einstellung des Werkzeugs fügt ausschließlich den Paketnamen `rnl-std-all` hinzu.
  - Um einige Pakete wegzulassen, lässt man sie mit leeren Inline-Paketen desselben Namens weg: `@USE-PACKAGE(name="rnl-std-fill") {}` schaltet die automatische Ergänzung aus. Ein Paket desselben Namens ist nur einmal wirksam, niemals zweimal.
  - Um nur einige wenige zu verwenden, schreibt man sie einzeln, mit einem leeren `rnl-std-all`, damit eine Einstellung des Werkzeugs nicht zusätzlich die Sammlung hinzufügt:

  ```rnl
  * @RNL(type="header") {
  *   @VERSION("0.4");
  *   @USE-PACKAGE(name="rnl-std-all") {};
  *   @USE-PACKAGE(name="rnl-std-style", version="0.4");
  *   @USE-PACKAGE(name="rnl-std-basic", version="0.4");
  *   @USE-PACKAGE(name="rnl-std-spice", version="0.4")
  * }
  ```

- **Erweitern des Standardpakets oder Ersetzen eines Teils davon.** Man schreibt eine Paketdatei, deren Kopfeintrag auf die verwendeten Standardpakete verweist, und fügt ihren eigenen Inhalt hinzu (16.2, Inline-Pakete). Definitionen und Regeln, die das Standardpaket lediglich ergänzen, dürfen auch direkt in die Netzliste geschrieben werden; Regeln, die das Standardpaket enthält, können nur durch ein Paket desselben Namens entfernt oder ersetzt werden.
- **Einfrieren oder Weglassen des gesamten Standardpakets.** Man schreibt ein Inline-Paket desselben Namens in den Kopfeintrag. Dies folgt aus „nur eines wirksam“, nach Paketname gezählt; es gibt keine weitere Regel:
  - `@USE-PACKAGE(name="rnl-std-all", version="1.2") {…}` friert das Standardpaket in die Datei ein und macht die Datei eigenständig; der Rumpf ist der Inhalt seiner Pakete nach der Expansion, eines nach dem anderen in der Reihenfolge der Expansion (16.2). In einem Rumpf zusammengefügt, können zwei Pakete kollidieren: ein privater `%`-Name, der in beiden verwendet wird, oder ein `$`-Name, den eines vor dem anderen verdeckt. Ein Werkzeug, das einfriert, benennt um, was kollidiert, sodass jeder Verweis weiterhin findet, was er in seinem eigenen Paket fand, und schreibt die DEVICE- und SOLVER-CONFIG-Anweisungen der Kopfeinträge der eingefrorenen Pakete in die Kopfeinträge der Datei, in ihrer Reihenfolge und vor den eigenen der Datei (2.8, 6.12, 14.10), da ein Inline-Paket sie nicht enthalten kann: jede in den Kopfeintrag für die Sprache, über die ihr Paket verwendet wird, oder in jenen ohne `language` für ein ohne eine solche verwendetes Paket; das SOLVER-CONFIG eines nur über Kopfeinträge mit `language` verwendeten Pakets wird nicht kopiert, da es keine Wirkung hat (14.10). Die Kopien kommen dann nach den Paketen, die nach dem eingefrorenen kamen. Wo ein solches Paket einen SOLVER-CONFIG-Schlüssel schreibt, den eine Kopie schreibt, oder ein DEVICE-Element (Anschlussnamen oder Typ) für Instanzen, auf die auch ein kopiertes DEVICE passt, schreibt das Werkzeug diese Anweisung in den Kopfeintrag, der die kollidierende Kopie enthält, nach ihr und vor den eigenen der Datei, wenn das Paket in einem Kopfeintrag derselben Datei verwendet wird, und meldet die Kollision, wenn es in einer eingebundenen Datei oder in einer Ebene verwendet wird. Abgesehen von den gemeldeten Kollisionen bedeutet das eingefrorene Paket dasselbe;
  - `@USE-PACKAGE(name="rnl-std-all") {}` verwendet kein Standardpaket, auch wenn eine Einstellung des Werkzeugs eines hinzufügen würde; jede Instanz hat dann das Standard-Blocksymbol (9.2.6), und das übrige Aussehen bleibt den Vorgaben des Renderers überlassen (Prinzipien 2.7).
- Konventionen wie fette Busse, Abzweigsymbole für Busse und Markierungen für die Breite eines Busses sind ebenfalls Inhalt des Standardpakets. Ohne Standardpaket hat jede Instanz das Standard-Blocksymbol (9.2.6), und der Renderer entscheidet über den Zeichenstil (Prinzipien 2.7).

### 16.4 Erweiterungen und unbekannter Inhalt

- **Unbekannter Inhalt** (Prinzipien 2.3). Unbekannte Schlüsselwörter, Parameter und Werte sind Fehler: die Anweisung schlägt fehl, die von ihr abhängenden Anweisungen schlagen mit ihr fehl (5.5, 7.3), und der Rest wird wie gewohnt interpretiert (7.6). Unbekannte Felder eines Eintragskopfs sind die Ausnahme und liefern nur eine Warnung (2.8). In SOLVER-CONFIG liefert ein dem Solver unbekannter Schlüssel eine Warnung, und ein Wert, der nicht angenommen wird, lässt nur seinen Schlüssel fehlschlagen (14.10). Die Zeichnung kann dann unvollständig sein, und die Diagnosemeldungen müssen dies angeben.
- **Beim Speichern beibehalten.** Prinzipien 2.3 empfiehlt, solchen Inhalt unverändert zu behalten; dieser Standard verlangt es:
  - Einträge, die nicht die eigenen des Werkzeugs sind, werden nicht geändert (Kapitel 17), und der Host-Text sowie gewöhnliche Kommentare bleiben unverändert;
  - schreibt ein Werkzeug seine eigenen Einträge neu, werden die darin enthaltenen Anweisungen, die es nicht kennt oder die fehlschlagen, mit ihrem Freitext in derselben Reihenfolge beibehalten;
  - eine Datei, deren Version nicht unterstützt wird, wird vollständig beibehalten (16.1);
  - die Einträge, die ein Werkzeug neu schreibt, müssen nicht Byte für Byte gleich bleiben.
- **Kein Erweiterungsmechanismus in dieser Version.** Neue Semantiken, virtuelle Bauteile und Zeichenweisen werden als Definitionen, Symbole und Stile in Paketen geschrieben (1.5 Punkt 7), nicht als neue Schlüsselwörter; ein neuer Mechanismus, den Implementierungen verstehen müssen, wartet auf eine neue Version dieses Standards. Werkzeugeigene Daten, wie Sperren oder der Ansichtsstatus eines Editors, werden nicht in RNL-Einträge geschrieben; sie gehören in Dateien des Werkzeugs oder in gewöhnliche Kommentare außerhalb von Einträgen.
  - Hinweis: Diese Version sorgt zunächst dafür, dass alle Werkzeuge dasselbe RNL lesen; herstellerspezifische Dialekte zu diesem frühen Zeitpunkt würden die Sprache aufspalten, und neue Semantiken lassen sich bereits in Paketen schreiben.
  - Hinweis: Eine spätere Version kann Erweiterungen als Schlüsselwörter mit einem Herstellerpräfix und einem `.` hinzufügen, etwa `@ACME.ROUTE`, deklariert im Kopfeintrag: eine erforderliche Erweiterung, die ein Werkzeug nicht kennt, ließe die gesamte Datei uninterpretiert, und eine optionale würde mit einem Hinweis übersprungen. Ein solches Schlüsselwort ist jetzt ein unbekanntes Schlüsselwort, ein Fehler, sodass das spätere Hinzufügen eine reine Lockerung ist (16.1); ein Herstellerpräfix dürfte nicht den Namen eines Standardschlüsselworts haben.
- Keine Form ist für Erweiterungen reserviert: ein `.` direkt nach einem Standardschlüsselwort ist Freitext, sodass `@BUNDLE.LOCK` ein BUNDLE gefolgt vom Freitext `.LOCK` ist und `{#M1 #M2} @GROUP.` am Ende eines Satzes gültig ist (3.5).
- **Erkannt, aber nicht interpretiert.** In dieser Version ist Inhalt dieser Art ein unbekanntes Feld eines Eintragskopfs (2.8) und ein Schlüssel von SOLVER-CONFIG, den der Solver nicht kennt (14.10). Optionale Erweiterungen wären eine zweite Art, falls eine spätere Version Erweiterungen hinzufügt.

### 16.5 Aliase

```rnl
* @RNL(type="header") {
*   @VERSION("0.4");
*   @USE-PACKAGE(name="rnl-std-all", version="0.4");
*   @USE-ALIASES(name="alias-zh");
*   @ALIAS("@at", "@PLACE")
* }
* @RNL {#M1 @位于(10, 20); {#M1 #M2} @对齐("y"); #M3 @at(x=4); #M1(左管) #M2(右管) #M5(尾管) @视作 $"差分对"}
```

- Aliase geben Standardwörtern andere Schreibweisen, zur Verträglichkeit mit einer anderen Groß-/Kleinschreibung oder zum Schreiben von RNL im Stil einer anderen natürlichen Sprache, etwa Chinesisch.
- **Woher Aliase stammen:**
  - direkt im Kopfeintrag geschriebene ALIAS-Anweisungen, für einige eigene Aliase;
  - Alias-Pakete, geladen mit `@USE-ALIASES(name=…, version=…, path=…)` im Kopfeintrag. Die Aliase einer Sprache werden in einem Paket zusammengefasst, damit Dateien sie nicht wiederholen müssen und der Wortlaut überall gleich bleibt. Das chinesische Standard-Alias-Paket ist `alias-zh`; es wird zusammen mit dem Standardpaket veröffentlicht und mit den Werkzeugen ausgeliefert (16.3).
  - Beides darf zusammen verwendet werden, und mehrere Alias-Pakete dürfen geladen werden; zusammen werden sie nach den unten stehenden Zuordnungsregeln geprüft.
  - Aliase gelten ausschließlich für die Datei, die sie deklariert, und werden nicht über include vererbt; eine eingebundene Datei oder eine Paketdatei, die Aliase verwendet, deklariert sie in ihrem eigenen Kopfeintrag. Dasselbe Standardwort darf in verschiedenen Dateien verschiedene Aliase haben. Die Schlüssel der in einem Symbol geschriebenen Texte werden gegen die Aliase der Datei geprüft, in der das Symbol geschrieben steht (6.5, 13.3).
  - Der Kopfeintrag selbst wird stets in der Standardschreibweise gelesen und erkennt keine Aliase: bis USE-ALIASES und ALIAS gelesen sind, sind die Aliase nicht bekannt, und Kopfeinträge werden meist von Werkzeugen geschrieben. Daher sind Aliase der Schlüsselwörter, die nur im Kopfeintrag geschrieben werden dürfen (VERSION, ALIAS, USE-ALIASES, DEVICE, PACKAGE), ohne Nutzen.
- **Alias-Pakete:**
  - Ein Alias-Paket ist eine Paketdatei (16.2), deren Kopfeintrag ausschließlich VERSION, PACKAGE und eine Liste von ALIAS-Anweisungen enthält und die keine weiteren Einträge hat. Es wird gesucht und seine Versionen werden gewählt wie in 16.2.
  - USE-ALIASES auf einer Datei, die kein Alias-Paket ist, oder USE-PACKAGE auf einem Alias-Paket ist ein Fehler. Wird ein Alias-Paket nicht gefunden, ist sein USE-ALIASES ein Fehler, und die Anweisungen, die seine Aliase verwenden, schlagen mit ihm fehl (7.3); die Diagnosemeldung nennt das fehlende Alias-Paket als Ursache.
  - Hinweis: Alias-Pakete werden nicht mit USE-PACKAGE geladen, weil USE-PACKAGE in jeder Ebene stehen darf, sich mit Definitionen und Regeln befasst und nach Position geordnet wird, während Aliase bestimmen, wie der Text einer Datei gelesen wird, und nur je Datei gelten können. Durch diese Trennung kann ein Hausstilpaket nicht verändern, wie der Text gelesen wird.
  - Der Inhalt eines Alias-Pakets folgt dem, worauf es verweist: ändert sich ein Name im Standardpaket, ändert sich `alias-zh` mit ihm und seine Version wird erhöht.
- **Form.** `@ALIAS("alias", "original")`: der Alias zuerst. Die beiden Parameter sind `alias` und `original`, üblicherweise nach Position angegeben; der dritte Parameter, `definition`, wird ausschließlich mit Namen angegeben (unten).
- **Originale:**
  - Ein von diesem Standard definiertes Schlüsselwort oder Standardwort in seiner Standardschreibweise. Ein Schlüsselwort wird mit seinem `@` geschrieben, wie `"@PLACE"`; andere Wörter ohne dieses, wie `"axis"` oder `"low"`. Der Alias wird ebenso geschrieben: der Alias eines Schlüsselworts hat `@`, andere Aliase nicht. Das `@` unterscheidet ein Schlüsselwort von einem gleich geschriebenen Standardwort, und ein Alias erscheint im Text exakt wie deklariert.
  - Ein `$`-Name. Sowohl der Alias als auch das Original tragen das `$`, etwa `@ALIAS("$接地补位集", "$GROUND_FILL_SET")`; im Text wird er `$"接地补位集"` geschrieben (ein Nicht-ASCII-Name nach `$` erfordert Anführungszeichen, 3.6) und als `$GROUND_FILL_SET` gelesen. `%`-Namen haben keine Aliase: Verfasser können eigene Namen direkt in einer anderen Sprache vergeben, und die `%`-Namen in Paketen sind privat.
  - Ein Name in einer Definition, deklariert mit `definition=`, wie `@ALIAS("左管", "left", definition="$DIFFERENTIAL_PAIR_N")` oder `@ALIAS("间距", "gap", definition="$GROUND_STUB")`. Ein solcher Alias wird nur innerhalb jener Definition nachgeschlagen: Rollennamen bei ihrer Anwendung (`#M1(左管) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N`), Namen von Parametern, die ihr über AS und ATTACH übergeben werden (`$GROUND_STUB @ATTACH(间距=3) …`), und lokale Namen, die über `::` von ihren Anwendungen genommen werden (`%c1::"中点"`). Der Wert von `definition` darf selbst als Alias geschrieben werden. Aliase von Standardwörtern werden an diesen Stellen nicht verwendet, außer für `priority`, den Parameter von AS und ATTACH selbst.
    - Die Aliase folgen dem Namen: sie gehören zum `$`-Namen und gelten auch dort, wo eine untere Ebene die Definition durch eine gleichnamige verdeckt, ebenso wie ein Alias eines `$`-Namens die Schreibweise ersetzt.
    - Beim Deklarieren des Alias wird das Original nicht gegen die Definition geprüft; beim Anwenden der Definition ist ein nicht gefundener Name wie üblich ein Fehler (wie „keine Rolle in_p“).
    - Namen nach `::` werden in der Reihenfolge von 6.5 nachgeschlagen: die Namen der Anwendung selbst, ihre `definition=`-Aliase, die Standardunterobjektnamen und ihre Aliase, die Namen der Pins des Symbols des Objekts, die Schlüssel von Texten.
    - Der Wert der Auswahlbedingung `role=` und ein Pfad in `${…}`, der entlang von Rollen hinabsteigt, erkennen diese Aliase ebenfalls, bei jedem Schritt in der Definition der Anwendung nachgeschlagen, wie `%dp1 @SELECT-INSTANCE(role="左管")` oder `${左管::w}` für eine Anwendung `%dp1` von `$DIFFERENTIAL_PAIR_N`. Enthält der Bereich Anwendungen mehrerer Definitionen, wird jede in ihrer eigenen Definition nachgeschlagen; die Originale funktionieren weiterhin.
- **Was Aliase haben darf:** alle Wörter der Wortliste mit Ausnahme der im nächsten Punkt aufgeführten: Schlüsselwörter, Slot-Namen eingebauter Schlüsselwörter, Parameternamen und -werte, Artnamen, Unterobjektnamen, Attributnamen, Stileigenschaften, `true` und `false` (5.3); sowie die `$`-Namen und Namen in Definitionen oben.
- **Was keine Aliase haben darf:** die äußere Form, also `@RNL`, die Feldnamen eines Eintragskopfs und die Werte von `type`, sowie `@VERSION`; außerdem `@ALIAS` und `@USE-ALIASES` selbst.
- **Andere Namen.** Ein Name ist bereits ein anderer Name eines Objekts: nach `$"差分对" = $DIFFERENTIAL_PAIR_N` ist `$"差分对"` dieselbe Definition. Ein solcher Name ist Teil des Texts, kann nicht wieder herausgenommen werden und erreicht keine Rollen- oder Parameternamen; für einen vollständigen Satz von Namen in einer anderen Sprache verwendet man Aliase.
- **Entsprechung:**
  - Ein Original darf mehrere Aliase haben; innerhalb eines Bereichs steht ein Alias nur für ein Original. Die `definition=`-Aliase jeder Definition bilden einen eigenen Bereich.
  - Ein Alias darf nicht so geschrieben sein wie irgendein Standardwort der von der Datei deklarierten Version, und darf nicht erneut für ein anderes Original deklariert werden. Ein `definition=`-Alias darf nicht so geschrieben sein wie ein Rollen-, Parameter- oder lokaler Name, den die Definition besitzt, und auch nicht wie ein Alias von `priority`, dem Parameter von AS und ATTACH selbst.
  - Ein Alias kann nicht als Original verwendet werden und darf nicht gleich seinem Original sein. Wird die Version einer Datei erhöht und ist ein neu hinzugekommenes Standardwort genauso geschrieben wie ein Alias, wird eine Warnung ausgegeben und die Version dennoch erhöht (16.1); in der neuen Version hat der Alias dann die Schreibweise eines Standardworts, was die obige Regel zu einem Fehler macht.
  - Ein Alias-Paket muss die Sprachversion der Datei deklarieren, die es verwendet (16.1); andernfalls ist sein USE-ALIASES ein Fehler.
- **Aliase fügen nur Schreibweisen hinzu.** Die Standardschreibweisen funktionieren stets, und beide dürfen in derselben Datei gemischt werden.
- **Aliase ersetzen nur ganze Tokens:**
  - ein Alias ersetzt ein ganzes Token; er ist kein Textmakro, sodass Positionen und Diagnosemeldungen korrekt bleiben;
  - Standardwörter ohne `@` erhalten Aliase nach ihrer Schreibweise, sodass gleich geschriebene Wörter (wie der Parametername `x` und der Wert `"x"`) ihre Aliase teilen;
  - ein Alias eines `$`-Namens ersetzt die Schreibweise: ein Verfasser, der `$"接地补位集" = {…}` schreibt, definiert `$GROUND_FILL_SET` in jener Ebene, was das des Standardpakets mit einem Hinweis verdeckt (6.8).
- **Lexikalische Regeln:**
  - Ein Alias eines Schlüsselworts sind die Buchstaben (jeder Schrift), Ziffern und Unterstriche unmittelbar nach `@`, ohne `-`; sie bilden ein Token, und vor dem `@` muss ein Leerzeichen stehen (2.3), auch nach chinesischer Interpunktion: `对 &b，@选出元件` ist ein Fehler und muss `对 &b， @选出元件` geschrieben werden. Jedes Zeichen, das kein Buchstabe, keine Ziffer und kein Unterstrich ist, beendet den Alias: ein Leerzeichen oder ein Satzzeichen beliebiger Schrift, wie `，` oder `。`. So ist `@对齐两个器件` ein unbekanntes Schlüsselwort, ein Fehler, und seine Diagnosemeldung muss fragen, ob nach `@对齐` ein Leerzeichen fehlt. Es gibt keine Regeln für die eine oder andere Sprache: was kein Buchstabe, keine Ziffer und kein Unterstrich irgendeiner Schrift ist, beendet einen Alias, und Nicht-ASCII-Zeichen außerhalb von Klammern sind Freitext (3.3). Tokens werden nicht anhand der deklarierten Aliase aufgespalten, sodass das Lesen der Tokens keine Deklarationen erfordert.
  - Nicht in Anführungszeichen gesetzte Nicht-ASCII-Wörter treten nur an zwei Stellen auf: nach `@` und innerhalb von Klammern. Nach `@` muss das Wort ein deklarierter Alias sein; andernfalls ist es ein Fehler („ein Nicht-ASCII-Name benötigt Anführungszeichen“). Innerhalb von Klammern (Parameternamen, Slot-Namen eingebauter Schlüsselwörter, `true` und `false`, Rollennamen) ist ein deklarierter Alias ein Alias; an der Stelle eines Rollennamens oder eines Parameternamens einer Definition ist ein Wort, das kein Alias ist, der Name selbst (3.6); an anderen Stellen ist ein Wort, das kein Alias ist, ein Fehler. Anderswo gilt 3.6: Host-Namen, RNL-Namen, Anschlussnamen und Namen nach `::` haben ohne Anführungszeichen ausschließlich ASCII-Zeichen, und ein nach `::` verwendeter Alias wird ebenfalls in Anführungszeichen gesetzt, wie `#M1::"位置"`.
  - Ein nicht in Anführungszeichen gesetzter Alias in Klammern, der genauso geschrieben ist wie ein Parameter- oder Rollenname einer an derselben Stelle verwendbaren Definition, ist ein Fehler, weil seine Bedeutung nicht eindeutig ist. Ein Wort in Anführungszeichen in Klammern ist stets ein Name, niemals ein Alias. Aliase von `$`-Namen und Aliase nach `::` werden ohnehin in Anführungszeichen geschrieben (3.6).
  - Aliase von Aufzählungswerten werden innerhalb der Zeichenketten geschrieben und funktionieren nur in Parametern, deren Werte eine Aufzählung sind. Attributnamen in `${…}` erkennen ebenfalls Aliase (13.6). Andere Zeichenketten, wie `text` von TEXT, werden nicht ersetzt.
- **Diagnosemeldungen und Übersetzung:**
  - Diagnosemeldungen zeigen die Wörter wie in der Datei geschrieben zusammen mit ihrer Standardschreibweise.
  - Ein Übersetzer arbeitet mechanisch in beiden Richtungen. Beim Übersetzen in Aliase schreibt er USE-ALIASES in den Kopfeintrag; für eine eigenständige Datei schreibt er stattdessen die verwendeten Aliase als ALIAS-Anweisungen. Hat ein Original mehrere Aliase, wird der zuerst deklarierte genommen. Das Zurückübersetzen in die Standardschreibweise entfernt USE-ALIASES und ALIAS und verliert nichts.
  - Freitext, Kommentare und Zeichenketten werden nicht übersetzt, außer Aufzählungswerten und Attributnamen in `${…}`.
  - LLMs, Solver und Editoren schreiben die Standardschreibweise. Ändern sie eine mit Aliasen geschriebene Datei, stehen die von ihnen hinzugefügten Anweisungen ebenfalls in der Standardschreibweise; Mischen ist gültig.

<!-- translated from en/ch17.md sha256:801f28995842 -->

## 17 Änderungsmodell

- RNL beschreibt ausschließlich den aktuellen Zustand, nicht die Bearbeitungen, die zu ihm geführt haben:
  - es gibt keine Rückgängig- oder Löschoperationen; Löschen ist das Bearbeiten des Texts;
  - es gibt keine relativen Verschiebungen; ein Editor schreibt neue absolute Werte.
- **Wer welche Einträge ändert.** Der Solver ersetzt die Einträge mit `type="generated"`. Andere Werkzeuge dürfen nur die Einträge ändern, deren `author` sie selbst sind, etwa ein Editor, der die von ihm geschriebenen Änderungen bereinigt, oder ein neuer Import, der ersetzt, was der vorherige Import geschrieben hat. Werkzeuge dürfen von Verfassern geschriebene Einträge ohne `author` nicht antasten (2.8).
- Später angehängte Beschreibungen überschreiben frühere der Reihe nach (8.4); bei geometrischen Fakten wird zuerst die Priorität verglichen (8.5). `type` und `author` beeinflussen die Reihenfolge nicht (8.3).
- **Die Ausgabe gehört zur Teilschaltung.** Die Ausgabe des Solvers und die Änderungen, die ein Benutzer in einem Editor vornimmt, werden in den eigenen Text der Ebene geschrieben: bei einer Teilschaltung innerhalb ihres `.subckt … .ends`, auch wenn sie in einer eingebundenen Datei liegt; bei der obersten Ebene auf der Dateiebene der Hauptdatei. Der Solver schreibt seine Ausgabe mit `priority="high"` an das Ende des eigenen Texts der Ebene; wenn er löst, werden die Einträge mit `type="generated"` bei der Interpretation weggelassen (2.8), und danach ersetzt er sie (14.8). Er muss jede von ihm getroffene Entscheidung zurückschreiben (14.8). `scope=` ist nur für Anweisungen gedacht, die der Verfasser bewusst außen schreibt (2.6). Ob eine schreibgeschützte Datei gespeichert wird, liegt beim Werkzeug.
- **Verilog-A-Dateien.** Auf einer Modulebene gibt es nichts zu lösen, und sie erhält keine Ausgabe des Solvers. In einer Verilog-A-Datei ändert ein Werkzeug ausschließlich den Kopfeintrag und die Einträge von Blocksymbolen innerhalb ihrer Module, geschrieben mit `//` (2.9).
- **Eine veraltete Lösung liefert nur einen Hinweis.** Ändert sich eine obere Ebene oder ein Paket, kann die Lösung einer Teilschaltung in einer Bibliothek veralten, etwa wenn die Hauptdatei ein Symbol ändert. Die alte Lösung bleibt wirksam; ein Viewer, der feststellt, dass der `hash` im Eintragskopf nicht zur aktuellen Eingabe passt, gibt einen Hinweis, erneut zu lösen (2.8).
  - Der Hash wird aus der tatsächlichen Eingabe der Ebene nach Vererbung berechnet: den herabfließenden und wirksam werdenden Regeln, den gewählten Symbolen, den Versionen der Pakete und den Optionen des Solvers, ohne die Einträge mit `type="generated"` und ohne die Metadatenfelder. Er wird je Ebene berechnet, nicht je Datei, und nicht allein aus dem eigenen Text der Teilschaltung, da sonst Änderungen in der Hauptdatei unbemerkt blieben.
  - Der Hash ändert die Interpretation nicht. Sein Algorithmus gehört den Werkzeugen; der Wert sollte eine Kennung für den Algorithmus tragen, etwa `"sig1:…"`, und ein Werkzeug, das den Algorithmus nicht kennt, vergleicht ihn nicht.
  - Hinweis: Würde ein nicht übereinstimmender Hash bedeuten, dass die alte Lösung nicht verwendet wird, würde der Algorithmus Teil der Sprache, jedes Werkzeug müsste ihn auf dieselbe Weise berechnen, und ein Viewer, der nicht löst, könnte die Ebene nicht zeichnen.
  - Nummern sind die Ausnahme: ändern sich die Anweisungen, die unbenannte Objekte erzeugen, stimmt die Prüfung der Nummern nicht mehr überein, und die Anweisungen, die Nummern verwenden, haben keine Wirkung, bis die Ebene erneut gelöst wird (5.9). Die Prüfung umfasst nur den Text der Ebene und der von ihr verwendeten Definitionen, sodass jedes Werkzeug sie auf dieselbe Weise berechnet und der Rest der alten Lösung weiterhin gezeichnet wird.
- **Generierte Objekte.** Hat der Solver von Regeln generierte Objekte platziert und verdrahtet, schreibt er sie als explizite Anweisungen samt den Leitungen in seinen eigenen Eintrag. Das explizite Objekt hat Vorrang, und die Ergänzungsregeln erzeugen dasselbe Objekt nicht mehr (10.5). Auf Objekte in ihren Rümpfen ohne lokalen Namen wird über Nummern verwiesen (5.9).
- **Ändern dessen, was der Solver erzeugt hat.** Ein Name, den der Solver einem von ihm ausgeschriebenen Objekt gibt, ist nur in seinem eigenen Eintrag gebunden, der beim Lösen der Ebene weggelassen wird (2.8). Daher findet eine Anweisung eines anderen Eintrags, die diesen Namen verwendet, beim Lösen nichts, und der Solver kann sie nicht berücksichtigen. Um ein solches Objekt für sich zu beschreiben, schreibt ein Editor es von Hand in seinen eigenen Eintrag, wie 10.5 angibt, und fügt die Änderung hinzu, etwa `%g_r4 = $GROUND_STUB @ATTACH #R4:2; %g_r4::g @PLACE(30, -4, priority="high")`: die Leitung des Stubs macht den Pin verdrahtet, die Ergänzung überspringt ihn, und der nächste Lösevorgang schreibt jenes Objekt nicht mehr; bis dahin wird auch das Objekt im Eintrag des Solvers gezeichnet. Ist das geänderte Objekt die generierte Anwendung selbst, etwa eine Port-Markierung, erreicht eine Auswahl vom Host-Objekt aus sie sowohl beim Lösen als auch beim Zeichnen, etwa `&out @SELECT-ANCESTOR-USAGE $IO_PORT => @PLACE(40, 20, priority="high")`; die lokalen Objekte einer Anwendung können nicht über die Anwendung ausgewählt werden, sondern nur nach Namen oder als Objekte der Ebene (9.4).
- Die Felder eines Eintragskopfs gelten für jede Anweisung im Eintrag (2.8).
- **Beispiel: eine Änderung im Editor.** Unten steht ein vollständiges Fragment mit den Kommentarpräfixen des Hosts. Nach dem Kopfeintrag schrieb der Schaltungsentwickler den ersten Eintrag und der Solver den zweiten; der Benutzer zog VIN im Editor nach links, und der Editor hängte den dritten an:

  ```spice
  * Common-source stage
  * @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
  .model nch nmos level=1
  M1 out in 0 0 nch W=4u L=1u
  RD vdd out 5k
  VDD vdd 0 1.8
  VIN in 0 DC 0.6 AC 1
  * @RNL {
  *   {#RD #M1} @ALIGN("x");
  *   #M1 #RD @DISTANCE("bottom-to-top", min=2);
  *   &vdd @ADD-TO $SUPPLY_NET_SET
  * }
  * @RNL(type="generated", priority="high", author="rnl-solver 0.3", hash="sig1:5be1c09a") {
  *   #M1 @PLACE(10, 0); #RD @PLACE(10, 8); #VIN @PLACE(0, -2); #VDD @PLACE(20, 4)
  * }
  * @RNL(author="rnl-editor 0.1") { #VIN @PLACE(x=-4, priority="high")}
  .end
  ```

  - Das Ziehen wird als neuer Eintrag geschrieben, nach dem Eintrag des Solvers angehängt, mit Priorität `"high"`. Beide sind hoch, sodass der spätere behalten wird und das vom Solver geschriebene x mit einem Hinweis verworfen wird (8.5); bei jedem erneuten Lösen der Ebene ist das neue x eine feste Position (14.8).
  - Es in den Eintrag des Entwicklers zu schreiben funktioniert nicht. Die Anweisungen dort sind standardmäßig `"normal"` und können das `"high"` des Solvers nicht überschreiben; mit `"high"` geschrieben, verlieren sie dennoch, weil der Eintrag des Solvers später kommt und innerhalb derselben Priorität der spätere Fakt behalten wird. Dieser Fehler ist kein Fehler im Sinne eines Fehlschlags; er gibt nur einen Hinweis zur Auflösung. Die Position wird beim nächsten Lösen der Ebene wirksam, wenn die Anweisung des Entwicklers eine der Bedingungen ist; bis dahin ändert sich die Zeichnung nicht.
  - Nach Position angegebene Parameter stehen vor jenen mit Namen (4.2 Punkt 9): `@PLACE(-4, -2, priority="high")` ist dasselbe wie `@PLACE(x=-4, y=-2, priority="high")`, und `@PLACE(x=-4, -2)` ist ein Fehler.
  - Um zu ändern, was der Entwickler geschrieben hat (ein Symbol ändern, umbenennen, eine Bedingung löschen), bearbeitet man den Eintrag des Entwicklers direkt. Der Eintrag des Solvers wird unangetastet gelassen und beim nächsten Lösen der Ebene als Ganzes ersetzt.
- Um eine Anweisung vorübergehend zu deaktivieren, schließt man sie in `/* … */` ein.
- **Round-Trips.** RNL garantiert nicht, dass eine Zeichnung nach einem beliebigen Export nach RNL und anschließendem Reimport gleich bleibt (Prinzipien 1); ein Exporter darf wählen, exaktes RNL auf Koordinatenebene zu schreiben (Prinzipien 3). Diese Version definiert weder, was exaktes RNL ist, noch Kriterien für Round-Trips.

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

<!-- translated from en/annex-c.md sha256:e15f7ba635ad -->

## Anhang C Für spätere Versionen reserviert (informativ)

Dieser Anhang führt auf, was diese Version des Standards auslässt und wovon sie sagt, dass es in einer späteren Version hinzukommen kann, mit den Abschnitten, die es erwähnen. Diese Version reserviert keine Schlüsselwörter oder Schreibweisen für diese Punkte (5.3, 16.4), und eine spätere Version muss sie nicht hinzufügen.

| Für eine spätere Version zurückgestellt | Siehe |
|---|---|
| Träger für weitere Hosts, einschließlich Blockkommentaren (wie `#com … #endcom` von Eldo), sowie der Umgang mit einem dadurch hinzukommenden Kommentarendezeichen des Hosts | 2.2, 2.5 |
| Eintrags-IDs; Werkzeuge finden ihre Einträge über `type`, `author` und die Schlüssel von Anweisungen | 2.8 |
| Ausdrücke | 3.7 |
| Standardsätze, die angeben, ob eine Definition auf eine ganze Menge oder einmal für jedes Mitglied (`each`) angewendet wird | 4.5 |
| Optionale Slots; ohne sie hat eine Auswahl stets einen Bereich, und sei es nur `#` | 5.3, 10.1, 13.7 |
| Mehr als ein in einem Verweis expandiertes Subskript sowie weitere Formen von Subskripten, etwa `<*2>A`, `A<0:2*2>` und `D[0..7]` | 6.11 |
| Standardisierte Diagnosecodes | 7.6 |
| Eine Möglichkeit, Diagnosemeldungen in einer Datei zu unterdrücken, sowie ein strenger Modus, etwa als Felder des Eintragskopfs | 7.6 |
| Einheiten aus mehreren Fakten: eine Definition oder Anwendung, deren Fakten bei der Konfliktauflösung gemeinsam behalten oder verworfen werden. Noch offen: die Priorität einer solchen Einheit, geschachtelte Anwendungen und von außen über `::` geschriebene Prioritäten | 8.5 |
| Skalierung von Symbolen und Instanzen | 9.2.1, 11.7 |
| Pfeile an den Enden von Leitungen | 9.2.5 |
| Weitere Linienarten | 9.3 |
| Farbnamen und `rgb()` | 9.3 |
| Musterfüllungen, Farbverläufe und Schatten | 9.3 |
| Die Schnittmenge von Mengen in einer Anweisung | 10.1 |
| Wahl des Symbols eines Grundelements nach seinen Parametern. Bei einigen wenigen Grundelementen jedes Dialekts entscheidet der Simulator die Art anhand der Zeile: in SPICE ist eine mit `V=` geschriebene B-Quelle eine Spannungsquelle und eine mit `I=` eine Stromquelle. Teilschaltungen und Modelle variieren auf diese Weise nicht. Ein Weg ist eine Bedingung von SELECT-INSTANCE wie `param=` (die Zeile besitzt diesen Parameter); ein anderer ist ein Typ, den der Adapter des Dialekts diesen Grundelementen gibt (6.12), ausgewählt mit `type=`. Die Anzahl der Anschlüsse wird mit `terminals=` ausgewählt (10.1). Bis dahin zeichnet das Standardpaket beide Arten nach seinen bisherigen Regeln | 10.1, 6.12 |
| Drehen einer ganzen Menge als ein Block mit ORIENT | 11.2 |
| Mehrere Zeichnungen eines Host-Objekts als getrennte Objekte mit eigenen Namen | 11.4 |
| Eine Grafik, die eine Gruppe von Objekten einrahmt | 13.4 |
| Auf Leitungen verankerte Anmerkungen: Positionen entlang einer Leitung (etwa `t` oder `offset`) sowie in der Mitte einer Leitung verankerte Symbole oder Pfeile | 13.3, 15.2 |
| Verweise in `${…}` auf Attribute anderer Objekte, wie `${R3:VALUE}` aus KiCad | 13.6 |
| Die diagonalen Projektionen `"x+y"` und `"x-y"` für Bedingungen | 14.1 |
| Ein USE-PACKAGE, das nur für seine eigene Ebene gilt | 16.2 |
| Ein über mehrere Dateien verteiltes Paket | 16.2 |
| Ein Manifestformat für eine Netzliste und ihre Paketdateien | 16.2 |
| Eine Inhaltsprüfsumme für Pakete, etwa ein Parameter `digest=` von USE-PACKAGE | 16.2 |
| Verweise, die ein Paket nur für sich selbst verwendet, getrennt von jenen, die es an seine Benutzer weiterreicht | 16.2 |
| Paketqualifizierte Namen, andere Namen für Pakete und Ebenen-Tags, wie `$acme.DIFFERENTIAL_PAIR` oder `as=`, in einer anderen Form als einem `.` nach dem Namen | 16.2 |
| Ein Erweiterungsmechanismus: Schlüsselwörter mit einem Herstellerpräfix, etwa `@ACME.ROUTE`, im Kopfeintrag als erforderliche oder optionale Erweiterungen deklariert | 2.8, 16.4 |
| Exaktes RNL, also die Bedingungen, unter denen eine Datei ohne Solver gezeichnet werden kann, sowie Kriterien für Round-Trips; diese dürfen stattdessen zwischen Werkzeugen vereinbart werden | 17 |

<!-- translated from en/annex-d.md sha256:72752f1bf7f8 -->

## Anhang D Änderungen gegenüber Version 0.3 (informativ)

Version 0.4 schreibt jeden Positionsfakt mit seiner Richtung, sodass ein Fakt ein Layout bedeutet und keine Wahl unter mehreren. Versionen 0.x geben keine Zusicherung gegenseitiger Kompatibilität (16.1); dieser Anhang führt auf, was sich geändert hat und wie eine Datei nach 0.4 überführt wird. Dieser Standard beschreibt ausschließlich Version 0.4; eine Datei der Version 0.3 wird mit einem Upgrade-Werkzeug nach 0.4 überführt, das Version 0.3 liest und die Änderungen am Ende dieses Anhangs vornimmt (16.1).

| Änderung | Siehe |
|---|---|
| Richtungen werden ausgeschrieben. ORDER und DISTANCE nehmen ausschließlich die vier Richtungen; `"x"` und `"y"`, die die Richtung offenließen, sind ungültige Parameterwerte. DISTRIBUTE nimmt eine Richtung anstelle einer Achse im Parameter `direction` (nicht mehr `axis`): seine Mitglieder folgen einander streng in dieser Richtung in gleichen Schritten, und zwei Mitglieder ergeben ein ORDER. ALIGN nimmt eine Projektion, keine Liste | 14.1–14.5, 8.5 |
| ORIENT und ORIENT-RELATION nehmen eine Liste von Codes an, etwa `["R0", "MY"]`: ein Fakt, der gilt, wenn einer von ihnen gilt; der Solver wählt und schreibt einen einzelnen Code zurück. Die Ausrichtung ist die einzige Art von Fakt, die eine solche Wahl bietet | 11.2, 14.9, 8.5 |
| ORIENT-RELATION setzt ein Objekt zu einem Objekt in Beziehung. Eine Menge in einem Slot wird Mitglied für Mitglied ausgeführt, ein Fakt für jedes Mitglied; eine Menge mit mehr als einem Mitglied in beiden Slots ist ein Fehler. Version 0.3 setzte zwei Mengen als Ganze in Beziehung | 14.9, 5.2.1 |
| Mitgliederstellen: ein Verweis auf eine Menge, auf den unmittelbar `{…}` folgt, nimmt Mitglieder nach ihren Stellen, gezählt über ihre eine Stufe ausgebreiteten Mitglieder, etwa `%a{1}`, `%a{end}`, `%a{2:end}`; eine Anwendung nimmt ihre Mitglieder nach Rolle, und Stellen einer Auswahl sind ein Fehler. In 0.3 war `%a{#b}` zwei Objekte | 5.2.3, 3.2, 3.3, 3.5, 4.1, 6.10 |
| Bedingungen nehmen ausschließlich die untersten Objekte an. Eine Menge hat keine Position: keinen Mittelpunkt und keine Ausdehnung, und ihr `::position` ist die Menge der Positionen ihrer Mitglieder. Eine Menge behält ihre Elemente wie geschrieben; in der Mitgliederliste von ALIGN, ORDER und DISTRIBUTE steht ein Element, das eine Menge ist (ein `{…}` oder ein Verweis auf eine Menge), für jedes seiner Mitglieder, eine Bedingung für jede Kombination, jede eine Einheit der Auflösung; DISTANCE setzt jedes Mitglied des einen Slots zu jedem Mitglied des anderen in Beziehung. In 0.3 wurde ein Verweis in einer Menge ausgebreitet, `{%x}` zählte über seinen Mittelpunkt, und DISTANCE verglich Ausdehnungen | 14.1, 14.3–14.5, 5.2, 5.2.1, 6.5, 8.5, 11.1, 11.2 |
| Eine Anwendung wird nicht mehr am Punkt des Objekts gezeichnet, das ihre erste Rolle füllt: ihr Symbol wird an ihrem eigenen Punkt gezeichnet, und eine Definition, die eine Markierung auf einem Pin zeichnet, bindet ihren Punkt in ihrem Rumpf an den Pin. Ein Pin, der eine Rolle bindet, wird auf dem Netz dessen gezeichnet, was sie füllt, sodass `$GROUND @ATTACH #M5:3` eine Masse an einer eigenen Position an das Netz von M5:3 hängt; eine Zahl im Symbol einer Anwendung bindet keine Rolle. In 0.3 wurde diese Anwendung auf dem Pin gezeichnet und hatte keine eigene Position | 12.1, 12.2, 12.5, 12.6, 6.5, 8.2, 9.2.7, 11.2, 11.3, 13.2, 14.1, 15.2 |
| Eine allgemeine Definition darf keine Rollen haben (`roles=[]`), angewendet als `$PORT_LIST @ATTACH`. Ihr Rumpf darf Regeln enthalten: sie wirken als Regeln der Ebene der Anwendung an der Position ihrer Anweisung, und `#` im Rumpf ist jene Ebene. Ein alleinstehendes `?` in einer allgemeinen Definition nimmt einen Pfad an, etwa `?::position` | 9.4, 8.2, 10.4, 4.1 |
| Dieser Standard beschreibt ausschließlich Version 0.4: Dateien von 0.3 und 0.2 sind Dateien anderer Hauptversionen, die mit einem Upgrade-Werkzeug, das Version 0.3 liest, auf die Sprache 0.4 und die Standardserie 0.4 überführt werden. Der Standard 0.3 beschrieb, wie ein Werkzeug Dateien von 0.2 liest | 16.1 |
| Das Standardpaket wird nicht mehr implizit verwendet: eine Datei schreibt `@USE-PACKAGE(name="rnl-std-all", …)`, oder ein Werkzeug fügt es über eine Einstellung hinzu, wie `--include-std` oder eine Einstellung eines Viewers, und schreibt es in die Datei, wenn es sie speichert. In 0.3 wurde es an der Wurzel jeder Datei verwendet | 16.3, 16.1, 16.2, 6.1 |
| Literale Koordinaten im Rumpf einer allgemeinen Definition, bei Punkten und bei PLACE, sind Versätze vom eigenen Punkt der Anwendung, und das ORIENT der Anwendung dreht sie nicht. In 0.3 waren es Koordinaten der Ebene | 9.4, 11.1, 11.2, 13.4 |
| DISTANCE gibt auch die Ordnung an: seine Schranken sind nicht negativ, und ein fehlendes `min` ist 0. In 0.3 nahm ein DISTANCE mit Richtung Schranken beliebigen Vorzeichens an | 14.5 |
| Regeln und Phasen: die Regeln von WIRE sind entfallen, und BUNDLE, WIRE sowie mit zwei Punkten geschriebene RECTANGLE, CIRCLE und ELLIPSE nehmen kein Ergebnis einer Auswahl an und keine davon abhängige Menge; die Regeln der vier Route-Schlüsselwörter werden in einem eigenen Schritt ausgewertet, Schritt 5, vor der Generierung, die ihre Bäume sieht; die Regeln von LINE, ARC und BEZIER sowie von CURRENT werden in Schritt 6 mit den Generierungsregeln ausgewertet; ein CURRENT auf einem Pin wird wirksam, sobald die Symbole zugewiesen sind. In 0.3 wurden diese Regeln in der Phase ihres Schlüsselworts wirksam | 10.4, 8.2, 11.8, 13.1, 9.2.5, 15.2 |
| Jede Anwendung hat einen eigenen Punkt, auch eine ohne Symbol, geschrieben `%u::position`: PLACE nimmt ihn an, ebenso Bedingungen auf einer mit Symbol gezeichneten Anwendung. In der Mitgliederliste einer Bedingung steht eine Anwendung ohne Symbol für ihre Mitglieder, bis hinab zu den untersten Objekten wie in GROUP, und eine Kombination, die dasselbe Objekt zweimal nimmt, wird weggelassen. Ein alleinstehendes `?` im Rumpf einer allgemeinen Definition ist die gerade expandierte Anwendung. Ein Hinweis wird gegeben, wenn ein mit Bedingungen versehener Punkt einer Anwendung an nichts Gezeichnetes gebunden ist. In 0.3 schlugen PLACE und Bedingungen auf einer Anwendung ohne Symbol fehl | 14.1, 9.4, 12.1, 6.2, 6.5, 3.3, 7.1, 8.2, 11.1 |
| Selektive Leitungsführung: ROUTE wird ersetzt durch ROUTE-NET (ein ganzes Netz, wie es ROUTE tat), ROUTE-OBJECT (die Pins einiger Objekte, Netz für Netz), UNROUTE-NET und UNROUTE-OBJECT (herausgenommene Pins). Jeder Pin wird durch die Anweisung höchster Priorität entschieden, die ihn erfasst, dann durch die spätere. Bäume auf einem Netz, die einen Pin teilen oder durch geschriebene Leitungen verbunden sind, sind ein Baum. Ein Pin in einem Baum zählt als `wired` | 13.7, 10.1, 10.2, Anhang A |
| Der Solver darf eine vom Verfasser geschriebene Leitung aufteilen, um aus ihrer Mitte abzuzweigen, auf einem Netz, das ein Baum verbindet; er schreibt die Leitung in seinem Eintrag neu und behält die Punkte des Verfassers in ihrer Reihenfolge | 13.1, 13.7 |
| Gruppen sind keine Objekte: keine Position, keine Bedingungen auf einer Gruppe. Die Mitglieder einer Gruppe sind die Objekte in ihrer Menge bis hinab zu den untersten; eine mit Symbol gezeichnete Anwendung ist selbst ein Mitglied; ROUTE-OBJECT folgt ihren Rollen ebenso und nimmt auch ihre eigenen Pins mit auf. Der Baum der Gruppen und das Heraushalten Außenstehender aus dem Bereich einer Gruppe gehören zum Solver als Kriterien nach jedem Fakt. GROUP nimmt `spacing_x` und `spacing_y`, den Abstand innerhalb der Gruppe in Rastereinheiten | 11.5, 13.7 |
| SELECT-INSTANCE nimmt die Bedingung `terminals`, die Anzahl der Anschlüsse der Instanz, sodass etwa MOS-Transistoren mit drei und mit vier Anschlüssen unterschiedliche Symbole erhalten | 10.1, Anhang A |
| Das Spectre-Grundelement `tline` hat die Standardanschlussnamen `p1`, `n1`, `p2` und `n2`, wie T in SPICE; die Tabelle der Modellfamilien ergänzt Master und die MESFET-Familie | 6.12 |
| Eine Instanz, der keine Anweisung und keine Regel ein Symbol gibt, hat das Standard-Blocksymbol: einen quadratischen Rahmen, zentriert auf ihrem Ursprung, mit einem Pin für jeden Port, oder für jeden Knoten bei einer Instanz, deren Definition nicht gelesen wird; keine Instanz bleibt ungezeichnet, und ein Viewer zeichnet keine eigene Ersatzdarstellung mehr | 9.2.6, 9.2.8 |
| Das Eintragskopffeld `language` lässt einen Eintrag nur auf die Objekte in Abschnitten jener Netzlistensprache wirken; eine Datei darf für jede Sprache neben dem gemeinsamen einen Kopfeintrag haben. Die Standardserie 0.4 nutzt dies, um den SPICE-Abschnitten einer Spectre-Datei die Regeln von SPICE zu geben; die Serie 0.4 lässt das Paket des anderen Dialekts nicht mehr implizit weg, wie es die Serie 0.3 tat | 2.8, 2.2, 6.12, 16.3, Anhang C |
| Die Sprache, in der eine Datei beginnt, ist eine Eingabe an das Werkzeug, standardmäßig nach ihrer Endung, und der Benutzer kann sie für jede Datei ändern | 2.2 |
| Regeln gelten standardmäßig für ihre eigene Ebene: `recursive` ist standardmäßig `false`, und eine Regel mit einer darin geschriebenen Auswahl erreicht die Ebenen darunter nur mit `recursive=true` auf einer dieser Auswahlen, und eine Regel ohne eine solche erreicht so weit wie die von ihr verwendeten Mengen; jede Auswahl und Menge der Regel wählt dann in jeder Ebene aus. In 0.3 hatte jede Auswahl ihr eigenes `recursive`. Eine Regel in einem Paket zählt als in der Ebene ihres USE-PACKAGE geschrieben, und ein Paket an der Wurzel auf der obersten Ebene seiner Datei, sodass die Regeln des Standardpakets `recursive=true` schreiben; eine Generierungsregel warnt nur dann wegen Objekten ihrer Ebene, wenn sie die Ebenen darunter erreicht. Eine Regel, die die Ebenen darunter erreicht und deren andere Slots Host-Objekte ihrer Ebene benennen, hat in den Ebenen darunter keine Wirkung, ohne Warnung | 10.1, 10.3, 10.4, 10.5, 6.1, 8.3, 16.2, 4.5, Anhang B, Prinzipien 2.5 |
| SOLVER-CONFIG hält die Einstellungen des Solvers in der Datei fest, oder im Kopfeintrag eines Pakets für die Ebenen, die es verwenden: benannte Parameter, deren Schlüssel der Solver definiert und von denen der Standard nur `spacing_x`, `spacing_y` und `junction`, die Definition der Punkte, mit der jeder Solver zeichnet, benennt; spätere Einstellungen überschreiben frühere Schlüssel für Schlüssel; der Kern reicht die Einstellungen wie geschrieben der Reihe nach an den Solver weiter; ein Wert, der nicht angenommen wird, lässt nur seinen Schlüssel fehlschlagen, und der Wert von vorher bleibt; Zeit- oder Arbeitsbudgets gibt nur das Werkzeug vor | 14.10, 14.8, Anhang C |
| CURRENT ist kein geometrischer Fakt, sondern nur ein Hinweis für den Solver; ORDER vergleicht Punkte, und das Auseinanderhalten der Rahmen von Symbolen ist ein Kriterium des Solvers; ORDER steht kurz für ein DISTANCE mit `min=1` zwischen Nachbarn; ein Renderer ohne Ergebnis zeichnet den ersten Code einer ORIENT-Liste | 15.2, 14.3, 11.2 |
| Klargestellt: ein TEXT mit leerem Inhalt zeichnet nichts; der Solver ersetzt generierte Einträge als Ganzes, ob er deren Anweisungen versteht oder nicht; die Anschlüsse eines Bündels entsprechen einander nach Namen, und Mitglieder, die in ihren Anschlussnamen oder ihrem Modell abweichen, liefern eine Warnung | 13.3, 2.8, 11.8 |
| Pins sind Objekte, Art `pin`, getrennt von Anschlüssen. Ein Pin wird durch PIN in einem Symbol erzeugt und über seinen lokalen Namen benannt, `#X1::vdd_t`; `terminal=` darf weggelassen werden. Ein Anschluss darf mehrere Pins haben; wo ein Pin benötigt wird, steht ein Anschluss für seinen Pin, den zuletzt geschriebenen. Pins ersetzen einander nicht mehr nach Anschluss. Ein Pin-Name darf nicht gleich einem Rollennamen, lokalen Namen, Standardunterobjektnamen oder Schlüssel eines Texts sein; `::` schlägt Pin-Namen vor den Schlüsseln von Texten nach | 9.2.4, 9.2.7, 5.1, 6.4, 6.5, 13.3 |
| SET-SYMBOL ordnet die Anschlussnamen des Symbols den Anschlüssen des Objekts nach Namen zu, oder bei Zahlen nach Stelle; `terminals=` ändert die Zuordnung, aufgelistet in der Reihenfolge der Knoten oder als Paare | 11.3 |
| Anwendungen haben keine Anschlüsse: `%g:p` aus 0.3 ist nun ein benannter Pin wie `%g::pin`. Die Pins einer Anwendung binden Rollen nach Namen und werden auf dem gezeichnet, was sie füllt; sie sind keine elektrischen Anschlüsse | 12.1, 6.4 |
| Objekte, die für eine andere Art angegeben werden, werden ausschließlich nach einer Tabelle umgewandelt: hin zur Zeichnung Anschluss zu Pin zu Punkt und gezeichnetes Objekt zu Position; hin zur Schaltung Pin zu Anschluss und Pin oder Anschluss zu Netz | 5.2.4 |
| SELECT-PIN wählt Pins aus; `wired` gehört zu Pins und Netzen; SELECT-TERMINAL wählt jeden Anschluss aus, mit Pin oder ohne, und Schritte über Anschlüsse zählen sie alle. SELECT-INSTANCE und SELECT-USAGE nehmen `symbol` und `drawn` an. Mit einer Größe geschriebene RECTANGLE, CIRCLE und ELLIPSE nehmen das Ergebnis einer Auswahl einzeln an; die Bedingung `type` nimmt den Platzhalter `*`, und `type="*"` wählt die Instanzen aus, die irgendeinen Typ haben | 10.1, 10.2, 8.2, 9.2.5, 13.3 |
| Zusicherungen: ASSERT-INSTANCE, ASSERT-TERMINAL, ASSERT-NET und ASSERT-USAGE prüfen, dass Mitglieder die Bedingungen der Auswahl ihrer Art erfüllen, ASSERT-USAGE auch, dass sie Anwendungen einer Definition sind; ASSERT-IN prüft, dass die Mitglieder in einer Menge sind. Sie prüfen, dass Attribute geschrieben sind (`has`) und gleich (`same`) oder verschieden (`different`) sind; ASSERT-TOPOLOGY prüft, dass Mitglieder in Reihe, parallel, als Stern oder als einzelne Schleife verbunden sind, wobei eine Anwendung oder eine Menge als Blackbox zählt; eine nicht geltende Prüfung liefert einen Verstoß, eine vierte Berichtsart neben Fehler, Warnung und Hinweis, und lässt keine Anweisung fehlschlagen. Die CURRENT-Anweisungen eines Netzes prüfen gemeinsam, dass der Strom nicht insgesamt in eine Richtung fließt; in 0.3 wurde nichts geprüft | 15.2, 15.3, 15.4, 7.6, Anhang C |
| TABLE ordnet Objekte als Tabelle aus geschachtelten `{…}` an, Zeilen von oben nach unten (oder mit `transpose=true` Spalten von links nach rechts), jede geschachtelte Liste im rechten Winkel; ein Fakt, und seine Objekte eine Gruppe | 14.11 |
| Text: Mathematikmodus `\(…\)` mit einer Teilmenge von LaTeX-Mathematik; `\subrest{…}` und `\subdigits{…}` zeigen den Rest eines Namens oder seine Ziffern am Ende tiefgestellt; TEXT nimmt `fix_rotate`, das die Richtung der Zeichen beibehält, während die Seite des Textfelds weiterhin der Ausrichtung des Objekts folgt | 13.6, 13.3 |
| Aliase enden an jedem Zeichen, das kein Buchstabe, keine Ziffer und kein Unterstrich ist, in jeder Schrift; keine Regel benennt chinesische Interpunktion. Das chinesische Alias-Paket ist `alias-zh`. Die Standardsätze sind ausschließlich auf Englisch angegeben | 16.5, 4.5 |
| Die Wortliste fügt `removed` hinzu, die Wörter, die 0.4 nicht mehr hat; das Schema fügt `open_params` und `removed` hinzu, und `at_least_one` wird eine Liste von Gruppen, wie es `together` und `exclusive` sind. | 5.3 |
| Der leere Knoten, `&` allein, besagt, dass es kein Netz gibt: eine Rolle der Art `net` und der Netz-Slot von WIRE nehmen ihn an, damit virtuelle Bauteile ein Blockdiagramm zeichnen können, das keine Netzlistenzeilen hat; er ist kein Objekt, und zwei Stellen, die ihn schreiben, haben nichts gemeinsam | 6.3, 9.4, 12.1, 13.1 |
| In einem Ebeneneintrag darf `junction` von SOLVER-CONFIG eine `%`-Definition jener Ebene benennen, damit eine für eine Teilschaltung definierte Punktvariante in ihr bleibt | 13.2, 14.10 |
| `$NO_CONNECT` nimmt einen Anschluss als ihn selbst an, sodass eine Markierung auf einem Mitglied eines Busses angibt, welcher Anschluss offen bleibt | 12.6 |
| TEXT: `rotate` nimmt jeden Winkel; `upright` (Standard `true`) dreht einen Text zum Lesen, und mit `false` folgen die Zeichen Drehung und Spiegelung, für Zeichen wie Pfeile; die Textstileigenschaft `textvisible` blendet einen Text aus, der dann keinen Platz einnimmt, während sein Inhalt erhalten bleibt | 9.2.5, 9.3, 13.3 |
| ANNOTATION sammelt die Grafiken eines Symbols, einer allgemeinen Definition oder einer Anwendung zu einer Anmerkung, an einem Punkt oder auf einem Objekt, um einen beliebigen Winkel gedreht; Pins und das Übrige werden weggelassen, ebenso die an andere Objekte gebundenen Grafiken eines Rumpfes | 13.5 |
| Eine Implementierung darf Grenzen für Ressourcen festlegen, etwa für die Schachtelungstiefe oder die Anzahl von Objekten; Eingaben jenseits einer Grenze liefern einen Fehler der Implementierung, niemals ein stillschweigend abgeschnittenes Ergebnis; mindestens 64 Schachtelungsebenen von Symbolen werden unterstützt | 1.6 |
| Eine Implementierung setzt ein Limit für die Zeit oder den Aufwand, den sie auf das Prüfen von Fakten verwendet; dieser Standard legt nicht fest, wie das Limit gemessen wird | 8.5 |
| DEVICE nimmt `type` an, was einer Instanz, der der Host keinen Typ gibt, wie einem PDK-Bauteil, den Typ verleiht, den Auswahlen und Zusicherungen lesen; `name` oder `model` sowie `terminals` oder `type` dürfen jeweils weggelassen werden, aber nicht beide, und für die Namen und den Typ getrennt zählt das letzte passende DEVICE, das sie schreibt. Die Standardanschlussnamen des Spectre-`transformer` sind `p1 n1 p2 n2` | 6.12 |
| Das Ergebnis einer Auswahl hat eine definierte Reihenfolge: die Reihenfolge der Szene, oder Bereichsmitglied für Bereichsmitglied ohne Wiederholungen, die von Regeln erzeugten Objekte nach den geschriebenen | 10.3 |
| Pins zeigen in eine Richtung in der Zeichnung, welche auf Pins geschriebene ORIENT und ORIENT-RELATION einschränken; ORIENT nimmt die Wörter `left`, `down`, `right` und `up` für R0, R90, R180 und R270 | 9.2.4, 11.2, 14.9 |
| Ein mit `ahdl_include` oder `.hdl` eingebrachtes Verilog-A-Modul kann seinen Instanzen ein Blocksymbol geben, geschrieben in `//`-Einträgen innerhalb des Moduls; das Modul ist eine Ebene ohne Schaltung, die nur Symbole und Stile enthält, und ein Vektorport ist ein Anschluss je Bit in der Reihenfolge seines deklarierten Bereichs | 2.9, 2.2, 6.1, 9.2.6, 17 |

**Überführen einer Datei von 0.3 nach 0.4.** Dieser Standard beschreibt keine Dateien der Version 0.3; ein Upgrade-Werkzeug, das Version 0.3 liest, überführt sie (16.1). Es erhöht die Version im Kopfeintrag, überführt die Datei in die Standardserie 0.4 und nimmt die folgenden Änderungen vor; was es nicht ändern kann, meldet es dem Verfasser:

- `"x"` oder `"y"` als Richtung von ORDER oder DISTANCE sowie `axis=` bei DISTRIBUTE: eine Richtung wählen.
- Die Anschlussnamen `p_plus`, `p_minus`, `s_plus` und `s_minus` eines Spectre-`transformer`: `p1`, `n1`, `p2` und `n2` schreiben.
- Mengen auf beiden Seiten von ORIENT-RELATION: für jedes Paar einen Fakt schreiben.
- Regeln ohne `recursive`: ein Werkzeug schreibt `recursive=true` in ihre Auswahlen, in der Datei, ihren eigenen Paketdateien und ihren Inline-Paketen, damit sie weiterhin die Ebenen darunter erreichen. Eine Regel, die `recursive=false` bei einigen ihrer Auswahlen und bei anderen nicht schreibt, wird gemeldet: in 0.4 lässt ein einziges `true` die ganze Regel nach unten reichen.
- ROUTE: ein Werkzeug schreibt ROUTE-NET an seiner Stelle.
- WIRE oder BUNDLE mit dem Ergebnis einer Auswahl: ROUTE-OBJECT für die Leitungen schreiben und die Mitglieder eines Bündels einzeln.
- `%g:p` und andere Anschlüsse von Anwendungen: den benannten Pin des Symbols schreiben, wie `%g::pin`. Die Symbole der Serie 0.3 binden die Rollen von Anwendungen nach Nummer, was 0.4 nicht tut (11.3).
- `$GROUND @ATTACH #M5:3` und andere virtuelle Bauteile, deren erste Rolle ein Anschluss oder ein Pin ist: sie stehen nun an einer eigenen Position auf diesem Netz; um sie am Pin zu halten, schreibt man einen Stub, `$GROUND_STUB @ATTACH #M5:3`, oder Bedingungen, die den Pin der Masse auf den Pin von M5 setzen (12.3).
- `wired=` auf SELECT-TERMINAL: SELECT-PIN schreiben, etwa `# @SELECT-INSTANCE => @SELECT-PIN(wired=false)`.
- `{%x}` nach seinem Mittelpunkt gezählt und `::position` einer Menge, das ihr Mittelpunkt war: eine Menge hat in 0.4 keine Position, und ihr `::position` ist die Menge der Positionen ihrer Mitglieder; das Upgrade belässt eine solche Anweisung zur Überprüfung. Um die Objekte als Einheit zu platzieren, macht man sie zu Mitgliedern einer Anwendung, deren Rumpf sie an ihren eigenen Punkt bindet, und versieht `%u::position` mit Bedingungen (14.1).
- Eine Anwendung in der Mitgliederliste einer Bedingung, die über einen Punkt zählte: das Upgrade schreibt `%u::position` für eine Anwendung ohne Symbol, die sonst für ihre Mitglieder stünde (14.1).
- DISTANCE zwischen zwei Mengen: es gilt nun für jedes Paar, sodass ein `max` jedes Paar begrenzt und nicht die Lücke zwischen Ausdehnungen.
- Literale Koordinaten im Rumpf einer allgemeinen Definition: sie sind Versätze vom eigenen Punkt der Anwendung; eine Definition, die die Koordinaten der Ebene meinte, lässt ihre Anwendungen bei `(0, 0)` platzieren, etwa `$TITLE @ATTACH => @PLACE(0, 0)`.
- Das Standardpaket: `@USE-PACKAGE(name="rnl-std-all", version="0.4")` schreiben und die Anwendungen der Serie 0.3 umschreiben, wie „Die Standardserie 0.4“ unten angibt: die neuen Namen und Rollen, eine Port-Markierung `$IO_PORT @ATTACH` für jedes Portnetz, das die Ergänzung von 0.3 markiert hat, und einen Bericht für jede Anwendung einer entfernten Definition.

**Die Standardserie 0.4.** Das Standardpaket ist nicht Teil dieses Standards (16.3); was folgt, ist für eine Datei wesentlich, die von der Serie 0.3 zur Serie 0.4 übergeht.

- **Verwendung**: eine Datei schreibt `@USE-PACKAGE(name="rnl-std-all", version="0.4")` in ihren Kopfeintrag oder wird mit einer Einstellung des Werkzeugs gelesen, die es hinzufügt (16.3).
- **Ports**: die Ergänzung gibt jedem Portnetz eine Port-Markierung, auch Versorgungs- und Massenetzen, in einer Spalte mindestens 6 Einheiten links von den Instanzen der Ebene, jede Markierung bei R0; eine Datei nimmt Netze aus `$PORT_FILL_SET` heraus, schreibt `$IO_PORT @ATTACH` für eine eigene Markierung oder `$PORT_LIST @ATTACH` für die ausgeschriebene Spalte (12.4).
- **Die Ergänzung**: sie fügt unverdrahteten Pins Netzbeschriftungen hinzu, jeweils einen kurzen Stub; kein Netz wird als Leitungen gezeichnet, sofern nicht eine Datei oder ein Paket dies mit ROUTE-NET oder ROUTE-OBJECT angibt; die Masse- und Versorgungsnetze werden mit `@UNROUTE-NET(priority="high")` ausgelassen, und eine Datei, die sie gezeichnet haben möchte, schreibt `@ROUTE-NET(priority="high")` oder WIREs. Die Ergänzung fügt Pins mit Leitungen kein Masse- oder Versorgungssymbol hinzu, ob die Datei sie von Hand verdrahtet oder mit ROUTE-NET oder ROUTE-OBJECT zeichnet, als Anweisungen oder als Regeln geschrieben; eine Datei, die eines wünscht, hängt es an, wie `$GROUND @ATTACH &vss` für ein ganz gezeichnetes Netz oder `$GROUND_STUB @ATTACH #R1:2` auf einem Pin. Ihre Regeln schreiben `recursive=true` und wählen Pins aus, jene virtueller Bauteile eingeschlossen. Die Netze in `$BIAS_NET_SET` erhalten Bias-Markierungen, wie die Masse- und Versorgungsnetze die ihren erhalten.
- **Die Pakete**: die einzige Sammlung ist `rnl-std-all`, die auf jedes Paket der Serie verweist und für jede Sprache einen Kopfeintrag hat (2.8, 16.3); die Sammlungen `rnl-std`, `rnl-std-core` und `rnl-std-full` der Serie 0.3 sind entfallen, und `rnl-std-structures` heißt nun `rnl-std-semantics`. `rnl-std-no-labels` wählt mit `drawn=true` aus und blendet Beschriftungen mit dem Stil `textvisible=false` statt leerer Texte aus, sodass eine Datei eine wieder mit `textvisible=true` anzeigt. Das optionale `rnl-std-45deg` ist neu.
- **Umbenannt oder geändert**: `$DIFFERENTIAL_PAIR` wird zu `$DIFFERENTIAL_PAIR_N` und `$DIFFERENTIAL_PAIR_P` mit den Rollen `left`, `right` und `tail`; `$MIRRORED` heißt nun `$MIRRORED_LAYOUT`, mit den Rollen `left`, `right` und einem `axis`, und das neue `$MIRRORED_VERTICAL_LAYOUT` hat `top`, `bottom` und `axis`, da eine Definition, die nur platziert, auf `_LAYOUT` endet; die Strukturen mit `_N` und `_P` sichern die Polarität ihrer Transistoren zu; `$FLOW` wird zu `$SIGNAL_FLOW`, das nichts platziert, keine Richtung annimmt und die Quellen sowie Lasten seines Pfades nennen kann, und `$POWER_FLOW` sowie `$BIAS_CIRCUIT` sind neu; `$CURRENT_MIRROR` wird zu `$CURRENT_MIRROR_N` und `$CURRENT_MIRROR_P`; die Rolle von `$NO_CONNECT` nimmt einen Anschluss oder einen Pin an. Die vollständige Liste der Definitionen, die die Serie umbenennt, zusammenlegt, aufteilt oder entfernt, steht in der Beschreibung des Standardpakets; der Befehl `rnl upgrade` in der letzten Version des RNL-Kerns 0.5.x (eine Version des Werkzeugs, nicht dieser Sprache) schreibt Dateien danach um. Jeder Positionsfakt in den Strukturen gibt seine Richtung an, und die andere Zeichnung ist eine eigene Variante, wie `$BRANCH_HORIZONTAL`.
- **Entfernt**: `$CHAIN`, `$SUPPLY_HIGH` und `$SUPPLY_LOW`.
