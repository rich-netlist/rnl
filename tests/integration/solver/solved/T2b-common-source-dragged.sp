* T2b Common-source stage, solved once, then VIN dragged in an editor
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
* @RNL(author="rnl-editor 0.1") { #VIN @PLACE(x=-4, priority="high")}
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:233a44ba5ee989e1.1f3b310f8bee492a") {
*   %gnd_stub_M1_s = $GND_STUB @ATTACH(gap=2) #M1:s;
*   %gnd_stub_VDD_n = $GND_STUB @ATTACH(gap=2) #VDD:n;
*   %gnd_stub_VIN_n = $GND_STUB @ATTACH(gap=2) #VIN:n;
*   %supply_stub_RD_p = $SUPPLY_STUB @ATTACH(gap=2) #RD:p;
*   %supply_stub_VDD_p = $SUPPLY_STUB @ATTACH(gap=2) #VDD:p;
*   #M1 @PLACE(3, -8) => @ORIENT("R0");
*   #RD @PLACE(3, 0) => @ORIENT("R0");
*   #VDD @PLACE(-4, 0) => @ORIENT("R0");
*   #VIN @PLACE(-4, -12) => @ORIENT("MY");
*   %gnd_stub_M1_s::g @PLACE(3, -12) => @ORIENT("R0");
*   %gnd_stub_VDD_n::g @PLACE(-4, -5) => @ORIENT("R0");
*   %gnd_stub_VIN_n::g @PLACE(-4, -17) => @ORIENT("R0");
*   %supply_stub_RD_p::s @PLACE(3, 5) => @ORIENT("R0");
*   %supply_stub_VDD_p::s @PLACE(-4, 5) => @ORIENT("R0");
*   &out @WIRE(mode="straight") {#M1:d #RD:n};
*   &in @WIRE(mode="straight") {#VIN:p (-4, -8) #M1:g}
* }
.end
