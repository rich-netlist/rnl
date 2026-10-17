* Header record placement, duplicate VERSION, .lib section versions, and 0.3 alias collision (2.8, 16.1, 16.5)
*! 16.1: A header record may declare VERSION only once; a second VERSION in the same
*!   header record is an error and is ignored, so the first ("0.3") stands.
*! 2.8: A header record must be the first record of the whole file (or .lib section)
*!   at the file level; a second header record or one written after a subcircuit's
*!   record is an error and is ignored.
*! 2.8, 16.1, 16.5: Each .lib section read counts as a file and can declare its own
*!   version. `numbering` is a new standard word in 0.3, so `@ALIAS("numbering", "axis")`
*!   is valid in the 0.2 section `sec_02` and an error in the 0.3 section `sec_03`.
* @RNL(type="header") {
*   @VERSION("0.3");
*   @VERSION("0.2")
* }

.lib "19-header-and-lib-versions.lib" sec_02
.lib "19-header-and-lib-versions.lib" sec_03
.lib "19-header-and-lib-versions.lib" sec_late_header

Rmain a 0 1k
* @RNL {
*   #Rmain @SET-SYMBOL $RESISTOR => @PLACE(0, 0)
* }

* A second header record later in the file is an error and is ignored (2.8)
* @RNL(type="header") {
*   @VERSION("0.2")
* }

.end
