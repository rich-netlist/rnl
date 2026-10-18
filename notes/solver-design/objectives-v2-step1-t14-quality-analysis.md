# Step1 T14 质量分析（只读证据检查点）

日期：2026-10-18。分析代理 B。**本轮尚未完成 T2/T4 实际候选池原因证明，不构成质量验收或 bless。** root 要求本检查点先交仪器、释放槽位，后续拿到匹配的 trace 再继续。

## 来源和比较口径

- 旧解：main `41d6b7d68544601f7a206af69c5b91286a2ed7c9`，CLI SHA-256 `fdab9f30fe8fb6258e855fecacc43b3a3168a863c066cf7e2e4816482b6d6cdd`。
- 新解实际 T14：`0b5afb352ae5a42e4744b77184e20f3ab71c311e`，保存 CLI `f510495fe92f2a95ae4c5db4cd377948fac4ad3d395a028ec5968893725e4cb6`。
- 当前 HEAD `c9adffd2403226ce0bad3cab4780e82e81f025fa` 的 R8 是 metadata 修复。Astra 允许复用上述几何/计分证据；旧耗时仍属于旧 binary，不能改标为 R8 执行。
- 主证据：`target/objectives-v2-validation/step1-r7-0b5afb35/comparison-t14/comparison.json`、`all-terms.csv`、`comparison.md`，18 层、252 项配对。每项包含旧/新 fixed-score、输出、report、SVG 路径。
- 图证据：同目录 `svg-pairs/manifest.json` 和 `visual-review-t14.{json,md}`。validation 已逐看 18 对图；本轮 B 直接查看了 raw0.2 T2/T4/T3.ota5t 及 0.3 T2 并排 PNG，其余视觉描述引用 validation 的逐图观察，不冒称 B 已逐图查看。
- 同配置 digest 为 `22233424deb2d8ec`，所有目标级容差均为 0，`time_limit=null`。以下 L 索引从 **0** 起。

默认优先级为 L0 符号框重叠，L1 非本网引脚短接，L2 线穿符号框，L3 标签重叠，L4 CURRENT，L5 turns，L6 symbol_direction，L7 group alignment，L8 group distribution，L9 area（不含线），L10 crossings，L11 aspect，L12 length。本组 CURRENT 没有合格的作者标记端子，不能用它解释行层变化。

Stage1 按 v3 能力表仍用旧几何定义：标签项是原 texts-over 近似，turns 是输出折线拐点，crossings 只跨不同网络，area 是元件框。这里的“精确”指这些已声明定义下的有理数计分，不把它冒充 Step2 新定义或视觉美观的完整度量。

## 本轮已复核的共同事实

1. 14 个文件的 before/after `solve-report.stdout` 中，全部 `kept [n]` / `discarded [n]` 行逐字相同：身份编号、事实内容、优先级、来源、冲突说明均相同。T4 两版本各丢 3 条，其余全保留；不是为新质量结果另丢事实。
2. 18 层新解的 live `scoring` JSON 与固定记录重算的 `after_scoring` **整个 ScoreReport**（含配置、参数、raw/weighted、estimated/inactive、容差及诊断）逐项一致；每层恰有一个对应 live report。未发现新解计分在写回前后漂移。
3. 28 次 before/after solved-check 均 exit 0、0 errors、0 warnings。T4 的 dropped/ok 汇总及 hints 两侧也相同。这是既有 checker 证据，不替代把完整旧 frame 固定后重做 retained-facts 一致性证明。
4. 两侧 18 层固定评分 coverage 都 complete、missing 为空，但其证书只保证 Stage1 所需的坐标/朝向可由记录或作者关系确定。它**不证明**完整旧解加入当前求解仍合法，也不证明生成导线所有端点身份/拓扑齐备。
5. `tests/unit/current.rs:378` 的固定记录 helper 明说 PreviousWire 只有输出坐标，生成顶点被赋独立 Bend 身份，未按坐标造连接。该报告可用于 Stage1 现有评分，不可据此声称已验证 Step2 turns/crossings/zero-link 身份。

