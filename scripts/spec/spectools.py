"""Shared pieces of the build of the RNL standard (scripts/spec/ in rnl; publish/ in rnl-dev):
assembling the combined text from the chapters, comparing the Chinese chapters with the English ones,
the JSON excerpt of 5.3, and Annex A from the word list. Standard library only."""

from __future__ import annotations

import json
import re
from collections import Counter
from functools import lru_cache
from pathlib import Path

I18N = Path(__file__).resolve().parents[2] / "i18n"


@lru_cache(maxsize=None)
def phrases(lang: str) -> dict:
    """The fixed phrases of the standard in a language, i18n/<lang>/spec.json: the word of annex headings ("Annex")
    and the templates of Annex A. English is the source, so its phrases are here; a language without the file has none."""
    if lang == "en":
        return {"annex": "Annex", "annex_a": EN_ANNEX_A}
    p = I18N / lang / "spec.json"
    return json.loads(p.read_text(encoding="utf-8")) if p.is_file() else {}


@lru_cache(maxsize=None)
def annex_words() -> tuple[str, ...]:
    """The words that start the heading of an annex, one for each language that has one, English first: "Annex",
    "附录", …. A heading such as `## Annex A Keyword signatures` or `## 附录 A …` is numbered by its letter."""
    words = ["Annex"]
    for p in sorted(I18N.glob("*/spec.json")):
        w = json.loads(p.read_text(encoding="utf-8")).get("annex")
        if w and w not in words:
            words.append(w)
    return tuple(words)


def heading_re(levels: str) -> re.Pattern:
    """A heading of the given levels, such as `#{2,4}`, whose number or annex letter comes after the annex word, if any."""
    return re.compile(r"^(" + levels + ") (?:(?:" + "|".join(re.escape(w) for w in annex_words()) + r") )?(\S+)")

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



def assemble(front: str, parts: list[str], lang: str, contents: str | None = None) -> str:
    """The combined text: the front matter, a table of contents of chapters and sections, the parts. `contents` is
    the title of the table of contents in the language (the site's messages); without it, "Contents" or "目录"."""
    front = front.rstrip() + "\n"
    bodies = [p.strip() + "\n" for p in parts]
    toc = ["## " + (contents or ("Contents" if lang == "en" else "目录")), ""]
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
        m = heading_re("#{1,4}" if by_position else "#{2,4}").match(line)
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



def compare(en_text: str, zh_text: str, part: str, by_position: bool = False, lang: str = "zh") -> list[str]:
    """Differences of structure and code between an English part and its translation into `lang`."""
    en, zh = sections(en_text, by_position), sections(zh_text, by_position)
    msgs = []
    if list(en) != list(zh):
        msgs.append(f"{part}: headings differ\n    en: {' '.join(en)}\n    {lang}: {' '.join(zh)}")
    for key in en:
        if key not in zh:
            continue
        e, z = en[key], zh[key]
        where = f"{part} {key}"
        if e["code"] != z["code"]:
            msgs.append(f"{where}: code blocks differ ({len(e['code'])} in en, {len(z['code'])} in {lang})")
        missing = e["inline"] - z["inline"]
        extra = z["inline"] - e["inline"]
        if missing:
            msgs.append(f"{where}: inline code missing in {lang}: " + ", ".join(f"{k}×{v}" for k, v in missing.items()))
        if extra:
            msgs.append(f"{where}: inline code only in {lang}: " + ", ".join(f"{k}×{v}" for k, v in extra.items()))
        for f, name in (("items", "list items"), ("rows", "table rows"), ("paras", "paragraphs")):
            if e[f] != z[f]:
                msgs.append(f"{where}: {name} {e[f]} in en, {z[f]} in {lang}")
    for w in OLD_WORDS if lang == "zh" else []:  # words of the Chinese drafts
        n = zh_text.count(w)
        if n:
            msgs.append(f"{part}: old word {w!r} ×{n}")
    return msgs


