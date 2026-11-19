<!-- translated from en/annex-a.md sha256:2adc09eec4f4 -->
## 附錄 A 關鍵字簽名（規範性）

本附錄由詞彙表 `rnl-words-0.4.json` 產生（5.3）；兩者若有差異，則為勘誤。表中列出每個關鍵字接收與給出的內容；其含義見最後一欄所標示的章節。

- **槽位**依標準順序列出，附帶槽位名稱（可標記為角色，5.3）與接收形式：*單一（one）*：單一物件；傳入集合時，將該關鍵字套用於每個成員（5.2.1），唯獨 PRIORITY 將標記了 GROUP 的集合作為該群組本身處理（14.8）；*集合（set）*：一個集合；寫入該槽位的散落物件構成單一集合（5.2.1）；*任意（any）*：單一物件或集合，依關鍵字規定使用（對 DISTANCE 而言，集合代表其每個成員，14.5）；*形狀（shape）*：單一點，或一組點的集合（圖形的兩種形式，9.2.5）；*定義本體（body）*：大括號內的敘述，採封裝消耗（5.4）；*定義（definition）*：一個定義——`$` 名稱、綁定至定義的 `%` 名稱，或匿名定義（5.6、6.6）。吸收型輸入採嚴格處理；封裝型輸入採容錯處理（5.4、7.3）。
- **槽位種類**為種類名稱（`instance`、`terminal`、`pin`、`net`、`point`、`wire`、`usage`、`bundle`、`text`、`annotation`、`label`）或：*任意（any）*，任何物件；*層級（level）*，單獨的 `#`：即該層級本身；*所屬物件（owner）*，單獨的 `?`：在符號定義本體中為使用該符號繪製的物件；在一般定義的定義本體中為正在展開的用例（9.4）；*位置（position）*，物件的子物件 `::position`（其原點）；*可定位（locatable）*，具有位置的物件：實例、疊畫、接腳、帶有接腳的端子、點、文字、以中心形式寫出的圖形之中心、用例（依其自身的點）；傳入集合時代表其每個成員（14.1）；*可定朝向（orientable）*，具有朝向的物件：實例、疊畫、帶有符號的用例；*約束（constraint）*，約束關鍵字建立的物件；*集合（set）*，一個集合，寫成名稱；在 ASSERT-TOPOLOGY 中亦包含直接寫出的 `{…}` 項目（15.4）；*空節點（empty node）*，單獨的 `&`：無網路；由種類為 `net` 的角色與 WIRE 的網路槽位接收，在其他位置視同空網路集合（6.3）。
- 標記為*位置 1*、*位置 2* 的**參數**可依位置按該順序傳入；其餘參數僅能依名稱傳入（4.2）。`priority` 一律接收 `"disabled"`、`"low"`、`"normal"` 或 `"high"`；未寫出時套用記錄標頭的優先順序，唯獨 AS 與 ATTACH 僅從記錄標頭繼承 `"disabled"`（14.8）。
- **結果**：該敘述給出的物件（5.5）以及其事實如何覆寫（5.8）。無覆寫機制時，事實累積疊加。
- 除非另有註明，否則關鍵字可寫在層級的任何記錄中；*亦可用於符號定義本體*表示它也可寫在 DEFINE-SYMBOL 的定義本體中。

### A.1 記錄

`@RNL(…) {…}` 將其敘述提交至所在層級（5.4）。記錄標頭欄位（2.8），皆須依名稱給出：

| 欄位 | 取值 | 預設值 | 是否影響解譯 |
|---|---|---|---|
| `type` | `"authored"`、`"generated"`、`"header"` | `"authored"` | 是 |
| `scope` | 字串 | — | 是 |
| `priority` | `"disabled"`、`"low"`、`"normal"`、`"high"` | `"normal"` | 是 |
| `author` | 字串 | — | 否 |
| `hash` | 字串 | — | 否 |
| `numbering` | 字串 | — | 是 |
| `modified` | 字串 | — | 否 |
| `language` | `"spice"`、`"spectre"` | — | 是 |

### A.2 檔頭記錄與套件

