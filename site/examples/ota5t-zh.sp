* 5T OTA
.model nch nmos level=1 vto=0.5
.model pch pmos level=1 vto=-0.5
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-ALIASES(name="alias-zh", version="0.4")}
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M3 x    x     vdd  vdd pch W=4u L=0.5u
M4 out  x     vdd  vdd pch W=4u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   %dp1 = #M1(左)、#M2(右) 和 #M5(尾管) @视作 $"N型差分对";
*   %cm1 = #M3(参考管) 和 #M4(输出管) @视作 $"P型电流镜";
*   &vdd @加入 $"电源网络集";
*   &vss @加入 $"地网络集";
*   #M5:d 有 @电流("流入")
* }
* @RNL(type="generated", priority="high", author="rnl-solver", hash="sig2:ffad8672ab912e17.ffc36700559c31e7") {
*   #M1 @PLACE(12, 8) => @ORIENT("R0");  #M2 @PLACE(28, 8) => @ORIENT("MY");
*   #M3 @PLACE(12, 16) => @ORIENT("MY");  #M4 @PLACE(28, 16) => @ORIENT("R0");
*   #M5 @PLACE(20, 0);  %ja = (16, 16);  %jb = (12, 12);
*   &x @WIRE {#M3:d %jb #M1:d};  &x @WIRE {#M3:g %ja #M4:g};
*   &x @WIRE(mode="vertical-horizontal") {%ja %jb};
*   %jo = (28, 12);  %jt = (20, 4);
*   &out @WIRE {#M4:d %jo};  &out @WIRE(mode="horizontal-vertical") {%jo #M2:d};
*   &tail @WIRE(mode="vertical-horizontal") {#M1:s %jt};
*   &tail @WIRE(mode="vertical-horizontal") {#M2:s %jt};
*   &tail @WIRE {%jt #M5:d};
*   $JUNCTION @ATTACH {%ja %jb %jo %jt};
*   %pi = $IO_PORT @ATTACH &inp;  %pi @PLACE(5, 8);
*   %pn = $IO_PORT @ATTACH &inn;  %pn @PLACE(35, 8) => @ORIENT("MY");
*   %po = $IO_PORT @ATTACH &out;  %po @PLACE(35, 12) => @ORIENT("MY");
*   %pb = $IO_PORT @ATTACH &vbias;  %pb @PLACE(13, 0);
*   &inp @WIRE {#M1:g %pi:p};  &inn @WIRE {#M2:g %pn:p};
*   &out @WIRE {%jo %po:p};  &vbias @WIRE {#M5:g %pb:p};
*   %vdd1 = $SUPPLY_STUB @ATTACH #M3:s;  %vdd1::s @PLACE(12, 20);
*   %vdd2 = $SUPPLY_STUB @ATTACH #M4:s;  %vdd2::s @PLACE(28, 20);
*   %gnd = $GROUND_STUB @ATTACH #M5:s;  %gnd::g @PLACE(20, -4)
* }
.ends ota5t
.end
