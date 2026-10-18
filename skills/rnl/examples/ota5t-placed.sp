* Five-transistor OTA, placed by hand
.model nch nmos level=1
.model pch pmos level=1
M1 x inp tail 0 nch w=4u l=0.5u
M2 out inn tail 0 nch w=4u l=0.5u
M3 x x vdd vdd pch w=8u l=0.5u
M4 out x vdd vdd pch w=8u l=0.5u
M5 tail bias 0 0 nch w=8u l=1u
M6 bias bias 0 0 nch w=8u l=1u
Ib vdd bias 20u
CL out 0 1p
Vdd vdd 0 1.8
* @RNL(type="header") { @VERSION("0.3")}
* @RNL {
*   # @SELECT-INSTANCE(name="M*") => @SET-SYMBOL $NMOS;
*   # @SELECT-INSTANCE(type="pmos") => @SET-SYMBOL $PMOS;
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*
*   %dp = #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR;
*   %cm = {#M3}(ref) {#M4}(out) @AS $CURRENT_MIRROR;
*
*   %jd = (-6, 4); %jg = (-2, 8); %jo = (6, 4); %jc = (16, 4); %jt = (0, -3);
*   %jb = (-22, -3.5); %j6 = (-14, -6);
*   &x @WIRE {#M3:1 %jd #M1:1};
*   &x @WIRE {#M3:2 %jg #M4:2};
*   &x @WIRE(mode="horizontal-vertical") {%jd %jg};
*   &out @WIRE {#M4:1 %jo #M2:1};
*   &tail @WIRE(mode="vertical-horizontal") {#M1:3 %jt};
*   &tail @WIRE(mode="vertical-horizontal") {#M2:3 %jt};
*   &tail @WIRE {%jt #M5:1};
*   &bias @WIRE {#Ib:2 %jb #M6:1};
*   &bias @WIRE {#M6:2 %j6 #M5:2};
*   &bias @WIRE(mode="vertical-horizontal") {%j6 %jb};
*   $JUNCTION @ATTACH {%jd %jg %jo %jc %jt %jb %j6};
*
*   %pi = $IO_PORT @ATTACH &inp;
*   $IO_PORT @ATTACH &inn => @ORIENT("MY") => %pn;
*   $IO_PORT @ATTACH &out => @ORIENT("MY") => %po;
*   &inp @WIRE {%pi:p #M1:2};
*   &inn @WIRE {%pn:p #M2:2};
*   &out @WIRE {%jo %jc %po:p};
*   &out @WIRE {%jc #CL:1}
* }
* @RNL(type="generated", priority="high") {
*   #M1 @PLACE(-6, 0); #M2 @PLACE(6, 0) => @ORIENT("MY");
*   #M3 @PLACE(-6, 8) => @ORIENT("MY"); #M4 @PLACE(6, 8);
*   #M5 @PLACE(0, -6); #M6 @PLACE(-22, -6) => @ORIENT("MY"); #Ib @PLACE(-22, 0);
*   #CL @PLACE(16, 1); #Vdd @PLACE(28, 4);
*   %pi @PLACE(-11, 0); %pn @PLACE(11, 0); %po @PLACE(21, 4);
*   %gs_M5_3 = $GROUND_STUB @ATTACH #M5:3; %gs_M5_3::g @PLACE(0, -10);
*   %gs_M6_3 = $GROUND_STUB @ATTACH #M6:3; %gs_M6_3::g @PLACE(-22, -10);
*   %gs_CL_2 = $GROUND_STUB @ATTACH #CL:2; %gs_CL_2::g @PLACE(16, -3);
*   %gs_Vdd_2 = $GROUND_STUB @ATTACH #Vdd:2; %gs_Vdd_2::g @PLACE(28, -1);
*   %ss_M3_3 = $SUPPLY_STUB @ATTACH #M3:3; %ss_M3_3::s @PLACE(-6, 12);
*   %ss_M4_3 = $SUPPLY_STUB @ATTACH #M4:3; %ss_M4_3::s @PLACE(6, 12);
*   %ss_Ib_1 = $SUPPLY_STUB @ATTACH #Ib:1; %ss_Ib_1::s @PLACE(-22, 5);
*   %ss_Vdd_1 = $SUPPLY_STUB @ATTACH #Vdd:1; %ss_Vdd_1::s @PLACE(28, 9)
* }
.end
