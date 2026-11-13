* Inverting amplifier: styles, layers, notes and rich text
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-basic", version="0.4"); @USE-PACKAGE(name="rnl-std-functional", version="0.4"); @VERSION("0.4")}
*! Styles: inheritance ({$base}), a direct assignment over one by selection (whatever the order), a graphic's own parameters highest, a label overridden item by item (only the color); SET-SYMBOL directly and by rule (an anonymous nested definition, 5.6);
*! line style parameters of wires; a LAYER background; note graphics in the scene (a box, an arrow, linestyle="none" filling without a stroke); rich text: nested sub- and superscripts, bold italic, overline, Greek letters, fractions, line breaks, escapes, ${x}, and a warning for an unknown markup;
*! a text is aligned on its baseline by default (the anchor on the first line's baseline), by its box when valign is written; dx and dy make a text follow the point it is attached to.
.subckt opamp inp inn out
E1 out 0 inp inn 1e5
* 






* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:9dafd040271e4dc3.ffc36700559c31e7") {
*   %ground_stub_E1_n = $GROUND_STUB @ATTACH(gap=1) #E1::n;
*   %net_label_stub_E1_cp = $NET_LABEL_STUB @ATTACH #E1::cp;
*   %net_label_stub_E1_cn = $NET_LABEL_STUB @ATTACH #E1::cn;
*   %net_label_stub_E1_p = $NET_LABEL_STUB @ATTACH #E1::p;
*   #E1 @PLACE(4, -2) => @ORIENT("R0");
*   %ground_stub_E1_n @PLACE(0, 0);
*   %ground_stub_E1_n::g @PLACE(4, -5) => @ORIENT("R0");
*   %net_label_stub_E1_cp @PLACE(0, 0);
*   %net_label_stub_E1_cp::l @PLACE(0, -1) => @ORIENT("R180");
*   %net_label_stub_E1_cn @PLACE(0, 0);
*   %net_label_stub_E1_cn::l @PLACE(0, -3) => @ORIENT("R180");
*   %net_label_stub_E1_p @PLACE(0, 0);
*   %net_label_stub_E1_p::l @PLACE(4, 0) => @ORIENT("R90")
* }
.ends
Vin in 0 AC 1
Rin in inn 10k
Rf inn out 100k
X1 0 inn out opamp
* @RNL {
*   $theme = @DEFINE-STYLE(linecolor="#37474f", textcolor="#37474f") {$base};
*   $hot = @DEFINE-STYLE(linecolor="#c62828", textcolor="#c62828") {};
*   #Rf @SET-STYLE $hot;
*   # @SELECT-INSTANCE(recursive=true) => @SET-STYLE $theme;
*   # @SELECT-USAGE(recursive=true) => @SET-STYLE $theme;
*   # @SELECT-WIRE(recursive=true) => @SET-STYLE $theme;
*   # @SELECT-LABEL(recursive=true) => @SET-STYLE $theme;
*   # @SELECT-ANNOTATION(recursive=true) => @SET-STYLE $theme;
*   # @SELECT-INSTANCE(name="R*", recursive=true) => @SET-STYLE $grey;
*
*   #Rin @SET-SYMBOL { @DEFINE-SYMBOL {
*     (0, 3) @PIN(terminal=1, direction="up"); (0, -3) @PIN(terminal=2, direction="down");
*     (0, 0) @RECTANGLE(width=1.2, height=3.6); @LINE {(0, 3) (0, 1.8)}; @LINE {(0, -1.8) (0, -3)};
*     ? @TEXT("${name}", key="name") => @PLACE(1.1, 0.25);
*     ? @TEXT("${value}", key="value") => @PLACE(1.1, -0.95)
*   }};
*   # @SELECT-INSTANCE(name="R*", recursive=true) => @SET-SYMBOL $RESISTOR;
*   #X1 @SET-SYMBOL $OPAMP;
*   $blue_bold = @DEFINE-STYLE(textcolor="#1565c0", bold=true) {};
*   #Rf::name @SET-STYLE $blue_bold;
*   #Rin @TEXT("R_{in}", key="name");
*
*   %ji = (5, 1); %jo = (15, 0); %vo = (18, 0);
*   &in @WIRE(mode="auto", linestyle="double", linewidth=0.08) {#Vin:1 #Rin:1};
*   &inn @WIRE(mode="auto") {#Rin:2 %ji #X1:2};
*   &inn @WIRE(mode="auto", linestyle="dash-dot") {%ji #Rf:1};
*   &out @WIRE(mode="auto") {#Rf:2 %jo};
*   &out @WIRE(mode="auto", linestyle="bold", linecolor="#2e7d32") {#X1:3 %jo %vo};
*   $JUNCTION @ATTACH {%ji %jo};
*   %vo @TEXT("V_{out}", dx=0.4, valign="middle");
*
*   (5, 0.5) @RECTANGLE(width=24, height=14, fill="fillcolor", fillcolor="#1f4fd81a", linecolor="#1f4fd8", linestyle="dotted") => @LAYER("bottom");
*   @LINE(arrow="end", linecolor="#c62828") {(17, 6) (15.3, 0.6)};
*   (16.6, 6.95) @CIRCLE(radius=0.25, fill="linecolor", linestyle="none", linecolor="#c62828");
*   (17.2, 6.6) @TEXT("A_{v} = −\frac{R_{f}}{R_{in}} = −10", textcolor="#c62828");
*   (-7, 9) @TEXT("\textbf{Inverting amplifier}\\\textit{V}_{\textit{DD}} = 1.8 V, I_{D_{1}} = 10 \mu A, \overline{RST} \pm 5\%");
*   (-7, -9) @TEXT("escapes, a\_b \$5 \${x} \{ \} C:\temp, unknown \sqrt{2}", size=0.8);
*   %p = (-7, -11);
*   %p @TEXT("this note is at (${x}, ${y})", size=0.8);
*   (-9, 0) @TEXT("rot=90 note", rotate=90, halign="middle", size=0.8)
* }
* 




* Fixed device positions retain the reviewed layout; the 0.4 solver record fills the remaining objects.
* @RNL(priority="high") {
*   #Vin @PLACE(-4, -2) => @ORIENT("R0");
*   #Rin @PLACE(1, 1) => @ORIENT("R90");
*   #Rf @PLACE(10, 5) => @ORIENT("R90");
*   #X1 @PLACE(10, 0) => @ORIENT("MX");
* }
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:d48ff0ad97acb92e.ffc36700559c31e7", numbering="c40ba99df6c0ad77") {
*   %ground_stub_Vin_n = $GROUND_STUB @ATTACH(gap=1) #Vin::n;
*   %ground_stub_X1_inp = $GROUND_STUB @ATTACH(gap=1) #X1::inp;
*   #Vin @PLACE(-4, -2) => @ORIENT("R0");
*   #Rin @PLACE(1, 1) => @ORIENT("R90");
*   #Rf @PLACE(10, 5) => @ORIENT("R90");
*   #X1 @PLACE(10, 0) => @ORIENT("MX");
*   %#6 @PLACE(5, 1) => @ORIENT("R0");
*   %#7 @PLACE(15, 0) => @ORIENT("R0");
*   %ground_stub_Vin_n @PLACE(0, 0);
*   %ground_stub_Vin_n::g @PLACE(-4, -5) => @ORIENT("R0");
*   %ground_stub_X1_inp @PLACE(0, 0);
*   %ground_stub_X1_inp::g @PLACE(4, -5) => @ORIENT("R0");
*   %ji @PLACE(5, 1);
*   %jo @PLACE(15, 0);
*   %vo @PLACE(18, 0);
*   %#9 @PLACE(5, 0.5);
*   %#12 @PLACE(17, 6);
*   %#13 @PLACE(15.3, 0.6);
*   %#14 @PLACE(16.6, 6.95);
*   %#16 @PLACE(17.2, 6.6);
*   %#18 @PLACE(-7, 9);
*   %#20 @PLACE(-7, -9);
*   %p @PLACE(-7, -11);
*   %#23 @PLACE(-9, 0);
*   %#8 @PLACE(y=0);
*   %#17 @PLACE(17.2, 6.6);
*   %#19 @PLACE(-7, 9);
*   %#21 @PLACE(-7, -9);
*   %#22 @PLACE(-7, -11);
*   %#24 @PLACE(-9, 0)
* }
.end
