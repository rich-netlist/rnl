* Numbers and a rewritten wire in a file of 0.3
*! The solver's record refers to objects without a name by number (5.9): the wire in the author's record is %#1, the free point on it %#2, and the record's own objects are numbered on from there, (4, 6) being %#3.
*! numbering is the check of these objects; the numbers take effect only when it matches. WIRE given a wire rewrites it (13.1): this statement rewrites the points and mode of %#1 and makes no new wire.
* @RNL(type="header") { @VERSION("0.3")}
R1 a b 1k
R2 b 0 1k
* @RNL {
*   #R1 @PLACE(0, 8);
*   #R2 @PLACE(0, 0);
*   &b @WIRE {#R1:2 (_, _) #R2:1}
* }
* @RNL(type="generated", priority="high", numbering="1457f767879ae90b") {
*   %#2 @PLACE(4, 4);
*   %#1 @WIRE(mode="straight") {#R1:2 (4, 6) %#2 #R2:1}
* }
.end
