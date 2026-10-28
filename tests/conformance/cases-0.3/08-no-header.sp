* A main file without a header record is read in 0.3
*! The main file has no header record: it is read as the tool's default version 0.3 (16.1), so the implied standard package is the 0.3 series too (16.3).
*! $RESISTOR is a name of the 0.3 series, which the 0.2 series lacks; T1 has the default terminal names p1 n1 p2 n2 that 0.3 gives (6.12), and the wires connect by those names.
R1 in out 1k
C1 out 0 1p
T1 out 0 far 0 z0=50 td=1n
R2 far 0 50
* @RNL {
*   #R1 @SET-SYMBOL $RESISTOR => @PLACE(0, 0);
*   #C1 @PLACE(4, -4);
*   #T1 @PLACE(10, 0);
*   #R2 @PLACE(16, -4);
*   &far @WIRE(mode="straight") {#T1:p2 #R2:1}
* }
.end
