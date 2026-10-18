"""Shared pieces of the build of the RNL standard (scripts/spec/ in rnl; publish/ in rnl-dev):
assembling the combined text from the chapters, comparing the Chinese chapters with the English ones,
the JSON excerpt of 5.3, and Annex A from the word list. Standard library only."""

from __future__ import annotations

import json
import re
from collections import Counter

def parts(names: list[str]) -> list[str]:
    """The parts of one version, in order, from the names of its files: the chapters ch01.md, ch02.md, …
    by number, then the annexes annex-a.md, annex-b.md, … by letter. front.md is not a part."""
    chapters = sorted((n for n in names if re.fullmatch(r"ch\d+\.md", n)), key=lambda n: int(n[2:-3]))
    return chapters + sorted(n for n in names if re.fullmatch(r"annex-[a-z]\.md", n))


def compared(parts: list[str]) -> list[str]:
    """The parts compared section by section: Annex A is generated in both languages, and Annex B of the
    Chinese version has a column of Chinese sentences."""
    return [p for p in parts if p not in ("annex-a.md", "annex-b.md")]


# Words of the discussion drafts that the Chinese version does not use (zh-guide.md).
OLD_WORDS = ["虚元件", "姿态", "注册点", "合画", "强度", "原型", "草稿", "（用户", "D 轮", "rnl-dev"]


# --- the combined text -------------------------------------------------------------------------------

def anchor(title: str) -> str:
    """The anchor GitHub gives a heading."""
    a = title.strip().lower()
    a = re.sub(r"[^\w\- ]", "", a, flags=re.UNICODE)
    return a.replace(" ", "-")



def assemble(front: str, parts: list[str], lang: str) -> str:
    """The combined text: the front matter, a table of contents of chapters and sections, the parts."""
    front = front.rstrip() + "\n"
    bodies = [p.strip() + "\n" for p in parts]
    toc = ["## Contents" if lang == "en" else "## 目录", ""]
    for body in bodies:
        in_code = False
        for line in body.split("\n"):
            if line.lstrip().startswith("```"):
                in_code = not in_code
            if in_code:
                continue
            m = re.match(r"^(##|###) (.+)$", line)
            if m:
                indent = "" if m.group(1) == "##" else "  "
                toc.append(f"{indent}- [{m.group(2)}](#{anchor(m.group(2))})")
    return front + "\n" + "\n".join(toc) + "\n\n" + "\n".join(bodies)


def excerpt(words: dict) -> str:
    """The JSON excerpt of 5.3: the word-list entry of PLACE."""
    place = next(k for k in words["keywords"] if k["keyword"] == "PLACE")
    return "```json\n" + json.dumps(place, ensure_ascii=False, indent=1) + "\n```"


# --- comparing the Chinese with the English ---------------------------------------------------------------

def sections(text: str, by_position: bool = False) -> dict:
    """Split a chapter into sections keyed by heading number ('9.4', or 'A' style for annexes); a text whose
    headings are not numbered, such as the short guide, by the level and the place of each heading."""
    out = {}
    key = "(top)"
    seen = 0
    cur = {"code": [], "inline": Counter(), "items": 0, "rows": 0, "paras": 0, "heading": ""}
    out[key] = cur
    in_code = False
    block: list[str] = []
    for line in text.split("\n"):
        if line.lstrip().startswith("```"):
            if in_code:
                block.append(line.strip())
                cur["code"].append("\n".join(block))
                block = []
                in_code = False
            else:
                in_code = True
                block = [line.strip()]
            continue
        if in_code:
            block.append(line.rstrip())
            continue
        if line.startswith("<!--"):  # the stamp of the English text a part was translated from
            continue
        m = re.match(r"^(#{1,4}) (?:Annex |附录 )?(\S+)" if by_position else r"^(#{2,4}) (?:Annex |附录 )?(\S+)", line)
        if m:
            seen += 1
            key = f"h{len(m.group(1))}.{seen}" if by_position else m.group(2)
            cur = {"code": [], "inline": Counter(), "items": 0, "rows": 0, "paras": 0, "heading": line}
            out[key] = cur
            continue
        cur["inline"].update(re.findall(r"`[^`\n]+`", line))
        if re.match(r"^\s*([-*]|\d+\.) ", line):
            cur["items"] += 1
        elif line.lstrip().startswith("|"):
            if not re.match(r"^\s*\|[\s|:-]+\|\s*$", line):
                cur["rows"] += 1
        elif line.strip():
            cur["paras"] += 1
    return out



