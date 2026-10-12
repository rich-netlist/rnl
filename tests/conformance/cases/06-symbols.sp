* Symbols: primitives, variants, usages inside symbols, eight orientations
*! 符号：片段（%）与变体（并入后按端子覆盖引脚、图元累加）；全部图元（两种写法、填充、线端箭头、闭合折线）；固定文字与 `?` 上带键的文字；
*! 体里画在点上的用例（箭头），嵌套的用例随符号合成姿态；同一个不对称符号的八种姿态，检查文字始终正读（9.2.5）。
R1 a1 b1 1k
R2 a2 b2 1k
R3 a3 b3 1k
R4 a4 b4 1k
R5 a5 b5 1k
R6 a6 b6 1k
R7 a7 b7 1k
R8 a8 b8 1k
D1 k1 k2 led
D2 k3 k4 led
X1 s1 s2 shapes
.model led d
.subckt shapes a b
.ends
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   %BODY = @DEFINE-SYMBOL {
*     (-3, 0) @PIN(terminal=1, label="a", direction="left"); (3, 0) @PIN(terminal=2, label="b", direction="right");
*     @LINE {(-3, 0) (-1.5, 0)}; @LINE {(1.5, 0) (3, 0)};
*     @RECTANGLE {(-1.5, -0.6) (1.5, 0.6)}
*   };
*   %FLAG = @DEFINE-SYMBOL { @LINE(fill="linecolor") {(1.5, 0.6) (1.5, 1.8) (2.5, 1.2) (1.5, 0.6)}};
*   $ASYM = @DEFINE-SYMBOL {
*     %BODY; %FLAG;
*     (-1, 1.2) @TEXT("F", size=0.8);
*     ? @TEXT("${name}", key="name", halign="middle", valign="top") => @PLACE(0, -1);
*     ? @TEXT("rot=90", key="side", rotate=90, size=0.6) => @PLACE(-2.2, 0.6)
*   };
*   $ASYM_DOT = @DEFINE-SYMBOL { $ASYM; (0, 0) @CIRCLE(radius=0.3, fill="linecolor")};
*   $ASYM_SHORT = @DEFINE-SYMBOL { $ASYM; (-2, 0) @PIN(terminal=1, label="a'", direction="left")};
*   {#R1 #R2 #R3 #R4 #R5 #R6} @SET-SYMBOL $ASYM;
*   #R7 @SET-SYMBOL $ASYM_DOT;
*   #R8 @SET-SYMBOL $ASYM_SHORT;
*   #R1 @TEXT("R0", key="code", dx=-1, dy=3.5);
*   #R2 @TEXT("R90", key="code", dx=-1, dy=3.5);
*   #R3 @TEXT("R180", key="code", dx=-1, dy=3.5);
*   #R4 @TEXT("R270", key="code", dx=-1, dy=3.5);
*   #R5 @TEXT("MX", key="code", dx=-1, dy=3.5);
*   #R6 @TEXT("MY", key="code", dx=-1, dy=3.5);
*   #R7 @TEXT("MXR90\\变体：加圆点", key="code", dx=-1, dy=4.6);
*   #R8 @TEXT("MYR90\\变体：挪引脚 1", key="code", dx=-1, dy=4.6);
*
*   $ARROW_LINE = @DEFINE-SYMBOL {
*     @LINE {(-0.9, 0) (0.1, 0)};
*     $ARROW @ATTACH (0.1, 0) => @SET-SYMBOL $ARROW_HEAD
*   };
*   $LED = @DEFINE-SYMBOL {
*     $DIODE;
*     $ARROW @ATTACH (-1.4, 0.6) => @SET-SYMBOL $ARROW_LINE => @ORIENT("R180");
*     $ARROW @ATTACH (-1.4, -0.2) => @SET-SYMBOL $ARROW_LINE => @ORIENT("R180")
*   };
*   {#D1 #D2} @SET-SYMBOL $LED;
*   #D2 @TEXT("LED R90", key="code", dx=-2, dy=3);
*   #D1 @TEXT("LED R0", key="code", dx=-2, dy=3.5);
*
*   $SHAPES = @DEFINE-SYMBOL(box=[-9, -5, 9, 5]) {
*     (-9, 0) @PIN(terminal="a", direction="left"); (9, 0) @PIN(terminal="b", direction="right");
*     @LINE {(-9, 0) (-8, 0)}; @LINE {(8, 0) (9, 0)};
*     @RECTANGLE {(-8, -5) (8, 5)};
*     (-5.5, 2.3) @RECTANGLE(width=3, height=2, fill="fillcolor", linestyle="dashed");
*     @CIRCLE {(-1, 2.3) (0, 2.3)};
*     (2.5, 2.3) @CIRCLE(radius=1, fill="linecolor");
*     @ELLIPSE {(4.5, 1.3) (7.5, 3.3)};
*     (-5.5, -2.5) @ELLIPSE(width=3, height=1.5, fill="linecolor");
*     @ARC(arrow="end") {(-2, -3.5) (-0.5, -1.5) (1, -3.5)};
*     @BEZIER(arrow="start") {(2, -3.5) (3, -1) (5, -4) (7, -2)};
*     @LINE(fill="linecolor") {(-7.5, -4.5) (-6.5, -4.5) (-7, -3.8) (-7.5, -4.5)};
*     @LINE(arrow="both", linestyle="dotted") {(-8, -0.5) (8, -0.5)};
*     (0, 4.6) @TEXT("\textbf{SHAPES} x^{2} \alpha\\second line", halign="middle", valign="top", size=0.7);
*     ? @TEXT("${name} (${model})", key="name") => @PLACE(-8, 5.25)
*   };
*   #X1 @SET-SYMBOL $SHAPES
* }
* @RNL(type="generated", priority="high") {
*   #R1 @PLACE(0, 10); #R2 @PLACE(10, 10) => @ORIENT("R90"); #R3 @PLACE(20, 10) => @ORIENT("R180"); #R4 @PLACE(30, 10) => @ORIENT("R270");
*   #R5 @PLACE(0, 0) => @ORIENT("MX"); #R6 @PLACE(10, 0) => @ORIENT("MY"); #R7 @PLACE(20, 0) => @ORIENT("MXR90"); #R8 @PLACE(30, 0) => @ORIENT("MYR90");
*   #D1 @PLACE(42, 10); #D2 @PLACE(42, 0) => @ORIENT("R90");
*   #X1 @PLACE(12, -14)
* }
.end
