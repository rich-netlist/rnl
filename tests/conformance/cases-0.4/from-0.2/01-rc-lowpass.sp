* RC low-pass filter
*! Basics: the standard package gives symbols by element letter; ground drawn by $GROUND_NET_SET on unconnected pins; how wires run; a junction, with a text beside it taking a point's coordinate through a role, ${p::x} (13.6); a note; a fixed solver block, where the ground stub is written back by the "solver".
V1 in 0 DC 0 AC 1
R1 in out 1k
C1 out 0 1n
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-basic", version="0.4"); @VERSION("0.4")}
* @RNL {
*   %input_corner = (0, 6);
*   &in @WIRE(mode="auto") {#V1:1 %input_corner #R1:1};
*   %j = (12, 6);
*   &out @WIRE(mode="auto") {#R1:2 %j #C1:1};
*   %o = (17, 6);
*   &out @WIRE(mode="auto") {%j %o};
*   $JUNCTION @ATTACH %j => @TEXT("(${p::x}, ${p::y})", dx=0.4, dy=-0.9, size=0.7);
*   %o @TEXT("V_{out}", dx=0.4, dy=0.8);
*   (6, 11) @TEXT("RC 低通：f_{c} = 1/(2\pi RC) ≈ 159 kHz", halign="middle")
* }
* 






* Fixed device positions retain the reviewed layout; the 0.4 solver record fills the remaining objects.
* @RNL(priority="high") {
*   #V1 @PLACE(0, 0) => @ORIENT("R0");
*   #R1 @PLACE(6, 6) => @ORIENT("R90");
*   #C1 @PLACE(12, 3) => @ORIENT("R0");
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:a7d9aa54a4c7887f.ffc36700559c31e7", numbering="86d3d5bfaf097545") {
*   %ground_stub_V1_n = $GROUND_STUB @ATTACH(gap=1) #V1::n;
*   %ground_stub_C1_n = $GROUND_STUB @ATTACH(gap=1) #C1::n;
*   #V1 @PLACE(0, 0) => @ORIENT("R0");
*   #R1 @PLACE(6, 6) => @ORIENT("R90");
*   #C1 @PLACE(12, 3) => @ORIENT("R0");
*   %#4 @PLACE(12, 6) => @ORIENT("R0");
*   %ground_stub_V1_n @PLACE(0, 0);
*   %ground_stub_V1_n::g @PLACE(0, -3) => @ORIENT("R0");
*   %ground_stub_C1_n @PLACE(0, 0);
*   %ground_stub_C1_n::g @PLACE(12, 0) => @ORIENT("R0");
*   %j @PLACE(12, 6);
*   %o @PLACE(17, 6);
*   %#7 @PLACE(6, 11);
*   %#8 @PLACE(6, 11)
* }
.end