下表逐层列 facts 保留数/总数与两侧 coverage 的已证对象数。coverage 的排除对象不是漏记录：raw0.2 ota/T4 是无 Stage1 计分图形的 `%ax`、`%mid`，0.3 ota/T4 是 `%ax`；其余无排除。若要证明这些点与作者关系的完整合法性，仍需对应 frame/facts 检验。

| 层 | facts kept/total（旧=新） | coverage 对象数（旧=新） | 排除数 |
|---|---:|---:|---:|
| 0.2 T1 | 5/5 | 11 | 0 |
| 0.2 T2 | 13/13 | 19 | 0 |
| 0.2 T2b | 14/14 | 19 | 0 |
| 0.2 T3 main | 17/17 | 27 | 0 |
| 0.2 T3 ota5t | 16/16 | 32 | 2 |
| 0.2 T4 | 38/41 | 54 | 2 |
| 0.2 T5 main | 9/9 | 26 | 0 |
| 0.2 T5 opamp | 2/2 | 10 | 0 |
| 0.2 T6 | 18/18 | 83 | 0 |
| 0.3 T1 | 7/7 | 11 | 0 |
| 0.3 T2 | 18/18 | 19 | 0 |
| 0.3 T2b | 19/19 | 19 | 0 |
| 0.3 T3 main | 25/25 | 27 | 0 |
| 0.3 T3 ota5t | 20/20 | 29 | 1 |
| 0.3 T4 | 50/53 | 54 | 1 |
| 0.3 T5 main | 13/13 | 26 | 0 |
| 0.3 T5 opamp | 3/3 | 10 | 0 |
| 0.3 T6 | 27/27 | 83 | 0 |

## 18 层首差及实际变化

“胜”只是在相同当前配置下对这两份完成解的比较，不代表已证明新解在它实际有限池里排名正确。P0 是报告中的 netlist-order 来源。`5→5/4` 表示旧 placed 5、新 placed 5、新 routed 4；不是候选预算的实际工作计数。

| 层 | 第一决定项，旧→新 | 两解比较 | 其它具体变化 | placed 旧→新 / 新 routed；新来源 |
|---|---|---|---|---|
| 0.2 T1 | L5 turns 2→1 | 新胜 | length 13→10；area 3441/20→190 变差但优先级较低。VIN 横放，C 保持竖放 | 5→5/4；beam |
| 0.2 T2 | L9 area 4617/20→2967/10 | 旧胜 | 前九层全相等；turns=1、length=18 均不变。框宽高 9.5×24.3→21.5×13.8 | 5→5/4；beam |
| 0.2 T2b | L9 area 4617/20→10951/20 | 旧胜 | turns=1，length 18→23；框宽高变为 23.5×23.3 | 5→5/4；P0 |
| 0.2 T3 main | L5 turns 7→6 | 新胜 | area 17253/20→3633/4、length 87→114 变差；源横排、三角形移右是可见代价 | 10→9/4；P0 |
| 0.2 T3 ota5t | L5 turns 3→8 | 旧胜 | area 2916/5→3926/5，crossings 0→2，length 45→73，生成线仍14、branch仍4 | 5→5/4；P0 |
| 0.2 T4 | L3 labels 0→5 | 旧胜 | turns 15→27，crossings 4→13，length 234→372；area 49491/20→30523/20 改善不能抵销 L3 | 5→10/4；P0 |
| 0.2 T5 main | L5 turns 10→8 | 新胜 | length 77→114、area 1443/2→4641/5 变差；RF 大反馈环、RI/L1 旋转 | 5→5/4；P0 |
| 0.2 T5 opamp | L5 turns 1→0 | 新胜 | area 2541/20 不变，length 14→8；out 连接更直 | 4→5/4；P0 |
| 0.2 T6 | L5 turns 33→59 | 旧胜 | crossings 3→64、length 332→1266；area 22638/5→31749/20 改善不抵销 turns | 6→10/4；P0 |
| 0.3 T1 | L9 area 18369/100→8463/50 | 新胜 | turns=2，length 17→23；R1 竖放，输出绕右上 | 5→5/4；P0 |
| 0.3 T2 | L5 turns 3→4 | 旧胜 | length 48→30，area 24651/50→14198/25；缩短长度不能解释高层 turn 损失 | 5→5/4；P0 |
| 0.3 T2b | L5 turns 3→4 | 旧胜 | 同 T2 的分数/形状；作者 VIN.x=-4 仍满足，不是丢 drag 事实 | 5→5/4；P0 |
| 0.3 T3 main | L1 shorts 2→0 | 新胜 | L2 穿框 3→0；turns 5→8、length 108→116 是低层退化，crossings 2→0 | 10→7/4；P0 |
| 0.3 T3 ota5t | L5 turns 9→10 | 旧胜 | area 22253/25→16473/25，length 80→95；面积改善不能解释高层 turn 损失 | 5→5/4；P0 |
| 0.3 T4 | L2 穿框 1→0 | 新胜 | labels 0→2、turns 18→24、crossings 3→8、length 388→397 仍真实变差；area 212197/50→84501/50 | 10→10/4；P0 |
| 0.3 T5 main | L1 shorts 1→0 | 新胜 | labels 0→1、area 78537/100→48323/50 变差；turns 7→6、crossings 2→1、length 94→78 改善 | 10→5/4；beam |
| 0.3 T5 opamp | L5 turns 7→6 | 新胜 | area 44541/200→11457/100、length 49→28 均改善；inp/inn 与菱形分开 | 10→5/4；P0 |
| 0.3 T6 | L5 turns 36→57 | 旧胜 | crossings 14→50、length 579→1244；area 221259/100→39221/25 改善不能抵销 turns | 10→10/4；P0 |

