* Text the shipped fonts do not map: other scripts, emoji and combining marks
*! 随核心发布的字体（Noto Sans SC，决定 0007）没有的字符：韩文、表情符号（零宽连接符连成的、带肤色的、带变体选择符的，其中 ⚠ 本身字体里有）、键帽、旗帜、阿拉伯文、天城文、泰文、字体里没有的组合符号（x̂、v⃗）和零宽空格；
*! 和字体里有的字符（拉丁字母、Ω、汉字、字体里有的组合符号 x́）混排，有粗体、斜体、上划线、上下标、分式、居中、右对齐、竖排，也有实例的名字。期望文件只记下文字的内容；rnl-view 的测试检查这些字符自成一段、按类估计宽度、在 SVG 里居中而不拉伸。
R1 a b 1k
* @RNL(type="header") { @VERSION("0.2")}
* @RNL {
*   #R1 @PLACE(0, 0);
*   #R1 @TEXT("R1 저항", key="name");
*   (6, 8) @TEXT("[한글 저항]");
*   (6, 6) @TEXT("R = 10 kΩ ⚡ ⚠️ 1️⃣");
*   (6, 4) @TEXT("👨‍👩‍👧 👍🏽 🇨🇳 🏳️‍🌈");
*   (6, 2) @TEXT("مقاومة 10k");
*   (6, 0) @TEXT("प्रतिरोध R1, कि");
*   (6, -2) @TEXT("ตัวต้านทาน R2");
*   (6, -4) @TEXT("x̂ = x́ + v⃗, a​b");
*   (6, -6) @TEXT("\textbf{한글 Bold ⚡} \textit{\overline{x̂ y}} V_{저항}^{⚡}");
*   (12, -9) @TEXT("\frac{저항}{R_{1}} 电阻", halign="middle");
*   (18, -12) @TEXT("R1 저항 ⚡", halign="end", valign="middle");
*   (-4, 0) @TEXT("한글 ⚡ R", rotate=90, halign="middle", size=0.8)
* }
.end
