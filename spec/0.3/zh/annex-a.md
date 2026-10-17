## 附录 A 关键字签名（规范性）

本附录由标准词表 `rnl-words-0.3.json` 生成（5.3）；两者不一致时是勘误。它列出每个关键字收什么、给什么；含义见最后一栏所指的节。

- **对象槽位**按标准写法的次序列出，带槽位名（可以写成角色，5.3）和收什么：*one*：单个对象；收到集合时，对每个成员各做一次（5.2.1），只有 PRIORITY 收到带 GROUP 的集合时作用于分组本身（14.8）；*set*：集合；为这个槽位散写的几个对象合成一个集合（5.2.1）；*any*：一个对象，或作为整体的一个集合；*shape*：一个点，或一组点（图元的两种写法，9.2.5）；*body*：花括号里的语句，封装（5.4）；*definition*：定义：`$` 名字、绑定了定义的 `%` 名字，或匿名定义（5.6、6.6）。吸收的输入是严格的，封装的输入是容错的（5.4、7.3）。
- **种类**是种类名（`instance`、`terminal`、`net`、`point`、`wire`、`usage`、`bundle`、`text`、`annotation`、`label`）或：*any*，任何对象；*level*，单独的 `#`：本层；*owner*，符号体里单独的 `?`：用这个符号画出的对象；*position*，对象的子对象 `::position`（它的原点）；*locatable*，有位置的对象：元件、叠画、有引脚的端子、点、文字、用中心写的图元的中心、有自己位置的画出的用例，或由它们组成的集合（14.1）；*orientable*，有朝向的对象：元件、叠画、有符号的用例，或由它们组成的集合（14.9）；*constraint*，约束关键字产生的对象；*set*，集合，写成名字。
- **参数**标了*位置 1*、*位置 2*的可以按位置写，按这个次序；其余只能写名字（4.2）。`priority` 的取值总是 `"disabled"`、`"low"`、`"normal"`、`"high"`；不写时用记录头的优先级，只是 AS、ATTACH 从记录头只取 `"disabled"`（14.8）。
- **结果**：语句给出的对象（5.5），以及它的事实怎样覆盖（5.8）。没写覆盖方式的，事实累加。
- 没有注明的，关键字可以写在层的任何记录里；注明*也可写在符号体里*的，还可以写在 DEFINE-SYMBOL 的体里。

### A.1 记录

`@RNL(…) {…}` 把它的语句提交到本层（5.4）。记录头的字段（2.8），都写名字：

| 字段 | 取值 | 默认值 | 影响解释 |
|---|---|---|---|
| `type` | `"authored"`、`"generated"`、`"header"` | `"authored"` | 是 |
| `scope` | 字符串 | — | 是 |
| `priority` | `"disabled"`、`"low"`、`"normal"`、`"high"` | `"normal"` | 是 |
| `author` | 字符串 | — | 否 |
| `hash` | 字符串 | — | 否 |
| `numbering` | 字符串 | — | 是 |
| `modified` | 字符串 | — | 否 |

### A.2 头段记录与包

| 关键字 | 对象槽位 | 参数 | 结果 | 节 |
|---|---|---|---|---|
| `@VERSION`（只在头段记录里） | — | `version`（位置 1；必需） | — | 16.1 |
| `@PACKAGE`（只在包文件的头段记录里） | — | `name`（必需），`version`（必需） | — | 2.7 |
| `@USE-PACKAGE`（也可写在头段记录里） | `body`：body，可省 | `name`（必需），`version`，`path` | — | 16.2 |
| `@ALIAS`（只在头段记录里） | — | `alias`（位置 1；必需），`original`（位置 2；必需），`definition` | — | 16.5 |
| `@USE-ALIASES`（只在头段记录里） | — | `name`（必需），`version`，`path` | — | 16.5 |
| `@DEVICE`（只在头段记录里） | — | `name`（必需），`model`，`terminals`（必需；数组） | — | 6.12 |

### A.3 定义

