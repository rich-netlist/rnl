# RNL 0.4 ショートガイド

RNLはSPICEまたはSpectreネットリストのコメント内に記述し、回路をどのように描画するか、また各部品が何を意味するかを表すものであり、回路自体を変更することは決してない。言語は `"0.4"`、標準パッケージシリーズは `"0.4"` であり、（14.8）のような番号は標準の節番号を指す。以下の各例は1つの連続したプログラムではなくそれぞれ独立した記述例であり、実際に用いる際はそのレベルに存在するオブジェクトを当てはめ、断片をRNLレコード内に記述する。`/* */` は例の説明である。末尾のカタログにはすべての名前をシグネチャ形式で掲載しているが、これはそのままコピーして使うRNLコードではない。

## ファイル

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

- レコードは行全体コメントとして記述し、1列目に `*` とそれに続く1つの空白を置く。Spectreモード（`.scs` ファイル、または `simulator lang=spectre` より後）では `//` も使用できる。ヘッダーレコードを先頭に置き、`.subckt … .ends` の内側にあるレコードはそのサブサーキットに属する。
- USE-PACKAGEがなければ標準の名前は認識されない。あらかじめ与えられたヘッダーレコードはそのまま残すこと。その中の `@DEVICE` 行はPDK素子に端子名と型を与えるものであり（6.12）、自分で記述したり変更したりしてはならない。
- すべての `@` の直前にはASCII半角スペースを1つ置く。パラメータの丸括弧はキーワードに密着させ、ロールの丸括弧はオブジェクトに密着させる。位置指定のパラメータは名前指定のパラメータより前に置く：`@PLACE(10, y=20)`。数値に単位はなく、式も存在しない。RNLの名前とキーワードは大文字・小文字を区別し、ネットリストの名前はホストの規則に従う。

## 参照

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

- 端子名：Mは `d g s b`、Qは `c b e s`、J・Zは `d g s`、Dは `a k`、R・C・L・V・I・B・F・H・Wは `p n`、E・G・Sは `p n cp cn`、T・Oは `p1 n1 p2 n2`、Uは `a b ref`、Xはサブサーキットのポートである。ピンが必要な箇所では、端子がそのピンを代表する。
- 名前はレベルごとに1度だけ束縛され、定義より前でも参照できる。`%p` を移動させるには、新たに `%p = ...` と書くのではなく `%p @PLACE(...)` と書く。`%` で始まる名前はそのレベル固有のローカル名であり、`$` で始まる定義は下位レベルへ継承されるが、下位レベルで別の定義により隠蔽されることがある。
- 接頭辞のない語は自由テキストである（何かの名前と一致する場合は警告となる）：`#M1 #M2 M3 @ALIGN("y");` はM1とM2のみを整列させる。

## 文とパイプ

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

各セグメントが次へ渡すオブジェクト：PLACE、ORIENT、SET-SYMBOL、SET-STYLE、GROUP、CURRENTは受け取った入力。AS、ATTACHは新しい適用オブジェクト（1つずつ適用された場合はそれらのセット）。ALIGN、ORDER、DISTRIBUTE、DISTANCE、TABLE、ORIENT-RELATIONは新しい制約。TEXTは新しいテキスト。SELECT-…は選択されたセットである。

## 定義、ロール、適用オブジェクト

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

- `{role}` はセット全体（1つのオブジェクトが渡された場合は要素数1のセット）を受け取り、省略してもよい。ロールを指定しない引数では、末尾のセットロールのみ省略できる。単一オブジェクトのロールは必須である。同じロールは最大1回しか指定できないため、複数のメンバーは1つのセットにまとめる：`{#R1 #R2}(members)`。
- 定義本体内では、単独の `?` が適用オブジェクト自身の点を表し、座標はその点からのオフセットとなる。`%mid = (_, _);` はローカルオブジェクトを作り、外部からは `%u::mid` として参照する。
- 配置を行う標準構造は適用オブジェクト自身の点を結び付ける：メンバーの行または列はその線上に、鏡像対称の2辺はその中央に自身の点を置くため、`%u::position` に対する制約で構造全体を移動できる。カタログの *描画* 列がその位置を示す：`$BRANCH` はメンバーのx座標のみを自身の点と共有し、`$BRANCH_HORIZONTAL` はy座標のみを共有し、`$PARALLEL_HORIZONTAL` と `$MIRRORED_LAYOUT` は自身の点をその中心に置く。それ以外の構造は自身の点を自由なまま残し、その `::position` を制約しても描画は何も変わらない。
- 部品が何であるかを示すだけでなく描画も行う構造を優先して用いる。`$BRANCH` はメンバーを上から下へ縦1列に積む直列構造であり、通常は電源からグラウンドへの経路などの DC ブランチに用いる。このような経路はそれぞれ1つの `$BRANCH` として記述し、その上の並列ペアには配置を行う変種を用いる：`{#R1 #M1 #R2} @AS $BRANCH; {#R2 #C2} @AS $PARALLEL_HORIZONTAL;`。`$PARALLEL` だけでは配線しか行わない。また、ブランチは電流について何も規定しないため、電流方向は `@CURRENT` で記述する。スイッチトキャパシタサンプラのようにスイッチとキャパシタを通る信号経路は、各行のピンに対するALIGNやORDERなどの原子的な事実で描画する：`{#S1:s #S1:d #C1:p #C1:n} @ALIGN("y"); {#S1:s #S1:d #C1:p #C1:n} @ORDER("left-to-right");`。*意味のみ* の構造は何も描画しないため、それが成り立つ箇所に書き、配置を行う構造の代わりには決して用いないこと。`$SIGNAL_FLOW` は確信を持てる明快な信号経路にのみ記述し、経路のメンバー数にかかわらず、信号が複数の経路に分かれる箇所、帰還する箇所、または回路の大部分を双方向に流れる箇所では記述しないこと。

## 選択とルール

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

- `name`、`model`、`net`、`key` にはワイルドカード `*` を使用できる。`type` は `.model` カードまたはDEVICEで与えられる型である。
- 選択の結果およびそこから構築されたセットにはメンバー位置指定（`%sel{1}`）を使用できず、WIREにも渡せない。対象オブジェクトに直接名前を付けるか、ROUTE-OBJECTを用いること。

## レイアウトの事実

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

- 方向：`"left-to-right"`、`"right-to-left"`、`"bottom-to-top"`、`"top-to-bottom"`。向きコード：`R0`、`R90`、`R180`、`R270` は反時計回りに回転し、`MY` は左右反転、`MX` は上下反転を表す。`MYR90` と `MXR90` は反転してから回転する。
- 制約のメンバーになれるのは、原点で位置が決まるインスタンス、ピン位置で決まる端子とピン、点、テキスト、および自身の点で決まるシンボル付きの適用オブジェクトである。シンボルを持たない適用オブジェクトはGROUPと同様に最も下位までたどってそのメンバーを表し、その自身の点は `%u::position` である。セットは原点を持たないため、GROUPの後であってもPLACEは各メンバーを個別に配置する。PLACEはピンや制約を配置できない。また「最左」のような絶対指定はないため、他の部品との相対関係で記述する。
- ソルバーは自由に残された要素を決定するだけで、部品の役割を推測することはないため、成立すべき条件を明記する必要がある。通常は意味と制約を書いてソルバーに配置を任せるのが望ましいが、課題として座標が求められている場合は、代わりにすべてに対してPLACE、ORIENT、WIREを記述する。両者を混在させてはならない。ソルバーに任せるためPLACEをほとんど（または一切）書かずに意味と制約だけを記述するか、あるいは求解を行わずすべての位置を書き切るかのいずれかに徹底すること。
- 個々の素子ではなく大きなまとまり単位で順序を付ける：`{#V2 #C1 %bias %stage #C3 #R5} @ORDER("left-to-right");` のように書けば列全体を横に並べられる。構造はそのメンバーを表すため、`%stage` のすべての素子は `%bias` のすべての素子とC3の間に置かれ、2つの構造が共有する素子がそれ自身に対して順序付けられることはない。代わりに構造の中央を制約するには自身の点を書く：`{%dp::position #X1} @ALIGN("y");` はX1を差動対の中央と同じ高さに置くが、`{%dp #X1} @ALIGN("y");` は3つのトランジスタをX1と同じ高さに置く（構造がどの位置に点を結ぶかはカタログの *描画* 列を参照）。
- 電源や信号源など回路周辺の部品は通常、回路の片側に少し離して置く。これはDISTANCEで記述する：`#VDD {#VIN #XU1 #CL} @DISTANCE("left-to-right", min=4);` は電源を他の各要素より少なくとも4単位左に保つ。
- 図面には明確なグループのツリーを与える：各機能ブロックを1つのグループとし、小さなグループを大きなグループに入れ子にし、各ノード直下の要素が5〜6個程度になるようにする。多くの構造はすでにグループである（*グループ* 列を参照）が、カレントミラーのように一部が別の構造の内側に跨り得る構造はグループではない。構造全体、あるいは増幅段とそのポートは自分でグループにまとめる：`%amp %p_inp %p_inn %p_out @GROUP;`。グループは交差しない：入れ子関係になくメンバーを共有する2つのグループがある場合、ソルバーは一方のみをまとまりとして保つ。
- 向きを固定するのは、他の条件から定まらず、かつ読み手が特定の向きを期待する箇所（通常は信号源、電源、負荷）だけにする：`{#V1 #V2 #R5} @ORIENT("R0");`。残りはソルバーに任せること。構造と電流の記述によって、ほとんどの向きは自然に定まる。