合计 9 层改善、9 层退化。改善的“为什么赢”由上表首差直接证明；“为什么生成了这个布局/旧优解在哪一步没留下”仍是不同的问题。

## 重点几何解释与因果边界

### raw0.2 T2/T2b

T2 旧记录是 M1=(7,-8)/R0、RD=(7,0)/R0、VIN=(0,-12)/MY、VDD=(0,0)/R0；新记录是 M1=(3,-3)/R0、RD=(3,2)/R90、VIN=(-5,-3)/R270、VDD=(-14,-3)/R0。旧 `in` 线带一个弯、新 `out` 线带一个弯：弯的位置换网，合计 turns 和长度不变，但横向元件/地符号排布增宽，确切 area 从 230.85 增至 296.7。改善 aspect 是 L11，不能抵销先出现的 L9 area 损失。

T2b 新选 P0，M1=(9,4)、RD=(9,11)、VIN=(-4,0)、VDD=(-12,0)，全部 R0；它与 T2 并非只是同一个新解平移。输入的旧 `generated` sig1 frame 可被替换，作者另写的 **VIN.x=-4/high 是保留的事实**。0.3 T2b 也逐字保留该作者记录，新 solver PLACE(-4,0) 满足它；图像像 T2 不能推成作者 drag 丢失。

旧源 `search::domain` 曾以 SourcesDown/GroundsDown、两端元件 rail、stub_on_body 等筛朝向，`ranked_values` 用 facing/texts_inwards/preference；当前 `domain` 返回 all8，排序读完整配置。两套机制确实不同，而且 T2 的 RD/VIN 实际朝向改变；**尚不能据此证明旧 RD=R0/VIN=MY 被当前某次 top2 截断**。必须查这个 case 的真实 sketch、8 值分数和实际保留分支。T2/T4 最小 trace 已交 validation。

### raw0.2 T3 ota5t 与 0.3 T3 ota5t

两版旧 PMOS M3/M4 是 MY/R0，新是 R0/MY，NMOS 对保持 R0/MY。raw0.2 旧 inn 是 M2:g→port 直线，新 inn 明写 `(8,-3),(13,-3)` 两个拐点；新 x 网 M4:g 支路明写 `(7,10),(-7,10)`，旧对应支路较直接。新 out 从 M4:d 绕 `(8,5),(8,4)`，端口移到内侧。tail 部分减少弯，抵不掉 inn/x/out 增量，最终真实总计 3→8。

