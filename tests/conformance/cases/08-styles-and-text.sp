* Inverting amplifier: styles, layers, notes and rich text
* @RNL(type="header") { @VERSION("0.2")}
*! Styles: inheritance ({$base}), a direct assignment over one by selection (whatever the order), a graphic's own parameters highest, a label overridden item by item (only the color); SET-SYMBOL directly and by rule (an anonymous nested definition, 5.6);
*! line style parameters of wires; a LAYER background; note graphics in the scene (a box, an arrow, linestyle="none" filling without a stroke); rich text: nested sub- and superscripts, bold italic, overline, Greek letters, fractions, line breaks, escapes, ${x}, and a warning for an unknown markup;
*! a text is aligned on its baseline by default (the anchor on the first line's baseline), by its box when valign is written; dx and dy make a text follow the point it is attached to.
.subckt opamp inp inn out
E1 out 0 inp inn 1e5
* @RNL(type="generated", priority="high") {
*   #E1 @PLACE(0, 0);
*   &inp @SELECT-ANCESTOR-USAGE $IOPORT => @PLACE(-4, 1);
*   &inn @SELECT-ANCESTOR-USAGE $IOPORT => @PLACE(-4, -1);
*   &out @SELECT-ANCESTOR-USAGE $IOPORT => @PLACE(0, 3) => @ORIENT("R270");
*   %gs_E1_2 = $GND_STUB @ATTACH #E1:2; %gs_E1_2::g @PLACE(0, -5)
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
*   # @SELECT-INSTANCE => @SET-STYLE $theme;
*   # @SELECT-USAGE => @SET-STYLE $theme;
*   # @SELECT-WIRE => @SET-STYLE $theme;
*   # @SELECT-LABEL => @SET-STYLE $theme;
*   # @SELECT-ANNOTATION => @SET-STYLE $theme;
*   # @SELECT-INSTANCE(name="R*") => @SET-STYLE $grey;
*
*   #Rin @SET-SYMBOL { @DEFINE-SYMBOL {
*     (0, 3) @PIN(terminal=1, direction="up"); (0, -3) @PIN(terminal=2, direction="down");
*     (0, 0) @RECTANGLE(width=1.2, height=3.6); @LINE {(0, 3) (0, 1.8)}; @LINE {(0, -1.8) (0, -3)};
*     ? @TEXT("${name}", key="name") => @PLACE(1.1, 0.25);
*     ? @TEXT("${value}", key="value") => @PLACE(1.1, -0.95)
*   }};
*   # @SELECT-INSTANCE(name="R*") => @SET-SYMBOL $RES;
*   #X1 @SET-SYMBOL $OPAMP;
*   $blue_bold = @DEFINE-STYLE(textcolor="#1565c0", bold=true) {};
*   #Rf::name @SET-STYLE $blue_bold;
*   #Rin @TEXT("R_{in}", key="name");
*
*   %ji = (5.5, 1); %jo = (15, 0); %vo = (18, 0);
*   &in @WIRE(linestyle="double", linewidth=0.08) {#Vin:1 #Rin:1};
*   &inn @WIRE {#Rin:2 %ji #X1:2};
*   &inn @WIRE(mode="vertical-horizontal", linestyle="dash-dot") {%ji #Rf:1};
*   &out @WIRE(mode="horizontal-vertical") {#Rf:2 %jo};
*   &out @WIRE(linestyle="bold", linecolor="#2e7d32") {#X1:3 %jo %vo};
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
* @RNL(type="generated", priority="high") {
*   #Vin @PLACE(-4, -2); #Rin @PLACE(2, 1) => @ORIENT("R90"); #Rf @PLACE(10, 5) => @ORIENT("R90"); #X1 @PLACE(10, 0);
*   %gs_Vin_2 = $GND_STUB @ATTACH #Vin:2; %gs_Vin_2::g @PLACE(-4, -7);
*   %gs_X1_1 = $GND_STUB @ATTACH #X1:1; %gs_X1_1::g @PLACE(6, -3)
* }
.end
