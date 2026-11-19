<!-- translated from en/annex-a.md sha256:2adc09eec4f4 -->
## 付録 A キーワードシグネチャ（規範的）

本付録は標準語リスト `rnl-words-0.4.json` から生成される（5.3）。両者が異なる場合は正誤表による訂正事項とする。ここでは各キーワードがとるものと返すものを列挙し、その意味は最終列に挙げた節で説明する。

- **スロット**は標準の順序で、スロット名（ロールとして記述できる；5.3）および受け取るものとともに列挙される。*one*：単一のオブジェクト。セットが渡された場合は各メンバーに適用される（5.2.1）が、PRIORITYのみはGROUPが付けられたセットをグループ自体として扱う（14.8）。*set*：セット。スロットに対してバラバラに書かれたオブジェクトは1つのセットをなす（5.2.1）。*any*：1つのオブジェクトまたはセットで、キーワードの定めに従って使用される（DISTANCEではセットはその各メンバーを表す；14.5）。*shape*：1つの点、または点のセット（図形の2つの形式；9.2.5）。*body*：波括弧内の文で、封入される（5.4）。*definition*：定義（`$` 名、定義に束縛された `%` 名、または無名定義；5.6、6.6）。吸収された入力は厳格であり、封入された入力は許容的である（5.4、7.3）。
- スロットの**種別**は種別名（`instance`、`terminal`、`pin`、`net`、`point`、`wire`、`usage`、`bundle`、`text`、`annotation`、`label`）または次のいずれかである：*any*：任意のオブジェクト；*level*：単独の `#`、すなわちレベル自身；*owner*：単独の `?`、シンボル定義本体内ではそのシンボルで描画されるオブジェクト、一般定義の定義本体内では展開中の適用オブジェクト（9.4）；*position*：オブジェクトのサブオブジェクト `::position`（その原点）；*locatable*：位置を持つオブジェクト（インスタンス、バンドル、ピン、ピンを持つ端子、点、テキスト、中心で書かれた図形の中心、自身の点による適用オブジェクト）；これに渡されたセットはその各メンバーを表す（14.1）；*orientable*：向きを持つオブジェクト（インスタンス、バンドル、シンボルを持つ適用オブジェクト）；*constraint*：制約キーワードが作るオブジェクト；*set*：名前として書かれたセット（ASSERT-TOPOLOGYでは直接書かれた `{…}` である項目も可；15.4）；*empty node*：単独の `&`、ネットなし。種別 `net` のロールおよびWIREのnetスロットがとり、それ以外ではネットの空セットとなる（6.3）。
- *position 1*、*position 2* と記された**パラメータ**はその順に位置指定で与えることができ、それ以外はすべて名前指定でのみ与えられる（4.2）。`priority` は常に `"disabled"`、`"low"`、`"normal"`、または `"high"` をとる。これがなければレコードヘッダーの優先度が適用されるが、ASとATTACHのみはレコードヘッダーから `"disabled"` のみを引き継ぐ（14.8）。
- **結果**：文が返すオブジェクト（5.5）およびその事実がどのように上書きするか（5.8）。上書きがない場合、事実は累積する。
- 特記がない限り、キーワードはレベルの任意のレコード内に書くことができる。*シンボル定義本体可* とはDEFINE-SYMBOLの定義本体内にも書けることを意味する。

### A.1 レコード

`@RNL(…) {…}` はその文をレベルへコミットする（5.4）。レコードヘッダーのフィールド（2.8）はすべて名前指定で与えられる：

| フィールド | 値 | デフォルト | 解釈への影響 |
|---|---|---|---|
| `type` | `"authored"`, `"generated"`, `"header"` | `"authored"` | あり |
| `scope` | 文字列 | — | あり |
| `priority` | `"disabled"`, `"low"`, `"normal"`, `"high"` | `"normal"` | あり |
| `author` | 文字列 | — | なし |
| `hash` | 文字列 | — | なし |
| `numbering` | 文字列 | — | あり |
| `modified` | 文字列 | — | なし |
| `language` | `"spice"`, `"spectre"` | — | あり |

### A.2 ヘッダーレコードとパッケージ

