* Cycles of sets: the statement that closes a cycle fails, and the rest is drawn
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4")}
*! A cycle of set dependencies is an error (5.2.2): in the order of position, the statement that closes the cycle fails, the diagnostic lists the whole cycle, the other statements take effect as usual, and the interpretation as a whole does not fail.
*! %row is added to %col and then %col to %row: the latter fails, %row still has only R1 and R2, and the alignment holds; %b = {%a} and then %b @REMOVE-FROM %a: the removal fails, and C1 is placed as usual.
*! Two package sets both with selections: the top level's rule adds $LEFT to $RIGHT, and the subcircuit cell adds $RIGHT to $LEFT. The cross-level additions do not form a cycle in 0.4: these nonrecursive package selections remain at the top level, and their sets are empty in cell. A separate pair of package sets closes a cycle in cell; its closing statement fails without affecting the top level.
R1 a b 1k
R2 b c 1k
R3 c 0 1k
C1 a 0 1p
X1 a c cell
.subckt cell p q
R1 p q 1k
* @RNL { @USE-PACKAGE(name="cell-cycles") { $LOCAL_A = {# @SELECT-INSTANCE(name="R*")}; $LOCAL_B = {# @SELECT-INSTANCE(name="C*")} }; $RIGHT @ADD-TO $LEFT; $LOCAL_A @ADD-TO $LOCAL_B; $LOCAL_B @ADD-TO $LOCAL_A; %empty = {}; $LEFT @ASSERT-IN %empty; $RIGHT @ASSERT-IN %empty; %cell_expected = {#R1}; #R1 @ASSERT-IN $LOCAL_A; $LOCAL_A @ASSERT-IN %cell_expected; #R1 @ASSERT-IN $LOCAL_B; $LOCAL_B @ASSERT-IN %cell_expected; %not_selected = {#R1}; $LEFT @REMOVE-FROM %not_selected; #R1 @ASSERT-IN %not_selected; #R1 @PLACE(0, 0) }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:e049fdc5192bcb04.ffc36700559c31e7") {
*   %net_label_stub_R1_p = $NET_LABEL_STUB @ATTACH #R1::p;
*   %net_label_stub_R1_n = $NET_LABEL_STUB @ATTACH #R1::n;
*   #R1 @PLACE(0, 0) => @ORIENT("R0");
*   %net_label_stub_R1_p @PLACE(0, 0);
*   %net_label_stub_R1_p::l @PLACE(0, 2) => @ORIENT("R90");
*   %net_label_stub_R1_n @PLACE(0, 0);
*   %net_label_stub_R1_n::l @PLACE(0, -2) => @ORIENT("R270")
* }
.ends
* @RNL {
*   @USE-PACKAGE(name="cycles") { $LEFT = {# @SELECT-INSTANCE(name="R*")}; $RIGHT = {# @SELECT-INSTANCE(name="C*")} };
*   $LEFT @ADD-TO $RIGHT;
*   $RIGHT @SET-STYLE $grey;
*   %left_expected = {#R1 #R2 #R3}; %right_expected = {#C1 #R1 #R2 #R3};
*   {#R1 #R2 #R3} @ASSERT-IN $LEFT; $LEFT @ASSERT-IN %left_expected;
*   {#C1 #R1 #R2 #R3} @ASSERT-IN $RIGHT; $RIGHT @ASSERT-IN %right_expected;
*
*   %row = {#R1 #R2};
*   %col = {#R3};
*   %row @ADD-TO %col;
*   %col @ADD-TO %row;
*   %row @ALIGN("y");
*
*   %a = {#C1};
*   %b = {%a};
*   %b @REMOVE-FROM %a;
*   #C1 @PLACE(0, -6);
*
*   #R1 @PLACE(0, 0); #R2 @PLACE(6, 0); #R3 @PLACE(12, -3); #X1 @PLACE(20, 0)
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:63983f5113f1f76a.ffc36700559c31e7") {
*   %ground_stub_R3_n = $GROUND_STUB @ATTACH(gap=1) #R3::n;
*   %ground_stub_C1_n = $GROUND_STUB @ATTACH(gap=1) #C1::n;
*   %net_label_stub_R1_p = $NET_LABEL_STUB @ATTACH #R1::p;
*   %net_label_stub_C1_p = $NET_LABEL_STUB @ATTACH #C1::p;
*   %net_label_stub_R1_n = $NET_LABEL_STUB @ATTACH #R1::n;
*   %net_label_stub_R2_p = $NET_LABEL_STUB @ATTACH #R2::p;
*   %net_label_stub_R2_n = $NET_LABEL_STUB @ATTACH #R2::n;
*   %net_label_stub_R3_p = $NET_LABEL_STUB @ATTACH #R3::p;
*   #R1 @PLACE(0, 0) => @ORIENT("R0");
*   #R2 @PLACE(6, 0) => @ORIENT("R0");
*   #R3 @PLACE(12, -3) => @ORIENT("R0");
*   #C1 @PLACE(0, -6) => @ORIENT("R0");
*   #X1 @PLACE(20, 0) => @ORIENT("R0");
*   %ground_stub_R3_n @PLACE(0, 0);
*   %ground_stub_R3_n::g @PLACE(12, -6) => @ORIENT("R0");
*   %ground_stub_C1_n @PLACE(0, 0);
*   %ground_stub_C1_n::g @PLACE(0, -9) => @ORIENT("R0");
*   %net_label_stub_R1_p @PLACE(0, 0);
*   %net_label_stub_R1_p::l @PLACE(0, 2) => @ORIENT("R90");
*   %net_label_stub_C1_p @PLACE(0, 0);
*   %net_label_stub_C1_p::l @PLACE(0, -4) => @ORIENT("R90");
*   %net_label_stub_R1_n @PLACE(0, 0);
*   %net_label_stub_R1_n::l @PLACE(0, -2) => @ORIENT("R270");
*   %net_label_stub_R2_p @PLACE(0, 0);
*   %net_label_stub_R2_p::l @PLACE(6, 2) => @ORIENT("R90");
*   %net_label_stub_R2_n @PLACE(0, 0);
*   %net_label_stub_R2_n::l @PLACE(6, -2) => @ORIENT("R270");
*   %net_label_stub_R3_p @PLACE(0, 0);
*   %net_label_stub_R3_p::l @PLACE(12, -1) => @ORIENT("R90")
* }
.end