0.3 ota 的 PMOS 朝向同样反转，新 out 的两条支路分别增加横向绕行，tail 写出的路径也变了；生成线仍14、branch仍4，输出总线数17不变，最终 turns 9→10。不能仅看“镜像关系仍对”就认定路径同优，也不能用更小的面积解释它。具体是旧朝向未进 top2、placed LP 改了端口距离，还是路由在同一个 frame 上改差，现有 report 无法分离；待对应最小 trace，不借用 raw0.2 T6 的证明。

### raw0.2 T4

新图的中心 reference/value 确实拥挤，与 Stage1 labels 0→5 一致；宽高由 58.5×42.3 变为 66.5×23.3，符号横排和矩形反馈环使图更扁。上表 L3 先失败，不能归为“牺牲低级文字换面积”。

作者冲突裁决两侧相同：丢 [7] M6/M7 的 x 对齐（与最小 x 距离 [3] 冲突），丢 [16] M1/M2 的 y 对齐（与 M1.y=0、M2.y=3 冲突），丢 [40] VIP/VIN 的 y 对齐（与固定 y 间距 [2] 冲突）。新解 notes 说丢的是求解器自己提出的 1 row alignment、1 行内 gap、14 行间 gap，不能混成丢作者事实。

总导线33→34来自 ROUTE 生成线22→23、branch7→8；它可能来自连接树划分变化，数量本身不证明错连或漏线。需要逐网 identity/tree trace 后解释新增 wire 的两端及首次连接原因；不按坐标或视觉触碰猜拓扑。新 chosen 展开21825格、总79378 A* expansions，也不能凭“搜索更多”推成预算耗尽或全局最优。

### 0.3 T2/T2b

旧 VIN 为 MXR90，新为 R0。旧输入线 `VIN:p→(-6,-7)→M1:g` 一个弯；新 `VIN:p→(-5,2)→(-5,3)→M1:g` 两个弯。out 两侧都是两个弯，故合计3→4，长度48→30。这已定位到具体输入网几何，而不是泛称整体布局变化。尚未证明旧更少弯 frame 在当前 finite pool 的缺席位置；T2b 还应单独检查作者固定 x 对 candidate/LP 的作用，不能以 T2 的未来 trace 自动覆盖。

### 0.3 T6

现有图/report证明梯形变为长公共行，多条跨级长线；turns36→57、crossings14→50、length579→1244，生成32线/8branch保持。它的作者 facts 是27个、标准包0.3，不能套用 raw0.2 T6 的18 facts/102 frame-fixings/544 trace 结论。最终池排名、旧 frame 合法性与具体截断点均待本例独立证据。

## 最终池 rank：已证与未证

当前源 `driver.rs:134–157` 先 placed 计分、用配置取 route_beam，路由后重算真分再 `active.rank`；`objectives.rs:217` 在本组全零容差时直接比较完整 Q 层数组，再 code、原索引。源代码未见 area 跨过 labels/turns 的排序路径。但源码检查不能代替本次实际池复算。

除下述 raw0.2 T6 外，现有 T14 report 只保留候选数量、chosen 来源及 chosen 分数，没有每个实际 placed/routed 的层数组/code。因此其余17层的最终池 rank **未独立复核**；旧完成解比新完成解优，也不能单独证明 rank bug，因为旧解未必在这个池内。相反，在找到真实池之前，也不能断言它一定因 top2 被丢弃。

raw0.2 T6 既有闭环见 `notes/reviews/objectives-v2-step1-adversarial-review-2026-10-18.md:220` 及 `performance-integer-hard/{quality-localisation.json,cached-pure-stage-profile.json}`：旧102 frame fixings全部保留，原18 facts保留集不变；10placed/4routed独立排序，chosen8正确；544真实8朝向trace全部独立复排一致；旧 VB0/2/3/4/7 的 MY 各16次均零基rank7，未进top2，P0为R0。这证明**本例**旧整朝向缺席有限池，Astra已接受为声明近似边界。本轮不重做、不外推。

