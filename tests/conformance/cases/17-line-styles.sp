* Line styles: every style on every shape kind, in a symbol and on a wire
*! Each line style drawn on each graphic: rectangle, circle, ellipse, polyline, closed polyline, arc, Bezier curve; first thin lines (line width 0), then double, dashed and dotted lines with a line width,
*! with butt, miter, square and bevel ends and joins; line style none fills without a stroke. Double lines on rectangles, circles and ellipses are also drawn as two lines with the fill between them (before, only paths were);
*! a double-line box in a symbol, a double line given by an instance's style (the zigzag of the standard resistor), and a double-line wire.
R1 a c 1k
R2 a b 1k
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   (-3, 0) @TEXT("solid", halign="end", valign="middle");
*   (0, 0) @RECTANGLE(width=5, height=3); (8, 0) @CIRCLE(radius=1.5); (16, 0) @ELLIPSE(width=5, height=3);
*   @LINE {(21.5, -1.5) (24, 1.5) (26.5, -1.5)}; @LINE {(29.5, -1.5) (34.5, -1.5) (32, 1.5) (29.5, -1.5)};
*   @ARC {(37.5, -1) (40, 1.5) (42.5, -1)}; @BEZIER {(45.5, 0) (47, 3) (49, -3) (50.5, 0)};
*
*   (-3, -5) @TEXT("dashed", halign="end", valign="middle");
*   (0, -5) @RECTANGLE(width=5, height=3, linestyle="dashed"); (8, -5) @CIRCLE(radius=1.5, linestyle="dashed");
*   (16, -5) @ELLIPSE(width=5, height=3, linestyle="dashed"); @LINE(linestyle="dashed") {(21.5, -6.5) (24, -3.5) (26.5, -6.5)};
*   @LINE(linestyle="dashed") {(29.5, -6.5) (34.5, -6.5) (32, -3.5) (29.5, -6.5)};
*   @ARC(linestyle="dashed") {(37.5, -6) (40, -3.5) (42.5, -6)}; @BEZIER(linestyle="dashed") {(45.5, -5) (47, -2) (49, -8) (50.5, -5)};
*
*   (-3, -10) @TEXT("dotted", halign="end", valign="middle");
*   (0, -10) @RECTANGLE(width=5, height=3, linestyle="dotted"); (8, -10) @CIRCLE(radius=1.5, linestyle="dotted");
*   (16, -10) @ELLIPSE(width=5, height=3, linestyle="dotted"); @LINE(linestyle="dotted") {(21.5, -11.5) (24, -8.5) (26.5, -11.5)};
*   @LINE(linestyle="dotted") {(29.5, -11.5) (34.5, -11.5) (32, -8.5) (29.5, -11.5)};
*   @ARC(linestyle="dotted") {(37.5, -11) (40, -8.5) (42.5, -11)}; @BEZIER(linestyle="dotted") {(45.5, -10) (47, -7) (49, -13) (50.5, -10)};
*
*   (-3, -15) @TEXT("dash-dot", halign="end", valign="middle");
*   (0, -15) @RECTANGLE(width=5, height=3, linestyle="dash-dot"); (8, -15) @CIRCLE(radius=1.5, linestyle="dash-dot");
*   (16, -15) @ELLIPSE(width=5, height=3, linestyle="dash-dot"); @LINE(linestyle="dash-dot") {(21.5, -16.5) (24, -13.5) (26.5, -16.5)};
*   @LINE(linestyle="dash-dot") {(29.5, -16.5) (34.5, -16.5) (32, -13.5) (29.5, -16.5)};
*   @ARC(linestyle="dash-dot") {(37.5, -16) (40, -13.5) (42.5, -16)}; @BEZIER(linestyle="dash-dot") {(45.5, -15) (47, -12) (49, -18) (50.5, -15)};
*
*   (-3, -20) @TEXT("bold", halign="end", valign="middle");
*   (0, -20) @RECTANGLE(width=5, height=3, linestyle="bold"); (8, -20) @CIRCLE(radius=1.5, linestyle="bold");
*   (16, -20) @ELLIPSE(width=5, height=3, linestyle="bold"); @LINE(linestyle="bold") {(21.5, -21.5) (24, -18.5) (26.5, -21.5)};
*   @LINE(linestyle="bold") {(29.5, -21.5) (34.5, -21.5) (32, -18.5) (29.5, -21.5)};
*   @ARC(linestyle="bold") {(37.5, -21) (40, -18.5) (42.5, -21)}; @BEZIER(linestyle="bold") {(45.5, -20) (47, -17) (49, -23) (50.5, -20)};
*
*   (-3, -25) @TEXT("double", halign="end", valign="middle");
*   (0, -25) @RECTANGLE(width=5, height=3, linestyle="double"); (8, -25) @CIRCLE(radius=1.5, linestyle="double");
*   (16, -25) @ELLIPSE(width=5, height=3, linestyle="double"); @LINE(linestyle="double") {(21.5, -26.5) (24, -23.5) (26.5, -26.5)};
*   @LINE(linestyle="double") {(29.5, -26.5) (34.5, -26.5) (32, -23.5) (29.5, -26.5)};
*   @ARC(linestyle="double") {(37.5, -26) (40, -23.5) (42.5, -26)}; @BEZIER(linestyle="double") {(45.5, -25) (47, -22) (49, -28) (50.5, -25)};
*
*   $wide = @DEFINE-STYLE(linestyle="double", linewidth=0.25, linecolor="#1f4fd8", fillcolor="#ffe0b2") {};
*   (-3, -30) @TEXT("double 0.25", halign="end", valign="middle");
*   {(0, -30) @RECTANGLE(width=5, height=3, fill="fillcolor"); (8, -30) @CIRCLE(radius=1.5, fill="fillcolor");
*    (16, -30) @ELLIPSE(width=5, height=3, fill="fillcolor"); @LINE {(21.5, -31.5) (24, -28.5) (26.5, -31.5)};
*    @LINE(fill="fillcolor") {(29.5, -31.5) (34.5, -31.5) (32, -28.5) (29.5, -31.5)};
*    @ARC {(37.5, -31) (40, -28.5) (42.5, -31)}; @BEZIER {(45.5, -30) (47, -27) (49, -33) (50.5, -30)}} @SET-STYLE $wide;
*
*   $butt = @DEFINE-STYLE(linestyle="dashed", linewidth=0.15, cap="butt", join="miter") {};
*   (-3, -35) @TEXT("dashed 0.15", halign="end", valign="middle");
*   {(0, -35) @RECTANGLE(width=5, height=3); (8, -35) @CIRCLE(radius=1.5); (16, -35) @ELLIPSE(width=5, height=3);
*    @LINE {(21.5, -36.5) (24, -33.5) (26.5, -36.5)}; @LINE {(29.5, -36.5) (34.5, -36.5) (32, -33.5) (29.5, -36.5)};
*    @ARC {(37.5, -36) (40, -33.5) (42.5, -36)}; @BEZIER {(45.5, -35) (47, -32) (49, -38) (50.5, -35)}} @SET-STYLE $butt;
*
*   $square = @DEFINE-STYLE(linestyle="dotted", linewidth=0.05, cap="square", join="bevel", linecolor="#c62828") {};
*   (-3, -40) @TEXT("dotted 0.05", halign="end", valign="middle");
*   {(0, -40) @RECTANGLE(width=5, height=3); (8, -40) @CIRCLE(radius=1.5); (16, -40) @ELLIPSE(width=5, height=3);
*    @LINE {(21.5, -41.5) (24, -38.5) (26.5, -41.5)}; @LINE {(29.5, -41.5) (34.5, -41.5) (32, -38.5) (29.5, -41.5)};
*    @ARC {(37.5, -41) (40, -38.5) (42.5, -41)}; @BEZIER {(45.5, -40) (47, -37) (49, -43) (50.5, -40)}} @SET-STYLE $square;
*
*   (-3, -45) @TEXT("none", halign="end", valign="middle");
*   (0, -45) @RECTANGLE(width=5, height=3, linestyle="none", fill="fillcolor", fillcolor="#c8e6c9");
*   (8, -45) @CIRCLE(radius=1.5, linestyle="none", fill="fillcolor", fillcolor="#c8e6c9");
*   (16, -45) @ELLIPSE(width=5, height=3, linestyle="none", fill="fillcolor", fillcolor="#c8e6c9");
*   @LINE(linestyle="none", fill="fillcolor", fillcolor="#c8e6c9") {(29.5, -46.5) (34.5, -46.5) (32, -43.5) (29.5, -46.5)};
*
*   $TWIN = @DEFINE-SYMBOL {
*     (-3, 0) @PIN(terminal=1, label="a", direction="left"); (3, 0) @PIN(terminal=2, label="b", direction="right");
*     @RECTANGLE(linestyle="double") {(-1.5, -0.6) (1.5, 0.6)}; @LINE {(-3, 0) (-1.5, 0)}; @LINE {(1.5, 0) (3, 0)}
*   };
*   #R1 @SET-SYMBOL $TWIN;
*   $twin = @DEFINE-STYLE(linestyle="double") {};
*   #R2 @SET-STYLE $twin;
*   &a @WIRE(mode="horizontal-vertical", linestyle="double", linewidth=0.15) {#R1:1 #R2:1};
*   #R1 @PLACE(62, 0); #R2 @PLACE(56, -10)
* }
.end