## ピンと向き

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

- 8通りの向きのうち、1本のピンを固定しただけでは2通りが残る。ゲートだけを固定すると正立と上下逆の2通りが残り、ドレインだけを固定するとゲート左向きと右向きの2通りが残るため、2本のピンを固定すること。
- R0における標準ピン配置：R、C、Lおよび電源類はpが上、nが下。`$DIODE` はaが上、kが下。`$NMOS` はdが上、gが左、sが下、bが右（4TERM）。`$PMOS` はsが上、gが左、dが下。`$NPN` はcが上、bが左、eが下。`$PNP` はeが上、bが左、cが下である。

## 電流

```rnl
#M1:d @CURRENT("in"); #M1:s @CURRENT("out"); /* the static current at a terminal: in, out, unspecified */
{#M1:d #M2:d #M5:d} @CURRENT("in"); /* a set does each member */
#:out @CURRENT("in"); /* at a port: current enters the subcircuit */
#M1::d @CURRENT("in"); /* a pin stands for its terminal */
```

- 電流の記述自体は何も描画しないが、回路がどのように動作するかを示し、ソルバーはこれに従って電流経路を配置する。主要な電流経路に記述すること。
- 直流電流が流れ、かつその向きが確実な箇所にだけ記述する。キャパシタ、MOSゲート、およびDC値を持たない電圧源・電流源には直流電流が流れないため対象外とし、向きが不明な端子も除外する。電源にも記述しないこと。電源の内部では電流がnからpへ上向きに流れるため、電流を下向きに流す配置を好むソルバーが電源を上下反転させてしまうおそれがある。
- 接続するすべての端子およびポートに電流方向が指定され、それらがネットに対して一方向にしか流れない場合は違反となる。0などのグローバルネットは検査対象外である。

## 優先度

```rnl
#M5 @PLACE(x=3, priority="low"); /* disabled, low, normal (default), high */
%dp1 = #M1(left) #M2(right) #M5(tail) @AS(priority="high") $DIFFERENTIAL_PAIR_N; /* covers every fact of the body */
%dp1 @PRIORITY("disabled"); /* turns the default drawing of the pair off */
```

- 競合時は優先度の低い事実から先に破棄され、優先度が同じ場合は先に書かれた事実が破棄される。採用された事実はハード制約となる。標準パッケージの描画制約は `"low"` で記述されている。レコードヘッダーの `priority=` はデフォルトの優先度を与えるが、ASとATTACHはそこから `"disabled"` のみを受け継ぐ。
- 構造に優先度を指定する必要はない：検査または求解で競合が示され、それを解決する場合にのみ記述する。

## 配線と自動配線

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

- 座標が一致しているだけではピン同士は接続されない。余分な自動補完が追加されるのを防ぎ配線済みであることを明示するため、長さが0であっても両者の間にWIREを記述すること。
- 各構造は内部の配線を優先度lowで描画する。2つの構造の間は、ROUTEを記述しない限りネットラベルで結ばれる。
- すべてを配線するのではなく、線として描いた方が読みやすい箇所だけを選んで配線する。ソルバーが描画するのはWIRE、ROUTE-NET、ROUTE-OBJECTで指示された箇所だけであり、自動補完がそれ以外の未配線ピンすべてにグラウンド、電源、またはネットラベルのスタブを付与する。主信号経路、ならびに近接する構造の内部および構造間の接続は配線し、バイアス線や図面全体で共有されるネットなど長大で錯綜しやすい接続はラベルに任せること。
- 小規模な回路では、電源・バイアス系のレール以外のネットをすべて配線すると通常読みやすくなるため、上記の最終行のように、または `# @SELECT-NET => @ROUTE-NET;` で自動配線する。どちらも同じ図面になる。いずれの場合も自動補完がグラウンド、電源、バイアスのネットをhighで配線解除し、各レールネットに対して、ツリーがすべてのピンを失った旨のヒント（RNL9403）が出される。このヒントは想定どおりのものである。
- そのレベルのポートへ結ばれるネットがポートに届くのは、ポートタグを書いたときだけである：`$IO_PORT @ATTACH &x;`、または駆動先のピンに `$IO_PORT_STUB` を書く。自動補完はどのピンが配線済みかを決めた後でポートタグを加えるため、自分のピンを1つだけ持つネット（たとえば1つのゲートを駆動する入力）は点が1つのツリーとなる。そのピンにはネットラベルが付き、ポートに届く配線はない。

## テキスト

```rnl
#M1 @TEXT("${w}/${l}", key="wl", dy=-3); /* a label that moves with M1; #M1::wl */
#M1 @TEXT("M_{in}", key="name"); /* the same key replaces the name the symbol shows */
(10, 10) @TEXT("bias", size=0.8); /* on a point: an annotation */
#M1 @TEXT("\overline{EN} \mu A \frac{W}{L}", halign="middle", valign="top", rotate=90);
```

- `${…}` で参照できる項目：`name`、`value`、`model`、`type`、`w` などのネットリストパラメータ、`net`、ロール、および `${pair::left::w}` のようなパス。
- `dx` と `dy` はテキストの取り付け先からのオフセットである。`#M1::wl @PLACE(5, 0);` はラベルを図面の (5, 0) に配置する。各軸ごとにPLACEとオフセットは互いに置き換え合い、後に書かれた方が有効になる。シンボルの定義本体内では、どちらもシンボル自身の座標系で表される。

## シンボルとスタイル

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

- スタイルプロパティ：`linecolor linestyle linewidth fillcolor textcolor size font bold italic textvisible`。色は `"#RRGGBB"` 形式で指定する。`%` で始まる名前でローカル定義に名前を付けることもできる。各プロパティごとに直接指定はルールに優先し、同一ティア内では後に書かれた設定が優先される。

## 検査

標準構造は自身が主張する条件を検査する。たとえば差動対の両半回路が同じモデル・サイズであるか、直列構造が実際に直列接続されているか、pmosロールにPMOSが入っているか、ポートタグがポート上に置かれているかなどである。検査に反した場合は**違反**として報告されるだけで、何も失敗せず回路図も変わらない。構造やロールを修正するか、主張する条件のより少ない構造を選ぶこと。

## 標準パッケージ

```rnl
@USE-PACKAGE(name="rnl-std-all", version="0.4"); /* everything: styles, symbols, rules for SPICE and Spectre, fill, structures, the blocks */
@USE-PACKAGE(name="rnl-std-no-labels", version="0.4"); /* optional, besides rnl-std-all: hides the labels symbols draw (name, value, model, net names); a label you write shows only with textvisible=true */
@USE-PACKAGE(name="rnl-std-45deg", version="0.4"); /* optional: symbols at 45 degrees */
@USE-PACKAGE(name="rnl-std-fill") {}; /* an empty package drops that part */
&vdd @ADD-TO $SUPPLY_NET_SET; &vss @ADD-TO $GROUND_NET_SET; /* supply and ground stubs on their pins; 0 is a ground already */
$PORT_LIST @ATTACH; /* a port tag on every port net, in one column */
$FIGURE @ATTACH %p => @SET-SYMBOL $MY_FIG; /* no netlist instance: use a symbol with unbound pins and no ${name}/${value}; not $RESISTOR */
```

- 既定シンボル：Rは `$RESISTOR`、Cは `$CAPACITOR`、Lは `$INDUCTOR`、Vは `$VOLTAGE_SOURCE`、Iは `$CURRENT_SOURCE`、Dは `$DIODE`、Eは `$VCVS_4TERM`、Gは `$VCCS_4TERM`、Fは `$CONTROLLED_CURRENT_SOURCE`、Hは `$CONTROLLED_VOLTAGE_SOURCE`、Bは `$BEHAVIORAL_SOURCE`、Sは `$VOLTAGE_CONTROLLED_SWITCH`、Wは `$SWITCH`、T・Oは `$TRANSMISSION_LINE`、Uは `$RC_LINE` である。トランジスタは型ごとに割り当てられ、nmosは `$NMOS_4TERM`、pmosは `$PMOS_4TERM`、npnは `$NPN`、pnpは `$PNP`、njfは `$NJFET`、pjfは `$PJFET` となる。バルクがグラウンドネットに接続されたNMOSと電源ネットに接続されたPMOSには、SET-SYMBOLを自分で書かなくても代わりに3ピンの `$NMOS` と `$PMOS` が割り当てられる。型がない（`.model` もDEVICEもない）場合、トランジスタにはデバイスのシンボルが付かず、ノードごとに1本のピンを持つ四角形として描画される（9.2.8）。
- 電源ネットはすべて `$SUPPLY_NET_SET` に、0以外のグラウンドネットはすべて `$GROUND_NET_SET` に上記のとおり登録する。これにより、それらの未配線ピンにはスタブが付き、自動配線から除外される。自動補完は配線または未接続マークが付いたピンをスキップし、それ以外の未配線のピン（素子のものも仮想部品のものも）すべてに対して、`$GROUND_NET_SET`（既定ではネット0）上のピンには `$GROUND_STUB` を、`$SUPPLY_NET_SET`（既定では空）上のピンには `$SUPPLY_STUB` を、`$BIAS_NET_SET`（既定では空）上のピンには `$BIAS_STUB` を、それ以外のネット上のピンには `$NET_LABEL_STUB` を付与する。ただし4端子MOSのバルクピンには、どのネット上であっても `$NET_LABEL_STUB` が付く。すべてのポートネットには、電源ネットとグラウンドネットも含め、そのレベルのインスタンス群より少なくとも6単位左に、それぞれR0で縦1列に `$IO_PORT` が付与される。特定のネットを除外するには `&vdd @REMOVE-FROM $PORT_FILL_SET;` と書き、自分でタグを配置するには `$IO_PORT @ATTACH &out;` と書く。ポートタグはそれ自体でネット名をテキスト `netname` として表示するため、ネット名用のTEXTを別途重ね書きしてはならず、名前を移動したいときは `%t::netname @PLACE(x, y);` と書く。テストベンチのネット名には `$NET_LABEL` を用いること。すでに `$IO_PORT_STUB` または `$NET_LABEL_STUB` が付いているピンにもネットラベルは追加されない（それらの引き出し線はWIREではなくシンボルによって描画される）。`*_FILL_SET` の各セットは自動補完自身がスタブとタグを生成するためのものである。求解を実行するとそれらが書き戻され、セットは該当のピンやネットを保持しなくなるため、そのピンにTEXTを付けるといった他のルールをこれらのセットに依存させてはならない。必要なピンは自分で選択すること。どのピンにスタブを付けるかを変更するには、これらのセットに対してピンを追加または削除する。独自のルールでスタブを取り付けても自動補完からは見えず、自動補完自身のスタブも重ねて追加されてしまう。

