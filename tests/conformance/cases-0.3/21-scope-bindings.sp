* Binding names across a subcircuit and `scope=` records (2.6, 6.6, 6.7)
*! 2.6, 6.7: Records with `scope=` publish their bound names in the target subcircuit's
*!   namespace, shared with the subcircuit's own records (including forward references).
*! 6.6: Binding a name again in `scope=` to the same object (`%same = #R1` and
*!   `%same = %r1_alias`) is allowed without error. Binding a name again in `scope=` to
*!   a different object (`%clash = #R2` when `%clash` is already `#R1`) is an error,
*!   and the name keeps its first valid binding (`#R1`).
* @RNL(type="header") {
*   @VERSION("0.3")
* }

.subckt cell a b
R1 a b 1k
R2 a b 2k
* @RNL {
*   %same = #R1;
*   %r1_alias = #R1;
*   %clash = #R1;
*   %from_scope @PLACE(10, 0)
* }
.ends cell

* @RNL(scope="cell") {
*   %same = #R1;
*   %same = %r1_alias;
*   %from_scope = #R2;
*   %clash = #R2;
*   %clash @PLACE(0, 0)
* }

X1 n1 n2 cell
* @RNL {
*   #X1 @PLACE(0, 0)
* }
.end
