* 4-bit register: subscripts, buses, bus pins and stacks
* @RNL(type="header") { @VERSION("0.2")}
*! 下标：#XI0<3:0>、&Q<3:0>、#XR:Q<3>、#RL<3:0>、步长与混写 &Q<0:3:2, 1>；总线导线收网络的集合，${net} 用逗号连起来；
*! block symbol 的总线引脚（terminal= 收数组）；分接线与总线共用点；合画 BUNDLE 把迭代实例画成一个，接到合画的端子；标准包给总线加粗。
.subckt dff d clk q
* @RNL(type="generated", priority="high") {
*   &d @SELECT-ANCESTOR-USAGE $IOPORT => @PLACE(-2, 1);
*   &clk @SELECT-ANCESTOR-USAGE $IOPORT => @PLACE(-2, -1);
*   &q @SELECT-ANCESTOR-USAGE $IOPORT => @PLACE(2, 1) => @ORIENT("MY")
* }
.ends
.subckt reg4 D<3> D<2> D<1> D<0> clk Q<3> Q<2> Q<1> Q<0>
XI0<3> D<3> clk Q<3> dff
XI0<2> D<2> clk Q<2> dff
XI0<1> D<1> clk Q<1> dff
XI0<0> D<0> clk Q<0> dff
* @RNL {
*   %blk = @DEFINE-SYMBOL {
*     (-4, 1) @PIN(terminal=["D<3>", "D<2>", "D<1>", "D<0>"], label="D<3:0>", direction="left");
*     (-4, -1) @PIN(terminal="clk", label="clk", direction="left");
*     (4, 1) @PIN(terminal=["Q<3>", "Q<2>", "Q<1>", "Q<0>"], label="Q<3:0>", direction="right");
*     @RECTANGLE {(-3, -3) (3, 3)};
*     @LINE(linestyle="bold") {(-4, 1) (-3, 1)}; @LINE {(-4, -1) (-3, -1)}; @LINE(linestyle="bold") {(3, 1) (4, 1)};
*     ? @TEXT("${name}", key="name") => @PLACE(-3, 3.25)
*   };
*   # @SET-SYMBOL %blk;
*   $DFF = @DEFINE-SYMBOL {
*     (-3, 1) @PIN(terminal="d", label="D", direction="left"); (-3, -1) @PIN(terminal="clk", label="C", direction="left");
*     (3, 1) @PIN(terminal="q", label="Q", direction="right");
*     @RECTANGLE {(-2, -2) (2, 2)};
*     @LINE {(-3, 1) (-2, 1)}; @LINE {(-3, -1) (-2, -1)}; @LINE {(2, 1) (3, 1)};
*     ? @TEXT("${name}", key="name") => @PLACE(-2, 2.25)
*   };
*   #XI0<3:0> @BUNDLE => @SET-SYMBOL $DFF => %st;
*   %st @TEXT("XI0<3:0>", key="name");
*   %st @TEXT("${model}", key="model", dx=-2, dy=-2.8);
*   {&D<3:0> &Q<3:0>} @REMOVE-FROM $PORT_FILL_SET;
*   %dp = (-7, 1); %qp = (7, 1);
*   %bd = &D<3:0> @WIRE {%dp %st:d};
*   %bq = &Q<3:0> @WIRE {%st:q %qp};
*   %dp @TEXT("D<3:0>", halign="end", dx=-0.3);
*   %qp @TEXT("Q<3:0>", dx=0.3)
* }
* @RNL(type="generated", priority="high") {
*   %st @PLACE(0, 0);
*   &clk @SELECT-ANCESTOR-USAGE $IOPORT => @PLACE(-3, -1)
* }
.ends
XR D<3> D<2> D<1> D<0> clk Q<3> Q<2> Q<1> Q<0> reg4
RL<3> Q<3> 0 10k
RL<2> Q<2> 0 10k
RL<1> Q<1> 0 10k
RL<0> Q<0> 0 10k
V1 clk 0 PULSE(0 1.8 0 1n 1n 5n 10n)
* @RNL {
*   %t3 = (10, 1); %t2 = (14, 1); %t1 = (18, 1); %t0 = (22, 1);
*   %bus = &Q<3:0> @WIRE {#XR:Q<3> %t3 %t2 %t1 %t0};
*   %bus @TEXT("${net}", size=0.7);
*   &Q<3> @WIRE {%t3 #RL<3>:1};
*   &Q<2> @WIRE {%t2 #RL<2>:1};
*   &Q<1> @WIRE {%t1 #RL<1>:1};
*   &Q<0> @WIRE {%t0 #RL<0>:1};
*   $JUNCTION @ATTACH {%t3 %t2 %t1};
*   {#RL<3:0>} @ALIGN("y");
*   #RL<3:0> @DISTRIBUTE("x");
*   &Q<0:3:2, 1> @SELECT-TERMINAL(name="R*") => @TEXT("${net}", key="even", dx=1, dy=-5, size=0.7);
*   %dpt = (-8, 1);
*   &D<3:0> @WIRE {%dpt #XR:D<0>};
*   %dpt @TEXT("D<3:0>", halign="end", dx=-0.3);
*   &clk @WIRE(mode="vertical-horizontal") {#V1:1 #XR:clk}
* }
* @RNL(type="generated", priority="high") {
*   #XR @PLACE(0, 0); #V1 @PLACE(-8, -5);
*   #RL<3> @PLACE(10, -3); #RL<2> @PLACE(14, -3); #RL<1> @PLACE(18, -3); #RL<0> @PLACE(22, -3);
*   %gs_RL3_2 = $GND_STUB @ATTACH #RL<3>:2; %gs_RL3_2::g @PLACE(10, -8);
*   %gs_RL2_2 = $GND_STUB @ATTACH #RL<2>:2; %gs_RL2_2::g @PLACE(14, -8);
*   %gs_RL1_2 = $GND_STUB @ATTACH #RL<1>:2; %gs_RL1_2::g @PLACE(18, -8);
*   %gs_RL0_2 = $GND_STUB @ATTACH #RL<0>:2; %gs_RL0_2::g @PLACE(22, -8);
*   %gs_V1_2 = $GND_STUB @ATTACH #V1:2; %gs_V1_2::g @PLACE(-8, -10)
* }
.end
