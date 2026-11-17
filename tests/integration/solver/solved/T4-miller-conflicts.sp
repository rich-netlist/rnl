* T4 Two-stage Miller OTA, with facts that conflict
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4")}
.model nch nmos level=1
.model pch pmos level=1
M1 x inp tail 0 nch W=2u L=0.5u
M2 y inn tail 0 nch W=2u L=0.5u
M3 x x vdd vdd pch W=4u L=0.5u
M4 y x vdd vdd pch W=4u L=0.5u
M5 tail vb 0 0 nch W=4u L=1u
M6 out y vdd vdd pch W=16u L=0.5u
M7 out vb 0 0 nch W=8u L=1u
CC y out 1p
VDD vdd 0 1.8
VB vb 0 0.6
VIP inp 0 DC 0.9 AC 0.5
VIN inn 0 DC 0.9 AC 0.5 180
CL out 0 2p
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   %routed_nets = # @SELECT-NET;
*   %routed_nets @ROUTE-NET;
*
*   /* the static currents (15.2), for the solver to draw them downwards */
*   {#M3:s #M4:s #M6:s #M1:d #M2:d #M5:d #M7:d} @CURRENT("in");
*   {#M3:d #M4:d #M6:d #M1:s #M2:s #M5:s #M7:s} @CURRENT("out");
*
*   /* what the circuit is: the input pair and its load mirrored about the tail M5, the load a
*      mirror, two stages from the sources to the load */
*   %dp = #M1(left) #M2(right) #M5(axis) @AS $MIRRORED_LAYOUT;
*   %load = #M3(left) #M4(right) #M5(axis) @AS $MIRRORED_LAYOUT;
*   %cm_load = {#M3}(ref) {#M4}(out) @AS $CURRENT_MIRROR_P;
*   %stage1 = {#M1 #M2 #M3 #M4 #M5} @GROUP;
*   %stage2 = {#M6 #M7 #CC} @GROUP;
*   %src = {#VIP #VIN} @GROUP;
*   %flow = {%stage1 %stage2}(path) {&inp &inn}(source) {&out}(load) @AS $SIGNAL_FLOW;
*   %order = {%src %stage1 %stage2 #CL} @ORDER("left-to-right");
*   {#VDD #VB #VIP #VIN #CL} @ORIENT("R0");
*
*   /* the facts of the stages, some of which conflict with the second record */
*   {#M1 #M2} @ALIGN("y");
*   %stage1 %stage2 @DISTANCE("left-to-right", min=3);
*   {#M6 #M7} @ALIGN("x");
*   #M7 #M6 @DISTANCE("bottom-to-top", min=4);
*   {#M5 #M7} @ALIGN("y");
*   {#VIP #VIN} @ALIGN("y", priority="low")
* }
* @RNL {
*   #M1 @PLACE(0, 0, priority="high");
*   #M2 @PLACE(y=3);
*   #M6 #M7 @DISTANCE("left-to-right", min=2);
*   #VIP #VIN @DISTANCE("bottom-to-top", exact=6)
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:828eeb8dd99e465c.8de46006f79b3e49") {
*   %ground_stub_M5_s = $GROUND_STUB @ATTACH(gap=3) #M5::s;
*   %ground_stub_M7_s = $GROUND_STUB @ATTACH(gap=3) #M7::s;
*   %ground_stub_VDD_n = $GROUND_STUB @ATTACH(gap=3) #VDD::n;
*   %ground_stub_VB_n = $GROUND_STUB @ATTACH(gap=3) #VB::n;
*   %ground_stub_VIP_n = $GROUND_STUB @ATTACH(gap=3) #VIP::n;
*   %ground_stub_VIN_n = $GROUND_STUB @ATTACH(gap=3) #VIN::n;
*   %ground_stub_CL_n = $GROUND_STUB @ATTACH(gap=3) #CL::n;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=3) #M3::s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=3) #M4::s;
*   %supply_stub_M6_s = $SUPPLY_STUB @ATTACH(gap=3) #M6::s;
*   %supply_stub_VDD_p = $SUPPLY_STUB @ATTACH(gap=3) #VDD::p;
*   %rt1 = (0, 7);
*   %rt2 = (7, 13);
*   %rt3 = (7, -3);
*   %rt4 = (14, 9);
*   %rt5 = (24, 5);
*   %rt6 = (17, -13);
*   %rt7 = (29, -3);
*   %rt8 = (32, -3);
*   #M1 @PLACE(0, 0) => @ORIENT("MY");
*   #M2 @PLACE(14, 3) => @ORIENT("R0");
*   #M3 @PLACE(0, 13) => @ORIENT("MY");
*   #M4 @PLACE(14, 13) => @ORIENT("R0");
*   #M5 @PLACE(7, -13) => @ORIENT("MY");
*   #M6 @PLACE(29, 5) => @ORIENT("R0");
*   #M7 @PLACE(32, -13) => @ORIENT("R0");
*   #CC @PLACE(20, 5) => @ORIENT("R270");
*   #VDD @PLACE(-13, -7) => @ORIENT("R0");
*   #VB @PLACE(17, -29) => @ORIENT("R0");
*   #VIP @PLACE(-13, -29) => @ORIENT("R0");
*   #VIN @PLACE(-3, -23) => @ORIENT("R0");
*   #CL @PLACE(35, -29) => @ORIENT("R0");
*   %dp @PLACE(7, -8.79);
*   %load @PLACE(7, 13);
*   %cm_load @PLACE(-4.05, 13);
*   %flow @PLACE(0, 0);
*   %ground_stub_M5_s @PLACE(0, 0);
*   %ground_stub_M5_s::g @PLACE(7, -17) => @ORIENT("R0");
*   %ground_stub_M7_s @PLACE(0, 0);
*   %ground_stub_M7_s::g @PLACE(32, -17) => @ORIENT("R0");
*   %ground_stub_VDD_n @PLACE(0, 0);
*   %ground_stub_VDD_n::g @PLACE(-13, -11) => @ORIENT("R0");
*   %ground_stub_VB_n @PLACE(0, 0);
*   %ground_stub_VB_n::g @PLACE(17, -33) => @ORIENT("R0");
*   %ground_stub_VIP_n @PLACE(0, 0);
*   %ground_stub_VIP_n::g @PLACE(-13, -33) => @ORIENT("R0");
*   %ground_stub_VIN_n @PLACE(0, 0);
*   %ground_stub_VIN_n::g @PLACE(-3, -27) => @ORIENT("R0");
*   %ground_stub_CL_n @PLACE(0, 0);
*   %ground_stub_CL_n::g @PLACE(35, -33) => @ORIENT("R0");
*   %supply_stub_M3_s @PLACE(0, 0);
*   %supply_stub_M3_s::s @PLACE(0, 17) => @ORIENT("R0");
*   %supply_stub_M4_s @PLACE(0, 0);
*   %supply_stub_M4_s::s @PLACE(14, 17) => @ORIENT("R0");
*   %supply_stub_M6_s @PLACE(0, 0);
*   %supply_stub_M6_s::s @PLACE(29, 9) => @ORIENT("R0");
*   %supply_stub_VDD_p @PLACE(0, 0);
*   %supply_stub_VDD_p::s @PLACE(-13, -3) => @ORIENT("R0");
*   &x @WIRE(mode="straight") {#M1::d %rt1};
*   &x @WIRE(mode="straight") {%rt1 #M3::d};
*   &x @WIRE(mode="straight") {%rt1 (7, 7) %rt2};
*   &x @WIRE(mode="straight") {#M3::g %rt2};
*   &x @WIRE(mode="straight") {%rt2 #M4::g};
*   &inp @WIRE(mode="straight") {#VIP::p (-13, -15) (4, -15) (4, 0) #M1::g};
*   &tail @WIRE(mode="straight") {#M1::s (0, -3) %rt3};
*   &tail @WIRE(mode="straight") {#M5::d %rt3};
*   &tail @WIRE(mode="straight") {%rt3 (14, -3) #M2::s};
*   &y @WIRE(mode="straight") {#M2::d %rt4};
*   &y @WIRE(mode="straight") {%rt4 #M4::d};
*   &y @WIRE(mode="straight") {%rt4 (24, 9) %rt5};
*   &y @WIRE(mode="straight") {#CC::p %rt5};
*   &y @WIRE(mode="straight") {%rt5 #M6::g};
*   &inn @WIRE(mode="straight") {#VIN::p (-3, 3) #M2::g};
*   &vb @WIRE(mode="straight") {#M5::g %rt6};
*   &vb @WIRE(mode="straight") {#VB::p %rt6};
*   &vb @WIRE(mode="straight") {%rt6 #M7::g};
*   &out @WIRE(mode="straight") {#CC::n (17, 5) (17, -3) %rt7};
*   &out @WIRE(mode="straight") {%rt7 #M6::d};
*   &out @WIRE(mode="straight") {%rt7 %rt8};
*   &out @WIRE(mode="straight") {#M7::d %rt8};
*   &out @WIRE(mode="straight") {%rt8 (35, -3) #CL::p};
*   $JUNCTION @ATTACH %rt1 => @PLACE(0, 7);
*   $JUNCTION @ATTACH %rt2 => @PLACE(7, 13);
*   $JUNCTION @ATTACH %rt3 => @PLACE(7, -3);
*   $JUNCTION @ATTACH %rt4 => @PLACE(14, 9);
*   $JUNCTION @ATTACH %rt5 => @PLACE(24, 5);
*   $JUNCTION @ATTACH %rt6 => @PLACE(17, -13);
*   $JUNCTION @ATTACH %rt7 => @PLACE(29, -3);
*   $JUNCTION @ATTACH %rt8 => @PLACE(32, -3)
* }
.end
