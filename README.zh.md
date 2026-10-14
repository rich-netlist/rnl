# Rich Netlist（RNL）

[English](README.md) | 中文

RNL 写在电路网表的注释里，描述电路怎样画成原理图、各部分是什么意思；电路本身只由网表定义。本仓库存放这门语言的标准，今后会逐步加入可移植的 RNL 运行时、求解器、查看器、编辑器和标准符号库。

**网站：[rich-netlist.github.io/rnl](https://rich-netlist.github.io/rnl/)**，有[写给电路设计者的介绍](https://rich-netlist.github.io/rnl/)、[语言导览](https://rich-netlist.github.io/rnl/language.html)和[语言标准 0.2](https://rich-netlist.github.io/rnl/spec/0.2/)，每页都有英文版。

**状态：** 本仓库存放 RNL 语言标准 0.2 版及其原则、项目文档，以及工具链的目录骨架。还没有解析器、求解器、渲染器、应用程序，也没有发布任何软件包。语言的 0.x 各版本之间不承诺兼容。

本文是 [README.md](README.md) 的中文版，两者不一致时以英文版为准。仓库里的文档大多只有英文；有中文的是语言标准、原则、工具链架构提案和网站。

## 一瞥

```spice
* Five-transistor OTA, input stage
* @RNL(type="header") { @VERSION("0.2")}
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL {
*   %dp1 = #M1(in_p) and #M2(in_n) treated @AS $DIFFPAIR;
*   {#M1 #M2} @ALIGN("y"), i.e., same y;
*   #M5 is at @PLACE(20, 6) => it @ORIENT("R0");
*   %g0 = $GND is @ATTACH to &vss;
*   net &vss has a @WIRE connecting {#M5:3 %g0:p}
* }
.ends ota5t
.end
```

网表本身不变。注释里的这条记录说：M1 和 M2 是一个差分对（M1 担任角色 `in_p`，M2 担任 `in_n`），两者 y 相同；M5 放在哪里、朝向如何；网络 `vss` 加一个地符号，并有导线连到 M5。记号之间的 `and`、`treated`、`is at` 等词是自由文本：标准例句加上它们，让语句读起来像句子，它们不改变含义。标准的 [1.3 节](https://rich-netlist.github.io/rnl/spec/0.2/#1.3)逐个解释了这些记号。

## 语言标准

| 内容 | 位置 |
| --- | --- |
| 网页 | [中文版](https://rich-netlist.github.io/rnl/spec/0.2/)和[英文版](https://rich-netlist.github.io/rnl/en/spec/0.2/)（正式文本），各附 PDF 和 Markdown 下载 |
| 每个修订的全部发布形式 | 标签为 `spec-0.2-r1`、`spec-0.2-r2`……的 [GitHub Releases](https://github.com/rich-netlist/rnl/releases)：PDF、单文件网页、Markdown、按章打包的 zip，以及 `SHA256SUMS` |
| 仓库里的 Markdown | [`spec/rnl-0.2.zh.md`](spec/rnl-0.2.zh.md)（中文版）和 [`spec/rnl-0.2.md`](spec/rnl-0.2.md)（英文版，正式文本），由 [`spec/0.2/`](spec/README.md) 里的各章合成 |
| 标准词表 | [`spec/rnl-words-0.2.json`](spec/rnl-words-0.2.json) 及其 [schema](spec/rnl-words.schema.json)，是标准的一部分 |
| 原则 | [`spec/rich-netlist.zh.md`](spec/rich-netlist.zh.md)（中文版）和 [`spec/rich-netlist.md`](spec/rich-netlist.md)（英文版，以它为准） |

所有形式都由同一个提交的各章生成，并标明各自的修订（[决定 0007](docs/decisions/0007-publishing-the-standard.md)）。语言各版本在研究仓库里讨论成形，再发布到这里（[决定 0006](docs/decisions/0006-language-versions-from-research-repository.md)）。

## 仓库结构

| 路径 | 计划中的职责 |
| --- | --- |
| [`spec/`](spec/README.md) | RNL 语言标准及其原则。 |
| [`crates/`](crates/README.md) | 将来的 Rust 运行时、宿主适配器、共享渲染器、求解器和命令行工具。 |
| [`bindings/`](bindings/README.md) | 运行时和求解器各自的 WASM 入口。 |
| [`packages/`](packages/README.md) | TypeScript SDK，以及不依赖框架的查看器、编辑器核心。 |
| [`apps/`](apps/README.md) | Web 应用和桌面外壳。 |
| [`libraries/`](libraries/README.md) | 外部数据库，先从标准符号开始。 |
| [`tests/`](tests/README.md) | 将来的一致性、往返和集成测试语料。 |
| [`examples/`](examples/README.md) | 将来的示例文档。 |
| [`scripts/`](scripts/README.md) | 项目维护脚本，例如生成标准各种发布形式的构建脚本。 |
| [`.github/workflows/`](.github/workflows/README.md) | CI 工作流：项目网站和标准的发布。 |
| [`docs/`](docs/README.md) | 工具链架构提案、路线图和决定记录。 |
| [`site/`](site/README.md) | [项目网站](https://rich-netlist.github.io/rnl/)：写给电路设计者的介绍页和语言导览，中英文两版由 `site/build.py` 从同一份双语源生成。 |

建议从[文档地图](docs/README.md)、[工具链架构提案](docs/rnl-toolchain-architecture.md)（中文）和[路线图](docs/roadmap.md)读起。每个组件目录里都有一份简短的 README，说明它计划承担的职责。

## 下一步

确定第一版工具链支持标准 0.2 的哪些部分，整理一致性和往返测试语料，并在开始实现时建立真正的 Cargo 和 pnpm 工作区。[路线图](docs/roadmap.md)跟踪这些工作，不会把未完成的写成已完成。

## 许可证

| 内容 | 路径 | 许可证 |
| --- | --- | --- |
| 其余一切，包括代码、配置和顶层文件 | 下面没有列出的全部路径 | [MIT](LICENSE) |
| 规范文本和项目文档 | `spec/`、`docs/` | [CC-BY-4.0](LICENSES/CC-BY-4.0.txt) |
| 数据：标准库、测试语料和示例 | `libraries/`、`tests/`、`examples/` | [CC0-1.0](LICENSES/CC0-1.0.txt) |

[`REUSE.toml`](REUSE.toml) 记录了这一对应关系，由 `reuse lint` 检查。从 Analog Canvas 迁来的文档由其版权人以 CC-BY-4.0 在这里发布；Analog Canvas 里冻结的副本仍按该仓库的 AGPL-3.0 许可。见[决定 0002](docs/decisions/0002-licenses.md)。

## 参与贡献

修改语言或模块边界之前，请先在 issue 或 pull request 里讨论，并让文档的状态始终写清楚。贡献按它所修改路径的许可证授权，不需要签署声明或贡献者协议。详见 [CONTRIBUTING.md](CONTRIBUTING.md)。
