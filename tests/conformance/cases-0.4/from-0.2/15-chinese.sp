* Five-transistor OTA written in the Chinese style
*! RNL in Chinese style (2026-10-12, D16, D17): the header record loads the standard alias package rnl-zh with @USE-ALIASES (the header record itself in the standard spelling); keywords, parameter names, enumerated values,
*! the standard package and local definitions ($"差分对") and the role, parameter and local names of ordinary definitions (#M1(正输入), @附着(间距=3), %tc::"中点") are all written as Chinese aliases, and an alias of an enumerated value passes through a parameter into a definition's body (the direction of $"定向信号流");
*! one's own ordinary definitions use Chinese role names directly, unquoted; parameters by position come before those by name; the definition of an ATTACH is written before the keyword; after a full-width comma the `@` has a space before it (`， @`), and `=> 记作 %x` is still a name.
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
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4");
* @USE-ALIASES(name="alias-zh", version="0.4");
* @ALIAS("$差分对", "$LEGACY_DIFFPAIR");
* @ALIAS("$电流镜", "$LEGACY_MIRROR");
* @ALIAS("$输入级", "$LEGACY_INPUT_STAGE");
* @ALIAS("$镜像对", "$LEGACY_MIRROR_PAIR");
* @ALIAS("$尾管居中", "$LEGACY_TAIL_CENTER");
* @ALIAS("$定向信号流", "$LEGACY_FLOW");
* @ALIAS("正输入", "in_p", definition="$LEGACY_DIFFPAIR");
* @ALIAS("负输入", "in_n", definition="$LEGACY_DIFFPAIR");
* @ALIAS("参考管", "ref", definition="$LEGACY_MIRROR");
* @ALIAS("输出管", "out", definition="$LEGACY_MIRROR");
* @ALIAS("输入对", "pair", definition="$LEGACY_INPUT_STAGE");
* @ALIAS("尾管", "tail", definition="$LEGACY_INPUT_STAGE");
* @ALIAS("甲", "a", definition="$LEGACY_MIRROR_PAIR");
* @ALIAS("乙", "b", definition="$LEGACY_MIRROR_PAIR");
* @ALIAS("轴", "axis", definition="$LEGACY_MIRROR_PAIR");
* @ALIAS("甲", "a", definition="$LEGACY_TAIL_CENTER");
* @ALIAS("乙", "b", definition="$LEGACY_TAIL_CENTER");
* @ALIAS("尾端", "t", definition="$LEGACY_TAIL_CENTER");
* @ALIAS("中点", "mid", definition="$LEGACY_TAIL_CENTER");
* @ALIAS("方向", "direction", definition="$LEGACY_FLOW");
* }
* @RNL {
*   $LEGACY_DIFFPAIR = @DEFINE(roles=[["in_p", "instance"], ["in_n", "instance"]]) {};
*   $LEGACY_MIRROR = @DEFINE(roles=[["ref", "instance"], ["out", "instance"]]) {};
*   $LEGACY_INPUT_STAGE = @DEFINE(roles=[["pair", "usage"], ["tail", "instance"]]) {};
*   $LEGACY_MIRROR_PAIR = @DEFINE(roles=["a", "b", "axis"]) { {?a ?b} @ALIGN("y"); {?a ?axis ?b} @DISTRIBUTE("left-to-right"); ?a ?b @ORIENT-RELATION("MY") };
*   $LEGACY_TAIL_CENTER = @DEFINE(roles=["a", "b", "t"]) { %mid = (_, _); {?a %mid ?b} @DISTRIBUTE("left-to-right"); {%mid ?t} @ALIGN("x") };
*   $LEGACY_FLOW = @DEFINE(roles=["{path}"], parameters=[["direction", "left-to-right"]]) { ?path @ORDER(?direction) };
*   从本层 # 及以下每一层的元件中 @选出元件(类型="nmos") => 其 @使用符号 $NMOS;
*   从本层 # 及以下每一层的元件中 @选出元件(类型="pmos") => 其 @使用符号 $PMOS;
*   &vdd @加入 $"电源网络集";
*   &vss @加入 $"地网络集";
*
*   语义：#M1(正输入) 和 #M2(负输入) @视作 $"差分对" => 记作 %dp;
*   #M3(参考管) 和 #M4(输出管) @视作 $"电流镜";
*   %dp(输入对) 和 #M5(尾管) @视作 $"输入级";
*   从本层 # 及以下每一层的用例中 @选出用例 $"差分对" => 从其各级成员中 @选出元件 => 其 @使用样式 $"信号";
*
*   摆法：%ax = (0, _);
*   #M1(甲)、#M2(乙) 和 %ax(轴) @视作 $"镜像对";
*   #M3(甲)、#M4(乙) 和 %ax(轴) @视作 $"镜像对";
*   %tc = #M1(甲)、#M2(乙) 和 #M5(尾端) @视作 $"尾管居中";
*   {#M1 #M2 #CL} @视作(方向="从左到右") $"定向信号流"，即信号从左往右走;
*   {#M1 #M3} @对齐("x")，即 x 坐标相同;
*   {#M1 #M2} @对齐("y")，即 y 坐标相同;
*   {#M5 #M1 #M3} 按 @顺序("从下到上") 排列;
*   #M5 到 #M1 有 @距离("从下到上", 最小值=4, 最大值=8);
*   #M3 到 #M4 的 @朝向关系("MY")，即左右镜像;
*   {%jc #CL} @对齐("x", 优先级="低");
*   %core = {#M1 #M2 #M3 #M4 #M5} 视作一个 @分组;
*
*   偏置：$"偏置链" = @定义(角色表=["{电阻}"]) {
*     ?"电阻" 的 x 坐标 @等距("从左到右");
*     %"一行" = ?"电阻" @对齐("y")
*   };
*   %bc = {#R1 #R2 #R3}(电阻) @视作 $"偏置链";
*   %bc::"一行" 的 @优先级("高");
*   {#R1 #R2 #R3} 按 @顺序("从左到右") 排列;
*   $"橙色" = @定义样式(线色="#e07000", 字色="#e07000") {};
*   {#R1 #R2 #R3} @使用样式 $"橙色";
*   尾管的漏极 #M5:1 有 @电流("流入");
*
*   连线：%jd = (-6, 4); %jg = (-2, 8); %jo = (6, 4); %jc = (16, 4); %jt = (0, -3); %jb = (-12, -12);
*   网络 &x 的 @导线(mode="auto") 连接了 {#M3:1 %jd #M1:1};
*   网络 &x 的 @导线(mode="auto") 连接了 {#M3:2 %jg #M4:2};
*   网络 &x 的 @导线(模式="auto") 连接了 {%jd (-2, 4) %jg};
*   网络 &out 的 @导线(mode="auto") 连接了 {#M4:1 %jo #M2:1};
*   网络 &tail 的 @导线(模式="auto") 连接了 {#M1:3 (-6, -3) %jt};
*   网络 &tail 的 @导线(模式="auto") 连接了 {#M2:3 (6, -3) %jt};
*   网络 &tail 的 @导线(mode="auto") 连接了 {%jt #M5:1};
*   网络 &out 的 @导线(mode="auto") 连接了 {%jo %jc %po:p};
*   网络 &out 的 @导线(mode="auto") 连接了 {%jc #CL:1};
*   网络 &n1 的 @导线(mode="auto") 连接了 {#R1:2 #R2:1};
*   网络 &bias 的 @导线(mode="auto") 连接了 {#R2:2 %jb #R3:1};
*   网络 &bias 的 @导线(模式="auto") 连接了 {%jb (-12, -6) #M5:2};
*   $"交汇点" @附着 %jd => @位于(-6, 4); $"交汇点" @附着 %jg => @位于(-2, 8); $"交汇点" @附着 %jo => @位于(6, 4); $"交汇点" @附着 %jc => @位于(16, 4); $"交汇点" @附着 %jt => @位于(0, -3); $"交汇点" @附着 %jb => @位于(-12, -12);
*
*   端口：%pi = $"网名标签" @附着 在 &inp 上 => @朝向("MY");
*   $"网名标签" @附着 在 &inn 上 => 其 @朝向("R0")，即左右镜像 => 记作 %pn;
*   $"网名标签" @附着 在 &out 上 => 其 @朝向("R0") => 记作 %po;
*   网络 &inp 的 @导线(mode="auto") 连接了 {%pi:p #M1:2};
*   网络 &inn 的 @导线(mode="auto") 连接了 {%pn:p #M2:2};
*
*   地和电源：$"地" @附着 在 &vss 上 => 其 @位于(16, -4) => 记作 %g0;
*   网络 &vss 的 @导线(mode="auto") 连接了 {#CL:2 %g0:p};
*   %gr = $"接地桩" @附着(间距=3) 在 #R3:2 上;
*
*   文字：#M1 上有 @文本("W/L = ${w}/${l}", 引用名="wl", dx=-9, dy=-4);
*   在 (-28, 18) 有 @文本("五管 OTA：偏置由 R1、R2、R3 分压给出", 字号=1.1) => 记作 %title;
*   以 (-3, -1) 为中心的 @矩形(宽度=62, 高度=34, 填充="填充色", 填充色="#1f4fd80d", 线型="点线") => 其处于 @图层("底")
* }
* 

* @RNL(type="generated", priority="high") {
*   #M1 @PLACE(-6,0) => @ORIENT("R0");
*   #M2 @PLACE(6,0) => @ORIENT("MY");
*   #M3 @PLACE(-6,8) => @ORIENT("MY");
*   #M4 @PLACE(6,8) => @ORIENT("R0");
*   #M5 @PLACE(0,-6);
*   #CL @PLACE(16,1);
*   #R1 @PLACE(-24,-12) => @ORIENT("R90");
*   #R2 @PLACE(-16,-12) => @ORIENT("R90");
*   #R3 @PLACE(-8,-12) => @ORIENT("R90");
*   %ax @PLACE(y=3);
*   %tc @PLACE(0,0);
*   %tc::"中点" @位于(0,0);
*   %pi @PLACE(-11,0);
*   %pn @PLACE(11,0);
*   %po @PLACE(21,4);
*   %gr::g @PLACE(-5,-15);
*   %gm5 = $GROUND @ATTACH #M5:3::net => @PLACE(0,-10); #M5:3::net @WIRE(mode="auto") {#M5:3 %gm5:p};
*   %sm3 = $SUPPLY @ATTACH #M3:3::net => @PLACE(-6,12); #M3:3::net @WIRE(mode="auto") {#M3:3 %sm3:p};
*   %sm4 = $SUPPLY @ATTACH #M4:3::net => @PLACE(6,12); #M4:3::net @WIRE(mode="auto") {#M4:3 %sm4:p};
*   %sr1 = $SUPPLY @ATTACH #R1:1::net => @PLACE(-27,-10); #R1:1::net @WIRE(mode="auto") {#R1:1 %sr1:p};
* }
.end
