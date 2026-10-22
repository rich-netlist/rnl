* Unkeyed labels attached to different wires need independent display groups.
* @RNL(type="header") { @VERSION("0.3") }
R1 in mid 1k
R2 mid out 2k
* @RNL {
*   #R1 @PLACE(0, 0); #R2 @PLACE(8, 0);
*   %input = &in @WIRE {(-8, 2) #R1:1};
*   %output = &out @WIRE {#R2:2 (14, -2)};
*   %input @TEXT("IN") => @PLACE(-5, 3);
*   %output @TEXT("OUT") => @PLACE(11, -1)
* }
.end
