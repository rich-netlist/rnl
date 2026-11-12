* A generated usage becomes explicit; its wire suppresses the next generation.
* @RNL(type="header") { @VERSION("0.4") }
R1 a b 1k
* @RNL {
*   $PART = @DEFINE-SYMBOL { (0, 2) @PIN(terminal=1); %bottom = (0, -2) @PIN(terminal=2) };
*   #R1 @SET-SYMBOL $PART;
*   $MARK = @DEFINE(roles=[["t", "terminal"]]) {
*     %p = (_, _); ?t::net @WIRE {?t %p};
*     %low = {?t %p} @ALIGN("x", priority="low");
*     %high = %p ?t @DISTANCE("bottom-to-top", min=2, priority="high");
*     %plain = %p ?t @DISTANCE("bottom-to-top", min=0, max=6)
*   };
*   $OPEN = # @SELECT-PIN(net="b", wired=false, recursive=true)
* }
* @RNL(priority="disabled") { $MARK @ATTACH $OPEN }
.end
