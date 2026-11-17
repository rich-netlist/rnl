* Resistively loaded NMOS differential pair with a current-source tail
* Two NMOS transistors with matched resistor loads and an ideal tail current source, driven by two input sources.
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4") }
.model nch nmos level=1 vto=0.5 kp=200u lambda=0.1
VDD vdd 0 DC 1.8
VIP inp 0 DC 0.9 AC 0.5
VIN inn 0 DC 0.9 AC 0.5 180
RD1 vdd outn 10k
RD2 vdd outp 10k
M1 outn inp tail 0 nch W=10u L=0.5u
M2 outp inn tail 0 nch W=10u L=0.5u
ITAIL tail 0 DC 100u
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   $NET_LABEL @ATTACH &outn => %po_n;
*   $NET_LABEL @ATTACH &outp => %po_p;
*   %routed_nets = # @SELECT-NET;
*   %routed_nets @ROUTE-NET;
*
*   /* the static currents (15.2), for the solver to draw them downwards; not criteria */
*   {#RD1:p #RD2:p #M1:d #M2:d #ITAIL:p} @CURRENT("in");
*   {#RD1:n #RD2:n #M1:s #M2:s #ITAIL:n} @CURRENT("out");
*
*   /* what the circuit is: an NMOS pair with matched resistor loads and an ideal tail current source */
*   %dp = #M1(left) #M2(right) #ITAIL(tail) @AS $DIFFERENTIAL_PAIR_N;
*   %rd = {#RD1 #RD2} @AS $MATCHED_RESISTOR;
*   %amp = %dp(transconductor) %rd(load) @AS $TRANSCONDUCTOR_Z_AMPLIFIER;
*   %flow = {%dp #RD1 #RD2}(path) {#VIP #VIN}(source) {&outn &outp}(load) @AS $SIGNAL_FLOW;
*
*   /* each side is a branch from the top down, the load resistor right above its transistor; the pair and the tail are
*      the pair's own */
*   %left_branch = {#RD1 #M1} @AS $BRANCH;
*   %right_branch = {#RD2 #M2} @AS $BRANCH;
*
*   /* from left to right the supply source, the input source, the two sides, the other input source */
*   %column_order = {#VDD #VIP %left_branch %right_branch #VIN} @ORDER("left-to-right");
*
*   /* the input sources are mirror images about the pair's axis, the output tags too */
*   %src_mirror = #VIP(left) #VIN(right) #ITAIL(axis) @AS $MIRRORED_LAYOUT;
*   %tag_mirror = %po_n(left) %po_p(right) #ITAIL(axis) @AS $MIRRORED_LAYOUT;
*
*   /* only the supply, the input source and the tail source are oriented */
*   %upright_parts = {#VDD #VIP #ITAIL} @ORIENT("R0")
* }
.end
