* T1 RC low-pass filter
* @RNL(type="header") { @VERSION("0.3")}
VIN in 0 DC 0 AC 1
R1 in out 1k
C1 out 0 1n
* @RNL {
*   {#VIN #R1 #C1} @AS $FLOW;
*   $WIRE_NET_SET @ROUTE
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:0535fc6821a663d3.22233424deb2d8ec") {
*   %ground_stub_VIN_n = $GROUND_STUB @ATTACH(gap=2) #VIN:n;
*   %ground_stub_C1_n = $GROUND_STUB @ATTACH(gap=2) #C1:n;
*   #VIN @PLACE(-9, 2) => @ORIENT("R0");
*   #R1 @PLACE(0, 2) => @ORIENT("R0");
*   #C1 @PLACE(6, 2) => @ORIENT("R0");
*   %ground_stub_VIN_n::g @PLACE(-9, -2) => @ORIENT("R0");
*   %ground_stub_C1_n::g @PLACE(6, -2) => @ORIENT("R0");
*   &in @WIRE(mode="straight") {#VIN:p #R1:p};
*   &out @WIRE(mode="straight") {#R1:n (3, 0) (3, 4) #C1:p}
* }
.end