## 声明近似不能直接当作个案归因

- v3 Step1 明留 top2 和 LP 固定四级。`coords.rs:427` 的四级依次是网 HPWL、元件 envelope、仅stub网的extent、到resolution witness/文字anchor的L1；它并不优化高层 label-overlaps 或 turns，也不精确最小化乘积 area。
- “LP没直接优化标签”能解释为何单候选可能挤，但不能解释为何最终池明有低标签解却不选；要先看实际池。
- “all8完整配置排序后只留top2”是可导致旧朝向缺席的机制；只有真实 trace 指定某类、某次 sketch、旧值排名才是个案原因。
- 去掉按内置名推 transistor/current/ground/port 的规则是已授权行为变更；不能仅看名称或本层无CURRENT就推定其导致这次退化。需比较旧/新提议的 rows/classes/attachments，并追到被丢的具体 frame/关系。
- A02 只承诺相同 Layout/顺序/硬参数/成本且全硬成功时兼容旧 NetOut；不承诺不同配置/朝向/摆放的整条记录相同。A04 局部有限权重不是最终字典序的精确路径优化。若最终池正确而同frame路由退化，仍要以具体 hard/soft/fallback、障碍及 wire 路径证据解释。

## 两例最小仪器交接（未执行）

本轮 root 扩大权限只给 test-only `crates/rnl-solver/tests/unit/profile.rs`；A确认单写，production不改。**最新冻结 SHA-256：`d825c48367f8f44ce66204fc482ba580b055e2edfe2a2ade4f153b909426444c`**。最新修正仅 `git diff --check` 通过，尚未编译执行；B 未运行 Cargo、rustc、native solve 或 build。validation此前旧 helper raw T2探索已自然exit0，标为探索；它碰巧score/work相同，不构成两pass/geometry证明。

新增可选参数复用 runtime `pins_std`、`raise_version` 和原entry的 MemoryProvider/Layered，重现已由真实CLI报告确认的**唯一self-contained主file-level**升级。它记录 original raw/decoded SHA、raised overlay SHA和文本、首读problem sig3、最终problem sig3、configSHA，明确 `first_solve_reexecuted=false`。不声称真的重跑了firstsolve，不从solved文件猜输入，也不推成所有0.2/多文件/多层通用等价。

从profile实际chosen `(placed,routing)` 按driver同分支转换writer Solution，调用真实 `write_record`/`finish_record`/`replace_generated_with_header`，严格断言：T14 final sig3相同、完整身份record bytes相同、整个输出file bytes相同、整个ScoreReport相同、工作行相同。先写 `.binding.json`、`.chosen-record.rnl`、`.chosen-file.sp`，任一失配即停，不能用partial字段凑通过。另保存所有placed origins/orientations/LP状态/dropped ownrelations、8朝向trace、4routed code与真实identity/writer几何。

执行仅由validation持统一令牌、jobs4和完整资源门禁。cwd=`C:/Users/jielu/.codex/worktrees/cdac/rnl`，依次T2→T4，先T2全部matching通过才扩T4。每个file环境如下（路径均绝对展开；输出目录先由validation建立）：

```text
RNL_PROFILE_INPUT=<cwd>/target/objectives-v2-validation/inputs/t-0.2/<file>
RNL_PROFILE_CONFIG=<cwd>/target/objectives-v2-validation/step1-r7-0b5afb35/bin/rnl-solver.toml
RNL_PROFILE_OUTPUT=<本轮独立日志目录>/<stem>-stage-profile.json
RNL_PROFILE_RAISE_FILE_LEVEL=0
RNL_PROFILE_EXPECTED=<cwd>/target/objectives-v2-validation/step1-r7-0b5afb35/results/t-0.2/<file>/solved/<file>
RNL_PROFILE_EXPECTED_REPORT=<cwd>/target/objectives-v2-validation/step1-r7-0b5afb35/results/t-0.2/<file>/solve-report.stdout
RNL_QUALITY_OLD=<cwd>/target/objectives-v2-validation/baseline-main-41d6b7d/results/t-0.2/<file>/solved/<file>
RNL_PROFILE_SOURCE=<实际HEAD+仪器SHA和source archive标签>
RNL_PROFILE_ORACLE unset
cargo test -p rnl-solver --lib test_profile::isolated_stage_profile -- --exact --ignored --nocapture
```

