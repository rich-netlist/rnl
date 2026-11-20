* Five-transistor OTA written in the Chinese style
.model nch nmos level=1
.model pch pmos level=1
M1 x inp tail vss nch w=4u l=0.5u
M2 out inn tail vss nch w=4u l=0.5u
M3 x x vdd vdd pch w=8u l=0.5u
M4 out x vdd vdd pch w=8u l=0.5u
M5 tail bias vss vss nch w=8u l=1u
R1 vdd n1 20k
R2 n1 bias 20k
R3 bias vss 10k
CL out vss 1p
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4"); @USE-ALIASES(name="alias-zh", version="0.4")}
* @RNL {
*   &vdd @加入 $"电源网络集";
*   &vss @加入 $"地网络集";
*   %dp1 = #M1(左)、#M2(右) 和 #M5(尾管) @视作 $"N型差分对";
*   %cm1 = #M3(参考管) 和 #M4(输出管) @视作 $"P型电流镜";
*   %amp = %dp1(跨导级) 和 %cm1(负载) @视作 $"跨导阻抗放大器";
*   %bias = {#R1 #R2 #R3} @视作 $"支路";
*   #M5:d 有 @电流("流入");
*   # @选出网络 => @布线网络;
*   {%bias %amp} @顺序("从左到右");
*   $"橙色" = @定义样式(线色="#e07000", 字色="#e07000") {};
*   {#R1 #R2 #R3} @使用样式 $"橙色";
*   &out 上有 @文本("输出");
*   &bias 上有 @文本("偏置")
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:cb0d26a7864e7d74.bc0cc790653fed65", numbering="b56308b56b934500") {
*   %ground_stub_M5_s = $GROUND_STUB @ATTACH(gap=3) #M5::s;
*   %ground_stub_R3_n = $GROUND_STUB @ATTACH(gap=3) #R3::n;
*   %ground_stub_CL_n = $GROUND_STUB @ATTACH(gap=3) #CL::n;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=3) #M3::s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=3) #M4::s;
*   %supply_stub_R1_p = $SUPPLY_STUB @ATTACH(gap=3) #R1::p;
*   %net_label_stub_M1_g = $NET_LABEL_STUB @ATTACH #M1::g;
*   %net_label_stub_M2_g = $NET_LABEL_STUB @ATTACH #M2::g;
*   %rt1 = (17, 15);
*   %rt2 = (11, 23);
*   %rt3 = (17, 27);
*   %rt4 = (23, 23);
*   %rt5 = (3, 1);
*   #M1 @PLACE(11, 19) => @ORIENT("R0");
*   #M2 @PLACE(23, 19) => @ORIENT("MY");
*   #M3 @PLACE(11, 27) => @ORIENT("MY");
*   #M4 @PLACE(23, 27) => @ORIENT("R0");
*   #M5 @PLACE(17, 1) => @ORIENT("R0");
*   #R1 @PLACE(3, 11) => @ORIENT("R0");
*   #R2 @PLACE(3, 5) => @ORIENT("R0");
*   #R3 @PLACE(3, -5) => @ORIENT("R0");
*   #CL @PLACE(31, 19) => @ORIENT("R0");
*   %dp1 @PLACE(17, 19);
*   %#1 @PLACE(13, 1);
*   %#2 @PLACE(17, 19);
*   %cm1 @PLACE(6.95, 27);
*   %amp @PLACE(13, 1);
*   %bias @PLACE(3, 3);
*   %#3 @PLACE(13, 1);
*   %ground_stub_M5_s @PLACE(13, 1);
*   %ground_stub_M5_s::g @PLACE(17, -3) => @ORIENT("R0");
*   %ground_stub_R3_n @PLACE(13, 1);
*   %ground_stub_R3_n::g @PLACE(3, -9) => @ORIENT("R0");
*   %ground_stub_CL_n @PLACE(13, 1);
*   %ground_stub_CL_n::g @PLACE(31, 15) => @ORIENT("R0");
*   %supply_stub_M3_s @PLACE(13, 1);
*   %supply_stub_M3_s::s @PLACE(11, 31) => @ORIENT("R0");
*   %supply_stub_M4_s @PLACE(13, 1);
*   %supply_stub_M4_s::s @PLACE(23, 31) => @ORIENT("R0");
*   %supply_stub_R1_p @PLACE(13, 1);
*   %supply_stub_R1_p::s @PLACE(3, 15) => @ORIENT("R0");
*   %net_label_stub_M1_g @PLACE(13, 1);
*   %net_label_stub_M1_g::l @PLACE(8, 19) => @ORIENT("R0");
*   %net_label_stub_M2_g @PLACE(13, 1);
*   %net_label_stub_M2_g::l @PLACE(26, 19) => @ORIENT("MY");
*   &tail @WIRE(mode="straight") {%rt1 #M5::d};
*   &tail @WIRE(mode="straight") {%rt1 (11, 15) #M1::s};
*   &tail @WIRE(mode="straight") {%rt1 (23, 15) #M2::s};
*   &x @WIRE(mode="straight") {#M1::d %rt2};
*   &x @WIRE(mode="straight") {%rt2 #M3::d};
*   &x @WIRE(mode="straight") {%rt2 (17, 23) %rt3};
*   &x @WIRE(mode="straight") {#M3::g %rt3};
*   &x @WIRE(mode="straight") {%rt3 #M4::g};
*   &out @WIRE(mode="straight") {#M2::d %rt4};
*   &out @WIRE(mode="straight") {%rt4 #M4::d};
*   &out @WIRE(mode="straight") {%rt4 (31, 23) #CL::p};
*   &bias @WIRE(mode="straight") {#R3::p %rt5};
*   &bias @WIRE(mode="straight") {%rt5 #R2::n};
*   &bias @WIRE(mode="straight") {%rt5 #M5::g};
*   &n1 @WIRE(mode="straight") {#R2::p #R1::n};
*   $JUNCTION @ATTACH %rt1 => @PLACE(17, 15);
*   $JUNCTION @ATTACH %rt2 => @PLACE(11, 23);
*   $JUNCTION @ATTACH %rt3 => @PLACE(17, 27);
*   $JUNCTION @ATTACH %rt4 => @PLACE(23, 23);
*   $JUNCTION @ATTACH %rt5 => @PLACE(3, 1)
* }
.end
