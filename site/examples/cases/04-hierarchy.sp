* Two-stage buffer with subcircuits
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-basic", version="0.4"); @USE-PACKAGE(name="rnl-std-semiconductors", version="0.4"); @VERSION("0.4")}
*! Levels: one drawing per subcircuit; a block symbol (`# @SET-SYMBOL` in a subcircuit) as the look of the instances above it, with pins by port name; an .include'd file is a level of its own;
*! scope= writes into a subcircuit from outside; the rules above apply to the levels below, recursive=false only to its own; USE-PACKAGE of an embedded package in a subcircuit changes how resistors are drawn;
*! ports filled in ($PORT_FILL_SET) and taken first; a port that the fill makes is named in the record of the solver, which places and wires it.
.include 04-hierarchy-lib.inc
.model nch nmos level=1
.subckt amp in out vdd vss
M1 out in vss vss nch w=1u l=0.18u
R1 vdd out 10k
* @RNL {
*   @USE-PACKAGE(name="iec-resistor") {
*     $RES_IEC = @DEFINE-SYMBOL {
*       (0, 3) @PIN(terminal=1, direction="up"); (0, -3) @PIN(terminal=2, direction="down");
*       (0, 0) @RECTANGLE(width=1.2, height=3.6);
*       @LINE {(0, 3) (0, 1.8)}; @LINE {(0, -1.8) (0, -3)};
*       ? @TEXT("${name}", key="name") => @PLACE(1.1, 0.25);
*       ? @TEXT("${value}", key="value") => @PLACE(1.1, -0.95)
*     };
*     # @SELECT-INSTANCE(name="R*", recursive=true) => @SET-SYMBOL $RES_IEC
*   };
*   %amp_block = @DEFINE-SYMBOL {
*     (-4, 0) @PIN(terminal="in", direction="left"); (4, 0) @PIN(terminal="out", direction="right");
*     (0, 3) @PIN(terminal="vdd", direction="up"); (0, -3) @PIN(terminal="vss", direction="down");
*     @LINE {(-2, 2) (2, 0) (-2, -2) (-2, 2)};
*     @LINE {(-4, 0) (-2, 0)}; @LINE {(2, 0) (4, 0)};
*     @LINE {(0, 3) (0, 1)}; @LINE {(0, -3) (0, -1)};
*     ? @TEXT("${name}", key="name") => @PLACE(-2, 2.25);
*     ? @TEXT("${model}", key="sub") => @PLACE(0.6, -2.55)
*   };
*   # @SET-SYMBOL %amp_block;
*   #M1 @SET-SYMBOL $NMOS;
*   &vdd @ADD-TO $SUPPLY_NET_SET; &vss @ADD-TO $GROUND_NET_SET;
*   {&in &vdd &vss} @REMOVE-FROM $PORT_FILL_SET;
*   %pin = $IO_PORT @ATTACH &in;
*   &in @WIRE(mode="auto") {%pin:p #M1:2};
*   %branch = {#R1 #M1} @AS $BRANCH;
*   &out @ROUTE-NET
* }
* 

* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:c9a37d5b364797b9.2e02c1e4aefd5467", numbering="2b95cd6ab6bfb99c") {
*   %ground_stub_M1_s = $GROUND_STUB @ATTACH(gap=3) #M1::s;
*   %supply_stub_R1_p = $SUPPLY_STUB @ATTACH(gap=3) #R1:p;
*   %io_port_out = $IO_PORT @ATTACH &out;
*   %rt1 = (6, 4);
*   #M1 @PLACE(6, 0) => @ORIENT("R0");
*   #R1 @PLACE(6, 8) => @ORIENT("R0");
*   %pin @PLACE(0, 0) => @ORIENT("R0");
*   %branch @PLACE(6, 0);
*   %#2 @PLACE(0, 0);
*   %ground_stub_M1_s @PLACE(0, 0);
*   %ground_stub_M1_s::g @PLACE(6, -4) => @ORIENT("R0");
*   %supply_stub_R1_p @PLACE(0, 0);
*   %supply_stub_R1_p::s @PLACE(6, 13) => @ORIENT("R0");
*   %io_port_out @PLACE(0, 4) => @ORIENT("R0");
*   &out @WIRE(mode="straight") {#M1::d %rt1};
*   &out @WIRE(mode="straight") {%rt1 #R1:n};
*   &out @WIRE(mode="straight") {%io_port_out::pin %rt1};
*   $JUNCTION @ATTACH %rt1 => @PLACE(6, 4)
* }
.ends
X1 a b vdd 0 amp
X2 b c vdd 0 amp
X3 c d 0 rdiv
Vdd vdd 0 1.8
Vin a 0 0.9
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   # @SELECT-INSTANCE(name="R*", recursive=true) => @SET-STYLE $grey;
*   # @SELECT-INSTANCE(name="V*", recursive=false) => @SET-STYLE $signal;
*   &a @WIRE(mode="auto") {#Vin:1 #X1:in};
*   &b @WIRE(mode="auto") {#X1:out #X2:in};
*   &c @WIRE(mode="auto") {#X2:out #X3:top}
* }
* @RNL(scope="amp") {
*   #R1 @TEXT("R_{L}", key="name");
*   #M1 @SET-STYLE $signal
* }
* 

* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:fadac595f316fc79.c8fb7aa80b2c7852", numbering="611c1274b5dae8ee") {
*   %ground_stub_X1_vss = $GROUND_STUB @ATTACH(gap=3) #X1:vss;
*   %ground_stub_X2_vss = $GROUND_STUB @ATTACH(gap=3) #X2:vss;
*   %ground_stub_X3_bot = $GROUND_STUB @ATTACH(gap=3) #X3:bot;
*   %ground_stub_Vdd_n = $GROUND_STUB @ATTACH(gap=3) #Vdd::n;
*   %ground_stub_Vin_n = $GROUND_STUB @ATTACH(gap=3) #Vin::n;
*   %supply_stub_X1_vdd = $SUPPLY_STUB @ATTACH(gap=3) #X1:vdd;
*   %supply_stub_X2_vdd = $SUPPLY_STUB @ATTACH(gap=3) #X2:vdd;
*   %supply_stub_Vdd_p = $SUPPLY_STUB @ATTACH(gap=3) #Vdd::p;
*   %net_label_stub_X3_mid = $NET_LABEL_STUB @ATTACH #X3:mid;
*   #X1 @PLACE(0, -3) => @ORIENT("R0");
*   #X2 @PLACE(10, -3) => @ORIENT("R0");
*   #X3 @PLACE(19, -4) => @ORIENT("R0");
*   #Vdd @PLACE(19, 9) => @ORIENT("R0");
*   #Vin @PLACE(-8, -3) => @ORIENT("R270");
*   %ground_stub_X1_vss @PLACE(0, 0);
*   %ground_stub_X1_vss::g @PLACE(0, -8) => @ORIENT("R0");
*   %ground_stub_X2_vss @PLACE(0, 0);
*   %ground_stub_X2_vss::g @PLACE(10, -8) => @ORIENT("R0");
*   %ground_stub_X3_bot @PLACE(0, 0);
*   %ground_stub_X3_bot::g @PLACE(23, -6) => @ORIENT("R0");
*   %ground_stub_Vdd_n @PLACE(0, 0);
*   %ground_stub_Vdd_n::g @PLACE(19, 5) => @ORIENT("R0");
*   %ground_stub_Vin_n @PLACE(0, 0);
*   %ground_stub_Vin_n::g @PLACE(-12, -5) => @ORIENT("R0");
*   %supply_stub_X1_vdd @PLACE(0, 0);
*   %supply_stub_X1_vdd::s @PLACE(0, 2) => @ORIENT("R0");
*   %supply_stub_X2_vdd @PLACE(0, 0);
*   %supply_stub_X2_vdd::s @PLACE(10, 2) => @ORIENT("R0");
*   %supply_stub_Vdd_p @PLACE(0, 0);
*   %supply_stub_Vdd_p::s @PLACE(19, 13) => @ORIENT("R0");
*   %net_label_stub_X3_mid @PLACE(0, 0);
*   %net_label_stub_X3_mid::l @PLACE(17, -5) => @ORIENT("R0");
*   %#6 @WIRE(mode="straight") {#X3:bot (23, -4) %ground_stub_X3_bot::g::pin};
*   %#8 @WIRE(mode="straight") {#Vin::n (-12, -3) %ground_stub_Vin_n::g::pin}
* }
.end
