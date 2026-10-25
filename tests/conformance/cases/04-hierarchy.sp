* Two-stage buffer with subcircuits
* @RNL(type="header") { @VERSION("0.2")}
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
*     # @SELECT-INSTANCE(name="R*") => @SET-SYMBOL $RES_IEC
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
*   {&vdd &vss} @REMOVE-FROM $PORT_FILL_SET;
*   %pin = $IOPORT @ATTACH &in;
*   &in @WIRE {%pin:p #M1:2};
*   %jo = (0, 2.5);
*   &out @WIRE {#R1:2 %jo #M1:1};
*   &out @WIRE {%jo (4, 2.5)};
*   $JUNCTION @ATTACH %jo
* }
* @RNL(type="generated", priority="high") {
*   #M1 @PLACE(0, 0); #R1 @PLACE(0, 6); %pin @PLACE(-5, 0);
*   &out @SELECT-ANCESTOR-USAGE $IOPORT => @PLACE(4, 2.5) => @ORIENT("MY");
*   %gs_M1_3 = $GND_STUB @ATTACH #M1:3; %gs_M1_3::g @PLACE(0, -4);
*   %ss_R1_1 = $SUPPLY_STUB @ATTACH #R1:1; %ss_R1_1::s @PLACE(0, 11)
* }
.ends
X1 a b vdd 0 amp
X2 b c vdd 0 amp
X3 c d 0 rdiv
Vdd vdd 0 1.8
Vin a 0 0.9
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   # @SELECT-INSTANCE(name="R*") => @SET-STYLE $grey;
*   # @SELECT-INSTANCE(name="V*", recursive=false) => @SET-STYLE $signal;
*   &a @WIRE {#Vin:1 #X1:in};
*   &b @WIRE {#X1:out #X2:in};
*   &c @WIRE(mode="vertical-horizontal") {#X2:out #X3:top}
* }
* @RNL(scope="amp") {
*   #R1 @TEXT("R_{L}", key="name");
*   #M1 @SET-STYLE $signal
* }
* @RNL(type="generated", priority="high") {
*   #X1 @PLACE(0, 0); #X2 @PLACE(14, 0); #X3 @PLACE(24, 0);
*   #Vin @PLACE(-8, -3); #Vdd @PLACE(-14, -3);
*   %gs_X1_4 = $GND_STUB @ATTACH #X1:4; %gs_X1_4::g @PLACE(0, -5);
*   %gs_X2_4 = $GND_STUB @ATTACH #X2:4; %gs_X2_4::g @PLACE(14, -5);
*   %gs_X3_3 = $GND_STUB @ATTACH #X3:3; %gs_X3_3::g @PLACE(27, -1);
*   %gs_Vdd_2 = $GND_STUB @ATTACH #Vdd:2; %gs_Vdd_2::g @PLACE(-14, -8);
*   %gs_Vin_2 = $GND_STUB @ATTACH #Vin:2; %gs_Vin_2::g @PLACE(-8, -8);
*   %ss_X1_3 = $SUPPLY_STUB @ATTACH #X1:3; %ss_X1_3::s @PLACE(0, 5);
*   %ss_X2_3 = $SUPPLY_STUB @ATTACH #X2:3; %ss_X2_3::s @PLACE(14, 5);
*   %ss_Vdd_1 = $SUPPLY_STUB @ATTACH #Vdd:1; %ss_Vdd_1::s @PLACE(-14, 2)
* }
.end
