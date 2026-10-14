* Current mirror bank: sets, selections and generation rules
*! 集合：命名、嵌套打散、ADD-TO / REMOVE-FROM、UNION、先用后定义（不分先后）；选择：按模型、管道里的 SELECT-TERMINAL(terminal=)、
*! current=、global=、SELECT-WIRE；生成：按 CURRENT 选端子逐个 ATTACH 电流箭头、给没接线的端子补网名（标准包的 $LABEL_TERM_SET）、规则按键改标签内容；
*! M3 的地画在源极引脚上：以用例为范围只取本种类的成员，标准包选得出这个端子，不再给它补地（10.1）。
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
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   # @SELECT-INSTANCE(model="nch") => @SET-SYMBOL $NMOS;
*   # @SELECT-NET(global=true, name="vdd!") => @ADD-TO $SUPPLY_NET_SET;
*
*   %late @SET-STYLE $signal;
*   %outs = {#M1 #M2 #M3};
*   %all = {#M0 %outs};
*   %wide = {};
*   # @SELECT-INSTANCE(model="nch") => @ADD-TO %wide;
*   #M0 @REMOVE-FROM %wide;
*   %wide @TEXT("×${w}", key="w", dx=1.2, dy=-2.3, size=0.8);
*   %u = {%outs {#R1 #R2 #R3}} @UNION;
*   %u @SELECT-TERMINAL(terminal=1) => @CURRENT("in");
*   # @SELECT-TERMINAL(current="in", name="M*") => $CURRENT_ARROW @ATTACH => @ORIENT("MYR90");
*   # @SELECT-INSTANCE(name="R*") => @TEXT("R=${value}", key="value");
*   $wire_green = @DEFINE-STYLE(linecolor="#2e7d32") {};
*   # @SELECT-WIRE => @SET-STYLE $wire_green;
*   $LABEL_TERM_SET @TEXT("${net}", key="netname", dx=-0.4, dy=0.5, halign="end", size=0.8);
*   %late = {#M3};
*   $GND @ATTACH #M3:3;
*
*   %jd = (0, 2.5);
*   &bias @WIRE {#Iref:2 %jd #M0:1};
*   &bias @WIRE(mode="vertical-horizontal") {#M0:2 %jd};
*   $JUNCTION @ATTACH %jd;
*   &o1 @WIRE {#R1:2 #M1:1};
*   &o2 @WIRE {#R2:2 #M2:1};
*   &o3 @WIRE {#R3:2 #M3:1}
* }
* @RNL(type="generated", priority="high") {
*   #M0 @PLACE(0, 0) => @ORIENT("MY"); #Iref @PLACE(0, 6);
*   #M1 @PLACE(10, 0); #R1 @PLACE(10, 6);
*   #M2 @PLACE(18, 0); #R2 @PLACE(18, 6);
*   #M3 @PLACE(26, 0); #R3 @PLACE(26, 6);
*   %gs_M0_3 = $GND_STUB @ATTACH #M0:3; %gs_M0_3::g @PLACE(0, -4);
*   %gs_M1_3 = $GND_STUB @ATTACH #M1:3; %gs_M1_3::g @PLACE(10, -4);
*   %gs_M2_3 = $GND_STUB @ATTACH #M2:3; %gs_M2_3::g @PLACE(18, -4);
*   %ss_R1_1 = $SUPPLY_STUB @ATTACH #R1:1; %ss_R1_1::s @PLACE(10, 11);
*   %ss_R2_1 = $SUPPLY_STUB @ATTACH #R2:1; %ss_R2_1::s @PLACE(18, 11);
*   %ss_R3_1 = $SUPPLY_STUB @ATTACH #R3:1; %ss_R3_1::s @PLACE(26, 11);
*   %ss_Iref_1 = $SUPPLY_STUB @ATTACH #Iref:1; %ss_Iref_1::s @PLACE(0, 11)
* }
.end
