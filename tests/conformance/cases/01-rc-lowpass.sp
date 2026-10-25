* RC low-pass filter
*! Basics: the standard package gives symbols by element letter; ground drawn by $GROUND_NET_SET on unconnected pins; how wires run; a junction, with a text beside it taking a point's coordinate through a role, ${p::x} (13.6); a note; a fixed solver block, where the ground stub is written back by the "solver".
V1 in 0 DC 0 AC 1
R1 in out 1k
C1 out 0 1n
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   &in @WIRE(mode="vertical-horizontal") {#V1:1 #R1:1};
*   %j = (12, 6);
*   &out @WIRE(mode="horizontal-vertical") {#R1:2 %j #C1:1};
*   %o = (17, 6);
*   &out @WIRE {%j %o};
*   $JUNCTION @ATTACH %j => @TEXT("(${p::x}, ${p::y})", dx=0.4, dy=-0.9, size=0.7);
*   %o @TEXT("V_{out}", dx=0.4, dy=0.8);
*   (6, 11) @TEXT("RC 低通：f_{c} = 1/(2\pi RC) ≈ 159 kHz", halign="middle")
* }
* @RNL(type="generated", priority="high") {
*   #V1 @PLACE(0, 0);
*   #R1 @PLACE(6, 6) => @ORIENT("R90");
*   #C1 @PLACE(12, 3);
*   %gs_V1_2 = $GND_STUB @ATTACH #V1:2; %gs_V1_2::g @PLACE(0, -5);
*   %gs_C1_2 = $GND_STUB @ATTACH #C1:2; %gs_C1_2::g @PLACE(12, -1)
* }
.end
