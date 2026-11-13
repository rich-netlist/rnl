* Current mirror bank: sets, selections and generation rules
*! Sets: names, nesting flattened, ADD-TO / REMOVE-FROM, UNION, used before defined (order does not matter); selections: by model, SELECT-TERMINAL(terminal=) in a pipe,
*! current=, global=, SELECT-WIRE; generating: an ATTACH of a current arrow to each terminal selected by CURRENT, net names added to unconnected terminals (the standard package's $LABEL_TERM_SET), a rule changing a label's content by key;
*! M3's ground is drawn on its source pin: with a usage as the scope, only the members of this kind are taken, the standard package can select that terminal, and adds no ground to it (10.1).
.global vdd!
.model nch nmos level=1
M0 bias bias 0 0 nch w=2u l=1u
M1 o1 bias 0 0 nch w=2u l=1u
M2 o2 bias 0 0 nch w=4u l=1u
M3 o3 bias 0 0 nch w=8u l=1u
R1 vdd! o1 10k
R2 vdd! o2 5k
R3 vdd! o3 2.5k
Iref vdd! bias 10u
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-basic", version="0.4"); @USE-PACKAGE(name="rnl-std-fill", version="0.4"); @USE-PACKAGE(name="rnl-std-semiconductors", version="0.4"); @VERSION("0.4")}
* @RNL {
*   # @SELECT-INSTANCE(model="nch", recursive=true) => @SET-SYMBOL $NMOS;
*   # @SELECT-NET(global=true, name="vdd!", recursive=true) => @ADD-TO $SUPPLY_NET_SET;
*
*   %late @SET-STYLE $signal;
*   %outs = {#M1 #M2 #M3};
*   %all = {#M0 %outs};
*   %wide = {};
*   # @SELECT-INSTANCE(model="nch", recursive=true) => @ADD-TO %wide;
*   #M0 @REMOVE-FROM %wide;
*   %wide @TEXT("×${w}", key="w", dx=1.2, dy=-2.3, size=0.8);
*   %u = {%outs {#R1 #R2 #R3}} @UNION;
* %u @SELECT-TERMINAL(terminal=1, recursive=true) => @CURRENT("in");
* # @SELECT-TERMINAL(current="in", name="M*", recursive=true) => $CURRENT_ARROW @ATTACH => @ORIENT("MYR90");
*   # @SELECT-INSTANCE(name="R*", recursive=true) => @TEXT("R=${value}", key="value");
*   $wire_green = @DEFINE-STYLE(linecolor="#2e7d32") {};
*   # @SELECT-WIRE(recursive=true) => @SET-STYLE $wire_green;
*   $LABEL_FILL_SET @TEXT("${net}", key="netname", dx=-0.4, dy=0.5, halign="end", size=0.8);
*   # @SELECT-USAGE $NET_LABEL => @TEXT("${p}", key="netname", dx=-0.4, dy=0.5, halign="end", size=0.8);
*   %late = {#M3};
*   $GROUND @ATTACH #M3:3::net => @PLACE(26, -2) => %m3_ground; #M3:3::net @WIRE(mode="auto") {#M3:3 %m3_ground:p}; $PIN_SCOPE = @DEFINE(roles=[["p", "pin"]]) {}; %pin_scope = $PIN_SCOPE @ATTACH #M3:3; %scope_pins = %pin_scope @SELECT-PIN; #M3:3 @SELECT-PIN => @ASSERT-IN %scope_pins;
*
*   %jd = (0, 3);
*   &bias @WIRE(mode="auto") {#Iref:2 %jd #M0:1};
*   &bias @WIRE(mode="auto") {#M0:2 (4, 0) (4, 3) %jd};
*   $JUNCTION @ATTACH %jd => @PLACE(0, 3);
*   &o1 @WIRE(mode="auto") {#R1:2 #M1:1};
*   &o2 @WIRE(mode="auto") {#R2:2 #M2:1};
*   &o3 @WIRE(mode="auto") {#R3:2 #M3:1}
* }
* 

* @RNL(type="generated", priority="high") {
*   #M0 @PLACE(0, 0) => @ORIENT("MY");
*   #Iref @PLACE(0, 6);
*   #M1 @PLACE(10, 0);
*   #R1 @PLACE(10, 6);
*   #M2 @PLACE(18, 0);
*   #R2 @PLACE(18, 6);
*   #M3 @PLACE(26, 0);
*   #R3 @PLACE(26, 6);
*   %gs_M0_3 = $GROUND @ATTACH #M0:3::net => @PLACE(0, -4); #M0:3::net @WIRE(mode="auto") {#M0:3 %gs_M0_3:p};
*   %gs_M1_3 = $GROUND @ATTACH #M1:3::net => @PLACE(10, -4); #M1:3::net @WIRE(mode="auto") {#M1:3 %gs_M1_3:p};
*   %gs_M2_3 = $GROUND @ATTACH #M2:3::net => @PLACE(18, -4); #M2:3::net @WIRE(mode="auto") {#M2:3 %gs_M2_3:p};
*   %ss_R1_1 = $SUPPLY @ATTACH #R1:1::net => @PLACE(10, 11); #R1:1::net @WIRE(mode="auto") {#R1:1 %ss_R1_1:p};
*   %ss_R2_1 = $SUPPLY @ATTACH #R2:1::net => @PLACE(18, 11); #R2:1::net @WIRE(mode="auto") {#R2:1 %ss_R2_1:p};
*   %ss_R3_1 = $SUPPLY @ATTACH #R3:1::net => @PLACE(26, 11); #R3:1::net @WIRE(mode="auto") {#R3:1 %ss_R3_1:p};
*   %ss_Iref_1 = $SUPPLY @ATTACH #Iref:1::net => @PLACE(0, 11); #Iref:1::net @WIRE(mode="auto") {#Iref:1 %ss_Iref_1:p};
*   %net_label_stub_M1_g = $NET_LABEL_STUB @ATTACH #M1::g;
*   %net_label_stub_M2_g = $NET_LABEL_STUB @ATTACH #M2::g;
*   %net_label_stub_M3_g = $NET_LABEL_STUB @ATTACH #M3::g;
*   %net_label_stub_M1_g @PLACE(0, 0); %net_label_stub_M1_g::l @PLACE(7, 0) => @ORIENT("R180");
*   %net_label_stub_M2_g @PLACE(0, 0); %net_label_stub_M2_g::l @PLACE(15, 0) => @ORIENT("R180");
*   %net_label_stub_M3_g @PLACE(0, 0); %net_label_stub_M3_g::l @PLACE(23, 0) => @ORIENT("R180");
* }
.end