# --- Annex A from the word list ---------------------------------------------------------------------------
# The fixed phrases of Annex A are templates, in English here and in i18n/<lang>/spec.json for each language that
# has them (decision 0015, step 4); {…} marks what the word list fills in. A language without the templates has its
# Annex A written by hand, and the build compares it with the English one section by section instead.

EN_ANNEX_A = {
    "title": "## Annex A Keyword signatures (normative)",
    "intro": "This annex is generated from the word list `rnl-words-{version}.json` (5.3); where the two differ, it is an "
             "erratum. It lists what each keyword takes and gives; what it means is in the section named in the last column.",
    "slots": "- **Slots** are listed in the standard order, with their names (which can be written as roles, 5.3) and what "
             "they take: {items}. Absorbed input is strict; enclosed input is tolerant (5.4, 7.3).",
    "item": "*{name}*: {text}", "items_sep": "; ",
    "kinds": "- **Kinds** of slots are kind names ({kinds}) or: {pseudo}.", "kinds_sep": ", ",
    "pseudo": "*{name}*, {text}", "pseudo_sep": "; ",
    "params": "- **Parameters** marked *position 1*, *position 2* can be given by position, in that order; all others only by "
              "name (4.2). `priority` always takes `\"disabled\"`, `\"low\"`, `\"normal\"` or `\"high\"`; {priority}",
    "result": "- **Result**: the object the statement gives (5.5) and how its facts override (5.8). Without an override, facts accumulate.",
    "where": "- Unless noted, a keyword can be written in any record of a level; *symbol bodies* means it can also be written in a "
             "DEFINE-SYMBOL body.",
    "a1_title": "The record",
    "a1_intro": "`@RNL(…) {…}` commits its statements to the level (5.4). Record head fields (2.8), all given by name:",
    "record_head": "| Field | Values | Default | Affects interpretation |",
    "string": "string", "yes": "yes", "no": "no", "list_sep": ", ",
    "keyword_head": "| Keyword | Slots | Parameters | Result | Section |",
    "other_title": "Other standard words",
    "other_kinds": "- Kinds: {names} (5.3, 9.4).",
    "other_subobjects": "- Sub-objects after `::`: {names} (6.5).",
    "other_attributes": "- Attributes read by `${{…}}`: {names} (13.6).",
    "other_truth": "- Truth values: `true`, `false` (3.7).",
    "style_intro": "Style properties (9.3):",
    "style_head": "| Property | Type and values | Default |",
    "style_groups": "Style properties that keywords take as parameters (9.2.5): {groups}.",
    "group_item": "*{name}*: {names}", "group_sep": "; ",
    "paren": " ({})",
    "colon": ": ",
    "slot_sep": "; ",
    "optional": ", optional",
    "role_slots": "{note} (`definition`, written {at})",
    "all_style": "style properties (9.3)",
    "position": "position {n}", "required": "required", "default": "default `{d}`",
    "bits_sep": "; ", "parts_sep": ", ",
    "or_array": ", or a non-empty array of {t}s",
    "definition_params": "the parameters of the definition (by position in the order of its parameter list, or by name; 9.4)",
    "style_params": "{style} style properties",
    "at_least_one": "at least one of {names}", "names_sep": ", ",
    "together": "{names} together or not at all", "together_sep": " and ",
    "exclusive_sep": " not with ", "exclusive_end": "", "extra_sep": "; ",
    "none": "—", "same": "the input object", "target": "the target set", "new": "a new {kind}", "definition": "a definition ({kind})",
    "selects": " of {sel}s",
    "graphic": "a graphic of the symbol; in the scene, a new annotation (13.4)",
    "key_subobject": "; with `key`, a sub-object of the attached object",
    "rewrites": "; given a {r}, rewrites it, overriding by key (key: that {r})",
    "override": "; {ov}", "override_key": " (key: {key})",
    "overrides": {"key": "overrides by key", "geometry": "geometric fact", "queue": "appearance queue"},
    "at": {"first": "first", "last": "last"},
    "role_note": {"AS": "the roles of the definition, then the definition", "ATTACH": "the definition, then its roles"},
    "where_note": {"header": "header records only", "package-header": "header record of a package file only",
                   "symbol": "symbol bodies only", "also-header": "also in header records", "also-symbol": "also in symbol bodies"},
    "priority_old": "without it the priority of the record head applies (14.8).",
    "priority_new": "without it the priority of the record head applies, except that AS and ATTACH take only `\"disabled\"` from it (14.8).",
    "words": {}, "keys": {}, "groups": {}, "takes": {}, "pseudo_kinds": {}, "kind_text": {
        "any": "any object", "level": "`#`", "owner": "`?`", "position": "`::position`",
        "general": "a general definition", "symbol": "a symbol", "style": "a style"},
}