## ソルバーのレコード

```spice fragment
* @RNL(type="generated", priority="high", author="rnl-solver") { #R1 @PLACE(0, 4); #R2 @PLACE(0, -4);} /* never edit; replaced on each solve */
* @RNL(author="editor") { #R2 @PLACE(y=-6, priority="high");} /* to fix a position: high, in a later record */
```

`%#3` のような番号はソルバーのレコード内にのみ現れる。手書きする場合は代わりにオブジェクトへ名前を付けること。

## その他のキーワード

パッケージやツールが書き出すキーワードであり、読むことはあっても自分で書くことはほとんどない：

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

## よくある誤り

左列は無効または誤解を招く記述である。自分のファイル内の名前を当てはめつつ、右列の修正例だけを参考にすること。

|誤り|修正|
|---|---|
|`#M1 @PLACE (10, 20);`|`#M1 @PLACE(10, 20);`|
|`#M1 (left) #M2 (right) @AS $CROSS_COUPLED_PAIR;`|`#M1(left) #M2(right) @AS $CROSS_COUPLED_PAIR;`|
|`#M1 #M2 @SET-SYMBOL $NMOS;`|`{#M1 #M2} @SET-SYMBOL $NMOS;`|
|`#M5 #M3 @AS $MATCHED;`|`{#M5 #M3} @AS $MATCHED;`|
|`{#R1 #R2} @ORDER("x");`|`{#R1 #R2} @ORDER("left-to-right");`|
|`#R1 @PLACE(x=2u);`|`#R1 @PLACE(x=2);`|
|0.3の名前 `$DIFFERENTIAL_PAIR`、ロール `in_p/in_n`|`#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N;`|
|`#R1 @SET-SYMBOL $GROUND;`|グラウンドには `$GROUND @ATTACH &n;` を使い、抵抗のシンボルはそのままにする|
|ピンを揃えるつもりの `%t #M1 @ALIGN("y");`|`{%t::pin #M1::g} @ALIGN("y");`|
|非ポートネットに対する `$IO_PORT @ATTACH &n;`|`$NET_LABEL @ATTACH &n;`|
|vssがグラウンドセットに入っているときの `&vss @ROUTE-NET;`|`&vss @ROUTE-NET(priority="high");`|
|手書きの記述内の `%#3 @PLACE(1, 2);`|そのオブジェクトに名前を付け、その名前を使う|
|`#M1.d; M3; &vdd!;`|`#M1:d; #M3; &"vdd!";`|
|`{@VERSION("0.4");}`|`{ @VERSION("0.4");}`|

`$NMOS` のバルクへ配線しようとすると失敗するため、`$NMOS_4TERM` を用いること。型が不明なトランジスタには、モデルカード、DEVICE、または明示的なシンボルが必要である。トップレベルのルールをサブサーキットまで到達させるには、`recursive=true` を指定する必要がある。

## 完全な例

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

ファイルを提出する前の確認事項：先頭にVERSIONとパッケージを含むヘッダーレコードを置いているか；すべての名前がそのレベルに存在するか；カタログにある0.4の名前のみを使用しているか；すべての `@` の前に空白、すべての文の後に `;` があるか；方向を明記しているか；セットロールにセットを渡しているか；グラウンドと電源をそれぞれのセットに入れているか；向きは必要な箇所だけに、電流は確実な箇所だけに記述しているか；揃えたピン同士を配線しているか；生成済みレコードを編集していないか；意図しない違反が残っていないか。

<!-- lists:begin -->
## カタログ

本節はそのままコピーして使うRNLではなくリファレンスであり、名前の `$` はそのまま記述する。定義はASまたはATTACHで適用し、シンボルはSET-SYMBOL、スタイルはSET-STYLEで割り当てる。ロールは宣言順に並び、それぞれが受け取る種別を伴う（種別のないロールはどの種別のオブジェクトも受け取る）。`{role}` はセット全体を受け取り、省略してもよい。パラメータは `;` の後に続く。したがって `$MATCHED({members}: instance/usage)` は `{#M1 #M2} @AS $MATCHED;` のように使用する。

### パッケージ

各パッケージはヘッダー内で `@USE-PACKAGE(name="...", version="0.4");` により指定する。

|パッケージ|内容|
|---|---|
|rnl-std-all|シリーズ0.4で唯一のパッケージコレクションであり、それ自体は内容を持たない。スタイル、基本および半導体のシンボルと定義、自動補完、回路セマンティクス、4つの特定分野パッケージ、ならびに各ホスト方言のルールパッケージを、その言語用のヘッダーレコードを通じて取り込む（2.8、16.3）。|
|rnl-std-style|スタイル（9.3）と既定のスタイルルール。|
|rnl-std-basic|受動素子、電源・信号源、スイッチ、伝送線路、仮想部品、マークのシンボル、ならびに仮想部品、マーク、図形、スタブの定義（それぞれ適用オブジェクトを描画するルールを含む）、およびグラウンド、電源、バイアスネットのセット。|
|rnl-std-semiconductors|ダイオード、MOSトランジスタ、バイポーラトランジスタ、JFET、ユニジャンクショントランジスタ（UJT、PUT）のシンボル。|
|rnl-std-spice|SPICE用のルールパッケージ：素子文字、モデル型、およびバルクまたは制御負側ネットによってインスタンスへシンボルを割り当てる（16.3）。|
|rnl-std-spectre|Spectre用のルールパッケージ：マスタ、モデル型、およびバルクまたは制御負側ネットによってインスタンスへシンボルを割り当てる（16.3）。|
|rnl-std-fill|自動補完のセットとルール：仮想部品のピンを含む未配線ピンにグラウンド、電源、バイアス、ネットラベルの各スタブを付与し、`$PORT_FILL_SET` 内のすべてのポートネットにポートタグを付与する（10.5、12.4、16.3）。|
|rnl-std-semantics|回路構造およびその既定の描画に関する意味定義。|
|rnl-std-functional|ビヘイビアモデル：オペアンプ、コンパレータ、増幅器、トランスコンダクタ、ADC、DAC、チョッパ変調器。*_SUPPLY変種は整数グリッド上のvp（上）ピンとvn（下）ピンを持つ。|
|rnl-std-rf|RF部品：センタータップ付きインダクタとTコイル。|
|rnl-std-logic|論理ゲート、バッファ、インバータ、遅延セル、Dフリップフロップ。|
|rnl-std-signal-flow|理想信号フローの仮想部品とそのシンボル：加算器、減算器、乗算器、連続時間積分器、離散時間積分器、量子化器、単位遅延。|
|rnl-std-no-labels|独自のラベルを記述するファイル向けに、標準シンボルの既定ラベルを非表示にする（16.3）。|
|rnl-std-no-instance-labels|インスタンスの既定ラベルのみを非表示にする。グラウンド、電源、バイアスマーク、ポートタグ、ネットラベルが描くネット名は残る（16.3）。|
|rnl-std-45deg|ブリッジ回路、交差結合対、斜め巻線、および45度配線用の45度シンボル。|

### 定義

描画列は、rnl-std-semanticsの構造について次を示す：*意味のみ* は配置も配線も行わない；*配置* はメンバーに位置の事実を持つ；*配線* はメンバーが共有するネットを結ぶ。これらの位置の事実と配線はすべて優先度lowである。グループ列は各定義の定義本体から読み取ったものである：1つのグループにまとめるロールを伴う *あり*、グループを作る定義 `$X` に渡すロールを伴う `$X` *経由*、グループを作らない場合は空欄となる。カレントミラーのように一部が別の構造の内側に跨り得る構造はグループを作らない（*レイアウトの事実* の下のグループのツリーを参照）。シンボルのない適用オブジェクトは、ALIGN、ORDER、DISTRIBUTE、DISTANCEではそのメンバーを表す。その自身の点 `%u::position`（14.1）がメンバーに結び付くのは *自身の点* を持つ構造に限られ、そのような適用オブジェクトだけが `::position` によって1つの項目として、あるいは配置を行う別の構造のメンバーとして動く。それ以外の構造では、メンバーまたはその名前付きセットを制約すること。他のパッケージは描画される部品およびマークを定義する。

