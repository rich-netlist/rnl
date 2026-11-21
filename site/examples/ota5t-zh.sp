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
*   %amp = %dp1(跨导级) 和 %cm1(负载) @视作 $"跨导阻抗放大器";
*   %left = {#M3 #M1} @视作 $"支路";
*   %right = {#M4 #M2} @视作 $"支路";
*   &vdd @加入 $"电源网络集";
*   &vss @加入 $"地网络集";
*   #M5:d 有 @电流("流入");
*   # @选出网络(端口=是) => @移出 $"端口补位集";
*   %p_inp = $"端口" @附着 &inp;
*   %p_inn = $"端口" @附着 &inn;
*   %p_out = $"端口" @附着 &out;
*   %p_bias = $"端口" @附着 &vbias;
*   # @选出网络 => @布线网络;
*   %amp_group = {%amp %p_inp %p_inn %p_out %p_bias} 视作一个 @分组;
*   {%amp %p_out} @顺序("从左到右")
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:edea2dac05e41f41.6a8320d2b09d0281", numbering="eb723051fdddd716") {
*   %ground_stub_M5_s = $GROUND_STUB @ATTACH(gap=3) #M5::s;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=3) #M3::s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=3) #M4::s;
*   %rt1 = (10, 12);
*   %rt2 = (16, 21);
*   %rt3 = (4, 21);
*   %rt4 = (10, 26);
*   #M1 @PLACE(4, 16) => @ORIENT("R0");
*   #M2 @PLACE(16, 16) => @ORIENT("MY");
*   #M3 @PLACE(4, 26) => @ORIENT("MY");
*   #M4 @PLACE(16, 26) => @ORIENT("R0");
*   #M5 @PLACE(10, 9) => @ORIENT("R0");
*   %dp1 @PLACE(10, 16);
*   %#1 @PLACE(4, 16);
*   %#2 @PLACE(10, 16);
*   %cm1 @PLACE(-1, 26);
*   %amp @PLACE(4, 16);
*   %left @PLACE(4, -0.95);
*   %#3 @PLACE(4, 16);
*   %right @PLACE(16, -0.95);
*   %#4 @PLACE(4, 16);
*   %p_inp @PLACE(-1, 16) => @ORIENT("R0");
*   %p_inn @PLACE(21, 16) => @ORIENT("MY");
*   %p_out @PLACE(19, 21) => @ORIENT("MY");
*   %p_bias @PLACE(-1, 9) => @ORIENT("R0");
*   %ground_stub_M5_s @PLACE(4, 16);
*   %ground_stub_M5_s::g @PLACE(10, 5) => @ORIENT("R0");
*   %supply_stub_M3_s @PLACE(4, 16);
*   %supply_stub_M3_s::s @PLACE(4, 30) => @ORIENT("R0");
*   %supply_stub_M4_s @PLACE(4, 16);
*   %supply_stub_M4_s::s @PLACE(16, 30) => @ORIENT("R0");
*   &tail @WIRE(mode="straight") {%rt1 #M5::d};
*   &tail @WIRE(mode="straight") {%rt1 (4, 12) #M1::s};
*   &tail @WIRE(mode="straight") {%rt1 (16, 12) #M2::s};
*   &out @WIRE(mode="straight") {#M2::d %rt2};
*   &out @WIRE(mode="straight") {%rt2 #M4::d};
*   &out @WIRE(mode="straight") {%rt2 %p_out::pin};
*   &x @WIRE(mode="straight") {#M1::d %rt3};
*   &x @WIRE(mode="straight") {%rt3 #M3::d};
*   &x @WIRE(mode="straight") {%rt3 (10, 21) %rt4};
*   &x @WIRE(mode="straight") {#M3::g %rt4};
*   &x @WIRE(mode="straight") {%rt4 #M4::g};
*   &inp @WIRE(mode="straight") {%p_inp::pin #M1::g};
*   &inn @WIRE(mode="straight") {#M2::g %p_inn::pin};
*   &vbias @WIRE(mode="straight") {%p_bias::pin #M5::g};
*   $JUNCTION @ATTACH %rt1 => @PLACE(10, 12);
*   $JUNCTION @ATTACH %rt2 => @PLACE(16, 21);
*   $JUNCTION @ATTACH %rt3 => @PLACE(4, 21);
*   $JUNCTION @ATTACH %rt4 => @PLACE(10, 26)
* }
.ends ota5t
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:f473dde1e5fad012.53706929d5b144cf") {}
.end
