# RNL 0.4 簡明指南

RNL 寫在 SPICE 或 Spectre 網表的註解裡，用來說明原理圖的繪製方式與電路各部分的含義，絕不會更動電路本身。本文對應語言版本 `"0.4"` 與標準套件系列 `"0.4"`，文中如（14.8）之類的編號皆指向語言標準的相應章節。各節範例皆為獨立的寫法示範，並非同一份完整程式；實際撰寫時，須在物件所屬的層級中引用既有物件，並將程式碼片段放入 RNL 記錄內（範例裡的 `/* */` 註解僅供說明）。文末目錄以簽名形式列出標準套件的所有名稱，專供查閱檢索，不可直接當作 RNL 敘述照抄。

## 檔案結構

```spice
* Two resistors                                   /* line 1 is the title, never read */
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
R1 a b 1k
R2 b 0 1k
* @RNL {
*   {#R1 #R2} @ALIGN("x"); /* every statement ends with ; */
*   &b @WIRE {#R1:n #R2:p};
* }
.end
```

- RNL 記錄寫在整行註解中，第 1 欄為 `*` 後接一個空格；在 Spectre 模式（`.scs` 檔案，或 `simulator lang=spectre` 之後）下亦可使用 `//`。檔頭記錄須置於最前；寫在 `.subckt … .ends` 內部的記錄則歸屬該子電路。
- 若未透過 USE-PACKAGE 引入標準套件，系統將無法辨識任何標準名稱。既有檔案所提供的檔頭記錄應維持原樣，其中的 `@DEVICE` 敘述用來為 PDK 器件設定端子名稱與類型（6.12），請勿自行撰寫或修改。
- 每個 `@` 前方都須留一個 ASCII 空格；參數小括號緊貼關鍵字，角色小括號緊貼物件，中間均不留空白。位置參數須排在具名參數之前，例如 `@PLACE(10, y=20)`。數值不帶單位，語言也不支援運算式；RNL 自身的名稱與關鍵字區分大小寫，網表名稱則依循宿主語言的規則。

## 引用

```rnl
#M1; /* an instance of this level; # alone is the level */
&vdd; &"vdd!"; /* nets; quote other characters */
#M1:d; #M1:1; /* a terminal, by name or number: the same one */
#M1::d; /* the pin of M1's symbol that draws terminal d */
#:inp; /* a port of this subcircuit */
%dp1; $GROUND; /* a name of this level; a definition, symbol, style or shared set */
%g::pin; %dp1::left; /* a part: the pin of a ground, the left role of a pair */
(10, 20); (0, _); /* points; _ leaves an axis free */
{#M1 #M2 #M3}; /* a set, in order */
&D<3:0>; /* D<3> D<2> D<1> D<0> */
%row_set{1}; %row_set{end}; %row_set{2:end}; /* members of a written set by place */
```

- 各類實例的標準端子名稱如下：M 為 `d g s b`，Q 為 `c b e s`，J、Z 為 `d g s`，D 為 `a k`，R、C、L、V、I、B、F、H、W 為 `p n`，E、G、S 為 `p n cp cn`，T、O 為 `p1 n1 p2 n2`，U 為 `a b ref`，X 則採用子電路各連接埠的名稱。凡是需要傳入接腳的位置，皆可直接寫出端子以代表其對應接腳。
- 每個名稱在同一層級內僅能綁定一次，且可在定義之前先行引用；若要移動已命名的 `%p`，須寫 `%p @PLACE(...)`，而不可再寫一條 `%p = ...`。`%` 名稱僅在當前層級有效，`$` 定義則會由下層級繼承，並可被下層級的同名定義遮蔽。
- 不帶前綴的詞語一律視為自由文字（若恰與既有物件同名會發出警告），因此 `#M1 #M2 M3 @ALIGN("y");` 實際上只會對齊 M1 與 M2。

## 敘述與管線

```rnl
#M1 @PLACE(10, 20); /* objects, one keyword, its parameters */
$GROUND @ATTACH &vss => @PLACE(20, -4); /* => passes the result on: the ground, not the net */
%g = $GROUND @ATTACH &vss; /* %x = names the result */
$GROUND @ATTACH &vdd => %g2 => @PLACE(20, 4); /* => %x names the segment before it */
#M1 #M2 @ALIGN("y"); /* loose objects are one set */
{#D1 #D2} @ORIENT("MX"); /* a one-object slot given a set does each member */
{#M1 #M2} @SET-SYMBOL $NMOS; /* two slots: exactly two objects */
{#R1 {#R2 #C2} #M1} @ORDER("left-to-right"); /* a set as one item stands for each member: R2 and C2 each between */
```

管線各段向下傳遞的結果取決於關鍵字：PLACE、ORIENT、SET-SYMBOL、SET-STYLE、GROUP 與 CURRENT 原樣傳遞其輸入物件；AS 與 ATTACH 傳遞新建的用例（逐一產生時則為用例集合）；ALIGN、ORDER、DISTRIBUTE、DISTANCE、TABLE 與 ORIENT-RELATION 傳遞新建的約束物件；TEXT 傳遞新建的文字物件；各類 SELECT-… 則傳遞篩選所得的集合。

## 定義、角色與用例

```rnl
$SIDE = @DEFINE(roles=["a", "b"], parameters=[["gap", 4]]) {
  {?a ?b} @ALIGN("y"); /* ?a: a role in the body */
  ?a ?b @DISTANCE("left-to-right", min=?gap); /* ?gap: a parameter */
};
#M3(a) #M4(b) @AS(gap=6) $SIDE; /* apply with roles marked, and a parameter */
#M3 #M4 @AS $SIDE; /* mark every role or none; without marks, use role order */
%dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N; /* every one-object role is filled: tail too */
{#M5 #M3} @AS $MATCHED; /* a set role takes the whole set: one usage */
{#M1 #M2} @AS $DIODE_CONNECTED; /* a one-object role given a set: one usage each */
$KIND = @DEFINE(roles=[["p", "net"], ["{at}", "point"]]) {}; /* common role kinds: instance terminal pin net point usage */
{%dp1::left #M9} @ALIGN("y"); /* place a structure through a member */
%g = $GROUND @ATTACH &vss; {%g::pin #M5::s} @ALIGN("x"); /* a virtual part is drawn at its own point; place it by its pin */
```

- 宣告為 `{role}` 的集合角色會整體接收一個集合（傳入單一物件時視為單一成員集合），套用時可以省略；但若採用不標角色名稱的依序傳入寫法，僅能省略排在尾端的集合角色。單一物件角色為必填。每次套用時每個角色至多標記一次，若有多個成員須合併放入同一個集合，例如 `{#R1 #R2}(members)`。
- 在定義本體中，單獨寫出的 `?` 代表該用例自身的點，字面座標皆為相對於該點的偏移量；以 `%mid = (_, _);` 建立的物件屬於該用例的區域物件，從外部寫作 `%u::mid` 存取。
- 具備擺放效果的標準結構會綁定用例自身的點：排成一列或一欄的成員將該點置於其直線上，互為鏡像的兩側則將該點置於兩者中心，因此對 `%u::position` 施加約束即可移動整個結構。文末目錄的「繪製」一欄標明了綁定方式：`$BRANCH` 僅與成員共用 x 座標，`$BRANCH_HORIZONTAL` 僅共用 y 座標，`$PARALLEL_HORIZONTAL` 與 `$MIRRORED_LAYOUT` 則取在成員的中心。任何其他結構皆讓該點保持自由，對其 `::position` 施加約束不會改變任何繪製結果。
- 撰寫時應優先選用既能標明電路語義、又能一併安排繪圖佈局的結構。例如 `$BRANCH` 是垂直堆疊的串聯結構，成員由上而下依序書寫，常用於電源到接地之間的直流支路；每條這類路徑應寫成一個 `$BRANCH`，路徑上若有並聯對，則搭配具備擺放效果的變體（單寫 `$PARALLEL` 僅負責走線而不擺放位置），如 `{#R1 #M1 #R2} @AS $BRANCH; {#R2 #C2} @AS $PARALLEL_HORIZONTAL;`。支路結構本身不描述電流方向，電流須另以 `@CURRENT` 標明。至於切換電容取樣器這類經由開關與電容傳遞的訊號路徑，則適合以原子事實直接繪製，例如對每一列的接腳分別寫出 ALIGN 與 ORDER：`{#S1:s #S1:d #C1:p #C1:n} @ALIGN("y"); {#S1:s #S1:d #C1:p #C1:n} @ORDER("left-to-right");`。標為「僅表語義」的結構不會繪製任何內容——在電路條件確實成立時可寫出以記錄語義，但切勿拿它取代具備擺放效果的結構。僅在確有把握的清晰訊號路徑上標註 `$SIGNAL_FLOW`；凡訊號分走多條路徑、形成回授或在大部分電路中雙向傳遞之處，無論路徑成員多寡皆不應標註。

## 篩選與規則

```rnl
# @SELECT-INSTANCE(model="nch", recursive=true) => @SET-STYLE $grey; /* a rule: here and in the levels below */
# @SELECT-INSTANCE(type="pmos") => @ORIENT("MY"); /* without recursive=true: this level only */
&en @SELECT-TERMINAL(terminal="g"); /* the gate terminals on net en */
# @SELECT-PIN(wired=false); /* pins without wires */
# @SELECT-NET(port=true); # @SELECT-NET(global=true); # @SELECT-NET(wired=false);
# @SELECT-USAGE $DIFFERENTIAL_PAIR_N => @SELECT-INSTANCE(role="left"); /* the left device of every pair */
&vss @ADD-TO $GROUND_NET_SET; /* edit a shared set of the package */
&agnd @REMOVE-FROM $GROUND_NET_SET;
```

- 篩選條件 `name`、`model`、`net` 與 `key` 皆支援萬用字元 `*`；`type` 則比對 `.model` 模型卡或 DEVICE 敘述所給出的器件類型。
- 篩選結果以及由篩選結果建構出的集合，既無法依索引位置取成員（如 `%sel{1}`），也不能傳給 WIRE；若需連線，請先為物件命名，或改用 ROUTE-OBJECT。

## 佈局事實