`file`分别为 `T2-common-source.sp`、`T4-miller-conflicts.sp`。T2期望5placed/4routed、beam、chosen expansions24，work=233/1430/13/2347；T4期望10/4、P0、chosen expansions21825，work=4028/15080/83/79378（依次resolve/pivots/candidates/A*）。完整score必须同时匹配上文与原JSON，非仅这几个计数。

### 第一轮严格匹配失败的具体诊断与窄修正

validation已运行旧仪器 SHA `80c9dc3b…`，`step1-gates/Q-T2-finalpass-r8-01.log`：compile6.73秒、test1.02秒，exit1；未扩T4。产物保存在 `target/objectives-v2-validation/t14-quality-finalpass-01/`，原失败日志不删除。

失败是仪器两侧比较了不同表示：裸writer record长度922；SPICE host读取的Record.text长度939。首差零基字节116，分别是 `\n  %gnd_stub` 与 `\n   %gnd_stub`。`rnl-host-spice/src/lib.rs:351` 只去 `*`，继续行仍保留carrier后的1个空格。B只读重建该提取，实算SHA `6964760d60e0a64011c01bbeb1f936375d5a3e88a2be803c980a1e2013f74757`，与失败binding的expected_record_sha256完全相同。

同一失败产物 `.binding.json` 中，**整个chosen-file bytes已经相同**：actual/expected SHA均为 `c3305421ed2f24c1569073b49a480d89e7072e05c4bce0036dbce21226cf2fd3`；full ScoreReport、work、final sig3也相同。这不是求解差异或端点身份bug。但测试在断言处终止，完整pool JSON及old-frame检验尚未跑到，不能据这些partial产物完成rank/旧解合法性归因。

root确认的窄修正（最新冻结SHA见上）把actual整个chosen-file也经同一 `rnl_view::interpret` + MemoryProvider/Layered读取，再严格比较两侧Record.text；不trim、不归并坐标、不删身份字段。完整file bytes、ScoreReport、work、sig3断言全部保留。额外保存 `.chosen-host-record.rnl` 和 bare_writer_record_sha256，便于核对载体转换。validation下一次只重跑T2，全断言通过才T4；尚无本修正版执行结果。

拿到匹配trace后先独立Fraction重排placed/routed并确认chosen，检查旧frame全部fixings与retained facts，再按旧朝向在每次真实rank的位置确认top2是否足够解释；若旧朝向在pool却坐标不同，则查具体LP关系/四级/取整，若同frame已入routed池则查逐网路径。两例闭环后才决定其余重点的最小追加包，不预先盲扩长跑。

## 当前结论分类及交接

- **已解释**：18层谁先赢/输及高优先级、14文件事实裁决不变、18层新live/fixed全ScoreReport一致；T2b作者x=-4未丢；重点路径/朝向及数量变化已定位；raw0.2 T6沿用已审闭环的特定top2边界。
- **待证**：T2仪器首版已编译且定位载体空格比较不对称，最新同Host比较修正版未执行、T4未执行；其余17层实际最终池rank、除既证T6外旧frame完整facts合法性、旧优解具体缺席阶段及同frame路由原因。一般coverage/图像/数量不能补这些证明。其余重点trace待两例结果后再申请。
- **必须修复**：本轮尚未发现可证的新增生产排序/定义bug，不以质量退化直接判bug，也不把待证标成已接受。若matching失败或独立实际池重排发现更优项被跳过，立即报A修复；不能改测试标准、改优先级、bless或用低层面积解释高层损失。

profile.rs已冻结交validation编译，必要纯仪器类型修正由A/validation通知B或A单写接管；不要并写。B本轮无活动工具session，按root安排暂时结束释放槽，等T2/T4证据后恢复分析。本稿仅改本文和上述test-only仪器，未改生产、作者输入、既有预算或断言。


