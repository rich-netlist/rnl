* 4-bit register: subscripts, buses, bus pins and stacks
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-basic", version="0.4"); @VERSION("0.4")}
*! Indices: #XI0<3:0>, &Q<3:0>, #XR:Q<3>, #RL<3:0>, a step and mixed &Q<0:3:2, 1>; a bus wire takes a set of nets, and ${net} joins them with commas;
*! a bus pin of a block symbol (terminal= takes an array); taps sharing a point with the bus; BUNDLE drawing the iterated instances as one, wired to the bundle's terminals; the standard package draws buses thicker.
.subckt dff d clk q
* @RNL { %dff_ports = # @SELECT-NET(port=true); $IO_PORT @ATTACH %dff_ports }
* @RNL(type="generated", priority="high") { &d @SELECT-ANCESTOR-USAGE $IO_PORT => @PLACE(-2, 1); &clk @SELECT-ANCESTOR-USAGE $IO_PORT => @PLACE(-2, -1); &q @SELECT-ANCESTOR-USAGE $IO_PORT => @PLACE(2, 1) => @ORIENT("MY") }
* 

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
*     %data_in = (-3, 1) @PIN(terminal="d", label="D", direction="left"); (-3, -1) @PIN(terminal="clk", label="C", direction="left");
*     %data_out = (3, 1) @PIN(terminal="q", label="Q", direction="right");
*     @RECTANGLE {(-2, -2) (2, 2)};
*     @LINE {(-3, 1) (-2, 1)}; @LINE {(-3, -1) (-2, -1)}; @LINE {(2, 1) (3, 1)};
*     ? @TEXT("${name}", key="name") => @PLACE(-2, 2.25)
*   };
*   #XI0<3:0> @BUNDLE => @SET-SYMBOL $DFF => %st;
*   %st @TEXT("XI0<3:0>", key="name");
*   %st @TEXT("${model}", key="model", dx=-2, dy=-2.8);
*   %port_nets = # @SELECT-NET(port=true); {&D<3:0> &Q<3:0>} @REMOVE-FROM %port_nets; $IO_PORT @ATTACH %port_nets;
*   %dp = (-7, 1); %qp = (7, 1);
*   %bd = &D<3:0> @WIRE(mode="auto") {%dp %st::data_in};
*   %bq = &Q<3:0> @WIRE(mode="auto") {%st::data_out %qp};
*   %dp @TEXT("D<3:0>", halign="end", dx=-0.3);
*   %qp @TEXT("Q<3:0>", dx=0.3)
* }
* 

* @RNL(type="generated", priority="high") { %st @PLACE(0, 0); &clk @SELECT-ANCESTOR-USAGE $IO_PORT => @PLACE(-3, -1); %net_label_stub_st_clk = $NET_LABEL_STUB @ATTACH %st:clk; %net_label_stub_st_clk @PLACE(0, 0); %net_label_stub_st_clk::l @PLACE(-3, -1) => @ORIENT("R180") }
.ends
XR D<3> D<2> D<1> D<0> clk Q<3> Q<2> Q<1> Q<0> reg4
RL<3> Q<3> 0 10k
RL<2> Q<2> 0 10k
RL<1> Q<1> 0 10k
RL<0> Q<0> 0 10k
V1 clk 0 PULSE(0 1.8 0 1n 1n 5n 10n)
* @RNL {
*   %t3 = (10, 1); %t2 = (14, 1); %t1 = (18, 1); %t0 = (22, 1);
*   %bus = &Q<3:0> @WIRE(mode="auto") {#XR:Q<3> %t3 %t2 %t1 %t0};
*   %bus @TEXT("${net}", size=0.7);
*   &Q<3> @WIRE(mode="auto") {%t3 #RL<3>:1};
*   &Q<2> @WIRE(mode="auto") {%t2 #RL<2>:1};
*   &Q<1> @WIRE(mode="auto") {%t1 #RL<1>:1};
*   &Q<0> @WIRE(mode="auto") {%t0 #RL<0>:1};
*   $JUNCTION @ATTACH %t3 => @PLACE(10, 1); $JUNCTION @ATTACH %t2 => @PLACE(14, 1); $JUNCTION @ATTACH %t1 => @PLACE(18, 1);
*   {#RL<3:0>} @ALIGN("y");
* #RL<3:0> @DISTRIBUTE("left-to-right");
*   &Q<0:3:2, 1> @SELECT-TERMINAL(name="R*", recursive=true) => @SELECT-PIN(recursive=true) => @TEXT("${net}", key="even", dx=1, dy=-5, size=0.7);
*   %dpt = (-8, 1);
*   &D<3:0> @WIRE(mode="auto") {%dpt #XR:D<0>};
*   %dpt @TEXT("D<3:0>", halign="end", dx=-0.3);
*   &clk @WIRE(mode="auto") {#V1:1 (-8, -1) #XR:clk}
* }
* 

* @RNL(type="generated", priority="high") {
*   #XR @PLACE(0, 0);
*   #V1 @PLACE(-8, -5);
*   #RL<3> @PLACE(10, -3);
*   #RL<2> @PLACE(14, -3);
*   #RL<1> @PLACE(18, -3);
*   #RL<0> @PLACE(22, -3);
*   %ground3 = $GROUND @ATTACH #RL<3>:2::net => @PLACE(10, -8); #RL<3>:2::net @WIRE(mode="auto") {#RL<3>:2 %ground3:p};
*   %ground2 = $GROUND @ATTACH #RL<2>:2::net => @PLACE(14, -8); #RL<2>:2::net @WIRE(mode="auto") {#RL<2>:2 %ground2:p};
*   %ground1 = $GROUND @ATTACH #RL<1>:2::net => @PLACE(18, -8); #RL<1>:2::net @WIRE(mode="auto") {#RL<1>:2 %ground1:p};
*   %ground0 = $GROUND @ATTACH #RL<0>:2::net => @PLACE(22, -8); #RL<0>:2::net @WIRE(mode="auto") {#RL<0>:2 %ground0:p};
*   %ground_v1 = $GROUND @ATTACH #V1:2::net => @PLACE(-8, -10); #V1:2::net @WIRE(mode="auto") {#V1:2 %ground_v1:p};
* }
.end
