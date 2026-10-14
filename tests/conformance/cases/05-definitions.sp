* Three-stage RC ladder
*! 普通定义：角色声明种类、参数及默认值、局部名（%s1::mid）、体里的导线与约束、参数写在字面量的位置（max=?drop）；
*! 用例填进别的用例的角色（$CHAIN 的集合角色）；按角色选择（role="c"）；自定义虚元件 $TP 与它的符号和规则，命名插在管道中间（=> %tp =>），文字 ${p} 显示角色里端子所在的网络；
*! 标准包的 $GND_STUB（体里 ATTACH $GND、接线、约束），求解器写回 %st::g 的位置。
V1 in 0 AC 1
R1 in n1 1k
C1 n1 0 1n
R2 n1 n2 1k
C2 n2 0 1n
R3 n2 out 1k
C3 out 0 1n
RL out 0 100k
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   $RC_STAGE = @DEFINE(roles=[["r", "instance"], ["c", "instance"]], parameters=[["drop", 3]]) {
*     %mid = (_, _);
*     ?r:2::net @WIRE {?r:2 %mid ?c:1};
*     $JUNCTION @ATTACH %mid;
*     {?r:2 %mid} @ALIGN("y");
*     {%mid ?c:1} @ALIGN("x");
*     ?c:1 %mid @DISTANCE("bottom-to-top", min=0.5, max=?drop)
*   };
*   %s1 = #R1 #C1 @AS $RC_STAGE;
*   %s2 = #R2 #C2 @AS(drop=2) $RC_STAGE;
*   %s3 = #R3(r) #C3(c) @AS(drop=0.8) $RC_STAGE;
*   %ch = #V1 {%s1 %s2 %s3} #RL @AS $CHAIN;
*   # @SELECT-USAGE $RC_STAGE => @SELECT-INSTANCE(role="c") => @SET-STYLE $grey;
*
*   &in @WIRE {#V1:1 #R1:1};
*   &n1 @WIRE {%s1::mid #R2:1};
*   &n2 @WIRE {%s2::mid #R3:1};
*   &out @WIRE {%s3::mid #RL:1};
*
*   $TP = @DEFINE(roles=[["p", ["net", "terminal"]]]) {};
*   $TP_SYM = @DEFINE-SYMBOL {(0, 0) @CIRCLE(radius=0.35); @LINE {(0.25, 0.25) (0.9, 0.9)}};
*   # @SELECT-USAGE $TP => @SET-SYMBOL $TP_SYM;
*   $TP @ATTACH #RL:1 => %tp => @TEXT("TP1 = ${p}", dx=1, dy=1.2);
*
*   %st = $GND_STUB @ATTACH #C1:2
* }
* @RNL(type="generated", priority="high") {
*   #V1 @PLACE(-3, 3);
*   #R1 @PLACE(4, 6) => @ORIENT("R90"); #C1 @PLACE(8, 3);
*   #R2 @PLACE(12, 6) => @ORIENT("R90"); #C2 @PLACE(16, 3);
*   #R3 @PLACE(20, 6) => @ORIENT("R90"); #C3 @PLACE(24, 3);
*   #RL @PLACE(28, 3);
*   %s1::mid @PLACE(8, 6); %s2::mid @PLACE(16, 6); %s3::mid @PLACE(24, 6);
*   %st::g @PLACE(8, -1);
*   %gs_V1_2 = $GND_STUB @ATTACH #V1:2; %gs_V1_2::g @PLACE(-3, -2);
*   %gs_C2_2 = $GND_STUB @ATTACH #C2:2; %gs_C2_2::g @PLACE(16, -1);
*   %gs_C3_2 = $GND_STUB @ATTACH #C3:2; %gs_C3_2::g @PLACE(24, -1);
*   %gs_RL_2 = $GND_STUB @ATTACH #RL:2; %gs_RL_2::g @PLACE(28, -2)
* }
.end
