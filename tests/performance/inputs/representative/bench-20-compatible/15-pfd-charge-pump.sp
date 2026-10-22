* CC0-1.0 — 15-pfd-charge-pump.sp: Phase-frequency detector (PFD), charge pump and loop filter (Hierarchical #3)
*! 电路说明：锁相环鉴频鉴相器（PFD）+ 电荷泵（Charge Pump）+ 二阶无源环路滤波器（层次化电路 #3），定义了带异步复位端的边沿触发器子电路 dff_rst（内部含 4 个 MOS 管），在顶层调用 2 次（XDFF_UP 检测参考时钟 fref，XDFF_DN 检测反馈时钟 fdiv），配合复位与非门（MN1、MN2、MP1、MP2）、上下拉电荷泵支路（IBIAS、MBN、MBP、MUP_CS、MUP_SW、MDN_SW、MDN_CS）及二阶 RC 环路滤波器（RLF、CLF1、CLF2），共 19 个顶层实例（含子电路展开共 25 个底元件）。
*! 视觉验收标准：
*! 1. 信号流方向：从左到右——左侧为电源与参考/分频时钟源（VDD、VFREF、VFDIV、IBIAS），中间偏左为上下对称的鉴频鉴相器双触发器（XDFF_UP、XDFF_DN）与复位逻辑（MP1/MP2/MN1/MN2），中间偏右为电荷泵偏置与上下拉充放电支路（MBN、MBP、MUP_CS–MDN_CS），最右侧为二阶无源环路滤波器（RLF、CLF1、CLF2）。
*! 2. 对称对与对称轴：鉴频鉴相器的 XDFF_UP 与 XDFF_DN 上下配对；电荷泵上拉支路（MUP_CS、MUP_SW）与下拉支路（MDN_CS、MDN_SW）在垂直支路上对称汇聚于控制电压节点 vctrl。
*! 3. 电源与地方向：电源网 vdd 位于最上方（MP1、MP2、MBP、MUP_CS 源端朝上接电源短线），地网 0 位于最下方（MN2、MBN、MDN_CS 源端及滤波电容 CLF1、CLF2 下端朝下接地短线）。
*! 4. 行与列对齐：电荷泵主充放电支路（MUP_CS -> MUP_SW -> MDN_SW -> MDN_CS）自上而下同列串联；环路滤波支路（RLF–CLF1 与 CLF2）在最右侧并列对齐。
*! 5. 连线与网络标签：电源网 vdd 与地网 0 采用电源/接地短线，其余控制与信号网络（fref、fdiv、up、dn、rst_n、vbp、vbn、vctrl）采用导线（ROUTE）连接。
* @RNL(type="header") { @VERSION("0.3") }

.model nch nmos level=1
.model pch pmos level=1

* Simplified resettable phase-detector latch cell (instantiated twice: XDFF_UP, XDFF_DN)
.subckt dff_rst clk rst_n q vdd vss
M1 q_n clk vdd vdd pch W=4u L=0.5u
M2 q_n rst_n vss vss nch W=4u L=0.5u
M3 q q_n vdd vdd pch W=4u L=0.5u
M4 q q_n vss vss nch W=4u L=0.5u
* @RNL {
*   # @SELECT-INSTANCE(type="nmos") => @SET-SYMBOL $NMOS;
*   # @SELECT-INSTANCE(type="pmos") => @SET-SYMBOL $PMOS;
*   $DFF_SYM = @DEFINE-SYMBOL(box=[-4, -3, 4, 3]) {
*     (-4, 1) @PIN(terminal="clk", direction="left");
*     (-4, -1) @PIN(terminal="rst_n", direction="left");
*     (4, 0) @PIN(terminal="q", direction="right");
*     (0, 3) @PIN(terminal="vdd", direction="up");
*     (0, -3) @PIN(terminal="vss", direction="down");
*     @LINE {(-3, 2) (3, 2) (3, -2) (-3, -2) (-3, 2)};
*     @LINE {(-4, 1) (-3, 1)};
*     @LINE {(-4, -1) (-3, -1)};
*     @LINE {(3, 0) (4, 0)};
*     @LINE {(0, 2) (0, 3)};
*     @LINE {(0, -2) (0, -3)};
*     (-2.1, 1) @TEXT("CLK", halign="start", valign="middle", size=0.65);
*     (-2.1, -1) @TEXT("RST", halign="start", valign="middle", size=0.65);
*     (2.1, 0) @TEXT("Q", halign="end", valign="middle", size=0.65);
*     ? @TEXT("${name}", key="name") => @PLACE(0, 3.4)
*   };
*   # @SET-SYMBOL $DFF_SYM;
*
*   {#M1 #M2} @AS $BRANCH;
*   {#M3 #M4} @AS $BRANCH;
*   {#M1:s #M3:s} @AS $SUPPLY_HIGH;
*   {#M2:s #M4:s} @AS $SUPPLY_LOW;
*   {{#M1 #M2} {#M3 #M4}} @AS $FLOW;
*
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   $WIRE_NET_SET @ROUTE
* }
.ends dff_rst

* Phase-Frequency Detector (two dff_rst instances + CMOS NAND2 reset)
XDFF_UP fref rst_n up vdd 0 dff_rst
XDFF_DN fdiv rst_n dn vdd 0 dff_rst

MP1 rst_n up vdd vdd pch W=4u L=0.5u
MP2 rst_n dn vdd vdd pch W=4u L=0.5u
MN1 rst_n up nand_mid 0 nch W=4u L=0.5u
MN2 nand_mid dn 0 0 nch W=4u L=0.5u

* Charge pump bias and UP/DN switched current sources
IBIAS vdd vbn 20u
MBN vbn vbn 0 0 nch W=8u L=1u
MBP vbp vbn 0 0 nch W=8u L=1u
MBP_LOAD vbp vbp vdd vdd pch W=16u L=1u

MUP_CS cp_up_s vbp vdd vdd pch W=16u L=1u
MUP_SW vctrl up cp_up_s vdd pch W=16u L=0.5u
MDN_SW vctrl dn cp_dn_d 0 nch W=8u L=0.5u
MDN_CS cp_dn_d vbn 0 0 nch W=8u L=1u

* 2nd-order passive loop filter
RLF vctrl lf_mid 10k
CLF1 lf_mid 0 10p
CLF2 vctrl 0 1p

VDD vdd 0 DC 1.8
VFREF fref 0 DC 1.8
VFDIV fdiv 0 DC 0

* @RNL {
*   # @SELECT-INSTANCE(type="nmos") => @SET-SYMBOL $NMOS;
*   # @SELECT-INSTANCE(type="pmos") => @SET-SYMBOL $PMOS;
*
*   %cm_n = #MBN(ref) {#MBP #MDN_CS}(out) @AS $CURRENT_MIRROR;
*   %cm_p = #MBP_LOAD(ref) #MUP_CS(out) @AS $CURRENT_MIRROR;
*   {#XDFF_UP #XDFF_DN} @AS $MATCHED;
*   {#MP1 #MP2} @AS $MATCHED;
*   {#IBIAS #MBN} @AS $BRANCH;
*   {#MBP_LOAD #MBP} @AS $BRANCH;
*   {#MUP_CS #MUP_SW #MDN_SW #MDN_CS} @AS $BRANCH;
*   {#RLF #CLF1} @AS $BRANCH;
*   #VFREF(source) {#XDFF_UP #MUP_SW #RLF}(chain) #CLF1(load) @AS $CHAIN;
*
*   {#XDFF_UP:vdd #XDFF_DN:vdd #MP1:s #MP2:s #MBP_LOAD:s #MUP_CS:s} @AS $SUPPLY_HIGH;
*   {#XDFF_UP:vss #XDFF_DN:vss #MN2:s #MBN:s #MBP:s #MDN_CS:s} @AS $SUPPLY_LOW;
*
*   %g_src = {#VDD #VFREF #VFDIV} @GROUP;
*   %g_pfd = {#XDFF_UP #XDFF_DN #MP1 #MP2 #MN1 #MN2} @GROUP;
*   %g_cp = {#IBIAS #MBN #MBP_LOAD #MBP #MUP_CS #MUP_SW #MDN_SW #MDN_CS} @GROUP;
*   %g_lf = {#RLF #CLF1 #CLF2} @GROUP;
*   {{%g_src} {%g_pfd} {%g_cp} {%g_lf}} @AS $FLOW;
*
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   $WIRE_NET_SET @ROUTE
* }
.end
