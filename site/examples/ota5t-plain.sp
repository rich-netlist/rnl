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
*   # @SELECT-NET(port=true) => @REMOVE-FROM $PORT_FILL_SET;
*   # @SELECT-NET => @ROUTE-NET
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:02d1093823b81959.dd4e344da2d518c0") {
*   %net_label_stub_M1_g = $NET_LABEL_STUB @ATTACH #M1::g;
*   %net_label_stub_M2_g = $NET_LABEL_STUB @ATTACH #M2::g;
*   %net_label_stub_M5_g = $NET_LABEL_STUB @ATTACH #M5::g;
*   %rt1 = (9, -1);
*   %rt2 = (9, 2);
*   %rt3 = (5, 7);
*   %rt4 = (7, 5);
*   %rt5 = (12, -5);
*   %rt6 = (18, -1);
*   %rt7 = (7, 11);
*   #M1 @PLACE(18, 3) => @ORIENT("R180");
*   #M2 @PLACE(2, 7) => @ORIENT("MX");
*   #M3 @PLACE(12, -1) => @ORIENT("MY");
*   #M4 @PLACE(5, -1) => @ORIENT("MXR90");
*   #M5 @PLACE(7, 7) => @ORIENT("MY");
*   %net_label_stub_M1_g @PLACE(5, 7);
*   %net_label_stub_M1_g::l @PLACE(21, 3) => @ORIENT("R180");
*   %net_label_stub_M2_g @PLACE(5, 7);
*   %net_label_stub_M2_g::l @PLACE(-1, 7) => @ORIENT("R0");
*   %net_label_stub_M5_g @PLACE(5, 7);
*   %net_label_stub_M5_g::l @PLACE(10, 7) => @ORIENT("MY");
*   &out @WIRE(mode="straight") {#M2::d (2, -1) #M4::d};
*   &vdd @WIRE(mode="straight") {#M4::b (5, 2) %rt2};
*   &vdd @WIRE(mode="straight") {#M4::s %rt1};
*   &vdd @WIRE(mode="straight") {%rt1 %rt2};
*   &vdd @WIRE(mode="straight") {%rt1 #M3::b};
*   &vdd @WIRE(mode="straight") {%rt2 (12, 2) #M3::s};
*   &vss @WIRE(mode="straight") {#M5::s %rt4};
*   &vss @WIRE(mode="straight") {#M2::b %rt3};
*   &vss @WIRE(mode="straight") {%rt3 (5, 5) %rt4};
*   &vss @WIRE(mode="straight") {%rt3 #M5::b};
*   &vss @WIRE(mode="straight") {%rt4 (7, 3) #M1::b};
*   &x @WIRE(mode="straight") {#M4::g (5, -5) %rt5};
*   &x @WIRE(mode="straight") {%rt5 #M3::d};
*   &x @WIRE(mode="straight") {%rt5 (18, -5) %rt6};
*   &x @WIRE(mode="straight") {#M3::g %rt6};
*   &x @WIRE(mode="straight") {%rt6 #M1::d};
*   &tail @WIRE(mode="straight") {#M2::s (2, 11) %rt7};
*   &tail @WIRE(mode="straight") {#M5::d %rt7};
*   &tail @WIRE(mode="straight") {%rt7 (18, 11) #M1::s};
*   $JUNCTION @ATTACH %rt1 => @PLACE(9, -1);
*   $JUNCTION @ATTACH %rt2 => @PLACE(9, 2);
*   $JUNCTION @ATTACH %rt3 => @PLACE(5, 7);
*   $JUNCTION @ATTACH %rt4 => @PLACE(7, 5);
*   $JUNCTION @ATTACH %rt5 => @PLACE(12, -5);
*   $JUNCTION @ATTACH %rt6 => @PLACE(18, -1);
*   $JUNCTION @ATTACH %rt7 => @PLACE(7, 11)
* }
.ends ota5t
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:fced5af0752234e8.53706929d5b144cf") {}
.end