def compare(en_text: str, zh_text: str, part: str, by_position: bool = False) -> list[str]:
    """Differences of structure and code between an English part and its Chinese version."""
    en, zh = sections(en_text, by_position), sections(zh_text, by_position)
    msgs = []
    if list(en) != list(zh):
        msgs.append(f"{part}: headings differ\n    en: {' '.join(en)}\n    zh: {' '.join(zh)}")
    for key in en:
        if key not in zh:
            continue
        e, z = en[key], zh[key]
        where = f"{part} {key}"
        if e["code"] != z["code"]:
            msgs.append(f"{where}: code blocks differ ({len(e['code'])} in en, {len(z['code'])} in zh)")
        missing = e["inline"] - z["inline"]
        extra = z["inline"] - e["inline"]
        if missing:
            msgs.append(f"{where}: inline code missing in zh: " + ", ".join(f"{k}×{v}" for k, v in missing.items()))
        if extra:
            msgs.append(f"{where}: inline code only in zh: " + ", ".join(f"{k}×{v}" for k, v in extra.items()))
        for f, name in (("items", "list items"), ("rows", "table rows"), ("paras", "paragraphs")):
            if e[f] != z[f]:
                msgs.append(f"{where}: {name} {e[f]} in en, {z[f]} in zh")
    for w in OLD_WORDS:
        n = zh_text.count(w)
        if n:
            msgs.append(f"{part}: old word {w!r} ×{n}")
    return msgs


# --- Annex A from the word list ---------------------------------------------------------------------------

ROLE_NOTE = {"en": {"AS": "the roles of the definition, then the definition", "ATTACH": "the definition, then its roles"},
             "zh": {"AS": "定义的各角色，然后是定义", "ATTACH": "定义，然后是它的各角色"}}

# Annex A of the Chinese version (zh-guide.md): the same tables, with the fixed phrases in Chinese.
TAKES_ZH = {
    "one": "单个对象；收到集合时，对每个成员各做一次（5.2.1），只有 PRIORITY 收到带 GROUP 的集合时作用于分组本身（14.8）",
    "set": "集合；为这个槽位散写的几个对象合成一个集合（5.2.1）",
    "any": "一个对象，或作为整体的一个集合",
    "shape": "一个点，或一组点（图元的两种写法，9.2.5）",
    "body": "花括号里的语句，封装（5.4）",
    "definition": "定义：`$` 名字、绑定了定义的 `%` 名字，或匿名定义（5.6、6.6）",
}
PSEUDO_KINDS_ZH = {
    "any": "任何对象",
    "level": "单独的 `#`：本层",
    "owner": "符号体里单独的 `?`：用这个符号画出的对象",
    "position": "对象的子对象 `::position`（它的原点）",
    "locatable": "有位置的对象：元件、叠画、有引脚的端子、点、文字、用中心写的图元的中心、有自己位置的画出的用例，或由它们组成的集合（14.1）",
    "orientable": "有朝向的对象：元件、叠画、有符号的用例，或由它们组成的集合（14.9）",
    "constraint": "约束关键字产生的对象",
    "set": "集合，写成名字",
}
GROUP_ZH = {
    "Header records and packages": "头段记录与包", "Definitions": "定义",
    "Symbol content and graphics": "符号内容与图元", "Placement and appearance": "放置与外观",
    "Selections": "选择", "Applying definitions": "套用", "Wires": "导线",
    "Sets, groups and bundles": "集合、分组与叠画", "Constraints and priority": "约束与优先级",
    "Electrical semantics": "电气语义",
}
WORD_ZH = {  # kinds and types named in the tables
    "instance": "元件", "terminal": "端子", "net": "网络", "point": "点", "wire": "导线", "usage": "用例",
    "bundle": "叠画", "text": "文字", "annotation": "标注", "label": "标签", "constraint": "约束", "pin": "引脚",
    "route": "布线", "set": "集合", "general": "普通定义", "symbol": "符号", "style": "样式",
    "number": "数", "string": "字符串", "bool": "布尔", "color": "颜色", "array": "数组",
}
KEY_ZH = {"object and axis": "对象和轴", "object": "对象", "terminal": "端子",
          "attached object and key, when key is given": "附着对象和引用名，写了 key 时"}
KIND_TEXT = {"en": {"any": "any object", "level": "`#`", "owner": "`?`", "position": "`::position`",
                    "general": "a general definition", "symbol": "a symbol", "style": "a style"},
             "zh": {"any": "任何对象", "level": "`#`", "owner": "`?`", "position": "`::position`",
                    "general": "普通定义", "symbol": "符号", "style": "样式"}}


