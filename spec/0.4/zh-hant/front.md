<!-- translated from en/front.md sha256:96a6e2a65dbe -->
# RNL 語言標準，0.4 版（草案）

RNL（Rich Netlist）寫在電路網表的註解中。它描述電路如何繪製成原理圖以及各部分的含義，而電路本身僅由網表定義。本文件是該語言 0.4 版的標準。

- 以英文正文為準。各語種譯文依 `zh`、`ja` 等語言標籤命名為 `rnl-0.4.<lang>.md`，僅供參考；兩者有差異時，以英文正文為準。
- 詞彙表 `rnl-words-0.4.json` 及其結構描述 `rnl-words-0.4.schema.json` 為本標準的一部分（5.3、附錄 A）。
- RNL 的設計原則見 `rich-netlist.md`（英文，具權威性；譯本：`rich-netlist.<lang>.md`）；本標準引用時稱之為「原則」。
- 0.x 各版本之間不保證彼此相容（16.1）。
- 本文依研究儲存庫 rnl-dev 提交 {SOURCE} 中的 0.4 版草案修訂自 0.3 版；附錄 D 列出了變更內容。
