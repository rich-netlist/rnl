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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig1:00000000") {
*   #M1 @PLACE(10, 0); #RD @PLACE(10, 8); #VIN @PLACE(0, -2); #VDD @PLACE(20, 4)
* }
* @RNL(author="rnl-editor 0.1") { #VIN @PLACE(x=-4, priority="high")}
.end
