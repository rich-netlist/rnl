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
*   %rt1 = (30, 4);
*   %rt2 = (56, 4);
*   %rt3 = (124, 4);
*   %rt4 = (146, 4);
*   %rt5 = (191, 0);
*   %rt6 = (251, -4);
*   %rt7 = (304, -4);
*   %rt8 = (274, -4);
*   #VB0 @PLACE(0, 0) => @ORIENT("R0");
*   #VB1 @PLACE(75, 0) => @ORIENT("R0");
*   #VB2 @PLACE(105, 0) => @ORIENT("R0");
*   #VB3 @PLACE(165, 0) => @ORIENT("R0");
*   #VB4 @PLACE(210, -4) => @ORIENT("R0");
*   #VB5 @PLACE(345, -8) => @ORIENT("R0");
*   #VB6 @PLACE(330, -8) => @ORIENT("R0");
*   #VB7 @PLACE(225, -4) => @ORIENT("R0");
*   #RT @PLACE(30, 0) => @ORIENT("R0");
*   #R2B0 @PLACE(15, 4) => @ORIENT("R90");
*   #R2B1 @PLACE(60, 4) => @ORIENT("R270");
*   #R2B2 @PLACE(120, 4) => @ORIENT("R90");
*   #R2B3 @PLACE(150, 4) => @ORIENT("R270");
*   #R2B4 @PLACE(195, 0) => @ORIENT("R270");
*   #R2B5 @PLACE(255, -4) => @ORIENT("R270");
*   #R2B6 @PLACE(315, -4) => @ORIENT("R270");
*   #R2B7 @PLACE(270, -4) => @ORIENT("R90");
*   #RS0 @PLACE(45, 4) => @ORIENT("R90");
*   #RS1 @PLACE(90, 0) => @ORIENT("R90");
*   #RS2 @PLACE(135, 4) => @ORIENT("R90");
*   #RS3 @PLACE(180, 0) => @ORIENT("R90");
*   #RS4 @PLACE(240, -4) => @ORIENT("R90");
*   #RS5 @PLACE(300, -4) => @ORIENT("R90");
*   #RS6 @PLACE(285, -4) => @ORIENT("R270");
*   %pout @PLACE(275, -4) => @ORIENT("MY");
*   %gnd_stub_VB0_n::g @PLACE(0, -5) => @ORIENT("R0");
*   %gnd_stub_VB1_n::g @PLACE(75, -5) => @ORIENT("R0");
*   %gnd_stub_VB2_n::g @PLACE(105, -5) => @ORIENT("R0");
*   %gnd_stub_VB3_n::g @PLACE(165, -5) => @ORIENT("R0");
*   %gnd_stub_VB4_n::g @PLACE(210, -9) => @ORIENT("R0");
*   %gnd_stub_VB5_n::g @PLACE(345, -13) => @ORIENT("R0");
*   %gnd_stub_VB6_n::g @PLACE(330, -13) => @ORIENT("R0");
*   %gnd_stub_VB7_n::g @PLACE(225, -9) => @ORIENT("R0");
*   %gnd_stub_RT_n::g @PLACE(30, -5) => @ORIENT("R0");
*   &b0 @WIRE(mode="straight") {#VB0:p (0, 4) #R2B0:p};
*   &b1 @WIRE(mode="straight") {#R2B1:p (75, 4) #VB1:p};
*   &b2 @WIRE(mode="straight") {#VB2:p (105, 4) #R2B2:p};
*   &b3 @WIRE(mode="straight") {#R2B3:p (165, 4) #VB3:p};
*   &b4 @WIRE(mode="straight") {#R2B4:p (210, 0) #VB4:p};
*   &b5 @WIRE(mode="straight") {#R2B5:p (265, -4) (265, -1) (345, -1) #VB5:p};
*   &b6 @WIRE(mode="straight") {#R2B6:p (330, -4) #VB6:p};
*   &b7 @WIRE(mode="straight") {#VB7:p (225, 0) (266, 0) (266, -4) #R2B7:p};
*   &n0 @WIRE(mode="straight") {#R2B0:n %rt1};
*   &n0 @WIRE(mode="straight") {%rt1 #RT:p};
*   &n0 @WIRE(mode="straight") {#RS0:p %rt1};
*   &n1 @WIRE(mode="straight") {#RS0:n %rt2};
*   &n1 @WIRE(mode="straight") {%rt2 #R2B1:n};
*   &n1 @WIRE(mode="straight") {#RS1:p (78, 0) (78, 6) (56, 6) %rt2};
*   &n2 @WIRE(mode="straight") {#R2B2:n %rt3};
*   &n2 @WIRE(mode="straight") {%rt3 #RS2:p};
*   &n2 @WIRE(mode="straight") {#RS1:n (102, 0) (102, 6) (124, 6) %rt3};
*   &n3 @WIRE(mode="straight") {#RS2:n %rt4};
*   &n3 @WIRE(mode="straight") {%rt4 #R2B3:n};
*   &n3 @WIRE(mode="straight") {#RS3:p (168, 0) (168, 6) (146, 6) %rt4};
*   &n4 @WIRE(mode="straight") {#RS3:n %rt5};
*   &n4 @WIRE(mode="straight") {%rt5 #R2B4:n};
*   &n4 @WIRE(mode="straight") {#RS4:p (228, -4) (228, 2) (191, 2) %rt5};
*   &n5 @WIRE(mode="straight") {#RS4:n %rt6};
*   &n5 @WIRE(mode="straight") {%rt6 #R2B5:n};
*   &n5 @WIRE(mode="straight") {#RS5:p (296, -4) (296, -2) (251, -2) %rt6};
*   &n6 @WIRE(mode="straight") {#RS5:n %rt7};
*   &n6 @WIRE(mode="straight") {%rt7 #R2B6:n};
*   &n6 @WIRE(mode="straight") {#RS6:p (295, -4) (295, -6) (304, -6) %rt7};
*   &n7 @WIRE(mode="straight") {#R2B7:n %rt8};
*   &n7 @WIRE(mode="straight") {%rt8 %pout:p};
*   &n7 @WIRE(mode="straight") {#RS6:n (279, -4) (279, -6) (274, -6) %rt8};
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
