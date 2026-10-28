* Facts of the segments of one pipe: the later segment is written later
*! When several segments of one statement's pipe write the position or orientation of the same object, the later segment counts as written later: at the same priority it is taken in first and kept (8.5), and replaces the earlier.
*! R1 has PLACE(0, 0) and then PLACE(5, 5): drawn at (5, 5). R2 has ORIENT("R0") and then ORIENT("R90"): turned R90.
*! R3 has x=20, then y=3, then x=25: drawn at (25, 3). R4's first segment is high and its second normal: the high one is kept, drawn at (0, -10).
R1 a1 b1 1k
R2 a2 b2 2k
R3 a3 b3 3k
R4 a4 b4 4k
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   #R1 @PLACE(0, 0) => @PLACE(5, 5);
*   #R2 @PLACE(10, 0) => @ORIENT("R0") => @ORIENT("R90");
*   #R3 @PLACE(x=20) => @PLACE(y=3) => @PLACE(x=25);
*   #R4 @PLACE(0, -10, priority="high") => @PLACE(5, -15)
* }
.end
