* CC0-1.0 — 04-fd-ota-cmfb.sp: Complete transistor-level fully differential folded-cascode OTA with CT-CMFB and bias network
*! 电路说明：大规模全差分折叠共源共栅运算放大器完整晶体管级电路（含主放大器、双差分对连续时间共模反馈放大器 CT-CMFB、宽摆幅共源共栅偏置网络、启动电路及全差分闭环测试激励与RC负载网络）。
*! 器件规模：56 个器件（36 个 MOS 管：启动电路 MS1–MS4，偏置网络 MB1–MB12，主放大器 M1–M12，共模反馈放大器 MC1–MC8；12 个无源器件：RS1、RCM1、RCM2、CCM1、CCM2、CC1、CC2、CL1、CL2、RL1、RL2、CB；8 个独立电源/激励源：VDD、VSS、IBIAS、VCMREF、VIP、VIN、VBP_EXT、VBN_EXT）。
*! 视觉验收标准：
*! 1. 信号流方向：从左到右——最左侧为电源源与启动电路（MS1–MS4），其右为宽摆幅共源共栅偏置生成网络（MB1–MB12），中间偏左为差分输入激励（VIP、VIN）与主全差分折叠共源共栅放大级（M1–M12），中间偏右为共模检测电阻电容网络（RCM1/RCM2、CCM1/CCM2）与双差分对共模反馈放大器（MC1–MC8），最右侧为输出负载与补偿网络（CC1/CC2、CL1/CL2、RL1/RL2）。
*! 2. 对称对与对称轴：
*!    - 主放大器输入对（M1、M2）与双尾管（M0A、M0B）关于主轴（%ax_main）镜像对称；折叠支路四层对管（M3/M4、M5/M6、M7/M8、M9/M10）关于折叠级对称轴（%ax_fc）左右镜像对称。
*!    - 共模反馈放大器左差分对（MC1、MC2）、右差分对（MC3、MC4）、有源负载电流镜对（MC5、MC6）及双尾管（MC7、MC8）关于 CMFB 对称轴（%ax_cmfb）镜像对称。
*! 3. 电源与地方向：正电源网 vdd 位于最上方（所有顶排 PMOS 源端朝上接电源短线），负电源网 vss 及地网 0 位于最下方（底层 NMOS 源端朝下接命名接地短线或接地短线）。
*! 4. 行与列对齐：偏置网络 6 条垂直支路（MB1/MB2、MB3/MB4、MB5/MB6、MB7/MB8、MB9/MB10、MB11/MB12）各自上下成列并按行水平对齐；主放大器折叠支路左列（M3、M5、M7、M9）与右列（M4、M6、M8、M10）严格成列对齐。
*! 5. 连线与网络标签：电源网 vdd 使用电源短线，负电源网 vss 与地网 0 使用接地短线；偏置总线（vbp1、vbp2、vbn1、vbn2、vbt）使用端口/网络名标签（TEXT），主信号通路与局部反馈网络使用导线（ROUTE）连接。
* @RNL(type="header") { @VERSION("0.3") }

.model nch nmos level=1
.model pch pmos level=1

* -----------------------------------------------------------------
* 1. Startup circuit (4 MOS + 1 resistor = 5 devices)
* -----------------------------------------------------------------
MS1 st_sense vbt vss vss nch W=2u L=2u
RS1 vdd st_sense 50k
MS2 st_gate st_sense vss vss nch W=4u L=0.5u
MS3 st_gate vdd vdd vdd pch W=1u L=4u
MS4 vbp1 st_gate vss vss nch W=4u L=0.5u

* -----------------------------------------------------------------
* 2. Wide-swing cascode bias network (12 MOS + 1 isource + 1 cap = 14 devices)
* -----------------------------------------------------------------
IBIAS vdd nb_in 25u
MB1 nb_in vbn1 nb_d1 vss nch W=8u L=0.5u
MB2 nb_d1 nb_in vss vss nch W=8u L=1u
MB3 vbn1 vbn1 nb_d2 vss nch W=4u L=1u
MB4 nb_d2 nb_in vss vss nch W=8u L=1u
MB5 vbp1 vbp1 vdd vdd pch W=16u L=1u
MB6 vbp1 vbn1 nb_d3 vss nch W=8u L=0.5u
MB7 nb_d3 nb_in vss vss nch W=8u L=1u
MB8 vbp2 vbp2 vdd vdd pch W=8u L=1u
MB9 vbp2 vbn1 nb_d4 vss nch W=8u L=0.5u
MB10 nb_d4 nb_in vss vss nch W=8u L=1u
MB11 vbt vbp1 pb_d5 vdd pch W=16u L=1u
MB12 vbt vbt vss vss nch W=16u L=1u
CB vbt vss 0.5p

