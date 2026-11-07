* Two placed resistors joined by a wire through a free point: the core draws a flyline between them (13.7)
* @RNL(type="header") { @VERSION("0.3") }
R1 a b 1k
R2 b c 1k
* @RNL { #R1 @PLACE(0, 0); #R2 @PLACE(9, 0); &b @WIRE {#R1:2 (_, _) #R2:1} }
.end