|シグネチャ|描画|グループ|意味|
|---|---|---|---|
|**rnl-std-basic**||||
|`$GROUND(p: net)`|—||ネット上のグラウンドであり、自身の点に描画される。ネット0上では3本線の記号のみが描かれ、それ以外のネット上ではネット名も表示される。|
|`$SUPPLY(p: net)`|—||ネット上の電源であり、自身の点に描画される。|
|`$BIAS(p: net)`|—||ネット上のバイアス電圧マークであり、自身の点に描画される。|
|`$IO_PORT(p: net)`|—||ネット上のポートタグであり、自身の点に描画される。対象ネットはそのレベルのポートでなければならない。|
|`$NET_LABEL(p: net/pin)`|—||ネットラベル：短い引き出し線付きのネット名で、自身の点に描画される。バンドルのピンなどのバスピンが渡された場合は（11.8）、ピンをそのまま保持してそのすべてのネットに対する1つのラベルとなり、各ネット名を表示する（13.6）。|
|`$NO_CONNECT(t: terminal/pin)`|—||意図的に未接続とした端子またはピンに付ける未接続マーク。バスビット上のマークでどのビットが開放されているかを記録できるよう、端子はそのまま保持される（12.6）。|
|`$JUNCTION(p: point/terminal)`|—||点または端子上の接続点の黒丸。|
|`$CURRENT_ARROW(p: terminal/point)`|—||端子または点上の電流矢印。|
|`$ARROW(at: point)`|—||シンボル内またはシーン内の点に描画される矢じり（9.2.7）。|
|`$SPDT_SWITCH(at: point)`|—||ネットリスト行を持たず、点上に図形として描画される単極双投（SPDT）スイッチ。|
|`$FIGURE(at: point)`|—||ネットリスト行を持たず、点上に描画される図形。SET-SYMBOLで独自のシンボルを割り当てる。|
|`$PORT_LIST()`|—||まだ `$IO_PORT` が付いていないレベルのすべてのポートネットに対し、rnl-std-fillと同様にそのレベルのインスタンス群より少なくとも6単位左に、上から順に並べてグループにまとめ、それぞれR0でピンを右向きにして、すべて優先度lowで縦1列にポートタグを配置する。この場合、自動補完はそのレベルにポートタグを追加しない（12.4）。|
|`$COMMENT({target}; text="")`|—||任意のオブジェクトに付与する描画されないコメント。targetを省略した場合はレベル自体に付与される。|
|`$GROUND_STUB(t: pin; gap=3)`|—||ピンの近くに置く、正立した、自身の配線付きのグラウンド：ピンと同じ高さかその下に置かれ、縦方向の距離は最大gap単位（優先度low）。|
|`$SUPPLY_STUB(t: pin; gap=3)`|—||ピンの近くに置く、正立した、自身の配線付きの電源：ピンと同じ高さかその上に置かれ、縦方向の距離は最大gap単位（優先度low）。|
|`$BIAS_STUB(t: pin)`|—||ピン上に直接取り付けるバイアス電圧マーク。ポートスタブと同様にピンから離れる向きを向き、そのバーはピンの上・下・左側では / に、右側では \ に傾く。|
|`$IO_PORT_STUB(t: pin)`|—||ピン上に直接取り付けるポートタグ。その引き出し線がスタブの短い配線として機能し、`$NET_LABEL_STUB` と同様にピンとタグは配線されないままとなる。|
|`$NET_LABEL_STUB(t: pin)`|—||ピン上に直接取り付けるネットラベル。その1単位長の引き出し線がスタブの短い配線として機能し、バスピン上ではバス全体に対する1つのラベルとなる。ピンとラベルは未配線のままにされる。|
|**rnl-std-semantics**||||
|`$MATCHED({members}: instance/usage)`|意味のみ||型または定義、およびレイアウトが整合（マッチ）した素子またはサブ構造。|
|`$MATCHED_MOS({members}: instance)`|意味のみ||型、モデル、およびチャネル長が整合したMOSFET。|
|`$MATCHED_BJT({members}: instance)`|意味のみ||型、モデル、および単位エミッタ形状が整合したバイポーラトランジスタ。|
|`$MATCHED_RESISTOR({members}: instance)`|意味のみ||型とモデルが整合した抵抗。|
|`$MATCHED_CAPACITOR({members}: instance)`|意味のみ||型とモデルが整合したキャパシタ。|
|`$MIRRORED_LAYOUT(left: instance/usage, right: instance/usage, axis: point/instance/usage)`|配置；自身の点を中央に結ぶ||同じ高さにあり、軸に対して互いに鏡像対称となる2つの対称なオブジェクト。レイアウトのみを表し、回路について何も述べない。|
|`$MIRRORED_VERTICAL_LAYOUT(top: instance/usage, bottom: instance/usage, axis: point/instance/usage)`|配置；自身の点を中央に結ぶ||縦1列に並び、水平軸に対して互いに鏡像対称となる2つの対称なオブジェクト。レイアウトのみを表す。|
|`$BRANCH({members}: instance/usage)`|配置、配線；自身の点を x に結ぶ||縦に積まれた直列構造：縦1列に上から下へ直列接続されたメンバー群であり、通常は電源からグラウンドへの経路などの DC ブランチに用いる。|
|`$BRANCH_HORIZONTAL({members}: instance/usage)`|配置、配線；自身の点を y に結ぶ||横1行に描かれる直列構造：同じ高さに左から右へ直列接続されたメンバー群。|
|`$CASCADE({stages}: instance/usage)`|意味のみ||信号経路に沿って縦続接続（カスケード接続）された段。|
|`$SERIES({members}: instance/usage)`|配線||1つの電流経路に沿って数珠つなぎ（直列）に接続されたメンバー群。|
|`$PARALLEL({members}: instance/usage)`|配線||同じ2つのネット間に並列接続されたメンバー群。|
|`$PARALLEL_HORIZONTAL({members}: instance/usage)`|配置、配線；自身の点を中央に結ぶ||同じ高さに横並びで並列接続され、列の中では1つの位置として扱われるメンバー群。|
|`$PARALLEL_VERTICAL({members}: instance/usage)`|配置、配線；自身の点を中央に結ぶ||同じx座標に上下に重ねて並列接続され、行の中では1つの位置として扱われるメンバー群。|
|`$SIGNAL_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|意味のみ||一方向に流れる信号経路：信号源から経路上の各段を記述順に経て負荷へ至る。|
|`$POWER_FLOW({path}: instance/usage, {source}: instance/usage/terminal/net, {load}: instance/usage/terminal/net)`|意味のみ||電力の経路：電源や電池などの供給源から、電力を変換またはスイッチする経路上の各段を記述順に経て、電力を消費する負荷へ至る。|
|`$BIAS_CIRCUIT({members}: instance/usage)`|意味のみ||バイアス回路：電流基準回路など、他の回路部分のバイアスを生成する素子または構造。|
|`$FEEDBACK_LOOP({forward}: instance/usage, {feedback}: instance/usage)`|意味のみ||帰還ループ：順方向経路と帰還回路網。|
|`$DIFFERENTIAL_PAIR_N(left: instance, right: instance, tail: instance/usage)`|配置、配線；自身の点を中央に結ぶ|あり：left, right, tail|テールを下に置くn型トランジスタの差動対。|
|`$DIFFERENTIAL_PAIR_P(left: instance, right: instance, tail: instance/usage)`|配置、配線；自身の点を中央に結ぶ|あり：left, right, tail|テールを上に置くp型トランジスタの差動対。|
|`$COMPLEMENTARY_DIFFERENTIAL_PAIR(pairup: usage, pairdown: usage)`|配線|あり：pairup, pairdown|レールツーレール入力段：同じ入力を共有する上側と下側の差動対。|
|`$PSEUDO_DIFFERENTIAL_PAIR(left: instance, right: instance)`|配置；自身の点を中央に結ぶ||疑似差動対：テールを持たず、鏡像対称に描かれる2つの整合入力トランジスタ。|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_H(left: instance, right: instance, tailleft: instance/usage, tailright: instance/usage, bridge: instance/usage)`|配置、配線；自身の点を中央に結ぶ|あり：left, right, tailleft, tailright, bridge|2つのテールの間をブリッジでディジェネレーションしたH型差動対。|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_N(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|配置、配線；自身の点を中央に結ぶ|あり：left, right, degleft, degright, tail|各入力素子の下の要素でディジェネレーションしたn型トランジスタのT型差動対で、テールは下側。|
|`$DEGENERATED_DIFFERENTIAL_PAIR_TYPE_T_P(left: instance, right: instance, degleft: instance/usage, degright: instance/usage, tail: instance/usage)`|配置、配線；自身の点を中央に結ぶ|あり：left, right, degleft, degright, tail|各入力素子の上の要素でディジェネレーションしたp型トランジスタの逆T型差動対で、テールは上側。|
|`$CROSS_COUPLED_PAIR(left: instance, right: instance)`|配置、配線；自身の点を中央に結ぶ|あり：left, right|交差結合対（クロスカップルドペア）：正帰還で交差接続され、鏡像対称に描かれる2つの整合トランジスタ。|
|`$CURRENT_MIRROR_N({ref}: instance, {out}: instance)`|配置；自身の点を y に結ぶ||ソースまたはエミッタを下向きにしたn型トランジスタのローサイドカレントミラー。|
|`$CURRENT_MIRROR_P({ref}: instance, {out}: instance)`|配置；自身の点を y に結ぶ||ソースまたはエミッタを上向きにしたp型トランジスタのハイサイドカレントミラー。|
|`$CASCODE_CURRENT_MIRROR_N({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|配置；自身の点を y に結ぶ||ローサイドカスコードカレントミラー：下段のミラー行とその上段のカスコード行からなる。|
|`$CASCODE_CURRENT_MIRROR_P({ref}: instance, {out}: instance, {refcascode}: instance, {outcascode}: instance)`|配置；自身の点を y に結ぶ||ハイサイドカスコードカレントミラー：上段のミラー行とその下段のカスコード行からなる。|
|`$TRANS_LINEAR_LOOP({a}: instance/usage, {b}: instance/usage)`|配置、配線；自身の点を x に結ぶ|あり：a, b|1列に並べて描かれる2組のトランジスタ群によるトランスリニアループ。|
|`$TRANS_LINEAR_LOOP_HORIZONTAL({a}: instance/usage, {b}: instance/usage)`|配置、配線；自身の点を y に結ぶ|あり：a, b|1行に並べて描かれる2組のトランジスタ群によるトランスリニアループ。|
|`$DIODE_CONNECTED(device: instance)`|配線||ゲートをドレインへ（またはベースをコレクタへ）接続したダイオード接続トランジスタ。|
|`$COMMON_SOURCE(device: instance)`|意味のみ||1個のFETによるソース接地段。|
|`$COMMON_GATE(device: instance)`|意味のみ||1個のFETによるゲート接地段。|
|`$COMMON_DRAIN(device: instance)`|意味のみ||1個のFETによるソースフォロワ（ドレイン接地段）。|
|`$COMMON_EMITTER(device: instance)`|意味のみ||1個のバイポーラトランジスタによるエミッタ接地段。|
|`$COMMON_BASE(device: instance)`|意味のみ||1個のバイポーラトランジスタによるベース接地段。|
|`$COMMON_COLLECTOR(device: instance)`|意味のみ||1個のバイポーラトランジスタによるエミッタフォロワ（コレクタ接地段）。|
|`$FOLLOWER(device: instance)`|意味のみ||1個のトランジスタによる電圧フォロワ段。|
|`$CASCODE(source: instance/usage, cascode: instance)`|配線||カスコード：ソース側素子またはサブ構造とゲート接地トランジスタの直列構成。|
|`$FOLDED_CASCODE(source: instance/usage, cascode: instance, bias: instance/usage)`|配線||フォールデッドカスコード：バイアスを伴うゲート接地トランジスタへ折り返された入力素子。|
|`$COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage)`|配線|`$TRANSCONDUCTOR_Z_AMPLIFIER` 経由：input, load|ソース接地増幅器：FET入力トランジスタとその負荷。|
|`$COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage)`|配線|`$TRANSCONDUCTOR_Z_AMPLIFIER` 経由：input, load|エミッタ接地増幅器：バイポーラ入力トランジスタとその負荷。|
|`$DEGENERATED_COMMON_SOURCE_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|配線|`$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER` 経由：input, load, degenerator|ディジェネレーション付きソース接地増幅器：FET入力トランジスタ、その負荷、およびディジェネレーション要素。|
|`$DEGENERATED_COMMON_EMITTER_AMPLIFIER(input: instance, load: instance/usage, degenerator: instance/usage)`|配線|`$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER` 経由：input, load, degenerator|ディジェネレーション付きエミッタ接地増幅器：バイポーラ入力トランジスタ、その負荷、およびディジェネレーション要素。|
|`$MILLER_AMPLIFIER(stage: instance/usage, compensation: instance/usage)`|配線|あり：stage, compensation|ミラー補償増幅器：利得段とその位相補償回路網。|
|`$PUSH_PULL_STAGE(pullup: instance/usage, pulldown: instance/usage)`|配線|あり：pullup, pulldown|プッシュプル出力段：直列接続されたプルアップとプルダウンの出力トランジスタのみからなり、そのバイアスや駆動段は含まない。|
|`$TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage)`|配線|あり：transconductor, load|増幅段：トランスコンダクタとその負荷。|
|`$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER(transconductor: instance/usage, load: instance/usage, degenerator: instance/usage)`|配線|あり：transconductor, load, degenerator|ディジェネレーション付き増幅段：トランスコンダクタ、その負荷、およびディジェネレーション要素。|
|`$PASS_SWITCH(device: instance/usage)`|意味のみ||両端の間で信号を通過または遮断するスイッチ。|
|`$RESET_SWITCH(device: instance)`|意味のみ||プリチャージ、イコライズ、またはリセット用のトランジスタ。|
|`$GROUND_SWITCH(device: instance)`|意味のみ||グラウンドネットに接続されたローサイドスイッチ。|
|`$POWER_SWITCH(device: instance)`|意味のみ||電源ネットに接続されたハイサイドスイッチ。|
|`$TRANSMISSION_GATE(pmos: instance/usage, nmos: instance/usage)`|配置、配線；自身の点を中央に結ぶ|あり：pmos, nmos|PMOSを上、NMOSを下に配置したCMOSトランスミッションゲート。|
|`$TRANSMISSION_GATE_HORIZONTAL(pmos: instance/usage, nmos: instance/usage)`|配置、配線；自身の点を中央に結ぶ|あり：pmos, nmos|PMOSを左にして横並びに配置したCMOSトランスミッションゲート。|
|`$MULTI_POLE_SWITCH({throws}: instance/usage)`|配線|あり：throws|1つの極（ポール）を共有する複数の接点（スロー）を持つスイッチ。|
|`$BOOTSTRAPPED_SWITCH(device: instance/usage)`|意味のみ||ブートストラップサンプリングスイッチ。|
|`$SC_SAMPLER({switch}: instance/usage, {capacitor}: instance/usage)`|配線||スイッチトキャパシタサンプラ：サンプリングスイッチとサンプリングキャパシタ。|
|`$CHARGE_SHARING(a: instance/usage, switch: instance/usage, b: instance/usage)`|配線||電荷再配分回路網：スイッチを介して電荷を共有する2つのキャパシタまたはキャパシタバンク。|
|`$CHOPPER({straight}: instance/usage, {crossed}: instance/usage)`|配線||チョッパ：信号の極性を反転させる4個のスイッチからなるブリッジ回路。|
|`$FLYING_CAPACITOR(capacitor: instance/usage)`|意味のみ||クロック位相間で切り替えられるフライングキャパシタ。|
|`$CMOS_INVERTER(pmos: instance, nmos: instance)`|配置、配線；自身の点を中央に結ぶ|`$CMOS_GATE` 経由：pmos, nmos|PMOSを上、NMOSを下に配置したCMOSインバータ。|
|`$CMOS_INVERTER_HORIZONTAL(pmos: instance, nmos: instance)`|配置、配線；自身の点を中央に結ぶ|`$CMOS_GATE` 経由：pmos, nmos|PMOSを左にして横並びに配置したCMOSインバータ。|
|`$CMOS_GATE({pullup}: instance/usage, {pulldown}: instance/usage)`|配線|あり：pullup, pulldown|静的CMOS論理ゲート：出力ノードで直列に接続されるプルアップ回路網とプルダウン回路網。|
|`$LATCH({members}: instance/usage)`|配線|あり：members|ラッチ：双安定コアとその周囲の素子。|
|`$INVERTER_LATCH(a: instance/usage, b: instance/usage)`|配線|`$LATCH` 経由：a, b|交差接続された2つのインバータによるラッチ。|
|`$SHIFT_REGISTER({stages}: instance/usage)`|意味のみ||シフトレジスタ：フリップフロップまたはラッチの連鎖であり、しばしば重ねて描かれ、クロックはめったに配線されない。|
|`$DIVIDER(upper: instance/usage, lower: instance/usage)`|配線||上側と下側の2つのブランチを直列に接続した分圧器・分流器。|
|`$RESISTOR_LADDER({resistors}: instance)`|配置、配線；自身の点を x に結ぶ|あり：resistors|縦1列に上から下へ直列に接続された抵抗ラダー。|
|`$RESISTOR_LADDER_HORIZONTAL({resistors}: instance)`|配置、配線；自身の点を y に結ぶ|あり：resistors|横1行に直列に接続された抵抗ラダー。|
|`$PI_NETWORK(shunt1: instance/usage, series: instance/usage, shunt2: instance/usage)`|配線||π型回路網：並列（シャント）ブランチ、直列ブランチ、もう1つの並列ブランチからなる。|
|`$T_NETWORK(series1: instance/usage, shunt: instance/usage, series2: instance/usage)`|配線||T型回路網：直列ブランチ、並列（シャント）ブランチ、もう1つの直列ブランチからなる。|
|`$LC_TANK_PARALLEL(inductor: instance/usage, capacitor: instance/usage)`|配線|あり：inductor, capacitor|並列LC共振タンク。|
|`$RLC_TANK_PARALLEL(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|配線|あり：resistor, inductor, capacitor|並列RLC共振タンク。|
|`$LC_TANK_SERIES(inductor: instance/usage, capacitor: instance/usage)`|配線|あり：inductor, capacitor|直列LC共振タンク。|
|`$RLC_TANK_SERIES(resistor: instance/usage, inductor: instance/usage, capacitor: instance/usage)`|配線|あり：resistor, inductor, capacitor|直列RLC共振タンク。|
|`$DC_FEED(device: instance/usage)`|意味のみ||ノードへDC電源またはバイアスを供給するRFチョークまたはバイアス抵抗。|
|`$DECAP(device: instance)`|配置；自身の点を x に結ぶ||電源ノードまたはバイアスノード上のデカップリングキャパシタ。|
|`$AC_COUPLE(device: instance/usage)`|意味のみ||AC信号を通過させDC成分を遮断する直列結合キャパシタ。|
|`$OSCILLATOR({members}: instance/usage)`|配線|あり：members|能動素子と共振素子または遅延要素からなる発振器。|
|**rnl-std-signal-flow**||||
|`$ADDER(in1: net, in2: net, out: net)`|—||`$ADDER_SYMBOL` で描画される理想信号フロー加算器（out = in1 + in2）。|
|`$SUBTRACTOR(in1: net, in2: net, out: net)`|—||`$SUBTRACTOR_SYMBOL` で描画される理想信号フロー減算器（out = in1 - in2）。|
|`$MULTIPLIER(in1: net, in2: net, out: net)`|—||`$MULTIPLIER_SYMBOL` で描画される理想信号フロー乗算器（out = in1 * in2）。|
|`$INTEGRATOR(in: net, out: net)`|—||`$INTEGRATOR_SYMBOL` で描画される理想連続時間積分器。|
|`$DISCRETE_TIME_INTEGRATOR(in: net, out: net)`|—||`$DISCRETE_TIME_INTEGRATOR_SYMBOL` で描画される理想離散時間積分器。|
|`$QUANTIZER(in: net, out: net)`|—||`$QUANTIZER_SYMBOL` で描画される理想量子化器。|
|`$UNIT_DELAY(in: net, out: net)`|—||`$UNIT_DELAY_SYMBOL` で描画される理想単位遅延。|
|**rnl-std-45deg**||||
|`$ARROW_45(at: point)`|—||シンボル内の点、またはシーン内の45度配線上に描画される45度の矢じり。|
|`$CURRENT_ARROW_45(p: terminal/point)`|—||45度ブランチ上の端子または点に置く45度の電流矢印。|

### シンボル

ピンは束縛先の端子またはロールで列挙する。`pin`（`p`）はロール `p` を束縛する名前 `pin` のピンであり、`pin` (nothing) は何も束縛しないことを示す。

|シンボル|ピン|意味|
|---|---|---|
|**rnl-std-basic**|||
|`$RESISTOR`|`p` `n`|抵抗。|
|`$VARIABLE_RESISTOR`|`p` `n`|可変抵抗：矢印が横切る抵抗。|
|`$CAPACITOR`|`p` `n`|キャパシタ。|
|`$VARIABLE_CAPACITOR`|`p` `n`|可変キャパシタ：矢印が横切るキャパシタ。|
|`$INDUCTOR`|`p` `n`|インダクタ。|
|`$VARIABLE_INDUCTOR`|`p` `n`|可変インダクタ：矢印が横切るインダクタ。|
|`$TRANSFORMER`|`p1` `n1` `p2` `n2`|変圧器（トランス）：横に並んだ2つの結合巻線で、極性ドットは上側にある。|
|`$POTENTIOMETER`|`n1` `nw` `n2`|中央にワイパを持つ3端子ポテンショメータ。|
|`$PHOTORESISTOR`|`p` `n`|フォトレジスタ（光依存抵抗）。|
|`$POLARIZED_CAPACITOR`|`p` `n`|有極（電解）キャパシタ。p側が正極板。|
|`$CRYSTAL`|`n1` `n2`|水晶振動子。|
|`$SPEAKER`|`p` `n`|スピーカ。|
|`$VOLTAGE_SOURCE`|`p` `n`|独立電圧源。|
|`$CURRENT_SOURCE`|`p` `n`|独立電流源。矢印はpからnへ向かう。|
|`$BATTERY`|`p` `n`|電池。|
|`$CONTROLLED_VOLTAGE_SOURCE`|`p` `n`|出力ピンのみを持ち、ひし形で描かれる制御電圧源。|
|`$CONTROLLED_CURRENT_SOURCE`|`p` `n`|出力ピンのみを持ち、ひし形で描かれる制御電流源。|
|`$VCVS_4TERM`|`p` `n` `cp` `cn`|左側に制御ピンを持つ電圧制御電圧源（E）。|
|`$VCCS_4TERM`|`p` `n` `cp` `cn`|左側に制御ピンを持つ電圧制御電流源（G）。|
|`$BEHAVIORAL_SOURCE`|`p` `n`|ビヘイビア電源（B）：Bと記された円。|
|`$RF_PORT`|`p` `n`|RFポート：内部抵抗と直列の正弦波源（Spectreのport用）。|
|`$SINE_VOLTAGE_SOURCE`|`p` `n`|正弦波電圧源。p側が +。|
|`$PULSE_VOLTAGE_SOURCE`|`p` `n`|パルス電圧源。p側が +。|
|`$MICROPHONE`|`p` `n`|マイクロホン。p側が +。|
|`$AMMETER`|`p` `n`|電流計。電流はpからnへ向かって計測する。|
|`$SWITCH`|`p` `n`|開状態で描かれるスイッチ。電流制御スイッチ（W）またはSpectreのswitch用。|
|`$CLOSED_SWITCH`|`p` `n`|閉状態で描かれるスイッチ。|
|`$EXTERNALLY_CONTROLLED_SWITCH`|`p` `n` `cp`|側面に単一の制御ピンを持つスイッチ。|
|`$VOLTAGE_CONTROLLED_SWITCH`|`p` `n` `cp` `cn`|下側に差動制御ピンを持つ電圧制御スイッチ（S）。|
|`$SPDT_SWITCH_SYMBOL`|`com` (nothing) `a` (nothing) `b` (nothing)|`$SPDT_SWITCH` のシンボル：ピンcom、a、bは何も束縛せず、名前によって配線される単極双投（SPDT）スイッチ（9.2.4）。|
|`$TRANSMISSION_LINE`|`p1` `n1` `p2` `n2`|伝送線路（TまたはO）。左側にポート1、右側にポート2。|
|`$RC_LINE`|`a` `b` `ref`|一様分布RC線路（U）。|
|`$GROUND_SYMBOL`|`pin` (`p`)|ネット0上の `$GROUND` のシンボル：上端にピンを持つ3本線のグラウンド。|
|`$NAMED_GROUND_SYMBOL`|`pin` (`p`)|0以外の任意のネット上の `$GROUND` のシンボル：下側にネット名を持つ3本線のグラウンド。|
|`$SUPPLY_SYMBOL`|`pin` (`p`)|`$SUPPLY` のシンボル：ネット名付きの水平バーで、下端にピンを持つ。|
|`$BIAS_SYMBOL`|`pin` (`p`)|`$BIAS` のシンボル：ネット名付きの斜めバーで、下端にピンを持つ。|
|`$IO_PORT_SYMBOL`|`pin` (`p`)|`$IO_PORT` のシンボル：ネット名付きの小さな円で、右側にピンを持つ。|
|`$NET_LABEL_SYMBOL`|`pin` (`p`)|`$NET_LABEL` のシンボル：`$IO_PORT_SYMBOL` と同様にピンが右向きで、1単位長の引き出し線の左側にネット名（バスピンの場合は各ネット名）を置く。|
|`$ARROW_SYMBOL`|—|右向きの塗りつぶし矢じり。`$ARROW` のシンボルであり、`$CURRENT_ARROW` はその半分の大きさのものを描く。|
|`$NO_CONNECT_SYMBOL`|—|`$NO_CONNECT` のシンボル：ばつ印（×）。|
|`$JUNCTION_SYMBOL`|—|`$JUNCTION` のシンボル：塗りつぶしの黒丸。|
|**rnl-std-semiconductors**|||
|`$DIODE`|`a` `k`|ダイオード。|
|`$ZENER_DIODE`|`a` `k`|ツェナーダイオード。|
|`$LED`|`a` `k`|発光ダイオード（LED）。|
|`$SCHOTTKY_DIODE`|`a` `k`|ショットキーバリアダイオード。|
|`$SHOCKLEY_DIODE`|`a` `k`|ショックレー4層ダイオード。|
|`$PUT`|`a` `g` `k`|プログラマブル単接合トランジスタ（PUT）。|
|`$NMOS`|`d` `g` `s`|バルクを描かない3ピンのNMOSトランジスタ。|
|`$NMOS_4TERM`|`d` `g` `s` `b`|バルクピンをドレイン・ソースと揃え、チャネルにバルク矢印を持つNMOSトランジスタ。|
|`$PMOS`|`d` `g` `s`|バルクを描かない3ピンのPMOSトランジスタ。|
|`$PMOS_4TERM`|`d` `g` `s` `b`|バルクピンをドレイン・ソースと揃えたPMOSトランジスタ。|
|`$NMOS_LOGIC`|`d` `g` `s`|ソース矢印を持たない3ピンのロジックNMOSトランジスタ。|
|`$PMOS_LOGIC`|`d` `g` `s`|ソース矢印を持たず、ゲートに反転丸を持つ3ピンのロジックPMOSトランジスタ。|
|`$DEPLETION_NMOS`|`d` `g` `s`|バルクを描かない3ピンのデプレッション型NMOSトランジスタ。|
|`$DEPLETION_NMOS_4TERM`|`d` `g` `s` `b`|バルクピンをドレイン・ソースと揃えたデプレッション型NMOSトランジスタ。|
|`$DEPLETION_PMOS`|`d` `g` `s`|バルクを描かない3ピンのデプレッション型PMOSトランジスタ。|
|`$DEPLETION_PMOS_4TERM`|`d` `g` `s` `b`|バルクピンをドレイン・ソースと揃えたデプレッション型PMOSトランジスタ。|
|`$NDMOS`|`d` `g` `s`|バルクを描かない3ピンのN型DMOSトランジスタ。|
|`$NDMOS_4TERM`|`d` `g` `s` `b`|バルクピンをドレイン・ソースと揃えたN型DMOSトランジスタ。|
|`$PDMOS`|`d` `g` `s`|バルクを描かない3ピンのP型DMOSトランジスタ。|
|`$PDMOS_4TERM`|`d` `g` `s` `b`|バルクピンをドレイン・ソースと揃えたP型DMOSトランジスタ。|
|`$NPN`|`c` `b` `e`|サブストレートを描かないNPNバイポーラトランジスタ。|
|`$PNP`|`c` `b` `e`|サブストレートを描かないPNPバイポーラトランジスタ。|
|`$NJFET`|`d` `g` `s`|チャネル向きにゲート矢印を持つNチャネルJFET。|
|`$PJFET`|`d` `g` `s`|チャネルから離れる向きにゲート矢印を持つPチャネルJFET。|
|`$UJT`|`e` `b1` `b2`|単接合トランジスタ（UJT）。上側がb2、下側がb1。|
|**rnl-std-functional**|||
|`$OPAMP`|`inp` `inn` `out`|inp、inn、outの3端子を持つオペアンプ。|
|`$OPAMP_SUPPLY`|`inp` `inn` `out` `vp` `vn`|電源ピンを持つオペアンプ：inp、inn、out、vp、vn。|
|`$OPAMP_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|電源ピンを持つオペアンプで、入力を入れ替え（innが上、inpが下）、電源は保つ（vpが上、vnが下；`$OPAMP_SUPPLY` をMX反転しただけではそうならない）：inp、inn、out、vp、vn。|
|`$DIFFERENTIAL_OPAMP`|`inp` `inn` `outp` `outn`|完全差動オペアンプ：inp、inn、outp、outn。|
|`$DIFFERENTIAL_OPAMP_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|電源ピンを持つ完全差動オペアンプ：inp、inn、outp、outn、vp、vn。|
|`$DIFFERENTIAL_OPAMP_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|電源ピンを持つ完全差動オペアンプで、入力と出力を入れ替え（innとoutnが上、inpとoutpが下）、電源は保つ（vpが上、vnが下；`$DIFFERENTIAL_OPAMP_SUPPLY` をMX反転しただけではそうならない）：inp、inn、outp、outn、vp、vn。|
|`$DIFFERENTIAL_OPAMP_TWISTED`|`inp` `inn` `outp` `outn`|出力極性を入れ替えた（outnが上、outpが下）完全差動オペアンプ：inp、inn、outp、outn。|
|`$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|出力極性を入れ替え、電源ピンを備えた完全差動オペアンプ：inp、inn、outp、outn、vp、vn。|
|`$DIFFERENTIAL_OPAMP_TWISTED_INVERTED_SUPPLY`|`inp` `inn` `outp` `outn` `vp` `vn`|電源ピンを持つ完全差動オペアンプで、入力を入れ替え（innが上、inpが下）、出力は `$DIFFERENTIAL_OPAMP_SUPPLY` と同じ向き（outpが上、outnが下）に、電源は保つ（vpが上、vnが下；`$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY` をMX反転しただけではそうならない）：inp、inn、outp、outn、vp、vn。|
|`$COMPARATOR`|`inp` `inn` `out`|inp、inn、outの3端子を持つコンパレータ。|
|`$COMPARATOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|電源ピンを持つコンパレータ：inp、inn、out、vp、vn。|
|`$COMPARATOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|電源ピンを持つコンパレータで、入力を入れ替え（innが上、inpが下）、電源は保つ（vpが上、vnが下；`$COMPARATOR_SUPPLY` をMX反転しただけではそうならない）。ステップ記号の向きは `$COMPARATOR_SUPPLY` と同じに保たれる：inp、inn、out、vp、vn。|
|`$VOLTAGE_AMPLIFIER`|`in` `out`|keyがlabelの任意の中央テキストを持つ電圧増幅器：in、out。|
|`$VOLTAGE_AMPLIFIER_SUPPLY`|`in` `out` `vp` `vn`|電源ピンを持つ電圧増幅器：in、out、vp、vn。|
|`$TRANSCONDUCTOR`|`in` `out`|keyがlabelの中央テキスト（既定はg_m）を持つトランスコンダクタ（gmセル）：in、out。|
|`$TRANSCONDUCTOR_SUPPLY`|`in` `out` `vp` `vn`|電源ピンと、keyがlabelの中央テキスト（既定はg_m）を持つトランスコンダクタ：in、out、vp、vn。|
|`$DIFFERENTIAL_TRANSCONDUCTOR`|`inp` `inn` `out`|差動入力と、key が label のテキスト（既定は g_m）を持つトランスコンダクタ：inp、inn、out。|
|`$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY`|`inp` `inn` `out` `vp` `vn`|電源ピンと、key が label のテキスト（既定は g_m）を持つ差動入力トランスコンダクタ：inp、inn、out、vp、vn。|
|`$DIFFERENTIAL_TRANSCONDUCTOR_INVERTED_SUPPLY`|`inp` `inn` `out` `vp` `vn`|電源ピンと、key が label のテキスト（既定は g_m）を持つ差動入力トランスコンダクタで、入力を入れ替え（innが上、inpが下）、電源は保つ（vpが上、vnが下；`$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY` をMX反転しただけではそうならない）：inp、inn、out、vp、vn。|
|`$ADC`|`in` `out`|アナログ-デジタル変換器（ADC）：in、out。|
|`$ADC_SUPPLY`|`in` `out` `vp` `vn`|電源ピンを持つアナログ-デジタル変換器（ADC）：in、out、vp、vn。|
|`$DAC`|`in` `out`|デジタル-アナログ変換器（DAC）：in、out。|
|`$DAC_SUPPLY`|`in` `out` `vp` `vn`|電源ピンを持つデジタル-アナログ変換器（DAC）：in、out、vp、vn。|
|`$CHOPPER_SYMBOL`|`inp` `inn` `outp` `outn`|チョッパ変調器ブロック（スイッチブリッジ構造 `$CHOPPER` とは別）：inp、inn、outp、outn。|
|**rnl-std-rf**|||
|`$CENTER_TAP_INDUCTOR`|`n1` `n2` `tap`|センタータップ付きインダクタ。|
|`$T_COIL`|`n1` `n2` `tap`|Tコイル：センタータップと橋絡キャパシタを備えた2つの結合コイル。|
|**rnl-std-logic**|||
|`$AND_2INPUT`|`a` `b` `y`|2入力ANDゲート：a、b、y。|
|`$AND_3INPUT`|`a` `b` `c` `y`|3入力ANDゲート：a、b、c、y。|
|`$NAND_2INPUT`|`a` `b` `y`|2入力NANDゲート：a、b、y。|
|`$NAND_3INPUT`|`a` `b` `c` `y`|3入力NANDゲート：a、b、c、y。|
|`$OR_2INPUT`|`a` `b` `y`|2入力ORゲート：a、b、y。|
|`$OR_3INPUT`|`a` `b` `c` `y`|3入力ORゲート：a、b、c、y。|
|`$NOR_2INPUT`|`a` `b` `y`|2入力NORゲート：a、b、y。|
|`$NOR_3INPUT`|`a` `b` `c` `y`|3入力NORゲート：a、b、c、y。|
|`$XOR_2INPUT`|`a` `b` `y`|2入力XORゲート：a、b、y。|
|`$XOR_3INPUT`|`a` `b` `c` `y`|3入力XORゲート：a、b、c、y。|
|`$XNOR_2INPUT`|`a` `b` `y`|2入力XNORゲート：a、b、y。|
|`$XNOR_3INPUT`|`a` `b` `c` `y`|3入力XNORゲート：a、b、c、y。|
|`$AND_4INPUT`|`a` `b` `c` `d` `y`|整数グリッド点上の4入力ANDゲート：a、b、c、d、y。|
|`$NAND_4INPUT`|`a` `b` `c` `d` `y`|整数グリッド点上の4入力NANDゲート：a、b、c、d、y。|
|`$OR_4INPUT`|`a` `b` `c` `d` `y`|整数グリッド点上の4入力ORゲート：a、b、c、d、y。|
|`$NOR_4INPUT`|`a` `b` `c` `d` `y`|整数グリッド点上の4入力NORゲート：a、b、c、d、y。|
|`$XOR_4INPUT`|`a` `b` `c` `d` `y`|整数グリッド点上の4入力XORゲート：a、b、c、d、y。|
|`$XNOR_4INPUT`|`a` `b` `c` `d` `y`|整数グリッド点上の4入力XNORゲート：a、b、c、d、y。|
|`$BUFFER`|`a` `y`|バッファ：a、y。|
|`$INVERTER`|`a` `y`|インバータ（NOTゲート）：a、y。|
|`$DELAY_CELL`|`a` `y`|遅延セル：a、y。|
|`$D_FLIP_FLOP`|`d` `ck` `q` `qb`|出力Qとその反転出力を持つDフリップフロップ：d、ck、q、qb。|
|`$D_FLIP_FLOP_Q_ONLY`|`d` `ck` `q`|出力Qのみを持つDフリップフロップ：d、ck、q。|
|`$D_FLIP_FLOP_WITH_RESET`|`d` `ck` `rst` `q` `qb`|リセット付きDフリップフロップ：d、ck、rst、q、qb。|
|`$AND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|電源ピン付き2入力ANDゲート：a、b、y、vp、vn。|
|`$AND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|電源ピン付き3入力ANDゲート：a、b、c、y、vp、vn。|
|`$AND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|電源ピン付き4入力ANDゲート：a、b、c、d、y、vp、vn。|
|`$NAND_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|電源ピン付き2入力NANDゲート：a、b、y、vp、vn。|
|`$NAND_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|電源ピン付き3入力NANDゲート：a、b、c、y、vp、vn。|
|`$NAND_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|電源ピン付き4入力NANDゲート：a、b、c、d、y、vp、vn。|
|`$OR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|電源ピン付き2入力ORゲート：a、b、y、vp、vn。|
|`$OR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|電源ピン付き3入力ORゲート：a、b、c、y、vp、vn。|
|`$OR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|電源ピン付き4入力ORゲート：a、b、c、d、y、vp、vn。|
|`$NOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|電源ピン付き2入力NORゲート：a、b、y、vp、vn。|
|`$NOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|電源ピン付き3入力NORゲート：a、b、c、y、vp、vn。|
|`$NOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|電源ピン付き4入力NORゲート：a、b、c、d、y、vp、vn。|
|`$XOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|電源ピン付き2入力XORゲート：a、b、y、vp、vn。|
|`$XOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|電源ピン付き3入力XORゲート：a、b、c、y、vp、vn。|
|`$XOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|電源ピン付き4入力XORゲート：a、b、c、d、y、vp、vn。|
|`$XNOR_2INPUT_SUPPLY`|`a` `b` `y` `vp` `vn`|電源ピン付き2入力XNORゲート：a、b、y、vp、vn。|
|`$XNOR_3INPUT_SUPPLY`|`a` `b` `c` `y` `vp` `vn`|電源ピン付き3入力XNORゲート：a、b、c、y、vp、vn。|
|`$XNOR_4INPUT_SUPPLY`|`a` `b` `c` `d` `y` `vp` `vn`|電源ピン付き4入力XNORゲート：a、b、c、d、y、vp、vn。|
|`$BUFFER_SUPPLY`|`a` `y` `vp` `vn`|電源ピン付きバッファ：a、y、vp、vn。|
|`$INVERTER_SUPPLY`|`a` `y` `vp` `vn`|電源ピン付きインバータ：a、y、vp、vn。|
|`$DELAY_CELL_SUPPLY`|`a` `y` `vp` `vn`|電源ピン付き遅延セル：a、y、vp、vn。|
|`$D_FLIP_FLOP_SUPPLY`|`d` `ck` `q` `qb` `vp` `vn`|電源ピン付きDフリップフロップ：d、ck、q、qb、vp、vn。|
|`$D_FLIP_FLOP_Q_ONLY_SUPPLY`|`d` `ck` `q` `vp` `vn`|出力Qのみで電源ピン付きのDフリップフロップ：d、ck、q、vp、vn。|
|`$D_FLIP_FLOP_WITH_RESET_SUPPLY`|`d` `ck` `rst` `q` `qb` `vp` `vn`|リセットと電源ピン付きのDフリップフロップ（vpとvnはx = 0で整列）：d、ck、rst、q、qb、vp、vn。|
|**rnl-std-signal-flow**|||
|`$ADDER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|`$ADDER` のシンボル：入力引き出し線a（in1）とb（in2）に矢じりを持ち、出力y（out）を持つ加算円。|
|`$SUBTRACTOR_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|`$SUBTRACTOR` のシンボル：中央に +、負側入力b（in2）の外側に - を記し、入力a（in1）とb（in2）、出力y（out）を持つ減算円。|
|`$MULTIPLIER_SYMBOL`|`a` (`in1`) `b` (`in2`) `y` (`out`)|`$MULTIPLIER` のシンボル：入力引き出し線a（in1）とb（in2）に矢じりを持ち、出力y（out）を持つ乗算円。|
|`$INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|`$INTEGRATOR` のシンボル：1/sと記され、入力a（in）と出力y（out）を持つ箱。|
|`$DISCRETE_TIME_INTEGRATOR_SYMBOL`|`a` (`in`) `y` (`out`)|`$DISCRETE_TIME_INTEGRATOR` のシンボル：z領域の伝達関数が記され、入力a（in）と出力y（out）を持つ箱。|
|`$QUANTIZER_SYMBOL`|`a` (`in`) `y` (`out`)|`$QUANTIZER` のシンボル：階段状の量子化特性を描き、入力a（in）と出力y（out）を持つ箱。|
|`$UNIT_DELAY_SYMBOL`|`a` (`in`) `y` (`out`)|`$UNIT_DELAY` のシンボル：zのマイナス1乗と記され、入力a（in）と出力y（out）を持つ箱。|
|**rnl-std-45deg**|||
|`$RESISTOR_45`|`p` `n`|45度抵抗。|
|`$CAPACITOR_45`|`p` `n`|45度キャパシタ。|
|`$INDUCTOR_45`|`p` `n`|45度インダクタ（斜めの変圧器やバランの巻線にも用いる）。|
|`$SWITCH_45`|`p` `n`|45度開スイッチ。|
|`$VOLTAGE_SOURCE_45`|`p` `n`|45度独立電圧源。p側の + とn側の - は正立に保たれる。|
|`$CURRENT_SOURCE_45`|`p` `n`|45度独立電流源。矢印はpからnへ向かう。|
|`$DIODE_45`|`a` `k`|45度ダイオード。|
|`$ZENER_DIODE_45`|`a` `k`|45度ツェナーダイオード。|
|`$SCHOTTKY_DIODE_45`|`a` `k`|45度ショットキーダイオード。|
|`$LED_45`|`a` `k`|45度発光ダイオード（LED）。|
|`$NMOS_45`|`d` `g` `s`|45度3ピンNMOSトランジスタ。|
|`$NMOS_4TERM_45`|`d` `g` `s` `b`|45度4ピンNMOSトランジスタ。バルクは (0, 0)。|
|`$PMOS_45`|`d` `g` `s`|45度3ピンPMOSトランジスタ。|
|`$PMOS_4TERM_45`|`d` `g` `s` `b`|45度4ピンPMOSトランジスタ。バルクは (0, 0)。|
|`$NPN_45`|`c` `b` `e`|45度NPNバイポーラトランジスタ。|
|`$PNP_45`|`c` `b` `e`|45度PNPバイポーラトランジスタ。|
|`$ARROW_45_SYMBOL`|—|-45度（右下向き、R90における45度シンボルのp->n / a->kに沿う）を指す塗りつぶし矢じり。R0/R90/R180/R270により4つの対角方向すべてに向く。|

### セットとスタイル

|名前|パッケージ|意味|
|---|---|---|
|`$GROUND_NET_SET`|rnl-std-basic|グラウンドネットのセット。既定では0が含まれ、ファイルからはADD-TOとREMOVE-FROMで編集する。rnl-std-fillはその未配線ピンにグラウンドスタブを付与し、自動配線から除外する。グラウンドの描画方法自体はこれに依存しない。|
|`$SUPPLY_NET_SET`|rnl-std-basic|電源ネットのセット。既定では空であり、ファイルからはADD-TOとREMOVE-FROMで編集する。|
|`$BIAS_NET_SET`|rnl-std-basic|バイアスネットのセット。既定では空であり、ファイルからはADD-TOとREMOVE-FROMで編集する。|
|`$PORT_FILL_SET`|rnl-std-basic|rnl-std-fillがポートタグを付与するポートネットのセット：すべてのレベルのすべてのポートネット（電源ネットとグラウンドネットも含む）から、`$IO_PORT` が書かれたネットおよび `$PORT_LIST` を持つレベルのネットを除いたもの。|
|`$GROUND_FILL_SET`|rnl-std-fill|グラウンドネット上の未配線ピン。自動補完がグラウンドスタブを付与する。|
|`$SUPPLY_FILL_SET`|rnl-std-fill|電源ネット上の未配線ピン。自動補完が電源スタブを付与する。|
|`$BIAS_FILL_SET`|rnl-std-fill|バイアスネット上の未配線ピン。自動補完がバイアススタブを付与する。|
|`$LABEL_FILL_SET`|rnl-std-fill|ネット上の残りの未配線ピン。自動補完がネットラベルスタブを付与する（バスピンには1つ付与される）。|
|`$base`|rnl-std-style|すべてのオブジェクトの基底スタイル：Analog Canvasが描画するときの線、テキスト、塗りつぶしを定める。|
|`$bus`|rnl-std-style|バスのスタイル：線が太線になる。|
|`$signal`|rnl-std-style|信号線用のスタイル：青色の太線。|
|`$note`|rnl-std-style|メモ用のスタイル：灰色で小さめの文字。|
|`$grey`|rnl-std-style|線とテキスト用の灰色スタイル。|

<!-- lists:end -->
