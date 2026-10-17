* Stale numbering check, invalid number uses, and binding to a number (4.1, 5.9)
*! 5.9: When `numbering` does not match the level's check, statements that use `%#n`
*! take no effect and the record gives one hint, while statements without `%#n` still
*! take effect. A number in an authored record or beyond the last numbered object is
*! an error.
*! 4.1, 5.9: `%#n` is not bound like a name; `%#1 = ...` and `=> %#1` are syntax errors.
* @RNL(type="header") {
*   @VERSION("0.3")
* }

.subckt stale_sub a b
R1 a b 1k
R2 a b 2k
* @RNL {
*   &a @WIRE {#R1:1 (_, _) #R2:1};
*   {#R1 #R2} @TEXT("${name}")
* }
* @RNL(type="generated", priority="high", numbering="0000000000000000") {
*   #R1 @PLACE(0, 0);
*   #R2 @PLACE(8, 0);
*   %#2 @PLACE(4, 2);
*   %#3 @PLACE(0, 3)
* }
.ends stale_sub

X1 a b stale_sub
Rtop a b 1k

* @RNL {
*   #Rtop @PLACE(0, 0);
*   (_, _);
*   %#1 @PLACE(2, 2);
*   $BAD = @DEFINE(roles=["x"]) {
*     %#1 @PLACE(0, 0)
*   };
*   %#1 = (1, 1);
*   (3, 3) => %#2
* }
* @RNL(type="generated", priority="high", numbering="27ccda88f25e17b3") {
*   %#99 @PLACE(5, 5)
* }
.end