* -----------------------------------------------------------------
* 3. Main Fully Differential Folded-Cascode Stage (12 MOS)
* -----------------------------------------------------------------
M1 fold_p inp tail vss nch W=24u L=0.5u
M2 fold_n inn tail vss nch W=24u L=0.5u
M0A tail vbt vss vss nch W=16u L=1u
M0B tail vcmfb vss vss nch W=16u L=1u

M3 fold_p vbp1 vdd vdd pch W=32u L=1u
M4 fold_n vbp1 vdd vdd pch W=32u L=1u
M5 outn vbp2 fold_p vdd pch W=24u L=0.5u
M6 outp vbp2 fold_n vdd pch W=24u L=0.5u
M7 outn vbn1 mir_p vss nch W=12u L=0.5u
M8 outp vbn1 mir_n vss nch W=12u L=0.5u
M9 mir_p vcmfb vss vss nch W=12u L=1u
M10 mir_n vcmfb vss vss nch W=12u L=1u

* -----------------------------------------------------------------
* 4. Continuous-Time CMFB Detector & Dual-Diff-Pair Amplifier (8 MOS + 4 passives = 12 devices)
* -----------------------------------------------------------------
RCM1 outp vcm_sense 40k
RCM2 vcm_sense outn 40k
CCM1 outp vcm_sense 0.5p
CCM2 vcm_sense outn 0.5p

MC1 vcmfb outp cm_tail1 vss nch W=6u L=0.5u
MC2 cm_x vcmref cm_tail1 vss nch W=6u L=0.5u
MC3 cm_x vcmref cm_tail2 vss nch W=6u L=0.5u
MC4 vcmfb outn cm_tail2 vss nch W=6u L=0.5u
MC5 vcmfb vcmfb vdd vdd pch W=12u L=1u
MC6 cm_x vcmfb vdd vdd pch W=12u L=1u
MC7 cm_tail1 vbt vss vss nch W=8u L=1u
MC8 cm_tail2 vbt vss vss nch W=8u L=1u

* -----------------------------------------------------------------
* 5. Compensation, Differential Load & Testbench Sources (6 passives + 7 vsource = 13 devices)
* -----------------------------------------------------------------
CC1 fold_p outn 1p
CC2 fold_n outp 1p
CL1 outp 0 2p
CL2 outn 0 2p
RL1 outp vcmref 20k
RL2 outn vcmref 20k

VDD vdd 0 DC 1.8
VSS vss 0 DC 0
VCMREF vcmref 0 DC 0.9
VIP inp 0 DC 0.9 AC 0.5
VIN inn 0 DC 0.9 AC 0.5 180
VBP_EXT pb_d5 vdd DC 0
VBN_EXT vbn2 nb_in DC 0

