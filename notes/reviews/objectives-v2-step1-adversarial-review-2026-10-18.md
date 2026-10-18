# Objectives v2 Step 1 独立对抗审阅

审阅者：独立代理 `/root/step1_adversarial_review`。审阅日期：2026-10-18。

## 当前结论

**最新结论：R1–R7 已关闭，candidate-r7-budget-02 的生产增量及四项定向执行已通过独立复核，当前未发现剩余生产 must-fix。T1 与 T6 的默认未触帽 trace、候选 code/精确层、routed 记录、选择和工作量已独立比对一致；另已核短 T1 真 CLI 解字节及 source 绑定。T6 的 profile 不含实际解几何字节，不能据此声称 T6 解文件逐字节相同。完整 tiny、最终语料与全套门禁仍待闭环，不能给出整体验收结论。raw 0.2 T6 的质量近似与量化性能风险裁决保持原适用范围。** 本记录不代表完整验收或最终合入批准。

审阅只读生产代码、测试和已有验证材料；未运行 cargo、Node 构建或 Playwright，未修改实现。运行验证由实施负责人持统一令牌完成。以下结论来自独立的静态数据流、数学和标准核查，不以实现者自报测试通过代替代码审查。

## 范围与依据

- 基准：`main=41d6b7d68544601f7a206af69c5b91286a2ed7c9`。
- 不可变候选：`target/objectives-v2-validation/review-candidate-01`；`review-manifest.json` SHA-256：`621209fd57cb8ee7e204ac5bc92c316cd7007f0ed87542c124cd95cf438df4c6`。
- 对 manifest 所列生产文件及相对基准的改动进行了完整通读；测试迁移逐文件对照，新增配置、CURRENT、路由、CLI 和 wasm 验收测试阅读。后续生产修改不自动继承本候选结论。
- 优先依据：用户交接 `C:/Users/jielu/Downloads/codex-handoff-objectives-v3.md`、只读主方案 `objectives-v2-proposal.md` v2.1、逐条回复 `objectives-v2-phase0-reply.md`，及本仓库 gate/implementation-contract。
- 标准基线：`D:/repositories/rnl-dev` 提交 `b5b2b1402045f8b4f0716afd51fbafa87c0a9d8b` 的 0.3 标准，重点 8.5、9.2.4、11.7、13.1/13.3/13.7、14.8、15.2、16.4、17；虚拟件补查 12.1/12.2。

下述代码行号均指 **candidate-01 中的同名文件**，不是移动中的工作树。

## 必须修复

### R1 — P1：附件符号朝向被旧规则覆写，绕开配置计分

位置：`crates/rnl-solver/src/search.rs:1506–1525`、`1653–1664`。

`Search::candidate` 对单成员 Port 类直接选择第一个让 pin 面向宿主的朝向并覆写 `plan.values`；`class_node` 又排除纯 Port 类。这使显式 `symbol_direction` 无法在这些合法虚拟件上选择最低分朝向，且无 Element 的图不进入根搜索。例：一个 R0 下从单 pin 指向框中心为 down 的 TAG，挂接到 left 出线宿主，只配置 TAG 朝 down；旧覆写可强制左右朝向、得分 1，并排除可达的 0 分候选。

依据：交接 Step 1“参数化结构 proposal”“完整配置排序”；v2.1 第 4 步删除隐性固定目标；标准 12.1 第 5 项规定虚拟件独立朝向不跟随主对象出线方向。

修复要求：移除固定覆写，让 Port 自由朝向类也使用完整配置评分的候选排序；无 Element 但存在自由朝向类时也可搜索。Step 1 保留前二截断可以接受。

最小回归：单 pin 自定义 TAG + 侧向宿主，单独 `symbol_direction=down` 得 0；再覆盖纯 Port 图和保留 ORIENT 事实。

状态：实施负责人接受；工作树已修并增加 fixture，待冻结复审。

### R2 — P1：生产硬路由仍固定使用旧 bend/crossing 成本

位置：`crates/rnl-solver/src/route.rs:2135–2148`，与硬 A* 原固定成本路径。

`configured_layout` 直接调用 legacy `hard_layout/route_layout`，只把 `RoutingWeights` 注入软搜索。通常硬阶段成功即停止，因此变更/关闭配置中的 wire_turns、wire_crossings 等项不会作用于生产硬阶段的局部比较。

依据：逐条回复 A02 明确把“新硬阶段成本受配置影响”与“固定旧成本的兼容承诺”分开；A04 和第 4 步不允许关闭项偷偷进入计分。“硬阶段不变”指保留旧障碍、连接/换对重试算法及 legacy seam，不能据此延后生产成本配置。

修复要求：生产硬搜索注入精确有限层级权重；关闭项 primary cost 为 0；保留显式旧成本 seam。允许按约定使用 legacy tie。A02 检查使用相同旧成本模式，不能要求新配置模拟全部旧 EXIT/ALONG tradeoff。

最小回归：同一 Layout 有“短而多弯”“长而少弯”两条可行路线，改变单项/层级后生产硬路线发生对应变化；显式旧成本下，新逐网驱动全硬成功与旧 seam 的 NetOut 和 expansions 相同。

状态：实施负责人接受，路由实施者返工中。

### R3 — P1：先跑完所有硬网络，再补软/兜底，破坏后续网络障碍上下文

位置：`crates/rnl-solver/src/route.rs:1331–1382`、`2143–2217`。

候选先以既定 half-perimeter/anchors/id 顺序完成所有硬路由，再按 problem 顺序处理失败网络的 soft/fallback。于是较早网络的软/兜底输出不进入后续网络的硬搜索障碍；软处理顺序也不等于已定网络顺序。这会改变之后网络的硬成功路径及交叉/沿线局部代价。

依据：主方案第 4 步明确要求软路由输出作为其他网络的线影响后续网络，并单独测试；A02 的全硬成功局部兼容 seam 仍须保留。

修复要求：生产使用既定网络顺序逐网 hard → soft → fallback，把完成几何交给下一个网络；或可证明等价的重路由流程。各网软预算独立于硬预算。

最小回归：先处理的 N1 硬失败但软/兜底完成，后处理 N2 的直接硬路径穿 N1；N2 必须观察到 N1 的其他网几何。改变 problem 存储顺序不能改变已定排序；全硬成功旧成本 seam 不变。

状态：实施负责人接受，路由实施者返工中。

### R4 — P2：切换配置文件及 Restore Defaults 后仍使用旧 defaults

位置：`apps/viewer-ui/solve-settings.js:71`、`83–90`、`117–143`。

defaults 仅在开窗时读取一次。选择或手输新 TOML 路径只改路径，显示值、来源文字和“是否省略 override”的判定仍针对旧文件。例：A 的 beam=2，B 的 beam=8；从 A 切到 B 后界面仍显示 2，保存却不发 beam override，实际 solve 使用 8。Restore Defaults 清空路径也继续显示 A 的值。

依据：Step 1 已要求可用的配置文件选择和标量规则 UI；显示有效值与实际求解不一致不是 Step 3 编辑器扩展事项。

修复要求：路径变化/Restore 时重载对应 defaults/source，保留显式 overrides 和精确文本，保存等待当前路径解析；过期异步响应不能覆盖新路径。

最小回归：fake shell 为 A/B 返回不同默认值，覆盖选择、手输、Restore、已有 override、异步乱序和加载中保存。

状态：实施负责人接受，工作树已修并加回归，待冻结复审。

### R5 — P2：虚拟件所有角色网络被错误映射到物理 pin 0

位置：`crates/rnl-solver/src/structure.rs:1093–1102`；下游 `route.rs:2334–2344`。

去除内置角色名时改成遍历 usage 的所有 role，但每个 Net filler 都插入 `(object, 0)`。合法双端虚拟件 roles a/b、symbol PIN(a)=0 / PIN(b)=1、ATTACH na/nb 会变成 pin 0 同属 na 和 nb，同时漏掉 pin 1 的 nb 身份。该图进入软短路计分和结构邻接，例如 nb 导线穿 pin 0 时会误认为同网而免罚。

依据：标准 12.1 明确 role 即 usage terminal，12.2 明确允许多端虚拟件；runtime `interp/export.rs:360–374` 已导出准确的 object/pin/net 映射。Step 2 的完整 net-table 升级延期并不允许本次新增错误 pin 0 身份。

修复要求：对 usage 使用已导出的逐 pin 网络映射，或严格匹配 role 对应的 symbol pin terminals；不恢复角色名硬编码。

