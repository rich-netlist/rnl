* A netlist for the container tests of the local viewer: two resistors placed, so that the page draws something at once, and a third
* that is left to the solver, so that a solve has something to do
R1 a b 1k
R2 b 0 1k
R3 b c 1k
* @RNL(type="header") { @VERSION("0.3") }
* @RNL { #R1 @PLACE(0, 0); #R2 @PLACE(9, 0); {#R1 #R2} @ALIGN("y") }
.end
