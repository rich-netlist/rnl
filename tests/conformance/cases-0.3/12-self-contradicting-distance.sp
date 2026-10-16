* A DISTANCE that contradicts itself, the example of 9.5
*! $DP_LAYOUT 的两个角色都由 M1 填上，展开的 DISTANCE 要 M1 离自己至少 4：它自身无解，报错，只丢掉它（9.5、8.5），ALIGN 照常成立。
*! 这样的事实在解析冲突之前就报错，不参与冲突，也不交给求解器。
* @RNL(type="header") { @VERSION("0.3")}
M1 d g 0 0 nch
M2 d2 g 0 0 nch
.model nch nmos level=1
* @RNL {
*   $DP_LAYOUT = @DEFINE(roles=[["in_p", "instance"], ["in_n", "instance"]]) {
*     {?in_p ?in_n} @ALIGN("y");
*     ?in_p ?in_n @DISTANCE("x", min=4)
*   };
*   #M1(in_p) #M1(in_n) @AS $DP_LAYOUT;
*   #M1 @PLACE(0, 0);
*   #M2 @PLACE(8, 0)
* }
.end
