* T6 Eight-bit R-2R ladder
* @RNL(type="header") { @VERSION("0.2")}
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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig1:10264c68f21824a7.1f3b310f8bee492a") {
*   %gnd_stub_VB0_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB0:n;
*   %gnd_stub_VB1_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB1:n;
*   %gnd_stub_VB2_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB2:n;
*   %gnd_stub_VB3_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB3:n;
*   %gnd_stub_VB4_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB4:n;
*   %gnd_stub_VB5_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB5:n;
*   %gnd_stub_VB6_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB6:n;
*   %gnd_stub_VB7_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB7:n;
*   %gnd_stub_RT_n = $GND_STUB @ATTACH(gap=2, priority="normal") #RT:n;
*   %rt1 = (16, 4);
*   %rt2 = (28, 4);
*   %rt3 = (70, 4);
*   %rt4 = (78, 4);
*   %rt5 = (103, 0);
*   %rt6 = (136, -4);
*   %rt7 = (175, -4);
*   %rt8 = (152, -4);
*   #VB0 @PLACE(0, 0) => @ORIENT("R0");
*   #VB1 @PLACE(39, 0) => @ORIENT("R0");
*   #VB2 @PLACE(56, 0) => @ORIENT("R0");
*   #VB3 @PLACE(89, 0) => @ORIENT("R0");
*   #VB4 @PLACE(114, -4) => @ORIENT("R0");
*   #VB5 @PLACE(194, -8) => @ORIENT("R0");
*   #VB6 @PLACE(186, -8) => @ORIENT("R0");
*   #VB7 @PLACE(122, -4) => @ORIENT("R0");
*   #RT @PLACE(16, 0) => @ORIENT("R0");
*   #R2B0 @PLACE(10, 4) => @ORIENT("R90");
*   #R2B1 @PLACE(32, 4) => @ORIENT("R270");
*   #R2B2 @PLACE(66, 4) => @ORIENT("R90");
*   #R2B3 @PLACE(82, 4) => @ORIENT("R270");
*   #R2B4 @PLACE(107, 0) => @ORIENT("R270");
*   #R2B5 @PLACE(140, -4) => @ORIENT("R270");
*   #R2B6 @PLACE(179, -4) => @ORIENT("R270");
*   #R2B7 @PLACE(148, -4) => @ORIENT("R90");
*   #RS0 @PLACE(24, 4) => @ORIENT("R90");
*   #RS1 @PLACE(49, 0) => @ORIENT("R90");
*   #RS2 @PLACE(74, 4) => @ORIENT("R90");
*   #RS3 @PLACE(99, 0) => @ORIENT("R90");
*   #RS4 @PLACE(132, -4) => @ORIENT("R90");
*   #RS5 @PLACE(171, -4) => @ORIENT("R90");
*   #RS6 @PLACE(163, -4) => @ORIENT("R270");
*   %pout @PLACE(153, -4) => @ORIENT("MY");
*   %gnd_stub_VB0_n::g @PLACE(0, -5) => @ORIENT("R0");
*   %gnd_stub_VB1_n::g @PLACE(39, -5) => @ORIENT("R0");
*   %gnd_stub_VB2_n::g @PLACE(56, -5) => @ORIENT("R0");
*   %gnd_stub_VB3_n::g @PLACE(89, -5) => @ORIENT("R0");
*   %gnd_stub_VB4_n::g @PLACE(114, -9) => @ORIENT("R0");
*   %gnd_stub_VB5_n::g @PLACE(194, -13) => @ORIENT("R0");
*   %gnd_stub_VB6_n::g @PLACE(186, -13) => @ORIENT("R0");
*   %gnd_stub_VB7_n::g @PLACE(122, -9) => @ORIENT("R0");
*   %gnd_stub_RT_n::g @PLACE(16, -5) => @ORIENT("R0");
*   &b0 @WIRE(mode="straight") {#VB0:p (0, 4) #R2B0:p};
*   &b1 @WIRE(mode="straight") {#R2B1:p (39, 4) #VB1:p};
*   &b2 @WIRE(mode="straight") {#VB2:p (56, 4) #R2B2:p};
*   &b3 @WIRE(mode="straight") {#R2B3:p (89, 4) #VB3:p};
*   &b4 @WIRE(mode="straight") {#R2B4:p (114, 0) #VB4:p};
*   &b5 @WIRE(mode="straight") {#R2B5:p (143, -1) (194, -1) #VB5:p};
*   &b6 @WIRE(mode="straight") {#R2B6:p (186, -4) #VB6:p};
*   &b7 @WIRE(mode="straight") {#VB7:p (122, 0) (144, 0) (144, -4) #R2B7:p};
*   &n0 @WIRE(mode="straight") {#R2B0:n %rt1};
*   &n0 @WIRE(mode="straight") {%rt1 #RT:p};
*   &n0 @WIRE(mode="straight") {#RS0:p %rt1};
*   &n1 @WIRE(mode="straight") {#RS0:n %rt2};
*   &n1 @WIRE(mode="straight") {%rt2 #R2B1:n};
*   &n1 @WIRE(mode="straight") {#RS1:p (42, 0) (42, 6) (28, 6) %rt2};
*   &n2 @WIRE(mode="straight") {#R2B2:n %rt3};
*   &n2 @WIRE(mode="straight") {%rt3 #RS2:p};
*   &n2 @WIRE(mode="straight") {#RS1:n (53, 0) (53, 6) (70, 6) %rt3};
*   &n3 @WIRE(mode="straight") {#RS2:n %rt4};
*   &n3 @WIRE(mode="straight") {%rt4 #R2B3:n};
*   &n3 @WIRE(mode="straight") {#RS3:p (92, 0) (92, 6) (78, 6) %rt4};
*   &n4 @WIRE(mode="straight") {#RS3:n %rt5};
*   &n4 @WIRE(mode="straight") {%rt5 #R2B4:n};
*   &n4 @WIRE(mode="straight") {#RS4:p (125, -4) (125, 2) (103, 2) %rt5};
*   &n5 @WIRE(mode="straight") {#RS4:n %rt6};
*   &n5 @WIRE(mode="straight") {%rt6 #R2B5:n};
*   &n5 @WIRE(mode="straight") {#RS5:p (168, -2) (136, -2) %rt6};
*   &n6 @WIRE(mode="straight") {#RS5:n %rt7};
*   &n6 @WIRE(mode="straight") {%rt7 #R2B6:n};
*   &n6 @WIRE(mode="straight") {#RS6:p (167, -4) (167, -6) (175, -6) %rt7};
*   &n7 @WIRE(mode="straight") {#R2B7:n %rt8};
*   &n7 @WIRE(mode="straight") {%rt8 %pout:p};
*   &n7 @WIRE(mode="straight") {#RS6:n (157, -4) (157, -6) (152, -6) %rt8};
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
