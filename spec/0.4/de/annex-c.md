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
