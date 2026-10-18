* T1 RC low-pass filter
* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="rnl-std", version="0.2")}
VIN in 0 DC 0 AC 1
R1 in out 1k
C1 out 0 1n
* @RNL {
*   {#VIN #R1 #C1} @AS $FLOW;
*   $WIRE_NET_SET @ROUTE
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:c0f121faee1a781e.22233424deb2d8ec") {
*   %gnd_stub_VIN_n = $GND_STUB @ATTACH(gap=2) #VIN:n;
*   %gnd_stub_C1_n = $GND_STUB @ATTACH(gap=2) #C1:n;
*   #VIN @PLACE(-8, 3) => @ORIENT("MXR90");
*   #R1 @PLACE(0, 3) => @ORIENT("R90");
*   #C1 @PLACE(6, 0) => @ORIENT("R0");
*   %gnd_stub_VIN_n::g @PLACE(-11, 1) => @ORIENT("R0");
*   %gnd_stub_C1_n::g @PLACE(6, -4) => @ORIENT("R0");
*   &in @WIRE(mode="straight") {#VIN:p #R1:p};
*   &out @WIRE(mode="straight") {#R1:n (6, 3) #C1:p}
* }
.end