* @RNL {
*   # @SELECT-INSTANCE(type="nmos") => @SET-SYMBOL $NMOS;
*   # @SELECT-INSTANCE(type="pmos") => @SET-SYMBOL $PMOS;
*
*   /* Semantic descriptions: differential pairs, current mirrors, matched sets, branches */
*   %dp_main = #M1(in_p) #M2(in_n) {#M0A #M0B}(tail) @AS $DIFFERENTIAL_PAIR;
*   %dp_cm1 = #MC1(in_p) #MC2(in_n) #MC7(tail) @AS $DIFFERENTIAL_PAIR;
*   %dp_cm2 = #MC4(in_p) #MC3(in_n) #MC8(tail) @AS $DIFFERENTIAL_PAIR;
*
*   %cm_bias_n = #MB2(ref) {#MB4 #MB7 #MB10}(out) @AS $CURRENT_MIRROR;
*   %cm_bias_p = #MB5(ref) {#MB11 #M3 #M4}(out) @AS $CURRENT_MIRROR;
*   %cm_tail = #MB12(ref) {#M0A #MC7 #MC8}(out) @AS $CURRENT_MIRROR;
*   %cm_cmfb = #MC5(ref) #MC6(out) @AS $CURRENT_MIRROR;
*
*   {#M1 #M2} @AS $MATCHED;
*   {#M3 #M4} @AS $MATCHED;
*   {#M5 #M6} @AS $MATCHED;
*   {#M7 #M8} @AS $MATCHED;
*   {#M9 #M10} @AS $MATCHED;
*   {#MC1 #MC2 #MC3 #MC4} @AS $MATCHED;
*   {#MC5 #MC6} @AS $MATCHED;
*   {#MC7 #MC8} @AS $MATCHED;
*   {#RCM1 #RCM2} @AS $MATCHED;
*   {#CCM1 #CCM2} @AS $MATCHED;
*   {#CL1 #CL2} @AS $MATCHED;
*   {#RL1 #RL2} @AS $MATCHED;
*
*   {#RS1 #MS1} @AS $BRANCH;
*   {#MS3 #MS2} @AS $BRANCH;
*   {#IBIAS #MB1 #MB2} @AS $BRANCH;
*   {#MB3 #MB4} @AS $BRANCH;
*   {#MB5 #MB6 #MB7} @AS $BRANCH;
*   {#MB8 #MB9 #MB10} @AS $BRANCH;
*   {#MB11 #MB12} @AS $BRANCH;
*   {#M3 #M5 #M7 #M9} @AS $BRANCH;
*   {#M4 #M6 #M8 #M10} @AS $BRANCH;
*
*   /* Layout symmetry pairs */
*   %ax_in = (_, _);
*   #M1(a) #M2(b) %ax_in(axis) @AS $MIRRORED;
*   #M0A(a) #M0B(b) %ax_in(axis) @AS $MIRRORED;
*
*   %ax_fc = (_, _);
*   #M3(a) #M4(b) %ax_fc(axis) @AS $MIRRORED;
*   #M5(a) #M6(b) %ax_fc(axis) @AS $MIRRORED;
*   #M7(a) #M8(b) %ax_fc(axis) @AS $MIRRORED;
*   #M9(a) #M10(b) %ax_fc(axis) @AS $MIRRORED;
*
*   %ax_cmfb = (_, _);
*   #MC1(a) #MC4(b) %ax_cmfb(axis) @AS $MIRRORED;
*   #MC2(a) #MC3(b) %ax_cmfb(axis) @AS $MIRRORED;
*   #MC5(a) #MC6(b) %ax_cmfb(axis) @AS $MIRRORED;
*   #MC7(a) #MC8(b) %ax_cmfb(axis) @AS $MIRRORED;
*
*   /* Supply high / low terminal semantics */
*   {#MS3:s #MB5:s #MB8:s #M3:s #M4:s #MC5:s #MC6:s} @AS $SUPPLY_HIGH;
*   {#MS1:s #MS2:s #MS4:s #MB2:s #MB4:s #MB7:s #MB10:s #MB12:s #M0A:s #M0B:s #M9:s #M10:s #MC7:s #MC8:s} @AS $SUPPLY_LOW;
*
*   /* Functional groups and overall signal flow */
*   %g_src = {#VDD #VSS #VCMREF #VBP_EXT #VBN_EXT} @GROUP;
*   %g_start = {#RS1 #MS1 #MS2 #MS3 #MS4} @GROUP;
*   %g_bias = {#IBIAS #MB1 #MB2 #MB3 #MB4 #MB5 #MB6 #MB7 #MB8 #MB9 #MB10 #MB11 #MB12 #CB} @GROUP;
*   %g_in = {#VIP #VIN #M1 #M2 #M0A #M0B} @GROUP;
*   %g_fc = {#M3 #M4 #M5 #M6 #M7 #M8 #M9 #M10 #CC1 #CC2} @GROUP;
*   %g_cmfb = {#RCM1 #RCM2 #CCM1 #CCM2 #MC1 #MC2 #MC3 #MC4 #MC5 #MC6 #MC7 #MC8} @GROUP;
*   %g_load = {#CL1 #CL2 #RL1 #RL2} @GROUP;
*   {{%g_src} {%g_start} {%g_bias} {%g_in} {%g_fc} {%g_cmfb} {%g_load}} @AS $FLOW;
*
*   /* Supply & ground sets; long bias nets use netname labels while signal nets use wires */
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   {&vbp1 &vbp2 &vbn1 &vbt &vcmref} @REMOVE-FROM $WIRE_NET_SET;
*   {&vbp1 &vbp2 &vbn1 &vbt &vcmref} @SELECT-TERMINAL(wired=false) => @TEXT("${net}", key="netname");
*   $WIRE_NET_SET @ROUTE
* }
.end