最小回归：双 pin 自定义 usage，na 仅 pin 0，nb 仅 pin 1；nb 线穿 pin 0 的最终 wire_shorts=1。

状态：实施负责人独立核实并接受，工作树已修并加 fixture/回归，待冻结复审。

## 已核查且可接受

| 能力 | candidate-01 静态结论 |
| --- | --- |
| 精确配置 | 原始 TOML span/十进制词素直接进 BigInt 比值；canonical 归一化参数与执行顺序；同级整数分类不受分数 weight 干扰；禁用项/空层剔除；未知字段、非法值及 Step 2 kind 拒绝；宿主 fallback 只允许默认路径 NotFound。 |
| CURRENT 与签名 | final CURRENT 按原 host terminal 导出；bundle 映射后仍保留独立语义端子；显式 direction_written 与生成默认 pin 区别；sig1/sig2 历史投影保留，新增 sig3 纳入新语义。 |
| CURRENT 图 | 原 semantic host 上先 SCC，再 DAG 分层，最后映射 display/bundle；多 direction 候选通过框架旋转 coexist；局部端口保留导出并诊断 inactive。 |
| 无名字结构 | 删除器件/供电/差分名称推理；生产 Rust 字面量 guard；stub 不再从 parent 参数重建 disabled rise/alignment；组归属来自 attached_host。R1/R5 是本项仍需修的具体缺口。 |
| 机制开关 | separate_symbols=false 在候选中归零 Gap/RowsApart/BlocksApart 几何间隙，并关闭 LP overlap/row-gap repair；事实仍先保留。 |
| 几何身份 | 独立 authored/generated wire ID、物理 pin/branch/bend ID、net 集和 source；生成边按身份组件连接；同坐标不同 pin 可输出零长度线；只切分 generated 路段，保留 authored WIRE 原点列及环。 |
| 软阶段与兜底 | 硬/软预算独立，局部 penalty seen-set 防同一对象反复收费；精确比较；兜底使用精确端点、组件逐步连接。R2/R3 修复后需重核调度/权重。 |
| 配置评分 | 排名只用已配置层及 code；容差采用锚定窗口；最终用实际 routing geometry 重算；报表有 canonical/digest、参数、raw/weight/weighted、层总分/容差、estimated/inactive/diagnostics。 |
| 宿主与兼容 | CLI problem/preview/solve 同一 resolver；solve 操作错误为 1、未知 CLI 选项为 2；旧 objectives localStorage 丢弃并一次提示，保留标量。R4 修复后需重核设置交互。 |
| wasm 与 CI | test-solver feature 直接解析 embedded TOML 并真实放置/路由；Node harness 实例化 wasm 并比较完整逻辑解和精确报告，而非仅 compile；config 路径已纳入 Rust/viewer/desktop CI 触发。实际运行结果由统一验证日志确认。 |

明确接受 Step 1 临时语义：旧正面积 symbol overlap；旧逐段 wire short/interior-box overlap；旧 texts_over；旧 bends/异网 crossing；符号包络 area 和新版 aspect；LP 四级及贴 resolution witness；朝向前二；auto WIRE 直线、不写回。symmetry、pin_wire_alignment、orientation_r0、origin_distance、include_wires=true 在本步拒绝，不要求提前实现。

极近端点的 fallback 引出若与无环身份树冲突，接受 D011 的确定性截断/去环优先级：保留不同身份连接与作者线。9.2.4 的出线方向作为正常引导，不据此要求保留必然产生几何回环的固定长度引出；这不是允许一般端点无故忽略方向。后续复审仍需检查新增 hard/soft 逻辑未破坏该边界。

## 非阻塞建议

- 更新 `lib.rs` 的 sig2 示例，以及 objectives/search/score 的旧固定目标、供电器件推理注释；其描述已与运行代码不符。
- 迁移测试中的旧注释（gap_kinds 小数部分、隐式 current depth 等）也应同步；避免仅改断言而留下反向说明。
- 最终证据应绑定最终生产快照和提交，说明哪些测试运行在旧 binary；candidate-01 之前的绿灯不能替代 R1–R5 修复后的定向和完整回归。

## 复审记录

### candidate-02：生产增量复审

`review-manifest.json` SHA-256：`24578866796e0fdd1b2879afaa5bf4e463ed3ad18351931f80db69eedb4e9137`；`production-manifest.json` SHA-256：`451c871ae337e93e73e171504ea299fa0e2c1e1dea4ba8a62a7d47ed183035c3`。清单所有文件 SHA 均重新核对一致。完整阅读相对 candidate-01 的生产增量及对应测试。

- **R1 关闭**：`search.rs` 删除 candidate 阶段 Port 朝向覆写；纯 Port 类由根节点参与完整配置排名；无 Element 但有自由类也启动搜索。单 pin TAG 回归直接验证取 R0 且 raw=0。
- **R2 关闭**：`route.rs:1031` 的 `search_configured` 使用 exact Q；primary 只加已配置 length/turns/crossings，随后 raw length 和 legacy tie。硬障碍语义保留；显式 None 旧成本模式仍走原搜索。`route/tests.rs:547` 以小于 f64 可保留范围的 `2^-1100` turns 权重验证路线选择反转；`:522` 比较全硬成功多网 seam 的完整输出和工作数。
- **R3 关闭**：`route.rs:2346` 逐网按共同 `net_order` 完成 hard/soft/fallback；完成线条 mark 到同一硬 grid 再处理下网。`:723` 新分数坐标轴段标记保持精确交叉/端点/共线区分，不重建 lattice；新增回归覆盖 soft、fallback、反向存储顺序及离格点段。
- **R5 关闭**：`structure.rs:1093` 对 usage 读取 runtime 的逐物理 pin 网络，不再合到 pin 0；双端虚拟件测试验证独立 membership 及 nb 线穿 na pin 的最终 short=1。
- **额外 rank 修复通过**：零容差快速分支仅比较配置的层数，none 配置忽略残留 score slice，纯 code 决胜。旧 joins-counter 断言改为纯 code 顺序，原容差窗口断言保留。
- 已读取集中验证日志 `step1-review-current-02.log`（9/9）、`step1-review-route-01.log`（30/30）、`step1-review-score-01.log`（5/5）；这些是执行者日志证据，审阅者未自行编译。

**R4 保持开放（candidate-02 的 `apps/viewer-ui/solve-settings.js`）：**

1. `read` 在约 136 行仅用 `explicit.has(key)` 决定发送。host defaults 的 tolerance 是 `1/1000000`（`apps/desktop/src/solve.rs:84`），但 `options.rs:333` override 只接受十进制。开窗后把 tolerance 改为 0.1，再改回显示的 `1/1000000`，就会发送非法 override 而无法保存。原先相等默认值被省略，这属于本轮显式状态跟踪引入的回归。应提供可解析的精确有限十进制文本或精确转换通道，不能用 JS Number/f64。
2. `answer` 等待 defaults 的代际检查正确，但在随后 `await shell.checkSolverOptions(...)` 期间字段仍可编辑（dialog 只禁用 OK）。校验 A 尚未返回时改为 B，旧校验返回会保存旧 `kept`，丢失用户当前路径/标量值。须在校验完成后核对表单 revision 并重试，或禁用校验期间可变字段。

两处已及时发送实施负责人和 root。现有 A/B 默认值切换、Restore、defaults 乱序/加载中保存测试保持有效；还需补“显示默认重新输入”和“host 校验中编辑”的回归。

### 旧测试失败迁移：待后继候选

独立读取 `step1-solver-lib-01.log`：旧 binary 177/192，通过数不视作修复后结果。实施者提供了逐项原因/选择/替代保障表 `notes/solver-design/objectives-v2-step1-test-migration.md`，目前只是待执行计划。

特别要求保留：tiny-budget 只对 `routing_fallbacks` 对应网放开 through-box 检查，其余 hard 网及全部身份树/事实不放开；正常 hard 引出方向失败不能使用 D011 极近 fallback 例外；旧精确布局测试的替代固定 frame 必须通过明确 PLACE/ORIENT 事实构建，不暗设 witness，不修改原始输入或通用 case helper。candidate-02 新 0.3 fixture 有来源及变换清单，原 0.2 测试保留并新增独立 0.3 覆盖。最终测试增量及固定布局评分工具尚待复审。

### candidate-03-ui：R4 残余关闭