## A 对 finalpass03 两例的技术归因（待独立最终复核）

正式证据为 `t14-quality-finalpass-03/{T2,T4}-stage-profile.json` 与 `A-quality-localisation.json`。CLI --report 会在读取 TOML 后强制 hints_for_low=true；仪器现显式用 RNL_PROFILE_CLI_REPORT=true 对齐。前一 T4 helper 少了这项，仅工作量少94 nodes/41 pivots，已保留失败且未硬补计数。03两例均完整 input/config/sig3、输出file/Host identity record、ScoreReport、work严格一致。

T2：Fraction逐项重排36次实际8向数组及5placed/4routed，最终顺序[1,4,3,2]，chosen1正确。候选1与P0(4)前九层相等，面积2967/10胜10951/20；另两个routed在更高层标签1即输。旧13原facts与27新增frame facts全保留，合法旧解并未被事实排除。旧VIN=MY在4次实际sketch均零基rank7、保留0次；旧RD=R0均rank4，所有beam VIN=R270而P0 VIN=R0，所以旧整朝向不在该次有限池。VIN保留R270/MXR90的sketch L2穿框7优于旧MY的8，使旧朝向在最终路由前就被top2截断；此sketch近似不能保证保持最终较小面积。这是本case明确的Step1有限搜索边界；不能由更低级aspect为面积损失辩护。4个routed均无fallback、LP均optimized，未发现该case最终rank/作者事实/soft fallback缺陷。

T4：Fraction核164次实际8向数组、10placed及4routed顺序[8,9,6,7]，chosen8正确。旧38原facts+76framefacts全保持。VIP/VIN旧MY各在实际8次sketch均rank7、保留0次；P0二者R0、所有beam均非旧MY，因此旧labels0完整朝向不在当前有限池。最终P0 L0=0、labels5，beam L0=3且labels7（各有1fallback），没有面积跨越高优先级标签的排序错误。旧解合法且更好，但被已声明top2/sketch阶段排除；labels真实挤叠仍是阶段质量退化，不称改善或不可避免。

T4新增1wire/1branch具体发生在out：旧用单rt7 degree4、4条输出连接CC:n/M6:d/CL:p/M7:d；新用rt7/rt8各degree3、5条输出边连接同四端子。按显式writer端点身份构图两侧均是连通树，不按坐标造连接，没有新增/漏物理端子。完整边表和文件SHA见A-quality-localisation.json。这里仅解释输出树划分变化，不外推其它net的Step2拓扑计分。

T2b、两版ota5t、0.3T2/T2b/T6仍各需实际入口/池证据；不得从这两例或raw0.2T6泛化。未bless。


## 余六层取证仪器04（尚未执行）

profile.rs SHA `5e61b15a831404dc3ffb4d8aef661fc67156e4bfe8206607f1bbf587c3cfbc12`，不可变副本/说明在 `target/objectives-v2-validation/t14-quality-instrument-04/`。root已批准六个定向case，唯一验证代理排队执行。两版T3新增显式RNL_PROFILE_LEVEL=ota5t，只搜索/摆放/布线该目标；原entry与按CLI已证wants调用raise_version得到的overlay始终是输入。目标sig3、全ScoreReport、work、同Host读取的完整身份record逐项严格匹配实际T14。其它level原已保存record只在全部搜索结束后作为输出载体，替换目标record再比载体完整文件bytes，binding明确other_levels_reexecuted=false及目标替换范围；此载体相同不称其它层本轮重新验证。没有更改任何production、预算或strict断言。

排队包为raw0.2 T2b（raise0）、0.3 T2/T2b/T6（无raise）、两版T3仅ota5t（0.2 raise0，0.3无raise）；所有case显式CLI_REPORT=true，clear不适用的level/raise环境参数及PROFILE_ORACLE。原T14整文件时间仅供排队参考：0.2T2b1.814s、T3整file15.837s；0.3T2/T2b0.820/0.711s、T3整file3.082s、T6 76.277s。估计总队列2–10分钟含编译/旧frame合法性，不当成受控性能对比。任何入口不匹配保留失败、停止该case，不作bless。


