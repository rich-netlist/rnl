* Five-transistor OTA
.model nch nmos level=1 vto=0.5 kp=100u lambda=0.05
.model pch pmos level=1 vto=-0.5 kp=100u lambda=0.05
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="eda-symbol", version="0.2"); @USE-PACKAGE(name="eda-style", version="0.2")}
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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:ec5bc350072282bf.37154b8889fd1b8b", numbering="980bcf92e6cf0958") {
*   %ground_stub_M5_s = $GROUND_STUB @ATTACH(gap=3) #M5::s;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=3) #M3::s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=3) #M4::s;
*   %net_label_stub_M3_b = $NET_LABEL_STUB @ATTACH #M3::b;
*   %net_label_stub_M4_b = $NET_LABEL_STUB @ATTACH #M4::b;
*   %net_label_stub_M1_b = $NET_LABEL_STUB @ATTACH #M1::b;
*   %net_label_stub_M2_b = $NET_LABEL_STUB @ATTACH #M2::b;
*   %net_label_stub_M5_b = $NET_LABEL_STUB @ATTACH #M5::b;
*   %io_port_vdd = $IO_PORT @ATTACH &vdd;
*   %io_port_vss = $IO_PORT @ATTACH &vss;
*   %pin_square_M1_d = $PIN_SQUARE @ATTACH #M1::d;
*   %pin_square_M1_g = $PIN_SQUARE @ATTACH #M1::g;
*   %pin_square_M1_s = $PIN_SQUARE @ATTACH #M1::s;
*   %pin_square_M1_b = $PIN_SQUARE @ATTACH #M1::b;
*   %pin_square_M2_d = $PIN_SQUARE @ATTACH #M2::d;
*   %pin_square_M2_g = $PIN_SQUARE @ATTACH #M2::g;
*   %pin_square_M2_s = $PIN_SQUARE @ATTACH #M2::s;
*   %pin_square_M2_b = $PIN_SQUARE @ATTACH #M2::b;
*   %pin_square_M3_d = $PIN_SQUARE @ATTACH #M3::d;
*   %pin_square_M3_g = $PIN_SQUARE @ATTACH #M3::g;
*   %pin_square_M3_s = $PIN_SQUARE @ATTACH #M3::s;
*   %pin_square_M3_b = $PIN_SQUARE @ATTACH #M3::b;
*   %pin_square_M4_d = $PIN_SQUARE @ATTACH #M4::d;
*   %pin_square_M4_g = $PIN_SQUARE @ATTACH #M4::g;
*   %pin_square_M4_s = $PIN_SQUARE @ATTACH #M4::s;
*   %pin_square_M4_b = $PIN_SQUARE @ATTACH #M4::b;
*   %pin_square_M5_d = $PIN_SQUARE @ATTACH #M5::d;
*   %pin_square_M5_g = $PIN_SQUARE @ATTACH #M5::g;
*   %pin_square_M5_s = $PIN_SQUARE @ATTACH #M5::s;
*   %pin_square_M5_b = $PIN_SQUARE @ATTACH #M5::b;
*   %rt1 = (21, 15);
*   %rt2 = (27, 26);
*   %rt3 = (15, 26);
*   %rt4 = (21, 31);
*   #M1 @PLACE(15, 20) => @ORIENT("R0");
*   #M2 @PLACE(27, 20) => @ORIENT("MY");
*   #M3 @PLACE(15, 31) => @ORIENT("MY");
*   #M4 @PLACE(27, 31) => @ORIENT("R0");
*   #M5 @PLACE(21, 11) => @ORIENT("R0");
*   %dp1 @PLACE(21, 20);
*   %#1 @PLACE(15, 20);
*   %#2 @PLACE(21, 20);
*   %cm1 @PLACE(9.79, 31);
*   %amp @PLACE(15, 20);
*   %left @PLACE(15, 0);
*   %#3 @PLACE(15, 20);
*   %right @PLACE(27, 0);
*   %#4 @PLACE(15, 20);
*   %#5 @PLACE(15, 20);
*   %#5::p @PLACE(11, 20) => @ORIENT("R0");
*   %#6 @PLACE(15, 20);
*   %#6::p @PLACE(31, 20) => @ORIENT("R180");
*   %#7 @PLACE(15, 20);
*   %#7::p @PLACE(17, 11) => @ORIENT("R0");
*   %po @PLACE(31, 26) => @ORIENT("MY");
*   %ground_stub_M5_s @PLACE(15, 20);
*   %ground_stub_M5_s::g @PLACE(21, 6) => @ORIENT("R0");
*   %supply_stub_M3_s @PLACE(15, 20);
*   %supply_stub_M3_s::s @PLACE(15, 36) => @ORIENT("R0");
*   %supply_stub_M4_s @PLACE(15, 20);
*   %supply_stub_M4_s::s @PLACE(27, 36) => @ORIENT("R0");
*   %net_label_stub_M3_b @PLACE(15, 20);
*   %net_label_stub_M3_b::l @PLACE(15, 31) => @ORIENT("R0");
*   %net_label_stub_M4_b @PLACE(15, 20);
*   %net_label_stub_M4_b::l @PLACE(27, 31) => @ORIENT("R180");
*   %net_label_stub_M1_b @PLACE(15, 20);
*   %net_label_stub_M1_b::l @PLACE(15, 20) => @ORIENT("R180");
*   %net_label_stub_M2_b @PLACE(15, 20);
*   %net_label_stub_M2_b::l @PLACE(27, 20) => @ORIENT("R0");
*   %net_label_stub_M5_b @PLACE(15, 20);
*   %net_label_stub_M5_b::l @PLACE(21, 11) => @ORIENT("R180");
*   %io_port_vdd @PLACE(9, 6) => @ORIENT("R0");
*   %io_port_vss @PLACE(9, 2) => @ORIENT("R0");
*   %pin_square_M1_d @PLACE(15, 20);
*   %pin_square_M1_g @PLACE(15, 20);
*   %pin_square_M1_s @PLACE(15, 20);
*   %pin_square_M1_b @PLACE(15, 20);
*   %pin_square_M2_d @PLACE(15, 20);
*   %pin_square_M2_g @PLACE(15, 20);
*   %pin_square_M2_s @PLACE(15, 20);
*   %pin_square_M2_b @PLACE(15, 20);
*   %pin_square_M3_d @PLACE(15, 20);
*   %pin_square_M3_g @PLACE(15, 20);
*   %pin_square_M3_s @PLACE(15, 20);
*   %pin_square_M3_b @PLACE(15, 20);
*   %pin_square_M4_d @PLACE(15, 20);
*   %pin_square_M4_g @PLACE(15, 20);
*   %pin_square_M4_s @PLACE(15, 20);
*   %pin_square_M4_b @PLACE(15, 20);
*   %pin_square_M5_d @PLACE(15, 20);
*   %pin_square_M5_g @PLACE(15, 20);
*   %pin_square_M5_s @PLACE(15, 20);
*   %pin_square_M5_b @PLACE(15, 20);
*   &tail @WIRE(mode="straight") {%rt1 #M5::d};
*   &tail @WIRE(mode="straight") {%rt1 (15, 15) #M1::s};
*   &tail @WIRE(mode="straight") {%rt1 (27, 15) #M2::s};
*   &out @WIRE(mode="straight") {#M2::d %rt2};
*   &out @WIRE(mode="straight") {%rt2 #M4::d};
*   &out @WIRE(mode="straight") {%rt2 %po::pin};
*   &x @WIRE(mode="straight") {#M1::d %rt3};
*   &x @WIRE(mode="straight") {%rt3 #M3::d};
*   &x @WIRE(mode="straight") {%rt3 (21, 26) %rt4};
*   &x @WIRE(mode="straight") {#M3::g %rt4};
*   &x @WIRE(mode="straight") {%rt4 #M4::g};
*   $JUNCTION @ATTACH %rt1 => @PLACE(21, 15);
*   $JUNCTION @ATTACH %rt2 => @PLACE(27, 26);
*   $JUNCTION @ATTACH %rt3 => @PLACE(15, 26);
*   $JUNCTION @ATTACH %rt4 => @PLACE(21, 31)
* }
.ends ota5t
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:29a1f719f9557893.53706929d5b144cf") {}
.end