```rnl
#M1 @PLACE(10, 20); #M1 @PLACE(x=10); /* origin at (x, y); x right, y up, grid units */
#M2 @ORIENT("MY"); #M5 @ORIENT(["R0", "MY"]); /* an object takes a code; a list lets the solver choose */
#M1 #M2 @ORIENT-RELATION("MY"); /* M2 is M1 flipped left-right */
{#M1 #M2} @ALIGN("y"); /* one row; "x" is one column */
{#M1 #M2 #M3} @ORDER("left-to-right"); /* one after the other, at least 1 apart */
{#R1 #R2 #R3} @DISTRIBUTE("top-to-bottom"); /* in order, equal steps */
#M3 #M1 @DISTANCE("top-to-bottom", min=4); /* from the first to the second: min, max or exact, none negative */
{#M1 #M2 #M5} @GROUP; /* groups the set for the solver; no own origin or rigid transform */
@TABLE layout: {{#M1 #M2 #M3} {#M4 {#M5 #M6} #M7}}; /* rows top down, cells left to right, a cell column inside; one group; no alignment */
@TABLE(transpose=true) layout: {{#M1 #M2} {#M3 #M4}}; /* columns left to right */
```

- 方向參數可取 `"left-to-right"`、`"right-to-left"`、`"bottom-to-top"` 或 `"top-to-bottom"`。朝向代碼中，`R0`、`R90`、`R180`、`R270` 為逆時針旋轉；`MY` 為左右翻轉，`MX` 為上下翻轉；`MYR90`、`MXR90` 則為先鏡像翻轉再逆時針旋轉。
- 參與佈局約束的成員各自依其基準點計算位置：實例取符號原點，端子與接腳取接腳位置，點與文字取自身位置，有符號的用例則取自身的點。沒有符號的用例同 GROUP 一樣向下展開並代表其最底層成員；其自身的點為 `%u::position`。集合本身沒有原點，即使套用了 GROUP，對集合使用 PLACE 仍會分別擺放每個成員；接腳與約束物件則不可用 PLACE 擺放。語言中也沒有「最左方」這類相對於整張圖面的絕對方位，須透過與其他組件的相對關係來表達。
- 凡保留為自由狀態的部分皆由求解器決定，而求解器不會自行猜測組件的電路用途，因此所有必須成立的條件都須明確寫出。一般建議優先撰寫語義與約束並交由求解器自動擺放；若任務明確要求指定座標，則改為替所有物件寫齊 PLACE、ORIENT 與 WIRE。切勿混用兩種做法：要嘛只寫語義與約束交給求解器（僅寫極少數甚至不寫 PLACE），要嘛寫出全部位置而不做求解。
- 排序時應針對較大的區塊或支路下約束，而非逐一排列單顆器件，例如 `{#V2 #C1 %bias %stage #C3 #R5} @ORDER("left-to-right");` 即可將各欄整體由左至右並排展開。結構代表其成員，因此 `%stage` 的每顆器件皆位於 `%bias` 的每顆器件與 C3 之間；兩個結構共用的器件不會與自身排序。若要改為約束結構的中心，請寫出其自身的點：`{%dp::position #X1} @ALIGN("y");` 會將 X1 置於差動對中心的高度，而 `{%dp #X1} @ALIGN("y");` 則會將三顆電晶體皆置於 X1 的高度（目錄的「繪製」一欄說明了結構將其點綁於何處）。
- 電源與訊號源等外圍組件通常置於電路一側，與其餘部分稍微拉開距離，可用 DISTANCE 表達：`#VDD {#VIN #XU1 #CL} @DISTANCE("left-to-right", min=4);` 使電源維持在其餘各物件左側至少 4 個單位處。
- 為圖面建立清晰的群組樹：每個功能方塊自成一個群組，小群組巢狀包入更大的群組中，每個節點下直接放置約五、六個項目。許多結構本身已是群組（見「群組」一欄）；像電流鏡這類可能部分落在另一結構內部的結構則不是。完整結構或放大級連同其連接埠請自行框為群組：`%amp %p_inp %p_inn %p_out @GROUP;`。群組不可交叉：若兩個群組共用成員卻未形成巢狀包含，求解器僅會將其中一個保留為區塊。
- 唯有當讀者對某個物件有慣例上的朝向預期、且其他約束無法決定其朝向時，才需要明確固定朝向，常見於訊號源、電源與負載，如 `{#V1 #V2 #R5} @ORIENT("R0");`；其餘交由求解器決定即可，電路結構與電流方向通常已能定出絕大部分朝向。

## 接腳與方向

```rnl
#M1::d @ORIENT("up"); /* a pin takes a direction: left down right up */
#M1::g @ORIENT("left"); /* drain up and gate left: M1 is R0, nothing else */
#M3::s @ORIENT("up"); /* a PMOS upright */
#M1::g #M2::g @ORIENT-RELATION("R180"); /* two pins face opposite ways */
{#M3::d #M1::d} @ALIGN("x"); /* stack: the drains in one column... */
&x @WIRE {#M3:d #M1:d}; /* ...and wired, or the fill labels them */
%t = $IO_PORT @ATTACH &inp; %t::pin @ORIENT("right"); {%t::pin #M1::g} @ALIGN("y"); &inp @WIRE {%t::pin #M1::g};
$IO_PORT_STUB @ATTACH #M1:g; /* the same tag, put on the pin by the package */
```

- 只固定一支接腳的方向時，八種朝向中仍會留下兩種可能：僅固定閘極時仍可正立或倒置，僅固定汲極時閘極仍可朝左或朝右；若要唯一確定朝向，須固定兩支接腳。
- 標準符號於 R0 下的接腳方向：R、C、L 與各類源皆為 p 朝上、n 朝下；`$DIODE` 為 a 朝上、k 朝下；`$NMOS` 為 d 朝上、g 朝左、s 朝下，四端符號（4TERM）另有 b 朝右；`$PMOS` 為 s 朝上、g 朝左、d 朝下；`$NPN` 為 c 朝上、b 朝左、e 朝下；`$PNP` 為 e 朝上、b 朝左、c 朝下。

## 電流

```rnl
#M1:d @CURRENT("in"); #M1:s @CURRENT("out"); /* the static current at a terminal: in, out, unspecified */
{#M1:d #M2:d #M5:d} @CURRENT("in"); /* a set does each member */
#:out @CURRENT("in"); /* at a port: current enters the subcircuit */
#M1::d @CURRENT("in"); /* a pin stands for its terminal */
```

- CURRENT 本身不繪製任何圖形，而是說明電路的靜態電流走向，供求解器據以安排電流路徑；主要路徑上均應標明。
- 僅在確實流過靜態直流電流且方向明確的端子上標註：電容、閘極與不具直流值的訊號源沒有靜態電流，應予省略；流向不確定的端子也不要標註。電源同樣不要標記 CURRENT——電源內部電流是由 n 流向 p（由下往上），若標出後遇到偏好電流由上往下流的求解器，反而會把電源符號上下顛倒。
- 若同一條網路上的所有端子與連接埠都被標成將電流推往同一側，檢查時會回報一項違規；如 0 之類的全域網路則不予檢查。

## 優先順序

```rnl
#M5 @PLACE(x=3, priority="low"); /* disabled, low, normal (default), high */
%dp1 = #M1(left) #M2(right) #M5(tail) @AS(priority="high") $DIFFERENTIAL_PAIR_N; /* covers every fact of the body */
%dp1 @PRIORITY("disabled"); /* turns the default drawing of the pair off */
```

- 發生衝突時，優先順序較低的事實先被捨棄，優先順序相同時則先捨棄較早寫出者；凡是最終保留下來的事實皆為求解器必須滿足的硬性要求。標準套件預設繪圖的優先順序皆為 `"low"`。記錄標頭的 `priority=` 提供記錄內敘述的預設值，但 AS 與 ATTACH 僅會從中繼承 `"disabled"`。
- 結構本身無須指定優先順序：唯有在檢查或求解顯示發生衝突時，才寫出優先順序加以排解。

## 連線與走線

```rnl
%j = (_, _);
&b @WIRE {#R1:n %j #R2:p}; /* wires join only at shared point objects and terminals */
&b @WIRE(mode="manhattan") {%j #R3:p}; /* auto straight horizontal-vertical vertical-horizontal manhattan manhattan45 */
$JUNCTION @ATTACH %j; /* the dot is written */
&a @ROUTE-NET; /* the solver draws the whole net */
{#R3 #R4} @ROUTE-OBJECT; /* the connections among these objects' pins */
#R3:p @UNROUTE-OBJECT; &x @UNROUTE-NET; /* take pins out of trees; on one pin the higher priority, then the later, wins */
&vss @ROUTE-NET(priority="high"); $GROUND @ATTACH &vss; /* ground and supply nets are unrouted at high: wire them so, and attach a ground */
&D<1:0> @WIRE {%a %b}; /* several nets: a bus */
%all = # @SELECT-NET; %all @ROUTE-NET; /* every net of this level as wires; the fill keeps the ground, supply and bias nets unrouted at high, so they keep their stubs */
```

- 兩支接腳即使座標相同也不會自動相連；必須在兩者之間寫出一條 WIRE（即使長度為零），才能將接腳標為已連線，避免自動補齊再補上多餘短樁。
- 標準結構會以低優先順序自動繪製其內部連線；連接兩個結構的網路則預設以網路標籤呈現，除非另外寫出 ROUTE 敘述。
- 只將畫成實線更易閱讀的路徑設為連線，切勿全圖一律拉線：求解器僅繪製 WIRE、ROUTE-NET 與 ROUTE-OBJECT 所要求的連線，其餘未連線接腳皆由自動補齊加上接地、電源或網路標籤短樁。主訊號路徑以及相鄰結構內部與彼此之間的連接宜畫成連線；距離較遠或較擁擠的連接（如偏壓線及跨全圖共用的網路）則宜留給網路標籤。
- 在小型電路中，除電源、接地與偏壓軌以外的網路畫成連線通常都很清楚，可照上方範例最後一行的寫法統一走線，或寫 `# @SELECT-NET => @ROUTE-NET;`，兩者繪出的圖面相同。無論採用哪種寫法，自動補齊都會以高優先順序排除接地、電源與偏壓網路的走線，每條電源軌網路都會收到一則提示（RNL9403），指出其走線樹已失去所有接腳；這則提示是預期中的。
- 連接到所在層級連接埠的網路，只有寫出連接埠標記時才會連到連接埠：`$IO_PORT @ATTACH &x;`，或在它驅動的接腳上使用 `$IO_PORT_STUB`。自動補齊是在決定哪些接腳已連線之後才加入連接埠標記，因此只有一個自己的接腳的網路（例如只驅動一個閘極的輸入）會構成只有一個點的樹：該接腳得到網路標籤，沒有連線到達連接埠。

## 文字

```rnl
#M1 @TEXT("${w}/${l}", key="wl", dy=-3); /* a label that moves with M1; #M1::wl */
#M1 @TEXT("M_{in}", key="name"); /* the same key replaces the name the symbol shows */
(10, 10) @TEXT("bias", size=0.8); /* on a point: an annotation */
#M1 @TEXT("\overline{EN} \mu A \frac{W}{L}", halign="middle", valign="top", rotate=90);
```

