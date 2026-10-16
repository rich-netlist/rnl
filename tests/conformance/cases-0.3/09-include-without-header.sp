* A file of 0.2 that includes a file without a header record
* @RNL(type="header") { @VERSION("0.2")}
*! 被包含的文件没有头段记录，随包含它的文件读成 0.2（16.1），用 0.2 系列（16.3）：库里的 $RES、$GND 是 0.2 系列的名字，照常找到。
*! 默认版本 0.3 只给主文件和没有版本的包文件；主文件声明了 0.2，整份文档就都是 0.2。
.include 09-include-without-header-lib.inc
X1 in 0 divider
* @RNL { #X1 @PLACE(0, 0) }
.end