| キーワード | スロット | パラメータ | 結果 | 節 |
|---|---|---|---|---|
| `@VERSION`（ヘッダーレコードのみ） | — | `version`（position 1；必須） | — | 16.1 |
| `@PACKAGE`（パッケージファイルのヘッダーレコードのみ） | — | `name`（必須）、`version`（必須） | — | 2.7 |
| `@USE-PACKAGE`（ヘッダーレコード内も可） | `body`：body、任意 | `name`（必須）、`version`、`path` | — | 16.2 |
| `@ALIAS`（ヘッダーレコードのみ） | — | `alias`（position 1；必須）、`original`（position 2；必須）、`definition` | — | 16.5 |
| `@USE-ALIASES`（ヘッダーレコードのみ） | — | `name`（必須）、`version`、`path` | — | 16.5 |
| `@DEVICE`（ヘッダーレコードのみ） | — | `name`、`model`、`terminals`（配列）、`type`（文字列）；`name` と `model` の少なくとも一方；`terminals` と `type` の少なくとも一方 | — | 6.12 |

### A.3 定義

| キーワード | スロット | パラメータ | 結果 | 節 |
|---|---|---|---|---|
| `@DEFINE-SYMBOL`（シンボル定義本体可） | `body`：body | `box`（配列） | 定義（シンボル） | 9.2 |
| `@DEFINE-STYLE` | `extends`：body（スタイル） | スタイルプロパティ（9.3） | 定義（スタイル） | 9.3 |
| `@DEFINE` | `body`：body | `roles`（必須；配列）、`parameters`（配列） | 定義（一般） | 9.4 |

### A.4 シンボル内容と図形

| キーワード | スロット | パラメータ | 結果 | 節 |
|---|---|---|---|---|
| `@PIN`（シンボル定義本体のみ） | `position`：one（点） | `terminal`（端子、または空でない端子の配列；省略可）、`label`、`direction`（`"right"`、`"up"`、`"left"`、`"down"`） | 新しいピン | 9.2.4 |
| `@LINE`（シンボル定義本体可） | `points`：set（点、ピン、端子） | `arrow`（`"none"`、`"start"`、`"end"`、`"both"`；デフォルト `"none"`）、`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；デフォルト `"none"`）、線スタイルプロパティ | シンボルの図形；シーン内では新しい注釈（13.4） | 9.2.5 |
| `@RECTANGLE`（シンボル定義本体可） | `points`：shape（点、ピン、端子） | `width`（数値）、`height`（数値）、`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；デフォルト `"none"`）、領域スタイルプロパティ；`width` と `height` は両方書くか両方省略 | シンボルの図形；シーン内では新しい注釈（13.4） | 9.2.5 |
| `@CIRCLE`（シンボル定義本体可） | `points`：shape（点、ピン、端子） | `radius`（数値）、`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；デフォルト `"none"`）、領域スタイルプロパティ | シンボルの図形；シーン内では新しい注釈（13.4） | 9.2.5 |
| `@ELLIPSE`（シンボル定義本体可） | `points`：shape（点、ピン、端子） | `width`（数値）、`height`（数値）、`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；デフォルト `"none"`）、領域スタイルプロパティ；`width` と `height` は両方書くか両方省略 | シンボルの図形；シーン内では新しい注釈（13.4） | 9.2.5 |
| `@ARC`（シンボル定義本体可） | `points`：set（点、ピン、端子） | `arrow`（`"none"`、`"start"`、`"end"`、`"both"`；デフォルト `"none"`）、`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；デフォルト `"none"`）、線スタイルプロパティ | シンボルの図形；シーン内では新しい注釈（13.4） | 9.2.5 |
| `@BEZIER`（シンボル定義本体可） | `points`：set（点、ピン、端子） | `arrow`（`"none"`、`"start"`、`"end"`、`"both"`；デフォルト `"none"`）、`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；デフォルト `"none"`）、線スタイルプロパティ | シンボルの図形；シーン内では新しい注釈（13.4） | 9.2.5 |
| `@ANNOTATION` | `at`：one（locatable）；`source`：one（シンボル、一般定義、または適用オブジェクト）、最後に記述 | `rotate`（数値、反時計回りの度；デフォルト `0`）、`mirror`（真偽値；デフォルト `false`）、スタイルプロパティ、および `source` として与えられたシンボルまたは一般定義のパラメータ（名前指定；9.4） | 新しい注釈 | 13.5 |
| `@TEXT`（シンボル定義本体可） | `attach`：one（点、インスタンス、適用オブジェクト、バンドル、ピン、端子、配線、注釈、ネット、`?`） | `text`（position 1；必須）、`key`、`dx`（数値）、`dy`（数値）、`halign`（`"start"`、`"middle"`、`"end"`；デフォルト `"start"`）、`valign`（`"baseline"`、`"top"`、`"middle"`、`"bottom"`；デフォルト `"baseline"`）、`rotate`（数値、反時計回りの度；デフォルト `0`）、`fix_rotate`（真偽値；デフォルト `false`）、`upright`（真偽値；デフォルト `true`）、テキストスタイルプロパティ | 新しいテキスト；`key` 付きの場合は取り付け先オブジェクトのサブオブジェクト；keyによる上書き（keyが与えられたとき、key：取り付け先オブジェクトとkey） | 13.3 |