- `${…}` 屬性引用可讀取：`name`、`value`、`model`、`type` 以及 `w` 等網表參數；端子或連線的 `net`；用例的角色名稱；以及如 `${pair::left::w}` 的多層屬性路徑。
- `dx`、`dy` 是文字相對於所附著物件的座標偏移量，而 `#M1::wl @PLACE(5, 0);` 則將標籤直接擺在圖面座標 (5, 0)；在每條座標軸上，PLACE 絕對位置與偏移量會互相取代，以較後寫出者為準。在符號定義本體內，兩者皆採用符號自身的區域座標。

## 符號與樣式

```rnl
$TRI = @DEFINE-SYMBOL {
  %in = (-4, 0) @PIN(terminal="in", direction="left"); /* a named pin: where, which terminal, which way */
  %out = (4, 0) @PIN(terminal="out", direction="right");
  @LINE {(-2, 2) (2, 0) (-2, -2) (-2, 2)};
  (0, 0) @RECTANGLE(width=4, height=2); (0, 0) @CIRCLE(radius=1);
  ? @TEXT("${name}", key="name") => @PLACE(-2, 2.5); /* ? is the object drawn */
};
#X1 @SET-SYMBOL $TRI; /* terminal names match by name, numbers by place */
#X2 @SET-SYMBOL(terminals=["inp", "inn", "out"]) $OPAMP; /* names differ: the symbol's names in node order */
# @SELECT-INSTANCE(model="nch", recursive=true) => @SET-SYMBOL $NMOS; /* by rule; a direct SET-SYMBOL wins */
%blk = @DEFINE-SYMBOL { %i = (-3, 0) @PIN(terminal="in", direction="left"); (0, 0) @RECTANGLE(width=6, height=3); };
# @SET-SYMBOL %blk; /* inside a .subckt: its block symbol, for its instances above */
$orange = @DEFINE-STYLE(linecolor="#e07000", textcolor="#e07000") {};
# @SELECT-INSTANCE => @SET-STYLE $grey; #M1 @SET-STYLE $orange; /* M1 is orange: direct beats rule, per property */
# @SELECT-USAGE(recursive=true) $GROUND => @SET-STYLE $grey; /* virtual parts are objects of their own */
```

- 樣式屬性包含 `linecolor linestyle linewidth fillcolor textcolor size font bold italic textvisible`，顏色採用 `"#RRGGBB"` 十六進位格式。`%` 名稱亦可用來命名區域定義。每項樣式屬性各自獨立結算：直接指定物件的設定優先於篩選規則；同一位階內則以較後寫出者為準。

## 檢查

標準套件中的結構會自動檢查其所宣稱的電路條件，例如差動對兩側是否具備相同模型與尺寸、串聯結構是否確實頭尾串接、pmos 角色填入的是否確為 PMOS 電晶體、連接埠標記是否掛在真正的連接埠網路上等。未通過的檢查會產生一項**違規**：它僅是一份診斷回報，不會使任何項目失效，圖面繪製結果也不會改變。遇到違規時，請修正結構或角色配置，或改用宣稱條件較少的一般結構。

## 標準套件

```rnl
@USE-PACKAGE(name="rnl-std-all", version="0.4"); /* everything: styles, symbols, rules for SPICE and Spectre, fill, structures, the blocks */
@USE-PACKAGE(name="rnl-std-no-labels", version="0.4"); /* optional, besides rnl-std-all: hides the labels symbols draw (name, value, model, net names); a label you write shows only with textvisible=true */
@USE-PACKAGE(name="rnl-std-45deg", version="0.4"); /* optional: symbols at 45 degrees */
@USE-PACKAGE(name="rnl-std-fill") {}; /* an empty package drops that part */
&vdd @ADD-TO $SUPPLY_NET_SET; &vss @ADD-TO $GROUND_NET_SET; /* supply and ground stubs on their pins; 0 is a ground already */
$PORT_LIST @ATTACH; /* a port tag on every port net, in one column */
$FIGURE @ATTACH %p => @SET-SYMBOL $MY_FIG; /* no netlist instance: use a symbol with unbound pins and no ${name}/${value}; not $RESISTOR */
```

- 標準套件預設指派的符號如下：R 為 `$RESISTOR`，C 為 `$CAPACITOR`，L 為 `$INDUCTOR`，V 為 `$VOLTAGE_SOURCE`，I 為 `$CURRENT_SOURCE`，D 為 `$DIODE`，E 為 `$VCVS_4TERM`，G 為 `$VCCS_4TERM`，F 為 `$CONTROLLED_CURRENT_SOURCE`，H 為 `$CONTROLLED_VOLTAGE_SOURCE`，B 為 `$BEHAVIORAL_SOURCE`，S 為 `$VOLTAGE_CONTROLLED_SWITCH`，W 為 `$SWITCH`，T、O 為 `$TRANSMISSION_LINE`，U 為 `$RC_LINE`；電晶體依類型指派：nmos 為 `$NMOS_4TERM`，pmos 為 `$PMOS_4TERM`，npn 為 `$NPN`，pnp 為 `$PNP`，njf 為 `$NJFET`，pjf 為 `$PJFET`；其中基體接在接地網路上的 NMOS 與基體接在電源網路上的 PMOS 會自動改用三接腳符號 `$NMOS` 與 `$PMOS`，無須手動撰寫 SET-SYMBOL。未知類型的電晶體（既無 `.model` 模型卡，亦無對應的 DEVICE 宣告）沒有器件符號，畫成方框，每個節點一支接腳（9.2.8）。
- 請如上方範例所示，將各電源網路加入 `$SUPPLY_NET_SET`，並將 0 以外的各接地網路加入 `$GROUND_NET_SET`，如此其上未連線的接腳便會獲得對應短樁，且網路本身保持不走線。自動補齊會略過已連有連線或標有未連接標記的接腳；其餘未連線的接腳（器件的或虛擬組件的）若位於 `$GROUND_NET_SET`（預設含網路 0）上會補上 `$GROUND_STUB`，位於 `$SUPPLY_NET_SET`（預設為空）上會補上 `$SUPPLY_STUB`，位於 `$BIAS_NET_SET`（預設為空）上會補上 `$BIAS_STUB`，位於其他網路上則補上 `$NET_LABEL_STUB`，而四端 MOS 的基體接腳無論接在何種網路上皆會補上 `$NET_LABEL_STUB`。每個連接埠網路（含電源與接地連接埠）都會自動補上一個 `$IO_PORT`，在本層級所有實例左側至少 6 個單位處排成一欄，均取 R0；若某個連接埠不需標記，可寫 `&vdd @REMOVE-FROM $PORT_FILL_SET;`，若要自行擺放標記，則寫 `$IO_PORT @ATTACH &out;`。連接埠標記本身即透過其 `netname` 文字顯示網路名稱，切勿重複附著顯示該名稱的 TEXT，若要移動名稱位置可寫 `%t::netname @PLACE(x, y);`；測試台內部的節點名稱則請改用 `$NET_LABEL`。已掛有 `$IO_PORT_STUB` 或 `$NET_LABEL_STUB` 的接腳同樣不會再補網路標籤（儘管它們的短引線是由符號自身繪出而非 WIRE）。`*_FILL_SET` 系列集合專供自動補齊放置自身的短樁與標記使用：一旦求解將結果寫回記錄，這些集合便不再保存已處理的接腳與網路，因此請勿在其上建立其他規則（例如對其接腳加上 TEXT），而應自行篩選接腳。若要調整哪些接腳獲得短樁，可對這些集合增刪接腳；注意，使用者自訂規則所附掛的短樁不會被自動補齊看見，自動補齊仍會照常再補上一個。

## 求解器記錄

```spice fragment
* @RNL(type="generated", priority="high", author="rnl-solver") { #R1 @PLACE(0, 4); #R2 @PLACE(0, -4);} /* never edit; replaced on each solve */
* @RNL(author="editor") { #R2 @PLACE(y=-6, priority="high");} /* to fix a position: high, in a later record */
```

諸如 `%#3` 的無名編號僅會出現在求解器產生的記錄中；手寫 RNL 時請先為物件命名再行引用。

## 其他關鍵字

以下關鍵字多由套件或工具寫出，平時極少手寫，能夠讀懂即可：

```rnl
@VERSION("0.4"); @PACKAGE(name="my-pkg", version="1.0"); /* header records */
@DEVICE(model="sky130_fd_pr__nfet_01v8*", terminals=["d", "g", "s", "b"], type="nmos"); /* names and types of devices */
@ALIAS("@at", "@PLACE"); @USE-ALIASES(name="alias-zh"); /* other spellings */
@SOLVER-CONFIG(spacing_x=2, junction="$JUNCTION"); /* settings of the solver */
#XI<3:0> @BUNDLE; /* an array of instances drawn as one */
#X @LAYER("bottom"); /* drawing layer */
%u_set = %a_set %b_set @UNION; /* a set without repeats */
(0, 0) @ELLIPSE(width=4, height=2); @ARC {(0, 2) (2, 0) (0, -2)}; @BEZIER {(0, 0) (1, 1) (2, 1) (3, 0)}; /* more graphics */
&n1 @SELECT-WIRE(bus=true); #M1 @SELECT-LABEL(key="wl"); # @SELECT-ANNOTATION; #M1 @SELECT-ANCESTOR-USAGE $CURRENT_MIRROR_N;
{#M1 #M2} @ASSERT-INSTANCE(same=["model", "w", "l"]); {#M1:s #M2:s} @ASSERT-TERMINAL(same=["net"]);
{&inp &inn} @ASSERT-NET(port=true); {#R1 #R2 #R3} @ASSERT-TOPOLOGY("series"); /* the checks inside structures */
```

## 常見錯誤

下表左欄為無效或易生誤解的寫法；撰寫時請僅採用右欄的修正寫法，並代入你檔案中的名稱。

