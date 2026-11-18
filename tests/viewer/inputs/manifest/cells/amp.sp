* amplifier cell
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4") }
.subckt amp in out
R1 in out 1k
C1 out 0 1p
* @RNL { #R1 @PLACE(0, 0); #C1 @PLACE(10, 0) }
* @RNL {
*   $BOX_amp = @DEFINE-SYMBOL {
*     @RECTANGLE(linestyle="dashed") {(-2, -1) (2, 1)};
*     @LINE {(-3, 0) (-2, 0)};
*     @LINE {(3, 0) (2, 0)};
*     (-3, 0) @PIN(terminal=1, direction="left", label="in");
*     (3, 0) @PIN(terminal=2, direction="right", label="out");
*     ? @TEXT("${name}", key="name", halign="middle", valign="bottom") => @PLACE(0, 1.3)
*   };
*   # @SET-SYMBOL $BOX_amp
* }
.ends amp
