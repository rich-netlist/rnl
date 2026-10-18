* T2b Common-source stage, solved once, then VIN dragged in an editor
* @RNL(type="header") { @VERSION("0.3")}
.model nch nmos level=1
M1 out in 0 0 nch W=4u L=1u
RD vdd out 5k
VDD vdd 0 1.8
VIN in 0 DC 0.6 AC 1
* @RNL {
*   # @SELECT-INSTANCE(model="nch") => @SET-SYMBOL $NMOS;
*   {#RD #M1} @ALIGN("x");
*   #M1 #RD @DISTANCE("bottom-to-top", min=2);
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   {#VIN #M1} @AS $FLOW;
*   $WIRE_NET_SET @ROUTE
* }
* @RNL(author="rnl-editor 0.1") { #VIN @PLACE(x=-4, priority="high")}
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:7ced2c3e7b625ce8.22233424deb2d8ec") {
*   %ground_stub_M1_s = $GROUND_STUB @ATTACH(gap=2) #M1:s;
*   %ground_stub_VDD_n = $GROUND_STUB @ATTACH(gap=2) #VDD:n;
*   %ground_stub_VIN_n = $GROUND_STUB @ATTACH(gap=2) #VIN:n;
*   %supply_stub_RD_p = $SUPPLY_STUB @ATTACH(gap=2) #RD:p;
*   %supply_stub_VDD_p = $SUPPLY_STUB @ATTACH(gap=2) #VDD:p;
*   #M1 @PLACE(9, 3) => @ORIENT("R0");
*   #RD @PLACE(9, 10) => @ORIENT("R0");
*   #VDD @PLACE(-12, 0) => @ORIENT("R0");
*   #VIN @PLACE(-4, 0) => @ORIENT("R0");
*   %ground_stub_M1_s::g @PLACE(9, -1) => @ORIENT("R0");
*   %ground_stub_VDD_n::g @PLACE(-12, -4) => @ORIENT("R0");
*   %ground_stub_VIN_n::g @PLACE(-4, -4) => @ORIENT("R0");
*   %supply_stub_RD_p::s @PLACE(9, 14) => @ORIENT("R0");
*   %supply_stub_VDD_p::s @PLACE(-12, 4) => @ORIENT("R0");
*   &out @WIRE(mode="straight") {#M1:d (12, 5) (12, 8) #RD:n};
*   &in @WIRE(mode="straight") {#VIN:p (4, 2) (4, 3) #M1:g}
* }
.end
