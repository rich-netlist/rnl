* Inverting amplifier: styles, layers, notes and rich text
* @RNL(type="header") { @VERSION("0.2")}
*! 样式：继承（{$base}）、直接指派盖过选择指派（与先后无关）、图元自己的参数最高、标签按项覆盖（只改颜色）；SET-SYMBOL 的直接与规则（匿名的嵌套定义，5.6）；
*! 导线的线型参数；LAYER 背景；场景里的说明图元（框、箭头、只填充不描边的 linestyle="none"）；富文本：上下标嵌套、粗斜体、上划线、希腊字母、分式、换行、转义、${x}，不认识的记号给警告；
*! 文字默认按基线对齐（锚点在第一行的基线上），另写 valign 的按文字框对齐；dx、dy 让文字跟着附着的点走。
.subckt opamp inp inn out
E1 out 0 inp inn 1e5
* @RNL(type="generated", priority="high") {
*   #E1 @PLACE(0, 0);
*   &inp @SELECT-ANCESTOR-USAGE $IOPORT => @PLACE(-4, 1);
*   &inn @SELECT-ANCESTOR-USAGE $IOPORT => @PLACE(-4, -1);
*   &out @SELECT-ANCESTOR-USAGE $IOPORT => @PLACE(0, 3) => @ORIENT("R270");
*   %gs_E1_2 = $GND_STUB @ATTACH #E1:2; %gs_E1_2::g @PLACE(0, -5)
* }
.ends
Vin in 0 AC 1
Rin in inn 10k
Rf inn out 100k
X1 0 inn out opamp
* @RNL {
*   $theme = @DEFINE-STYLE(linecolor="#37474f", textcolor="#37474f") {$base};
*   $hot = @DEFINE-STYLE(linecolor="#c62828", textcolor="#c62828") {};
*   #Rf @SET-STYLE $hot;
*   # @SELECT-INSTANCE => @SET-STYLE $theme;
*   # @SELECT-USAGE => @SET-STYLE $theme;
*   # @SELECT-WIRE => @SET-STYLE $theme;
*   # @SELECT-LABEL => @SET-STYLE $theme;
*   # @SELECT-ANNOTATION => @SET-STYLE $theme;
*   # @SELECT-INSTANCE(name="R*") => @SET-STYLE $grey;
*
*   #Rin @SET-SYMBOL { @DEFINE-SYMBOL {
*     (0, 3) @PIN(terminal=1, direction="up"); (0, -3) @PIN(terminal=2, direction="down");
*     (0, 0) @RECTANGLE(width=1.2, height=3.6); @LINE {(0, 3) (0, 1.8)}; @LINE {(0, -1.8) (0, -3)};
*     ? @TEXT("${name}", key="name") => @PLACE(1.1, 0.25);
*     ? @TEXT("${value}", key="value") => @PLACE(1.1, -0.95)
*   }};
*   # @SELECT-INSTANCE(name="R*") => @SET-SYMBOL $RES;
*   #X1 @SET-SYMBOL $OPAMP;
*   $blue_bold = @DEFINE-STYLE(textcolor="#1565c0", bold=true) {};
*   #Rf::name @SET-STYLE $blue_bold;
*   #Rin @TEXT("R_{in}", key="name");
*
*   %ji = (5.5, 1); %jo = (15, 0); %vo = (18, 0);
*   &in @WIRE(linestyle="double", linewidth=0.08) {#Vin:1 #Rin:1};
*   &inn @WIRE {#Rin:2 %ji #X1:2};
*   &inn @WIRE(mode="vertical-horizontal", linestyle="dash-dot") {%ji #Rf:1};
*   &out @WIRE(mode="horizontal-vertical") {#Rf:2 %jo};
*   &out @WIRE(linestyle="bold", linecolor="#2e7d32") {#X1:3 %jo %vo};
*   $JUNCTION @ATTACH {%ji %jo};
*   %vo @TEXT("V_{out}", dx=0.4, valign="middle");
*
*   (5, 0.5) @RECTANGLE(width=24, height=14, fill="fillcolor", fillcolor="#1f4fd81a", linecolor="#1f4fd8", linestyle="dotted") => @LAYER("bottom");
*   @LINE(arrow="end", linecolor="#c62828") {(17, 6) (15.3, 0.6)};
*   (16.6, 6.95) @CIRCLE(radius=0.25, fill="linecolor", linestyle="none", linecolor="#c62828");
*   (17.2, 6.6) @TEXT("A_{v} = −\frac{R_{f}}{R_{in}} = −10", textcolor="#c62828");
*   (-7, 9) @TEXT("\textbf{Inverting amplifier}\\\textit{V}_{\textit{DD}} = 1.8 V, I_{D_{1}} = 10 \mu A, \overline{RST} \pm 5\%");
*   (-7, -9) @TEXT("escapes, a\_b \$5 \${x} \{ \} C:\temp, unknown \sqrt{2}", size=0.8);
*   %p = (-7, -11);
*   %p @TEXT("this note is at (${x}, ${y})", size=0.8);
*   (-9, 0) @TEXT("rot=90 note", rotate=90, halign="middle", size=0.8)
* }
* @RNL(type="generated", priority="high") {
*   #Vin @PLACE(-4, -2); #Rin @PLACE(2, 1) => @ORIENT("R90"); #Rf @PLACE(10, 5) => @ORIENT("R90"); #X1 @PLACE(10, 0);
*   %gs_Vin_2 = $GND_STUB @ATTACH #Vin:2; %gs_Vin_2::g @PLACE(-4, -7);
*   %gs_X1_1 = $GND_STUB @ATTACH #X1:1; %gs_X1_1::g @PLACE(6, -3)
* }
.end
