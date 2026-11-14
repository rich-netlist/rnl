* Set-difference generation rules and explicit usages (10.5)
*! 10.5: There is no built-in "add none where one exists" rule; instead, generation
*!   rules avoid duplicating explicit objects by taking the difference of two sets
*!   with `REMOVE-FROM` (evaluated on the state before step 5, 5.2, 8.2).
*! - In `cell`, port `inp` has an explicit `$IO_PORT @ATTACH &inp => %p_in` placed at
*!   (-6, 0), so `# @SELECT-USAGE $IO_PORT => @SELECT-NET => @REMOVE-FROM $PORT_FILL_SET`
*!   leaves out `&inp` and generates `$IO_PORT` only for `&out`.
*! - `#R1:2` has an explicit `$GROUND_STUB @ATTACH(gap=0) #R1::n` on the pin, so
*!   `# @SELECT-USAGE $GROUND_STUB => @SELECT-PIN => @REMOVE-FROM $GROUND_FILL_SET`
*!   leaves out `#R1:2` and generates `$GROUND_STUB` only for `#R2:2`.
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4");
*   @VERSION("0.4")
* }
* @RNL { # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; $SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET; $GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET }

.subckt cell inp out
R1 inp 0 1k
R2 out 0 2k
* @RNL {
*   #R1 @PLACE(0, 0);
*   #R2 @PLACE(10, 0);
*   %p_in = $IO_PORT @ATTACH &inp => @PLACE(-6, 0);
*   %g_pin = $GROUND_STUB @ATTACH(gap=0) #R1::n;
*   # @SELECT-USAGE(recursive=true) $GROUND_STUB => @SELECT-PIN(recursive=true) => @REMOVE-FROM $GROUND_FILL_SET
* }
.ends cell

X1 a b cell
* @RNL {
*   #X1 @PLACE(0, 0)
* }
.end



