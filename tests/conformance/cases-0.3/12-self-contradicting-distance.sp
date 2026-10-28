* A DISTANCE that contradicts itself, the example of 9.5
*! Both roles of $DP_LAYOUT are filled by M1, so the DISTANCE it expands to wants M1 at least 4 from itself: it has no solution on its own, is an error, and only it is dropped (9.5, 8.5); the ALIGN holds as usual.
*! Such a fact is reported before conflicts are resolved, takes no part in them, and is not handed to the solver.
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