| 關鍵字 | 槽位 | 參數 | 結果 | 章節 |
|---|---|---|---|---|
| `@VERSION`（僅限檔頭記錄） | — | `version`（位置 1；必填） | — | 16.1 |
| `@PACKAGE`（僅限套件檔案的檔頭記錄） | — | `name`（必填）、`version`（必填） | — | 2.7 |
| `@USE-PACKAGE`（亦可用於檔頭記錄） | `body`：定義本體，選填 | `name`（必填）、`version`、`path` | — | 16.2 |
| `@ALIAS`（僅限檔頭記錄） | — | `alias`（位置 1；必填）、`original`（位置 2；必填）、`definition` | — | 16.5 |
| `@USE-ALIASES`（僅限檔頭記錄） | — | `name`（必填）、`version`、`path` | — | 16.5 |
| `@DEVICE`（僅限檔頭記錄） | — | `name`、`model`、`terminals`（陣列）、`type`（字串）；`name` 與 `model` 至少其一；`terminals` 與 `type` 至少其一 | — | 6.12 |

### A.3 定義

| 關鍵字 | 槽位 | 參數 | 結果 | 章節 |
|---|---|---|---|---|
| `@DEFINE-SYMBOL`（亦可用於符號定義本體） | `body`：定義本體 | `box`（陣列） | 定義（符號） | 9.2 |
| `@DEFINE-STYLE` | `extends`：定義本體（樣式） | 樣式屬性（9.3） | 定義（樣式） | 9.3 |
| `@DEFINE` | `body`：定義本體 | `roles`（必填；陣列）、`parameters`（陣列） | 定義（一般） | 9.4 |

### A.4 符號內容與圖形

| 關鍵字 | 槽位 | 參數 | 結果 | 章節 |
|---|---|---|---|---|
| `@PIN`（僅限符號定義本體） | `position`：單一（點） | `terminal`（端子，或端子的非空陣列；可省略）、`label`、`direction`（`"right"`、`"up"`、`"left"`、`"down"`） | 新接腳 | 9.2.4 |
| `@LINE`（亦可用於符號定義本體） | `points`：集合（點、接腳、端子） | `arrow`（`"none"`、`"start"`、`"end"`、`"both"`；預設 `"none"`）、`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；預設 `"none"`）、線條樣式屬性 | 符號的圖形；在場景中為新標註（13.4） | 9.2.5 |
| `@RECTANGLE`（亦可用於符號定義本體） | `points`：形狀（點、接腳、端子） | `width`（數字）、`height`（數字）、`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；預設 `"none"`）、區域樣式屬性；`width` 與 `height` 須同時寫出或同時省略 | 符號的圖形；在場景中為新標註（13.4） | 9.2.5 |
| `@CIRCLE`（亦可用於符號定義本體） | `points`：形狀（點、接腳、端子） | `radius`（數字）、`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；預設 `"none"`）、區域樣式屬性 | 符號的圖形；在場景中為新標註（13.4） | 9.2.5 |
| `@ELLIPSE`（亦可用於符號定義本體） | `points`：形狀（點、接腳、端子） | `width`（數字）、`height`（數字）、`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；預設 `"none"`）、區域樣式屬性；`width` 與 `height` 須同時寫出或同時省略 | 符號的圖形；在場景中為新標註（13.4） | 9.2.5 |
| `@ARC`（亦可用於符號定義本體） | `points`：集合（點、接腳、端子） | `arrow`（`"none"`、`"start"`、`"end"`、`"both"`；預設 `"none"`）、`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；預設 `"none"`）、線條樣式屬性 | 符號的圖形；在場景中為新標註（13.4） | 9.2.5 |
| `@BEZIER`（亦可用於符號定義本體） | `points`：集合（點、接腳、端子） | `arrow`（`"none"`、`"start"`、`"end"`、`"both"`；預設 `"none"`）、`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；預設 `"none"`）、線條樣式屬性 | 符號的圖形；在場景中為新標註（13.4） | 9.2.5 |
| `@ANNOTATION` | `at`：單一（可定位）；`source`：單一（符號、一般定義或用例），寫在最後 | `rotate`（數字，逆時針度數；預設 `0`）、`mirror`（布林；預設 `false`）、樣式屬性，以及作為 `source` 傳入之符號或一般定義的參數，依名稱傳入（9.4） | 新標註 | 13.5 |
| `@TEXT`（亦可用於符號定義本體） | `attach`：單一（點、實例、用例、疊畫、接腳、端子、連線、標註、網路、`?`） | `text`（位置 1；必填）、`key`、`dx`（數字）、`dy`（數字）、`halign`（`"start"`、`"middle"`、`"end"`；預設 `"start"`）、`valign`（`"baseline"`、`"top"`、`"middle"`、`"bottom"`；預設 `"baseline"`）、`rotate`（數字，逆時針度數；預設 `0`）、`fix_rotate`（布林；預設 `false`）、`upright`（布林；預設 `true`）、文字樣式屬性 | 新文字；帶有 `key` 時為所附著物件的子物件；依 key 覆寫（給出 key 時，key 為附著物件與 key） | 13.3 |

