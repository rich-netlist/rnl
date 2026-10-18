* T6 Eight-bit R-2R ladder
* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="rnl-std", version="0.2")}
VB0 b0 0 DC 1
VB1 b1 0 DC 1
VB2 b2 0 DC 1
VB3 b3 0 DC 1
VB4 b4 0 DC 1
VB5 b5 0 DC 1
VB6 b6 0 DC 1
VB7 b7 0 DC 1
RT n0 0 2k
R2B0 b0 n0 2k
R2B1 b1 n1 2k
R2B2 b2 n2 2k
R2B3 b3 n3 2k
R2B4 b4 n4 2k
R2B5 b5 n5 2k
R2B6 b6 n6 2k
R2B7 b7 n7 2k
RS0 n0 n1 1k
RS1 n1 n2 1k
RS2 n2 n3 1k
RS3 n3 n4 1k
RS4 n4 n5 1k
RS5 n5 n6 1k
RS6 n6 n7 1k
* @RNL {
*   $IOPORT @ATTACH &n7 => %pout;
*   $WIRE_NET_SET @ROUTE
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:9e355997de62a0b1.22233424deb2d8ec") {
*   %gnd_stub_VB0_n = $GND_STUB @ATTACH(gap=2) #VB0:n;
*   %gnd_stub_VB1_n = $GND_STUB @ATTACH(gap=2) #VB1:n;
*   %gnd_stub_VB2_n = $GND_STUB @ATTACH(gap=2) #VB2:n;
*   %gnd_stub_VB3_n = $GND_STUB @ATTACH(gap=2) #VB3:n;
*   %gnd_stub_VB4_n = $GND_STUB @ATTACH(gap=2) #VB4:n;
*   %gnd_stub_VB5_n = $GND_STUB @ATTACH(gap=2) #VB5:n;
*   %gnd_stub_VB6_n = $GND_STUB @ATTACH(gap=2) #VB6:n;
*   %gnd_stub_VB7_n = $GND_STUB @ATTACH(gap=2) #VB7:n;
*   %gnd_stub_RT_n = $GND_STUB @ATTACH(gap=2) #RT:n;
*   %rt1 = (61, 0);
*   %rt2 = (125, 6);
*   %rt3 = (127, 0);
*   %rt4 = (133, 0);
*   %rt5 = (139, 0);
*   %rt6 = (145, 0);
*   %rt7 = (151, 0);
*   %rt8 = (114, -2);
*   #VB0 @PLACE(-8, 3) => @ORIENT("R0");
*   #VB1 @PLACE(0, 3) => @ORIENT("R0");
*   #VB2 @PLACE(8, 3) => @ORIENT("R0");
*   #VB3 @PLACE(16, 3) => @ORIENT("R0");
*   #VB4 @PLACE(24, 3) => @ORIENT("R0");
*   #VB5 @PLACE(32, 3) => @ORIENT("R0");
*   #VB6 @PLACE(40, 3) => @ORIENT("R0");
*   #VB7 @PLACE(48, 3) => @ORIENT("R0");
*   #RT @PLACE(55, 3) => @ORIENT("R0");
*   #R2B0 @PLACE(61, 3) => @ORIENT("R0");
*   #R2B1 @PLACE(68, 3) => @ORIENT("R0");
*   #R2B2 @PLACE(75, 3) => @ORIENT("R0");
*   #R2B3 @PLACE(82, 3) => @ORIENT("R0");
*   #R2B4 @PLACE(89, 3) => @ORIENT("R0");
*   #R2B5 @PLACE(96, 3) => @ORIENT("R0");
*   #R2B6 @PLACE(103, 3) => @ORIENT("R0");
*   #R2B7 @PLACE(114, 3) => @ORIENT("R0");
*   #RS0 @PLACE(121, 3) => @ORIENT("R0");
*   #RS1 @PLACE(127, 3) => @ORIENT("R0");
*   #RS2 @PLACE(133, 3) => @ORIENT("R0");
*   #RS3 @PLACE(139, 3) => @ORIENT("R0");
*   #RS4 @PLACE(145, 3) => @ORIENT("R0");
*   #RS5 @PLACE(151, 3) => @ORIENT("R0");
*   #RS6 @PLACE(157, 3) => @ORIENT("R0");
*   %pout @PLACE(114, -2) => @ORIENT("R0");
*   %gnd_stub_VB0_n::g @PLACE(-8, -2) => @ORIENT("R0");
*   %gnd_stub_VB1_n::g @PLACE(0, -2) => @ORIENT("R0");
*   %gnd_stub_VB2_n::g @PLACE(8, -2) => @ORIENT("R0");
*   %gnd_stub_VB3_n::g @PLACE(16, -2) => @ORIENT("R0");
*   %gnd_stub_VB4_n::g @PLACE(24, -2) => @ORIENT("R0");
*   %gnd_stub_VB5_n::g @PLACE(32, -2) => @ORIENT("R0");
*   %gnd_stub_VB6_n::g @PLACE(40, -2) => @ORIENT("R0");
*   %gnd_stub_VB7_n::g @PLACE(48, -2) => @ORIENT("R0");
*   %gnd_stub_RT_n::g @PLACE(55, -2) => @ORIENT("R0");
*   &b0 @WIRE(mode="straight") {#VB0:p (-3, 6) (-3, -1) (59, -1) (59, 6) #R2B0:p};
*   &b1 @WIRE(mode="straight") {#VB1:p (5, 6) (5, -8) (66, -8) (66, 6) #R2B1:p};
*   &b2 @WIRE(mode="straight") {#VB2:p (13, 6) (13, -7) (73, -7) (73, 6) #R2B2:p};
*   &b3 @WIRE(mode="straight") {#VB3:p (21, 6) (21, -5) (80, -5) (80, 6) #R2B3:p};
*   &b4 @WIRE(mode="straight") {#VB4:p (24, 10) (89, 10) #R2B4:p};
*   &b5 @WIRE(mode="straight") {#VB5:p (32, 9) (96, 9) #R2B5:p};
*   &b6 @WIRE(mode="straight") {#VB6:p (40, 8) (103, 8) #R2B6:p};
*   &b7 @WIRE(mode="straight") {#VB7:p (53, 6) (53, -6) (87, -6) (87, 7) (114, 7) #R2B7:p};
*   &n0 @WIRE(mode="straight") {#RT:p (58, 6) (58, 0) %rt1};
*   &n0 @WIRE(mode="straight") {#RS0:p (121, 9) (98, 9) (98, -2) (61, -2) %rt1};
*   &n0 @WIRE(mode="straight") {%rt1 #R2B0:n};
*   &n1 @WIRE(mode="straight") {#RS0:n (125, 0) %rt2};
*   &n1 @WIRE(mode="straight") {%rt2 #RS1:p};
*   &n1 @WIRE(mode="straight") {#R2B1:n (68, -1) (110, -1) (110, 8) (125, 8) %rt2};
*   &n2 @WIRE(mode="straight") {%rt3 (131, 0) (131, 6) #RS2:p};
*   &n2 @WIRE(mode="straight") {#R2B2:n (75, -8) (127, -8) %rt3};
*   &n2 @WIRE(mode="straight") {%rt3 #RS1:n};
*   &n3 @WIRE(mode="straight") {%rt4 (137, 0) (137, 6) #RS3:p};
*   &n3 @WIRE(mode="straight") {#R2B3:n (82, -7) (133, -7) %rt4};
*   &n3 @WIRE(mode="straight") {%rt4 #RS2:n};
*   &n4 @WIRE(mode="straight") {%rt5 (143, 0) (143, 6) #RS4:p};
*   &n4 @WIRE(mode="straight") {#R2B4:n (89, -6) (139, -6) %rt5};
*   &n4 @WIRE(mode="straight") {%rt5 #RS3:n};
*   &n5 @WIRE(mode="straight") {%rt6 (149, 0) (149, 6) #RS5:p};
*   &n5 @WIRE(mode="straight") {#R2B5:n (96, -5) (145, -5) %rt6};
*   &n5 @WIRE(mode="straight") {%rt6 #RS4:n};
*   &n6 @WIRE(mode="straight") {%rt7 (155, 0) (155, 6) #RS6:p};
*   &n6 @WIRE(mode="straight") {#R2B6:n (103, -4) (151, -4) %rt7};
*   &n6 @WIRE(mode="straight") {%rt7 #RS5:n};
*   &n7 @WIRE(mode="straight") {%rt8 #R2B7:n};
*   &n7 @WIRE(mode="straight") {#RS6:n (157, -2) %rt8};
*   &n7 @WIRE(mode="straight") {%rt8 %pout:p};
*   $JUNCTION @ATTACH %rt1;
*   $JUNCTION @ATTACH %rt2;
*   $JUNCTION @ATTACH %rt3;
*   $JUNCTION @ATTACH %rt4;
*   $JUNCTION @ATTACH %rt5;
*   $JUNCTION @ATTACH %rt6;
*   $JUNCTION @ATTACH %rt7;
*   $JUNCTION @ATTACH %rt8
* }
.end
