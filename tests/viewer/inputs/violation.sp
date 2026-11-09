* A net whose CURRENT statements all send the current one way: a violation (0.4), with a warning and a hint beside it
* @RNL(type="header") { @VERSION("0.4")}
R1 a b 1k
R2 b 0 1k
C1 out 0 1p
* @RNL {
*   #R1 @PLACE(0, 0); #R2 @PLACE(0, -4);
*   #R1:2 @CURRENT("in"); #R2:1 @CURRENT("in")
* }
.end
