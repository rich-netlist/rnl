* Five-transistor OTA written in the Chinese style
*! RNL in Chinese style (2026-10-12, D16, D17): the header record loads the standard alias package rnl-zh with @USE-ALIASES (the header record itself in the standard spelling); keywords, parameter names, enumerated values,
*! the $ names of the standard package ($"差分对") and the role, parameter and local names of ordinary definitions (#M1(正输入), @附着(间距=3), %tc::"中点") are all written as Chinese aliases, and an alias of an enumerated value passes through a parameter into a definition's body (the direction of $"信号流向");
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
* @RNL(type="header") { @VERSION("0.2"); @USE-ALIASES(name="rnl-zh", version="0.2")}
* @RNL {
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
*   {#M1 #M2 #CL} @视作(方向="从左到右") $"信号流向"，即信号从左往右走;
*   {#M1 #M3} @对齐("x")，即 x 坐标相同;
*   {#M1 #M2} @对齐("y")，即 y 坐标相同;
*   {#M5 #M1 #M3} 按 @顺序("从下到上") 排列;
*   #M5 到 #M1 有 @距离("从下到上", 最小值=4, 最大值=8);
*   #M3 到 #M4 的 @朝向关系("MY")，即左右镜像;
*   {%jc #CL} @对齐("x", 优先级="低");
*   %core = {#M1 #M2 #M3 #M4 #M5} 视作一个 @分组;
*
*   偏置：$"偏置链" = @定义(角色表=["{电阻}"]) {
*     ?"电阻" 的 x 坐标 @等距("x");
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
*   网络 &x 的 @导线 连接了 {#M3:1 %jd #M1:1};
*   网络 &x 的 @导线 连接了 {#M3:2 %jg #M4:2};
*   网络 &x 的 @导线(模式="先横后竖") 连接了 {%jd %jg};
*   网络 &out 的 @导线 连接了 {#M4:1 %jo #M2:1};
*   网络 &tail 的 @导线(模式="先竖后横") 连接了 {#M1:3 %jt};
*   网络 &tail 的 @导线(模式="先竖后横") 连接了 {#M2:3 %jt};
*   网络 &tail 的 @导线 连接了 {%jt #M5:1};
*   网络 &out 的 @导线 连接了 {%jo %jc %po:p};
*   网络 &out 的 @导线 连接了 {%jc #CL:1};
*   网络 &n1 的 @导线 连接了 {#R1:2 #R2:1};
*   网络 &bias 的 @导线 连接了 {#R2:2 %jb #R3:1};
*   网络 &bias 的 @导线(模式="先竖后横") 连接了 {%jb #M5:2};
*   $"交汇点" @附着 在 {%jd %jg %jo %jc %jt %jb} 上;
*
*   端口：%pi = $"端口" @附着 在 &inp 上;
*   $"端口" @附着 在 &inn 上 => 其 @朝向("MY")，即左右镜像 => 记作 %pn;
*   $"端口" @附着 在 &out 上 => 其 @朝向("MY") => 记作 %po;
*   网络 &inp 的 @导线 连接了 {%pi:p #M1:2};
*   网络 &inn 的 @导线 连接了 {%pn:p #M2:2};
*
*   地和电源：$"地" @附着 在 &vss 上 => 其 @位于(16, -4) => 记作 %g0;
*   网络 &vss 的 @导线 连接了 {#CL:2 %g0:p};
*   %gr = $"接地桩" @附着(间距=3) 在 #R3:2 上;
*
*   文字：#M1 上有 @文本("W/L = ${w}/${l}", 引用名="wl", dx=-9, dy=-4);
*   在 (-28, 18) 有 @文本("五管 OTA：偏置由 R1、R2、R3 分压给出", 字号=1.1) => 记作 %title;
*   以 (-3, -1) 为中心的 @矩形(宽度=62, 高度=34, 填充="填充色", 填充色="#1f4fd80d", 线型="点线") => 其处于 @图层("底")
* }
* @RNL(type="generated", priority="high") {
*   /* the solver's record; a tool writes the standard form, here it is written with the aliases as well */
*   #M1 @位于(-6, 0) => @朝向("R0"); #M2 @位于(6, 0) => @朝向("MY");
*   #M3 @位于(-6, 8) => @朝向("MY"); #M4 @位于(6, 8) => @朝向("R0");
*   #M5 @位于(0, -6); #CL @位于(16, 1);
*   #R1 @位于(-24, -12) => @朝向("R90"); #R2 @位于(-16, -12) => @朝向("R90"); #R3 @位于(-8, -12) => @朝向("R90");
*   %ax @位于(y=3); %tc::"中点" @位于(0, 0);
*   %pi @位于(-11, 0); %pn @位于(11, 0); %po @位于(21, 4);
*   %gr::"地" @位于(-5, -15);
*   %gs_M5_3 = $"接地桩" @附着 #M5:3; %gs_M5_3::"地" @位于(0, -10);
*   %ss_M3_3 = $"电源桩" @附着 #M3:3; %ss_M3_3::"电源" @位于(-6, 12);
*   %ss_M4_3 = $"电源桩" @附着 #M4:3; %ss_M4_3::"电源" @位于(6, 12);
*   %ss_R1_1 = $"电源桩" @附着 #R1:1; %ss_R1_1::"电源" @位于(-27, -10)
* }
.end
