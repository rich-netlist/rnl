* Common-source amplifier with PMOS load
*! The model type chooses the symbol, and a direct SET-SYMBOL overrides the rules (a three-terminal symbol); &vdd added to $SUPPLY_NET_SET, the supply and ground added by the standard package; the ground at M1's source sits on the pin,
*! connected by a wire of length 0 whose two ends are terminals (13.1), so the standard package no longer adds one; CURRENT, and a current arrow drawn at a point;
*! the labels of a symbol overridden by key (only the content / only the position); the property reference ${w}; rich text; a style given to a wire directly.
.model nch nmos level=1
.model pch pmos level=1
M1 out in 0 0 nch w=2u l=0.18u
M2 out bias vdd vdd pch w=4u l=0.18u
Vdd vdd 0 1.8
Vin in 0 DC 0.6 AC 1
Vb bias 0 1.0
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-basic", version="0.4"); @USE-PACKAGE(name="rnl-std-semiconductors", version="0.4"); @VERSION("0.4")}
* @RNL {
*   Three terminal symbols for this drawing, #M1 @SET-SYMBOL $NMOS; #M2 @SET-SYMBOL $PMOS;
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   #M1:1 @CURRENT("in"); #M1:3 @CURRENT("out");
*
*   %o = (0, 4);
*   &out @WIRE(mode="auto") {#M2:1 %o #M1:1};
*   $JUNCTION @ATTACH %o;
*   %vo = (6, 4);
*   &out @WIRE(mode="auto") {%o %vo} => @SET-STYLE $signal => %wo;
*   %vo @TEXT("V_{out}", dx=0.4);
*   %input_corner = (-8, 0);
*   &in @WIRE(mode="auto") {#Vin:1 %input_corner #M1:2};
*   &bias @WIRE(mode="auto") {#Vb:1 #M2:2};
*   The ground sits on the source pin, $GROUND @ATTACH &0 => @PLACE(0, -2) => %g1;
*   joined by a wire of length 0, &0 @WIRE(mode="auto") {#M1:3 %g1::pin};
*
*   Drain current drawn on the wire, $CURRENT_ARROW @ATTACH (0, 2.9) => @ORIENT("R270") => @TEXT("I_{D}", dx=1.1);
*   #M1 @TEXT("W/L=${w}/${l}", key="model");
*   #M2 @TEXT("M_{2}（负载）", key="name");
*   #M1::name @PLACE(-3, 1.8);
*   (-15, 13) @TEXT("A_{v} = −g_{m1}(r_{o1} \parallel r_{o2})")
* }
* 








* Fixed device positions retain the reviewed layout; the 0.4 solver record fills the remaining objects.
* @RNL(priority="high") {
*   #M1 @PLACE(0, 0) => @ORIENT("R0");
*   #M2 @PLACE(0, 8) => @ORIENT("R0");
*   #Vdd @PLACE(12, 7) => @ORIENT("R0");
*   #Vin @PLACE(-8, -4) => @ORIENT("R0");
*   #Vb @PLACE(-13, 5) => @ORIENT("R0");
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:ecb2220828ad190b.ffc36700559c31e7", numbering="5b90ba4d3ac19d33") {
*   %ground_stub_Vdd_n = $GROUND_STUB @ATTACH(gap=1) #Vdd::n;
*   %ground_stub_Vin_n = $GROUND_STUB @ATTACH(gap=1) #Vin::n;
*   %ground_stub_Vb_n = $GROUND_STUB @ATTACH(gap=1) #Vb::n;
*   %supply_stub_M2_s = $SUPPLY_STUB @ATTACH(gap=1) #M2::s;
*   %supply_stub_Vdd_p = $SUPPLY_STUB @ATTACH(gap=1) #Vdd::p;
*   #M1 @PLACE(0, 0) => @ORIENT("R0");
*   #M2 @PLACE(0, 8) => @ORIENT("R0");
*   #Vdd @PLACE(12, 7) => @ORIENT("R0");
*   #Vin @PLACE(-8, -4) => @ORIENT("R0");
*   #Vb @PLACE(-13, 5) => @ORIENT("R0");
*   %#2 @PLACE(0, 4) => @ORIENT("R0");
*   %g1 @PLACE(0, -2) => @ORIENT("R0");
*   %#7 @PLACE(0, 2.9) => @ORIENT("R270");
*   %ground_stub_Vdd_n @PLACE(0, 0);
*   %ground_stub_Vdd_n::g @PLACE(12, 4) => @ORIENT("R0");
*   %ground_stub_Vin_n @PLACE(0, 0);
*   %ground_stub_Vin_n::g @PLACE(-8, -7) => @ORIENT("R0");
*   %ground_stub_Vb_n @PLACE(0, 0);
*   %ground_stub_Vb_n::g @PLACE(-13, 2) => @ORIENT("R0");
*   %supply_stub_M2_s @PLACE(0, 0);
*   %supply_stub_M2_s::s @PLACE(0, 11) => @ORIENT("R0");
*   %supply_stub_Vdd_p @PLACE(0, 0);
*   %supply_stub_Vdd_p::s @PLACE(12, 10) => @ORIENT("R0");
*   %o @PLACE(0, 4);
*   %vo @PLACE(6, 4);
*   %#8 @PLACE(0, 2.9);
*   %#10 @PLACE(-15, 13);
*   %#3 @PLACE(y=4);
*   %#9 @PLACE(y=4.2);
*   %#11 @PLACE(-15, 13);
*   #M1::name @PLACE(-3, 1.8)
* }
.end
