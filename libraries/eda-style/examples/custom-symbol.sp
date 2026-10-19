* Independent EDA geometry and style, including a user-defined symbol
* @RNL(type="header") {
*   @VERSION("0.3");
*   @USE-PACKAGE(name="eda-symbol", version="0.1");
*   @USE-PACKAGE(name="eda-style", version="0.1")
* }
.subckt custom p n
.ends custom
R1 a b 1k
X1 a b custom
* @RNL {
*   $CUSTOM = @DEFINE-SYMBOL(box=[-2, -1, 2, 1]) {
*     @RECTANGLE {(-1, -1) (1, 1)};
*     @LINE {(-2, 0) (-1, 0)};
*     @LINE {(1, 0) (2, 0)};
*     (-2, 0) @PIN(terminal=1, direction="left");
*     (2, 0) @PIN(terminal=2, direction="right");
*     (0, 0) @TEXT("custom", halign="middle", valign="middle")
*   };
*   #R1 @PLACE(-8, 0);
*   #X1 @SET-SYMBOL $CUSTOM;
*   #X1 @PLACE(8, 0);
*   &a @WIRE(mode="vertical-horizontal") {#R1:1 #X1:1};
*   %ground = $GROUND @ATTACH &a => @PLACE(-8, -10);
*   %supply = $SUPPLY @ATTACH &b => @PLACE(0, -10);
*   %unused = $NO_CONNECT @ATTACH #X1:2
* }
.end