增量基于 candidate-02；`review-manifest.json` SHA-256：`be42421c2d52738233ec05f2f3362cccc0c4f48bf8bd70bcef948ab56767dbd0`。三个文件 hash 全部独立核对一致。

`written` 用 BigInt 检查分母仅含 2/5 因子，再做精确有限长除，显示值可原样输入十进制 override，未经过 Number/f64；超过 f64 精度的默认值回输回归检查完整文本。表单 revision 覆盖 input/change、文件选择和 Restore；host 校验返回后若 revision/path 改变，重新等最新 defaults 并校验，不会保存旧 `kept`。慢校验 A→B 回归要求第二次校验 B 后才保存。

**R4 静态复审关闭。** 截至这一增量，已审生产修复没有剩余 must-fix；浏览器执行日志尚待统一验证提供，最终测试迁移/评分工具和验收结论仍未审结。

后续执行证据：独立读取 `target/objectives-v2-validation/step1-gates/P0-settings-r4-02-ui03.log` 与对应 JSON，Chromium 9 项、WebKit 9 项，共 18/18 通过，进程 exit=0。包含精确默认值回输、默认值异步乱序、加载中保存及 host 校验中切换路径。R4 已完成执行闭环。

### candidate-04-tool：固定记录评分工具，覆盖校验待修

清单 SHA-256：`d7dc21606b0f730d5e22617ce4ac3fe9ddc2aaf5ebbc0851b455f25253fd880b`，四个文件 hash 均独立核对一致。适度审阅 `crates/rnl-solver/tests/unit/current.rs:238–343` 的 test-only 评分入口及其旧 T1 输入、输出与执行日志。

可接受部分：原始 problem 与 runtime previous 通过稳定身份映射既有 placed/wire 记录；所有样本共享同一 effective config；未调用 search/place/route，8.5 resolve 只作为原始事实及结构基底。最终从固定几何执行真实 `active.report`，输出精确 FractionReport。PreviousWire 不携带端点身份，因此输出顶点使用独立 Bend；这与 Step 1 的形状/网络计数兼容，但工具不得据此宣称完成 Step 2 身份计数验收。

验收证据缺口已发送实施负责人：输入解释的 sink errors 未拒绝，且仅检查 previous 存在/签名兼容，未确认自由坐标和朝向由记录完整覆盖。缺失字段会静默保留新 resolve witness，结果不能称为纯旧解评分。应拒绝解释错误，并报告/校验覆盖情况；未覆盖且不能由原事实唯一确定的可计分自由量应拒绝或明确排除。此项属于验收工具可靠性，未发现相应生产回归；待小增量和回归证据后复审。

### candidate-05-coverage：评分工具覆盖缺口静态关闭

清单 SHA-256：`14bf216cd96f1f92bdf26cb98c527315abe873888cedb0596f65e4d64e9f886c`；两个文件 hash 独立核对一致，增量完整阅读。两次解释的 error 均产生 N/A；缺失自由轴或朝向使整层 N/A，并输出 missing/excluded/逐对象证书。证书只接受 solver record、保留且 as-written 的直接常量、由已覆盖父对象递归求值的 Position 关系；未把普通 LP witness 或 ALIGN 当作唯一解。作者线重建不再 filter_map，避免漏计和按原 wire ID 索引 compact Vec 的错位。

静态缺口关闭，新增完整/缺轴/缺朝向回归和批评分覆盖统计仍待集中执行。N/A 必须在最终对比表明确显示，不能转成 0 或称为完整评分。此工具只验证 Step 1 定义。

### driver-migration-review：测试增量复审，执行与出口诊断待闭环

清单 SHA-256：`766f288cd9e066c052c984bc3c31019a444ca364db0c8540fd34344a718e814d`；before/after 及八份 fixture hash 全部独立核对，完整阅读 22KB 测试增量。保留原通用 raw-input helper；历史 frame 通过显式 PLACE/ORIENT 加入，并断言原有 kept 列表不变及新增事实全部保留。固定 frame 测试只证明该机制，不再证明自由布局的旧质量。

迁移有据：tiny-budget 仅按精确 route+net 的 soft/fallback 报告放开穿框，其他轴/身份/树检查不变；结构 orphan 例外仍受 Port+Generated+无 attachable pin 约束和有限精确清单限制；T3 只在目标 out 网络要求正长度，保留其他网零长度身份连接；T5 显式选择原七对象，T6 用固定几何 x/id 邻居替换私有 candidate row；作者 WIRE 保留精确形状并补独立 1 short/1 body 夹具。原 raw facts/readback/determinism 保障继续存在。两项长测试被主动中断，不能算通过。

固定 T3 的 XU1:inp 与 VB:p 沿框边偏离 direction 提示，不能套 D011 fallback 例外。标准 9.2.4 说 direction 是提示；更高优先级交接能力表明确将“对自己的框只许沿引脚方向出框”列为 Step 2。因此可接受 Step 1 有限 legacy EXIT/tie 近似，但需保留：同 Layout 的 None/Some 诊断、open/blocked-exit 微夹具、其余 pin 原出口断言，以及两个具体例外首段沿自己框边且不穿内部的几何断言。禁止仅按 label 全局忽略。最终代码及执行证据待复审。

### candidate-06-performance：新实测风险与无损表示增量

同 raw 0.2 T6 的端到端记录约 6.342→305.568 秒。独立读取内部报告亦为 3.066→173.051 秒，A* 工作 13045→177821、LP pivots 3252→8943、routed 候选仍为 4，不能仅用并行墙钟噪声解释。root 授权进行定向性能修复，不改目标、预算或搜索策略。

清单 SHA-256：`863eca0edfe0ab504bda627fa4c471c21bb337919adbd65f58692705e2fb58e9`；route.rs/tests.rs hash 一致，完整阅读相对 candidate-02 的增量。HardScale 取 `D=lcm(den(length)*unit,den(turns),den(crossings))`，D 与 lattice unit 均正；weighted 乘 D、raw length/legacy 乘 unit，所以比较与原 exact Q 同构。hard clearance 恒 0，省略不改变字典序。BigInt 无定长溢出；零权重和极小非零权重保留；原 Q 搜索仅留作 test oracle，算法分支、障碍、预算和 state tie 未改。

生产增量静态无 must-fix；分数格点/混合权重/2^-1100 的完整 NetOut+expansion 对照、T6 串行前后计时与完整回归待执行。性能风险尚未因静态等价自动关闭。

### driver-migration-review-02 与路由执行证据

第二份 driver 清单 SHA-256：`2af6644423a44ac31d5fb0e0cdf0c143f128cd4eced7513a96cec71aea1197a6`，全部 hash 独立核对一致。只读比较第一份 after 与第二份 after：ExitViolation 现在保留物理 pin、起点和首个非零段；仅 fixed T3 文件主层允许 XU1:0/VB:0，逐个断言沿自己的 bbox 边且不穿内部，seen 集合必须精确等于这两 pin，其余 pin 仍必须满足方向提示。**测试契约边界静态关闭**，新 driver binary 执行仍待后续。

独立读取 `logs/step1-integer-hard-route-01.log`：32 通过、1 ignored；T3 同 Layout 的 None legacy 为 426 expansions / 0 个出口偏离，configured 为 1461 / 恰好两个。日志逐项记录 bbox/start/next，两个起点均不在框内部，首段均不穿体。open/blocked-exit 原微夹具继续通过。

独立读取 `logs/step1-exact-hard-profile-01.log`：fixed T6 同 Layout 原 Q 2.444586 秒、整数 0.244732 秒，约 9.989 倍；完整 NetOut、分叉顺序和 29378 expansions 相同。执行源码包含 candidate-06 test helper 的借用修正，最终须由后继快照绑定，不能声称不可变 06 原文件原样编译通过。

整 CLI 受控旧 02 记录另为 wall 337.820 秒 / CPU 307.797 秒，新整数 CLI 尚未完成，因此局部 10 倍提升不等于整流程退化已闭环。只读检查支持后续 profile 假设：`search.rs:1373` 每自由类八朝向各计算全配置精确 estimate，含全级碰撞/文字扫描、连接段平方交叉计数；`orient` 逐类执行且更新当前 ori。安全优化应先按阶段和 score_term 计数/计时，再在同一 View 内共享确定相同的几何中间值。跨类缓存不能仅按 class id，必须涵盖变化的 ori/at；不得把全配置评分缩为当前对象局部评分或恢复隐藏朝向偏好。任何优化均需保留完整 Q 层值、八朝向排序及最终输出/work 的 oracle 对照。

