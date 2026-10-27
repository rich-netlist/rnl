* Two-stage Miller-compensated op-amp with its bias
.model nch nmos level=1 vto=0.5
.model pch pmos level=1 vto=-0.5
* @RNL(type="header") { @VERSION("0.3"); @USE-ALIASES(name="rnl-zh", version="0.3")}
.subckt opamp2 inp inn out vdd vss ibias
* first stage: differential pair, tail current source, current-mirror load
M1 x    inp    tail vss nch W=40u  L=1u
M2 n1   inn    tail vss nch W=40u  L=1u
M3 x    x      vdd  vdd pch W=20u  L=1u
M4 n1   x      vdd  vdd pch W=20u  L=1u
M5 tail ibias  vss  vss nch W=20u  L=1u
* second stage: common-source amplifier with a current-source load
M6 out  n1     vdd  vdd pch W=200u L=1u
M7 out  ibias  vss  vss nch W=100u L=1u
* Miller compensation: a nulling resistor and a capacitor from n1 to out
RZ n1 nz 2k
CC nz out 1p
* bias: the current into ibias is mirrored to the tail and to the second stage
MB ibias ibias vss vss nch W=10u L=1u
* PROVISIONAL: the standard packages have no definition of the Miller compensation yet. This local one
* says only what it is; it is replaced by the standard one when there is one.
* @RNL { $MILLER_COMPENSATION = @DEFINE(roles=["{stage}", "{comp}"]) {}}
* @RNL {
*   &vdd @加入 $"电源网络集";
*   &vss @加入 $"地网络集";
*   {#M1 #M2 #M5 #M7 #MB} @使用符号 $NMOS;
*   {#M3 #M4 #M6} @使用符号 $PMOS;
*   %dp1 = #M1(正输入)、#M2(负输入) 和 #M5(尾管) @视作 $"差分对";
*   %cm1 = #M3(参考管) 和 #M4(输出管) @视作 $"电流镜";
*   %cm2 = #MB(参考管) 和 {#M5 #M7}(输出管) @视作 $"电流镜";
*   %mc = {#M6}(stage) 和 {#RZ #CC}(comp) @视作 $MILLER_COMPENSATION;
*   %ax = (_, _);
*   %sym12 = #M1(甲)、#M2(乙) 和 %ax(轴) @视作 $"镜像对称";
*   %sym34 = #M3(甲)、#M4(乙) 和 %ax(轴) @视作 $"镜像对称";
*   %flow = {#M1 #M2 #M6} @视作 $"信号流向";
*   #M5:d 有 @电流("流入");
*   #M7:d 有 @电流("流入");
*   #M6:d 有 @电流("流出")
* }
* @RNL(author="agent") {
*   #M1 @PLACE(12, 8) => @ORIENT("R0");  #M2 @PLACE(32, 8) => @ORIENT("MY");
*   #M3 @PLACE(12, 16) => @ORIENT("MY");  #M4 @PLACE(32, 16) => @ORIENT("R0");
*   #M5 @PLACE(22, 0);  #MB @PLACE(14, 0) => @ORIENT("MY");
*   #M6 @PLACE(48, 20);  #M7 @PLACE(48, 8);
*   #RZ @PLACE(39, 12) => @ORIENT("R90");  #CC @PLACE(45, 12) => @ORIENT("R90");
*   %ja = (16, 16);  %jb = (12, 12);
*   &x @WIRE {#M3:d %jb #M1:d};  &x @WIRE {#M3:g %ja #M4:g};
*   &x @WIRE(mode="vertical-horizontal") {%ja %jb};
*   %jo = (32, 12);  %j6 = (36, 12);
*   &n1 @WIRE {#M4:d %jo #M2:d};
*   &n1 @WIRE {%jo %j6};  &n1 @WIRE {%j6 #RZ:1};
*   &n1 @WIRE(mode="vertical-horizontal") {%j6 #M6:g};
*   &nz @WIRE {#RZ:2 #CC:1};
*   %jt = (22, 4);
*   &tail @WIRE(mode="vertical-horizontal") {#M1:s %jt};
*   &tail @WIRE(mode="vertical-horizontal") {#M2:s %jt};
*   &tail @WIRE {%jt #M5:d};
*   %jq = (48, 12);
*   &out @WIRE {#M6:d %jq #M7:d};  &out @WIRE {#CC:2 %jq};
*   %jv = (18, 0);  %jy = (18, -9);  %jz = (45, -9);
*   &ibias @WIRE(mode="horizontal-vertical") {#MB:d #MB:g};
*   &ibias @WIRE {#MB:g %jv #M5:g};
*   &ibias @WIRE {%jv %jy %jz};  &ibias @WIRE(mode="vertical-horizontal") {%jz #M7:g};
*   $JUNCTION @ATTACH {%ja %jb %jo %j6 %jt %jq %jv};
*   %pi = $IO_PORT @ATTACH &inp;  %pi @PLACE(5, 8);  &inp @WIRE {#M1:g %pi:p};
*   %pn = $IO_PORT @ATTACH &inn;  %pn @PLACE(37, 8) => @ORIENT("MY");  &inn @WIRE {#M2:g %pn:p};
*   %po = $IO_PORT @ATTACH &out;  %po @PLACE(58, 12) => @ORIENT("MY");  &out @WIRE {%jq %po:p};
*   %pb = $IO_PORT @ATTACH &ibias;  %pb @PLACE(5, 2);  &ibias @WIRE(mode="horizontal-vertical") {%pb:p #MB:d};
*   %vdd1 = $SUPPLY_STUB @ATTACH #M3:s;  %vdd1::s @PLACE(12, 20);
*   %vdd2 = $SUPPLY_STUB @ATTACH #M4:s;  %vdd2::s @PLACE(32, 20);
*   %vdd3 = $SUPPLY_STUB @ATTACH #M6:s;  %vdd3::s @PLACE(48, 24);
*   %gnd1 = $GROUND_STUB @ATTACH #M5:s;  %gnd1::g @PLACE(22, -4);
*   %gnd2 = $GROUND_STUB @ATTACH #MB:s;  %gnd2::g @PLACE(14, -4);
*   %gnd3 = $GROUND_STUB @ATTACH #M7:s;  %gnd3::g @PLACE(48, 4);
*   %ax @PLACE(22, 8)
* }
.ends opamp2
.end
