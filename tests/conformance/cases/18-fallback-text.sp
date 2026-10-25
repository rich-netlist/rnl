* Text the shipped fonts do not map: other scripts, emoji and combining marks
*! Characters the font shipped with the core (Noto Sans SC, decision 0009) lacks: Korean, emoji (joined by zero-width joiners, with skin tones, with variation selectors, where ⚠ itself is in the font), keycaps, flags, Arabic, Devanagari, Thai, combining marks the font lacks (x̂, v⃗) and a zero-width space;
*! mixed with characters the font has (Latin letters, Ω, Chinese characters, a combining mark it has, x́), in bold, italic, overline, sub- and superscripts, fractions, centred, right-aligned and vertical, and in an instance's name. The expected file records only the texts' content; rnl-view's tests check that these characters form runs of their own, have widths estimated by class, and are centred in SVG without stretching.
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
