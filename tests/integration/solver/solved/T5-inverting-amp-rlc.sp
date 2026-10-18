* T5 Inverting amplifier with an RLC output filter
* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="rnl-std", version="0.2")}
.subckt opamp inp inn out
E1 out 0 inp inn 1e5
* @RNL { # @SET-SYMBOL $OPAMP }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:2ff014ec97d55902.22233424deb2d8ec") {
*   %gnd_stub_E1_n = $GND_STUB @ATTACH(gap=2) #E1:n;
*   %ioport_inp = $IOPORT @ATTACH &inp;
*   %ioport_inn = $IOPORT @ATTACH &inn;
*   %ioport_out = $IOPORT @ATTACH &out;
*   #E1 @PLACE(0, 3) => @ORIENT("R0");
*   %gnd_stub_E1_n::g @PLACE(0, -2) => @ORIENT("R0");
*   %ioport_inp @PLACE(-6, 4) => @ORIENT("R0");
*   %ioport_inn @PLACE(-6, 2) => @ORIENT("R0");
*   %ioport_out @PLACE(0, 8) => @ORIENT("R0");
*   &inp @WIRE(mode="straight") {%ioport_inp:p #E1:cp};
*   &inn @WIRE(mode="straight") {%ioport_inn:p #E1:cn};
*   &out @WIRE(mode="straight") {#E1:p %ioport_out:p}
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
*   $IOPORT @ATTACH &n2 => %pout;
*   $WIRE_NET_SET @ROUTE
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig3:fa9ffcd48205aede.22233424deb2d8ec") {
*   %gnd_stub_XA_inp = $GND_STUB @ATTACH(gap=2) #XA:inp;
*   %gnd_stub_C1_n = $GND_STUB @ATTACH(gap=2) #C1:n;
*   %gnd_stub_RL_n = $GND_STUB @ATTACH(gap=2) #RL:n;
*   %gnd_stub_VIN_n = $GND_STUB @ATTACH(gap=2) #VIN:n;
*   %rt1 = (9, -1);
*   %rt2 = (20, -2);
*   %rt3 = (27, -9);
*   %rt4 = (33, -4);
*   #XA @PLACE(16, -2) => @ORIENT("R0");
*   #RI @PLACE(7, 3) => @ORIENT("R0");
*   #RF @PLACE(-7, 3) => @ORIENT("R0");
*   #L1 @PLACE(27, -6) => @ORIENT("R0");
*   #C1 @PLACE(33, -6) => @ORIENT("R0");
*   #RL @PLACE(39, -7) => @ORIENT("R0");
*   #VIN @PLACE(0, 3) => @ORIENT("R0");
*   %pout @PLACE(27, -11) => @ORIENT("R0");
*   %gnd_stub_XA_inp::g @PLACE(12, -5) => @ORIENT("R0");
*   %gnd_stub_C1_n::g @PLACE(33, -10) => @ORIENT("R0");
*   %gnd_stub_RL_n::g @PLACE(39, -12) => @ORIENT("R0");
*   %gnd_stub_VIN_n::g @PLACE(0, -2) => @ORIENT("R0");
*   &n1 @WIRE(mode="straight") {#RI:n (7, -1) %rt1};
*   &n1 @WIRE(mode="straight") {%rt1 #XA:inn};
*   &n1 @WIRE(mode="straight") {#RF:p (-7, 7) (9, 7) %rt1};
*   &vo @WIRE(mode="straight") {%rt2 (27, -2) #L1:p};
*   &vo @WIRE(mode="straight") {#RF:n (-7, -8) (20, -8) %rt2};
*   &vo @WIRE(mode="straight") {%rt2 #XA:out};
*   &vin @WIRE(mode="straight") {#VIN:p #RI:p};
*   &n2 @WIRE(mode="straight") {%pout:p %rt3};
*   &n2 @WIRE(mode="straight") {%rt4 (29, -4) (29, -9) %rt3};
*   &n2 @WIRE(mode="straight") {#RL:p %rt4};
*   &n2 @WIRE(mode="straight") {%rt3 #L1:n};
*   &n2 @WIRE(mode="straight") {%rt4 #C1:p};
*   $JUNCTION @ATTACH %rt1;
*   $JUNCTION @ATTACH %rt2;
*   $JUNCTION @ATTACH %rt3;
*   $JUNCTION @ATTACH %rt4
* }
.end
