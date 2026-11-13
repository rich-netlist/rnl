* Common-source amplifier with PMOS load
.model nch nmos level=1
.model pch pmos level=1
M1 out in 0 0 nch w=2u l=0.18u
M2 out bias vdd vdd pch w=4u l=0.18u
Vdd vdd 0 1.8
Vin in 0 0.6
Vb bias 0 1.0
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-basic", version="0.4"); @USE-PACKAGE(name="rnl-std-semiconductors", version="0.4"); @VERSION("0.4")}
* @RNL {
*   Three terminal symbols for this drawing, #M1 @SET-SYMBOL $NMOS; #M2 @SET-SYMBOL $PMOS;
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   #M1:d @CURRENT("in"); #M1:s @CURRENT("out");
*
*   %o = (0, 4);
*   &out @WIRE(mode="auto") {#M2:d %o #M1:d};
*   $JUNCTION @ATTACH %o => @PLACE(0, 4);
*   %vo = (6, 4);
*   &out @WIRE(mode="auto") {%o %vo} => @SET-STYLE $signal => %wo;
*   %vo @TEXT("V_{out}", dx=0.4);
*   &in @WIRE(mode="auto") {#Vin:p (-8, 0) #M1:g};
*   &bias @WIRE(mode="auto") {#Vb:p #M2:g};
*   The ground sits on the source pin, $GROUND @ATTACH &0 => @PLACE(0, -2) => %g1;
*   joined by a wire of length 0, &0 @WIRE(mode="auto") {#M1:s %g1:p};
*
*   Drain current drawn on the wire, $CURRENT_ARROW @ATTACH (0, 2.9) => @PLACE(0, 2.9) => @ORIENT("R270") => @TEXT("I_{D}", dx=1.1);
*   #M1 @TEXT("W/L=${w}/${l}, ${type}", key="model");
*   #M2 @TEXT("M_{2} (load)", key="name");
*   #M1::name @PLACE(-3, 1.8);
*   # @SELECT-INSTANCE(type="pmos", recursive=true) => @SET-STYLE $grey;
*   # @SELECT-TERMINAL(current="in", recursive=true) => @SELECT-PIN(recursive=true) => @TEXT("in", key="dir");
*   (-15, 13) @TEXT("A_{v} = -g_{m1}(r_{o1} \parallel r_{o2})")
* }
* 
* @RNL(type="generated", priority="high") {
*   #M1 @PLACE(0, 0); #M2 @PLACE(0, 8); #Vin @PLACE(-8, -4); #Vb @PLACE(-13, 5); #Vdd @PLACE(12, 7);
*   %gs_Vdd_2 = $GROUND @ATTACH #Vdd:2::net => @PLACE(12, 2); #Vdd:2::net @WIRE(mode="auto") {#Vdd:2 %gs_Vdd_2:p};
*   %gs_Vin_2 = $GROUND @ATTACH #Vin:2::net => @PLACE(-8, -9); #Vin:2::net @WIRE(mode="auto") {#Vin:2 %gs_Vin_2:p};
*   %gs_Vb_2 = $GROUND @ATTACH #Vb:2::net => @PLACE(-13, 0); #Vb:2::net @WIRE(mode="auto") {#Vb:2 %gs_Vb_2:p};
*   %ss_M2_3 = $SUPPLY @ATTACH #M2:3::net => @PLACE(0, 12); #M2:3::net @WIRE(mode="auto") {#M2:3 %ss_M2_3:p};
*   %ss_Vdd_1 = $SUPPLY @ATTACH #Vdd:1::net => @PLACE(12, 12); #Vdd:1::net @WIRE(mode="auto") {#Vdd:1 %ss_Vdd_1:p};
* }

.end
