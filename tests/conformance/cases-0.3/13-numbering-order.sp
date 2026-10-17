* Numbering order (5.9): authored records, usage bodies, scope= records, and solver record's own objects
*! 5.9: Objects without a `%` name of the level are numbered from 1 in order:
*!   1. authored records of the level (its own text, then records with `scope=`),
*!      with the unnamed objects of each usage body right after the usage, and
*!      one number per member when a statement creates an object for each member
*!      of a set;
*!   2. the solver record's own unnamed objects, where a number may be referenced
*!      before the statement that makes its object.
* @RNL(type="header") {
*   @VERSION("0.3")
* }

* @RNL {
*   $TAP = @DEFINE(roles=["dev"]) {
*     (0, 0)
*   }
* }

.subckt cell a b
R1 a b 1k
R2 a b 2k
* @RNL {
*   #R1 @PLACE(0, 0);
*   #R2 @PLACE(10, 0);
*   #R1(dev) @AS $TAP;
*   {#R1 #R2} @TEXT("${name}")
* }
* @RNL(type="generated", priority="high", numbering="1b5c5e8ccd15844c") {
*   %#2 @PLACE(1, 2);
*   %#3 @PLACE(0, 3);
*   %#4 @PLACE(10, 3);
*   %#5 @PLACE(5, 4);
*   %#6 @LAYER("top");
*   %#7 @PLACE(5, 1);
*   &a @WIRE {#R1:1 (_, _) #R2:1}
* }
.ends cell

* @RNL(scope="cell") {
*   (_, _)
* }

X1 n1 n2 cell
.end
