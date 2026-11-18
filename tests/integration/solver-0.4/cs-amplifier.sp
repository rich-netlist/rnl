* Common-source amplifier: voltage-divider bias, source degeneration with bypass, RC coupling
* The divider RG1/RG2 biases the gate of M1 (3.3 V supply), the source resistor RS with its bypass capacitor CS
* degenerates the source, CIN and COUT couple the signal in and out.
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4") }
.model nch nmos level=1 vto=0.5 kp=200u lambda=0.05
VDD vdd 0 DC 3.3
VSIG sig 0 DC 0 AC 1
CIN sig g 1u
RG1 vdd g 200k
RG2 g 0 100k
M1 d g s 0 nch W=20u L=1u
RD vdd d 4.7k
RS s 0 1k
CS s 0 10u
COUT d out 1u
RL out 0 10k
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   %routed_nets = # @SELECT-NET;
*   %routed_nets @ROUTE-NET;
*
*   /* the static currents (15.2), for the solver to draw them downwards; not criteria */
*   {#M1:d #RD:p #RS:p #RG1:p #RG2:p} @CURRENT("in");
*   {#M1:s #RD:n #RS:n #RG1:n #RG2:n} @CURRENT("out");
*
*   /* what the circuit is: M1 amplifies, loaded by RD; RS || CS degenerates the source; RG1/RG2 divide the supply into the gate bias */
*   %degen = {#RS #CS} @AS $PARALLEL_HORIZONTAL;
*   %amp = #M1(transconductor) #RD(load) %degen(degenerator) @AS $DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER;
*   %div = #RG1(upper) #RG2(lower) @AS $DIVIDER;
*   %bias = {%div} @AS $BIAS_CIRCUIT;
*   /* the amplifier and its bias network are one block; the whole signal chain, from the source to the load, is the outer block */
*   %core_group = {%amp %bias} @GROUP;
*   %chain_group = {#VSIG #CIN %amp %bias #COUT #RL} @GROUP;
*
*   /* the two coupling capacitors lie across the signal path, turned a quarter turn either way */
*   %coupling = {#CIN #COUT} @ORIENT(["R90","R270"]) => @AS $AC_COUPLE;
*   %flow = {#CIN #M1 #COUT}(path) {#VSIG}(source) {#RL}(load) @AS $SIGNAL_FLOW;
*
*   /* two branches, each in one column from the top down: the drain branch and the bias divider */
*   %branch1 = {#RD #M1 #RS} @AS $BRANCH;
*   %branch2 = {#RG1 #RG2} @AS $BRANCH;
*
*   /* the signal goes from left to right: the source, the input coupling capacitor, the bias branch, the drain branch,
*      the output coupling capacitor and the load */
*   %signal_order = {#VSIG #CIN %branch2 %branch1 #COUT #RL} @ORDER("left-to-right");
*
*   /* only the sources and the load are oriented; the rest is left to the solver */
*   %source_load_orient = {#VDD #VSIG #RL} @ORIENT("R0");
*
*   /* the supply source stands on the far left, beside the signal source, in one row with it */
*   %supply_distance = #VDD #VSIG @DISTANCE("left-to-right", min=4);
*   %supply_row = {#VDD #VSIG} @ALIGN("y")
* }
.end
