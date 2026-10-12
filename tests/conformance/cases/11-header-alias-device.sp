* sky130 inverter: aliases, devices and a package found by path
*! header 记录：@ALIAS 给关键字、参数名、枚举值、属性名起中文代称，与标准写法混写；@DEVICE 给 sky130 的子电路器件起端子名（包里一条、文件里一条）；
*! @USE-PACKAGE 按 path= 找包，版本 1.2 没有、用同一大版本的 1.3 并警告；包里的规则按包的位置排在本层语句之前；钉住标准包的版本。
XM1 out in 0 0 sky130_fd_pr__nfet_01v8 w=1 l=0.15
XM2 out in vdd vdd sky130_fd_pr__pfet_01v8 w=2 l=0.15
V1 vdd 0 1.8
V2 in 0 0.9
* @RNL(type="header") {
*   @VERSION("0.2");
*   @ALIAS("@放在", "@PLACE"); @ALIAS("@朝向", "@ORIENT"); @ALIAS("@对齐", "@ALIGN");
*   @ALIAS("轴", "axis"); @ALIAS("横", "x");
*   @ALIAS("@文字", "@TEXT"); @ALIAS("内容", "text"); @ALIAS("名字", "name");
*   @ALIAS("@画成", "@SET-SYMBOL"); @ALIAS("@导线", "@WIRE");
*   @DEVICE(name="X*", model="sky130_fd_pr__pfet_01v8", terminals=["d", "g", "s", "b"]);
*   @USE-PACKAGE(name="rnl-std", version="0.2");
*   @USE-PACKAGE(name="acme-style", version="1.2", path="11-acme-style.rnl")
* }
* @RNL {
*   #XM2 @画成 $PMOS;
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   #XM1:g @CURRENT("in");
*   {#XM1 #XM2} @对齐(轴="横");
*   #XM1 @文字(key="label", 内容="${名字}，W=${w}") => @放在(3, -3);
*   %jo = (0, 3); %jg = (-3, 3); %vo = (5, 3);
*   &out @导线 {#XM2:d %jo #XM1:d};
*   &out @WIRE {%jo %vo};
*   &in @导线 {#XM2:g %jg #XM1:g};
*   &in @WIRE {%jg #V2:1};
*   $JUNCTION @ATTACH {%jo %jg};
*   %vo @文字(内容="V_{out}", dx=0.3)
* }
* @RNL(type="generated", priority="high") {
*   #XM1 @PLACE(0, 0); #XM2 @PLACE(0, 6); #V1 @PLACE(9, 4); #V2 @PLACE(-8, 0);
*   %gs_XM1_3 = $GND_STUB @ATTACH #XM1:3; %gs_XM1_3::g @PLACE(0, -4);
*   %gs_V1_2 = $GND_STUB @ATTACH #V1:2; %gs_V1_2::g @PLACE(9, -1);
*   %gs_V2_2 = $GND_STUB @ATTACH #V2:2; %gs_V2_2::g @PLACE(-8, -5);
*   %ss_XM2_3 = $SUPPLY_STUB @ATTACH #XM2:3; %ss_XM2_3::s @PLACE(0, 10);
*   %ss_V1_1 = $SUPPLY_STUB @ATTACH #V1:1; %ss_V1_1::s @PLACE(9, 9)
* }
.end
