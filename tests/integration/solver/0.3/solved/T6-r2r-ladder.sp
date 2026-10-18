* T6 Eight-bit R-2R ladder
* @RNL(type="header") { @VERSION("0.3")}
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
*   $IO_PORT @ATTACH &n7 => %pout;
*   $WIRE_NET_SET @ROUTE
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:4fa1c9ef23e5e59f.22233424deb2d8ec") {
*   %ground_stub_VB0_n = $GROUND_STUB @ATTACH(gap=2) #VB0:n;
*   %ground_stub_VB1_n = $GROUND_STUB @ATTACH(gap=2) #VB1:n;
*   %ground_stub_VB2_n = $GROUND_STUB @ATTACH(gap=2) #VB2:n;
*   %ground_stub_VB3_n = $GROUND_STUB @ATTACH(gap=2) #VB3:n;
*   %ground_stub_VB4_n = $GROUND_STUB @ATTACH(gap=2) #VB4:n;
*   %ground_stub_VB5_n = $GROUND_STUB @ATTACH(gap=2) #VB5:n;
*   %ground_stub_VB6_n = $GROUND_STUB @ATTACH(gap=2) #VB6:n;
*   %ground_stub_VB7_n = $GROUND_STUB @ATTACH(gap=2) #VB7:n;
*   %ground_stub_RT_n = $GROUND_STUB @ATTACH(gap=2) #RT:n;
*   %rt1 = (67, 0);
*   %rt2 = (124, 0);
*   %rt3 = (130, 0);
*   %rt4 = (136, 0);
*   %rt5 = (142, 0);
*   %rt6 = (148, 0);
*   %rt7 = (154, 0);
*   %rt8 = (119, -2);
*   #VB0 @PLACE(-9, 2) => @ORIENT("R0");
*   #VB1 @PLACE(0, 2) => @ORIENT("R0");
*   #VB2 @PLACE(9, 2) => @ORIENT("R0");
*   #VB3 @PLACE(18, 2) => @ORIENT("R0");
*   #VB4 @PLACE(27, 2) => @ORIENT("R0");
*   #VB5 @PLACE(36, 2) => @ORIENT("R0");
*   #VB6 @PLACE(45, 2) => @ORIENT("R0");
*   #VB7 @PLACE(54, 2) => @ORIENT("R0");
*   #RT @PLACE(61, 2) => @ORIENT("R0");
*   #R2B0 @PLACE(67, 2) => @ORIENT("R0");
*   #R2B1 @PLACE(74, 2) => @ORIENT("R0");
*   #R2B2 @PLACE(81, 2) => @ORIENT("R0");
*   #R2B3 @PLACE(88, 2) => @ORIENT("R0");
*   #R2B4 @PLACE(95, 2) => @ORIENT("R0");
*   #R2B5 @PLACE(102, 2) => @ORIENT("R0");
*   #R2B6 @PLACE(109, 2) => @ORIENT("R0");
*   #R2B7 @PLACE(117, 2) => @ORIENT("R0");
*   #RS0 @PLACE(124, 2) => @ORIENT("R0");
*   #RS1 @PLACE(130, 2) => @ORIENT("R0");
*   #RS2 @PLACE(136, 2) => @ORIENT("R0");
*   #RS3 @PLACE(142, 2) => @ORIENT("R0");
*   #RS4 @PLACE(148, 2) => @ORIENT("R0");
*   #RS5 @PLACE(154, 2) => @ORIENT("R0");
*   #RS6 @PLACE(160, 2) => @ORIENT("R0");
*   %pout @PLACE(117, -2) => @ORIENT("R0");
*   %ground_stub_VB0_n::g @PLACE(-9, -2) => @ORIENT("R0");
*   %ground_stub_VB1_n::g @PLACE(0, -2) => @ORIENT("R0");
*   %ground_stub_VB2_n::g @PLACE(9, -2) => @ORIENT("R0");
*   %ground_stub_VB3_n::g @PLACE(18, -2) => @ORIENT("R0");
*   %ground_stub_VB4_n::g @PLACE(27, -2) => @ORIENT("R0");
*   %ground_stub_VB5_n::g @PLACE(36, -2) => @ORIENT("R0");
*   %ground_stub_VB6_n::g @PLACE(45, -2) => @ORIENT("R0");
*   %ground_stub_VB7_n::g @PLACE(54, -2) => @ORIENT("R0");
*   %ground_stub_RT_n::g @PLACE(61, -2) => @ORIENT("R0");
*   &b0 @WIRE(mode="straight") {#VB0:p (-9, 6) (67, 6) #R2B0:p};
*   &b1 @WIRE(mode="straight") {#VB1:p (5, 4) (5, -9) (72, -9) (72, 4) #R2B1:p};
*   &b2 @WIRE(mode="straight") {#VB2:p (14, 4) (14, -8) (79, -8) (79, 4) #R2B2:p};
*   &b3 @WIRE(mode="straight") {#VB3:p (23, 4) (23, -7) (86, -7) (86, 4) #R2B3:p};
*   &b4 @WIRE(mode="straight") {#VB4:p (32, 4) (32, -6) (93, -6) (93, 4) #R2B4:p};
*   &b5 @WIRE(mode="straight") {#VB5:p (36, 9) (102, 9) #R2B5:p};
*   &b6 @WIRE(mode="straight") {#VB6:p (45, 8) (109, 8) #R2B6:p};
*   &b7 @WIRE(mode="straight") {#VB7:p (54, 7) (117, 7) #R2B7:p};
*   &n0 @WIRE(mode="straight") {#RT:p (65, 4) (65, 0) %rt1};
*   &n0 @WIRE(mode="straight") {#RS0:p (124, 6) (97, 6) (97, -2) (67, -2) %rt1};
*   &n0 @WIRE(mode="straight") {%rt1 #R2B0:n};
*   &n1 @WIRE(mode="straight") {%rt2 (128, 0) (128, 4) #RS1:p};
*   &n1 @WIRE(mode="straight") {#R2B1:n (74, -9) (124, -9) %rt2};
*   &n1 @WIRE(mode="straight") {%rt2 #RS0:n};
*   &n2 @WIRE(mode="straight") {%rt3 (134, 0) (134, 4) #RS2:p};
*   &n2 @WIRE(mode="straight") {#R2B2:n (81, -8) (130, -8) %rt3};
*   &n2 @WIRE(mode="straight") {%rt3 #RS1:n};
*   &n3 @WIRE(mode="straight") {%rt4 (140, 0) (140, 4) #RS3:p};
*   &n3 @WIRE(mode="straight") {#R2B3:n (88, -7) (136, -7) %rt4};
*   &n3 @WIRE(mode="straight") {%rt4 #RS2:n};
*   &n4 @WIRE(mode="straight") {%rt5 (146, 0) (146, 4) #RS4:p};
*   &n4 @WIRE(mode="straight") {#R2B4:n (95, -6) (142, -6) %rt5};
*   &n4 @WIRE(mode="straight") {%rt5 #RS3:n};
*   &n5 @WIRE(mode="straight") {%rt6 (152, 0) (152, 4) #RS5:p};
*   &n5 @WIRE(mode="straight") {#R2B5:n (102, -5) (148, -5) %rt6};
*   &n5 @WIRE(mode="straight") {%rt6 #RS4:n};
*   &n6 @WIRE(mode="straight") {%rt7 (158, 0) (158, 4) #RS6:p};
*   &n6 @WIRE(mode="straight") {#R2B6:n (109, -4) (154, -4) %rt7};
*   &n6 @WIRE(mode="straight") {%rt7 #RS5:n};
*   &n7 @WIRE(mode="straight") {%pout:p %rt8};
*   &n7 @WIRE(mode="straight") {%rt8 (119, 0) #R2B7:n};
*   &n7 @WIRE(mode="straight") {#RS6:n (160, -3) (119, -3) %rt8};
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
