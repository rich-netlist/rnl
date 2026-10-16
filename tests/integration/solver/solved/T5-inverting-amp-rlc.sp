* T5 Inverting amplifier with an RLC output filter
* @RNL(type="header") { @VERSION("0.2")}
.subckt opamp inp inn out
E1 out 0 inp inn 1e5
* @RNL { # @SET-SYMBOL $OPAMP }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig1:30cf1131fb409563.e17f1522b2b059c0") {
*   %gnd_stub_E1_n = $GND_STUB @ATTACH(gap=2, priority="normal") #E1:n;
*   %ioport_inp = $IOPORT @ATTACH(priority="normal") &inp;
*   %ioport_inn = $IOPORT @ATTACH(priority="normal") &inn;
*   %ioport_out = $IOPORT @ATTACH(priority="normal") &out;
*   #E1 @PLACE(0, 3) => @ORIENT("R0");
*   %gnd_stub_E1_n::g @PLACE(0, -2) => @ORIENT("R0");
*   %ioport_inp @PLACE(-6, 4) => @ORIENT("R0");
*   %ioport_inn @PLACE(-6, 2) => @ORIENT("R0");
*   %ioport_out @PLACE(-6, 6) => @ORIENT("R0");
*   &inp @WIRE(mode="straight") {%ioport_inp:p #E1:cp};
*   &inn @WIRE(mode="straight") {%ioport_inn:p #E1:cn};
*   &out @WIRE(mode="straight") {%ioport_out:p (-5, 6) (-5, 7) (0, 7) #E1:p}
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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig1:e0b9075ac51e7fc2.e17f1522b2b059c0") {
*   %gnd_stub_XA_inp = $GND_STUB @ATTACH(gap=2, priority="normal") #XA:inp;
*   %gnd_stub_C1_n = $GND_STUB @ATTACH(gap=2, priority="normal") #C1:n;
*   %gnd_stub_RL_n = $GND_STUB @ATTACH(gap=2, priority="normal") #RL:n;
*   %gnd_stub_VIN_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VIN:n;
*   %rt1 = (23, 6);
*   %rt2 = (46, 6);
*   %rt3 = (60, 6);
*   %rt4 = (70, 7);
*   #XA @PLACE(28, 5) => @ORIENT("R0");
*   #RI @PLACE(14, 6) => @ORIENT("R90");
*   #RF @PLACE(42, 6) => @ORIENT("R90");
*   #L1 @PLACE(56, 6) => @ORIENT("R90");
*   #C1 @PLACE(84, 3) => @ORIENT("R0");
*   #RL @PLACE(70, 3) => @ORIENT("MY");
*   #VIN @PLACE(0, 3) => @ORIENT("R0");
*   %pout @PLACE(61, 6) => @ORIENT("MY");
*   %gnd_stub_XA_inp::g @PLACE(24, 2) => @ORIENT("R0");
*   %gnd_stub_C1_n::g @PLACE(84, -1) => @ORIENT("R0");
*   %gnd_stub_RL_n::g @PLACE(70, -2) => @ORIENT("R0");
*   %gnd_stub_VIN_n::g @PLACE(0, -2) => @ORIENT("R0");
*   &n1 @WIRE(mode="straight") {#RI:n %rt1};
*   &n1 @WIRE(mode="straight") {%rt1 #XA:inn};
*   &n1 @WIRE(mode="straight") {#RF:p (33, 6) (33, 9) (23, 9) %rt1};
*   &vo @WIRE(mode="straight") {#RF:n %rt2};
*   &vo @WIRE(mode="straight") {%rt2 #L1:p};
*   &vo @WIRE(mode="straight") {#XA:out (38, 5) (38, 4) (46, 4) %rt2};
*   &vin @WIRE(mode="straight") {#VIN:p (0, 7) (10, 7) (10, 6) #RI:p};
*   &n2 @WIRE(mode="straight") {#L1:n %rt3};
*   &n2 @WIRE(mode="straight") {%rt3 %pout:p};
*   &n2 @WIRE(mode="straight") {#RL:p %rt4};
*   &n2 @WIRE(mode="straight") {%rt4 (70, 8) (60, 8) %rt3};
*   &n2 @WIRE(mode="straight") {#C1:p (84, 7) %rt4};
*   $JUNCTION @ATTACH %rt1;
*   $JUNCTION @ATTACH %rt2;
*   $JUNCTION @ATTACH %rt3;
*   $JUNCTION @ATTACH %rt4
* }
.end
