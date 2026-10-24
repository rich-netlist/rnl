* amplifier cell
.subckt amp in out
R1 in out 1k
C1 out 0 1p
* @RNL { #R1 @PLACE(0, 0); #C1 @PLACE(10, 0) }
.ends amp