### A.5 位置與外觀

| 關鍵字 | 槽位 | 參數 | 結果 | 章節 |
|---|---|---|---|---|
| `@PLACE`（亦可用於符號定義本體） | `target`：單一（實例、用例、疊畫、點、文字、`::position`） | `x`（位置 1；數字）、`y`（位置 2；數字）、`priority`；`x`、`y` 至少其一 | 輸入物件；幾何事實（key：物件與座標軸） | 11.1 |
| `@ORIENT`（亦可用於符號定義本體） | `target`：單一（實例、用例、疊畫、接腳） | `code`（位置 1；必填；`"R0"`、`"R90"`、`"R180"`、`"R270"`、`"MX"`、`"MY"`、`"MXR90"`、`"MYR90"`，或代表前四者的 `"left"`、`"down"`、`"right"`、`"up"`；或它們的非空陣列）、`priority` | 輸入物件；幾何事實（key：物件，或接腳） | 11.2 |
| `@LAYER` | `target`：單一（實例、用例、疊畫、連線、標註、文字） | `layer`（位置 1；必填；`"bottom"`、`"middle"`、`"top"`） | 輸入物件；依 key 覆寫（key：物件） | 11.6 |
| `@SET-SYMBOL`（亦可用於符號定義本體） | `target`：單一（實例、用例、疊畫、`#`）；`symbol`：定義（符號） | `terminals`（端子名稱陣列，或配對陣列） | 輸入物件；外觀佇列 | 11.3 |
| `@SET-STYLE` | `target`：單一（實例、用例、疊畫、連線、標註、文字）；`style`：定義（樣式） | — | 輸入物件；外觀佇列 | 11.3 |

### A.6 篩選

| 關鍵字 | 槽位 | 參數 | 結果 | 章節 |
|---|---|---|---|---|
| `@SELECT-INSTANCE` | `scope`：任意（`#`、任何物件） | `name`、`model`、`type`、`terminals`（數字，整數）、`symbol`（字串：`$` 名稱，可含 `*`）、`drawn`（布林）、`role`、`recursive`（布林；預設 `false`） | 新實例集合 | 10.1 |
| `@SELECT-TERMINAL` | `scope`：任意（`#`、任何物件） | `name`、`model`、`type`、`terminal`（端子）、`net`、`current`（`"in"`、`"out"`、`"unspecified"`）、`role`、`recursive`（布林；預設 `false`） | 新端子集合 | 10.1 |
| `@SELECT-PIN` | `scope`：任意（`#`、任何物件） | `name`、`model`、`type`、`terminal`（端子）、`net`、`wired`（布林）、`role`、`recursive`（布林；預設 `false`） | 新接腳集合 | 10.1 |
| `@SELECT-NET` | `scope`：任意（`#`、任何物件） | `name`、`port`（布林）、`wired`（布林）、`global`（布林）、`role`、`recursive`（布林；預設 `false`） | 新網路集合 | 10.1 |
| `@SELECT-WIRE` | `scope`：任意（`#`、任何物件） | `net`、`bus`（布林）、`role`、`recursive`（布林；預設 `false`） | 新連線集合 | 10.1 |
| `@SELECT-LABEL` | `scope`：任意（`#`、任何物件） | `key`、`role`、`recursive`（布林；預設 `false`） | 新標籤集合 | 10.1 |
| `@SELECT-ANNOTATION` | `scope`：任意（`#`、任何物件） | `role`、`recursive`（布林；預設 `false`） | 新標註集合 | 10.1 |
| `@SELECT-USAGE` | `scope`：任意（`#`、任何物件）；`definition`：定義（一般定義），選填 | `symbol`（字串：`$` 名稱，可含 `*`）、`drawn`（布林）、`recursive`（布林；預設 `false`） | 新用例集合 | 10.1 |
| `@SELECT-ANCESTOR-USAGE` | `scope`：任意（任何物件）；`definition`：定義（一般定義） | `recursive`（布林；預設 `false`） | 新用例集合 | 10.1 |

### A.7 套用定義

