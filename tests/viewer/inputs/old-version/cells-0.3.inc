* A library written in RNL 0.3
* @RNL(type="header") { @VERSION("0.3") }
.subckt rdiv a b
R1 a b 1k
R2 b 0 1k
* @RNL { #R1 @PLACE(0, 0); #R2 @PLACE(0, -4) }
.ends
