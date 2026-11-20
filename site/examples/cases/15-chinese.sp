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
*   #M5 @位于(24, 0);
*   # @选出网络 => @布线网络;
*   {%bias %amp} @顺序("从左到右");
*   $"橙色" = @定义样式(线色="#e07000", 字色="#e07000") {};
*   {#R1 #R2 #R3} @使用样式 $"橙色";
*   {#M1 #M2} @使用样式 $"信号";
*   $"网络标签" @附着 在 &out 上;
*   #M1 上有 @文本("W/L = ${w}/${l}", 引用名="wl", dx=-9, dy=-4);
*   在 (-6, 28) 有 @文本("五管 OTA：偏置由 R1、R2、R3 分压给出", 字号=1.1) => 记作 %title;
*   以 (17, 3) 为中心的 @矩形(宽度=50, 高度=46, 填充="填充色", 填充色="#1f4fd80d", 线型="点线") => 其处于 @图层("底")
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:dc067c4f6e3592b9.287704ecdc57150d", numbering="3bcac899f01c9a41") {
*   %ground_stub_M5_s = $GROUND_STUB @ATTACH(gap=3) #M5::s;
*   %ground_stub_R3_n = $GROUND_STUB @ATTACH(gap=3) #R3::n;
*   %ground_stub_CL_n = $GROUND_STUB @ATTACH(gap=3) #CL::n;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=3) #M3::s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=3) #M4::s;
*   %supply_stub_R1_p = $SUPPLY_STUB @ATTACH(gap=3) #R1::p;
*   %net_label_stub_M1_g = $NET_LABEL_STUB @ATTACH #M1::g;
*   %net_label_stub_M2_g = $NET_LABEL_STUB @ATTACH #M2::g;
*   %rt1 = (24, 7);
*   %rt2 = (5, 14);
*   %rt3 = (20, 14);
*   %rt4 = (28, 14);
*   %rt5 = (36, 14);
*   %rt6 = (-3, 0);
*   #M1 @PLACE(20, 10) => @ORIENT("R0");
*   #M2 @PLACE(28, 10) => @ORIENT("MY");
*   #M3 @PLACE(5, 17) => @ORIENT("R0");
*   #M4 @PLACE(36, 17) => @ORIENT("R0");
*   #M5 @PLACE(24, 0) => @ORIENT("R0");
*   #R1 @PLACE(-3, 17) => @ORIENT("R0");
*   #R2 @PLACE(-3, 3) => @ORIENT("R0");
*   #R3 @PLACE(-3, -12) => @ORIENT("R0");
*   #CL @PLACE(36, -12) => @ORIENT("R0");
*   %dp1 @PLACE(24, 10);
*   %#1 @PLACE(0, 0);
*   %#2 @PLACE(24, 10);
*   %cm1 @PLACE(0, 17);
*   %amp @PLACE(0, 0);
*   %bias @PLACE(-3, 0);
*   %#3 @PLACE(0, 0);
*   %#4 @PLACE(28, 15) => @ORIENT("R270");
*   %ground_stub_M5_s @PLACE(0, 0);
*   %ground_stub_M5_s::g @PLACE(24, -4) => @ORIENT("R0");
*   %ground_stub_R3_n @PLACE(0, 0);
*   %ground_stub_R3_n::g @PLACE(-3, -16) => @ORIENT("R0");
*   %ground_stub_CL_n @PLACE(0, 0);
*   %ground_stub_CL_n::g @PLACE(36, -16) => @ORIENT("R0");
*   %supply_stub_M3_s @PLACE(0, 0);
*   %supply_stub_M3_s::s @PLACE(5, 21) => @ORIENT("R0");
*   %supply_stub_M4_s @PLACE(0, 0);
*   %supply_stub_M4_s::s @PLACE(36, 21) => @ORIENT("R0");
*   %supply_stub_R1_p @PLACE(0, 0);
*   %supply_stub_R1_p::s @PLACE(-3, 21) => @ORIENT("R0");
*   %net_label_stub_M1_g @PLACE(0, 0);
*   %net_label_stub_M1_g::l @PLACE(17, 10) => @ORIENT("R0");
*   %net_label_stub_M2_g @PLACE(0, 0);
*   %net_label_stub_M2_g::l @PLACE(31, 10) => @ORIENT("R180");
*   %#5 @PLACE(-6, 28);
*   %#6 @PLACE(17, 3);
*   %title @PLACE(-6, 28);
*   &tail @WIRE(mode="straight") {#M1::s (20, 7) %rt1};
*   &tail @WIRE(mode="straight") {%rt1 (28, 7) #M2::s};
*   &tail @WIRE(mode="straight") {#M5::d %rt1};
*   &x @WIRE(mode="straight") {#M3::g (1, 17) (1, 14) %rt2};
*   &x @WIRE(mode="straight") {%rt2 #M3::d};
*   &x @WIRE(mode="straight") {#M1::d %rt3};
*   &x @WIRE(mode="straight") {%rt3 %rt2};
*   &x @WIRE(mode="straight") {#M4::g (20, 17) %rt3};
*   &out @WIRE(mode="straight") {#M2::d %rt4};
*   &out @WIRE(mode="straight") {%rt4 %#4::pin};
*   &out @WIRE(mode="straight") {#M4::d %rt5};
*   &out @WIRE(mode="straight") {%rt5 %rt4};
*   &out @WIRE(mode="straight") {#CL::p %rt5};
*   &bias @WIRE(mode="straight") {#R3::p %rt6};
*   &bias @WIRE(mode="straight") {%rt6 #R2::n};
*   &bias @WIRE(mode="straight") {#M5::g %rt6};
*   &n1 @WIRE(mode="straight") {#R2::p #R1::n};
*   $JUNCTION @ATTACH %rt1 => @PLACE(24, 7);
*   $JUNCTION @ATTACH %rt2 => @PLACE(5, 14);
*   $JUNCTION @ATTACH %rt3 => @PLACE(20, 14);
*   $JUNCTION @ATTACH %rt4 => @PLACE(28, 14);
*   $JUNCTION @ATTACH %rt5 => @PLACE(36, 14);
*   $JUNCTION @ATTACH %rt6 => @PLACE(-3, 0)
* }
.end