| 關鍵字 | 槽位 | 參數 | 結果 | 章節 |
|---|---|---|---|---|
| `@AS`（亦可用於符號定義本體） | 定義的各角色，其次為定義（`definition`，寫在最後） | 定義的參數（依參數列表順序按位置傳入，或依名稱；9.4）、`priority` | 新用例 | 9.4 |
| `@ATTACH`（亦可用於符號定義本體） | 定義，其次為其各角色（`definition`，寫在最前） | 定義的參數（依參數列表順序按位置傳入，或依名稱；9.4）、`priority` | 新用例 | 9.4 |

### A.8 連線

| 關鍵字 | 槽位 | 參數 | 結果 | 章節 |
|---|---|---|---|---|
| `@WIRE` | `net`：任意（網路，或空節點）；`points`：集合（點、接腳；端子代表其接腳，5.2.4） | `mode`（`"auto"`、`"straight"`、`"horizontal-vertical"`、`"vertical-horizontal"`、`"manhattan"`、`"manhattan45"`；預設 `"auto"`）、連線樣式屬性 | 新連線；若傳入一條連線，則改寫之，依 key 覆寫（key：該連線） | 13.1 |
| `@ROUTE-NET` | `net`：單一（網路） | `mode`（`"auto"`、`"straight"`、`"horizontal-vertical"`、`"vertical-horizontal"`、`"manhattan"`、`"manhattan45"`；預設 `"auto"`）、連線樣式屬性、`priority` | 新走線 | 13.7 |
| `@ROUTE-OBJECT` | `objects`：集合（任意物件） | `mode`（同 ROUTE-NET）、連線樣式屬性、`priority` | 輸入物件 | 13.7 |
| `@UNROUTE-NET` | `net`：單一（網路） | `priority` | 輸入物件 | 13.7 |
| `@UNROUTE-OBJECT` | `objects`：集合（任意物件） | `priority` | 輸入物件 | 13.7 |

### A.9 集合、群組與疊畫

| 關鍵字 | 槽位 | 參數 | 結果 | 章節 |
|---|---|---|---|---|
| `@BUNDLE` | `members`：集合（實例） | — | 新疊畫 | 11.8 |
| `@GROUP` | `members`：集合（任何物件） | `spacing_x`（數字，整數）、`spacing_y`（數字，整數） | 輸入物件 | 11.5 |
| `@ADD-TO` | `items`：任意（任何物件）；`target`：任意（集合） | — | 目標集合 | 5.2.2 |
| `@REMOVE-FROM` | `items`：任意（任何物件）；`target`：任意（集合） | — | 目標集合 | 5.2.2 |
| `@UNION` | `sets`：集合（任何物件） | — | 新集合 | 5.2.2 |

### A.10 約束與優先順序

| 關鍵字 | 槽位 | 參數 | 結果 | 章節 |
|---|---|---|---|---|
| `@ALIGN` | `members`：集合（可定位） | `axis`（位置 1；必填；`"x"`、`"y"`）、`priority` | 新約束；幾何事實 | 14.2 |
| `@ORDER` | `members`：集合（可定位） | `direction`（位置 1；必填；`"left-to-right"`、`"right-to-left"`、`"bottom-to-top"`、`"top-to-bottom"`）、`priority` | 新約束；幾何事實 | 14.3 |
| `@DISTRIBUTE` | `members`：集合（可定位） | `direction`（位置 1；必填；`"left-to-right"`、`"right-to-left"`、`"bottom-to-top"`、`"top-to-bottom"`）、`priority` | 新約束；幾何事實 | 14.4 |
| `@DISTANCE` | `from`：任意（可定位）；`to`：任意（可定位） | `direction`（位置 1；必填；`"left-to-right"`、`"right-to-left"`、`"bottom-to-top"`、`"top-to-bottom"`）、`min`（數字）、`max`（數字）、`exact`（數字）、`priority`；`min`、`max`、`exact` 至少其一；`exact` 不得與 `min` 並用；`exact` 不得與 `max` 並用 | 新約束；幾何事實 | 14.5 |
| `@ORIENT-RELATION` | `from`：單一（可定朝向，或接腳）；`to`：單一（可定朝向，或接腳，與 `from` 一致） | `relation`（位置 1；必填；ORIENT 的八種代碼，不含其方位詞，或它們的非空陣列）、`priority` | 新約束；幾何事實 | 14.9 |
| `@TABLE` | `rows`：集合（可定位；巢狀集合為表格的列表） | `transpose`（布林；預設 `false`）、`priority` | 新約束；幾何事實 | 14.11 |
| `@SOLVER-CONFIG`（不帶 `language` 的檔頭記錄、套件檔案的檔頭記錄，以及層級） | — | 求解器的具名參數：數字、字串、布林值與這些類型的陣列；`spacing_x`、`spacing_y`（數字，整數）與 `junction`（字串：`$` 名稱；在層級記錄中亦可為該層級的 `%` 名稱；或空字串）由本標準命名；其餘 key 由各求解器定義 | 無物件；求解器設定 | 14.10 |
| `@PRIORITY` | `target`：單一（約束、用例、集合） | `priority`（位置 1；必填） | 輸入物件；依 key 覆寫（key：物件） | 14.8 |