后续整 CLI 已完成：同 input/config hash，旧 02 wall/CPU 为 337.820/307.797 秒，整数为 267.626/250.109 秒。独立比较两份输出字节完全相同（SHA-256 `bb59c4ea8920c1bc100b9f9300e31e23b118cb726f43e710e3e669eebb7ccc43`）。这确认无损但未关闭全程性能风险；继续仅在 profile 证据触发的范围检查。

评分覆盖执行闭环：独立读取 `step1-gates/step1-record-coverage-regression-01.log`（1/1）与 `step1-fixed-score-t6-01.log`（1/1）；`fixed-scores-t6/{main,candidate02,integer}.json` 三份均 complete、missing=0，整数与 02 每项相同。main 和 02 的前五层皆 0，首个差异 wire_turns 为 33→59；后者 area 虽变小，也不能解释为完整配置下更优的交换。最终验收应明确这是有限候选搜索的质量退化及其已核查原因，不得以低优先级收益掩盖首个恶化层。

### 全流程 profile 与 T6 最终池排序的独立核查

已读 `step1-gates/step1-isolated-stage-profile-02.log`、`performance-integer-hard/stage-profile.json` 及 test-only `profile.rs`。入口用 Options::solver 忽略生成记录，按生产步骤执行；工作数与 CLI 一致。总 102.351 秒中 search 95.774、coords 5.232、route 1.173、final_score 0.045；4417 次 estimate，其中 orient/ranked_values 分别 94.436/94.351 秒。嵌套计时不能相加。`Search::made` 不包含逐类八朝向估计，因此 71 candidates 不是完整计分次数。

用 Python Fraction 对 JSON 中完整层值和 code 做独立数学重排（未构建/调用 solver）：10 个 placed 的 estimate 顺序与报告一致；4 个 routed 的严格字典序为 `[8,9,0,1]`，chosen=8。P0 候选 8/9 的前三层为 0/0/0，turns 为 59/61；beam 0/1 的 symbol_overlaps=4、wire_overlaps=4，turns 为 35/36，不能越过更高层胜过 P0。因此本例未见最终排序错误。旧固定解可合法、较优但不在有限候选池中；其具体缺席原因仍待旧事实有效性与实际八朝向 trace 证据，不推断全局不可达。

窄优化建议已给实施者：每次完整 View 共享作者线几何、按 object 索引的 bbox/pin、保留原 PointId/点序/Prim tie 的连接及 envelope。仍须各项分别应用 components/solid/owner-stub/nets_of 等原 eligibility；不能统一过滤集合，不能按 kind 缓存不同参数的 raw 分数，也不能跨 class/plan 复用 ori/at 已变化的数据。待冻结实现和完整 Q/八朝向 rank/work oracle 后复审。

### candidate-07-scoring-cache：完整增量复审；R6/P2 待修

清单 SHA-256：`6c7b9b8a7a60f4857afbe1f2096600f292b3864b011a53b77a2d8e3278f0387c`，八文件 hash 全部独立核对一致；完整阅读相对已审候选的生产、oracle 测试和 profile 增量。06 测试借用修正及 T3 None/Some 精确 pin 集合、沿边断言已经纳入该候选。

**R6 — P2 must-fix：退化符号框被误计为正面积重叠。** 位置为该候选 `crates/rnl-solver/src/score.rs:110–113`。缓存的 `ViewGeometry::overlaps` 改用原先用于文字的 `meet` 交叉不等式，和旧 `coords::overlapping` 的 `max(loA,loB)<min(hiA,hiB)` 不等价。反例：A=`[0,0,0,1]`、B=`[-1,-1,1,2]`；旧 x 相交宽度为 0，计 0；新 meet 四个不等式均真，计 1。合法线状/点状符号可以有零宽/零高 bbox，交接 Step 1 明确保留有面积重叠定义，因此不能由性能优化修改。

要求仅缓存 symbol-overlap 路径恢复旧 max/min 判定，并加入零宽、零高、点框包含于普通框的逐 term oracle。不得修改共享 `score::meet` 来顺便改变 label_overlaps 的旧口径。已及时通知实施者及 root，实施者独立确认接受。

其余静态检查可接受：Scoring cache 仅存在于单次 score/report；None 时 author collision 与 Prim estimate 集合分离；Prim 点序、PointId、重复点和 tie 不变；各 term 原 eligibility/inner-pin 豁免保留；同 kind 不同参数仍分别计算，Some(routed) 路径保持原定义。新 oracle 逐 term/full Q/rank/report 对照有实质覆盖；实际 T6 full-search oracle 与旧解事实/八朝向 trace 仍在执行，不能提前称全部等价通过。

### candidate-08-scoring-cache：R6 关闭；T6 质量边界闭环

清单 SHA-256：`6d0e17b8d6614f90aa6bcb65d177c66fe738cd5458cf1ebb3d7fe0f77d589053`，八文件 hash 独立核对一致。相对 07 唯一生产差异是 overlaps 恢复原 max/min 正面积公式，没有改共享 meet。新增零宽/零高/点框含于普通框的八朝向 oracle，通用 oracle 另增加 author 与 ROUTE 同网混合。独立读取 `step1-scoring-cache-degenerate-oracle-01.log`，2/2 通过。**R6 静态与定向执行闭环。**

07 的完整 T6 oracle 已完成：4417 次完整 Q 层数组与旧函数相同，候选 code/层值/route 层值及工作数与 pre-cache 相同；旧记录新增 102 个 PLACE/ORIENT 后全部保留，原 18 个 kept facts 不变。该 T6 的 symbol_overlaps 独占一层，07 每次该层等于旧函数；08 恢复旧函数，因此可复用 07 本例的语义 trace，无须重跑旧 337 秒 CLI。

独立读取全部 544 条真实 ranked_values trace，并用 Fraction 逐条重排，544/544 一致。34 类皆为单成员且 class label=R0。旧解 VB0/VB2/VB3/VB4/VB7 的 MY 根每类 16 次均排第 8（零基 rank=7），从未进入前二；P0 两份的前 34 个朝向均为 R0，故至少 VB0 的旧 MY 既不在 beam 的实际分支也不在 P0。旧整朝向在本次实际有限搜索中缺席，不能归罪最终 rank 错选。

技术裁决：raw 0.2 T6 的 33→59 turns 是明确的 Step 1 前二朝向/固定 sketch 近似局限；旧解合法且在当前完整评分下更优，不能写成更低层 area 的合理交换。用户允许 0.2 绘图质量下降，能力表明确 Step 2 才保证八朝向可达，因此本步不应恢复名字偏好、改预算或追求全局最优来掩盖。最终基准说明应列明此退化和实际证据。

性能仍需最终 08 纯运行证明：07 oracle 多花 73.108 秒 reference.score，不能作为无 oracle 性能数字；后续一次最终 raw T6 CLI 的受控 CPU/墙钟、输出字节和 work 对照可绑定最终生产源码，无须无依据重复旧基准。driver 新 binary 回归及交接全套门禁仍待集中执行。


### 最终 08 的性能风险专项裁决（不代替整体验收）

重新核对交接 v3：第 3 节（原文第 87/89 行）要求 0.2 能读能解且不新增失败，搜索/LP/A* 按工作量计；第 4 节能力表及临时行为（125/170 行）保留 Step 1 朝向前二；第 4/5 节（179/317–322 行）要求全套基准、用时和逐项计分前后并列、解释变化。没有数值倍率门槛，但这不能推导为任意退化都合格。

独立读取最终 `performance-integer-hard/comparison.json`（SHA-256 `55e7588e1e48ccf172fe7382dc1b3a0d2491a92dec763a5ada8066ad594e1bb1`）、原始 CLI stdout/stderr、纯 profile 和资源记录。最终 CLI exit=0，wall=179.946247 秒、CPU=172.203125 秒；受控旧 02 为 337.820340/307.796875 秒，分别改善 1.877/1.787 倍。对先前 main 的 6.342 秒跨轮比约 28.374 倍，只作为退化量级参考，不能写成严格隔离的倍率。最终期间存在其他任务的 CPU 活动，空闲内存由启动前 8.956/8.590 GiB 降到采样的 4.615 GiB；不能宣称机器完全空闲，也不能据此抹去真实的 172 秒 CPU 消耗。

