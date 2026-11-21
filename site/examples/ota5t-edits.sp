* 5T OTA
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
*   %dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;
*   %cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR_P;
*   %amp = %dp1(transconductor) %cm1(load) @AS $TRANSCONDUCTOR_Z_AMPLIFIER;
*   %left = {#M3 #M1} @AS $BRANCH;
*   %right = {#M4 #M2} @AS $BRANCH;
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   #M5:d @CURRENT("in");
*   # @SELECT-NET(port=true) => @REMOVE-FROM $PORT_FILL_SET;
*   %p_inp = $IO_PORT @ATTACH &inp;
*   %p_inn = $IO_PORT @ATTACH &inn;
*   %p_out = $IO_PORT @ATTACH &out;
*   %p_bias = $IO_PORT @ATTACH &vbias;
*   # @SELECT-NET => @ROUTE-NET;
*   %amp_group = {%amp %p_inp %p_inn %p_out %p_bias} @GROUP;
*   {%amp %p_out} @ORDER("left-to-right")
*   /* {#M1 #M2 #M9} @ALIGN("y") */
* }
* @RNL(author="rnl-editor") {
*   #M2 @PLACE(x=18, priority="high")
*   /* #M4 @PLACE(2O, 26, priority="high") */
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:a0c875a9451d0e20.3cb60203afe238d5", numbering="9912cd934557ce94") {
*   %ground_stub_M5_s = $GROUND_STUB @ATTACH(gap=3) #M5::s;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=3) #M3::s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=3) #M4::s;
*   %rt1 = (12, 12);
*   %rt2 = (18, 21);
*   %rt3 = (6, 21);
*   %rt4 = (12, 26);
*   #M1 @PLACE(6, 16) => @ORIENT("R0");
*   #M2 @PLACE(18, 16) => @ORIENT("MY");
*   #M3 @PLACE(6, 26) => @ORIENT("MY");
*   #M4 @PLACE(18, 26) => @ORIENT("R0");
*   #M5 @PLACE(12, 9) => @ORIENT("R0");
*   %dp1 @PLACE(12, 16);
*   %#1 @PLACE(-4, 16);
*   %#2 @PLACE(12, 16);
*   %cm1 @PLACE(1, 26);
*   %amp @PLACE(-4, 16);
*   %left @PLACE(6, -0.95);
*   %#3 @PLACE(-4, 16);
*   %right @PLACE(18, -0.95);
*   %#4 @PLACE(-4, 16);
*   %p_inp @PLACE(1, 16) => @ORIENT("R0");
*   %p_inn @PLACE(23, 16) => @ORIENT("MY");
*   %p_out @PLACE(21, 21) => @ORIENT("MY");
*   %p_bias @PLACE(1, 9) => @ORIENT("R0");
*   %ground_stub_M5_s @PLACE(-4, 16);
*   %ground_stub_M5_s::g @PLACE(12, 5) => @ORIENT("R0");
*   %supply_stub_M3_s @PLACE(-4, 16);
*   %supply_stub_M3_s::s @PLACE(6, 30) => @ORIENT("R0");
*   %supply_stub_M4_s @PLACE(-4, 16);
*   %supply_stub_M4_s::s @PLACE(18, 30) => @ORIENT("R0");
*   &tail @WIRE(mode="straight") {%rt1 #M5::d};
*   &tail @WIRE(mode="straight") {%rt1 (6, 12) #M1::s};
*   &tail @WIRE(mode="straight") {%rt1 (18, 12) #M2::s};
*   &out @WIRE(mode="straight") {#M2::d %rt2};
*   &out @WIRE(mode="straight") {%rt2 #M4::d};
*   &out @WIRE(mode="straight") {%rt2 %p_out::pin};
*   &x @WIRE(mode="straight") {#M1::d %rt3};
*   &x @WIRE(mode="straight") {%rt3 #M3::d};
*   &x @WIRE(mode="straight") {%rt3 (12, 21) %rt4};
*   &x @WIRE(mode="straight") {#M3::g %rt4};
*   &x @WIRE(mode="straight") {%rt4 #M4::g};
*   &inp @WIRE(mode="straight") {%p_inp::pin #M1::g};
*   &inn @WIRE(mode="straight") {#M2::g %p_inn::pin};
*   &vbias @WIRE(mode="straight") {%p_bias::pin #M5::g};
*   $JUNCTION @ATTACH %rt1 => @PLACE(12, 12);
*   $JUNCTION @ATTACH %rt2 => @PLACE(18, 21);
*   $JUNCTION @ATTACH %rt3 => @PLACE(6, 21);
*   $JUNCTION @ATTACH %rt4 => @PLACE(12, 26)
* }
.ends ota5t
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:dd825507dae03304.53706929d5b144cf") {}
.end