### A.11 電氣語義

| 關鍵字 | 槽位 | 參數 | 結果 | 章節 |
|---|---|---|---|---|
| `@CURRENT` | `terminal`：單一（端子） | `direction`（位置 1；必填；`"in"`、`"out"`、`"unspecified"`） | 輸入物件；依 key 覆寫（key：端子） | 15.2 |
| `@ASSERT-INSTANCE` | `members`：集合（實例） | SELECT-INSTANCE 中除 `role` 與 `recursive` 外的條件、`has`（屬性名稱陣列）、`same`（屬性名稱陣列）、`different`（屬性名稱陣列）、`message` | 無物件；檢查項 | 15.3 |
| `@ASSERT-TERMINAL` | `members`：集合（端子） | SELECT-TERMINAL 中除 `role` 與 `recursive` 外的條件、`same`、`different`、`message` | 無物件；檢查項 | 15.3 |
| `@ASSERT-NET` | `members`：集合（網路） | SELECT-NET 中除 `role` 與 `recursive` 外的條件、`same`、`different`、`message` | 無物件；檢查項 | 15.3 |
| `@ASSERT-USAGE` | `members`：集合（用例）；`definition`：定義（一般定義），選填 | `symbol`、`drawn`（布林）、`same`、`different`（陣列；僅限 `"definition"`）、`message` | 無物件；檢查項 | 15.3 |
| `@ASSERT-IN` | `members`：集合（任意物件）；`set`：任意（集合） | `message` | 無物件；檢查項 | 15.3 |
| `@ASSERT-TOPOLOGY` | `members`：集合（實例、疊畫、用例、集合） | `topology`（位置 1；必填；`"series"`、`"parallel"`、`"star"`、`"loop"`）、`exclude`（配對陣列：類型、端子）、`message` | 無物件；檢查項 | 15.4 |

### A.12 其他標準詞彙

- 種類：`instance`、`terminal`、`pin`、`net`、`point`、`wire`、`usage`、`bundle`、`text`、`annotation`、`label`（5.3、9.4）。
- `::` 後方的子物件：`position`、`net`（6.5）。
- `${…}` 讀取的屬性：`name`、`value`、`model`、`type`、`net`、`x`、`y`（13.6）。
- 布林值：`true`、`false`（3.7）。

樣式屬性（9.3）：

| 屬性 | 類型與取值 | 預設值 |
|---|---|---|
| `linecolor` | 顏色 | — |
| `linestyle` | `"solid"`、`"dashed"`、`"dotted"`、`"dash-dot"`、`"double"`、`"bold"`、`"none"` | `"solid"` |
| `linewidth` | 數字 | `0` |
| `cap` | `"butt"`、`"round"`、`"square"` | — |
| `join` | `"miter"`、`"round"`、`"bevel"` | — |
| `fillcolor` | 顏色 | — |
| `textcolor` | 顏色 | — |
| `size` | 數字 | — |
| `font` | 字串 | — |
| `bold` | 布林 | — |
| `italic` | 布林 | — |
| `arrowsize` | 數字 | — |
| `textvisible` | 布林 | `true` |

關鍵字作為參數接收的樣式屬性（9.2.5）：*線條*：`linecolor`、`linestyle`、`linewidth`、`cap`、`join`、`fillcolor`、`arrowsize`；*區域*：`linecolor`、`linestyle`、`linewidth`、`cap`、`join`、`fillcolor`；*連線*：`linecolor`、`linestyle`、`linewidth`、`cap`、`join`；*文字*：`textcolor`、`size`、`font`、`bold`、`italic`、`textvisible`。