def md_params(e, A) -> str:
    w = lambda x: A["words"].get(x, x)
    if e.get("style") == "all":
        return A["all_style"]
    parts = []
    pos = e.get("positional", [])
    for p in e["params"]:
        s = f"`{p['name']}`"
        bits = []
        if p["name"] in pos:
            bits.append(A["position"].format(n=pos.index(p["name"]) + 1))
        if p.get("required"):
            bits.append(A["required"])
        if p["type"] == "enum":
            vals = A["list_sep"].join(f'`{json.dumps(v, ensure_ascii=False)}`' for v in p["values"])
            if p["name"] != "priority":
                bits.append(vals)
        elif p["type"] != "string":
            t = w(p["type"])
            if "items" in p and p["type"] != "array":
                t += A["or_array"].format(t=t)
            bits.append(t)
        if "default" in p:
            bits.append(A["default"].format(d=json.dumps(p["default"], ensure_ascii=False)))
        if bits:
            s += A["paren"].format(A["bits_sep"].join(bits))
        parts.append(s)
    if e.get("definition_params"):
        parts.insert(0, A["definition_params"])
    if e.get("style") and e["style"] != "all":
        parts.append(A["style_params"].format(style=e["style"]))
    extra = []
    if e.get("at_least_one"):
        extra.append(A["at_least_one"].format(names=A["names_sep"].join(f"`{n}`" for n in e["at_least_one"])))
    for g in e.get("together", []):
        extra.append(A["together"].format(names=A["together_sep"].join(f"`{n}`" for n in g)))
    for g in e.get("exclusive", []):
        extra.append(A["exclusive_sep"].join(f"`{n}`" for n in g) + A["exclusive_end"])
    text = A["parts_sep"].join(parts) if parts else "—"
    if extra:
        text += A["extra_sep"] + A["extra_sep"].join(extra)
    return text


def md_slots(e, A) -> str:
    if e.get("role_slots"):
        at = e["slots"][0]["at"]
        return A["role_slots"].format(note=A["role_note"][e["keyword"]], at=A["at"][at])
    if not e["slots"]:
        return "—"
    out = []
    for s in e["slots"]:
        txt = f"`{s['name']}`{A['colon']}{s['takes']}"
        if s.get("kinds"):
            txt += A["paren"].format(A["list_sep"].join(A["kind_text"].get(k, k) for k in s["kinds"]))
        if s.get("optional"):
            txt += A["optional"]
        out.append(txt)
    return A["slot_sep"].join(out)


def md_result(e, A) -> str:
    w = lambda x: A["words"].get(x, x)
    kind = w(e.get("output_kind", ""))
    out = {"none": A["none"], "same": A["same"], "target": A["target"],
           "new": A["new"].format(kind=kind), "definition": A["definition"].format(kind=kind)}[e["output"]]
    if e.get("selects"):
        out += A["selects"].format(sel=w(e["selects"]))
    if e["group"] == "Symbol content and graphics" and e["keyword"] not in ("PIN", "TEXT"):
        out = A["graphic"]
    if e.get("key_subobject"):
        out += A["key_subobject"]
    if e.get("rewrites"):
        out += A["rewrites"].format(r=w(e["rewrites"]))
    if e.get("override"):
        ov = A["overrides"][e["override"]]
        if e.get("key"):
            ov += A["override_key"].format(key=A["keys"].get(e["key"], e["key"]))
        out += A["override"].format(ov=ov)
    return out


