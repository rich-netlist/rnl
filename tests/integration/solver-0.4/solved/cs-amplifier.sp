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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:5e1a0c4ce0756568.6812a3e5d5469100", numbering="e4081387d5b0c56a") {
*   %ground_stub_VDD_n = $GROUND_STUB @ATTACH(gap=3) #VDD::n;
*   %ground_stub_VSIG_n = $GROUND_STUB @ATTACH(gap=3) #VSIG::n;
*   %ground_stub_RG2_n = $GROUND_STUB @ATTACH(gap=3) #RG2::n;
*   %ground_stub_RS_n = $GROUND_STUB @ATTACH(gap=3) #RS::n;
*   %ground_stub_CS_n = $GROUND_STUB @ATTACH(gap=3) #CS::n;
*   %ground_stub_RL_n = $GROUND_STUB @ATTACH(gap=3) #RL::n;
*   %supply_stub_VDD_p = $SUPPLY_STUB @ATTACH(gap=3) #VDD::p;
*   %supply_stub_RG1_p = $SUPPLY_STUB @ATTACH(gap=3) #RG1::p;
*   %supply_stub_RD_p = $SUPPLY_STUB @ATTACH(gap=3) #RD::p;
*   %rt1 = (0, 6);
*   %rt2 = (9, 9);
*   %rt3 = (9, 3);
*   #VDD @PLACE(-21, 0) => @ORIENT("R0");
*   #VSIG @PLACE(-11, 0) => @ORIENT("R0");
*   #CIN @PLACE(-8, 6) => @ORIENT("R90");
*   #RG1 @PLACE(0, 9) => @ORIENT("R0");
*   #RG2 @PLACE(0, 3) => @ORIENT("R0");
*   #M1 @PLACE(9, 6) => @ORIENT("R0");
*   #RD @PLACE(9, 13) => @ORIENT("R0");
*   #RS @PLACE(9, 0) => @ORIENT("R0");
*   #CS @PLACE(17, 0) => @ORIENT("R90");
*   #COUT @PLACE(27, 9) => @ORIENT("R90");
*   #RL @PLACE(30, 0) => @ORIENT("R0");
*   %degen @PLACE(0, 0);
*   %#1 @PLACE(0, 0);
*   %amp @PLACE(0, 0);
*   %#2 @PLACE(0, 0);
*   %div @PLACE(0, 0);
*   %#3 @PLACE(0, 0);
*   %bias @PLACE(0, 0);
*   %#4 @PLACE(0, 0);
*   %#5 @PLACE(0, 0);
*   %flow @PLACE(0, 0);
*   %branch1 @PLACE(9, 0);
*   %#6 @PLACE(0, 0);
*   %branch2 @PLACE(0, 0);
*   %#7 @PLACE(0, 0);
*   %ground_stub_VDD_n @PLACE(0, 0);
*   %ground_stub_VDD_n::g @PLACE(-21, -4) => @ORIENT("R0");
*   %ground_stub_VSIG_n @PLACE(0, 0);
*   %ground_stub_VSIG_n::g @PLACE(-11, -4) => @ORIENT("R0");
*   %ground_stub_RG2_n @PLACE(0, 0);
*   %ground_stub_RG2_n::g @PLACE(0, -1) => @ORIENT("R0");
*   %ground_stub_RS_n @PLACE(0, 0);
*   %ground_stub_RS_n::g @PLACE(9, -4) => @ORIENT("R0");
*   %ground_stub_CS_n @PLACE(0, 0);
*   %ground_stub_CS_n::g @PLACE(21, -2) => @ORIENT("R0");
*   %ground_stub_RL_n @PLACE(0, 0);
*   %ground_stub_RL_n::g @PLACE(30, -4) => @ORIENT("R0");
*   %supply_stub_VDD_p @PLACE(0, 0);
*   %supply_stub_VDD_p::s @PLACE(-21, 4) => @ORIENT("R0");
*   %supply_stub_RG1_p @PLACE(0, 0);
*   %supply_stub_RG1_p::s @PLACE(0, 13) => @ORIENT("R0");
*   %supply_stub_RD_p @PLACE(0, 0);
*   %supply_stub_RD_p::s @PLACE(9, 17) => @ORIENT("R0");
*   &sig @WIRE(mode="straight") {#VSIG::p (-11, 6) #CIN::p};
*   &g @WIRE(mode="straight") {#RG2::p %rt1};
*   &g @WIRE(mode="straight") {%rt1 #RG1::n};
*   &g @WIRE(mode="straight") {#CIN::n %rt1};
*   &g @WIRE(mode="straight") {#M1::g %rt1};
*   &d @WIRE(mode="straight") {#M1::d %rt2};
*   &d @WIRE(mode="straight") {%rt2 #RD::n};
*   &d @WIRE(mode="straight") {#COUT::p %rt2};
*   &s @WIRE(mode="straight") {#RS::p %rt3};
*   &s @WIRE(mode="straight") {%rt3 #M1::s};
*   &s @WIRE(mode="straight") {#CS::p (12, 0) (12, 3) %rt3};
*   &out @WIRE(mode="straight") {#COUT::n (30, 9) #RL::p};
*   %#12 @WIRE(mode="straight") {#CS::n (21, 0) %ground_stub_CS_n::g::pin};
*   $JUNCTION @ATTACH %rt1 => @PLACE(0, 6);
*   $JUNCTION @ATTACH %rt2 => @PLACE(9, 9);
*   $JUNCTION @ATTACH %rt3 => @PLACE(9, 3)
* }
.end