最终 binary SHA-256 为 `97f5ae8745f7c0b8246b0175d39f5525efebf067efb050827a1adbf60efa5c1d`；build-input manifest SHA-256 为 `65b6ccc2a805ad12f54980ff7f635779bf523fcaf1b0590d32fd8f6def491062`，159 项当前文件 hash 全部重新核对一致。02、整数化和最终缓存三份解字节一致（`bb59c4ea8920c1bc100b9f9300e31e23b118cb726f43e710e3e669eebb7ccc43`）。最终 work=2993 resolve nodes / 8943 pivots / 71 candidates / 177821 A* expansions，配置、P0 选择、事实及精确分数不变。

08 纯单层 profile 的总值为 88.844 秒（测试进程 88.96 秒），search=80.893、coords=6.193、route=1.354。ranked_values 占 79.627 秒；16 次 orient × 34 个自由类 × 8 个朝向=4352 次完整配置评分，另 65 次 estimate，共 4417 次。08 与 07 的全部 544 条真实朝向 trace、old-record facts、placed 层数组/code、routed 数组和工作量独立逐项比较相等；最终 CLI 的单层 time=88.746 秒。`solve.rs:364/408` 的原有 0.2 升级前后两次 pass 解释了 raw 0.2 整 CLI 约两倍于最终单层时间，不能把两者混作同一口径。

**专项结论：当前 T6 剩余成本不新增技术 must-fix，可作为显著、已量化的 Step 1 已知风险保留，继续完成阶段验证。** 理由不仅是没有倍率门槛：完整精确配置排序属于已定行为；本例调用数由现有有限朝向/草图循环直接解释；此前两类可局部修复的重复成本已有无损优化和独立 oracle；最终输出、计分、预算和工作不变，未见错误重试、超出既定阶段或不终止的证据。当前再改变采样、启发式、预算或用局部评分替代完整配置，会改变已审语义，不能仅为恢复旧速度混入本轮。

保留边界：4417 只适用于此例，不是所有输入的评分上限；定位了成本不等于证明不可继续无损优化，也不等于性能已解决。最终 T1–T6 与 20 个电路仍须按同一要求完成并解释，若出现新增读/解失败、不完成、异常资源增长或成本无法由有限工作解释，必须重新审查，不能引用本单点裁决一并豁免。最终 PR/验收说明必须突出剩余耗时及适用范围；Step 2 几何和全八朝向实现应沿同基线重新测量，不能预先承诺恢复主干速度。本节只关闭当前已定位 T6 性能风险的代码返工要求，不关闭 driver、新测试执行或交接全套门禁验收。


### 最终 driver 新暴露的 T3 子层出口（证据待闭环）

已读取 `step1-gates/T-driver-final08-corrections-01.log`：4 个定向测试中 3 个通过，原主层精确两 pin 例外检查通过后，严格出口测试在 T3 的 ota5t 子层失败。M1:s 的首正段为 (-8,-7)→(-4,-7)，提示方向为 (0,-1)；62/62 facts 保留，4 placed/4 routed，107 expansions。未把此失败算通过，未修改实现或测试。

静态核查 fixture 第 12 行明确选择 0.2 NMOS；`libraries/standard/0.2/rnl-std-symbols.rnl:112–115` 的 box=[-3,-2,0,2]、source=(0,-2)。M1 的 R0 原点 (-8,-5) 给出实际 box=[-11,-7,-8,-3]，已观测首段是从右下角向框外离开，除起点不碰框。它不同于主层有正长度的沿框边段，`route::runs_along` 对此应为 false，不能无差别复用主层 true 断言。M2 的对称情形只能预测，不能据此提前加入允许集合。

标准边界仍是 Step 1 direction 提示与 Step 2 严格自盒出口。现有几何支持“角点向外而偏离提示”的临时近似分类，但最终裁决待同 Layout 的 None/Some 全部物理 pin、bbox、首段、inside/through 和完整集合诊断。若确认，新增测试须仅在具体 fixture+子层+实际 pin 集合内锁定 corner/outward/除起点不交框，保持主层 along-edge 条件及其他 pin 严格要求；不得仅按名字豁免，也不能把 D011 极近 fallback 当理由。


### T3 ota5t 子层实际诊断裁决

独立读取 `step1-gates/T-t3-ota-exit-diagnostic-01.log`（SHA-256 `ed9da91bd7c33944de9d5bb642f74ea8099d7a327716cd0d632db9f2ff9d3948`）和对应 JSON，及执行源码 `route/tests.rs`（SHA-256 `20a88f9614ba5bad5c09e5e71e138702a25d2b43990f1dcd7100a1aee22d2c4b`）。该诊断完整遍历每根输出线的两个物理端点，None 与 Some 使用同一 fixed Layout。None 为 96 expansions、违例空；Some 为 107、违例恰 `[(0,Some(2)),(1,Some(2))]`，均为 tail 网络的 terminal=s。M1 实际首段 (-8,-7)→(-4,-7)、box=[-11,-7,-8,-3]；M2 为 (0,-7)→(-4,-7)、box=[0,-7,3,-3]。二者 inside/through/along 均 false。其余存在首正段的端点均符合 direction；M3 的两个零长度身份边无可定义首正段，不伪造方向。

**技术裁决：可接受为 Step 1 已声明 direction hint 近似，不新增生产 must-fix。** 本例不是沿自己框边有正长度交集，而是从角点向框外离开，首正段与闭框仅交于起点；严格沿指示方向出自己框属于交接能力表 Step 2。不能使用 D011 fallback 或“配置成本改变”单独作为理由。旧 None seam 仍保留原行为。

已给实施者和验证者的必要测试边界：仅固定 `T3-ota5t.sp` 的 `ota5t` 子层允许 M1:s/M2:s 的物理 pin 2，确认 tail 网络与实际 terminal；违例数恰 2、集合精确相等，其他 pin/层仍严格。各例需断言起点为 bbox 角点、首段非零轴向；本例水平时必须从 x-max 向更大 x 或 x-min 向更小 x，固定 y 是 bbox 边，因此除起点不交闭框，并保留 !inside/!through。不能只用不穿内部代替向外条件。主层两个有正长度沿边的条件保持原样。route 的打印诊断应转为 None 空集/Some 精确物理 key 集合及相同几何回归；96/107 作为实测工作证据，无需额外锁死计步值。

当前诊断 1/1 只是成功运行并产出证据，其源码尚无违例集合/几何回归断言；最终测试迁移增量仍需复审并执行，不能将本裁决写成新 driver 已通过。

### 上游 main 476578a（#92）的证据复用边界

完整读取 `41d6b7d68544601f7a206af69c5b91286a2ed7c9..476578a74010cf37406f15fd82bd978298ad8248`：四文件 85+/5-，唯一生产逻辑改变是 `crates/rnl-edit/src/form.rs` 的 `Op::AddText`，有 key 时不再省略显式 halign=start、valign=baseline、rotate=0；另为测试、README 和 TypeScript 注释。它不改变 runtime 对既有 RNL 的解释、符号或文本测量。

复用依据不是仅无文件交叉。调用链核查：`rnl solve` 在 `crates/rnl-cli/src/solve.rs:534/542` 使用 `rnl_runtime::solution::write_record` 和 `rnl_view::finish_record`，后者直接调用 runtime 的 finish_record；`rnl-view` 无 `rnl-edit` 依赖。CLI 中 edit::write 只在单独 write 命令分支；wasm 的 write 请求独立调用它。现有 T6 profile、固定记录 evaluator 和基准输入只经 interpret/solver/runtime 写回，不经过新的 AddText 分支。固定文本/配置未重生成时，此上游改动不改变已核查的 solver 几何、计分或朝向 trace。

因此允许复用旧 main41 与 candidate08 的固定输入性能/质量证据及技术风险裁决，无须仅因这次 merge 再做 T6 专项长跑。保留原始 commit、binary、input/config hash 和实际测量时间，不能把旧测量改名为 main476 或合并后 binary 的实测；在验收说明注明调用路径等价的复用理由。合并后仍应核对 solver/runtime/render/config 与相关固定输入未变，按计划重建 CLI 绑定新 revision 并执行最终完整门禁。若冲突处理改变上述路径，则重新评估。使用新 AddText 导出而变化的文本、editor 往返、wasm write 行为不在此复用结论内；其正确性由更新后的相应测试和全门禁覆盖。


### candidate-09 与最终出口全集：迁移裁决