## A 对余六层 finalpass04 的逐例归因（待独立最终复核）

六个入口已由validation严格PASS。A再次用Fraction独立重排每个实际8向数组、完整placed顺序及routed顺序；并用runtime Orient矩阵组合核对相关class root→member变换，不靠名称猜镜像。全部最终chosen正确，旧原facts保留不变且新增frame facts全被接受。证据和每个候选相对旧朝向的精确差异在 `t14-quality-finalpass-04/A-quality-localisation.json`。

| case | 原facts/新增旧frame facts | 真实失选证据 | 实际最终池为何选新解 |
| --- | --- | --- | --- |
| raw0.2 T2b | 14/27 | VIN旧MY四次实际rank7、RD旧R0四次rank4，不在beam前二；P0 VIN=R0。与T2是独立36trace证明，不套T2。 | P0的L0符号重叠0，三个routed beam均1且L2穿框2，故P0在面积之前即胜；作者VIN.x=-4仍保留，未取消拖动事实。 |
| raw0.2 T3 ota5t | 16/46 | PMOS类根M3旧MY（M4为其MY关系）两次实际rank2，恰排第三被截；P0 M3=R0，所有beam M3=R90，旧整朝向缺席5个placed。 | P0 L2穿框0、turns8；另三个routed的L2均1（且两者L0=1），因此少至2/4弯的beam在更高层先输。旧3弯是合法但未生成的整帧。 |
| 0.3 T2 | 18/27 | RD旧R0四次rank4，M1旧R0四次rank2，均未在beam前二；旧VIN=MXR90虽四次rank1可达，但P0 VIN=R0、beam RD非R0，完整旧朝向组合不在任何placed。 | P0 L1短接0；其它routed均1，故选4弯P0而不是低面积beam。不是wire_length越过turns，旧3弯frame不在池。 |
| 0.3 T2b | 19/27 | 独立16trace与T2相同的RD/M1截断；旧VIN虽然有top2资格，但P0与beam均不含完整旧组合。 | P0 L1短接0，另外三个routed均1。显式VIN.x=-4保留；不能因图与T2相同认为丢事实。 |
| 0.3 T3 ota5t | 20/38 | PMOS根M3旧MY两次实际rank6（第七），从未前二；P0 M3R0，全部beam MXR90，旧整朝向组合缺席5个placed。 | P0 L2穿框0/labels0/turns10，三个routed beam均L2=1/labels2/turns7，在更高层先输；小面积不拿来解释高优先级turn损失。 |
| 0.3 T6 | 27/102 | 独立425trace；VB0旧MY的17次实际rank分别3×11、5×6，从未前二；所有beam VB0=MYR90，P0=R0。没有复用raw0.2的MY/rank7结论。 | 两个P0 labels0/turns57，同turns后面积39221/25较小者胜；两个routed beam虽然turns48/46，但labels5在更高层先输。旧36弯frame合法却不在池。 |

这些是每例已观察到的旧整帧再现障碍，加上完整实际候选池排序正确的证据。它们足以定位到已声明的Step1有限提议/top2-sketch阶段限制，而不构成全局最优保证，也不证明单改top2必定恢复旧坐标/导线；固定LP四级及有限rows/局部路由仍可能共同限制可达性。没有恢复隐藏名称偏好、扩预算或把低级area用来掩盖高级退化。各例所有placed均coordinate_lp_optimised=true、placement_undecided=false；候选工作计数均未触256默认帽，不能把这些例子说成预算耗尽。

T3两版的证据严格限于ota5t目标：原entry上下文/明确raise（0.2）作为输入，target sig3/完整score/work/Host identity record均匹配T14；其它层仅作为事后输出carrier，未重新求解、不作为main层新增验证。T6.3此次84.37s只称诊断执行时间，不复标正式bench或受控性能。当前质量解释等待独立审阅整体接受，不构成bless授权。