### A.5 配置と外観

| キーワード | スロット | パラメータ | 結果 | 節 |
|---|---|---|---|---|
| `@PLACE`（シンボル定義本体可） | `target`：one（インスタンス、適用オブジェクト、バンドル、点、テキスト、`::position`） | `x`（position 1；数値）、`y`（position 2；数値）、`priority`；`x` と `y` の少なくとも一方 | 入力オブジェクト；幾何的事実（key：オブジェクトと軸） | 11.1 |
| `@ORIENT`（シンボル定義本体可） | `target`：one（インスタンス、適用オブジェクト、バンドル、ピン） | `code`（position 1；必須；`"R0"`、`"R90"`、`"R180"`、`"R270"`、`"MX"`、`"MY"`、`"MXR90"`、`"MYR90"`、または最初の4つを表す語 `"left"`、`"down"`、`"right"`、`"up"`；あるいはそれらの空でない配列）、`priority` | 入力オブジェクト；幾何的事実（key：オブジェクト、またはピン） | 11.2 |
| `@LAYER` | `target`：one（インスタンス、適用オブジェクト、バンドル、配線、注釈、テキスト） | `layer`（position 1；必須；`"bottom"`、`"middle"`、`"top"`） | 入力オブジェクト；keyによる上書き（key：オブジェクト） | 11.6 |
| `@SET-SYMBOL`（シンボル定義本体可） | `target`：one（インスタンス、適用オブジェクト、バンドル、`#`）；`symbol`：definition（シンボル） | `terminals`（端子名の配列、またはペアの配列） | 入力オブジェクト；外観キュー | 11.3 |
| `@SET-STYLE` | `target`：one（インスタンス、適用オブジェクト、バンドル、配線、注釈、テキスト）；`style`：definition（スタイル） | — | 入力オブジェクト；外観キュー | 11.3 |

### A.6 選択

| キーワード | スロット | パラメータ | 結果 | 節 |
|---|---|---|---|---|
| `@SELECT-INSTANCE` | `scope`：any（`#`、任意のオブジェクト） | `name`、`model`、`type`、`terminals`（数値、整数）、`symbol`（文字列：`$` 名、`*` 可）、`drawn`（真偽値）、`role`、`recursive`（真偽値；デフォルト `false`） | 新しいインスタンスのセット | 10.1 |
| `@SELECT-TERMINAL` | `scope`：any（`#`、任意のオブジェクト） | `name`、`model`、`type`、`terminal`（端子）、`net`、`current`（`"in"`、`"out"`、`"unspecified"`）、`role`、`recursive`（真偽値；デフォルト `false`） | 新しい端子のセット | 10.1 |
| `@SELECT-PIN` | `scope`：any（`#`、任意のオブジェクト） | `name`、`model`、`type`、`terminal`（端子）、`net`、`wired`（真偽値）、`role`、`recursive`（真偽値；デフォルト `false`） | 新しいピンのセット | 10.1 |
| `@SELECT-NET` | `scope`：any（`#`、任意のオブジェクト） | `name`、`port`（真偽値）、`wired`（真偽値）、`global`（真偽値）、`role`、`recursive`（真偽値；デフォルト `false`） | 新しいネットのセット | 10.1 |
| `@SELECT-WIRE` | `scope`：any（`#`、任意のオブジェクト） | `net`、`bus`（真偽値）、`role`、`recursive`（真偽値；デフォルト `false`） | 新しい配線のセット | 10.1 |
| `@SELECT-LABEL` | `scope`：any（`#`、任意のオブジェクト） | `key`、`role`、`recursive`（真偽値；デフォルト `false`） | 新しいラベルのセット | 10.1 |
| `@SELECT-ANNOTATION` | `scope`：any（`#`、任意のオブジェクト） | `role`、`recursive`（真偽値；デフォルト `false`） | 新しい注釈のセット | 10.1 |
| `@SELECT-USAGE` | `scope`：any（`#`、任意のオブジェクト）；`definition`：definition（一般定義）、任意 | `symbol`（文字列：`$` 名、`*` 可）、`drawn`（真偽値）、`recursive`（真偽値；デフォルト `false`） | 新しい適用オブジェクトのセット | 10.1 |
| `@SELECT-ANCESTOR-USAGE` | `scope`：any（任意のオブジェクト）；`definition`：definition（一般定義） | `recursive`（真偽値；デフォルト `false`） | 新しい適用オブジェクトのセット | 10.1 |