09 清单 SHA-256 `3c8156504accadedf1b55d87fa7770408ed2662e646d940c4581aa5bbde1bd5e` 的两个文件独立核对一致，且与合并后 e977116 的相应文件内容一致。完整阅读 OTA 新增 route 回归和 driver 增量：实际 net/terminal 与物理 key、精确违例向量、corner/horizontal-outward、!inside/!through/!along 均按先前裁决保留。`T-t3-final-boundary-01.log` 表明新 route 回归通过，driver 越过 T3 两层后在 T5 失败；因此 09 的 T3 修正闭环，但整个严格出口测试尚未通过。

独立核对验证交接 SHA-256 `da2b6e7129ce1b7975586e26e40f207ed6112691b3253ed9a7c06ee097441bf5`，以及 `exit-diagnostic-delta-02` 的 driver（`c39b5e9b2f5d59070dc8578d859bbed30c3464849eccbf1fa44ab153c66148b1`）和 ignored 诊断源码（`0a42302cced8b9dce4e75d556bc6e8076cfb8ac3c21f4156629aee985e302f56`）。诊断继续使用历史 fixed frame、真实 solved endpoints 和首非零段，没有修改严格断言。`fixed-exit-all-layers-01.json` SHA-256 为 `7cd19e4347bc9ef567469661bcdbcbedccc9b5ebaef750b5b282870372ddfe71`；独立用 Fraction 重算全部 9 层 18 条的 start/next inside、bounded-edge 正长交集和 through，与记录全部一致。不能把收集完成 1/1 当作回归通过。

新增 T5/T6 全部属于既已裁决的 Step 1 own-edge 出口提示近似，不新增生产 must-fix。允许迁移的范围如下；label 仅用于在这一固定 fixture/层中解析 object ID，必须连同物理 pin、terminal、net 检查，不能形成求解器或通用测试名字豁免。

| 固定文件及 file 层 | label | pin | terminal | net |
| --- | --- | --- | --- | --- |
| T5-inverting-amp-rlc.sp | #XA | 1 | inn | n1 |
| T5-inverting-amp-rlc.sp | #RF | 1 | n | vo |
| T5-inverting-amp-rlc.sp | #RL | 0 | p | n2 |
| T6-r2r-ladder.sp | #R2B5 | 0 | p | b5 |
| T6-r2r-ladder.sp | #RS1 | 0 | p | n1 |
| T6-r2r-ladder.sp | #RS1 | 1 | n | n2 |
| T6-r2r-ladder.sp | #RS2 | 1 | n | n3 |
| T6-r2r-ladder.sp | #RS3 | 1 | n | n4 |
| T6-r2r-ladder.sp | #RS5 | 0 | p | n5 |
| T6-r2r-ladder.sp | #RS5 | 1 | n | n6 |
| T6-r2r-ladder.sp | #RS6 | 1 | n | n7 |

必要断言：每层违例长度分别恰 3/8，完整 identity/terminal/net 集合等于表，不能用集合去重隐藏重复违例；fallback 列表空；原事实保持（本次分别 45/45、120/120）。每个首段轴向非零、固定坐标是自身 bbox 某边，变化轴上的 segment span 与实际有界边的交集严格为正长度，起止均不在内部且 !meets_inside。只与无限支撑线共线不充分，也不能套用 OTA corner-outward 分支。原 T3 主层两沿边和子层两角点条件保留；T1/T2/T2b/T5-opamp 仍要求空集，T4 原排除不扩大。无需针对已证明的同一有限 EXIT/ALONG 机制另做 T5/T6 长诊断，但修后完整严格回归必须执行。

### tiny T4：零长度身份线的检查器修正裁决

独立读取 `tiny-t4-owner-exemption-01.json`（SHA-256 `29afd729e3d8e11287a321c8cf2b807aaa55a4d4619073c542d72b415ca68afa`）、原始日志和只执行一次 solve 的诊断源码。inn 网确实只有 M2:g=(object1,pin0) 与 VIN:p=(object11,pin0)，坐标同为 (3,3)；二者均不严格在自己的框内部，recomputed exempt owners 为空。route4 仅一条具有两个不同 Anchor 身份的 [(3,3),(3,3)] 连接，无该网 fallback。无关的 VB 框为 [3/2,0,9/2,6]，点在其内部，但没有正长度线段穿体。先前 owner-exemption 假说被实际证据否定，不应为其增加豁免。

**接受仅 test-only 检查器修正，不新增生产 must-fix。** A03 明确要求“同一点上的不同端子：保留零长度连接，不按坐标去重”；main41 的 legacy `score.rs` 已对 windows(2) 使用 a!=b 过滤，08 的缓存与原 oracle 路径同样如此（当前 score.rs:164/299）。`route.rs` 仍按既有规则把单节点路径写成两个坐标相同但身份不同的端点。`driver.rs:498–505` 直接把所有相邻坐标送入 meets_inside；该谓词对内部零点为真，因而把身份连接误称为正长穿体。

最窄修正：只在“正长段穿过本体”检查中跳过相同坐标的相邻对；轴向断言和该 wire 的端点合法性、物理身份、路径坐标对应、分叉、连通/树等检查全部保留。不要跳过整根 wire，不按坐标合并端点，不改生产 meets_inside，不加 T4/VB/owner 特例，不伪造 soft/fallback 记录，也不调整预算。必须补两项小边界保障：两个不同 anchor 在无关框内部同点时保留一条双端零路径，hard 成功/parts=1/该网无 fallback；同一检查器仍拒绝无 owner 豁免、无 fallback 的非零穿体段。随后执行原 tiny-budget 完整回归，不能以单点取证代替。

本次 167.93 秒包含外部负载，不是隔离性能基准。hard=12、soft=200000 是每网独立预算；总 work=8 candidates/54156 expansions、chosen=10112 expansions 不能直接与 12 比较来判超预算。当前只裁定这次明确的零段误报；后续 tiny 完整执行或全语料若暴露不同失败，须另行分类，不被本结论豁免。原单点诊断与全集 collector 无需为本 test-only 修正重复；修正源码的静态复审、新小回归、严格出口和 tiny 完整回归仍待闭环，全套门禁仍未验收。


### driver-migration-final-review：已审实际增量与三项通过证据

清单 SHA-256 `6ac2c3fa70971923999519217c5d7645142ecc924686c17457b5193ceb0552dc`；三个副本、两份 before 和 tracked patch 哈希全部独立核对一致，before 与 e977116 对应文件一致。完整阅读 tracked-delta.patch 和新 ignored 诊断文件。T5/T6 只在正确文件/file 层匹配精确 object/pin/terminal/net；长度与完整集合双检，fallback 空、facts 全 kept，有限边段交集正长与不穿体检查正确。T3 两类和其他层保持原约束。zero-segment 仅替换 check_wires 的穿体谓词，所有 wire/endpoint/branch/连通检查保留。

新 route 小夹具在无关第三方框内部用两个不同 Anchor 身份同点，验证 hard 成功、一根零路径、parts=1、无 phase fallback；driver 小测试验证正长度横穿及从内部出发仍被正长穿体谓词识别。已读取 `T-migration-final-small-01.log/.json`：三项通过（含完整严格出口测试），exit=0、测试 12.71 秒。当前正在进行的完整 tiny 回归没有被视为通过。下述预算修复将改变生产时，不能把这些旧 binary 执行结果冒称为新生产的全套通过。

### R7 — P1：候选预算在 bootstrap 和候选派生路径上未被执行（必须在 Step 1 修复）

本条行号指 e977116 的生产源码：`search.rs:1441–1442` 只比较 made 与预算，`:1523–1530` 的 estimate 无条件 made+=1；`:1716` 和 `:1762` 的 `!next.is_empty` / `!tried.is_empty` 条件允许每节点第一个尝试越过耗尽额度；`:1746–1756` barycentre 准备未先检预算；`:2082–2107` P0、EvenRows twins 和 CURRENT 多 direction 派生无统一准入，最终只汇总 work。`driver.rs:133–151` 会实际摆放并布线这些额外输出，非只多报计数。

最小已有反例：tiny T4 的 candidates=0，却 work.candidates=8、4 placed/4 routed；其 JSON 和源码已独立核查。这个结论与 167.93 秒耗时、当前长测试是否完成无关，也不从 A* 总展开数推断每网超限。main41 已有 bootstrap 漏洞；但“旧代码已有”不等于公开契约准许。

