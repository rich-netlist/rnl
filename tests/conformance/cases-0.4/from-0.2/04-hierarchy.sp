* Two-stage buffer with subcircuits
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-basic", version="0.4"); @USE-PACKAGE(name="rnl-std-semiconductors", version="0.4"); @VERSION("0.4")}
*! Levels: one drawing per subcircuit; a block symbol (`# @SET-SYMBOL` in a subcircuit) as the look of the instances above it, with pins by port name; an .include'd file is a level of its own;
*! scope= writes into a subcircuit from outside; the rules above apply to the levels below, recursive=false only to its own; USE-PACKAGE of an embedded package in a subcircuit changes how resistors are drawn;
*! ports filled in ($PORT_FILL_SET) and taken first; a generated port can only be placed through a selection and cannot be wired (findings).
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
*   %ports = # @SELECT-NET(port=true); {&in &vdd &vss} @REMOVE-FROM %ports;
*   $IO_PORT @ATTACH %ports;
*   %pin = $IO_PORT @ATTACH &in;
*   &in @WIRE(mode="auto") {%pin:p #M1:2};
*   %jo = (0, 3);
*   &out @WIRE(mode="auto") {#R1:2 %jo #M1:1};
*   &out @WIRE(mode="auto") {%jo (4, 3)};
*   $JUNCTION @ATTACH %jo => @PLACE(0, 3)
* }
* 

* @RNL(type="generated", priority="high") {
*   #M1 @PLACE(0, 0);
*   #R1 @PLACE(0, 7);
*   %pin @PLACE(-5, 0);
*   &out @SELECT-ANCESTOR-USAGE $IO_PORT => @PLACE(4, 3) => @ORIENT("MY");
*   %gs_M1_3 = $GROUND @ATTACH #M1:3::net => @PLACE(0, -4); #M1:3::net @WIRE(mode="auto") {#M1:3 %gs_M1_3:p};
*   %ss_R1_1 = $SUPPLY @ATTACH #R1:1::net => @PLACE(0, 12); #R1:1::net @WIRE(mode="auto") {#R1:1 %ss_R1_1:p};
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

* @RNL(type="generated", priority="high") {
*   #X1 @PLACE(0, 0);
*   #X2 @PLACE(14, 0);
*   #X3 @PLACE(24, 0);
*   #Vin @PLACE(-8, -3);
*   #Vdd @PLACE(-14, -3);
*   %gs_X1_4 = $GROUND @ATTACH #X1:4::net => @PLACE(0, -5); #X1:4::net @WIRE(mode="auto") {#X1:4 %gs_X1_4:p};
*   %gs_X2_4 = $GROUND @ATTACH #X2:4::net => @PLACE(14, -5); #X2:4::net @WIRE(mode="auto") {#X2:4 %gs_X2_4:p};
*   %gs_X3_3 = $GROUND @ATTACH #X3:3::net => @PLACE(28, -2); #X3:3::net @WIRE(mode="auto") {#X3:3 %gs_X3_3:p};
*   %gs_Vdd_2 = $GROUND @ATTACH #Vdd:2::net => @PLACE(-14, -8); #Vdd:2::net @WIRE(mode="auto") {#Vdd:2 %gs_Vdd_2:p};
*   %gs_Vin_2 = $GROUND @ATTACH #Vin:2::net => @PLACE(-8, -8); #Vin:2::net @WIRE(mode="auto") {#Vin:2 %gs_Vin_2:p};
*   %ss_X1_3 = $SUPPLY @ATTACH #X1:3::net => @PLACE(0, 5); #X1:3::net @WIRE(mode="auto") {#X1:3 %ss_X1_3:p};
*   %ss_X2_3 = $SUPPLY @ATTACH #X2:3::net => @PLACE(14, 5); #X2:3::net @WIRE(mode="auto") {#X2:3 %ss_X2_3:p};
*   %ss_Vdd_1 = $SUPPLY @ATTACH #Vdd:1::net => @PLACE(-14, 2); #Vdd:1::net @WIRE(mode="auto") {#Vdd:1 %ss_Vdd_1:p};
* }
.end
