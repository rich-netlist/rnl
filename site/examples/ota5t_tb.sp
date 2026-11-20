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
*   # @SELECT-NET => @ROUTE-NET;
*   $IO_PORT_STUB @ATTACH {#M1:g #M2:g #M5:g};
*   %po = $IO_PORT @ATTACH &out;
*   {%amp %po::pin} @ORDER("left-to-right");
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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:13bd9770c3063bbd.c9d4c659d9406c2d", numbering="09e867fdaf53775a") {
*   %ground_stub_M5_s = $GROUND_STUB @ATTACH(gap=3) #M5::s;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=3) #M3::s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=3) #M4::s;
*   %io_port_vdd = $IO_PORT @ATTACH &vdd;
*   %io_port_vss = $IO_PORT @ATTACH &vss;
*   %rt1 = (15, 13);
*   %rt2 = (20, 21);
*   %rt3 = (10, 21);
*   %rt4 = (15, 26);
*   #M1 @PLACE(10, 16) => @ORIENT("R0");
*   #M2 @PLACE(20, 16) => @ORIENT("MY");
*   #M3 @PLACE(10, 26) => @ORIENT("MY");
*   #M4 @PLACE(20, 26) => @ORIENT("R0");
*   #M5 @PLACE(15, 9) => @ORIENT("MY");
*   %dp1 @PLACE(15, 16);
*   %#1 @PLACE(10, 16);
*   %#2 @PLACE(15, 16);
*   %cm1 @PLACE(5.95, 26);
*   %amp @PLACE(10, 16);
*   %left @PLACE(10, -0.95);
*   %#3 @PLACE(10, 16);
*   %right @PLACE(20, -0.95);
*   %#4 @PLACE(10, 16);
*   %#5 @PLACE(10, 16);
*   %#5::p @PLACE(7, 16) => @ORIENT("R0");
*   %#6 @PLACE(10, 16);
*   %#6::p @PLACE(23, 16) => @ORIENT("R180");
*   %#7 @PLACE(10, 16);
*   %#7::p @PLACE(18, 9) => @ORIENT("MY");
*   %po @PLACE(23, 21) => @ORIENT("MY");
*   %ground_stub_M5_s @PLACE(10, 16);
*   %ground_stub_M5_s::g @PLACE(15, 5) => @ORIENT("R0");
*   %supply_stub_M3_s @PLACE(10, 16);
*   %supply_stub_M3_s::s @PLACE(10, 30) => @ORIENT("R0");
*   %supply_stub_M4_s @PLACE(10, 16);
*   %supply_stub_M4_s::s @PLACE(20, 30) => @ORIENT("R0");
*   %io_port_vdd @PLACE(4, 5) => @ORIENT("R0");
*   %io_port_vss @PLACE(4, 1) => @ORIENT("R0");
*   &tail @WIRE(mode="straight") {#M1::s (10, 13) %rt1};
*   &tail @WIRE(mode="straight") {#M5::d %rt1};
*   &tail @WIRE(mode="straight") {%rt1 (20, 13) #M2::s};
*   &out @WIRE(mode="straight") {#M2::d %rt2};
*   &out @WIRE(mode="straight") {%rt2 #M4::d};
*   &out @WIRE(mode="straight") {%rt2 %po::pin};
*   &x @WIRE(mode="straight") {#M1::d %rt3};
*   &x @WIRE(mode="straight") {%rt3 #M3::d};
*   &x @WIRE(mode="straight") {%rt3 (15, 21) %rt4};
*   &x @WIRE(mode="straight") {#M3::g %rt4};
*   &x @WIRE(mode="straight") {%rt4 #M4::g};
*   $JUNCTION @ATTACH %rt1 => @PLACE(15, 13);
*   $JUNCTION @ATTACH %rt2 => @PLACE(20, 21);
*   $JUNCTION @ATTACH %rt3 => @PLACE(10, 21);
*   $JUNCTION @ATTACH %rt4 => @PLACE(15, 26)
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
*   {#VIP #VIN #XU1 #CL} are in @ORDER("left-to-right");
*   nets {&inp &inn &out &vdd &vb} are @ROUTE-NET;
*   &inp has @TEXT("inp"); &inn has @TEXT("inn"); &vdd has @TEXT("vdd"); &out has @TEXT("out");
*   at (-0.5, 24) there is a @TEXT("5T OTA testbench") => it @SET-STYLE to $note
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:4d9e482c9f11e5c0.8673ead7048bc0bc", numbering="a4a24d54cf4ff55f") {
*   %ground_stub_XU1_vss = $GROUND_STUB @ATTACH(gap=3) #XU1:vss;
*   %ground_stub_VDD_n = $GROUND_STUB @ATTACH(gap=3) #VDD::n;
*   %ground_stub_VB_n = $GROUND_STUB @ATTACH(gap=3) #VB::n;
*   %ground_stub_VIP_n = $GROUND_STUB @ATTACH(gap=3) #VIP::n;
*   %ground_stub_VIN_n = $GROUND_STUB @ATTACH(gap=3) #VIN::n;
*   %ground_stub_CL_n = $GROUND_STUB @ATTACH(gap=3) #CL::n;
*   #XU1 @PLACE(20, 14) => @ORIENT("R0");
*   #VDD @PLACE(22, 26) => @ORIENT("R90");
*   #VB @PLACE(17, 3) => @ORIENT("R0");
*   #VIP @PLACE(-1, 12) => @ORIENT("MY");
*   #VIN @PLACE(10, 12) => @ORIENT("R270");
*   #CL @PLACE(32, 14) => @ORIENT("R90");
*   %ground_stub_XU1_vss @PLACE(0, 0);
*   %ground_stub_XU1_vss::g @PLACE(21, 10) => @ORIENT("R0");
*   %ground_stub_VDD_n @PLACE(0, 0);
*   %ground_stub_VDD_n::g @PLACE(26, 24) => @ORIENT("R0");
*   %ground_stub_VB_n @PLACE(0, 0);
*   %ground_stub_VB_n::g @PLACE(17, -1) => @ORIENT("R0");
*   %ground_stub_VIP_n @PLACE(0, 0);
*   %ground_stub_VIP_n::g @PLACE(-1, 8) => @ORIENT("R0");
*   %ground_stub_VIN_n @PLACE(0, 0);
*   %ground_stub_VIN_n::g @PLACE(6, 10) => @ORIENT("R0");
*   %ground_stub_CL_n @PLACE(0, 0);
*   %ground_stub_CL_n::g @PLACE(36, 12) => @ORIENT("R0");
*   %#5 @PLACE(-0.5, 24);
*   %#6 @PLACE(-0.5, 24);
*   &inp @WIRE(mode="straight") {#VIP::p (-1, 16) #XU1:inp};
*   &inn @WIRE(mode="straight") {#VIN::p #XU1:inn};
*   &out @WIRE(mode="straight") {#XU1:out #CL::p};
*   &vdd @WIRE(mode="straight") {#XU1:vdd (19, 26) #VDD::p};
*   &vb @WIRE(mode="straight") {#VB::p #XU1:vbias};
*   %#8 @WIRE(mode="straight") {#VDD::n (26, 26) %ground_stub_VDD_n::g::pin};
*   %#11 @WIRE(mode="straight") {#VIN::n (6, 12) %ground_stub_VIN_n::g::pin};
*   %#12 @WIRE(mode="straight") {#CL::n (36, 14) %ground_stub_CL_n::g::pin}
* }
.end