依据交叉核对：main 与现生产 `search.rs:46–47` 明言 no more than Budget::candidates；`driver.rs:37` 明言 at most candidates 且 P0 在内；README 的 Budget 段只明确 P0 总是存在、额度耗尽保留已有结果；`lib.rs:264–275` 又明确 Work.candidates 是 Budget 所计的候选。v2.1 机制表对预算写“其余照旧”，应继承这项工作量承诺，不能解释为继承未声明越帽缺陷。v3 第 3 节要求按工作量计；Step 1 明确延期的是朝向前二、LP 等，没有每节点 seed 超帽例外。B01 的全八可达属于 Step 2，本裁决不要求提前实现它。

**裁决：R7 为 Step 1 must-fix，不能仅改文档增加任意 bootstrap 例外或标到 Step 2。** 为兼容合法的非负零选项与始终可解的 P0，最小明确边界是：N>=1 时把一个必要 P0 预留并计入 N，整个候选工作不超过 N；N=0 时只产生这一份必要 P0，Work 如实为 1，不运行额外 beam/bootstrap/barycentre/EvenRows/方向派生候选。这是一份最低兜底的例外，不是每节点例外。

修复应在新候选工作开始前统一准入：包含既有 Search::estimate 的实际尝试、额外 EvenRows 与 CURRENT frame 派生；额度耗尽时不再为未准入的新候选做朝向/barycentre 准备。不得仅在最终 Vec 截断、给 report clamp、去重后退款来掩盖已耗工作。已计费 Plan 转换为输出不能重复计费；确实重复执行一次 estimate 的尝试仍照实消费。一个已准入候选内部的有限八朝向全配置排序不自动变为本轮新设的八个预算单位，不能借此改变 top2、目标层序、其他 caps 或旧工作单位；边界和报告应写清楚。

最小保障：预算 0/1/2，深分组树，以及可产生 EvenRows 和多 CURRENT direction frame 的结构；同时检查实际输出候选数、Work.candidates、P0 存在、没有超帽的准备/派生尝试，以及保持事实、可写/身份连通。应证明默认预算未触帽的既有输入维持候选 trace/输出；触帽导致的预期变化单独解释并绑定新快照。当前旧 tiny 回归继续自然运行，无需为本审阅中断、重跑或改预算；之后由 root 调度修复候选的定向与必要全门禁。

### candidate-r7-budget-02：R7 关闭，完整验收仍待执行

本轮完整审阅 e977116 → R7-02 的生产增量及新测试。不可变清单 `target/objectives-v2-validation/candidate-r7-budget-02/manifest.json` SHA-256 为 `bf9ab30a472d7eb9a92ce8357d294fa0d64dfb5ac022b6b360a5c063341dfba9`；全部 7 个副本哈希及工作树同名文件均独立核对一致。生产行为变化仅在 search.rs；driver/options 是对应预算文档，profile 是 test-only 计数入口。未运行构建或求解。

R7-01 曾把全部后续 barycentre orders 的准备挂在已付首 order 上：平坦树 N=2 时虽最终只估计首 order，仍进行后续 orders 的完整朝向准备。本审阅指出其违反“额度耗尽不做未准入准备”，A 在 02 修复。以下行号指 02 冻结副本：`search.rs:1448` 统一 admit；`:1534` 所有普通 estimate 先准入；`:1754` 首 order 先准入；`:1774–1787` 后续 distinct orders 仅在剩余槽位内准备。此批次没有其他 counter 消费者，故预留数始终不超过 remaining；随后 `:1793` 对每个额外 order 正式计费。重复 bary cycle 立即结束，未制造额外候选；有限探测不另立新的预算单位。N=2 不再调用额外 orient/barycentre。

`search.rs:2092` 预留并计入一个 P0；节点与子组合不再有 !empty bootstrap 例外；邻域、反向关系均经过 admit。EvenRows `:2118` 和不同 CURRENT frame `:2139` 统一收费；base 已是第一个配置方向，其输出转换不重计。`:2149` 保留原 P0 code；实际重复 estimate 和已生成后再去重的派生不退款，`:2159` 如实累计 made。审阅未发现遗漏入口或通过末端截断/report clamp 掩盖工作。正预算总量 ≤ N；0 只保留必要 P0、work=1。内部已准入候选的有限八朝向评分、top2、路由/LP caps 均未改。

已独立读取 `step1-gates/T-r7-candidate-budget-02.log/.json`：3/3 通过，包括平坦树 N=0/1/2 的实际 orient=0/0/1、bary=0，深分组树/EvenRows/CURRENT 的工作量与 P0，以及真实 0.3 零长度双身份连接的 solve/write/read、全部事实保留。`even-current.log` 1/1 通过，覆盖不耗尽与多个限额下的 EvenRows/多 CURRENT frames。保存 test binary SHA-256 `a1a12865f5e020d064393e235bdb9d210c5a3ebc929b0260c75c692a9af92fcd` 已实算核对。这四项是定向证明，不代表完整 tiny 或全库通过。

默认未触帽对照独立复算，而非仅读取 comparison 的布尔结论：T1 对旧保存 binary `4ace8f7c…`，10 次完整八朝向 trace、5 个候选的 code/精确层、routed 记录、选择、work=80/448/7/132 全相等。T6 对 08 `cached-pure-stage-profile.json`，544 次 trace、10 个候选、4 个 routed 记录、选择、work=2993/8943/71/177821 全相等；输入、旧 profile、配置哈希均重新核对。新 T6 profile SHA-256 `d46009ed384d84292db660826c030c3ffbf4b901ae6d79b7fc0e52a8c42cbd73`，stage 总 92.388 秒/search 80.557 秒；这是一轮实际运行，不替换原隔离性能证据。routed 字段记录分数/expansions/fallback，不含完整路径几何，不扩称 T6 解字节相等。

另独立核短 T1 真 CLI：旧 08 与 R7 两轮 exit=0，完整解文件逐字节相同，SHA-256 `2cf8b533512ab963772f227d4b7bf28c90a36e1221aff4a8b32ad9c5c66f46c5`；完整日志仅 revision、time、输出路径不同，scoring/facts/work/hash 相同。R7 CLI SHA-256 `094b63385906eba024dd0388364c07c25e4a4e86a9858a19998e5bb3a5420cd5`；`B-r7-final-cli-01` 构建 exit=0。618 文件清单 SHA-256 `0d7f6c36d9851fbd13099a2fa73c9d6110a413066dab62d5ebbb32b0f7731546` 已核，其生产/测试/config/Cargo 文件无漂移；核查时仅本审阅报告及 R7 说明文档因补录本轮证据变化，不影响构建输入。

**本轮技术结论：R7 关闭，目前无剩余生产 must-fix。** 既有 tiny 长轮后来由 root 为 R7 修复主动中断并留存证据，不算通过或超时失败；需按新快照完成原完整 tiny、最终语料与全套门禁。触帽输入的池、工作量或布局预期可能变化，应按新生产实测并解释；未触帽的 T1/T6 证据只按上述字段与来源复用。不得据此次短轮取消最终验证、宣称性能问题已消失，或把旧 08 长 CLI 测量改名为 R7 实测。

### 后续窄审：0.2 升级的输入 provenance（源码通过，执行待补）

独立核查 `target/objectives-v2-validation/cli-upgrade-provenance-02/manifest.json`，SHA-256 `40085796320a3b746f195cb7eaf09247583cc36e6bf7a8582d359b30a4ca5773`，两个源码/测试副本哈希全部一致。新发现的 P2 根因是 `rnl-cli/src/solve.rs:409` 附近的升级流程只恢复第一 pass 的 run.read；第二 pass 的 memory overlay 绕过 Noting，导致实际升级后的 invocation.inputs 可能为空。既有 R7 T1 的 `input_manifest=[]` 不能作为“无依赖”或完整来源证书。

冻结修复保存第一 pass invocation；第二 pass 完成后追加仅在第二 pass 首次读取的依赖 path，同 path 保留第一 pass 的原 Provider 解码文本 SHA，再恢复 invocation。静态审阅通过：它不改变 solver 的输入、options、候选、记录、读文件列表或排序，未引入重新求解。新真实 CLI 回归包含 0.2 entry 的 UTF-8 BOM、include、两个原始解码文本 SHA、实际升级到 0.3 的断言，针对根因；本轮没有执行该回归。

