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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig1:10264c68f21824a7.e17f1522b2b059c0") {
*   %gnd_stub_VB0_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB0:n;
*   %gnd_stub_VB1_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB1:n;
*   %gnd_stub_VB2_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB2:n;
*   %gnd_stub_VB3_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB3:n;
*   %gnd_stub_VB4_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB4:n;
*   %gnd_stub_VB5_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB5:n;
*   %gnd_stub_VB6_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB6:n;
*   %gnd_stub_VB7_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VB7:n;
*   %gnd_stub_RT_n = $GND_STUB @ATTACH(gap=2, priority="normal") #RT:n;
*   %rt1 = (26, 7);
*   %rt2 = (48, 6);
*   %rt3 = (108, 6);
*   %rt4 = (147, 9);
*   %rt5 = (199, 9);
*   %rt6 = (217, 9);
*   %rt7 = (251, 6);
*   %rt8 = (264, 9);
*   #VB0 @PLACE(0, 3) => @ORIENT("R0");
*   #VB1 @PLACE(65, 3) => @ORIENT("R0");
*   #VB2 @PLACE(91, 3) => @ORIENT("R0");
*   #VB3 @PLACE(130, 6) => @ORIENT("R0");
*   #VB4 @PLACE(169, 9) => @ORIENT("R0");
*   #VB5 @PLACE(273, 9) => @ORIENT("R0");
*   #VB6 @PLACE(299, 6) => @ORIENT("R0");
*   #VB7 @PLACE(182, 9) => @ORIENT("R0");
*   #RT @PLACE(26, 3) => @ORIENT("R0");
*   #R2B0 @PLACE(13, 6) => @ORIENT("R90");
*   #R2B1 @PLACE(52, 6) => @ORIENT("R270");
*   #R2B2 @PLACE(104, 6) => @ORIENT("R90");
*   #R2B3 @PLACE(143, 9) => @ORIENT("R90");
*   #R2B4 @PLACE(195, 9) => @ORIENT("R90");
*   #R2B5 @PLACE(221, 9) => @ORIENT("R270");
*   #R2B6 @PLACE(286, 9) => @ORIENT("R270");
*   #R2B7 @PLACE(260, 9) => @ORIENT("R90");
*   #RS0 @PLACE(39, 6) => @ORIENT("R90");
*   #RS1 @PLACE(78, 3) => @ORIENT("R90");
*   #RS2 @PLACE(117, 6) => @ORIENT("R90");
*   #RS3 @PLACE(156, 9) => @ORIENT("R90");
*   #RS4 @PLACE(208, 9) => @ORIENT("R90");
*   #RS5 @PLACE(247, 9) => @ORIENT("R90");
*   #RS6 @PLACE(234, 9) => @ORIENT("R270");
*   %pout @PLACE(265, 9) => @ORIENT("MY");
*   %gnd_stub_VB0_n::g @PLACE(0, -2) => @ORIENT("R0");
*   %gnd_stub_VB1_n::g @PLACE(65, -2) => @ORIENT("R0");
*   %gnd_stub_VB2_n::g @PLACE(91, -2) => @ORIENT("R0");
*   %gnd_stub_VB3_n::g @PLACE(130, 1) => @ORIENT("R0");
*   %gnd_stub_VB4_n::g @PLACE(169, 4) => @ORIENT("R0");
*   %gnd_stub_VB5_n::g @PLACE(273, 4) => @ORIENT("R0");
*   %gnd_stub_VB6_n::g @PLACE(299, 1) => @ORIENT("R0");
*   %gnd_stub_VB7_n::g @PLACE(182, 4) => @ORIENT("R0");
*   %gnd_stub_RT_n::g @PLACE(26, -2) => @ORIENT("R0");
*   &b0 @WIRE(mode="straight") {#VB0:p (0, 7) (9, 7) (9, 6) #R2B0:p};
*   &b1 @WIRE(mode="straight") {#R2B1:p (62, 6) (62, 7) (65, 7) #VB1:p};
*   &b2 @WIRE(mode="straight") {#VB2:p (91, 7) (100, 7) (100, 6) #R2B2:p};
*   &b3 @WIRE(mode="straight") {#VB3:p (130, 10) (139, 10) (139, 9) #R2B3:p};
*   &b4 @WIRE(mode="straight") {#VB4:p (169, 14) (191, 14) (191, 9) #R2B4:p};
*   &b5 @WIRE(mode="straight") {#R2B5:p (229, 9) (229, 13) (273, 13) #VB5:p};
*   &b6 @WIRE(mode="straight") {#R2B6:p (296, 9) (296, 10) (299, 10) #VB6:p};
*   &b7 @WIRE(mode="straight") {#VB7:p (182, 13) (190, 13) (190, 4) (256, 4) (256, 9) #R2B7:p};
*   &n0 @WIRE(mode="straight") {#R2B0:n (24, 6) (24, 7) %rt1};
*   &n0 @WIRE(mode="straight") {%rt1 #RT:p};
*   &n0 @WIRE(mode="straight") {#RS0:p (28, 6) (28, 7) %rt1};
*   &n1 @WIRE(mode="straight") {#RS0:n %rt2};
*   &n1 @WIRE(mode="straight") {%rt2 #R2B1:n};
*   &n1 @WIRE(mode="straight") {#RS1:p (68, 3) (68, 8) (48, 8) %rt2};
*   &n2 @WIRE(mode="straight") {#R2B2:n %rt3};
*   &n2 @WIRE(mode="straight") {%rt3 #RS2:p};
*   &n2 @WIRE(mode="straight") {#RS1:n (88, 3) (88, 8) (108, 8) %rt3};
*   &n3 @WIRE(mode="straight") {#R2B3:n %rt4};
*   &n3 @WIRE(mode="straight") {%rt4 #RS3:p};
*   &n3 @WIRE(mode="straight") {#RS2:n (127, 6) (127, 11) (147, 11) %rt4};
*   &n4 @WIRE(mode="straight") {#R2B4:n %rt5};
*   &n4 @WIRE(mode="straight") {%rt5 #RS4:p};
*   &n4 @WIRE(mode="straight") {#RS3:n (166, 9) (166, 15) (199, 15) %rt5};
*   &n5 @WIRE(mode="straight") {#RS4:n %rt6};
*   &n5 @WIRE(mode="straight") {%rt6 #R2B5:n};
*   &n5 @WIRE(mode="straight") {#RS5:p (239, 9) (239, 7) (217, 7) %rt6};
*   &n6 @WIRE(mode="straight") {#RS6:p (238, 9) (238, 6) %rt7};
*   &n6 @WIRE(mode="straight") {%rt7 (251, 9) #RS5:n};
*   &n6 @WIRE(mode="straight") {#R2B6:n (276, 9) (276, 5) (251, 5) %rt7};
*   &n7 @WIRE(mode="straight") {#R2B7:n %rt8};
*   &n7 @WIRE(mode="straight") {%rt8 %pout:p};
*   &n7 @WIRE(mode="straight") {#RS6:n (230, 9) (230, 11) (264, 11) %rt8};
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
