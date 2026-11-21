* 5T OTA testbench
.model nch nmos level=1 vto=0.5
.model pch pmos level=1 vto=-0.5
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M3 x    x     vdd  vdd pch W=4u L=0.5u
M4 out  x     vdd  vdd pch W=4u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   %dp1 = #M1(left), #M2(right) and #M5(tail) treated @AS $DIFFERENTIAL_PAIR_N;
*   %cm1 = #M3(ref) and #M4(out) treated @AS $CURRENT_MIRROR_P;
*   %amp = %dp1(transconductor) and %cm1(load) treated @AS $TRANSCONDUCTOR_Z_AMPLIFIER;
*   %left = {#M3 #M1} treated @AS $BRANCH;
*   %right = {#M4 #M2} treated @AS $BRANCH;
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   #M5:d has @CURRENT("in");
*   # @SELECT-NET(port=true) => @REMOVE-FROM $PORT_FILL_SET;
*   %p_inp = $IO_PORT @ATTACH &inp;
*   %p_inn = $IO_PORT @ATTACH &inn;
*   %p_out = $IO_PORT @ATTACH &out;
*   %p_bias = $IO_PORT @ATTACH &vbias;
*   # @SELECT-NET => @ROUTE-NET;
*   %amp_group = {%amp %p_inp %p_inn %p_out %p_bias} @GROUP;
*   {%amp %p_out} @ORDER("left-to-right");
*   $OTA_SYM = @DEFINE-SYMBOL {
*     (-5, 2) @PIN(terminal="inp", direction="left");
*     (-5, -2) @PIN(terminal="inn", direction="left");
*     (5, 0) @PIN(terminal="out", direction="right");
*     (-1, 3) @PIN(terminal="vdd", direction="up");
*     (1, -2) @PIN(terminal="vss", direction="down");
*     (-3, -4) @PIN(terminal="vbias", direction="down");
*     @LINE {(-5, 5) (5, 0) (-5, -5) (-5, 5)};
*     ? @TEXT("${name}", key="name") => @PLACE(-2, 0)
*   };
*   this subcircuit # @SET-SYMBOL to $OTA_SYM
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:edea2dac05e41f41.7853daec65a52d57", numbering="d4bd4a77786955a8") {
*   %ground_stub_M5_s = $GROUND_STUB @ATTACH(gap=3) #M5::s;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=3) #M3::s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=3) #M4::s;
*   %rt1 = (10, 12);
*   %rt2 = (16, 21);
*   %rt3 = (4, 21);
*   %rt4 = (10, 26);
*   #M1 @PLACE(4, 16) => @ORIENT("R0");
*   #M2 @PLACE(16, 16) => @ORIENT("MY");
*   #M3 @PLACE(4, 26) => @ORIENT("MY");
*   #M4 @PLACE(16, 26) => @ORIENT("R0");
*   #M5 @PLACE(10, 9) => @ORIENT("R0");
*   %dp1 @PLACE(10, 16);
*   %#1 @PLACE(4, 16);
*   %#2 @PLACE(10, 16);
*   %cm1 @PLACE(-1, 26);
*   %amp @PLACE(4, 16);
*   %left @PLACE(4, -0.95);
*   %#3 @PLACE(4, 16);
*   %right @PLACE(16, -0.95);
*   %#4 @PLACE(4, 16);
*   %p_inp @PLACE(-1, 16) => @ORIENT("R0");
*   %p_inn @PLACE(21, 16) => @ORIENT("MY");
*   %p_out @PLACE(19, 21) => @ORIENT("MY");
*   %p_bias @PLACE(-1, 9) => @ORIENT("R0");
*   %ground_stub_M5_s @PLACE(4, 16);
*   %ground_stub_M5_s::g @PLACE(10, 5) => @ORIENT("R0");
*   %supply_stub_M3_s @PLACE(4, 16);
*   %supply_stub_M3_s::s @PLACE(4, 30) => @ORIENT("R0");
*   %supply_stub_M4_s @PLACE(4, 16);
*   %supply_stub_M4_s::s @PLACE(16, 30) => @ORIENT("R0");
*   &tail @WIRE(mode="straight") {%rt1 #M5::d};
*   &tail @WIRE(mode="straight") {%rt1 (4, 12) #M1::s};
*   &tail @WIRE(mode="straight") {%rt1 (16, 12) #M2::s};
*   &out @WIRE(mode="straight") {#M2::d %rt2};
*   &out @WIRE(mode="straight") {%rt2 #M4::d};
*   &out @WIRE(mode="straight") {%rt2 %p_out::pin};
*   &x @WIRE(mode="straight") {#M1::d %rt3};
*   &x @WIRE(mode="straight") {%rt3 #M3::d};
*   &x @WIRE(mode="straight") {%rt3 (10, 21) %rt4};
*   &x @WIRE(mode="straight") {#M3::g %rt4};
*   &x @WIRE(mode="straight") {%rt4 #M4::g};
*   &inp @WIRE(mode="straight") {%p_inp::pin #M1::g};
*   &inn @WIRE(mode="straight") {#M2::g %p_inn::pin};
*   &vbias @WIRE(mode="straight") {%p_bias::pin #M5::g};
*   $JUNCTION @ATTACH %rt1 => @PLACE(10, 12);
*   $JUNCTION @ATTACH %rt2 => @PLACE(16, 21);
*   $JUNCTION @ATTACH %rt3 => @PLACE(4, 21);
*   $JUNCTION @ATTACH %rt4 => @PLACE(10, 26)
* }
.ends ota5t
XU1 inp inn out vdd 0 vb ota5t
VDD vdd 0 1.8
VB  vb  0 0.6
VIP inp 0 DC 0.9 AC 0.5
VIN inn 0 DC 0.9 AC 0.5 180
CL  out 0 1p
* @RNL {
*   #XU1 is at @PLACE(20, 14);
*   nets {&inp &inn &out &vdd &vb} are @ROUTE-NET;
*   &inp has @TEXT("inp"); &inn has @TEXT("inn"); &vdd has @TEXT("vdd"); &out has @TEXT("out");
*   %flow = {#XU1}(path) {#VIP #VIN}(source) {#CL}(load) @AS $SIGNAL_FLOW;
*   {#VIP #VIN #XU1 #CL} are in @ORDER("left-to-right");
*   {#VB #XU1} @ORDER("bottom-to-top");
*   #VDD {#VIP #VIN #XU1 #CL} @DISTANCE("left-to-right", min=4);
*   /* each source and the load stand upright: the + pin above the - pin */
*   {#VDD:n #VDD:p} @ORDER("bottom-to-top"); {#VB:n #VB:p} @ORDER("bottom-to-top");
*   {#VIP:n #VIP:p} @ORDER("bottom-to-top"); {#VIN:n #VIN:p} @ORDER("bottom-to-top"); {#CL:n #CL:p} @ORDER("bottom-to-top");
*   at (-0.5, 24) there is a @TEXT("5T OTA testbench") => it @SET-STYLE to $note
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:74e01853fd16c474.5b382edd3e662eff", numbering="3ff6b02e1bed8dda") {
*   %ground_stub_XU1_vss = $GROUND_STUB @ATTACH(gap=3) #XU1:vss;
*   %ground_stub_VDD_n = $GROUND_STUB @ATTACH(gap=3) #VDD::n;
*   %ground_stub_VB_n = $GROUND_STUB @ATTACH(gap=3) #VB::n;
*   %ground_stub_VIP_n = $GROUND_STUB @ATTACH(gap=3) #VIP::n;
*   %ground_stub_VIN_n = $GROUND_STUB @ATTACH(gap=3) #VIN::n;
*   %ground_stub_CL_n = $GROUND_STUB @ATTACH(gap=3) #CL::n;
*   #XU1 @PLACE(20, 14) => @ORIENT("R0");
*   #VDD @PLACE(-19, 3) => @ORIENT("MY");
*   #VB @PLACE(17, 3) => @ORIENT("R0");
*   #VIP @PLACE(-6, 3) => @ORIENT("MY");
*   #VIN @PLACE(9, 3) => @ORIENT("MY");
*   #CL @PLACE(26, 3) => @ORIENT("R0");
*   %flow @PLACE(0, 0);
*   %ground_stub_XU1_vss @PLACE(0, 0);
*   %ground_stub_XU1_vss::g @PLACE(21, 10) => @ORIENT("R0");
*   %ground_stub_VDD_n @PLACE(0, 0);
*   %ground_stub_VDD_n::g @PLACE(-19, -1) => @ORIENT("R0");
*   %ground_stub_VB_n @PLACE(0, 0);
*   %ground_stub_VB_n::g @PLACE(17, -1) => @ORIENT("R0");
*   %ground_stub_VIP_n @PLACE(0, 0);
*   %ground_stub_VIP_n::g @PLACE(-6, -1) => @ORIENT("R0");
*   %ground_stub_VIN_n @PLACE(0, 0);
*   %ground_stub_VIN_n::g @PLACE(9, -1) => @ORIENT("R0");
*   %ground_stub_CL_n @PLACE(0, 0);
*   %ground_stub_CL_n::g @PLACE(26, -1) => @ORIENT("R0");
*   %#5 @PLACE(-0.5, 24);
*   %#6 @PLACE(-0.5, 24);
*   &inp @WIRE(mode="straight") {#VIP::p (-6, 16) #XU1:inp};
*   &inn @WIRE(mode="straight") {#VIN::p (9, 12) #XU1:inn};
*   &out @WIRE(mode="straight") {#XU1:out (26, 14) #CL::p};
*   &vdd @WIRE(mode="straight") {#VDD::p (-19, 17) #XU1:vdd};
*   &vb @WIRE(mode="straight") {#VB::p #XU1:vbias}
* }
.end
