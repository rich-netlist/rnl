* A DISTANCE that contradicts itself, the example of 9.5
*! Both roles of $DP_LAYOUT are filled by M1, so the DISTANCE and the ALIGN it expands to would relate M1 to itself: a combination that takes one object twice is left out (14.1, 14.5).
*! With none left, each holds trivially and has a too-few-places hint; neither takes part in conflicts or is handed to the solver.
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4")}
M1 d g 0 0 nch
M2 d2 g 0 0 nch
.model nch nmos level=1
* @RNL {
*   $DP_LAYOUT = @DEFINE(roles=[["in_p", "instance"], ["in_n", "instance"]]) {
*     {?in_p ?in_n} @ALIGN("y");
*     ?in_p ?in_n @DISTANCE("left-to-right", min=4)
*   };
*   #M1(in_p) #M1(in_n) @AS $DP_LAYOUT;
*   #M1 @PLACE(0, 0);
*   #M2 @PLACE(8, 0)
* }
.end


