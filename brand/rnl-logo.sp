* RNL Official Logo (RNL 0.4 — Final Compact Triangle Ground Version)
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4") }
* @RNL { # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; $SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET; $GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET }
.model nch nmos level=1
M1 0 0 0 0 nch W=4u L=0.5u

* @RNL {
*   /* 1. Generic drawable circuit & layout using standard library names */
*   # @SELECT-INSTANCE(type="nmos", recursive=true) => @SET-SYMBOL $NMOS;
*   #M1:b @REMOVE-FROM $GROUND_FILL_SET;
*   #M1 @PLACE(-1, 1) => @ORIENT("MY");
*   %g0 = $GROUND @ATTACH &0 => @PLACE(0.9, -4.2) => @ORIENT("R90");
*   %jg = (3, 1);
*   &0 @WIRE(mode="straight") {
*     #M1:d #M1:s (-1, -2.2) (3, -2.2) %jg
*     (3, 4.2) (-3, 4.2) (-3, -4.2) %g0::pin
*   };
*   &0 @WIRE(mode="straight") {#M1:g %jg};
*   $JUNCTION @ATTACH #M1:d => @PLACE(-1, 2);
*   $JUNCTION @ATTACH #M1:s => @PLACE(-1, 0);
*   $JUNCTION @ATTACH %jg => @PLACE(3, 1)
* }

* @RNL {
*   /* 2. Logo theme override layer (can be removed to render with pure rnl-std 0.4) */
*   $base = @DEFINE-STYLE(linecolor="#1d1d1f", textcolor="#1d1d1f", fillcolor="#1d1d1f", linewidth=0.36, cap="round", join="round") {};
*   $signal = @DEFINE-STYLE(linecolor="#e8590c", fillcolor="#e8590c") {$base};
*
*   $NMOS = @DEFINE-SYMBOL(box=[-3.4, -1.75, 0.4, 1.75]) {
*     %d = (0, 1) @PIN(terminal="d", direction="up");
*     %g = (-3, 0) @PIN(terminal="g", direction="left");
*     %s = (0, -1) @PIN(terminal="s", direction="down");
*     @LINE {(0, 1) (-1.85, 1)};
*     @LINE {(0, -1) (-1.85, -1)};
*     @LINE(fill="linecolor", linestyle="none") {(-0.60, -1) (-1.43, -0.44) (-1.43, -1.56) (-0.60, -1)};
*     @LINE {(-1.85, -1.68) (-1.85, 1.68)};
*     @LINE {(-2.65, -0.88) (-2.65, 0.88)};
*     @LINE {(-2.65, 0) (-3, 0)}
*   };
*
*   $GROUND_SYMBOL = @DEFINE-SYMBOL(box=[-0.9, -2.15, 0.9, 0]) {
*     %pin = (0, 0) @PIN(terminal="p", direction="up");
*     @LINE {(0, 0) (0, -0.90)};
*     @LINE {(-0.85, -0.90) (0.85, -0.90) (0, -2.10) (-0.85, -0.90)}
*   };
*
*   $JUNCTION_SYMBOL = @DEFINE-SYMBOL(box=[-0.5, -0.5, 0.5, 0.5]) {
*     (0, 0) @CIRCLE(radius=0.50, fill="linecolor", linestyle="none")
*   };
*
*   # @SELECT-INSTANCE(recursive=true) => @SET-STYLE $base;
*   # @SELECT-USAGE(recursive=true) => @SET-STYLE $base;
*   # @SELECT-WIRE(recursive=true) => @SET-STYLE $base;
*   # @SELECT-INSTANCE(type="nmos", recursive=true) => @SET-SYMBOL $NMOS;
*   # @SELECT-USAGE(recursive=true) $GROUND => @SET-SYMBOL $GROUND_SYMBOL;
*   # @SELECT-USAGE(recursive=true) $JUNCTION => @SET-SYMBOL $JUNCTION_SYMBOL => @SET-STYLE $signal => @LAYER("top")
* }
.end
