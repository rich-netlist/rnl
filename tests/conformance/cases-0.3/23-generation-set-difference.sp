* Set-difference generation rules and explicit usages (10.5)
*! 10.5: There is no built-in "add none where one exists" rule; instead, generation
*!   rules avoid duplicating explicit objects by taking the difference of two sets
*!   with `REMOVE-FROM` (evaluated on the state before step 5, 5.2, 8.2).
*! - In `cell`, port `inp` has an explicit `$IO_PORT @ATTACH &inp => %p_in` placed at
*!   (-6, 0), so `# @SELECT-USAGE $IO_PORT => @SELECT-NET => @REMOVE-FROM $PORT_FILL_SET`
*!   leaves out `&inp` and generates `$IO_PORT` only for `&out`.
*! - `#R1:2` has an explicit `$GROUND @ATTACH #R1:2` on the pin, so
*!   `# @SELECT-USAGE $GROUND => @SELECT-TERMINAL => @REMOVE-FROM $GROUND_FILL_SET`
*!   leaves out `#R1:2` and generates `$GROUND_STUB` only for `#R2:2`.
* @RNL(type="header") {
*   @VERSION("0.3")
* }

.subckt cell inp out
R1 inp 0 1k
R2 out 0 2k
* @RNL {
*   #R1 @PLACE(0, 0);
*   #R2 @PLACE(10, 0);
*   %p_in = $IO_PORT @ATTACH &inp => @PLACE(-6, 0);
*   %g_pin = $GROUND @ATTACH #R1:2
* }
.ends cell

X1 a b cell
* @RNL {
*   #X1 @PLACE(0, 0)
* }
.end
