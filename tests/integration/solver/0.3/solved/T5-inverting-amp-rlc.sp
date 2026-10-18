* T5 Inverting amplifier with an RLC output filter
* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="rnl-std-functional", version="0.3")}
.subckt opamp inp inn out
E1 out 0 inp inn 1e5
* @RNL { # @SET-SYMBOL $OPAMP }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:e409143787de84f6.22233424deb2d8ec") {
*   %ground_stub_E1_n = $GROUND_STUB @ATTACH(gap=2) #E1:n;
*   %io_port_inp = $IO_PORT @ATTACH &inp;
*   %io_port_inn = $IO_PORT @ATTACH &inn;
*   %io_port_out = $IO_PORT @ATTACH &out;
*   #E1 @PLACE(0, 2) => @ORIENT("R0");
*   %ground_stub_E1_n::g @PLACE(0, -2) => @ORIENT("R0");
*   %io_port_inp @PLACE(-7, 3) => @ORIENT("R0");
*   %io_port_inn @PLACE(-7, 1) => @ORIENT("R0");
*   %io_port_out @PLACE(0, 6) => @ORIENT("R0");
*   &inp @WIRE(mode="straight") {%io_port_inp:p (-7, 6) (-4, 6) #E1:cp};
*   &inn @WIRE(mode="straight") {%io_port_inn:p (-7, -2) (-4, -2) #E1:cn};
*   &out @WIRE(mode="straight") {#E1:p (3, 4) (3, 6) %io_port_out:p}
* }
.ends opamp
XA 0 n1 vo opamp
RI vin n1 10k
RF n1 vo 100k
L1 vo n2 10u
C1 n2 0 1n
RL n2 0 1k
VIN vin 0 AC 1
* @RNL {
*   {#VIN #RI #XA #L1 {#C1 #RL}} @AS $FLOW;
*   $IO_PORT @ATTACH &n2 => %pout;
*   $WIRE_NET_SET @ROUTE
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:4944800cb64a1c0f.22233424deb2d8ec") {
*   %ground_stub_XA_inp = $GROUND_STUB @ATTACH(gap=2) #XA:inp;
*   %ground_stub_C1_n = $GROUND_STUB @ATTACH(gap=2) #C1:n;
*   %ground_stub_RL_n = $GROUND_STUB @ATTACH(gap=2) #RL:n;
*   %ground_stub_VIN_n = $GROUND_STUB @ATTACH(gap=2) #VIN:n;
*   %rt1 = (17, 3);
*   %rt2 = (26, 3);
*   %rt3 = (37, 2);
*   %rt4 = (41, 2);
*   #XA @PLACE(16, 8) => @ORIENT("R90");
*   #RI @PLACE(7, 3) => @ORIENT("R90");
*   #RF @PLACE(24, 3) => @ORIENT("R90");
*   #L1 @PLACE(31, 3) => @ORIENT("R90");
*   #C1 @PLACE(41, 0) => @ORIENT("R0");
*   #RL @PLACE(47, 0) => @ORIENT("R0");
*   #VIN @PLACE(0, 3) => @ORIENT("MYR90");
*   %pout @PLACE(37, 3) => @ORIENT("R0");
*   %ground_stub_XA_inp::g @PLACE(15, 2) => @ORIENT("R0");
*   %ground_stub_C1_n::g @PLACE(41, -4) => @ORIENT("R0");
*   %ground_stub_RL_n::g @PLACE(47, -4) => @ORIENT("R0");
*   %ground_stub_VIN_n::g @PLACE(2, 1) => @ORIENT("R0");
*   &n1 @WIRE(mode="straight") {#XA:inn %rt1};
*   &n1 @WIRE(mode="straight") {%rt1 #RF:p};
*   &n1 @WIRE(mode="straight") {#RI:n %rt1};
*   &vo @WIRE(mode="straight") {%rt2 (26, 1) (29, 1) #L1:p};
*   &vo @WIRE(mode="straight") {#XA:out (26, 11) %rt2};
*   &vo @WIRE(mode="straight") {%rt2 #RF:n};
*   &vin @WIRE(mode="straight") {#VIN:p (-2, 7) (5, 7) #RI:p};
*   &n2 @WIRE(mode="straight") {#L1:n (33, 2) %rt3};
*   &n2 @WIRE(mode="straight") {%rt3 %pout:p};
*   &n2 @WIRE(mode="straight") {%rt4 %rt3};
*   &n2 @WIRE(mode="straight") {#RL:p %rt4};
*   &n2 @WIRE(mode="straight") {%rt4 #C1:p};
*   $JUNCTION @ATTACH %rt1;
*   $JUNCTION @ATTACH %rt2;
*   $JUNCTION @ATTACH %rt3;
*   $JUNCTION @ATTACH %rt4
* }
.end