def md_params(e, lang="en") -> str:
    zh = lang == "zh"
    if e.get("style") == "all":
        return "样式属性（9.3）" if zh else "style properties (9.3)"
    parts = []
    pos = e.get("positional", [])
    for p in e["params"]:
        s = f"`{p['name']}`"
        bits = []
        if p["name"] in pos:
            bits.append(f"位置 {pos.index(p['name']) + 1}" if zh else f"position {pos.index(p['name']) + 1}")
        if p.get("required"):
            bits.append("必需" if zh else "required")
        if p["type"] == "enum":
            vals = ("、" if zh else ", ").join(f'`{json.dumps(v, ensure_ascii=False)}`' for v in p["values"])
            if p["name"] != "priority":
                bits.append(vals)
        elif p["type"] != "string":
            t = p["type"]
            if zh:
                t = WORD_ZH.get(t, t)
                if "items" in p and p["type"] != "array":
                    t += f"，或非空的{t}数组"
            elif "items" in p and t != "array":
                t += f", or a non-empty array of {t}s"
            bits.append(t)
        if "default" in p:
            d = json.dumps(p['default'], ensure_ascii=False)
            bits.append(f"默认 `{d}`" if zh else f"default `{d}`")
        if bits:
            s += ("（" + "；".join(bits) + "）") if zh else (" (" + "; ".join(bits) + ")")
        parts.append(s)
    if e.get("definition_params"):
        parts.insert(0, "定义的参数（按参数表的次序按位置写，或写名字；9.4）" if zh else
                     "the parameters of the definition (by position in the order of its parameter list, or by name; 9.4)")
    if e.get("style") and e["style"] != "all":
        parts.append(f"{e['style']} 组的样式属性" if zh else f"{e['style']} style properties")
    extra = []
    sep = "、" if zh else ", "
    if e.get("at_least_one"):
        names = sep.join(f"`{n}`" for n in e["at_least_one"])
        extra.append(f"{names} 至少写一个" if zh else "at least one of " + names)
    for g in e.get("together", []):
        names = ("、" if zh else " and ").join(f"`{n}`" for n in g)
        extra.append(f"{names} 同时写或都不写" if zh else names + " together or not at all")
    for g in e.get("exclusive", []):
        extra.append((" 不与 " if zh else " not with ").join(f"`{n}`" for n in g) + (" 同写" if zh else ""))
    text = ("，" if zh else ", ").join(parts) if parts else "—"
    if extra:
        text += ("；" if zh else "; ") + ("；" if zh else "; ").join(extra)
    return text


def md_slots(e, lang="en") -> str:
    zh = lang == "zh"
    if e.get("role_slots"):
        at = e["slots"][0]["at"]
        if zh:
            return ROLE_NOTE["zh"][e["keyword"]] + f"（`definition`，写在{'最前' if at == 'first' else '最后'}）"
        return ROLE_NOTE["en"][e["keyword"]] + " (`definition`, written " + at + ")"
    if not e["slots"]:
        return "—"
    out = []
    for s in e["slots"]:
        txt = f"`{s['name']}`{'：' if zh else ': '}{s['takes']}"
        if s.get("kinds"):
            kinds = ("、" if zh else ", ").join(KIND_TEXT[lang].get(k, k) for k in s["kinds"])
            txt += f"（{kinds}）" if zh else f" ({kinds})"
        if s.get("optional"):
            txt += "，可省" if zh else ", optional"
        out.append(txt)
    return ("；" if zh else "; ").join(out)


def md_result(e, lang="en") -> str:
    zh = lang == "zh"
    kind = e.get("output_kind", "")
    if zh:
        k = WORD_ZH.get(kind, kind)
        out = {"none": "—", "same": "输入的对象", "target": "目标集合",
               "new": f"新的{k}", "definition": f"定义（{k}）"}[e["output"]]
        if e.get("selects"):
            out += f"，成员是{WORD_ZH.get(e['selects'], e['selects'])}"
        if e["group"] == "Symbol content and graphics" and e["keyword"] not in ("PIN", "TEXT"):
            out = "符号里的一个图元；写在场景里时是新的标注（13.4）"
        if e.get("key_subobject"):
            out += "；写了 `key` 时是附着对象的子对象"
        if e.get("rewrites"):
            out += f"；收到{WORD_ZH.get(e['rewrites'], e['rewrites'])}时改写它，按键覆盖（键：这个对象）"
        if e.get("override"):
            ov = {"key": "按键覆盖", "geometry": "几何事实", "queue": "外观队列"}[e["override"]]
            if e.get("key"):
                ov += f"（键：{KEY_ZH.get(e['key'], e['key'])}）"
            out += f"；{ov}"
        return out
    out = {"none": "—", "same": "the input object", "target": "the target set",
           "new": f"a new {kind}", "definition": f"a definition ({kind})"}[e["output"]]
    if e.get("selects"):
        out += f" of {e['selects']}s"
    if e["group"] == "Symbol content and graphics" and e["keyword"] != "PIN" and e["keyword"] != "TEXT":
        out = "a graphic of the symbol; in the scene, a new annotation (13.4)"
    if e.get("key_subobject"):
        out += "; with `key`, a sub-object of the attached object"
    if e.get("rewrites"):
        out += f"; given a {e['rewrites']}, rewrites it, overriding by key (key: that {e['rewrites']})"
    if e.get("override"):
        ov = {"key": "overrides by key", "geometry": "geometric fact", "queue": "appearance queue"}[e["override"]]
        if e.get("key"):
            ov += f" (key: {e['key']})"
        out += f"; {ov}"
    return out