def where_note(where, A) -> str:
    n = A["where_note"]
    if where == ["header"]:
        return n["header"]
    if where == ["package-header"]:
        return n["package-header"]
    if where == ["symbol"]:
        return n["symbol"]
    if "header" in where and "level" in where:
        return n["also-header"]
    if "symbol" in where and "level" in where:
        return n["also-symbol"]
    return ""


def priority_default(words, A) -> str:
    """What applies when `priority` is not written (14.8): from 0.3 on, the record head gives AS and
    ATTACH only "disabled"."""
    if tuple(int(n) for n in words["version"].split(".")) < (0, 3):
        return A["priority_old"]
    return A["priority_new"]


def annex_a_phrases(lang: str) -> dict | None:
    """The templates of Annex A of a language, or None when it has none (its Annex A is written by hand)."""
    p = phrases(lang)
    return p.get("annex_a")


def annex_a(words, lang="en", A=None) -> str:
    A = A or annex_a_phrases(lang)
    sep = A["list_sep"]
    takes = {t["name"]: A["takes"].get(t["name"], t.get("meaning")) for t in words["takes"]}
    pseudo = {k["name"]: A["pseudo_kinds"].get(k["name"], k.get("meaning")) for k in words["pseudo_kinds"]}
    L = [A["title"], "",
         A["intro"].format(version=words["version"]), "",
         A["slots"].format(items=A["items_sep"].join(A["item"].format(name=t["name"], text=takes[t["name"]]) for t in words["takes"])),
         A["kinds"].format(kinds=A["kinds_sep"].join(f"`{k['name']}`" for k in words["kinds"]),
                           pseudo=A["pseudo_sep"].join(A["pseudo"].format(name=k["name"], text=pseudo[k["name"]]) for k in words["pseudo_kinds"])),
         A["params"].format(priority=priority_default(words, A)),
         A["result"],
         A["where"],
         "",
         f"### A.1 {A['a1_title']}", "",
         A["a1_intro"], "",
         A["record_head"], "|---|---|---|---|"]
    for f in words["record"]["fields"]:
        vals = sep.join(f"`{json.dumps(v)}`" for v in f.get("values", [])) or A["string"]
        default = f"`{json.dumps(f['default'])}`" if "default" in f else "—"
        affects = A["yes"] if f["affects_interpretation"] else A["no"]
        L.append(f"| `{f['name']}` | {vals} | {default} | {affects} |")
    n = 2
    for group in dict.fromkeys(e["group"] for e in words["keywords"]):  # in word-list order
        L += ["", f"### A.{n} {A['groups'].get(group, group)}", "", A["keyword_head"], "|---|---|---|---|---|"]
        for e in words["keywords"]:
            if e["group"] != group:
                continue
            where = where_note(e["where"], A)
            name = f"`@{e['keyword']}`" + (A["paren"].format(where) if where else "")
            L.append(f"| {name} | {md_slots(e, A)} | {md_params(e, A)} | {md_result(e, A)} | {e['section']} |")
        n += 1
    L += ["", f"### A.{n} {A['other_title']}", "",
          A["other_kinds"].format(names=sep.join(f"`{k['name']}`" for k in words["kinds"])),
          A["other_subobjects"].format(names=sep.join(f"`{k['name']}`" for k in words["subobjects"])),
          A["other_attributes"].format(names=sep.join(f"`{k['name']}`" for k in words["attributes"])),
          A["other_truth"], "", A["style_intro"], "", A["style_head"], "|---|---|---|"]
    for s in words["style_properties"]:
        if s["type"] == "enum":
            tv = sep.join(f"`{json.dumps(v)}`" for v in s["values"])
        else:
            tv = A["words"].get(s["type"], s["type"])
        d = f"`{json.dumps(s['default'])}`" if "default" in s else "—"
        L.append(f"| `{s['name']}` | {tv} | {d} |")
    groups = A["group_sep"].join(
        A["group_item"].format(name=g, names=sep.join(f"`{p}`" for p in ps)) for g, ps in words["style_groups"].items())
    L += ["", A["style_groups"].format(groups=groups), ""]
    return "\n".join(L)
