* A file of SPICE that a Spectre file includes: it drops the rules of Spectre
* @RNL(type="header") { @VERSION("0.3")}
.subckt spsub a b
I1 a b 1m
R1 a b 1k
* @RNL { #I1 @PLACE(0, 0); #R1 @PLACE(10, 0) }
.ends
