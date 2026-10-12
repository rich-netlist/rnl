* Common-source amplifier with PMOS load
*! 模型类型选符号，直接的 SET-SYMBOL 盖过规则（三端符号）；&vdd 加进 $SUPPLY_NET_SET，电源、地由标准包补上；M1 源极的地贴在引脚上，
*! 用一根两端都是端子、长度为 0 的导线接上（13.1），标准包就不再补它；CURRENT 与画在点上的电流箭头；
*! 按键覆盖符号里的标签（只改内容 / 只改位置）；${w} 属性引用；富文本；导线直接指派样式。
.model nch nmos level=1
.model pch pmos level=1
M1 out in 0 0 nch w=2u l=0.18u
M2 out bias vdd vdd pch w=4u l=0.18u
Vdd vdd 0 1.8
Vin in 0 DC 0.6 AC 1
Vb bias 0 1.0
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   Three terminal symbols for this drawing, #M1 @SET-SYMBOL $NMOS; #M2 @SET-SYMBOL $PMOS;
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   #M1:1 @CURRENT("in"); #M1:3 @CURRENT("out");
*
*   %o = (0, 4);
*   &out @WIRE {#M2:1 %o #M1:1};
*   $JUNCTION @ATTACH %o;
*   %vo = (6, 4);
*   &out @WIRE {%o %vo} => @SET-STYLE $signal => %wo;
*   %vo @TEXT("V_{out}", dx=0.4);
*   &in @WIRE(mode="vertical-horizontal") {#Vin:1 #M1:2};
*   &bias @WIRE {#Vb:1 #M2:2};
*   The ground sits on the source pin, $GND @ATTACH &0 => @PLACE(0, -2) => %g1;
*   joined by a wire of length 0, &0 @WIRE(mode="straight") {#M1:3 %g1:p};
*
*   Drain current drawn on the wire, $CURRENT_ARROW @ATTACH (0, 3) => @ORIENT("R270") => @TEXT("I_{D}", dx=1.1);
*   #M1 @TEXT("W/L=${w}/${l}", key="model");
*   #M2 @TEXT("M_{2}（负载）", key="name");
*   #M1::name @PLACE(-3, 1.8);
*   (-15, 13) @TEXT("A_{v} = −g_{m1}(r_{o1} \parallel r_{o2})")
* }
* @RNL(type="generated", priority="high") {
*   #M1 @PLACE(0, 0); #M2 @PLACE(0, 8);
*   #Vin @PLACE(-8, -4); #Vb @PLACE(-13, 5); #Vdd @PLACE(12, 7);
*   %gs_Vdd_2 = $GND_STUB @ATTACH #Vdd:2; %gs_Vdd_2::g @PLACE(12, 2);
*   %gs_Vin_2 = $GND_STUB @ATTACH #Vin:2; %gs_Vin_2::g @PLACE(-8, -9);
*   %gs_Vb_2 = $GND_STUB @ATTACH #Vb:2; %gs_Vb_2::g @PLACE(-13, 0);
*   %ss_M2_3 = $SUPPLY_STUB @ATTACH #M2:3; %ss_M2_3::s @PLACE(0, 12);
*   %ss_Vdd_1 = $SUPPLY_STUB @ATTACH #Vdd:1; %ss_Vdd_1::s @PLACE(12, 12)
* }
.end
