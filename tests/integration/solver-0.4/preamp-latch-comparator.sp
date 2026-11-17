* Comparator: two diode-loaded preamplifier stages and a StrongARM latch with its reset switches
* The two preamplifiers are NMOS pairs with diode-connected PMOS loads and a biased tail; the latch is clocked,
* with cross-coupled pairs and reset switches. The top level chains them and loads the outputs with capacitors.
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-no-instance-labels", version="0.4"); @USE-PACKAGE(name="rnl-std-semantics", version="0.4"); @VERSION("0.4")}
.model nch nmos level=1 vto=0.5 kp=200u lambda=0.1
.model pch pmos level=1 vto=-0.5 kp=80u lambda=0.1

* A fully differential preamplifier with diode loads: outp rises with inp - inn (gain about 2)
.subckt diff_preamp inp inn outp outn vdd vss vbias
M1 outn inp tail vss nch W=16u L=1u
M2 outp inn tail vss nch W=16u L=1u
M3 outn outn vdd vdd pch W=8u L=1u
M4 outp outp vdd vdd pch W=8u L=1u
M5 tail vbias vss vss nch W=8u L=1u
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   $IO_PORT @ATTACH &inp => %p_inp;
*   $IO_PORT @ATTACH &inn => %p_inn;
*   $IO_PORT @ATTACH &outp => %p_outp;
*   $IO_PORT @ATTACH &outn => %p_outn;
*   $IO_PORT @ATTACH &vbias => %p_vbias;
*
*   %routed_nets = # @SELECT-NET;
*   %routed_nets @ROUTE-NET;
*
*   /* the static currents (15.2), for the solver to draw them downwards; not criteria */
*   {#M1:d #M2:d #M5:d #M3:s #M4:s} @CURRENT("in");
*   {#M1:s #M2:s #M5:s #M3:d #M4:d} @CURRENT("out");
*
*   /* what the circuit is: an NMOS pair with diode-connected PMOS loads and a biased tail; the ports are mirror images
*      about the tail */
*   %dp = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
*   %loads = {#M3 #M4} @AS $DIODE_CONNECTED;
*   %load = #M3(left) #M4(right) #M5(axis) @AS $MIRRORED_LAYOUT;
*   %matched_loads = {#M3 #M4} @AS $MATCHED_MOS;
*   %in_mirror = %p_inp(left) %p_inn(right) #M5(axis) @AS $MIRRORED_LAYOUT;
*   %out_mirror = %p_outn(left) %p_outp(right) #M5(axis) @AS $MIRRORED_LAYOUT;
*   %amp = %dp(transconductor) %load(load) @AS $TRANSCONDUCTOR_Z_AMPLIFIER;
*   %flow = {%amp}(path) {%p_inp %p_inn}(source) {%p_outn %p_outp}(load) @AS $SIGNAL_FLOW;
*
*   /* the two sides are branches from the top down: the diode load over the input transistor; the tail below the pair
*      is the pair's own */
*   %br_left = {#M3 #M1} @AS $BRANCH;
*   %br_right = {#M4 #M2} @AS $BRANCH;
*
*   /* the amplifier with its signal ports; the bias port stays outside */
*   %amp_group = %amp %p_inp %p_inn %p_outp %p_outn @GROUP;
*
*   /* the left ports face left, the right ports face right; the bias port is at the tail's gate */
*   %left_ports_orient = {%p_inp %p_outn %p_vbias} @ORIENT("left");
*   %right_ports_orient = {%p_inn %p_outp} @ORIENT("right");
*
*   /* the block symbol: a trapezoid with differential inputs/outputs and spaced bottom pins */
*   $PREAMP_SYM = @DEFINE-SYMBOL(box=[-5, -3, 5, 3]) {
*     (-5, 1) @PIN(terminal="inp", direction="left");
*     (-5, -1) @PIN(terminal="inn", direction="left");
*     (5, 1) @PIN(terminal="outp", direction="right");
*     (5, -1) @PIN(terminal="outn", direction="right");
*     (0, 3) @PIN(terminal="vdd", direction="up");
*     (2, -3) @PIN(terminal="vss", direction="down");
*     (-2, -3) @PIN(terminal="vbias", direction="down");
*     @LINE {(-4, 2.5) (4, 1.5) (4, -1.5) (-4, -2.5) (-4, 2.5)};
*     @LINE {(-5, 1) (-4, 1)};
*     @LINE {(-5, -1) (-4, -1)};
*     @LINE {(4, 1) (5, 1)};
*     @LINE {(4, -1) (5, -1)};
*     @LINE {(0, 2) (0, 3)};
*     @LINE {(2, -1.75) (2, -3)};
*     @LINE {(-2, -2.25) (-2, -3)};
*     @LINE {(-3.6, 1) (-2.8, 1)};
*     @LINE {(-3.2, 1.4) (-3.2, 0.6)};
*     @LINE {(-3.6, -1) (-2.8, -1)};
*     ? @TEXT("${name}", key="name") => @PLACE(0.8, 3.6)
*   };
*   # @SET-SYMBOL $PREAMP_SYM
* }
.ends diff_preamp

* A StrongARM latch with the four reset switches: outp goes high when inp > inn as clk rises
.subckt latch inp inn outp outn clk vdd vss
M0 tail clk vss vss nch W=4u L=0.18u
M1 p inp tail vss nch W=4u L=0.18u
M2 q inn tail vss nch W=4u L=0.18u
M3 outn outp p vss nch W=2u L=0.18u
M4 outp outn q vss nch W=2u L=0.18u
M5 outn outp vdd vdd pch W=4u L=0.18u
M6 outp outn vdd vdd pch W=4u L=0.18u
MS1 outn clk vdd vdd pch W=2u L=0.18u
MS2 outp clk vdd vdd pch W=2u L=0.18u
MS3 p clk vdd vdd pch W=2u L=0.18u
MS4 q clk vdd vdd pch W=2u L=0.18u
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   $IO_PORT @ATTACH &inp => %p_inp;
*   $IO_PORT @ATTACH &inn => %p_inn;
*   $IO_PORT @ATTACH &outp => %p_outp;
*   $IO_PORT @ATTACH &outn => %p_outn;
*
*   /* the outputs are named at the drains of the PMOS latch; the clock uses net-label stubs */
*   &clk @UNROUTE-NET(priority="high");
*   %routed_nets = # @SELECT-NET;
*   %routed_nets @ROUTE-NET;
*
*   /* the static currents (15.2) while the latch evaluates; not criteria */
*   {#M0:d #M1:d #M2:d #M3:d #M4:d #M5:s #M6:s} @CURRENT("in");
*   {#M0:s #M1:s #M2:s #M3:s #M4:s #M5:d #M6:d} @CURRENT("out");
*
*   /* what the circuit is: a clocked latch, an input pair with a clocked tail, an NMOS and a PMOS cross-coupled pair, and
*      reset switches on the outputs and on the internal nodes */
*   %dp = #M1(left) #M2(right) #M0(tail) @AS $DIFFERENTIAL_PAIR_N;
*   %cc_n = #M3(left) #M4(right) @AS $CROSS_COUPLED_PAIR;
*   %cc_p = #M5(left) #M6(right) @AS $CROSS_COUPLED_PAIR;
*   %tail_switch = #M0 @AS $GROUND_SWITCH;
*   %reset_switches = {#MS1 #MS2 #MS3 #MS4} @AS $RESET_SWITCH;
*   %power_switches = {#MS1 #MS2 #MS3 #MS4} @AS $POWER_SWITCH;
*   %latch_core = {%cc_n %cc_p %dp} @AS $LATCH;
*
*   /* one stack per side from the top down: the PMOS of the latch, the NMOS of the latch, the input transistor; the
*      clocked tail below the pair is the pair's own */
*   %br_left = {#M5 #M3 #M1} @AS $BRANCH;
*   %br_right = {#M6 #M4 #M2} @AS $BRANCH;
*
*   /* the reset switches, the input ports and the output ports are mirror images about the tail */
*   %rst_out_mirror = #MS1(left) #MS2(right) #M0(axis) @AS $MIRRORED_LAYOUT;
*   %rst_int_mirror = #MS3(left) #MS4(right) #M0(axis) @AS $MIRRORED_LAYOUT;
*   %in_mirror = %p_inp(left) %p_inn(right) #M0(axis) @AS $MIRRORED_LAYOUT;
*   %out_mirror = %p_outn(left) %p_outp(right) #M0(axis) @AS $MIRRORED_LAYOUT;
*
*   /* the left switch gates and ports face left, the right ones face right */
*   %rst_gate_left = {#MS1:g #MS3:g} @ORIENT("left");
*   %rst_gate_right = {#MS2:g #MS4:g} @ORIENT("right");
*   %left_ports_orient = {%p_inp %p_outn} @ORIENT("left");
*   %right_ports_orient = {%p_inn %p_outp} @ORIENT("right");
*
*   /* the block symbol: a box with a clock wedge at its clock pin */
*   $LATCH_SYM = @DEFINE-SYMBOL(box=[-5, -3, 5, 3]) {
*     (-5, 1) @PIN(terminal="inp", direction="left");
*     (-5, -1) @PIN(terminal="inn", direction="left");
*     (5, 1) @PIN(terminal="outp", direction="right");
*     (5, -1) @PIN(terminal="outn", direction="right");
*     (0, 3) @PIN(terminal="vdd", direction="up");
*     (2, -3) @PIN(terminal="vss", direction="down");
*     (-2, -3) @PIN(terminal="clk", direction="down");
*     @LINE {(-4, 2) (4, 2) (4, -2) (-4, -2) (-4, 2)};
*     @LINE {(-5, 1) (-4, 1)};
*     @LINE {(-5, -1) (-4, -1)};
*     @LINE {(4, 1) (5, 1)};
*     @LINE {(4, -1) (5, -1)};
*     @LINE {(0, 2) (0, 3)};
*     @LINE {(2, -2) (2, -3)};
*     @LINE {(-2, -2) (-2, -3)};
*     @LINE {(-2.6, -2) (-2, -1.4) (-1.4, -2)};
*     ? @TEXT("${name}", key="name") => @PLACE(0.8, 3.6)
*   };
*   # @SET-SYMBOL $LATCH_SYM
* }
.ends latch

XPRE1 inp inn p1p p1n vdd 0 vb diff_preamp
XPRE2 p1p p1n p2p p2n vdd 0 vb diff_preamp
XL p2p p2n outp outn clk vdd 0 latch
CL1 outp 0 50f
CL2 outn 0 50f
VDD vdd 0 DC 1.8
VB vb 0 DC 0.7
VCLK clk 0 PULSE(0 1.8 0 50p 50p 0.45n 1n)
VIP inp 0 DC 0.905
VIN inn 0 DC 0.895
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vb @ADD-TO $BIAS_NET_SET;
*   &clk @UNROUTE-NET(priority="high");
*
*   %routed_nets = # @SELECT-NET;
*   %routed_nets @ROUTE-NET;
*
*   /* what the circuit is: two preamplifiers and the latch in a chain, driven by two sources, loaded by two capacitors */
*   %chain = {#XPRE1 #XPRE2 #XL} @AS $CASCADE;
*   %matched_load_caps = {#CL1 #CL2} @AS $MATCHED_CAPACITOR;
*   %flow = {#XPRE1 #XPRE2 #XL}(path) {#VIP #VIN}(source) {#CL1 #CL2}(load) @AS $SIGNAL_FLOW;
*
*   /* from left to right the input sources, the chain of two preamplifiers and the latch, the load capacitors */
*   %testbench_order = {{#VIP #VIN} #XPRE1 #XPRE2 #XL {#CL1 #CL2}} @ORDER("left-to-right");
*
*   /* the supply, bias and clock sources in one row below the chain, each under its stage; the supply source stands at
*      least 4 below the chain */
*   %source_row = {#VDD #VB #VCLK} @ALIGN("y");
*   %sources_below = {{#XPRE1 #XPRE2 #XL} {#VB #VCLK}} @ORDER("top-to-bottom");
*   %supply_distance = #VDD {#XPRE1 #XPRE2 #XL} @DISTANCE("bottom-to-top", min=4);
*   %vdd_align = {#VDD #XPRE1} @ALIGN("x");
*   %vb_align = {#VB #XPRE2} @ALIGN("x");
*   %vclk_align = {#VCLK #XL} @ALIGN("x");
*
*   /* only the sources, the loads and the stages are oriented */
*   %upright_parts = {#XPRE1 #XPRE2 #XL #VIP #VIN #CL1 #CL2 #VDD #VB #VCLK} @ORIENT("R0");
* }
.end