### A.7 定義の適用

| キーワード | スロット | パラメータ | 結果 | 節 |
|---|---|---|---|---|
| `@AS`（シンボル定義本体可） | 定義のロール群、次に定義（`definition`、最後に記述） | 定義のパラメータ（パラメータリストの順に位置指定、または名前指定；9.4）、`priority` | 新しい適用オブジェクト | 9.4 |
| `@ATTACH`（シンボル定義本体可） | 定義、次にそのロール群（`definition`、最初に記述） | 定義のパラメータ（パラメータリストの順に位置指定、または名前指定；9.4）、`priority` | 新しい適用オブジェクト | 9.4 |

### A.8 配線

| キーワード | スロット | パラメータ | 結果 | 節 |
|---|---|---|---|---|
| `@WIRE` | `net`：any（ネット、または空ノード）；`points`：set（点、ピン；端子はそのピンを表す、5.2.4） | `mode`（`"auto"`、`"straight"`、`"horizontal-vertical"`、`"vertical-horizontal"`、`"manhattan"`、`"manhattan45"`；デフォルト `"auto"`）、配線スタイルプロパティ | 新しい配線；配線を与えた場合はそれを書き換え、keyにより上書き（key：その配線） | 13.1 |
| `@ROUTE-NET` | `net`：one（ネット） | `mode`（`"auto"`、`"straight"`、`"horizontal-vertical"`、`"vertical-horizontal"`、`"manhattan"`、`"manhattan45"`；デフォルト `"auto"`）、配線スタイルプロパティ、`priority` | 新しい経路 | 13.7 |
| `@ROUTE-OBJECT` | `objects`：set（任意のオブジェクト） | `mode`（ROUTE-NETと同じ）、配線スタイルプロパティ、`priority` | 入力オブジェクト | 13.7 |
| `@UNROUTE-NET` | `net`：one（ネット） | `priority` | 入力オブジェクト | 13.7 |
| `@UNROUTE-OBJECT` | `objects`：set（任意のオブジェクト） | `priority` | 入力オブジェクト | 13.7 |

### A.9 セット、グループおよびバンドル

| キーワード | スロット | パラメータ | 結果 | 節 |
|---|---|---|---|---|
| `@BUNDLE` | `members`：set（インスタンス） | — | 新しいバンドル | 11.8 |
| `@GROUP` | `members`：set（任意のオブジェクト） | `spacing_x`（数値、整数）、`spacing_y`（数値、整数） | 入力オブジェクト | 11.5 |
| `@ADD-TO` | `items`：any（任意のオブジェクト）；`target`：any（セット） | — | 対象のセット | 5.2.2 |
| `@REMOVE-FROM` | `items`：any（任意のオブジェクト）；`target`：any（セット） | — | 対象のセット | 5.2.2 |
| `@UNION` | `sets`：set（任意のオブジェクト） | — | 新しいセット | 5.2.2 |

### A.10 制約と優先度

| キーワード | スロット | パラメータ | 結果 | 節 |
|---|---|---|---|---|
| `@ALIGN` | `members`：set（locatable） | `axis`（position 1；必須；`"x"`、`"y"`）、`priority` | 新しい制約；幾何的事実 | 14.2 |
| `@ORDER` | `members`：set（locatable） | `direction`（position 1；必須；`"left-to-right"`、`"right-to-left"`、`"bottom-to-top"`、`"top-to-bottom"`）、`priority` | 新しい制約；幾何的事実 | 14.3 |
| `@DISTRIBUTE` | `members`：set（locatable） | `direction`（position 1；必須；`"left-to-right"`、`"right-to-left"`、`"bottom-to-top"`、`"top-to-bottom"`）、`priority` | 新しい制約；幾何的事実 | 14.4 |
| `@DISTANCE` | `from`：any（locatable）；`to`：any（locatable） | `direction`（position 1；必須；`"left-to-right"`、`"right-to-left"`、`"bottom-to-top"`、`"top-to-bottom"`）、`min`（数値）、`max`（数値）、`exact`（数値）、`priority`；`min`、`max`、`exact` の少なくとも1つ；`exact` は `min` と併用不可；`exact` は `max` と併用不可 | 新しい制約；幾何的事実 | 14.5 |
| `@ORIENT-RELATION` | `from`：one（orientable、またはピン）；`to`：one（orientable、またはピン、`from` と同じ） | `relation`（position 1；必須；ORIENTの8つのコード（語は不可）、またはそれらの空でない配列）、`priority` | 新しい制約；幾何的事実 | 14.9 |
| `@TABLE` | `rows`：set（locatable；入れ子のセットが表のリスト） | `transpose`（真偽値；デフォルト `false`）、`priority` | 新しい制約；幾何的事実 | 14.11 |
| `@SOLVER-CONFIG`（`language` なしのヘッダーレコード、パッケージファイルのヘッダーレコード、およびレベル） | — | ソルバーの名前付きパラメータ：数値、文字列、真偽値およびこれらの配列；`spacing_x`、`spacing_y`（数値、整数）および `junction`（文字列：`$` 名、レベルレコード内ではそのレベルの `%` 名も可、または空）が本標準により名指しされる；その他のkeyはソルバー独自のもの | オブジェクトなし；ソルバーの設定 | 14.10 |
| `@PRIORITY` | `target`：one（制約、適用オブジェクト、セット） | `priority`（position 1；必須） | 入力オブジェクト；keyによる上書き（key：オブジェクト） | 14.8 |

