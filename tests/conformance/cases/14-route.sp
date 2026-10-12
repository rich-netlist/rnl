* Common-emitter amplifier, wiring left to the solver
*! ROUTE（13.7，PR5）：只说"这些网络画成导线"，不写分叉点和汇合点，弱模型也写得出；求解器会写回具体的导线和圆点，原型不布线，画虚线飞线。
*! 标准包的 $WIRE_NET_SET（全部网络去掉地、电源）配一条规则；out 先手写了一段，ROUTE 只补其余的；地、电源的 stub 是"求解器"写回的。
.model npn1 npn
Q1 c b e npn1
R1 vcc b 47k
R2 b 0 10k
RC vcc c 4.7k
RE e 0 1k
C1 in b 1u
C2 c out 1u
RL out 0 10k
Vcc vcc 0 12
Vin in 0 AC 1
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   &vcc @ADD-TO $SUPPLY_NET_SET;
*   $WIRE_NET_SET @ROUTE;
*   &out @WIRE(mode="straight") {#C2:2 (16, 3)};
*   (-5, 0) @TEXT("v_{in}", dx=-0.5, dy=0.8, halign="end");
*   (-2, 14) @TEXT("弱模型的写法：器件位置加 ROUTE，导线、分叉和圆点都交给求解器")
* }
* @RNL(type="generated", priority="high") {
*   #Q1 @PLACE(8, 0);
*   #RC @PLACE(8, 6); #RE @PLACE(8, -6);
*   #R1 @PLACE(2, 6); #R2 @PLACE(2, -5);
*   #C1 @PLACE(-3, 0) => @ORIENT("R90"); #C2 @PLACE(12, 3) => @ORIENT("R90");
*   #RL @PLACE(18, 0); #Vin @PLACE(-9, -3); #Vcc @PLACE(24, 6);
*   %gs_R2_2 = $GND_STUB @ATTACH #R2:2; %gs_R2_2::g @PLACE(2, -10);
*   %gs_RE_2 = $GND_STUB @ATTACH #RE:2; %gs_RE_2::g @PLACE(8, -11);
*   %gs_RL_2 = $GND_STUB @ATTACH #RL:2; %gs_RL_2::g @PLACE(18, -5);
*   %gs_Vcc_2 = $GND_STUB @ATTACH #Vcc:2; %gs_Vcc_2::g @PLACE(24, 1);
*   %gs_Vin_2 = $GND_STUB @ATTACH #Vin:2; %gs_Vin_2::g @PLACE(-9, -8);
*   %ss_R1_1 = $SUPPLY_STUB @ATTACH #R1:1; %ss_R1_1::s @PLACE(2, 11);
*   %ss_RC_1 = $SUPPLY_STUB @ATTACH #RC:1; %ss_RC_1::s @PLACE(8, 11);
*   %ss_Vcc_1 = $SUPPLY_STUB @ATTACH #Vcc:1; %ss_Vcc_1::s @PLACE(24, 11)
* }
.end
