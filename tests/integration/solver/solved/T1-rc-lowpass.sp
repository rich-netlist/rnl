* T1 RC low-pass filter
* @RNL(type="header") { @VERSION("0.2")}
VIN in 0 DC 0 AC 1
R1 in out 1k
C1 out 0 1n
* @RNL {
*   {#VIN #R1 #C1} @AS $FLOW;
*   $WIRE_NET_SET @ROUTE
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig1:f77bf03d1ee4f145.e17f1522b2b059c0") {
*   %gnd_stub_VIN_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VIN:n;
*   %gnd_stub_C1_n = $GND_STUB @ATTACH(gap=2, priority="normal") #C1:n;
*   #VIN @PLACE(-7, 3) => @ORIENT("MY");
*   #R1 @PLACE(0, 6) => @ORIENT("R90");
*   #C1 @PLACE(7, 4) => @ORIENT("R0");
*   %gnd_stub_VIN_n::g @PLACE(-7, -2) => @ORIENT("R0");
*   %gnd_stub_C1_n::g @PLACE(7, 0) => @ORIENT("R0");
*   &in @WIRE(mode="straight") {#VIN:p (-7, 7) (-4, 7) (-4, 6) #R1:p};
*   &out @WIRE(mode="straight") {#R1:n (5, 6) (5, 7) (7, 7) #C1:p}
* }
.end
