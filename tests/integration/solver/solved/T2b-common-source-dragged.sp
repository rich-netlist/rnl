* T2b Common-source stage, solved once, then VIN dragged in an editor
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4")}
.model nch nmos level=1
M1 out in 0 0 nch W=4u L=1u
RD vdd out 5k
VDD vdd 0 1.8
VIN in 0 DC 0.6 AC 1
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   %routed_nets = # @SELECT-NET;
*   %routed_nets @ROUTE-NET;
*
*   /* the static currents (15.2), for the solver to draw them downwards */
*   {#M1:d #RD:p} @CURRENT("in");
*   {#M1:s #RD:n} @CURRENT("out");
*
*   /* what the circuit is: one stage, its drain branch from the top down, after the source */
*   %amp = #M1(transconductor) #RD(load) @AS $TRANSCONDUCTOR_Z_AMPLIFIER;
*   %flow = {#M1}(path) {#VIN}(source) {&out}(load) @AS $SIGNAL_FLOW;
*   %branch = {#RD #M1} @AS $BRANCH;
*   %order = {#VIN %branch} @ORDER("left-to-right");
*   {#VDD #VIN} @ORIENT("R0")
* }
* @RNL(author="rnl-editor 0.1") { #VIN @PLACE(x=-4, priority="high")}
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:7e5f62816c3ac64c.8f59b260a417cb37", numbering="e01fe77925c0d466") {
*   %ground_stub_M1_s = $GROUND_STUB @ATTACH(gap=3) #M1::s;
*   %ground_stub_VDD_n = $GROUND_STUB @ATTACH(gap=3) #VDD::n;
*   %ground_stub_VIN_n = $GROUND_STUB @ATTACH(gap=3) #VIN::n;
*   %supply_stub_RD_p = $SUPPLY_STUB @ATTACH(gap=3) #RD::p;
*   %supply_stub_VDD_p = $SUPPLY_STUB @ATTACH(gap=3) #VDD::p;
*   #M1 @PLACE(5, 1) => @ORIENT("R0");
*   #RD @PLACE(5, 7) => @ORIENT("R0");
*   #VDD @PLACE(-4, 11) => @ORIENT("R0");
*   #VIN @PLACE(-4, -3) => @ORIENT("R0");
*   %amp @PLACE(0, 13);
*   %flow @PLACE(0, 13);
*   %branch @PLACE(5, 5);
*   %#1 @PLACE(0, 13);
*   %ground_stub_M1_s @PLACE(0, 13);
*   %ground_stub_M1_s::g @PLACE(5, -3) => @ORIENT("R0");
*   %ground_stub_VDD_n @PLACE(0, 13);
*   %ground_stub_VDD_n::g @PLACE(-4, 7) => @ORIENT("R0");
*   %ground_stub_VIN_n @PLACE(0, 13);
*   %ground_stub_VIN_n::g @PLACE(-4, -7) => @ORIENT("R0");
*   %supply_stub_RD_p @PLACE(0, 13);
*   %supply_stub_RD_p::s @PLACE(5, 11) => @ORIENT("R0");
*   %supply_stub_VDD_p @PLACE(0, 13);
*   %supply_stub_VDD_p::s @PLACE(-4, 15) => @ORIENT("R0");
*   &out @WIRE(mode="straight") {#M1::d #RD::n};
*   &in @WIRE(mode="straight") {#VIN::p (-4, 1) #M1::g}
* }
.end
