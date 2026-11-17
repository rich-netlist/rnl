* Tow-Thomas biquad, f0 = 1 kHz, Q = 1: band-pass at bp, low-pass at lp, with ideal op-amps
* Three op-amp stages in a ring: a lossy integrator, an integrator and an inverter, inside the outer feedback loop R1.
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-semantics", version="0.4"); @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-functional", version="0.4")}

* An ideal op-amp: an open-loop gain of 1e5
.subckt opamp inp inn out
E1 out 0 inp inn 1e5
* @RNL {
*   $IO_PORT @ATTACH &inp => %p_inp;
*   $IO_PORT @ATTACH &inn => %p_inn;
*   $IO_PORT @ATTACH &out => %p_out;
*   %routed_nets = # @SELECT-NET;
*   %routed_nets @ROUTE-NET;
*   %flow = {#E1}(path) {%p_inp %p_inn}(source) {%p_out}(load) @AS $SIGNAL_FLOW;
*
*   /* the inputs on the left, + above -, the output on the right */
*   %port_order = {{%p_inp %p_inn} #E1 %p_out} @ORDER("left-to-right");
*   %input_order = {%p_inp %p_inn} @ORDER("top-to-bottom");
*   # @SET-SYMBOL(terminals=[[1, "inp"], [2, "inn"], [3, "out"]]) $OPAMP
* }
.ends opamp

VIN vin 0 DC 0 AC 1
RIN vin n1 10k
RQ n1 bp 10k
C1 n1 bp 15.9n
XOA1 0 n1 bp opamp
R2 bp n2 10k
C2 n2 lp 15.9n
XOA2 0 n2 lp opamp
R3 lp n3 10k
R4 n3 lpn 10k
XOA3 0 n3 lpn opamp
R1 lpn n1 10k
* @RNL {
*   $NET_LABEL @ATTACH &bp => %p_bp;
*   $NET_LABEL @ATTACH &lp => %p_lp;
*   %routed_nets = # @SELECT-NET;
*   %routed_nets @ROUTE-NET;
*
*   /* what the circuit is: three op-amp stages in a ring, a lossy integrator (RQ || C1), an integrator (C2) and an
*      inverter (R4), inside the outer feedback loop R1; the band-pass and low-pass outputs are tagged */
*   %lossy_rc = {#RQ #C1} @AS $PARALLEL_VERTICAL;
*   %fb1 = {#XOA1}(forward) {%lossy_rc}(feedback) @AS $FEEDBACK_LOOP => @GROUP;
*   %rq_over_xoa1 = %lossy_rc #XOA1 @ORDER("top-to-bottom");
*   %fb2 = {#XOA2}(forward) {#C2}(feedback) @AS $FEEDBACK_LOOP => @GROUP;
*   %c2_over_xoa2 = #C2 #XOA2 @ORDER("top-to-bottom");
*   %fb3 = {#XOA3}(forward) {#R4}(feedback) @AS $FEEDBACK_LOOP => @GROUP;
*   %r4_over_xoa2 = #R4 #XOA2 @ORDER("top-to-bottom");
*   %stages = {%fb1 #R2 %fb2 #R3 %fb3} @AS $CASCADE => @GROUP;
*   %loop_outer = {%stages}(forward) {#R1}(feedback) @AS $FEEDBACK_LOOP => @GROUP;
*   %r1_over_stages = #R1 %stages @ORDER("top-to-bottom");
*   %matched_resistors = {#RIN #R1 #R2 #R3 #R4 #RQ} @AS $MATCHED_RESISTOR;
*   %matched_caps = {#C1 #C2} @AS $MATCHED_CAPACITOR;
*
*   /* C2 and R4 lie down (turned a quarter turn either way) */
*   %lying_parts = {#C2 #R4} @ORIENT(["R90","R270"]);
*
*   /* from left to right the source and the three stages: the lossy integrator, the integrator, the inverter */
*   %stage_order = {#VIN #RIN %fb1 #R2 %fb2 #R3 %fb3} @ORDER("left-to-right");
*
*   /* each stage's input resistor lies in the row of its op-amp's - input; the op-amps are flipped so that the - input is
*      on top */
*   %opamp_row = {#XOA1 #XOA2 #XOA3} @ALIGN("y");
*   %xoa1_input_order = #XOA1:inn #XOA1:inp @ORDER("top-to-bottom");
*   %xoa2_input_order = #XOA2:inn #XOA2:inp @ORDER("top-to-bottom");
*   %xoa3_input_order = #XOA3:inn #XOA3:inp @ORDER("top-to-bottom");
*   %xoa1_out_orient = #XOA1:out @ORIENT("right");
*   %xoa2_out_orient = #XOA2:out @ORIENT("right");
*   %xoa3_out_orient = #XOA3:out @ORIENT("right");
*
*   /* the band-pass and low-pass outputs tagged under their op-amps */
*   %tag_distance = {%p_bp %p_lp} %loop_outer @DISTANCE("bottom-to-top", min=8);
*
*   /* only the source is oriented */
*   %source_orient = #VIN @ORIENT("R0");
* }
.end