|錯誤寫法|修正寫法|
|---|---|
|`#M1 @PLACE (10, 20);`|`#M1 @PLACE(10, 20);`|
|`#M1 (left) #M2 (right) @AS $CROSS_COUPLED_PAIR;`|`#M1(left) #M2(right) @AS $CROSS_COUPLED_PAIR;`|
|`#M1 #M2 @SET-SYMBOL $NMOS;`|`{#M1 #M2} @SET-SYMBOL $NMOS;`|
|`#M5 #M3 @AS $MATCHED;`|`{#M5 #M3} @AS $MATCHED;`|
|`{#R1 #R2} @ORDER("x");`|`{#R1 #R2} @ORDER("left-to-right");`|
|`#R1 @PLACE(x=2u);`|`#R1 @PLACE(x=2);`|
|沿用 0.3 版名稱 `$DIFFERENTIAL_PAIR` 與角色 `in_p/in_n`|`#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;`|
|`#R1 @SET-SYMBOL $GROUND;`|加上接地符號應寫 `$GROUND @ATTACH &n;`，電阻本身保留原有符號|
|`%t #M1 @ALIGN("y");`（欲對齊兩者的接腳）|`{%t::pin #M1::g} @ALIGN("y");`|
|在非連接埠網路上寫 `$IO_PORT @ATTACH &n;`|`$NET_LABEL @ATTACH &n;`|
|vss 已在接地網路集合中時僅寫 `&vss @ROUTE-NET;`|`&vss @ROUTE-NET(priority="high");`|
|在手寫內容中使用 `%#3 @PLACE(1, 2);`|先為該物件命名，再使用其名稱引用|
|`#M1.d; M3; &vdd!;`|`#M1:d; #M3; &"vdd!";`|
|`{@VERSION("0.4");}`|`{ @VERSION("0.4");}`|

將連線接往三接腳符號 `$NMOS` 的基體端子會導致敘述失效，此時應改用四端符號 `$NMOS_4TERM`。未知類型的電晶體須補上模型卡、DEVICE 宣告或明確指定的符號。頂層規則須加上 `recursive=true` 才會觸及下層子電路。

## 完整範例

```spice
* Five-transistor OTA
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
.model nch nmos level=1
.model pch pmos level=1
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M3 x    x     vdd  vdd pch W=4u L=0.5u
M4 out  x     vdd  vdd pch W=4u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   &vss @ADD-TO $GROUND_NET_SET; /* grounds and supplies instead of labels */
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   %dp1 = #M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N; /* mirrored about the tail, tail below */
*   %cm1 = #M3(ref) #M4(out) @AS $CURRENT_MIRROR_P;
*   {%dp1 %cm1} @ROUTE-OBJECT; /* the nets between them as wires */
*   {#M1::d #M2::d #M5::d} @ORIENT("up"); /* NMOS upright */
*   #M1::g @ORIENT("left"); /* gates outwards */
*   {#M3::s #M4::s} @ORIENT("up"); /* PMOS upright */
*   {#M3::d #M1::d} @ALIGN("x"); /* each load over its input device */
*   {#M4::d #M2::d} @ALIGN("x");
*   #M3 #M1 @DISTANCE("top-to-bottom", min=4);
*   {#M1:d #M2:d #M5:d #M3:s #M4:s} @CURRENT("in"); /* current down each branch */
*   {#M1:s #M2:s #M5:s #M3:d #M4:d} @CURRENT("out");
*   $PORT_LIST @ATTACH; /* the ports in one column */
*   {#M1 #M2} @TEXT("${w}/${l}", key="wl", dy=-3);
* }
.ends ota5t
.end
```

交回檔案前請逐項確認：檔頭記錄置於最前且包含 VERSION 與所用套件；所有名稱皆存在於所屬層級；僅使用 0.4 版標準名稱（以文末目錄為準）；每個 `@` 前皆有空格、每條敘述結尾皆有 `;`；約束方向均已寫明；集合角色傳入集合；接地與電源網路已加入對應集合；僅在必要處固定朝向、僅在流向確定處標註電流；已對齊的相接接腳均已連上連線；未手動修改產生記錄；且沒有非預期的違規。

<!-- lists:begin -->
## 名稱目錄

本目錄僅供查閱簽名，請勿直接複製為 RNL 程式碼：使用時請保留名稱前的 `$` 前綴；以 AS 或 ATTACH 套用定義，以 SET-SYMBOL 指派符號，以 SET-STYLE 指派樣式。簽名中的角色依宣告順序列出，並註明可接收的物件種類（未註明種類的角色可接收任何種類的物件）；寫作 `{role}` 的集合角色接收整個集合且可省略；定義參數列於分號 `;` 之後。例如 `$MATCHED({members}: instance/usage)` 在實際使用時寫作 `{#M1 #M2} @AS $MATCHED;`。

### 套件

以下各套件均於檔頭記錄中以 `@USE-PACKAGE(name="...", version="0.4");` 引入。

|套件|內容|
|---|---|
|rnl-std-all|0.4 標準系列唯一的套件集合，自身不含具體定義，而是透過對應宿主語言的檔頭記錄一併引入樣式套件、基礎與半導體符號及定義套件、自動補齊套件、電路語義套件、四個領域套件，以及各宿主方言的規則套件（2.8、16.3）。|
|rnl-std-style|標準樣式定義（9.3）與預設樣式規則。|
|rnl-std-basic|被動元件、獨立源與受控源、開關、傳輸線、虛擬組件與標記的符號；虛擬組件、標記、獨立圖形與短樁的定義（各附繪製其用例的規則）；以及接地、電源與偏壓網路的共用集合。|
|rnl-std-semiconductors|二極體以及 MOS、雙極性（BJT）、JFET 與單接面（UJT、PUT）電晶體的符號。|
|rnl-std-spice|SPICE 方言規則套件：依網表元素首字母、模型類型以及基體或負控制端所接網路，自動為實例指派符號（16.3）。|
|rnl-std-spectre|Spectre 方言規則套件：依 master 名稱、模型類型以及基體或負控制端所接網路，自動為實例指派符號（16.3）。|
|rnl-std-fill|自動補齊的集合與規則：為未連線的接腳（含虛擬組件的接腳）加上接地、電源、偏壓與網路標籤短樁，並為 `$PORT_FILL_SET` 中的每個連接埠網路加上連接埠標記（10.5、12.4、16.3）。|
|rnl-std-semantics|各類電路結構及其預設繪圖佈局的語義定義。|
|rnl-std-functional|行為模型功能方塊符號：運算放大器、比較器、放大器、轉導放大器、ADC、DAC 與斬波調變器，另提供帶有整數格點電源接腳 vp（頂部）與 vn（底部）的 *_SUPPLY 變體。|
|rnl-std-rf|射頻組件符號：中心抽頭電感與 T 型線圈（T-coil）。|
|rnl-std-logic|邏輯閘、緩衝器、反相器、延遲單元與 D 型正反器符號。|
|rnl-std-signal-flow|理想訊號流虛擬組件及其符號：加法器、減法器、乘法器、連續時間與離散時間積分器、量化器及單位延遲。|
|rnl-std-no-labels|隱藏標準符號的所有預設標籤，供自行撰寫全部標籤的檔案使用（16.3）。|
|rnl-std-no-instance-labels|僅隱藏實例的預設標籤，同時保留接地、電源、偏壓標記、連接埠標記與網路標籤所繪出的網路名稱（16.3）。|
|rnl-std-45deg|適用於橋式電路、交叉耦合對、對角線變壓器繞組與 45 度連線的 45 度傾角符號。|

### 定義

針對 rnl-std-semantics 中的電路結構，「繪製」一欄標示其套用效果：「僅表語義」既不擺放位置也不走線；「擺放」對成員施加位置事實；「走線」自動連接成員共用的內部網路。結構內建的位置事實與走線優先順序皆為 low。「群組」一欄讀自各定義的定義本體：「是」後列其歸為同一群組的角色，「經由」 `$X` 後列其傳給建立群組之定義 `$X` 的角色，留空則表示不建立群組；像電流鏡這類可能部分落在另一結構內部的結構不建立群組（見「佈局事實」一節下的群組樹）。沒有符號的用例在 ALIGN、ORDER、DISTRIBUTE、DISTANCE 中代表其成員；它自身的點 `%u::position`（14.1）僅在標註「自身的點」的結構中才與其成員綁定：只有這類用例可憑其 `::position` 作為單一項目移動，或作為另一擺放結構的成員；其餘結構則請約束其成員，或由成員組成的具名集合。其他套件定義的則是實際繪出的組件與標記。

