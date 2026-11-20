* Five-transistor OTA
.model nch nmos level=1 vto=0.5 kp=100u lambda=0.05
.model pch pmos level=1 vto=-0.5 kp=100u lambda=0.05
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
*   {%amp %po::pin} @ORDER("left-to-right")
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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:5c1c4e5cfe594a16.53706929d5b144cf") {}
.end
