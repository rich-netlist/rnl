# Rich Netlist（RNL）

[English](README.md) | 中文

RNL 写在电路网表的注释里，描述电路怎样画成原理图、各部分是什么意思；电路本身只由网表定义。本仓库存放这门语言的标准和它的工具链：可移植的运行时、求解器、`rnl` 命令、桌面版和网页版查看器、标准符号库，以及给 AI agent 的 skill 包。还没有拖动器件的编辑器。

**网站：[rich-netlist.github.io/rnl/zh](https://rich-netlist.github.io/rnl/zh/)**，有[写给电路设计者的介绍](https://rich-netlist.github.io/rnl/zh/)、[语言导览](https://rich-netlist.github.io/rnl/zh/language.html)、[语言标准 0.3](https://rich-netlist.github.io/rnl/zh/spec/0.3/)、画出每个符号的[标准包图库](https://rich-netlist.github.io/rnl/zh/packages.html)和[查看器](https://rich-netlist.github.io/rnl/zh/tools.html)：一个[在浏览器里用](https://rich-netlist.github.io/rnl/viewer/)，一个供 Windows 下载。每页都有英文版。

**状态：** 本仓库存放 RNL 语言标准 0.3 版（现在是第 2 修订，取代 0.2 版，工具仍能读 0.2 版的文件）及其原则、项目文档，以及 RNL 工具链，目前是它的第一版 0.1。工具链读取带 RNL 记录的 SPICE 和 Spectre 网表，报告诊断，生成有效模型，画出有坐标的部分，并能把记录写回网表。附带 `rnl` 命令；0.3 和 0.2 两个系列的标准包及其符号，以及 0.3 系列没有画的 Analog Canvas 符号的包；教 AI agent 读、写、检查和绘制 RNL 的 skill 包（[`skills/rnl`](skills/README.md)）；还有求解器（[`crates/rnl-solver`](crates/rnl-solver/README.md)），`rnl solve` 用它给没有坐标的部分排位、走线，并把求解器的记录写进文件的副本或原文件。求解器还没有 WebAssembly 绑定，它的优化目标第二版正在做。原理图由一个共享核心决定（[决定 0008](docs/decisions/0008-shared-core-per-platform-frontends.md)）：它用随附的字体排字（[决定 0009](docs/decisions/0009-fonts-shipped-with-the-core.md)），原生运行和编译成 WebAssembly 运行得到的结果完全相同（[`bindings/runtime-wasm`](bindings/runtime-wasm/README.md)）；后者已作为预览版发布到 npm，即 `@rich-netlist/runtime@next`（[`packages/runtime`](packages/runtime/README.md)）。两个查看器共用一个页面显示这些结果（[`apps/viewer-ui`](apps/viewer-ui/README.md)）：Windows 桌面版（[`apps/desktop`](apps/desktop/README.md)，已发布 0.1.1 版）和网页版（[`apps/web`](apps/web/README.md)）；它们打开网表，画出各层，显示网表文本、诊断和层次结构，在原理图和文本之间跟随选中的实例或网络，列出没画出来的对象（没有位置的，以及找不到模型卡的晶体管），还能改文本、应用后重画：桌面版保存时写回文件，网页版保存时下载（[决定 0011](docs/decisions/0011-viewers-edit-the-text-and-write-only-what-the-user-saves.md)）。桌面版还能求解，结果作为预览，可以另存求解副本（[决定 0010](docs/decisions/0010-desktop-viewer-solves-as-a-preview.md)）。还没有拖动器件的编辑器。语言的 0.x 各版本之间不承诺兼容。

本文是 [README.md](README.md) 的中文版，两者不一致时以英文版为准。仓库里的文档大多只有英文；有中文的是语言标准、原则、工具链架构提案和网站。

## 一瞥

```spice
* Five-transistor OTA, input stage
* @RNL(type="header") { @VERSION("0.3")}
.model nch nmos
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   %dp1 = #M1(in_p) and #M2(in_n) treated @AS $DIFFERENTIAL_PAIR;
*   {#M1 #M2} @ALIGN("y"), i.e., same y;
*   #M5 is at @PLACE(20, 6) => it @ORIENT("R0");
*   %g0 = $GROUND is @ATTACH to &vss;
*   net &vss has a @WIRE connecting {#M5:3 %g0:p}
* }
.ends ota5t
.end
```

网表本身不变。注释里的这条记录说：M1 和 M2 是一个差分对（M1 担任角色 `in_p`，M2 担任 `in_n`），两者 y 相同；M5 放在哪里、朝向如何；网络 `vss` 加一个地符号，并有导线连到 M5。记号之间的 `and`、`treated`、`is at` 等词是自由文本：标准例句加上它们，让语句读起来像句子，它们不改变含义。标准的 [1.3 节](https://rich-netlist.github.io/rnl/zh/spec/0.3/#1.3)逐个解释了这些记号。

## 语言标准

| 内容 | 位置 |
| --- | --- |
| 网页 | [中文版](https://rich-netlist.github.io/rnl/zh/spec/0.3/)和[英文版](https://rich-netlist.github.io/rnl/en/spec/0.3/)（正式文本），各附 PDF 和 Markdown 下载；0.2 版仍在[中文版](https://rich-netlist.github.io/rnl/zh/spec/0.2/)和[英文版](https://rich-netlist.github.io/rnl/en/spec/0.2/) |
| 每个修订的全部发布形式 | 标签为 `spec-0.3-r2`、`spec-0.3-r1`、`spec-0.2-r1`……的 [GitHub Releases](https://github.com/rich-netlist/rnl/releases)：PDF、单文件网页、Markdown、按章打包的 zip，以及 `SHA256SUMS` |
| 仓库里的 Markdown | [`spec/rnl-0.3.zh.md`](spec/rnl-0.3.zh.md)（中文版）和 [`spec/rnl-0.3.md`](spec/rnl-0.3.md)（英文版，正式文本），由 [`spec/0.3/`](spec/README.md) 里的各章合成；0.2 版也留在旁边 |
| 标准词表 | [`spec/rnl-words-0.3.json`](spec/rnl-words-0.3.json) 及其 [schema](spec/rnl-words.schema.json)，是标准的一部分 |
| 原则 | [`spec/rich-netlist.zh.md`](spec/rich-netlist.zh.md)（中文版）和 [`spec/rich-netlist.md`](spec/rich-netlist.md)（英文版，以它为准） |

所有形式都由同一个提交的各章生成，并标明各自的修订（[决定 0007](docs/decisions/0007-publishing-the-standard.md)）。语言各版本在研究仓库里讨论成形，再发布到这里（[决定 0006](docs/decisions/0006-language-versions-from-research-repository.md)）。

## 仓库结构

| 路径 | 职责 |
| --- | --- |
| [`spec/`](spec/README.md) | RNL 语言标准及其原则。 |
| [`crates/`](crates/README.md) | Rust 运行时、SPICE 和 Spectre 宿主适配器、共享渲染器、给查看器的核心接口、写入编辑的写入器、求解器和 `rnl` 命令。 |
| [`bindings/`](bindings/README.md) | 编译成 WebAssembly 的核心，以及在 Worker 里运行它的 JavaScript 模块，一起构建成 npm 包 `@rich-netlist/runtime`；求解器单独的绑定是将来的工作。 |
| [`packages/`](packages/README.md) | TypeScript 包：npm 包 `@rich-netlist/runtime` 的 README 和冒烟测试（`runtime/`；包本身由 `bindings/runtime-wasm` 构建）；不依赖框架的查看器核心、编辑器核心和求解器的接口是将来的工作。 |
| [`apps/`](apps/README.md) | 桌面版和网页版查看器，以及两者共用的页面。 |
| [`libraries/`](libraries/README.md) | 外部数据库：0.3 和 0.2 两个系列的标准包及其符号、中文代称包，以及 `analog-canvas-extensions`。 |
| [`tests/`](tests/README.md) | 一致性测试语料、求解器的用例，以及查看器的浏览器测试；专门的往返用例是将来的工作。 |
| [`examples/`](examples/README.md) | 将来的示例文档。 |
| [`skills/`](skills/README.md) | 给 AI agent 的 skill 包：RNL skill，教 agent 读、写、检查和绘制 RNL 0.3，随网站发布，入口是[给 agent 的页面](https://rich-netlist.github.io/rnl/zh/agents.html)。 |
| [`scripts/`](scripts/README.md) | 项目维护脚本：生成标准的各种发布形式、构建和检查 skill 包，以及随核心附带的字体。 |
| [`.github/workflows/`](.github/workflows/README.md) | CI 工作流：项目网站、标准、桌面版查看器和 npm 包的发布、Rust 工作区、skill 包、查看器及其浏览器测试。 |
| [`docs/`](docs/README.md) | 工具链架构提案、路线图和决定记录。 |
| [`site/`](site/README.md) | [项目网站](https://rich-netlist.github.io/rnl/zh/)，中英文两版由 `site/build.py` 从同一份双语源生成：介绍页、语言导览（含“一览”和例子）、标准包图库、工具页（含工具链）和给 agent 页；[`pages.yml`](.github/workflows/pages.yml) 再加上语言标准的网页（含简明指南和设计原则）、网页版查看器和 skill 包。 |

建议从[文档地图](docs/README.md)、[工具链架构提案](docs/rnl-toolchain-architecture.md)（中文）和[路线图](docs/roadmap.md)读起。每个组件目录里都有一份 README，说明它现在有什么、还有什么要做。

## 试用

```bash
cargo run --release -p rnl-cli -- check tests/conformance/cases/03-five-transistor-ota.sp
cargo run --release -p rnl-cli -- render tests/conformance/cases/03-five-transistor-ota.sp -o target/svg
```

其他命令见 [`crates/rnl-cli`](crates/rnl-cli/README.md)。Windows 桌面版的安装包在[发布页](https://github.com/rich-netlist/rnl/releases)（最新的 `viewer-v…` 标签），核心在 npm 上：`npm i @rich-netlist/runtime@next`。要从源码运行桌面版（比如在 macOS 或 Linux 上），并用它打开网表：

```bash
cargo run --manifest-path apps/desktop/Cargo.toml -- tests/conformance/cases/03-five-transistor-ota.sp
```

或者用网页版，网站上就有，带着同样的示例：[rich-netlist.github.io/rnl/viewer/?url=samples/cases/03-five-transistor-ota.sp](https://rich-netlist.github.io/rnl/viewer/?url=samples/cases/03-five-transistor-ota.sp)。要在本机运行，先连同示例构建站点并在本机提供，再打开 `http://127.0.0.1:8000/?url=samples/cases/03-five-transistor-ota.sp`。用 `python scripts/fonts.py web` 生成字体后（[`scripts`](scripts/README.md)），两个查看器都用随附的字体显示文字；没有字体时退回系统字体。

```bash
node apps/web/build.mjs --samples
```

```bash
node apps/web/serve.mjs
```

## 下一步

[路线图](docs/roadmap.md)是完整的清单，写明哪些做完了、哪些没有。接下来：语言 0.4，计划让规则能按器件的端子个数选符号，并让网表能配置求解器；求解器优化目标的第二版，把标准包里的名字从求解器里移到它的配置中；求解器的 WebAssembly 绑定，让网页版也能求解；晶圆厂 PDK 器件的符号包；桌面版查看器的 macOS 和 Linux 版本，以及 Windows 版本的签名；还有一份工具链所支持的标准范围的记录，对照已发布的正文核对，并补上专门的往返用例。拖动器件的编辑器暂时搁置，路线图写了原因。

## 许可证

| 内容 | 路径 | 许可证 |
| --- | --- | --- |
| 其余一切，包括代码、配置和顶层文件 | 下面没有列出的全部路径 | [MIT](LICENSE) |
| 规范文本和项目文档 | `spec/`、`docs/` | [CC-BY-4.0](LICENSES/CC-BY-4.0.txt) |
| 数据：标准库、测试语料和示例 | `libraries/`、`tests/`、`examples/` | [CC0-1.0](LICENSES/CC0-1.0.txt) |
| 随核心附带的字体 Noto Sans SC 的度量数据，版权 2014-2021 Adobe（[决定 0009](docs/decisions/0009-fonts-shipped-with-the-core.md)） | `crates/rnl-render/fonts/noto-sans-sc.metrics` | [OFL-1.1](LICENSES/OFL-1.1.txt) |

[`REUSE.toml`](REUSE.toml) 记录了这一对应关系，由 `reuse lint` 检查。从 Analog Canvas 迁来的文档由其版权人以 CC-BY-4.0 在这里发布；Analog Canvas 里冻结的副本仍按该仓库的 AGPL-3.0 许可。见[决定 0002](docs/decisions/0002-licenses.md)。

## 参与贡献

修改语言或模块边界之前，请先在 issue 或 pull request 里讨论，并让文档的状态始终写清楚。贡献按它所修改路径的许可证授权，不需要签署声明或贡献者协议。详见 [CONTRIBUTING.md](CONTRIBUTING.md)。

公开扩展和工艺包由 [rnl-packages](https://github.com/rich-netlist/rnl-packages) 维护，网页版查看器从清单下载并校验 SHA-256；标准系列仍然内置。开发和测试前运行 `python scripts/packages.py download`，版本由 `packages.lock.json` 固定，详见 [贡献指南](CONTRIBUTING.md)。