def where_note(where, lang="en") -> str:
    zh = lang == "zh"
    if where == ["header"]:
        return "只在头段记录里" if zh else "header records only"
    if where == ["package-header"]:
        return "只在包文件的头段记录里" if zh else "header record of a package file only"
    if where == ["symbol"]:
        return "只在符号体里" if zh else "symbol bodies only"
    if "header" in where and "level" in where:
        return "也可写在头段记录里" if zh else "also in header records"
    if "symbol" in where and "level" in where:
        return "也可写在符号体里" if zh else "also in symbol bodies"
    return ""


def priority_default(words, lang="en") -> str:
    """What applies when `priority` is not written (14.8): from 0.3 on, the record head gives AS and
    ATTACH only "disabled"."""
    zh = lang == "zh"
    if tuple(int(n) for n in words["version"].split(".")) < (0, 3):
        return "不写时用记录头的优先级（14.8）。" if zh else "without it the priority of the record head applies (14.8)."
    if zh:
        return "不写时用记录头的优先级，只是 AS、ATTACH 从记录头只取 `\"disabled\"`（14.8）。"
    return "without it the priority of the record head applies, except that AS and ATTACH take only `\"disabled\"` from it (14.8)."


def annex_a(words, lang="en") -> str:
    zh = lang == "zh"
    if zh:
        L = ["## 附录 A 关键字签名（规范性）", "",
             f"本附录由标准词表 `rnl-words-{words['version']}.json` 生成（5.3）；两者不一致时是勘误。它列出每个关键字收什么、给什么；"
             "含义见最后一栏所指的节。", "",
             "- **对象槽位**按标准写法的次序列出，带槽位名（可以写成角色，5.3）和收什么：" + "；".join(
                 f"*{t['name']}*：{TAKES_ZH[t['name']]}" for t in words["takes"]) + "。吸收的输入是严格的，封装的输入是容错的（5.4、7.3）。",
             "- **种类**是种类名（" + "、".join(f"`{k['name']}`" for k in words["kinds"]) + "）或："
             + "；".join(f"*{k['name']}*，{PSEUDO_KINDS_ZH[k['name']]}" for k in words["pseudo_kinds"]) + "。",
             "- **参数**标了*位置 1*、*位置 2*的可以按位置写，按这个次序；其余只能写名字（4.2）。"
             "`priority` 的取值总是 `\"disabled\"`、`\"low\"`、`\"normal\"`、`\"high\"`；" + priority_default(words, lang),
             "- **结果**：语句给出的对象（5.5），以及它的事实怎样覆盖（5.8）。没写覆盖方式的，事实累加。",
             "- 没有注明的，关键字可以写在层的任何记录里；注明*也可写在符号体里*的，还可以写在 DEFINE-SYMBOL 的体里。",
             "",
             "### A.1 记录", "",
             "`@RNL(…) {…}` 把它的语句提交到本层（5.4）。记录头的字段（2.8），都写名字：", "",
             "| 字段 | 取值 | 默认值 | 影响解释 |", "|---|---|---|---|"]
    else:
        L = ["## Annex A Keyword signatures (normative)", "",
             f"This annex is generated from the word list `rnl-words-{words['version']}.json` (5.3); where the two differ, it is an "
             "erratum. It lists what each keyword takes and gives; what it means is in the section named in the last column.", "",
             "- **Slots** are listed in the standard order, with their names (which can be written as roles, 5.3) and what "
             "they take: " + "; ".join(f"*{t['name']}*: {t['meaning']}" for t in words["takes"]) + ". Absorbed input is strict; enclosed input is tolerant (5.4, 7.3).",
             "- **Kinds** of slots are kind names (" + ", ".join(f"`{k['name']}`" for k in words["kinds"]) + ") or: "
             + "; ".join(f"*{k['name']}*, {k['meaning']}" for k in words["pseudo_kinds"]) + ".",
             "- **Parameters** marked *position 1*, *position 2* can be given by position, in that order; all others only by name (4.2). "
             "`priority` always takes `\"disabled\"`, `\"low\"`, `\"normal\"` or `\"high\"`; " + priority_default(words, lang),
             "- **Result**: the object the statement gives (5.5) and how its facts override (5.8). Without an override, facts accumulate.",
             "- Unless noted, a keyword can be written in any record of a level; *symbol bodies* means it can also be written in a DEFINE-SYMBOL body.",
             "",
             "### A.1 The record", "",
             "`@RNL(…) {…}` commits its statements to the level (5.4). Record head fields (2.8), all given by name:", "",
             "| Field | Values | Default | Affects interpretation |", "|---|---|---|---|"]
    for f in words["record"]["fields"]:
        vals = ("、" if zh else ", ").join(f"`{json.dumps(v)}`" for v in f.get("values", [])) or ("字符串" if zh else "string")
        default = f"`{json.dumps(f['default'])}`" if "default" in f else "—"
        affects = ("是" if zh else "yes") if f["affects_interpretation"] else ("否" if zh else "no")
        L.append(f"| `{f['name']}` | {vals} | {default} | {affects} |")
    n = 2
    for group in dict.fromkeys(e["group"] for e in words["keywords"]):  # in word-list order
        title = GROUP_ZH[group] if zh else group
        head = "| 关键字 | 对象槽位 | 参数 | 结果 | 节 |" if zh else "| Keyword | Slots | Parameters | Result | Section |"
        L += ["", f"### A.{n} {title}", "", head, "|---|---|---|---|---|"]
        for e in words["keywords"]:
            if e["group"] != group:
                continue
            where = where_note(e["where"], lang)
            name = f"`@{e['keyword']}`" + ((f"（{where}）" if zh else f" ({where})") if where else "")
            L.append(f"| {name} | {md_slots(e, lang)} | {md_params(e, lang)} | {md_result(e, lang)} | {e['section']} |")
        n += 1
    if zh:
        L += ["", f"### A.{n} 其他标准词", "",
              "- 种类：" + "、".join(f"`{k['name']}`" for k in words["kinds"]) + "（5.3、9.4）。",
              "- `::` 后面的子对象：" + "、".join(f"`{k['name']}`" for k in words["subobjects"]) + "（6.5）。",
              "- `${…}` 读的属性：" + "、".join(f"`{k['name']}`" for k in words["attributes"]) + "（13.6）。",
              "- 真假值：`true`、`false`（3.7）。", "",
              "样式属性（9.3）：", "", "| 属性 | 类型和取值 | 默认值 |", "|---|---|---|"]
    else:
        L += ["", f"### A.{n} Other standard words", "",
              "- Kinds: " + ", ".join(f"`{k['name']}`" for k in words["kinds"]) + " (5.3, 9.4).",
              "- Sub-objects after `::`: " + ", ".join(f"`{k['name']}`" for k in words["subobjects"]) + " (6.5).",
              "- Attributes read by `${…}`: " + ", ".join(f"`{k['name']}`" for k in words["attributes"]) + " (13.6).",
              "- Truth values: `true`, `false` (3.7).", "",
              "Style properties (9.3):", "", "| Property | Type and values | Default |", "|---|---|---|"]
    for s in words["style_properties"]:
        if s["type"] == "enum":
            tv = ("、" if zh else ", ").join(f"`{json.dumps(v)}`" for v in s["values"])
        else:
            tv = WORD_ZH.get(s["type"], s["type"]) if zh else s["type"]
        d = f"`{json.dumps(s['default'])}`" if "default" in s else "—"
        L.append(f"| `{s['name']}` | {tv} | {d} |")
    groups = ("；" if zh else "; ").join(
        f"*{g}*{'：' if zh else ': '}" + ("、" if zh else ", ").join(f"`{p}`" for p in ps)
        for g, ps in words["style_groups"].items())
    if zh:
        L += ["", "关键字可以当参数收的样式属性（9.2.5）：" + groups + "。", ""]
    else:
        L += ["", "Style properties that keywords take as parameters (9.2.5): " + groups + ".", ""]
    return "\n".join(L)