证据复用边界：已有几何、逐项评分和工作量仍可按旧 source/binary 标签引用；旧空 manifest 不可冒充完整依赖绑定。修后 CLI 需要新 source/build/binary 绑定及短 T1 真 CLI 的 solved 字节、非 provenance 评分/work 对照。旧用时仍只属于旧 binary，不改名为修后 CLI 测量；没有理由仅因这处 metadata 修复再跑长 T6。P2 的源码闭环不等于执行或最终全门禁闭环。

root 后续通知：corrected tiny 已自然 exit=0，九层功能回归通过，总 987.07 秒，T6 两解与 checker 838.099 秒；T14 的旧 R7 0.2 T6 真 solve 179.370236 秒、solved SHA `bb59c4ea…` 与 08 相同，来源仍 `0b5afb35/f510495f`。这是执行方通知，尚待本审阅下一轮独立读取完整绑定/日志后正式更新验收，不替代此前对 profile 不含真实路径的限制，也不充当新 metadata CLI 的计时。高耗时风险和最终 corpus/full gates 仍未验收。

### R8 关闭：升级 provenance 的执行与绑定复核

上述 provenance 修复正式编号 R8，提交 `c9adffd2403226ce0bad3cab4780e82e81f025fa`。已独立读取 `R8-T-cli-provenance-01.log`，专用真实 CLI BOM/include 测试 1/1 通过（编译 10.21 秒、测试 1.33 秒）。`target/objectives-v2-validation/step1-r8-c9adffd2` 的 618 文件 source archive 逐文件哈希全部与 manifest 相符；manifest SHA `c2287c706535e60e051446899c0cdc9c4c977d488f3ca14d0226903d5b640ece`、archive SHA `7ed3bb5cff5c1e972241d7420da6f181c41a5a1b9647e118090a792cca10f334`、CLI SHA `91c3e4e8f822a2d996464d4548178a158527b015cef648b8f4c4a55aab81bf40`、config SHA `a1228faa0bc8d244fba04b5d161c8248a1922c2bbab1df0e26428852de210045` 均实算核对。

短 T1 真实旧/新 CLI 均 exit=0；解文件逐字节相同，SHA `2cf8b533512ab963772f227d4b7bf28c90a36e1221aff4a8b32ad9c5c66f46c5`。独立比较两份完整 24 行 stdout，差异仅 revision、input manifest 和 wrote 路径，facts/scoring/work/hash 一致，stderr 相同且空。新 entry manifest 的解码文本 SHA 按实际 UTF-8/BOM 解码、保留 CRLF 实算正确；未把 Python 普通文本读取的换行归一化误认为 Provider 的解码行为。

**R8 已关闭。** 源码作用域与该短轮支持复用 T14 旧 `0b5afb35/f510495f` 的 solver 几何/逐项质量证据；所有旧耗时仍是旧 binary 实测，不能重新标为 R8 CLI 的新测量。T14 的质量变化解释和最终完整门禁/语料/视觉验收不因 R8 关闭而自动通过。


### T14 九层质量退化裁决：允许限定 14 份快照迁移

本轮只读审阅了 `notes/solver-design/objectives-v2-step1-t14-quality-analysis.md` 与 finalpass-03/04 原始 profile、绑定文件、原始/输出 RNL 和 T14 stdout。`t14-quality-finalpass-03/A-quality-localisation.json` SHA 为 `77532fb7abaa9c578f40bb5f1dc4e65359f4e9e1ecd1aa139748d4ba21ab06f4`，04 对应 SHA 为 `698ba64e1950634f6d0157a8f61d4ebd98d9f56473ffabdc21b495c4dad6b455`，均实算相符。没有执行新的求解或构建。

**结论：九层退化的解释缺口已关闭，允许验证方受控迁移已列明的 14 份求解快照。没有由这批证据发现新的 production must-fix。** 依据 v3 第 125、170 行，Step1 明确保留前二朝向截断；第 179、322 行要求逐处解释变化并检查快照，而没有要求该有限搜索一定胜过旧解。本结论依据具体可达性和排序证据，不以“旧算法近似”泛化豁免，也不把面积改善当作高优先级退化的补偿。

独立核查包括：

- 03 的 raw T2/T4 和 04 的 raw T2b/ota、0.3 T2/T2b/ota/T6：共 731 个实际八向数组，逐个用 Python Fraction 对完整层向量及稳定顺序重排，核对实际 rank 与前二；每例所有 placed 顺序、四个 routed 候选的完整层/code 顺序、最终 chosen 均一致。没有只引用分析 JSON 的布尔自报。
- 每例 profile SHA、原 entry/config SHA、实际 chosen-file 与保存 T14 的逐字节输出、Host 读回 record SHA 实算匹配。逐层从原 T14 stdout 重新取完整 ScoreReport、work、sig3 和 kept fact 序列，均匹配该诊断。raw T4 已使用 CLI `--report` 实际启用的 `hints_for_low=true`，没有复用此前不匹配的 helper 工作量。
- 检查仪器源码：搜索/摆放/布线来自原始 entry 或已绑定的显式 0.2 升级 overlay；旧记录只在完成候选选择之后用于对照/原帧合法性测试。旧 frame 新增 PLACE/ORIENT facts 后，原 retained facts 序列必须完全不变且所有新增 frame facts 被保留。03 两例原事实/frame 数为 13/27、38/76；04 六例分别为 14/27、16/46、18/27、19/27、20/38、27/102。
- 两版 ota 只重执行所指子层；其它层保存记录作为事后输出载体。目标完整身份 record/ScoreReport/work/sig3 的严格匹配有效，但载体文件相同不能说其它层在本轮重求解。
- 每例旧完整朝向向量在所有实际 placed 候选中缺席。raw T2/T2b 的旧 VIN MY 与 RD R0 在实际八向评价中未进前二；raw T4 的 VIP/VIN MY 均在全部相关调用中排第八。raw ota 的旧 PMOS 类根 MY 排第三；0.3 ota 排第七。0.3 T6 的 VB0 MY 在 17 次调用中排第四或第六，未进前二。
- **0.3 T2/T2b 必须保留联合可达性的准确表述**：旧 VIN MXR90 在相关调用中排第二，可单独进入 beam；旧 RD R0 和 M1 R0 分别排第五、第三，而 P0 的 VIN 为 R0。因此旧完整向量仍不在实际池中，不能写成“所有旧 MY 都被截断”或“任何旧单体朝向都不可达”。
- raw T6 继续复用此前已独立审查的 544 次 trace、旧 18 facts+102 frame facts 以及旧整体向量缺席证据，保留原 source/binary 标签。不能把这项或 T14 旧用时重标为 R8 新 CLI 实测。

新池中的低弯数候选在更早的 body/short/label 层落败，完整排序本身正确。这证明具体的 Step1 前二/有限 sketch 候选边界，既不证明全局最优，也不证明只把前二改为八向就一定恢复旧坐标或路由。原逐项表中 18 层的九好九坏、各层首差、低优先级反向变化以及高耗时风险必须继续公开。

raw T4 新增一条输出线和一个分叉的原因也独立核对：读取旧/新 `&out @WIRE` 的显式端点（不按坐标合并），旧四条边连 CC:n/M6:d/CL:p/M7:d 与单一 rt7，degree=4；新五条边连同四个物理端子与 rt7/rt8，后两者各 degree=3。独立重算两侧均连通且 E=V-1，未新增或遗漏端子。该结论只解释此 net 的输出树划分，不能外推 Step2 身份计分。

迁移限制：仅此 14 个已解释文件；保留原事实、身份树、作者线、端点及几何保障，不以泛化豁免换快照通过。完成 bless 后必须运行正常非 bless 快照检查。Step1 全门禁、20 电路语料及最终整体验收仍未完成；本轮质量裁决没有宣称它们通过。质量文档开头仍有早期“本轮尚未完成”的历史状态，建议在最终验收整理时明确最新状态，保留失败历史但避免读者误判当前结论。

附：Step3 平台资源冒烟边界已单独裁定。v3 第 177、202 行要求无配置文件时内置回退及开发/安装/portable/内置各自冒烟，没有 GNU 必须单 exe 的要求。已证依赖 WebView2Loader.dll 的 GNU 可用“exe + 明确列出并绑定 SHA 的加载依赖，目录内无 solver config”的实际发行闭包；不得靠 PATH 偶然 DLL 冒充单 exe。必须记录目录、PE imports、resource_dir 并真实求解检查 builtin/missing/error 语义。MSVC 单 exe 仅在已有可用且已证静态产物时测试；本轮没有执行平台冒烟，也没有审 Step3 API 全部实现。
