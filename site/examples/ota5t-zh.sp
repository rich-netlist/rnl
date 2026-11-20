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
*   # @选出网络 => @布线网络;
*   $"端口桩" @附着 {#M1:g #M2:g #M5:g};
*   %po = $"端口" @附着 &out;
*   {%amp %po::pin} @顺序("从左到右")
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:13bd9770c3063bbd.c9d4c659d9406c2d", numbering="57ae7396f4341817") {
*   %ground_stub_M5_s = $GROUND_STUB @ATTACH(gap=3) #M5::s;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=3) #M3::s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=3) #M4::s;
*   %io_port_vdd = $IO_PORT @ATTACH &vdd;
*   %io_port_vss = $IO_PORT @ATTACH &vss;
*   %rt1 = (15, 13);
*   %rt2 = (20, 21);
*   %rt3 = (10, 21);
*   %rt4 = (15, 26);
*   #M1 @PLACE(10, 16) => @ORIENT("R0");
*   #M2 @PLACE(20, 16) => @ORIENT("MY");
*   #M3 @PLACE(10, 26) => @ORIENT("MY");
*   #M4 @PLACE(20, 26) => @ORIENT("R0");
*   #M5 @PLACE(15, 9) => @ORIENT("MY");
*   %dp1 @PLACE(15, 16);
*   %#1 @PLACE(10, 16);
*   %#2 @PLACE(15, 16);
*   %cm1 @PLACE(5.95, 26);
*   %amp @PLACE(10, 16);
*   %left @PLACE(10, -0.95);
*   %#3 @PLACE(10, 16);
*   %right @PLACE(20, -0.95);
*   %#4 @PLACE(10, 16);
*   %#5 @PLACE(10, 16);
*   %#5::p @PLACE(7, 16) => @ORIENT("R0");
*   %#6 @PLACE(10, 16);
*   %#6::p @PLACE(23, 16) => @ORIENT("R180");
*   %#7 @PLACE(10, 16);
*   %#7::p @PLACE(18, 9) => @ORIENT("MY");
*   %po @PLACE(23, 21) => @ORIENT("MY");
*   %ground_stub_M5_s @PLACE(10, 16);
*   %ground_stub_M5_s::g @PLACE(15, 5) => @ORIENT("R0");
*   %supply_stub_M3_s @PLACE(10, 16);
*   %supply_stub_M3_s::s @PLACE(10, 30) => @ORIENT("R0");
*   %supply_stub_M4_s @PLACE(10, 16);
*   %supply_stub_M4_s::s @PLACE(20, 30) => @ORIENT("R0");
*   %io_port_vdd @PLACE(4, 5) => @ORIENT("R0");
*   %io_port_vss @PLACE(4, 1) => @ORIENT("R0");
*   &tail @WIRE(mode="straight") {#M1::s (10, 13) %rt1};
*   &tail @WIRE(mode="straight") {#M5::d %rt1};
*   &tail @WIRE(mode="straight") {%rt1 (20, 13) #M2::s};
*   &out @WIRE(mode="straight") {#M2::d %rt2};
*   &out @WIRE(mode="straight") {%rt2 #M4::d};
*   &out @WIRE(mode="straight") {%rt2 %po::pin};
*   &x @WIRE(mode="straight") {#M1::d %rt3};
*   &x @WIRE(mode="straight") {%rt3 #M3::d};
*   &x @WIRE(mode="straight") {%rt3 (15, 21) %rt4};
*   &x @WIRE(mode="straight") {#M3::g %rt4};
*   &x @WIRE(mode="straight") {%rt4 #M4::g};
*   $JUNCTION @ATTACH %rt1 => @PLACE(15, 13);
*   $JUNCTION @ATTACH %rt2 => @PLACE(20, 21);
*   $JUNCTION @ATTACH %rt3 => @PLACE(10, 21);
*   $JUNCTION @ATTACH %rt4 => @PLACE(15, 26)
* }
.ends ota5t
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:f473dde1e5fad012.53706929d5b144cf") {}
.end