### A.11 電気的意味

| キーワード | スロット | パラメータ | 結果 | 節 |
|---|---|---|---|---|
| `@CURRENT` | `terminal`：one（端子） | `direction`（position 1；必須；`"in"`、`"out"`、`"unspecified"`） | 入力オブジェクト；keyによる上書き（key：端子） | 15.2 |
| `@ASSERT-INSTANCE` | `members`：set（インスタンス） | `role` と `recursive` を除くSELECT-INSTANCEの条件、`has`（属性名の配列）、`same`（属性名の配列）、`different`（属性名の配列）、`message` | オブジェクトなし；検査 | 15.3 |
| `@ASSERT-TERMINAL` | `members`：set（端子） | `role` と `recursive` を除くSELECT-TERMINALの条件、`same`、`different`、`message` | オブジェクトなし；検査 | 15.3 |
| `@ASSERT-NET` | `members`：set（ネット） | `role` と `recursive` を除くSELECT-NETの条件、`same`、`different`、`message` | オブジェクトなし；検査 | 15.3 |
| `@ASSERT-USAGE` | `members`：set（適用オブジェクト）；`definition`：definition（一般定義）、任意 | `symbol`、`drawn`（真偽値）、`same`、`different`（配列；`"definition"` のみ）、`message` | オブジェクトなし；検査 | 15.3 |
| `@ASSERT-IN` | `members`：set（任意のオブジェクト）；`set`：any（セット） | `message` | オブジェクトなし；検査 | 15.3 |
| `@ASSERT-TOPOLOGY` | `members`：set（インスタンス、バンドル、適用オブジェクト、セット） | `topology`（position 1；必須；`"series"`、`"parallel"`、`"star"`、`"loop"`）、`exclude`（型と端子のペアの配列）、`message` | オブジェクトなし；検査 | 15.4 |

### A.12 その他の標準語

- 種別（kind）：`instance`、`terminal`、`pin`、`net`、`point`、`wire`、`usage`、`bundle`、`text`、`annotation`、`label`（5.3、9.4）。
- `::` の後のサブオブジェクト：`position`、`net`（6.5）。
- `${…}` で読み取られる属性：`name`、`value`、`model`、`type`、`net`、`x`、`y`（13.6）。
- 真偽値：`true`、`false`（3.7）。

スタイルプロパティ（9.3）：

| プロパティ | 型と値 | デフォルト |
|---|---|---|
| `linecolor` | 色 | — |
| `linestyle` | `"solid"`, `"dashed"`, `"dotted"`, `"dash-dot"`, `"double"`, `"bold"`, `"none"` | `"solid"` |
| `linewidth` | 数値 | `0` |
| `cap` | `"butt"`, `"round"`, `"square"` | — |
| `join` | `"miter"`, `"round"`, `"bevel"` | — |
| `fillcolor` | 色 | — |
| `textcolor` | 色 | — |
| `size` | 数値 | — |
| `font` | 文字列 | — |
| `bold` | 真偽値 | — |
| `italic` | 真偽値 | — |
| `arrowsize` | 数値 | — |
| `textvisible` | 真偽値 | `true` |

キーワードがパラメータとしてとるスタイルプロパティ（9.2.5）：*line*：`linecolor`、`linestyle`、`linewidth`、`cap`、`join`、`fillcolor`、`arrowsize`；*area*：`linecolor`、`linestyle`、`linewidth`、`cap`、`join`、`fillcolor`；*wire*：`linecolor`、`linestyle`、`linewidth`、`cap`、`join`；*text*：`textcolor`、`size`、`font`、`bold`、`italic`、`textvisible`。