| 关键字 | 对象槽位 | 参数 | 结果 | 节 |
|---|---|---|---|---|
| `@DEFINE-SYMBOL`（也可写在符号体里） | `body`：body | `box`（数组） | 定义（符号） | 9.2 |
| `@DEFINE-STYLE` | `extends`：body（样式） | 样式属性（9.3） | 定义（样式） | 9.3 |
| `@DEFINE` | `body`：body | `roles`（必需；数组），`parameters`（数组） | 定义（普通定义） | 9.4 |

### A.4 符号内容与图元

| 关键字 | 对象槽位 | 参数 | 结果 | 节 |
|---|---|---|---|---|
| `@PIN`（只在符号体里） | `position`：one（point） | `terminal`（必需；端子，或非空的端子数组），`label`，`direction`（`"right"`、`"up"`、`"left"`、`"down"`） | 新的引脚 | 9.2.4 |
| `@LINE`（也可写在符号体里） | `points`：set（point、terminal） | `arrow`（`"none"`、`"start"`、`"end"`、`"both"`；默认 `"none"`），`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；默认 `"none"`），line 组的样式属性 | 符号里的一个图元；写在场景里时是新的标注（13.4） | 9.2.5 |
| `@RECTANGLE`（也可写在符号体里） | `points`：shape（point、terminal） | `width`（数），`height`（数），`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；默认 `"none"`），area 组的样式属性；`width`、`height` 同时写或都不写 | 符号里的一个图元；写在场景里时是新的标注（13.4） | 9.2.5 |
| `@CIRCLE`（也可写在符号体里） | `points`：shape（point、terminal） | `radius`（数），`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；默认 `"none"`），area 组的样式属性 | 符号里的一个图元；写在场景里时是新的标注（13.4） | 9.2.5 |
| `@ELLIPSE`（也可写在符号体里） | `points`：shape（point、terminal） | `width`（数），`height`（数），`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；默认 `"none"`），area 组的样式属性；`width`、`height` 同时写或都不写 | 符号里的一个图元；写在场景里时是新的标注（13.4） | 9.2.5 |
| `@ARC`（也可写在符号体里） | `points`：set（point、terminal） | `arrow`（`"none"`、`"start"`、`"end"`、`"both"`；默认 `"none"`），`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；默认 `"none"`），line 组的样式属性 | 符号里的一个图元；写在场景里时是新的标注（13.4） | 9.2.5 |
| `@BEZIER`（也可写在符号体里） | `points`：set（point、terminal） | `arrow`（`"none"`、`"start"`、`"end"`、`"both"`；默认 `"none"`），`fill`（`"none"`、`"linecolor"`、`"fillcolor"`；默认 `"none"`），line 组的样式属性 | 符号里的一个图元；写在场景里时是新的标注（13.4） | 9.2.5 |
| `@TEXT`（也可写在符号体里） | `attach`：one（point、instance、usage、bundle、terminal、wire、annotation、net、`?`） | `text`（位置 1；必需），`key`，`dx`（数），`dy`（数），`halign`（`"start"`、`"middle"`、`"end"`；默认 `"start"`），`valign`（`"baseline"`、`"top"`、`"middle"`、`"bottom"`；默认 `"baseline"`），`rotate`（`0`、`90`、`180`、`270`；默认 `0`），text 组的样式属性 | 新的文字；写了 `key` 时是附着对象的子对象；按键覆盖（键：附着对象和引用名，写了 key 时） | 13.3 |

### A.5 放置与外观

| 关键字 | 对象槽位 | 参数 | 结果 | 节 |
|---|---|---|---|---|
| `@PLACE`（也可写在符号体里） | `target`：one（instance、usage、bundle、point、text、`::position`） | `x`（位置 1；数），`y`（位置 2；数），`priority`；`x`、`y` 至少写一个 | 输入的对象；几何事实（键：对象和轴） | 11.1 |
| `@ORIENT`（也可写在符号体里） | `target`：one（instance、usage、bundle） | `code`（位置 1；必需；`"R0"`、`"R90"`、`"R180"`、`"R270"`、`"MX"`、`"MY"`、`"MXR90"`、`"MYR90"`），`priority` | 输入的对象；几何事实（键：对象） | 11.2 |
| `@LAYER` | `target`：one（instance、usage、bundle、wire、annotation、text） | `layer`（位置 1；必需；`"bottom"`、`"middle"`、`"top"`） | 输入的对象；按键覆盖（键：对象） | 11.6 |
| `@SET-SYMBOL`（也可写在符号体里） | `target`：one（instance、usage、bundle、`#`）；`symbol`：definition（符号） | — | 输入的对象；外观队列 | 11.3 |
| `@SET-STYLE` | `target`：one（instance、usage、bundle、wire、annotation、text）；`style`：definition（样式） | — | 输入的对象；外观队列 | 11.3 |

