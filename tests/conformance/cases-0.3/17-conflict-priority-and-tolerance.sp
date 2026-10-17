* Conflict resolution, pipe segment order, self-contradictions, and 1e-6 tolerance (2.7, 5.5, 8.5, 11.7, 14.1-14.5)
*! 2.7, 5.5, 8.5: Facts are ordered by priority ("high" > "normal" > "low"), then
*!   reverse position (later statement / later pipe segment wins).
*! 8.5: Self-contradicting ORDER ({#R1 #R1} @ORDER("left-to-right")) and
*!   ORIENT-RELATION ({#R1 #R1} @ORIENT-RELATION("MY")) fail with an error and
*!   take no part in conflict resolution.
*! 8.5: Discarded "normal"/"high" facts emit conflict hints naming the conflicting
*!   kept facts, while discarded "low" facts are dropped silently without hints.
*! 11.7, 14.2-14.4: Geometric comparisons use tolerance 1e-6 (ALIGN within 1e-6,
*!   ORDER >= 1 - 1e-6, DISTRIBUTE step difference within 1e-6).
* @RNL(type="header") {
*   @VERSION("0.3")
* }

* High priority record comes FIRST in text, yet wins over normal and low records written later
* @RNL(type="generated", priority="high") {
*   #Rprio @PLACE(9, 0) => @ORIENT("R0");
*   #Rpipe @ORIENT("R0")
* }

* Normal priority record comes SECOND: loses to earlier high record (with hint),
* beats later low record on #Rlow, and tests pipe segment order, self-contradictions, and 1e-6 tolerance
* @RNL {
*   #Rprio @PLACE(2, 0);
*   #Rlow @PLACE(7, 0) => @ORIENT("R0");
*   #Rpipe @PLACE(3, 0) => @PLACE(4, 0);
*   {#R1 #R1} @ORDER("left-to-right");
*   {#R1 #R1} @ORIENT-RELATION("MY");
*   #R1 @PLACE(0, 0) => @ORIENT("R0");
*   #R2 @PLACE(0.0000005, 5) => @ORIENT("R0");
*   #R3 @PLACE(0.000002, 10) => @ORIENT("R0");
*   #R4 @PLACE(0.9999995, 15) => @ORIENT("R0");
*   #R5 @PLACE(0.999998, 20) => @ORIENT("R0");
*   #R6 @PLACE(2, 0) => @ORIENT("R0");
*   #R7 @PLACE(4.0000005, 0) => @ORIENT("R0");
*   %align_ok = {#R1 #R2} @ALIGN("x");
*   %align_drop = {#R1 #R3} @ALIGN("x");
*   %order_ok = {#R1 #R4} @ORDER("left-to-right");
*   %order_drop = {#R1 #R5} @ORDER("left-to-right");
*   %dist_ok = {#R1 #R6 #R7} @DISTRIBUTE("x")
* }

* Low priority record comes LAST in text, yet loses to earlier high and normal records silently (no hint)
* @RNL(priority="low") {
*   #Rprio @PLACE(1, 0);
*   #Rlow @PLACE(1, 0)
* }

Rprio a b 1k
Rlow a b 1k
Rpipe a b 1k
R1 a b 1k
R2 a b 1k
R3 a b 1k
R4 a b 1k
R5 a b 1k
R6 a b 1k
R7 a b 1k
.end
