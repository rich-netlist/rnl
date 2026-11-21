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
*   /* 结构：差分对、电流镜、放大器和三条支路；电阻串单独成一列，自成一个分组 */
*   %dp1 = #M1(左)、#M2(右) 和 #M5(尾管) @视作 $"N型差分对";
*   %cm1 = #M3(参考管) 和 #M4(输出管) @视作 $"P型电流镜";
*   %amp = %dp1(跨导级) 和 %cm1(负载) @视作 $"跨导阻抗放大器";
*   %left = 把 {#M3 #M1} @视作 $"支路";
*   %right = 把 {#M4 #M2} @视作 $"支路";
*   %bias = {#R1 #R2 #R3} @视作 $"支路" => 视作一个 @分组;
*   /* 电源和地，电流方向，朝向和布线 */
*   &vdd @加入 $"电源网络集";
*   &vss @加入 $"地网络集";
*   {#M1:d #M2:d #M3:s #M4:s #M5:d} 有 @电流("流入");
*   {#M1:s #M2:s #M3:d #M4:d #M5:s} 有 @电流("流出");
*   #CL:1 的 @朝向("上");
*   从顶层 # @选出网络 => @布线网络;
*   /* 位置关系：从左到右依次是电阻串、放大器、负载电容 */
*   %bias 到 %amp 的 @距离("从左到右", 最小值=4);
*   %amp 到 #CL 的 @距离("从左到右", 最小值=4);
*   /* 样式：电阻串用橙色，差分对用信号色 */
*   $"橙色" = @定义样式(线色="#e07000", 字色="#e07000") {};
*   {#R1 #R2 #R3} @使用样式 $"橙色";
*   {#M1 #M2} @使用样式 $"信号";
*   /* 标注：输出的网络标签、M1 的 W/L、标题和底层的框；框以 M1 为中心，标题在框上方、与框左右对齐 */
*   $"网络标签" @附着 在 &out 上 => @朝向("右");
*   #M1 上有 @文本("W/L = ${w}/${l}", 引用名="wl", dx=-9, dy=-4);
*   在 (_,_) 有 @文本("五管 OTA：偏置由 R1、R2、R3 分压给出", 字号=1.1, 横向对齐="中") => 记作 %title;
*   以 #M1::position 为中心的 @矩形(宽度=50, 高度=46, 填充="填充色", 填充色="#1f4fd80d", 线型="点线") => 其处于 @图层("底") => 记作 %box;
*   %title 和 %box @对齐("x");
*   %title 到 %box 的 @距离("从上到下", 最小值=25)
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:4f3a6c9c20562f46.6f545a8f6337a726", numbering="0ebfb596d73fe6e7") {
*   %ground_stub_M5_s = $GROUND_STUB @ATTACH(gap=3) #M5::s;
*   %ground_stub_R3_n = $GROUND_STUB @ATTACH(gap=3) #R3::n;
*   %ground_stub_CL_n = $GROUND_STUB @ATTACH(gap=3) #CL::n;
*   %supply_stub_M3_s = $SUPPLY_STUB @ATTACH(gap=3) #M3::s;
*   %supply_stub_M4_s = $SUPPLY_STUB @ATTACH(gap=3) #M4::s;
*   %supply_stub_R1_p = $SUPPLY_STUB @ATTACH(gap=3) #R1::p;
*   %net_label_stub_M1_g = $NET_LABEL_STUB @ATTACH #M1::g;
*   %net_label_stub_M2_g = $NET_LABEL_STUB @ATTACH #M2::g;
*   %rt1 = (4, -3);
*   %rt2 = (4, 11);
*   %rt3 = (0, 8);
*   %rt4 = (8, 6);
*   %rt5 = (8, 3);
*   %rt6 = (-14, -10);
*   #M1 @PLACE(0, 0) => @ORIENT("R0");
*   #M2 @PLACE(8, 0) => @ORIENT("MY");
*   #M3 @PLACE(0, 11) => @ORIENT("MY");
*   #M4 @PLACE(8, 11) => @ORIENT("R0");
*   #M5 @PLACE(4, -10) => @ORIENT("R0");
*   #R1 @PLACE(-14, 11) => @ORIENT("MY");
*   #R2 @PLACE(-14, -7) => @ORIENT("MY");
*   #R3 @PLACE(-14, -13) => @ORIENT("MY");
*   #CL @PLACE(17, 0) => @ORIENT("R0");
*   %dp1 @PLACE(4, 0);
*   %#1 @PLACE(0, 0);
*   %#2 @PLACE(4, 0);
*   %cm1 @PLACE(0, 11);
*   %amp @PLACE(0, 0);
*   %left @PLACE(0, 0);
*   %#3 @PLACE(0, 0);
*   %right @PLACE(8, 0);
*   %#4 @PLACE(0, 0);
*   %#5 @PLACE(-14, 0);
*   %#6 @PLACE(0, 0);
*   %#7 @PLACE(9, 6) => @ORIENT("R180");
*   %ground_stub_M5_s @PLACE(0, 0);
*   %ground_stub_M5_s::g @PLACE(4, -14) => @ORIENT("R0");
*   %ground_stub_R3_n @PLACE(0, 0);
*   %ground_stub_R3_n::g @PLACE(-14, -17) => @ORIENT("R0");
*   %ground_stub_CL_n @PLACE(0, 0);
*   %ground_stub_CL_n::g @PLACE(17, -4) => @ORIENT("R0");
*   %supply_stub_M3_s @PLACE(0, 0);
*   %supply_stub_M3_s::s @PLACE(0, 15) => @ORIENT("R0");
*   %supply_stub_M4_s @PLACE(0, 0);
*   %supply_stub_M4_s::s @PLACE(8, 15) => @ORIENT("R0");
*   %supply_stub_R1_p @PLACE(0, 0);
*   %supply_stub_R1_p::s @PLACE(-14, 15) => @ORIENT("R0");
*   %net_label_stub_M1_g @PLACE(0, 0);
*   %net_label_stub_M1_g::l @PLACE(-3, 0) => @ORIENT("R0");
*   %net_label_stub_M2_g @PLACE(0, 0);
*   %net_label_stub_M2_g::l @PLACE(11, 0) => @ORIENT("R180");
*   %#8 @PLACE(0, 0);
*   %title @PLACE(0, 25);
*   &tail @WIRE(mode="straight") {#M1::s (0, -3) %rt1};
*   &tail @WIRE(mode="straight") {%rt1 (8, -3) #M2::s};
*   &tail @WIRE(mode="straight") {#M5::d %rt1};
*   &x @WIRE(mode="straight") {#M3::g %rt2};
*   &x @WIRE(mode="straight") {%rt2 #M4::g};
*   &x @WIRE(mode="straight") {#M3::d %rt3};
*   &x @WIRE(mode="straight") {%rt3 (4, 8) %rt2};
*   &x @WIRE(mode="straight") {#M1::d %rt3};
*   &out @WIRE(mode="straight") {#M4::d %rt4};
*   &out @WIRE(mode="straight") {%rt4 %#7::pin};
*   &out @WIRE(mode="straight") {#M2::d %rt5};
*   &out @WIRE(mode="straight") {%rt5 %rt4};
*   &out @WIRE(mode="straight") {#CL::p (17, 3) %rt5};
*   &bias @WIRE(mode="straight") {#R3::p %rt6};
*   &bias @WIRE(mode="straight") {%rt6 #R2::n};
*   &bias @WIRE(mode="straight") {#M5::g %rt6};
*   &n1 @WIRE(mode="straight") {#R2::p #R1::n};
*   $JUNCTION @ATTACH %rt1 => @PLACE(4, -3);
*   $JUNCTION @ATTACH %rt2 => @PLACE(4, 11);
*   $JUNCTION @ATTACH %rt3 => @PLACE(0, 8);
*   $JUNCTION @ATTACH %rt4 => @PLACE(8, 6);
*   $JUNCTION @ATTACH %rt5 => @PLACE(8, 3);
*   $JUNCTION @ATTACH %rt6 => @PLACE(-14, -10)
* }
.end