|簽名|繪製|群組|含義|
|---|---|---|---|
|**rnl-std-basic**||||
|`$GROUND(p: net)`|—||網路上的接地符號，繪製於用例自身的點：在網路 0 上僅畫三條接地橫線，在其他任何網路上則一併顯示網路名稱。|
|`$SUPPLY(p: net)`|—||網路上的電源符號，繪製於用例自身的點。|
|`$BIAS(p: net)`|—||網路上的偏壓標記，繪製於用例自身的點。|
|`$IO_PORT(p: net)`|—||網路上的連接埠標記，繪製於用例自身的點；所接網路須為所在層級的連接埠。|
|`$NET_LABEL(p: net/pin)`|—||網路標籤：帶有一小段引線的網路名稱，繪製於用例自身的點；傳入匯流排接腳（如疊畫的接腳，11.8）時會保留該接腳，並以單一標籤代表其所有網路、一併顯示各網路名稱（13.6）。|
|`$NO_CONNECT(t: terminal/pin)`|—||標在刻意留空的端子或接腳上的未連接標記；傳入端子時會原樣保留該端子，使標在匯流排單一位元上的標記能記錄具體哪一個位元開路（12.6）。|
|`$JUNCTION(p: point/terminal)`|—||繪製在點或端子上的連線交接圓點。|
|`$CURRENT_ARROW(p: terminal/point)`|—||繪製在端子或點上的電流方向箭頭。|
|`$ARROW(at: point)`|—||繪製在符號內部或場景某一點上的箭頭（9.2.7）。|
|`$SPDT_SWITCH(at: point)`|—||在指定點上作為圖形繪製的單刀雙擲開關，無對應網表行。|
|`$FIGURE(at: point)`|—||在指定點上繪製的獨立圖形，無對應網表行，須搭配 SET-SYMBOL 指派其符號。|
|`$PORT_LIST()`|—||為當前層級中尚未寫出 `$IO_PORT` 的每個連接埠網路加上連接埠標記，由上而下排成一欄並成組、置於本層級所有實例左側至少 6 個單位處，各採 R0 朝向且接腳朝右，優先順序皆為 low，畫法與 rnl-std-fill 相同；使用後自動補齊在該層級不再另加標記（12.4）。|
|`$COMMENT({target}; text="")`|—||附著於任意物件上的非繪製註解；省略 target 時附著於當前層級本身。|
|`$GROUND_STUB(t: pin; gap=3)`|—||靠近接腳的正立接地短樁，自帶連接線：與接腳等高或位於其下方，垂直方向相距至多 gap 個單位（優先順序 low）。|
|`$SUPPLY_STUB(t: pin; gap=3)`|—||靠近接腳的正立電源短樁，自帶連接線：與接腳等高或位於其上方，垂直方向相距至多 gap 個單位（優先順序 low）。|
|`$BIAS_STUB(t: pin)`|—||直接貼在接腳上的偏壓短樁，如同連接埠短樁般背離接腳向外延伸；其短斜槓在接腳上方、下方與左側呈 / 斜向，在右側則呈 \ 斜向。|
|`$IO_PORT_STUB(t: pin)`|—||直接貼在接腳上的連接埠標記短樁，以符號自身的引線充當短連線；與 `$NET_LABEL_STUB` 相同，該接腳與標記皆保持不走線。|
|`$NET_LABEL_STUB(t: pin)`|—||直接貼在接腳上的網路標籤短樁，以符號自身 1 單位長的引線充當短連線；掛在匯流排接腳上時整組匯流排共用一個標籤；該接腳與標籤皆不走線。|
|**rnl-std-semantics**||||
|`$MATCHED({members}: instance/usage)`|僅表語義||在器件類型（或結構定義）以及佈局上相互匹配的器件或子結構。|
|`$MATCHED_MOS({members}: instance)`|僅表語義||類型、模型與通道長度皆匹配的 MOSFET 組。|
|`$MATCHED_BJT({members}: instance)`|僅表語義||類型、模型與單位射極幾何尺寸皆匹配的雙極性電晶體組。|
|`$MATCHED_RESISTOR({members}: instance)`|僅表語義||類型與模型匹配的電阻組。|
|`$MATCHED_CAPACITOR({members}: instance)`|僅表語義||類型與模型匹配的電容組。|
|`$MIRRORED_LAYOUT(left: instance/usage, right: instance/usage, axis: point/instance/usage)`|擺放；自身的點位於中心||位於同一高度、關於給定軸線左右互為鏡像的兩個對稱物件：僅為鏡像佈局，不陳述任何電路含義。|
|`$MIRRORED_VERTICAL_LAYOUT(top: instance/usage, bottom: instance/usage, axis: point/instance/usage)`|擺放；自身的點位於中心||位於同一欄、關於給定水平軸上下互為鏡像的兩個對稱物件：僅為鏡像佈局。|
|`$BRANCH({members}: instance/usage)`|擺放、走線；自身的點綁定 x||垂直堆疊的串聯支路結構：各成員排成一欄由上而下依序串聯，常用於電源至接地之間的直流支路。|
|`$BRANCH_HORIZONTAL({members}: instance/usage)`|擺放、走線；自身的點綁定 y||繪製為橫向一列的串聯支路結構：各成員位於同一高度，由左至右依序串聯。|
|`$CASCADE({stages}: instance/usage)`|僅表語義||沿訊號路徑依序串級的多個放大或處理級。|
|`$SERIES({members}: instance/usage)`|走線||沿同一條電流路徑頭尾相接串聯的成員組。|
|`$PARALLEL({members}: instance/usage)`|走線||跨接在相同兩條網路之間並聯的成員組。|
|`$PARALLEL_HORIZONTAL({members}: instance/usage)`|擺放、走線；自身的點位於中心||在同一高度左右並排並聯的成員組，在直欄中合佔一個位置。|
|`$PARALLEL_VERTICAL({members}: instance/usage)`|擺放、走線；自身的點位於中心||在同一 x 座標上下堆疊並聯的成員組，在橫列中合佔一個位置。|
|`$SIGNAL_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|僅表語義||單向傳遞的訊號流：從訊號源出發，依書寫順序流經路徑上的各級，最終抵達負載。|
|`$POWER_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|僅表語義||功率傳遞路徑（功率流）：從電源或電池等來源出發，依書寫順序經由負責轉換或切換功率的各級，送往消耗功率的負載。|
|`$BIAS_CIRCUIT({members}: instance/usage)`|僅表語義||偏壓電路：如電流基準等用來為其他部分產生偏壓的器件或結構。|
|`$FEEDBACK_LOOP({forward}: instance/usage, {feedback}: instance/usage)`|僅表語義||回授迴路：由前向路徑與回授網路構成的閉迴路結構。|
|`$DIFFERENTIAL_PAIR_N(left: instance, right: instance, tail: instance/usage)`|擺放、走線；自身的點位於中心|是：left, right, tail|尾端位於下方的 n 型電晶體差動對。|
|`$DIFFERENTIAL_PAIR_P(left: instance, right: instance, tail: instance/usage)`|擺放、走線；自身的點位於中心|是：left, right, tail|尾端位於上方的 p 型電晶體差動對。|
|`$COMPLEMENTARY_DIFFERENTIAL_PAIR(pairup: usage, pairdown: usage)`|走線|是：pairup, pairdown|軌對軌互補輸入級：共用同一組差動輸入的上下兩組差動對。|
|`$PSEUDO_DIFFERENTIAL_PAIR(left: instance, right: instance)`|擺放；自身的點位於中心||偽差動對：不帶尾端電晶體、以左右鏡像繪製的兩顆匹配輸入電晶體。|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_H(left: instance, right: instance, tailleft: instance/usage, tailright: instance/usage, bridge: instance/usage)`|擺放、走線；自身的點位於中心|是：left, right, tailleft, tailright, bridge|H 型退化差動對：在左右兩條尾端支路之間跨接橋接組件以引入負回授，整體呈 H 型。|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_N(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|擺放、走線；自身的點位於中心|是：left, right, degleft, degright, tail|n 型 T 型退化差動對：在左右輸入電晶體下方各串入一個退化組件後再匯合至公共尾端，整體呈 T 型，尾端在下。|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_P(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|擺放、走線；自身的點位於中心|是：left, right, degleft, degright, tail|p 型倒 T 型退化差動對：在左右輸入電晶體上方各串入一個退化組件後再匯合至公共尾端，整體呈倒 T 型，尾端在上。|
|`$CROSS_COUPLED_PAIR(left: instance, right: instance)`|擺放、走線；自身的點位於中心|是：left, right|交叉耦合對：交叉互連構成正回授、以左右鏡像繪製的兩顆匹配電晶體。|
|`$CURRENT_MIRROR_N({ref}: instance, {out}: instance)`|擺放；自身的點綁定 y||n 型電晶體構成的低側電流鏡，源極或射極朝下。|
|`$CURRENT_MIRROR_P({ref}: instance, {out}: instance)`|擺放；自身的點綁定 y||p 型電晶體構成的高側電流鏡，源極或射極朝上。|
|`$CASCODE_CURRENT_MIRROR_N({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|擺放；自身的點綁定 y||低側疊接（cascode）電流鏡：底層為電流鏡列，上方堆疊疊接列。|
|`$CASCODE_CURRENT_MIRROR_P({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|擺放；自身的點綁定 y||高側疊接（cascode）電流鏡：頂層為電流鏡列，下方堆疊疊接列。|
|`$TRANS_LINEAR_LOOP({a}: instance/usage, {b}: instance/usage)`|擺放、走線；自身的點綁定 x|是：a, b|由兩組電晶體構成、以單一直欄方式排列的跨線性迴路（translinear loop）。|
|`$TRANS_LINEAR_LOOP_HORIZONTAL({a}: instance/usage, {b}: instance/usage)`|擺放、走線；自身的點綁定 y|是：a, b|由兩組電晶體構成、以單一橫列方式排列的跨線性迴路。|
|`$DIODE_CONNECTED(device: instance)`|走線||二極體接法電晶體：閘極接往汲極，或基極接往集極。|
|`$COMMON_SOURCE(device: instance)`|僅表語義||單顆 FET 的共源極放大級。|
|`$COMMON_GATE(device: instance)`|僅表語義||單顆 FET 的共閘極放大級。|
|`$COMMON_DRAIN(device: instance)`|僅表語義||單顆 FET 的源極隨耦器（共汲極級）。|
|`$COMMON_EMITTER(device: instance)`|僅表語義||單顆雙極性電晶體的共射極放大級。|
|`$COMMON_BASE(device: instance)`|僅表語義||單顆雙極性電晶體的共基極放大級。|
|`$COMMON_COLLECTOR(device: instance)`|僅表語義||單顆雙極性電晶體的射極隨耦器（共集極級）。|
|`$FOLLOWER(device: instance)`|僅表語義||單顆電晶體的電壓隨耦器級。|
|`$CASCODE(source: instance/usage, cascode: instance)`|走線||疊接結構（cascode）：輸入源器件或子結構與共閘極電晶體串聯。|
|`$FOLDED_CASCODE(source: instance/usage, cascode: instance, bias: instance/usage)`|走線||摺疊式疊接結構（folded cascode）：輸入器件搭配偏壓折入異型共閘極電晶體。|
|`$COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage)`|走線|經由 `$TRANSCONDUCTOR_Z_AMPLIFIER`：input, load|共源極放大器：由 FET 輸入電晶體及其負載組成。|
|`$COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage)`|走線|經由 `$TRANSCONDUCTOR_Z_AMPLIFIER`：input, load|共射極放大器：由雙極性輸入電晶體及其負載組成。|
|`$DEGENERATED_COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|走線|經由 `$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER`：input, load, degenerator|源極退化共源極放大器：由 FET 輸入電晶體、負載與源極退化支路組成。|
|`$DEGENERATED_COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|走線|經由 `$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER`：input, load, degenerator|射極退化共射極放大器：由雙極性輸入電晶體、負載與射極退化支路組成。|
|`$MILLER_AMPLIFIER(stage: instance/usage, compensation: instance/usage)`|走線|是：stage, compensation|米勒補償放大器：由增益級及其補償網路組成。|
|`$PUSH_PULL_STAGE(pullup: instance/usage, pulldown: instance/usage)`|走線|是：pullup, pulldown|推挽式輸出級：僅含輸出電晶體本身，即串聯的上拉與下拉部分；其偏壓及驅動它們的前級皆不屬於其中。|
|`$TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage)`|走線|是：transconductor, load|轉導-阻抗放大級：由轉導放大器及其負載組成。|
|`$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage, degenerator: instance/usage)`|走線|是：transconductor, load, degenerator|帶退化之轉導-阻抗放大級：由轉導放大器、負載與退化支路組成。|
|`$PASS_SWITCH(device: instance/usage)`|僅表語義||控制兩側訊號導通或阻斷的傳輸開關。|
|`$RESET_SWITCH(device: instance)`|僅表語義||用於預充電、等化或重置的開關電晶體。|
|`$GROUND_SWITCH(device: instance)`|僅表語義||接往接地網路的低側接地開關。|
|`$POWER_SWITCH(device: instance)`|僅表語義||接往電源網路的高側電源開關。|
|`$TRANSMISSION_GATE(pmos: instance/usage, nmos: instance/usage)`|擺放、走線；自身的點位於中心|是：pmos, nmos|CMOS 傳輸閘，PMOS 在上、NMOS 在下。|
|`$TRANSMISSION_GATE_HORIZONTAL(pmos: instance/usage, nmos: instance/usage)`|擺放、走線；自身的點位於中心|是：pmos, nmos|左右並排繪製的水平 CMOS 傳輸閘，PMOS 在左、NMOS 在右。|
|`$MULTI_POLE_SWITCH({throws}: instance/usage)`|走線|是：throws|共用同一個刀端（公共端）的多擲開關。|
|`$BOOTSTRAPPED_SWITCH(device: instance/usage)`|僅表語義||自舉式（bootstrapped）取樣開關。|
|`$SC_SAMPLER({switch}: instance/usage, {capacitor}: instance/usage)`|走線||切換電容取樣器：由取樣開關與取樣電容組成。|
|`$CHARGE_SHARING(a: instance/usage, switch: instance/usage, b: instance/usage)`|走線||電荷分享網路：透過開關相連以分享電荷的兩顆電容或電容陣列。|
|`$CHOPPER({straight}: instance/usage, {crossed}: instance/usage)`|走線||斬波器：由四個開關組成、用來翻轉訊號極性的橋式結構。|
|`$FLYING_CAPACITOR(capacitor: instance/usage)`|僅表語義||在不同時脈相位之間切換連接的飛跨電容（flying capacitor）。|
|`$CMOS_INVERTER(pmos: instance, nmos: instance)`|擺放、走線；自身的點位於中心|經由 `$CMOS_GATE`：pmos, nmos|CMOS 反相器，PMOS 在上、NMOS 在下。|
|`$CMOS_INVERTER_HORIZONTAL(pmos: instance, nmos: instance)`|擺放、走線；自身的點位於中心|經由 `$CMOS_GATE`：pmos, nmos|左右並排繪製的水平 CMOS 反相器，PMOS 在左、NMOS 在右。|
|`$CMOS_GATE({pullup}: instance/usage, {pulldown}: instance/usage)`|走線|是：pullup, pulldown|靜態 CMOS 邏輯閘：上拉網路與下拉網路於輸出端串聯交會。|
|`$LATCH({members}: instance/usage)`|走線|是：members|閂鎖器（latch）：雙穩態核心及其周圍輔助器件。|
|`$INVERTER_LATCH(a: instance/usage, b: instance/usage)`|走線|經由 `$LATCH`：a, b|由兩個反相器交叉耦合構成的閂鎖器。|
|`$SHIFT_REGISTER({stages}: instance/usage)`|僅表語義||移位暫存器：由正反器或閂鎖器串接而成的鏈狀結構，常以局部交疊方式繪製，且時脈極少畫出連線。|
|`$DIVIDER(upper: instance/usage, lower: instance/usage)`|走線||由上下兩條支路串聯構成的分壓器。|
|`$RESISTOR_LADDER({resistors}: instance)`|擺放、走線；自身的點綁定 x|是：resistors|排成一欄、由上而下依序串聯的電阻梯。|
|`$RESISTOR_LADDER_HORIZONTAL({resistors}: instance)`|擺放、走線；自身的點綁定 y|是：resistors|排成橫向一列依序串聯的電阻梯。|
|`$PI_NETWORK(shunt1: instance/usage, series: instance/usage, shunt2: instance/usage)`|走線||π 型網路：依序由並聯支路、串聯支路與另一條並聯支路構成。|
|`$T_NETWORK(series1: instance/usage, shunt: instance/usage, series2: instance/usage)`|走線||T 型網路：依序由串聯支路、並聯支路與另一條串聯支路構成。|
|`$LC_TANK_PARALLEL(inductor: instance/usage, capacitor: instance/usage)`|走線|是：inductor, capacitor|並聯 LC 諧振槽。|
|`$RLC_TANK_PARALLEL(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|走線|是：resistor, inductor, capacitor|並聯 RLC 諧振槽。|
|`$LC_TANK_SERIES(inductor: instance/usage, capacitor: instance/usage)`|走線|是：inductor, capacitor|串聯 LC 諧振槽。|
|`$RLC_TANK_SERIES(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|走線|是：resistor, inductor, capacitor|串聯 RLC 諧振槽。|
|`$DC_FEED(device: instance/usage)`|僅表語義||將直流電源或偏壓饋入節點的射頻扼流圈或偏壓電阻（直流饋電）。|
|`$DECAP(device: instance)`|擺放；自身的點綁定 x||接在電源或偏壓節點上的去耦電容。|
|`$AC_COUPLE(device: instance/usage)`|僅表語義||導通交流訊號並阻隔直流的串聯交流耦合電容。|
|`$OSCILLATOR({members}: instance/usage)`|走線|是：members|由主動器件搭配諧振或延遲組件構成的振盪器。|
|**rnl-std-signal-flow**||||
|`$ADDER(in1: net, in2: net, out: net)`|—||理想訊號流加法器（out = in1 + in2），以 `$ADDER_SYMBOL` 繪製。|
|`$SUBTRACTOR(in1: net, in2: net, out: net)`|—||理想訊號流減法器（out = in1 - in2），以 `$SUBTRACTOR_SYMBOL` 繪製。|
|`$MULTIPLIER(in1: net, in2: net, out: net)`|—||理想訊號流乘法器（out = in1 * in2），以 `$MULTIPLIER_SYMBOL` 繪製。|
|`$INTEGRATOR(in: net, out: net)`|—||理想連續時間積分器，以 `$INTEGRATOR_SYMBOL` 繪製。|
|`$DISCRETE_TIME_INTEGRATOR(in: net, out: net)`|—||理想離散時間積分器，以 `$DISCRETE_TIME_INTEGRATOR_SYMBOL` 繪製。|
|`$QUANTIZER(in: net, out: net)`|—||理想量化器，以 `$QUANTIZER_SYMBOL` 繪製。|
|`$UNIT_DELAY(in: net, out: net)`|—||理想單位延遲單元，以 `$UNIT_DELAY_SYMBOL` 繪製。|
|**rnl-std-45deg**||||
|`$ARROW_45(at: point)`|—||繪製在符號內某點或場景 45 度連線上的 45 度箭頭。|
|`$CURRENT_ARROW_45(p: terminal/point)`|—||沿 45 度支路繪製在端子或點上的 45 度電流箭頭。|

### 符號

下表依接腳所綁定的端子或角色列出各符號接腳：如 `pin`（`p`）代表名為 `pin` 且綁定至角色 `p` 的接腳；`pin` (nothing) 則代表不綁定任何端子或角色。

|符號|接腳|含義|
|---|---|---|
|**rnl-std-basic**|||
|`$RESISTOR`|`p` `n`|電阻。|
|`$VARIABLE_RESISTOR`|`p` `n`|可變電阻：帶斜向箭頭橫跨其上的電阻符號。|
|`$CAPACITOR`|`p` `n`|電容。|
|`$VARIABLE_CAPACITOR`|`p` `n`|可變電容：帶斜向箭頭橫跨其上的電容符號。|
|`$INDUCTOR`|`p` `n`|電感。|
|`$VARIABLE_INDUCTOR`|`p` `n`|可變電感：帶斜向箭頭橫跨其上的電感符號。|
|`$TRANSFORMER`|`p1` `n1` `p2` `n2`|變壓器：左右並立的兩組耦合繞組，極性圓點位於頂部。|
|`$POTENTIOMETER`|`n1` `nw` `n2`|三端電位器，中間接腳為滑動端（nw）。|
|`$PHOTORESISTOR`|`p` `n`|光敏電阻。|
|`$POLARIZED_CAPACITOR`|`p` `n`|極性（電解）電容，正極板位於 p 端。|
|`$CRYSTAL`|`n1` `n2`|石英晶體諧振器（石英晶振）。|
|`$SPEAKER`|`p` `n`|揚聲器。|
|`$VOLTAGE_SOURCE`|`p` `n`|獨立電壓源。|
|`$CURRENT_SOURCE`|`p` `n`|獨立電流源，內部箭頭自 p 指向 n。|
|`$BATTERY`|`p` `n`|電池。|
|`$CONTROLLED_VOLTAGE_SOURCE`|`p` `n`|僅繪出輸出接腳的菱形受控電壓源。|
|`$CONTROLLED_CURRENT_SOURCE`|`p` `n`|僅繪出輸出接腳的菱形受控電流源。|
|`$VCVS_4TERM`|`p` `n` `cp` `cn`|四端壓控電壓源（E），控制接腳位於左側。|
|`$VCCS_4TERM`|`p` `n` `cp` `cn`|四端壓控電流源（G），控制接腳位於左側。|
|`$BEHAVIORAL_SOURCE`|`p` `n`|行為源（B）：標有字母 B 的圓圈。|
|`$RF_PORT`|`p` `n`|射頻連接埠：正弦源串聯內阻，對應 Spectre 的 port。|
|`$SINE_VOLTAGE_SOURCE`|`p` `n`|正弦波電壓源，+ 號位於 p 端。|
|`$PULSE_VOLTAGE_SOURCE`|`p` `n`|脈波電壓源，+ 號位於 p 端。|
|`$MICROPHONE`|`p` `n`|麥克風，+ 號位於 p 端。|
|`$AMMETER`|`p` `n`|電流表，量測自 p 流向 n 的電流。|
|`$SWITCH`|`p` `n`|繪為斷開狀態的開關，用於流控開關（W）或 Spectre 的 switch。|
|`$CLOSED_SWITCH`|`p` `n`|繪為閉合狀態的開關。|
|`$EXTERNALLY_CONTROLLED_SWITCH`|`p` `n` `cp`|側邊帶有單一控制接腳的外控開關。|
|`$VOLTAGE_CONTROLLED_SWITCH`|`p` `n` `cp` `cn`|四端壓控開關（S），差動控制接腳位於下方。|
|`$SPDT_SWITCH_SYMBOL`|`com` (nothing) `a` (nothing) `b` (nothing)|`$SPDT_SWITCH` 的符號：單刀雙擲開關，其接腳 com、a、b 不綁定任何端子或角色，直接依接腳名稱連線（9.2.4）。|
|`$TRANSMISSION_LINE`|`p1` `n1` `p2` `n2`|傳輸線（T 或 O），連接埠 1 在左、連接埠 2 在右。|
|`$RC_LINE`|`a` `b` `ref`|均勻分佈式 RC 傳輸線（U）。|
|`$GROUND_SYMBOL`|`pin` (`p`)|網路 0 上 `$GROUND` 的符號：接腳位於頂部的三橫線接地符號。|
|`$NAMED_GROUND_SYMBOL`|`pin` (`p`)|非 0 網路上 `$GROUND` 的符號：下方附帶網路名稱的三橫線接地符號。|
|`$SUPPLY_SYMBOL`|`pin` (`p`)|`$SUPPLY` 的符號：上方為水平橫槓與網路名稱、接腳位於底部的電源符號。|
|`$BIAS_SYMBOL`|`pin` (`p`)|`$BIAS` 的符號：上方為斜槓與網路名稱、接腳位於底部的偏壓符號。|
|`$IO_PORT_SYMBOL`|`pin` (`p`)|`$IO_PORT` 的符號：帶小圓圈與網路名稱、接腳朝右的連接埠標記符號。|
|`$NET_LABEL_SYMBOL`|`pin` (`p`)|`$NET_LABEL` 的符號：如同 `$IO_PORT_SYMBOL` 接腳朝右，於 1 單位長引線左側顯示網路名稱（匯流排接腳上則顯示各網路名稱）。|
|`$ARROW_SYMBOL`|—|指向右側的實心箭頭符號，即 `$ARROW` 的符號；`$CURRENT_ARROW` 繪出一個一半大小的。|
|`$NO_CONNECT_SYMBOL`|—|`$NO_CONNECT` 的符號：叉號標記。|
|`$JUNCTION_SYMBOL`|—|`$JUNCTION` 的符號：實心圓點。|
|**rnl-std-semiconductors**|||
|`$DIODE`|`a` `k`|二極體。|
|`$ZENER_DIODE`|`a` `k`|稽納二極體。|
|`$LED`|`a` `k`|發光二極體（LED）。|
|`$SCHOTTKY_DIODE`|`a` `k`|蕭特基二極體。|
|`$SHOCKLEY_DIODE`|`a` `k`|蕭克利四層二極體。|
|`$PUT`|`a` `g` `k`|可程式單接面電晶體（PUT）。|
|`$NMOS`|`d` `g` `s`|三接腳 NMOS 電晶體，不繪出基體端。|
|`$NMOS_4TERM`|`d` `g` `s` `b`|四接腳 NMOS 電晶體，基體接腳與汲極、源極對齊於同一欄，通道處帶有基體箭頭。|
|`$PMOS`|`d` `g` `s`|三接腳 PMOS 電晶體，不繪出基體端。|
|`$PMOS_4TERM`|`d` `g` `s` `b`|四接腳 PMOS 電晶體，基體接腳與汲極、源極對齊於同一欄。|
|`$NMOS_LOGIC`|`d` `g` `s`|不帶源極箭頭的三接腳邏輯 NMOS 電晶體。|
|`$PMOS_LOGIC`|`d` `g` `s`|不帶源極箭頭、閘極帶有反相圓圈的三接腳邏輯 PMOS 電晶體。|
|`$DEPLETION_NMOS`|`d` `g` `s`|三接腳空乏型 NMOS 電晶體，不繪出基體端。|
|`$DEPLETION_NMOS_4TERM`|`d` `g` `s` `b`|四接腳空乏型 NMOS 電晶體，基體接腳與汲極、源極對齊於同一欄。|
|`$DEPLETION_PMOS`|`d` `g` `s`|三接腳空乏型 PMOS 電晶體，不繪出基體端。|
|`$DEPLETION_PMOS_4TERM`|`d` `g` `s` `b`|四接腳空乏型 PMOS 電晶體，基體接腳與汲極、源極對齊於同一欄。|
|`$NDMOS`|`d` `g` `s`|三接腳 N 型 DMOS 電晶體，不繪出基體端。|
|`$NDMOS_4TERM`|`d` `g` `s` `b`|四接腳 N 型 DMOS 電晶體，基體接腳與汲極、源極對齊於同一欄。|
|`$PDMOS`|`d` `g` `s`|三接腳 P 型 DMOS 電晶體，不繪出基體端。|
|`$PDMOS_4TERM`|`d` `g` `s` `b`|四接腳 P 型 DMOS 電晶體，基體接腳與汲極、源極對齊於同一欄。|
|`$NPN`|`c` `b` `e`|NPN 雙極性電晶體，不繪出基板端。|
|`$PNP`|`c` `b` `e`|PNP 雙極性電晶體，不繪出基板端。|
|`$NJFET`|`d` `g` `s`|N 通道 JFET，閘極箭頭指向通道。|
|`$PJFET`|`d` `g` `s`|P 通道 JFET，閘極箭頭背離通道。|
|`$UJT`|`e` `b1` `b2`|單接面電晶體（UJT），b2 在上、b1 在下。|
|**rnl-std-functional**|||
|`$OPAMP`|`inp` `inn` `out`|三端運算放大器：inp、inn、out。|
|`$OPAMP_SUPPLY`|`inp` `inn` `out` `vp` `vn`|帶電源接腳的運算放大器：inp、inn、out、vp、vn。|
|`$OPAMP_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|帶電源接腳的輸入對調運算放大器：輸入端上下對調（inn 在上、inp 在下），同時維持電源端原有方向（vp 在上、vn 在下，而非單純將 `$OPAMP_SUPPLY` 作 MX 翻轉）：inp、inn、out、vp、vn。|
|`$DIFFERENTIAL_OPAMP`|`inp` `inn` `outp` `outn`|全差動運算放大器：inp、inn、outp、outn。|
|`$DIFFERENTIAL_OPAMP_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|帶電源接腳的全差動運算放大器：inp、inn、outp、outn、vp、vn。|
|`$DIFFERENTIAL_OPAMP_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|帶電源接腳的輸入輸出對調全差動運算放大器：輸入與輸出端均上下對調（inn 與 outn 在上、inp 與 outp 在下），電源端維持原位（vp 在上、vn 在下，而非將 `$DIFFERENTIAL_OPAMP_SUPPLY` 作 MX 翻轉）：inp、inn、outp、outn、vp、vn。|
|`$DIFFERENTIAL_OPAMP_TWISTED`|`inp` `inn` `outp` `outn`|輸出極性交叉對調的全差動運算放大器（outn 在上、outp 在下）：inp、inn、outp、outn。|
|`$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|帶電源接腳且輸出極性交叉對調的全差動運算放大器：inp、inn、outp、outn、vp、vn。|
|`$DIFFERENTIAL_OPAMP_TWISTED_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|帶電源接腳且僅輸入對調的全差動運算放大器：輸入端上下對調（inn 在上、inp 在下），輸出端與 `$DIFFERENTIAL_OPAMP_SUPPLY` 相同（outp 在上、outn 在下），電源端維持原位（vp 在上、vn 在下，而非將 `$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY` 作 MX 翻轉）：inp、inn、outp、outn、vp、vn。|
|`$COMPARATOR`|`inp` `inn` `out`|三端比較器：inp、inn、out。|
|`$COMPARATOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|帶電源接腳的比較器：inp、inn、out、vp、vn。|
|`$COMPARATOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|帶電源接腳的輸入對調比較器：輸入端上下對調（inn 在上、inp 在下），電源端維持原位（vp 在上、vn 在下，而非將 `$COMPARATOR_SUPPLY` 作 MX 翻轉），內部階躍符號方向亦與 `$COMPARATOR_SUPPLY` 相同：inp、inn、out、vp、vn。|
|`$VOLTAGE_AMPLIFIER`|`in` `out`|電壓放大器，中央帶選用的 key 為 label 之置中文字：in、out。|
|`$VOLTAGE_AMPLIFIER_SUPPLY`|`in` `out` `vp` `vn`|帶電源接腳的電壓放大器：in、out、vp、vn。|
|`$TRANSCONDUCTOR`|`in` `out`|轉導放大器（gm 單元），中央帶 key 為 label 之置中文字（預設為 g_m）：in、out。|
|`$TRANSCONDUCTOR_SUPPLY`|`in` `out` `vp` `vn`|帶電源接腳的轉導放大器，中央帶 key 為 label 之置中文字（預設為 g_m）：in、out、vp、vn。|
|`$DIFFERENTIAL_TRANSCONDUCTOR`|`inp` `inn` `out`|帶 key 為 label 之文字（預設為 g_m）的差動輸入轉導放大器：inp、inn、out。|
|`$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|帶電源接腳與 key 為 label 之文字（預設為 g_m）的差動輸入轉導放大器：inp、inn、out、vp、vn。|
|`$DIFFERENTIAL_TRANSCONDUCTOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|帶電源接腳與 key 為 label 之文字（預設為 g_m）的輸入對調差動轉導放大器：輸入端上下對調（inn 在上、inp 在下），電源端維持原位（vp 在上、vn 在下，而非將 `$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY` 作 MX 翻轉）：inp、inn、out、vp、vn。|
|`$ADC`|`in` `out`|類比數位轉換器（ADC）：in、out。|
|`$ADC_SUPPLY`|`in` `out` `vp` `vn`|帶電源接腳的類比數位轉換器：in、out、vp、vn。|
|`$DAC`|`in` `out`|數位類比轉換器（DAC）：in、out。|
|`$DAC_SUPPLY`|`in` `out` `vp` `vn`|帶電源接腳的數位類比轉換器：in、out、vp、vn。|
|`$CHOPPER_SYMBOL`|`inp` `inn` `outp` `outn`|斬波調變器方塊符號（有別於四開關橋式結構 `$CHOPPER`）：inp、inn、outp、outn。|
|**rnl-std-rf**|||
|`$CENTER_TAP_INDUCTOR`|`n1` `n2` `tap`|中心抽頭電感。|
|`$T_COIL`|`n1` `n2` `tap`|T 型線圈（T-coil）：帶中心抽頭與橋接電容的兩組耦合線圈。|
|**rnl-std-logic**|||
|`$AND_2INPUT`|`a` `b` `y`|二輸入及閘（AND）：a、b、y。|
|`$AND_3INPUT`|`a` `b` `c` `y`|三輸入及閘（AND）：a、b、c、y。|
|`$NAND_2INPUT`|`a` `b` `y`|二輸入反及閘（NAND）：a、b、y。|
|`$NAND_3INPUT`|`a` `b` `c` `y`|三輸入反及閘（NAND）：a、b、c、y。|
|`$OR_2INPUT`|`a` `b` `y`|二輸入或閘（OR）：a、b、y。|
|`$OR_3INPUT`|`a` `b` `c` `y`|三輸入或閘（OR）：a、b、c、y。|
|`$NOR_2INPUT`|`a` `b` `y`|二輸入反或閘（NOR）：a、b、y。|
|`$NOR_3INPUT`|`a` `b` `c` `y`|三輸入反或閘（NOR）：a、b、c、y。|
|`$XOR_2INPUT`|`a` `b` `y`|二輸入互斥或閘（XOR）：a、b、y。|
|`$XOR_3INPUT`|`a` `b` `c` `y`|三輸入互斥或閘（XOR）：a、b、c、y。|
|`$XNOR_2INPUT`|`a` `b` `y`|二輸入反互斥或閘（XNOR）：a、b、y。|
|`$XNOR_3INPUT`|`a` `b` `c` `y`|三輸入反互斥或閘（XNOR）：a、b、c、y。|
|`$AND_4INPUT`|`a` `b` `c` `d` `y`|接腳位於整數格點上的四輸入及閘（AND）：a、b、c、d、y。|
|`$NAND_4INPUT`|`a` `b` `c` `d` `y`|接腳位於整數格點上的四輸入反及閘（NAND）：a、b、c、d、y。|
|`$OR_4INPUT`|`a` `b` `c` `d` `y`|接腳位於整數格點上的四輸入或閘（OR）：a、b、c、d、y。|
|`$NOR_4INPUT`|`a` `b` `c` `d` `y`|接腳位於整數格點上的四輸入反或閘（NOR）：a、b、c、d、y。|
|`$XOR_4INPUT`|`a` `b` `c` `d` `y`|接腳位於整數格點上的四輸入互斥或閘（XOR）：a、b、c、d、y。|
|`$XNOR_4INPUT`|`a` `b` `c` `d` `y`|接腳位於整數格點上的四輸入反互斥或閘（XNOR）：a、b、c、d、y。|
|`$BUFFER`|`a` `y`|緩衝器：a、y。|
|`$INVERTER`|`a` `y`|反相器（NOT 閘）：a、y。|
|`$DELAY_CELL`|`a` `y`|延遲單元：a、y。|
|`$D_FLIP_FLOP`|`d` `ck` `q` `qb`|帶互補輸出 Q 與 QB 的 D 型正反器：d、ck、q、qb。|
|`$D_FLIP_FLOP_Q_ONLY`|`d` `ck` `q`|僅帶輸出 Q 的 D 型正反器：d、ck、q。|
|`$D_FLIP_FLOP_WITH_RESET`|`d` `ck` `rst` `q` `qb`|帶重置端的 D 型正反器：d、ck、rst、q、qb。|
|`$AND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|帶電源接腳的二輸入及閘：a、b、y、vp、vn。|
|`$AND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|帶電源接腳的三輸入及閘：a、b、c、y、vp、vn。|
|`$AND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|帶電源接腳的四輸入及閘：a、b、c、d、y、vp、vn。|
|`$NAND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|帶電源接腳的二輸入反及閘：a、b、y、vp、vn。|
|`$NAND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|帶電源接腳的三輸入反及閘：a、b、c、y、vp、vn。|
|`$NAND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|帶電源接腳的四輸入反及閘：a、b、c、d、y、vp、vn。|
|`$OR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|帶電源接腳的二輸入或閘：a、b、y、vp、vn。|
|`$OR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|帶電源接腳的三輸入或閘：a、b、c、y、vp、vn。|
|`$OR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|帶電源接腳的四輸入或閘：a、b、c、d、y、vp、vn。|
|`$NOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|帶電源接腳的二輸入反或閘：a、b、y、vp、vn。|
|`$NOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|帶電源接腳的三輸入反或閘：a、b、c、y、vp、vn。|
|`$NOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|帶電源接腳的四輸入反或閘：a、b、c、d、y、vp、vn。|
|`$XOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|帶電源接腳的二輸入互斥或閘：a、b、y、vp、vn。|
|`$XOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|帶電源接腳的三輸入互斥或閘：a、b、c、y、vp、vn。|
|`$XOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|帶電源接腳的四輸入互斥或閘：a、b、c、d、y、vp、vn。|
|`$XNOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|帶電源接腳的二輸入反互斥或閘：a、b、y、vp、vn。|
|`$XNOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|帶電源接腳的三輸入反互斥或閘：a、b、c、y、vp、vn。|
|`$XNOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|帶電源接腳的四輸入反互斥或閘：a、b、c、d、y、vp、vn。|
|`$BUFFER_SUPPLY`|`a` `y` `vp` `vn`|帶電源接腳的緩衝器：a、y、vp、vn。|
|`$INVERTER_SUPPLY`|`a` `y` `vp` `vn`|帶電源接腳的反相器：a、y、vp、vn。|
|`$DELAY_CELL_SUPPLY`|`a` `y` `vp` `vn`|帶電源接腳的延遲單元：a、y、vp、vn。|
|`$D_FLIP_FLOP_SUPPLY`|`d` `ck` `q` `qb` `vp` `vn`|帶電源接腳的 D 型正反器：d、ck、q、qb、vp、vn。|
|`$D_FLIP_FLOP_Q_ONLY_SUPPLY`|`d` `ck` `q` `vp` `vn`|帶電源接腳且僅有 Q 輸出的 D 型正反器：d、ck、q、vp、vn。|
|`$D_FLIP_FLOP_WITH_RESET_SUPPLY`|`d` `ck` `rst` `q` `qb` `vp` `vn`|帶重置端與電源接腳（vp 與 vn 於 x = 0 處上下對齊）的 D 型正反器：d、ck、rst、q、qb、vp、vn。|
|**rnl-std-signal-flow**|||
|`$ADDER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|`$ADDER` 的符號：輸入引線 a（in1）與 b（in2）末端帶箭頭的加總圓圈，輸出為 y（out）。|
|`$SUBTRACTOR_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|`$SUBTRACTOR` 的符號：圓心標有 +、負輸入端 b（in2）外側標有 - 的加總圓圈，輸入為 a（in1）與 b（in2），輸出為 y（out）。|
|`$MULTIPLIER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|`$MULTIPLIER` 的符號：輸入引線 a（in1）與 b（in2）末端帶箭頭的乘法圓圈，輸出為 y（out）。|
|`$INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|`$INTEGRATOR` 的符號：標有 1/s 的方塊，輸入為 a（in），輸出為 y（out）。|
|`$DISCRETE_TIME_INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|`$DISCRETE_TIME_INTEGRATOR` 的符號：標有 z 域轉移函數的方塊，輸入為 a（in），輸出為 y（out）。|
|`$QUANTIZER_SYMBOL`|`a` (`in`) `y` (`out`)|`$QUANTIZER` 的符號：內繪階梯折線的方塊，輸入為 a（in），輸出為 y（out）。|
|`$UNIT_DELAY_SYMBOL`|`a` (`in`) `y` (`out`)|`$UNIT_DELAY` 的符號：標有 z⁻¹ 的方塊，輸入為 a（in），輸出為 y（out）。|
|**rnl-std-45deg**|||
|`$RESISTOR_45`|`p` `n`|45 度電阻。|
|`$CAPACITOR_45`|`p` `n`|45 度電容。|
|`$INDUCTOR_45`|`p` `n`|45 度電感（亦可用於對角線變壓器或巴倫繞組）。|
|`$SWITCH_45`|`p` `n`|45 度斷開開關。|
|`$VOLTAGE_SOURCE_45`|`p` `n`|45 度獨立電壓源，靠近 p 的 + 號與靠近 n 的 - 號保持正立。|
|`$CURRENT_SOURCE_45`|`p` `n`|45 度獨立電流源，內部箭頭自 p 指向 n。|
|`$DIODE_45`|`a` `k`|45 度二極體。|
|`$ZENER_DIODE_45`|`a` `k`|45 度稽納二極體。|
|`$SCHOTTKY_DIODE_45`|`a` `k`|45 度蕭特基二極體。|
|`$LED_45`|`a` `k`|45 度發光二極體（LED）。|
|`$NMOS_45`|`d` `g` `s`|45 度三接腳 NMOS 電晶體。|
|`$NMOS_4TERM_45`|`d` `g` `s` `b`|45 度四接腳 NMOS 電晶體，基體接腳位於原點 (0, 0)。|
|`$PMOS_45`|`d` `g` `s`|45 度三接腳 PMOS 電晶體。|
|`$PMOS_4TERM_45`|`d` `g` `s` `b`|45 度四接腳 PMOS 電晶體，基體接腳位於原點 (0, 0)。|
|`$NPN_45`|`c` `b` `e`|45 度 NPN 雙極性電晶體。|
|`$PNP_45`|`c` `b` `e`|45 度 PNP 雙極性電晶體。|
|`$ARROW_45_SYMBOL`|—|指向 -45 度方向（右下方，即 R90 下 45 度符號沿 p->n 或 a->k 的方向）的實心箭頭符號，搭配 R0/R90/R180/R270 可轉向全部四個對角線方向。|

### 集合與樣式

|名稱|所屬套件|含義|
|---|---|---|
|`$GROUND_NET_SET`|rnl-std-basic|接地網路集合，預設包含網路 0，可透過 ADD-TO 與 REMOVE-FROM 增刪；rnl-std-fill 會為其中未連線的接腳加上接地短樁，並讓這些網路保持不走線。接地符號如何繪製不取決於此集合。|
|`$SUPPLY_NET_SET`|rnl-std-basic|電源網路集合，預設為空，可透過 ADD-TO 與 REMOVE-FROM 增刪。|
|`$BIAS_NET_SET`|rnl-std-basic|偏壓網路集合，預設為空，可透過 ADD-TO 與 REMOVE-FROM 增刪。|
|`$PORT_FILL_SET`|rnl-std-basic|由 rnl-std-fill 自動加上連接埠標記的連接埠網路集合：涵蓋每一層級的所有連接埠網路（電源與接地連接埠亦在內），扣除已寫出 `$IO_PORT` 的網路以及已使用 `$PORT_LIST` 之層級的網路。|
|`$GROUND_FILL_SET`|rnl-std-fill|接地網路上待由自動補齊加上接地短樁的未連線接腳集合。|
|`$SUPPLY_FILL_SET`|rnl-std-fill|電源網路上待由自動補齊加上電源短樁的未連線接腳集合。|
|`$BIAS_FILL_SET`|rnl-std-fill|偏壓網路上待由自動補齊加上偏壓短樁的未連線接腳集合。|
|`$LABEL_FILL_SET`|rnl-std-fill|其餘網路上待由自動補齊加上網路標籤短樁的未連線接腳集合；匯流排接腳僅計一個。|
|`$base`|rnl-std-style|所有物件的基礎樣式：依 Analog Canvas 的預設風格繪製線條、文字與填色。|
|`$bus`|rnl-std-style|匯流排樣式：粗線條。|
|`$signal`|rnl-std-style|訊號連線樣式：藍色粗線條。|
|`$note`|rnl-std-style|註記說明樣式：灰色較小字級文字。|
|`$grey`|rnl-std-style|線條與文字皆為灰色的樣式。|

<!-- lists:end -->
