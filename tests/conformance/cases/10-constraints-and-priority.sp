* Constraints and priority without a solver
*! 四种原子约束对照写死的坐标检查：满足、按取舍次序丢弃、取舍次序在坐标之前（要求解器改坐标）、成员是自由点（未检查）、disabled；
*! 优先级的覆盖顺序：记录头默认 < 语句参数 < PRIORITY 修改 < 外层用例的优先级 < 经 :: 写在局部对象上的 PRIORITY；
*! PLACE 按坐标分单位丢弃、ORIENT 冲突、priority="disabled" 的 PLACE；编辑器在求解器之后写的 high PLACE 盖过求解结果；DISTANCE 的 exact 是精确距离。
R1 a b 1k
R2 b c 1k
R3 c d 1k
R4 d 0 1k
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   %c1 = {#R1 #R2 #R3} @ALIGN("y");
*   %c2 = {#R1 #R2 #R3 #R4} @DISTRIBUTE("x");
*   %c3 = {#R3 #R4} @ALIGN("y");
*   %c4 = {#R4 #R1} @ORDER("left-to-right");
*   %c4 @PRIORITY("disabled");
*   %c5 = #R1 #R4 @DISTANCE("y", min=1, max=3);
*   %f = (_, _);
*   %c6 = {#R1 %f} @ALIGN("x");
*   %c8 = #R1 #R2 @DISTANCE("left-to-right", exact=8);
*
*   $PAIR_Y = @DEFINE(roles=["a", "b"]) { {?a ?b} @ALIGN("y", priority="low")};
*   %p1 = #R3 #R4 @AS $PAIR_Y;
*   $ROW = @DEFINE(roles=["{m}"]) {
*     %row = ?m @ALIGN("y", priority="low");
*     %ord = ?m @ORDER("left-to-right")
*   };
*   %r = {#R1 #R2 #R3} @AS(priority="low") $ROW;
*   %r::ord @PRIORITY("high");
*
*   #R1 @ORIENT("R0");
*   #R2 @PLACE(7, 0);
*   #R3 @PLACE(x=0, y=0, priority="disabled");
*   &b @WIRE {#R1:2 #R2:1}; &c @WIRE {#R2:2 #R3:1}; &d @WIRE(mode="vertical-horizontal") {#R3:2 #R4:1}
* }
* @RNL(priority="low") {
*   %c7 = {#R2 #R4} @ALIGN("x")
* }
* @RNL(type="generated", priority="high") {
*   #R1 @PLACE(0, 0) => @ORIENT("R90"); #R2 @PLACE(8, 0) => @ORIENT("R90"); #R3 @PLACE(16, 0) => @ORIENT("R90");
*   #R4 @PLACE(24, 2);
*   %gs_R4_2 = $GND_STUB @ATTACH #R4:2; %gs_R4_2::g @PLACE(24, -7)
* }
* @RNL(author="rnl-editor 0.1") {
*   #R4 @PLACE(y=-2, priority="high");
*   %p2 = #R3 #R4 @AS(priority="high") $PAIR_Y;
*   # @SELECT-USAGE $ROW => @PRIORITY("disabled");
*   %t = (0, 6) @TEXT("各约束的状态和生效的优先级见下方列表")
* }
.end
