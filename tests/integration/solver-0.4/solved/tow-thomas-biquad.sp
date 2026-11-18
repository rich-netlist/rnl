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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:87227728b961906f.46592a9e675e4bdd") {
*   %ground_stub_E1_n = $GROUND_STUB @ATTACH(gap=3) #E1::n;
*   #E1 @PLACE(11, 1) => @ORIENT("R0");
*   %p_inp @PLACE(1, 2) => @ORIENT("R0");
*   %p_inn @PLACE(5, 0) => @ORIENT("R0");
*   %p_out @PLACE(12, 6) => @ORIENT("MY");
*   %flow @PLACE(11, -3);
*   %ground_stub_E1_n @PLACE(11, -3);
*   %ground_stub_E1_n::g @PLACE(11, -3) => @ORIENT("R0");
*   &inp @WIRE(mode="straight") {%p_inp::pin #E1::cp};
*   &inn @WIRE(mode="straight") {%p_inn::pin #E1::cn};
*   &out @WIRE(mode="straight") {#E1::p (11, 6) %p_out::pin}
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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:3b05a2c0d1f2dda7.dfa2893df8788152", numbering="8ff66e12e3137d98") {
*   %ground_stub_VIN_n = $GROUND_STUB @ATTACH(gap=3) #VIN::n;
*   %ground_stub_XOA1_inp = $GROUND_STUB @ATTACH(gap=3) #XOA1::inp;
*   %ground_stub_XOA2_inp = $GROUND_STUB @ATTACH(gap=3) #XOA2::inp;
*   %ground_stub_XOA3_inp = $GROUND_STUB @ATTACH(gap=3) #XOA3::inp;
*   %rt1 = (18, 15);
*   %rt2 = (18, 22);
*   %rt3 = (35, 15);
*   %rt4 = (35, 8);
*   %rt5 = (35, 9);
*   %rt6 = (44, 9);
*   %rt7 = (55, 8);
*   %rt8 = (55, 15);
*   %rt9 = (64, 15);
*   %rt10 = (75, 15);
*   #VIN @PLACE(8, 19) => @ORIENT("R0");
*   #RIN @PLACE(14, 22) => @ORIENT("R90");
*   #RQ @PLACE(22, 22) => @ORIENT("R90");
*   #C1 @PLACE(22, 15) => @ORIENT("R90");
*   #XOA1 @PLACE(30, 8) => @ORIENT("MX");
*   #R2 @PLACE(39, 9) => @ORIENT("R90");
*   #C2 @PLACE(48, 15) => @ORIENT("R90");
*   #XOA2 @PLACE(50, 8) => @ORIENT("MX");
*   #R3 @PLACE(59, 15) => @ORIENT("R90");
*   #R4 @PLACE(71, 15) => @ORIENT("R90");
*   #XOA3 @PLACE(70, 8) => @ORIENT("MX");
*   #R1 @PLACE(22, 28) => @ORIENT("R270");
*   %p_bp @PLACE(35, 0) => @ORIENT("R90");
*   %p_lp @PLACE(55, 0) => @ORIENT("R90");
*   %lossy_rc @PLACE(22, 18.5);
*   %#1 @PLACE(40, 0);
*   %#2 @PLACE(40, 0);
*   %#3 @PLACE(40, 0);
*   %#4 @PLACE(40, 0);
*   %#5 @PLACE(40, 0);
*   %#6 @PLACE(40, 0);
*   %matched_resistors @PLACE(40, 0);
*   %#7 @PLACE(40, 0);
*   %matched_caps @PLACE(40, 0);
*   %#8 @PLACE(40, 0);
*   %ground_stub_VIN_n @PLACE(40, 0);
*   %ground_stub_VIN_n::g @PLACE(8, 15) => @ORIENT("R0");
*   %ground_stub_XOA1_inp @PLACE(40, 0);
*   %ground_stub_XOA1_inp::g @PLACE(24, 5) => @ORIENT("R0");
*   %ground_stub_XOA2_inp @PLACE(40, 0);
*   %ground_stub_XOA2_inp::g @PLACE(44, 5) => @ORIENT("R0");
*   %ground_stub_XOA3_inp @PLACE(40, 0);
*   %ground_stub_XOA3_inp::g @PLACE(64, 5) => @ORIENT("R0");
*   &vin @WIRE(mode="straight") {#VIN::p (8, 22) #RIN::p};
*   &n1 @WIRE(mode="straight") {#RIN::n %rt2};
*   &n1 @WIRE(mode="straight") {%rt1 (18, 9) #XOA1::inn};
*   &n1 @WIRE(mode="straight") {%rt1 %rt2};
*   &n1 @WIRE(mode="straight") {%rt1 #C1::p};
*   &n1 @WIRE(mode="straight") {%rt2 (18, 28) #R1::n};
*   &n1 @WIRE(mode="straight") {%rt2 #RQ::p};
*   &bp @WIRE(mode="straight") {#C1::n %rt3};
*   &bp @WIRE(mode="straight") {#RQ::n (35, 22) %rt3};
*   &bp @WIRE(mode="straight") {%rt3 %rt5};
*   &bp @WIRE(mode="straight") {#XOA1::out %rt4};
*   &bp @WIRE(mode="straight") {%p_bp::pin %rt4};
*   &bp @WIRE(mode="straight") {%rt4 %rt5};
*   &bp @WIRE(mode="straight") {%rt5 #R2::p};
*   &n2 @WIRE(mode="straight") {#R2::n %rt6};
*   &n2 @WIRE(mode="straight") {%rt6 (44, 15) #C2::p};
*   &n2 @WIRE(mode="straight") {%rt6 #XOA2::inn};
*   &lp @WIRE(mode="straight") {#C2::n %rt8};
*   &lp @WIRE(mode="straight") {#XOA2::out %rt7};
*   &lp @WIRE(mode="straight") {%p_lp::pin %rt7};
*   &lp @WIRE(mode="straight") {%rt7 %rt8};
*   &lp @WIRE(mode="straight") {%rt8 #R3::p};
*   &n3 @WIRE(mode="straight") {#R3::n %rt9};
*   &n3 @WIRE(mode="straight") {%rt9 (64, 9) #XOA3::inn};
*   &n3 @WIRE(mode="straight") {%rt9 #R4::p};
*   &lpn @WIRE(mode="straight") {#R1::p (75, 28) %rt10};
*   &lpn @WIRE(mode="straight") {#R4::n %rt10};
*   &lpn @WIRE(mode="straight") {#XOA3::out (75, 8) %rt10};
*   %#10 @WIRE(mode="straight") {#XOA1::inp (24, 7) %ground_stub_XOA1_inp::g::pin};
*   %#11 @WIRE(mode="straight") {#XOA2::inp (44, 7) %ground_stub_XOA2_inp::g::pin};
*   %#12 @WIRE(mode="straight") {#XOA3::inp (64, 7) %ground_stub_XOA3_inp::g::pin};
*   $JUNCTION @ATTACH %rt1 => @PLACE(18, 15);
*   $JUNCTION @ATTACH %rt2 => @PLACE(18, 22);
*   $JUNCTION @ATTACH %rt3 => @PLACE(35, 15);
*   $JUNCTION @ATTACH %rt4 => @PLACE(35, 8);
*   $JUNCTION @ATTACH %rt5 => @PLACE(35, 9);
*   $JUNCTION @ATTACH %rt6 => @PLACE(44, 9);
*   $JUNCTION @ATTACH %rt7 => @PLACE(55, 8);
*   $JUNCTION @ATTACH %rt8 => @PLACE(55, 15);
*   $JUNCTION @ATTACH %rt9 => @PLACE(64, 15);
*   $JUNCTION @ATTACH %rt10 => @PLACE(75, 15)
* }
.end
