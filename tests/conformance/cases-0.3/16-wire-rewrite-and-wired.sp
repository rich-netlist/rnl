* Wire rewrites, failed rewrites, `wired=` selection, and wire in net slot (13.1, 10.2)
*! 13.1: Given one wire in the net slot, WIRE rewrites its point list and routing mode
*! while keeping its net. Among several rewrites (including later segments of a pipe,
*! 5.5), the later one replaces the point list and mode, while style parameters merge
*! in the style queue (8.3). A rewrite that fails in the geometry phase (such as a
*! terminal with no pin) produces an error and has no effect, its style parameters
*! included. A set containing a wire in the net slot of WIRE fails (wrong kind).
*! 10.2, 13.1: `wired=` uses the points of the latest rewrite as of step 3.
* @RNL(type="header") {
*   @VERSION("0.3")
* }

R1 a b 1k
R2 a b 2k
R3 a b 3k

* @RNL {
*   $ONE_PIN = @DEFINE-SYMBOL {
*     (0, 0) (2, 2) @RECTANGLE;
*     (0, 1) @PIN(terminal=1)
*   };
*   #R3 @SET-SYMBOL $ONE_PIN;
*   #R1 @PLACE(0, 0);
*   #R2 @PLACE(10, 0);
*   #R3 @PLACE(20, 0);
*
*   %w1 = &a @WIRE(mode="auto", linecolor="#112233") {#R1:1 #R2:1};
*   %w1 @WIRE(mode="manhattan", linewidth=0.3) {#R1:1 (4, 2) #R3:1}
*     => @WIRE(mode="straight", linecolor="#00AA00") {#R1:1 (5, 3) #R3:1};
*
*   %w2 = &b @WIRE(mode="manhattan", linecolor="#445566") {#R1:2 (5, -2) #R2:2};
*   %w2 @WIRE(mode="straight", linestyle="dashed") {#R1:2 #R3:2};
*
*   # @SELECT-TERMINAL(name="R*", terminal=1, wired=true) => @TEXT("w1");
*
*   {%w1} @WIRE {#R1:1 #R2:1}
* }
.end
