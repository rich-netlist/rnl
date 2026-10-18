* T2 Common-source stage
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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:4501cf00b461cd34.22233424deb2d8ec") {
*   %ground_stub_M1_s = $GROUND_STUB @ATTACH(gap=2) #M1:s;
*   %ground_stub_VDD_n = $GROUND_STUB @ATTACH(gap=2) #VDD:n;
*   %ground_stub_VIN_n = $GROUND_STUB @ATTACH(gap=2) #VIN:n;
*   %supply_stub_RD_p = $SUPPLY_STUB @ATTACH(gap=2) #RD:p;
*   %supply_stub_VDD_p = $SUPPLY_STUB @ATTACH(gap=2) #VDD:p;
*   #M1 @PLACE(0, 3) => @ORIENT("R0");
*   #RD @PLACE(0, 10) => @ORIENT("R0");
*   #VDD @PLACE(-21, 0) => @ORIENT("R0");
*   #VIN @PLACE(-13, 0) => @ORIENT("R0");
*   %ground_stub_M1_s::g @PLACE(0, -1) => @ORIENT("R0");
*   %ground_stub_VDD_n::g @PLACE(-21, -4) => @ORIENT("R0");
*   %ground_stub_VIN_n::g @PLACE(-13, -4) => @ORIENT("R0");
*   %supply_stub_RD_p::s @PLACE(0, 14) => @ORIENT("R0");
*   %supply_stub_VDD_p::s @PLACE(-21, 4) => @ORIENT("R0");
*   &out @WIRE(mode="straight") {#M1:d (3, 5) (3, 8) #RD:n};
*   &in @WIRE(mode="straight") {#VIN:p (-5, 2) (-5, 3) #M1:g}
* }
.end
