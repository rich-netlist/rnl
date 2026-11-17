* T2b Common-source stage, solved once, then VIN dragged in an editor
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4")}
.model nch nmos level=1
M1 out in 0 0 nch W=4u L=1u
RD vdd out 5k
VDD vdd 0 1.8
VIN in 0 DC 0.6 AC 1
* @RNL {
*   # @SELECT-INSTANCE(model="nch", recursive=true) => @SET-SYMBOL $NMOS;
*   {#RD #M1} @ALIGN("x");
*   #M1 #RD @DISTANCE("bottom-to-top", min=2);
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   {#VIN #M1} @AS $SIGNAL_FLOW; {#VIN #M1} @ORDER("left-to-right", priority="low");
*   # @SELECT-NET => @ROUTE-NET
* }
* @RNL(author="rnl-editor 0.1") { #VIN @PLACE(x=-4, priority="high")}
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:caf7d766f97cf58c.a797f739752c5cf1", numbering="788d9cd4b658081e") {
*   %ground_stub_M1_s = $GROUND_STUB @ATTACH(gap=3) #M1::s;
*   %ground_stub_VDD_n = $GROUND_STUB @ATTACH(gap=3) #VDD::n;
*   %ground_stub_VIN_n = $GROUND_STUB @ATTACH(gap=3) #VIN::n;
*   %supply_stub_RD_p = $SUPPLY_STUB @ATTACH(gap=3) #RD::p;
*   %supply_stub_VDD_p = $SUPPLY_STUB @ATTACH(gap=3) #VDD::p;
*   #M1 @PLACE(5, 4) => @ORIENT("R0");
*   #RD @PLACE(5, 10) => @ORIENT("R0");
*   #VDD @PLACE(-11, 0) => @ORIENT("MY");
*   #VIN @PLACE(-4, 0) => @ORIENT("R0");
*   %#1 @PLACE(2, 6);
*   %ground_stub_M1_s @PLACE(2, 6);
*   %ground_stub_M1_s::g @PLACE(5, 0) => @ORIENT("R0");
*   %ground_stub_VDD_n @PLACE(2, 6);
*   %ground_stub_VDD_n::g @PLACE(-11, -4) => @ORIENT("R0");
*   %ground_stub_VIN_n @PLACE(2, 6);
*   %ground_stub_VIN_n::g @PLACE(-4, -4) => @ORIENT("R0");
*   %supply_stub_RD_p @PLACE(2, 6);
*   %supply_stub_RD_p::s @PLACE(5, 14) => @ORIENT("R0");
*   %supply_stub_VDD_p @PLACE(2, 6);
*   %supply_stub_VDD_p::s @PLACE(-11, 4) => @ORIENT("R0");
*   &out @WIRE(mode="straight") {#M1::d #RD::n};
*   &in @WIRE(mode="straight") {#VIN::p (-4, 4) #M1::g}
* }
.end