### A.6 选择

| 关键字 | 对象槽位 | 参数 | 结果 | 节 |
|---|---|---|---|---|
| `@SELECT-INSTANCE` | `scope`：any（`#`、任何对象） | `name`，`model`，`type`，`role`，`recursive`（布尔；默认 `true`） | 新的集合，成员是元件 | 10.1 |
| `@SELECT-TERMINAL` | `scope`：any（`#`、任何对象） | `name`，`model`，`type`，`terminal`（端子），`net`，`wired`（布尔），`current`（`"in"`、`"out"`、`"unspecified"`），`role`，`recursive`（布尔；默认 `true`） | 新的集合，成员是端子 | 10.1 |
| `@SELECT-NET` | `scope`：any（`#`、任何对象） | `name`，`port`（布尔），`wired`（布尔），`global`（布尔），`role`，`recursive`（布尔；默认 `true`） | 新的集合，成员是网络 | 10.1 |
| `@SELECT-WIRE` | `scope`：any（`#`、任何对象） | `net`，`bus`（布尔），`role`，`recursive`（布尔；默认 `true`） | 新的集合，成员是导线 | 10.1 |
| `@SELECT-LABEL` | `scope`：any（`#`、任何对象） | `key`，`role`，`recursive`（布尔；默认 `true`） | 新的集合，成员是标签 | 10.1 |
| `@SELECT-ANNOTATION` | `scope`：any（`#`、任何对象） | `role`，`recursive`（布尔；默认 `true`） | 新的集合，成员是标注 | 10.1 |
| `@SELECT-USAGE` | `scope`：any（`#`、任何对象）；`definition`：definition（普通定义），可省 | `recursive`（布尔；默认 `true`） | 新的集合，成员是用例 | 10.1 |
| `@SELECT-ANCESTOR-USAGE` | `scope`：any（任何对象）；`definition`：definition（普通定义） | `recursive`（布尔；默认 `true`） | 新的集合，成员是用例 | 10.1 |

### A.7 套用

| 关键字 | 对象槽位 | 参数 | 结果 | 节 |
|---|---|---|---|---|
| `@AS`（也可写在符号体里） | 定义的各角色，然后是定义（`definition`，写在最后） | 定义的参数（按参数表的次序按位置写，或写名字；9.4），`priority` | 新的用例 | 9.4 |
| `@ATTACH`（也可写在符号体里） | 定义，然后是它的各角色（`definition`，写在最前） | 定义的参数（按参数表的次序按位置写，或写名字；9.4），`priority` | 新的用例 | 9.4 |

### A.8 导线

| 关键字 | 对象槽位 | 参数 | 结果 | 节 |
|---|---|---|---|---|
| `@WIRE` | `net`：any（net）；`points`：set（terminal、point） | `mode`（`"auto"`、`"straight"`、`"horizontal-vertical"`、`"vertical-horizontal"`、`"manhattan"`、`"manhattan45"`；默认 `"auto"`），wire 组的样式属性 | 新的导线；收到导线时改写它，按键覆盖（键：这个对象） | 13.1 |
| `@ROUTE` | `net`：one（net） | `mode`（`"auto"`、`"straight"`、`"horizontal-vertical"`、`"vertical-horizontal"`、`"manhattan"`、`"manhattan45"`；默认 `"auto"`），wire 组的样式属性 | 新的布线 | 13.7 |

### A.9 集合、分组与叠画

