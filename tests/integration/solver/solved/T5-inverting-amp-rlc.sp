* T5 Inverting amplifier with an RLC output filter
* @RNL(type="header") { @VERSION("0.2")}
.subckt opamp inp inn out
E1 out 0 inp inn 1e5
* @RNL { # @SET-SYMBOL $OPAMP }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig1:91f5bdd803e84b2a.1f3b310f8bee492a") {
*   %gnd_stub_E1_n = $GND_STUB @ATTACH(gap=2, priority="normal") #E1:n;
*   %ioport_inp = $IOPORT @ATTACH(priority="normal") &inp;
*   %ioport_inn = $IOPORT @ATTACH(priority="normal") &inn;
*   %ioport_out = $IOPORT @ATTACH(priority="normal") &out;
*   #E1 @PLACE(0, 3) => @ORIENT("R0");
*   %gnd_stub_E1_n::g @PLACE(0, -2) => @ORIENT("R0");
*   %ioport_inp @PLACE(-6, 4) => @ORIENT("R0");
*   %ioport_inn @PLACE(-6, 2) => @ORIENT("R0");
*   %ioport_out @PLACE(-6, 8) => @ORIENT("R0");
*   &inp @WIRE(mode="straight") {%ioport_inp:p #E1:cp};
*   &inn @WIRE(mode="straight") {%ioport_inn:p #E1:cn};
*   &out @WIRE(mode="straight") {%ioport_out:p (0, 8) #E1:p}
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
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig1:b12c19712eb68687.1f3b310f8bee492a") {
*   %gnd_stub_XA_inp = $GND_STUB @ATTACH(gap=2, priority="normal") #XA:inp;
*   %gnd_stub_C1_n = $GND_STUB @ATTACH(gap=2, priority="normal") #C1:n;
*   %gnd_stub_RL_n = $GND_STUB @ATTACH(gap=2, priority="normal") #RL:n;
*   %gnd_stub_VIN_n = $GND_STUB @ATTACH(gap=2, priority="normal") #VIN:n;
*   %rt1 = (22, 7);
*   %rt2 = (44, 7);
*   %rt3 = (56, 7);
*   %rt4 = (65, 7);
*   #XA @PLACE(39, 7) => @ORIENT("R0");
*   #RI @PLACE(13, 7) => @ORIENT("R90");
*   #RF @PLACE(26, 7) => @ORIENT("R90");
*   #L1 @PLACE(52, 7) => @ORIENT("R90");
*   #C1 @PLACE(78, 4) => @ORIENT("R0");
*   #RL @PLACE(65, 3) => @ORIENT("R0");
*   #VIN @PLACE(0, 3) => @ORIENT("R0");
*   %pout @PLACE(57, 7) => @ORIENT("MY");
*   %gnd_stub_XA_inp::g @PLACE(35, 4) => @ORIENT("R0");
*   %gnd_stub_C1_n::g @PLACE(78, 0) => @ORIENT("R0");
*   %gnd_stub_RL_n::g @PLACE(65, -2) => @ORIENT("R0");
*   %gnd_stub_VIN_n::g @PLACE(0, -2) => @ORIENT("R0");
*   &n1 @WIRE(mode="straight") {#RI:n %rt1};
*   &n1 @WIRE(mode="straight") {%rt1 #RF:p};
*   &n1 @WIRE(mode="straight") {#XA:inn (30, 8) (30, 9) (22, 9) %rt1};
*   &vo @WIRE(mode="straight") {#XA:out %rt2};
*   &vo @WIRE(mode="straight") {%rt2 #L1:p};
*   &vo @WIRE(mode="straight") {#RF:n (33, 7) (33, 1) (44, 1) %rt2};
*   &vin @WIRE(mode="straight") {#VIN:p (0, 7) #RI:p};
*   &n2 @WIRE(mode="straight") {#L1:n %rt3};
*   &n2 @WIRE(mode="straight") {%rt3 %pout:p};
*   &n2 @WIRE(mode="straight") {#RL:p %rt4};
*   &n2 @WIRE(mode="straight") {%rt4 (65, 9) (56, 9) %rt3};
*   &n2 @WIRE(mode="straight") {#C1:p (78, 7) %rt4};
*   $JUNCTION @ATTACH %rt1;
*   $JUNCTION @ATTACH %rt2;
*   $JUNCTION @ATTACH %rt3;
*   $JUNCTION @ATTACH %rt4
* }
.end
