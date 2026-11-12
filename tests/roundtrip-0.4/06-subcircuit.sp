* Numbered objects, keyed and unkeyed text, a named and an unnamed wire.
* @RNL(type="header") { @VERSION("0.4") }
.subckt cell a b
R1 a b 1k
R2 a b 2k
* @RNL {
*   $PART = @DEFINE-SYMBOL { (0, 2) @PIN(terminal=1); (0, -2) @PIN(terminal=2); (0, 0) @RECTANGLE(width=2, height=2) };
*   {#R1 #R2} @SET-SYMBOL $PART;
*   #R1 @TEXT("R_{one}", key="note", dx=1);
*   #R2 @TEXT("电阻 \alpha");
*   &a @WIRE {#R1:1 (_, _) #R2:1};
*   %"return path" = &b @WIRE {#R1:2 #R2:2}
* }
* @RNL(type="generated", priority="high", author="old") { #R1 @PLACE(-99, -99) }
.ends cell
X1 a b cell
.end
