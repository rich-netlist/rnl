* The usages an object is a member of: written, nested two deep, of a set role, disabled, failed, of a
* terminal, of a net, made by a rule, and one whose PLACE loses a conflict (the core's tests of its reverse index)
.model nch nmos level=1
R1 a b 1k
R2 b c 1k
R3 c 0 1k
M1 d g 0 0 nch
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4")}
* @RNL { /* rnl upgrade wrote this record, which keeps the labels and ports the file had before 0.4, but for its last REMOVE-FROM, of $GROUND_NET_SET, left out: rnl-std-fill gives the pins of M1's grounded terminals their stubs, usages a rule of a package made */ # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; # @SELECT-INSTANCE(type="nmos", recursive=true) => @SET-SYMBOL $NMOS_4TERM; %rnl_upgrade_bulk_ground = $GROUND_NET_SET @SELECT-TERMINAL(type="nmos", terminal="b", recursive=true) => @SELECT-PIN(wired=false, recursive=true); $GROUND_NET_SET @SELECT-TERMINAL(type="pmos", terminal="b", recursive=true) => @SELECT-PIN(wired=false, recursive=true) => @ADD-TO %rnl_upgrade_bulk_ground; # @SELECT-USAGE(recursive=true) $NO_CONNECT => @SELECT-PIN(role="t", recursive=true) => @REMOVE-FROM %rnl_upgrade_bulk_ground; # @SELECT-USAGE(recursive=true) $NO_CONNECT => @SELECT-TERMINAL(role="t", recursive=true) => @SELECT-PIN(recursive=true) => @REMOVE-FROM %rnl_upgrade_bulk_ground; %rnl_upgrade_bulk_ground @ADD-TO $GROUND_FILL_SET; %rnl_upgrade_bulk_supply = $SUPPLY_NET_SET @SELECT-TERMINAL(type="nmos", terminal="b", recursive=true) => @SELECT-PIN(wired=false, recursive=true); $SUPPLY_NET_SET @SELECT-TERMINAL(type="pmos", terminal="b", recursive=true) => @SELECT-PIN(wired=false, recursive=true) => @ADD-TO %rnl_upgrade_bulk_supply; # @SELECT-USAGE(recursive=true) $NO_CONNECT => @SELECT-PIN(role="t", recursive=true) => @REMOVE-FROM %rnl_upgrade_bulk_supply; # @SELECT-USAGE(recursive=true) $NO_CONNECT => @SELECT-TERMINAL(role="t", recursive=true) => @SELECT-PIN(recursive=true) => @REMOVE-FROM %rnl_upgrade_bulk_supply; %rnl_upgrade_bulk_supply @ADD-TO $SUPPLY_FILL_SET; $SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET }
* @RNL {
*   $PAIR = @DEFINE(roles=["left", "right"]) {};
*   $TWO = @DEFINE(roles=[["first", "usage"], "second"]) {};
*   $SET = @DEFINE(roles=["{members}"]) {};
*   $PIN = @DEFINE(roles=[["pin", "terminal"]]) {};
*   $NET = @DEFINE(roles=[["signal", "net"]]) {};
*   $LONE = @DEFINE(roles=["part"]) {};
*   %pair1 = #R1(left) #R2(right) @AS $PAIR;
*   %outer = %pair1(first) #R3(second) @AS $TWO;
*   %top = %outer(first) #R1(second) @AS $TWO;
*   %all = {#R1 #R3} @AS $SET;
*   %off = #R2(left) #R3(right) @AS(priority="disabled") $PAIR;
*   %bad = #R1(left) #R2(right) #R3(extra) @AS $PAIR;
*   %drain = #M1:d @AS $PIN;
*   %sig = &b @AS $NET;
*   # @SELECT-INSTANCE(name="R3", recursive=true) => @AS $LONE;
*   %gnd = $GROUND @ATTACH #R3:2::net => @PLACE(12, -2);
*   %port = $IO_PORT @ATTACH &c => @PLACE(0, 0);
*   %port @PLACE(5, 5, priority="high");
*   #R1 @PLACE(-6, 0); #R2 @PLACE(6, 0); #R3 @PLACE(12, 0); #M1 @PLACE(20, 0)
* }
.end
