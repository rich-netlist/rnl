* CC0-1.0 — 13-sample-and-hold.sp: Fully differential bottom-plate sample-and-hold circuit
*! 电路说明：全差分极板下采样保持（Sample-and-Hold）电路，含正负两路输入采样开关（S1、S2）、采样电容（CS1、CS2）、共模归零采样开关（S3、S4）、保持传递开关（S5、S6）、保持负载电容（CH1、CH2）以及差分输入源、共模参考源和两相非交叠时钟源（VIP、VIN、VCM、VPHI1、VPHI2），共 15 个器件。
*! 视觉验收标准：
*! 1. 信号流方向：从左到右——左侧为输入与时钟激励源（VIP、VIN、VCM、VPHI1、VPHI2），然后依次为输入采样开关（S1、S2）、采样电容与极板接地开关（CS1/CS2、S3/S4）、保持开关（S5、S6）及最右侧保持负载电容（CH1、CH2）。
*! 2. 对称对与对称轴：上支路（正端通路：S1、CS1、S3、S5、CH1）与下支路（负端通路：S2、CS2、S4、S6、CH2）关于水平中心轴上下对称（或配对成列对齐）。
*! 3. 电源与地方向：地网 0 位于下方（各电压源负端及 CH1、CH2 负端朝下接地短线）。
*! 4. 行与列对齐：正信号链（S1 -> CS1 -> S5 -> CH1）位于上排，负信号链（S2 -> CS2 -> S6 -> CH2）位于下排；对应每一级（S1/S2、CS1/CS2、S3/S4、S5/S6、CH1/CH2）上下同列对齐。
*! 5. 连线与网络标签：地网 0 采用接地短线；两相时钟控制网（phi1、phi2）及共模参考网（vcm）采用网络名标签（TEXT）；主差分信号通路（inp、inn、sp、sn、bp、bn、outp、outn）用导线（ROUTE）连接。
* @RNL(type="header") { @VERSION("0.3") }

.model sw_sh sw vt=0.9 vh=0.05 ron=50 roff=1e9

* Positive & negative input sampling switches (phi1)
S1 inp sp phi1 0 sw_sh
S2 inn sn phi1 0 sw_sh

* Sampling capacitors
CS1 sp bp 2p
CS2 sn bn 2p

* Bottom-plate reference switches (phi1)
S3 bp vcm phi1 0 sw_sh
S4 bn vcm phi1 0 sw_sh

* Output hold transfer switches (phi2)
S5 bp outp phi2 0 sw_sh
S6 bn outn phi2 0 sw_sh

* Output hold capacitors
CH1 outp 0 1p
CH2 outn 0 1p

* Stimulus and clock sources
VIP inp 0 DC 0.9 AC 0.5
VIN inn 0 DC 0.9 AC 0.5 180
VCM vcm 0 DC 0.9
VPHI1 phi1 0 DC 1.8
VPHI2 phi2 0 DC 0

* @RNL {
*   {#S1 #S2} @AS $MATCHED;
*   {#CS1 #CS2} @AS $MATCHED;
*   {#S3 #S4} @AS $MATCHED;
*   {#S5 #S6} @AS $MATCHED;
*   {#CH1 #CH2} @AS $MATCHED;
*
*   #VIP(source) {#S1 #CS1 #S5}(chain) #CH1(load) @AS $CHAIN;
*   #VIN(source) {#S2 #CS2 #S6}(chain) #CH2(load) @AS $CHAIN;
*
*   %g_src = {#VIP #VIN #VCM #VPHI1 #VPHI2} @GROUP;
*   %g_sw_in = {#S1 #S2} @GROUP;
*   %g_cap_s = {#CS1 #CS2 #S3 #S4} @GROUP;
*   %g_sw_out = {#S5 #S6} @GROUP;
*   %g_cap_h = {#CH1 #CH2} @GROUP;
*   {{%g_src} {%g_sw_in} {%g_cap_s} {%g_sw_out} {%g_cap_h}} @AS $FLOW;
*
*   {&phi1 &phi2 &vcm} @REMOVE-FROM $WIRE_NET_SET;
*   {&phi1 &phi2 &vcm} @SELECT-TERMINAL(wired=false) => @TEXT("${net}", key="netname");
*   $WIRE_NET_SET @ROUTE
* }
.end