| 关键字 | 对象槽位 | 参数 | 结果 | 节 |
|---|---|---|---|---|
| `@BUNDLE` | `members`：set（instance） | — | 新的叠画 | 11.8 |
| `@GROUP` | `members`：set（任何对象） | — | 输入的对象 | 11.5 |
| `@ADD-TO` | `items`：any（任何对象）；`target`：any（set） | — | 目标集合 | 5.2.2 |
| `@REMOVE-FROM` | `items`：any（任何对象）；`target`：any（set） | — | 目标集合 | 5.2.2 |
| `@UNION` | `sets`：set（任何对象） | — | 新的集合 | 5.2.2 |

### A.10 约束与优先级

| 关键字 | 对象槽位 | 参数 | 结果 | 节 |
|---|---|---|---|---|
| `@ALIGN` | `members`：set（locatable） | `axis`（位置 1；必需；`"x"`、`"y"`），`priority` | 新的约束；几何事实 | 14.2 |
| `@ORDER` | `members`：set（locatable） | `direction`（位置 1；必需；`"left-to-right"`、`"right-to-left"`、`"bottom-to-top"`、`"top-to-bottom"`、`"x"`、`"y"`），`priority` | 新的约束；几何事实 | 14.3 |
| `@DISTRIBUTE` | `members`：set（locatable） | `axis`（位置 1；必需；`"x"`、`"y"`），`priority` | 新的约束；几何事实 | 14.4 |
| `@DISTANCE` | `from`：any（locatable）；`to`：any（locatable） | `direction`（位置 1；必需；`"left-to-right"`、`"right-to-left"`、`"bottom-to-top"`、`"top-to-bottom"`、`"x"`、`"y"`），`min`（数），`max`（数），`exact`（数），`priority`；`min`、`max`、`exact` 至少写一个；`exact` 不与 `min` 同写；`exact` 不与 `max` 同写 | 新的约束；几何事实 | 14.5 |
| `@ORIENT-RELATION` | `from`：any（orientable）；`to`：any（orientable） | `relation`（位置 1；必需；`"R0"`、`"R90"`、`"R180"`、`"R270"`、`"MX"`、`"MY"`、`"MXR90"`、`"MYR90"`），`priority` | 新的约束；几何事实 | 14.9 |
| `@PRIORITY` | `target`：one（constraint、usage、set） | `priority`（位置 1；必需） | 输入的对象；按键覆盖（键：对象） | 14.8 |

### A.11 电气语义

| 关键字 | 对象槽位 | 参数 | 结果 | 节 |
|---|---|---|---|---|
| `@CURRENT` | `terminal`：one（terminal） | `direction`（位置 1；必需；`"in"`、`"out"`、`"unspecified"`） | 输入的对象；按键覆盖（键：端子） | 15.2 |

### A.12 其他标准词

- 种类：`instance`、`terminal`、`net`、`point`、`wire`、`usage`、`bundle`、`text`、`annotation`、`label`（5.3、9.4）。
- `::` 后面的子对象：`position`、`net`（6.5）。
- `${…}` 读的属性：`name`、`value`、`model`、`type`、`net`、`x`、`y`（13.6）。
- 真假值：`true`、`false`（3.7）。

样式属性（9.3）：

| 属性 | 类型和取值 | 默认值 |
|---|---|---|
| `linecolor` | 颜色 | — |
| `linestyle` | `"solid"`、`"dashed"`、`"dotted"`、`"dash-dot"`、`"double"`、`"bold"`、`"none"` | `"solid"` |
| `linewidth` | 数 | `0` |
| `cap` | `"butt"`、`"round"`、`"square"` | — |
| `join` | `"miter"`、`"round"`、`"bevel"` | — |
| `fillcolor` | 颜色 | — |
| `textcolor` | 颜色 | — |
| `size` | 数 | — |
| `font` | 字符串 | — |
| `bold` | 布尔 | — |
| `italic` | 布尔 | — |
| `arrowsize` | 数 | — |

关键字可以当参数收的样式属性（9.2.5）：*line*：`linecolor`、`linestyle`、`linewidth`、`cap`、`join`、`fillcolor`、`arrowsize`；*area*：`linecolor`、`linestyle`、`linewidth`、`cap`、`join`、`fillcolor`；*wire*：`linecolor`、`linestyle`、`linewidth`、`cap`、`join`；*text*：`textcolor`、`size`、`font`、`bold`、`italic`。
