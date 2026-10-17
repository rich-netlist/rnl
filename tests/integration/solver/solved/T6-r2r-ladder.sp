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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:69b3dd72d03b1950.1f3b310f8bee492a") {
*   %gnd_stub_VB0_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB0:n;
*   %gnd_stub_VB1_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB1:n;
*   %gnd_stub_VB2_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB2:n;
*   %gnd_stub_VB3_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB3:n;
*   %gnd_stub_VB4_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB4:n;
*   %gnd_stub_VB5_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB5:n;
*   %gnd_stub_VB6_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB6:n;
*   %gnd_stub_VB7_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB7:n;
*   %gnd_stub_RT_n = $GND_STUB @ATTACH(gap=2, priority="normal") #RT:n;
*   %rt1 = (13, 4);
*   %rt2 = (25, 4);
*   %rt3 = (67, 4);
*   %rt4 = (92, 8);
*   %rt5 = (117, 12);
*   %rt6 = (125, 12);
*   %rt7 = (174, 16);
*   %rt8 = (150, 16);
*   #VB0 @PLACE(0, 0) => @ORIENT("MY");
*   #VB1 @PLACE(36, 0) => @ORIENT("R0");
*   #VB2 @PLACE(56, 0) => @ORIENT("MY");
*   #VB3 @PLACE(81, 4) => @ORIENT("MY");
*   #VB4 @PLACE(106, 8) => @ORIENT("MY");
*   #VB5 @PLACE(193, 8) => @ORIENT("R0");
*   #VB6 @PLACE(185, 8) => @ORIENT("R0");
*   #VB7 @PLACE(139, 12) => @ORIENT("MY");
*   #RT @PLACE(13, 0) => @ORIENT("R0");
*   #R2B0 @PLACE(7, 4) => @ORIENT("R90");
*   #R2B1 @PLACE(29, 4) => @ORIENT("R270");
*   #R2B2 @PLACE(63, 4) => @ORIENT("R90");
*   #R2B3 @PLACE(88, 8) => @ORIENT("R90");
*   #R2B4 @PLACE(113, 12) => @ORIENT("R90");
*   #R2B5 @PLACE(129, 12) => @ORIENT("R270");
*   #R2B6 @PLACE(178, 16) => @ORIENT("R270");
*   #R2B7 @PLACE(146, 16) => @ORIENT("R90");
*   #RS0 @PLACE(21, 4) => @ORIENT("R90");
*   #RS1 @PLACE(46, 0) => @ORIENT("R90");
*   #RS2 @PLACE(71, 4) => @ORIENT("R90");
*   #RS3 @PLACE(96, 8) => @ORIENT("R90");
*   #RS4 @PLACE(121, 12) => @ORIENT("R90");
*   #RS5 @PLACE(161, 16) => @ORIENT("R90");
*   #RS6 @PLACE(170, 16) => @ORIENT("R270");
*   %pout @PLACE(151, 16) => @ORIENT("MY");
*   %gnd_stub_VB0_n::g @PLACE(0, -5) => @ORIENT("R0");
*   %gnd_stub_VB1_n::g @PLACE(36, -5) => @ORIENT("R0");
*   %gnd_stub_VB2_n::g @PLACE(56, -5) => @ORIENT("R0");
*   %gnd_stub_VB3_n::g @PLACE(81, -1) => @ORIENT("R0");
*   %gnd_stub_VB4_n::g @PLACE(106, 3) => @ORIENT("R0");
*   %gnd_stub_VB5_n::g @PLACE(193, 3) => @ORIENT("R0");
*   %gnd_stub_VB6_n::g @PLACE(185, 3) => @ORIENT("R0");
*   %gnd_stub_VB7_n::g @PLACE(139, 7) => @ORIENT("R0");
*   %gnd_stub_RT_n::g @PLACE(13, -5) => @ORIENT("R0");
*   &b0 @WIRE(mode="straight") {#VB0:p (0, 4) #R2B0:p};
*   &b1 @WIRE(mode="straight") {#R2B1:p (36, 4) #VB1:p};
*   &b2 @WIRE(mode="straight") {#VB2:p (56, 4) #R2B2:p};
*   &b3 @WIRE(mode="straight") {#VB3:p (81, 8) #R2B3:p};
*   &b4 @WIRE(mode="straight") {#VB4:p (106, 12) #R2B4:p};
*   &b5 @WIRE(mode="straight") {#R2B5:p (136, 12) (136, 8) (182, 8) (182, 12) (193, 12) #VB5:p};
*   &b6 @WIRE(mode="straight") {#R2B6:p (185, 16) #VB6:p};
*   &b7 @WIRE(mode="straight") {#VB7:p (139, 16) #R2B7:p};
*   &n0 @WIRE(mode="straight") {#R2B0:n %rt1};
*   &n0 @WIRE(mode="straight") {%rt1 #RT:p};
*   &n0 @WIRE(mode="straight") {#RS0:p %rt1};
*   &n1 @WIRE(mode="straight") {#RS0:n %rt2};
*   &n1 @WIRE(mode="straight") {%rt2 #R2B1:n};
*   &n1 @WIRE(mode="straight") {#RS1:p (39, 0) (39, 6) (25, 6) %rt2};
*   &n2 @WIRE(mode="straight") {#R2B2:n %rt3};
*   &n2 @WIRE(mode="straight") {%rt3 #RS2:p};
*   &n2 @WIRE(mode="straight") {#RS1:n (53, 0) (53, 6) (67, 6) %rt3};
*   &n3 @WIRE(mode="straight") {#R2B3:n %rt4};
*   &n3 @WIRE(mode="straight") {%rt4 #RS3:p};
*   &n3 @WIRE(mode="straight") {#RS2:n (78, 4) (78, 10) (92, 10) %rt4};
*   &n4 @WIRE(mode="straight") {#R2B4:n %rt5};
*   &n4 @WIRE(mode="straight") {%rt5 #RS4:p};
*   &n4 @WIRE(mode="straight") {#RS3:n (103, 8) (103, 14) (117, 14) %rt5};
*   &n5 @WIRE(mode="straight") {#RS4:n %rt6};
*   &n5 @WIRE(mode="straight") {%rt6 #R2B5:n};
*   &n5 @WIRE(mode="straight") {#RS5:p (155, 16) (155, 19) (125, 19) %rt6};
*   &n6 @WIRE(mode="straight") {#RS6:p %rt7};
*   &n6 @WIRE(mode="straight") {%rt7 #R2B6:n};
*   &n6 @WIRE(mode="straight") {#RS5:n (165, 16) (165, 14) (174, 14) %rt7};
*   &n7 @WIRE(mode="straight") {#R2B7:n %rt8};
*   &n7 @WIRE(mode="straight") {%rt8 %pout:p};
*   &n7 @WIRE(mode="straight") {#RS6:n (166, 16) (166, 18) (150, 18) %rt8};
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
