* Diagnostics showcase
*! 故意写错的语句，各对应一条标准规则：错误（条目失效）、警告（合法但可能不是本意）、提示（覆盖与取舍）。其余语句照常生效，图照样画出来。
.model nch nmos level=1
M1 d g 0 0 nch
R1 a b 1k
R2 b 0 1k
C1 b 0 1p
.subckt sub x y
R9 x y 1k
.ends
X1 a b sub
* @RNL(type="header") {
*   @VERSION("0.2");
*   @ALIAS("@记录", "@RNL");
*   @ALIAS("@对齐", "@ALIGN");
*   @USE-PACKAGE(name="no-such-package", version="1.0")
* }
* @RNL {
*   #M1 @SET-SYMBOL $NMOS;
*   {#R1 #R2} @ALGN(axis="y");
*   #R1 @SET-SYMBOL $NOPE;
*   &a &b @AS $DIFFPAIR;
*   {} @PLACE(1, 2);
*   #M1.d @TEXT("drain?", key="note");
*   {#R1 #R2} R2 @ALIGN("x");
*   %x = {#R1}; %x = {#R2};
*   #M1:4 @TEXT("bulk");
*   %w = &b @WIRE {#R1:2 #R2:1};
*   {%w #R1} @ALIGN("x");
*   %w @PLACE(0, 0);
*   #R1 #R2 @DISTANCE("bottom-to-top", min=3, max=1);
*   #R1 #R2 @DISTANCE("y", min=-1);
*   #C1 @PLACE(x=1, 2);
*   #C1 @PLACE(z=1);
*   #C1 @ORIENT("R45");
*   {#R1 #R2} @对齐两个器件(axis="y");
*   #R1 @TEXT("${nosuch}", key="v");
*   $T = @DEFINE(roles=["a"]) { #R1 @PLACE(0, 0)};
*   $U = @DEFINE(roles=["a"], parameters=[["k", 1]]) { ?k @PLACE(0, 0)};
*   # @SET-SYMBOL { @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1)}};
*   @DEFINE-STYLE(linecolor="#ff0000") {};
*   @DEVICE(name="R*", terminals=["p", "n"]);
*   (0, 0) @PIN(terminal=1);
*   #R1 @PLACE(x=4, y=4) => @PRIORITY("disabled");
*   %c1 = {%c2}; %c2 = {%c1}; %c1 @ALIGN("x");
*   $A2 = @DEFINE-SYMBOL { $ARROW @ATTACH (0, 0)};
*   $LINK = @DEFINE(roles=["p", "q"]) {};
*   # @SELECT-NET(name="y") => &a @AS $LINK;
*   %two = #R1 #R2;
*   &a @WIRE {#R1:1 #R2:1};
*   $JUNCTION @ATTACH (3, 3) => @TEXT("${p}");
*   #R1 => %r1::x;
*   #R2 @PLACE (9, 9);
*   #M1 (in_p) @SET-SYMBOL $NMOS;
*   &a @ATTACH $GND;
*   #R1 @SELECT-USAGE $GND => @SET-STYLE $grey;
*   # @SELECT-ANCESTOR-USAGE $GND => @SET-STYLE $grey;
*   #R1 <1, 2> @PLACE(1, 1);
*   #R1 | @PLACE(1, 1);
*   #R2 @PLACE(9, 9)
* }
* @RNL(scope="nosuch") { #R1 @PLACE(0, 0)}
* @RNL { #C1 @TEXT("ok", key="c") /* a comment that ends with a backslash \
*   */}
* @RNL { #R1 @PLACE(1, 1)
R3 a 0 1k
* @RNL(type="generated", priority="high") {
*   #M1 @PLACE(0, 0); #R1 @PLACE(6, 3); #R2 @PLACE(6, -3); #C1 @PLACE(10, -3); #X1 @PLACE(-8, 0);
*   %gs_M1_3 = $GND_STUB @ATTACH #M1:3; %gs_M1_3::g @PLACE(0, -4);
*   %gs_R2_2 = $GND_STUB @ATTACH #R2:2; %gs_R2_2::g @PLACE(6, -8);
*   %gs_C1_2 = $GND_STUB @ATTACH #C1:2; %gs_C1_2::g @PLACE(10, -7)
* } #R1 @PLACE(0, 0)
.end
