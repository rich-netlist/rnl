* T2 Common-source stage
* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="rnl-std", version="0.2")}
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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:df472807084943cf.1f3b310f8bee492a") {
*   %gnd_stub_M1_s = $GND_STUB @ATTACH(gap=2, priority="normal") #M1:s;
*   %gnd_stub_VDD_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VDD:n;
*   %gnd_stub_VIN_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VIN:n;
*   %supply_stub_RD_p = $SUPPLY_STUB @ATTACH(gap=2, priority="normal") #RD:p;
*   %supply_stub_VDD_p = $SUPPLY_STUB @ATTACH(gap=2, priority="normal") #VDD:p;
*   #M1 @PLACE(7, -8) => @ORIENT("R0");
*   #RD @PLACE(7, 0) => @ORIENT("R0");
*   #VDD @PLACE(0, 0) => @ORIENT("R0");
*   #VIN @PLACE(0, -12) => @ORIENT("MY");
*   %gnd_stub_M1_s::g @PLACE(7, -12) => @ORIENT("R0");
*   %gnd_stub_VDD_n::g @PLACE(0, -5) => @ORIENT("R0");
*   %gnd_stub_VIN_n::g @PLACE(0, -17) => @ORIENT("R0");
*   %supply_stub_RD_p::s @PLACE(7, 5) => @ORIENT("R0");
*   %supply_stub_VDD_p::s @PLACE(0, 5) => @ORIENT("R0");
*   &out @WIRE(mode="straight") {#M1:d #RD:n};
*   &in @WIRE(mode="straight") {#VIN:p (0, -8) #M1:g}
* }
.end
