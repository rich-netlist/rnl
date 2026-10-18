"""Build every published form of the RNL standard from its chapters, and check that they agree.

    python scripts/spec/build.py                 check, then build every version into scripts/spec/dist/
    python scripts/spec/build.py --check         the checks only
    python scripts/spec/build.py --version 0.3   only this version, as for the release of a tag spec-0.3-rN
    python scripts/spec/build.py --site DIR      also put the web pages and their downloads into a built site
    python scripts/spec/build.py --no-pdf        skip the PDFs
    python scripts/spec/build.py --stamp         stamp the Chinese parts, once they are in line with the English

Input, the spec/ folder of the repository (or --spec DIR), with a folder for each version V, such as 0.2:
    V/en/, V/zh/            front.md, ch01.md, ch02.md, …, annex-a.md, annex-b.md, …: the source
    rnl-V.md, rnl-V.zh.md   the combined texts, which must be what the chapters give
    rnl-words-V.json        the word list of the version
    rnl-words.schema.json   the schema of the word lists, shared by all versions

Output, for each version, in English and Chinese, all from the same chapters and with the same revision
label: the combined Markdown, the chapters as a zip, one web page, a PDF, and SHA256SUMS. With --site, the
Chinese page of version V goes to DIR/zh/spec/V/ and the English one to DIR/en/spec/V/, each with its
downloads; DIR/zh/spec/ and DIR/en/spec/ lead to the newest version, and each page links to the others.
The Chinese pages of the versions in MOVED_ZH were at DIR/spec/V/ until the Chinese pages of the site
moved to zh/; their old addresses, and DIR/spec/, lead to the new ones, and their downloads are copied there.
Next to the page of V go the pages that compare it with each earlier version W, compare-W.html, which
mark what was added and what was removed (specdiff.py).

Before building, it checks for each version that
  1. the Chinese chapters have the sections, list items, table rows and code of the English ones;
  2. each Chinese part is stamped with the hash of the English part it was translated from, and that the
     hash is current;
  3. Annex A in both languages is what the word list gives, and so is the JSON excerpt of 5.3;
  4. every section number cited in the text exists;
  5. the combined texts are what the chapters give.
pandoc and typst must finish without warnings. Any failure stops the build (exit status 1).

Each version has its own revision label, from git (decision 0007): "revision n" at the tag spec-V-rn,
"revision n + later changes" once the chapters or the word list of V have changed since that tag, and
"not released" before its first tag. Commits that change other files do not count.

Needs pandoc 3.x and typst 0.13 or later on PATH, and git for the revision labels.
"""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import os
import re
import shutil
import subprocess
import sys
import time
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import specdiff  # noqa: E402
import spectools  # noqa: E402

READER = "commonmark_x-smart-tex_math_dollars-subscript-superscript-emoji-implicit_header_references"
FONTS = {  # preferences; only the families typst finds are passed on
    "serif": ["Libertinus Serif", "Noto Serif CJK SC", "Source Han Serif SC", "SimSun"],
    "sans": ["Segoe UI", "Noto Sans", "DejaVu Sans", "Noto Sans CJK SC", "Source Han Sans SC", "Microsoft YaHei"],
    "mono": ["DejaVu Sans Mono", "Noto Sans Mono CJK SC", "Noto Sans CJK SC", "Microsoft YaHei"],
}
SITE_URL = "https://rich-netlist.github.io/rnl/"  # the project site; the downloaded page links to it
FONTS_CSS = ("https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500;600"
             "&family=IBM+Plex+Sans+Condensed:wght@500;600;700&family={body}&display=swap")
LANGS = {
    "en": {"suffix": "", "html_lang": "en", "other": "zh", "other_label": "中文", "contents": "Contents",
           "zip_label": "Markdown, chapters", "md_label": "Markdown", "sums_label": "SHA-256 checksums",
           "intro": "Introduction", "tour": "Language tour", "spec": "Standard", "packages": "Packages",
           "tools": "Tools", "agents": "Agents", "pages": "Pages",
           "guide_note": "This is the short version of the standard: what it takes to read and write RNL files. It is not "
                         "normative; where it differs from the text below, the text below prevails. The same guide is "
                         "part of the [skill package for AI agents]({agents}).",
           "principles_note": "These are the principles of RNL, the text of [`spec/rich-netlist.md`]"
                              "(https://github.com/rich-netlist/rnl/blob/main/spec/rich-netlist.md), shown here to be read "
                              "with the standard. They are not a part of the standard and are not tied to a version of it; "
                              "the standard refers to them as “Principles”. The English text is authoritative.",
           "kicker": "Rich Netlist language standard", "theme_dark": "Switch to dark theme",
           "theme_light": "Switch to light theme", "site_home": "en/",
           "body_font": "IBM+Plex+Sans:wght@400;500;600;700",
           "foot": "The text of the standard is licensed under CC-BY-4.0. The English text is normative.",
           "files": "Files", "file": "File", "content": "Content", "versions": "Versions: ",
           "label": "Version {v} · {state} ({date}, {commit})", "released": "revision {n}",
           "after": "revision {n} + later changes", "draft": "not released", "dirty": " + uncommitted changes",
           "site_dir": "en/spec/{v}", "site_other": "../../../zh/spec/{v}/",
           "compare": "compare with {w}", "compare_title": "RNL {v} compared with {w}",
           "compared": "Compared with {label}:", "added": "added", "removed": "removed",
           "list_changed": "Changed", "list_added": "Added", "list_removed": "Removed", "front": "beginning",
           "annex": "Annex {a}", "changes": "Changes", "prev": "Previous change", "next": "Next change",
           "only": "Only what changed"},
    "zh": {"suffix": ".zh", "html_lang": "zh-CN", "other": "en", "other_label": "EN", "contents": "目录",
           "zip_label": "Markdown 分章包", "md_label": "Markdown 合订本", "sums_label": "SHA-256 校验值",
           "intro": "介绍", "tour": "语言导览", "spec": "标准", "packages": "标准包", "tools": "工具", "agents": "给 agent",
           "pages": "页面",
           "guide_note": "这是标准的简版：读写 RNL 文件需要知道的东西。它不是正式文本，与下面的正文不一致时，以正文为准。"
                         "同一份指南也在[给 AI agent 的 skill 包]({agents})里。",
           "principles_note": "这是 RNL 的设计原则，即 [`spec/rich-netlist.zh.md`]"
                              "(https://github.com/rich-netlist/rnl/blob/main/spec/rich-netlist.zh.md) 的原文，放在这里方便和标准一起读。"
                              "它们不是标准的一部分，也不随标准编版本；标准引用它们时写作“原则”。以英文版为准。",
           "kicker": "Rich Netlist 语言标准", "theme_dark": "切换到深色", "theme_light": "切换到浅色", "site_home": "zh/",
           "body_font": "Noto+Sans+SC:wght@400;500;700;900",
           "foot": "标准文本按 CC-BY-4.0 许可发布。本文是中文译本，以英文版为准。",
           "files": "文件", "file": "文件", "content": "内容", "versions": "版本：",
           "label": "{v} 版 · {state}（{date}，{commit}）", "released": "修订 {n}",
           "after": "修订 {n} 之后的修改", "draft": "未发布", "dirty": " + 未提交的修改",
           "site_dir": "zh/spec/{v}", "site_other": "../../../en/spec/{v}/",
           "compare": "与 {w} 对比", "compare_title": "RNL {v} 与 {w} 对比",
           "compared": "与 {label}对比：", "added": "新增", "removed": "删除",
           "list_changed": "改动", "list_added": "新增", "list_removed": "删掉", "front": "开头",
           "annex": "附录 {a}", "changes": "改动", "prev": "上一处改动", "next": "下一处改动",
           "only": "只看改动"},
}
# The versions whose Chinese pages were published at spec/V/, at the root of the site, until the Chinese
# pages moved to zh/ on 2026-10-18 (site/build.py); later versions have only the addresses under zh/.
MOVED_ZH = ("0.2",)
VERSION_DIR = re.compile(r"(0|[1-9]\d*)\.(0|[1-9]\d*)")  # a version folder of spec/, such as 0.2 (16.1)
STAMP = re.compile(r"^<!-- translated from en/(\S+) sha256:([0-9a-f]{12}) -->\n")
FENCE = re.compile(r"^([ \t]*)```([\w-]+)((?:[ \t]+\S+)+)[ \t]*$", re.M)
HEADING_ID = re.compile(r"^#{2,4} (?:Annex |附录 )?(\d+(?:\.\d+)*|[A-Z](?:\.\d+)?)\s", re.M)


class Failed(Exception):
    pass


def read(p: Path) -> str:
    return p.read_text(encoding="utf-8")


def versions(spec: Path) -> list[str]:
    """The versions in spec/, oldest first: the folders named major.minor that hold en/ and zh/."""
    found = [d.name for d in spec.iterdir()
             if d.is_dir() and VERSION_DIR.fullmatch(d.name) and (d / "en").is_dir() and (d / "zh").is_dir()]
    return sorted(found, key=lambda v: tuple(int(x) for x in v.split(".")))


def parts(spec: Path, v: str, lang: str = "en") -> list[str]:
    return spectools.parts([p.name for p in (spec / v / lang).glob("*.md")])


def translated(spec: Path, v: str) -> list[str]:
    return ["front.md"] + [p for p in parts(spec, v) if p != "annex-a.md"]  # Annex A is generated in both


def name(v: str, lang: str, ext: str) -> str:
    return f"rnl-{v}{LANGS[lang]['suffix']}{ext}"


def guide_path(spec: Path, v: str, lang: str) -> Path:
    """The short guide of version v, next to the combined texts: rnl-0.3-guide.md, rnl-0.3-guide.zh.md. It is
    a section of the web page of the version and not a part of the standard, so it is in none of the other
    forms and leaves the revision label as it is."""
    return spec / f"rnl-{v}-guide{LANGS[lang]['suffix']}.md"


def own_files(out: Path, v: str) -> list[Path]:
    """The files of version v in a folder of downloads: rnl-0.2.md, rnl-0.2-chapters.zip, … (not rnl-0.20…)."""
    return sorted({*out.glob(f"rnl-{v}.*"), *out.glob(f"rnl-{v}-*")})


def git(cwd: Path, *args: str) -> str | None:
    r = subprocess.run(["git", "-C", str(cwd), *args], capture_output=True, text=True)
    return r.stdout.strip() if r.returncode == 0 else None


def revision(spec: Path, v: str) -> dict:
    """The revision of version v: the newest tag spec-v-rN reachable from HEAD, and how many commits changed
    the sources of v (its chapters and its word list) after it. The label names the tagged commit while
    nothing has changed, and otherwise the last commit that changed the sources."""
    fmt = "--format=%h %cs %ct"
    if git(spec, "rev-parse", "--show-toplevel") is None:  # not in a repository, such as a staged copy
        return {"n": None, "after": 0, "dirty": False, "commit": "local", "date": time.strftime("%Y-%m-%d"),
                "epoch": 0}
    sources = [str(spec / v), str(spec / f"rnl-words-{v}.json")]
    tag = git(spec, "describe", "--tags", "--abbrev=0", "--match", f"spec-{v}-r*") or ""
    m = re.fullmatch(rf"spec-{re.escape(v)}-r(\d+)", tag)
    after = int(git(spec, "rev-list", "--count", f"{tag}..HEAD", "--", *sources) or 0) if m else 0
    if m and not after:
        line = git(spec, "log", "-1", fmt, tag)
    else:
        line = git(spec, "log", "-1", fmt, "--", *sources) or git(spec, "log", "-1", fmt)
    commit, date, epoch = (line or f"local {time.strftime('%Y-%m-%d')} 0").split()
    dirty = bool(git(spec, "status", "--porcelain", "--", *sources))
    return {"n": int(m.group(1)) if m else None, "after": after, "dirty": dirty,
            "commit": commit, "date": date, "epoch": int(epoch)}


def revision_label(lang: str, v: str, rev: dict) -> str:
    """At a tag "revision 2", after it "revision 2 + later changes", before the first tag "not released"."""
    L = LANGS[lang]
    if rev["n"] is None:
        state = L["draft"]
    else:
        state = L["released" if rev["after"] == 0 and not rev["dirty"] else "after"].format(n=rev["n"])
    label = L["label"].format(v=v, state=state, date=rev["date"], commit=rev["commit"])
    return label + (L["dirty"] if rev["dirty"] else "")


def en_hash(text: str) -> str:
    return hashlib.sha256(text.replace("\r\n", "\n").encode("utf-8")).hexdigest()[:12]


# --- checks, each for one version -------------------------------------------------------------------

def check_structure(spec: Path, v: str) -> list[str]:
    en, zh = parts(spec, v), parts(spec, v, "zh")
    if en != zh:
        return [f"{v}: the parts differ: en/ has {', '.join(en)}; zh/ has {', '.join(zh)}"]
    msgs = []
    for part in spectools.compared(en):
        msgs += [f"{v} {m}" for m in spectools.compare(read(spec / v / "en" / part), read(spec / v / "zh" / part),
                                                        part.removesuffix(".md"))]
    return msgs


def check_stamps(spec: Path, v: str) -> list[str]:
    msgs = []
    for part in translated(spec, v):
        m = STAMP.match(read(spec / v / "zh" / part))
        want = en_hash(read(spec / v / "en" / part))
        if not m:
            msgs.append(f"{v}/zh/{part}: no stamp of the English part it was translated from")
        elif m.group(1) != part or m.group(2) != want:
            msgs.append(f"{v}/zh/{part}: translated from en/{part} {m.group(2)}, the English is now {want}: "
                        f"bring the Chinese in line, then run --stamp")
    return msgs


def check_word_list(spec: Path, v: str) -> list[str]:
    words = json.loads(read(spec / f"rnl-words-{v}.json"))
    msgs = [] if words.get("version") == v else [f"rnl-words-{v}.json: its version is {words.get('version')}"]
    for lang in LANGS:
        if read(spec / v / lang / "annex-a.md") != spectools.annex_a(words, lang):
            msgs.append(f"{v}/{lang}/annex-a.md: not what the word list gives")
        ch05 = read(spec / v / lang / "ch05.md")
        m = re.search(r"^([ \t]*)```json$", ch05, re.M)
        want = "\n".join(m.group(1) + x if x else x for x in spectools.excerpt(words).split("\n")) if m else None
        if not m or want not in ch05:
            msgs.append(f"{v}/{lang}/ch05.md: the JSON excerpt of 5.3 is not the PLACE entry of the word list")
    return msgs


def prose_blocks(text: str) -> str:
    """The text without code blocks."""
    return re.sub(r"^[ \t]*```.*?^[ \t]*```[ \t]*$", "", text, flags=re.M | re.S)


def prose(text: str) -> str:
    """The text without code blocks and inline code."""
    return re.sub(r"`[^`\n]+`", "", prose_blocks(text))


REF = re.compile(r"(?<![\w.])(\d{1,2}(?:\.\d{1,2}){1,3})(?![\w])")
# Numbers that name principles ("Principles 2.4, 2.9") or versions; the same rule as rnl.lua.
NOT_A_SECTION = re.compile(r"(?:(?:Principles|原则)\s*(?:\d+(?:\.\d+)*\s*(?:,|、|and|or|和|或)\s*)*"
                           r"|(?:[Vv]ersions?|版本|before|RNL|0\.x)\s*)$")


def not_sections(text: str, ids: set[str], last: int) -> list[str]:
    """The numbers in the text that cite a section and name none of ids; those beyond chapter `last`, and
    versions and principles, are not citations."""
    body, found = prose(text), []
    for m in REF.finditer(body):
        num = m.group(1).rstrip(".")
        if num in ids or num.split(".")[0] == "0" or int(num.split(".")[0]) > last:
            continue
        if NOT_A_SECTION.search(body[max(0, m.start() - 60):m.start()]):
            continue
        found.append(num)
    return found


def check_refs(spec: Path, v: str) -> list[str]:
    msgs = []
    names = parts(spec, v)
    last = max(int(p[2:-3]) for p in names if p.startswith("ch"))  # numbers beyond the chapters are not sections
    for lang in LANGS:
        texts = {p: read(spec / v / lang / p) for p in ["front.md"] + names}
        ids = set(HEADING_ID.findall("\n".join(texts.values())))
        for part, text in texts.items():
            msgs += [f"{v}/{lang}/{part}: cites {num}, which is not a section" for num in not_sections(text, ids, last)]
    return msgs


def check_guide(spec: Path, v: str) -> list[str]:
    """The short guide, when the version has one: both languages, one title, the same structure and code in
    both, and every section it cites exists."""
    found = {lang: guide_path(spec, v, lang).exists() for lang in LANGS}
    if not any(found.values()):
        return []
    msgs = [f"{guide_path(spec, v, lang).name}: missing" for lang in LANGS if not found[lang]]
    if msgs:
        return msgs
    texts = {lang: read(guide_path(spec, v, lang)) for lang in LANGS}
    for lang, text in texts.items():
        titles = re.findall(r"^# ", prose_blocks(text), re.M)
        if not text.startswith("# ") or len(titles) != 1:
            msgs.append(f"{guide_path(spec, v, lang).name}: it starts with one title, `# ...`, and has no other level 1 heading")
    if msgs:
        return msgs
    msgs += [f"{v} guide {m}" for m in spectools.compare(texts["en"], texts["zh"], "guide", by_position=True)]
    names = parts(spec, v)
    last = max(int(p[2:-3]) for p in names if p.startswith("ch"))
    for lang, text in texts.items():
        ids = set(HEADING_ID.findall("\n".join(read(spec / v / lang / p) for p in ["front.md"] + names)))
        msgs += [f"{guide_path(spec, v, lang).name}: cites {num}, which is not a section" for num in not_sections(text, ids, last)]
    return msgs


def combined(spec: Path, v: str, lang: str) -> str:
    src = spec / v / lang
    return spectools.assemble(read(src / "front.md"), [read(src / p) for p in parts(spec, v)], lang)


def check_combined(spec: Path, v: str) -> list[str]:
    return [f"{name(v, lang, '.md')}: not what the chapters give" for lang in LANGS
            if read(spec / name(v, lang, ".md")) != combined(spec, v, lang)]


CHECKS = (("structure", check_structure), ("translation stamps", check_stamps), ("word list", check_word_list),
          ("references", check_refs), ("combined texts", check_combined), ("short guide", check_guide))


# --- builds ----------------------------------------------------------------------------------------

def fence_attrs(m: re.Match) -> str:
    """```rnl error id=a  ->  ```{.rnl .error data-id="a"}: pandoc keeps only the first word otherwise."""
    indent, lang, rest = m.group(1), m.group(2), m.group(3).split()
    classes = [lang] + [w for w in rest if "=" not in w]
    attrs = [f'data-{k}="{v}"' for k, v in (w.split("=", 1) for w in rest if "=" in w)]
    return indent + "```{" + " ".join([f".{c}" for c in classes] + attrs) + "}"


def pandoc_input(spec: Path, v: str, lang: str, work: Path) -> Path:
    # The translation stamps are for the checks; before the title they would also keep it from being one.
    texts = [STAMP.sub("", read(spec / v / lang / p)).strip() + "\n" for p in ["front.md"] + parts(spec, v)]
    p = work / f"{v}-{lang}.md"
    p.write_text(FENCE.sub(fence_attrs, "\n".join(texts)), encoding="utf-8", newline="\n")
    return p


def guide_section(spec: Path, v: str, lang: str, agents: str) -> str:
    """The guide as the first section of the page, after the front matter: its title one level down, with the
    id `guide` and a note that it is not the standard, and every heading below it one level down as well (not
    the lines of code blocks, which may start with #)."""
    out, in_code, first = [], False, True
    for line in read(guide_path(spec, v, lang)).strip().split("\n"):
        if re.match(r"^[ \t]*```", line):
            in_code = not in_code
        elif not in_code and re.match(r"^#{1,3} ", line):
            if first:
                first = False
                out += [f"#{line} {{#guide}}", "", LANGS[lang]["guide_note"].format(agents=agents)]
                continue
            line = "#" + line
        out.append(line)
    return "\n".join(out) + "\n"


def principles_path(spec: Path, lang: str) -> Path:
    """The principles of RNL, which are not a part of the standard and not tied to a version: spec/rich-netlist.md
    and spec/rich-netlist.zh.md. The web page of each version shows them, after the annexes."""
    return spec / ("rich-netlist.md" if lang == "en" else "rich-netlist.zh.md")


NUMBERED_HEADING = re.compile(r"^#{2,4} (\d+(?:\.\d+)*)\.? ")


def principles_section(spec: Path, lang: str) -> str:
    """The principles as the last section of the page, from their own file and in its words: the title one
    level down, with the id `principles` and a note that they are not the standard, then their sections from
    the first one on (what stands before it, the status, language, source and licence of the file, is left
    out), every heading one level down too. The numbered headings, which would take the ids of the standard's
    own sections, get the ids principles-1, principles-2.1 and so on (rnl.lua leaves those alone), and the
    section is a div of the class principles, in which rnl.lua does not link the numbers of the principles
    (such as the 2.1 of "(2.1)") to the standard's sections. Not the lines of code blocks, which may start
    with #."""
    out, in_code, state = [], False, "before"
    for line in read(principles_path(spec, lang)).strip().split("\n"):
        if re.match(r"^[ \t]*```", line):
            in_code = not in_code
        elif not in_code and re.match(r"^#{1,4} ", line):
            if state == "before":
                state = "skipping"
                out += ["::: {.principles}", "", f"#{line} {{#principles}}", "", LANGS[lang]["principles_note"], ""]
                continue
            if state == "skipping":
                if not line.startswith("## "):
                    continue
                state = "body"
            n = NUMBERED_HEADING.match(line)
            line = "#" + line + (f" {{#principles-{n.group(1)}}}" if n else "")
        elif state == "skipping" and not in_code:
            continue
        if state != "skipping":
            out.append(line)
    assert state == "body", f"{principles_path(spec, lang)}: no sections after the title"
    return "\n".join(out) + "\n\n:::\n"


def html_input(spec: Path, v: str, lang: str, work: Path, site_root: str) -> Path:
    """The input of the web page of a version: that of the other forms, with the short guide before chapter 1
    when the version has one, and the principles after the annexes when the repository has them. site_root is
    where the links of the page lead, such as the page for agents."""
    guide, principles = guide_path(spec, v, lang).exists(), principles_path(spec, lang).exists()
    if not guide and not principles:
        return pandoc_input(spec, v, lang, work)
    texts = [STAMP.sub("", read(spec / v / lang / "front.md")).strip() + "\n"]
    if guide:
        texts.append(guide_section(spec, v, lang, site_root + "agents.html"))
    texts += [STAMP.sub("", read(spec / v / lang / p)).strip() + "\n" for p in parts(spec, v)]
    if principles:
        texts.append(principles_section(spec, lang))
    p = work / f"{v}-{lang}-web.md"
    p.write_text(FENCE.sub(fence_attrs, "\n".join(texts)), encoding="utf-8", newline="\n")
    return p


def run(cmd: list[str], what: str, env: dict | None = None) -> None:
    r = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", env=env)
    noise = (r.stderr or "").strip()
    if r.returncode != 0 or noise:
        raise Failed(f"{what}:\n{noise or r.stdout}")


def typst_fonts() -> dict:
    listed = subprocess.run(["typst", "fonts"], capture_output=True, text=True, encoding="utf-8").stdout
    have = {line.strip().lower() for line in listed.splitlines()}
    return {k: [f for f in v if f.lower() in have] for k, v in FONTS.items()}


MARKDOWN = ("-f", READER, "--shift-heading-level-by=-1")  # how pandoc reads the chapters


def common_args(lang: str, v: str, rev: dict, reader: tuple = MARKDOWN) -> list[str]:
    return [*reader, "--lua-filter", str(HERE / "rnl.lua"),
            "--syntax-highlighting=none", "--fail-if-warnings", "--wrap=none",  # values go into scripts intact
            "-M", f"rnl-lang={lang}", "-M", f"typst-lang={lang}", "-M", f"revision-label={revision_label(lang, v, rev)}",
            "-M", f"revision={rev['commit']}", "-M", f"contents-title={LANGS[lang]['contents']}"]


def size(p: Path) -> str:
    if not p.exists():
        return ""
    n = p.stat().st_size
    return f"{n / 1048576:.1f} MB" if n >= 1048576 else f"{max(1, n // 1024)} KB"


def earlier(all_versions: list[str], v: str) -> list[str]:
    return all_versions[:all_versions.index(v)]


def version_links(lang: str, v: str, all_versions: list[str], href, compare_href=None,
                  comparing: str | None = None) -> str:
    """The versions, newest first, for the head of the page of v, then the pages that compare v with each
    earlier version, when compare_href is given; nothing while there is only one version. On the page that
    compares v with the version comparing, that comparison is in bold instead of v."""
    if len(all_versions) < 2:
        return ""
    L = LANGS[lang]
    items = [f"<b>{x}</b>" if x == v and not comparing else f'<a href="{href(x)}">{x}</a>'
             for x in reversed(all_versions)]
    for w in reversed(earlier(all_versions, v)) if compare_href else []:
        text = L["compare"].format(w=w)
        items.append(f"<b>{text}</b>" if w == comparing else f'<a href="{compare_href(w)}">{text}</a>')
    return L["versions"] + " · ".join(items)


def page_meta(lang: str, title: str, other_href: str, site_root: str) -> dict:
    """What every page has: its language and title, the top bar, the theme and the foot."""
    L, other = LANGS[lang], LANGS[lang]["other"]
    return {"html-lang": L["html_lang"], "pagetitle": title,
            "other-lang": LANGS[other]["html_lang"], "other-label": L["other_label"], "other-href": other_href,
            "site-root": site_root, "intro-label": L["intro"], "tour-label": L["tour"], "spec-label": L["spec"],
            "packages-label": L["packages"], "tools-label": L["tools"], "agents-label": L["agents"],
            "pages-label": L["pages"],
            "kicker": L["kicker"],
            "theme-label": L["theme_dark"],
            "theme-dark": L["theme_dark"], "theme-light": L["theme_light"], "foot-note": L["foot"],
            "font-href": FONTS_CSS.format(body=L["body_font"])}


def build_html(src: Path, lang: str, v: str, rev: dict, dest: Path, other_href: str, site_root: str,
               links: str, out: Path, work: Path) -> None:
    """The web page. Its downloads are the files of the same build, next to it; site_root is where the
    links of the top bar lead: the site's pages for this language. links is the line of versions."""
    L = LANGS[lang]
    log = work / f"links-{v}-{lang}.tsv"
    log.unlink(missing_ok=True)
    meta = {**page_meta(lang, f"RNL {v} · {L['kicker']}", other_href, site_root),
            "pdf-href": name(v, lang, ".pdf"), "md-href": name(v, lang, ".md"),
            "zip-href": name(v, lang, "-chapters.zip"), "pdf-size": size(out / name(v, lang, ".pdf")),
            "md-size": size(out / name(v, lang, ".md")), "zip-size": size(out / name(v, lang, "-chapters.zip")),
            "md-label": L["md_label"], "zip-label": L["zip_label"], "sums-label": L["sums_label"]}
    args = [a for k, val in meta.items() for a in ("-M", f"{k}={val}")]
    if links:  # HTML, so a variable (not escaped) rather than metadata
        args += ["-V", f"versions-html={links}"]
    run(["pandoc", str(src), *common_args(lang, v, rev), "-t", "html5", "--standalone",
         "--template", str(HERE / "page.html"), "--toc", "--toc-depth=2", "--section-divs", *args,
         "-o", str(dest)], f"pandoc html ({v}, {lang})", dict(os.environ, RNL_LINKS_LOG=str(log)))


def pandoc_json(spec: Path, v: str, lang: str, work: Path) -> dict:
    """Version v as pandoc reads it, before the filter: what the pages that compare versions are made of."""
    p = work / f"{v}-{lang}.json"
    run(["pandoc", str(pandoc_input(spec, v, lang, work)), *MARKDOWN, "--fail-if-warnings", "-t", "json",
         "-o", str(p)], f"pandoc json ({v}, {lang})")
    return json.loads(read(p))


def compare_head(lang: str, w: str, rev_w: dict, summary: dict) -> str:
    """The head of the page that compares with version w: which revision of w, the key to the marks, and
    links to the sections that changed, were added and were removed."""
    L = LANGS[lang]
    out = [f'<p class="compare-with">{html.escape(L["compared"].format(label=revision_label(lang, w, rev_w)))}'
           f'<span class="diff-ins">{L["added"]}</span><span class="diff-del">{L["removed"]}</span></p>']
    for what in ("changed", "added", "removed"):
        refs = []
        for k, title in summary[what]:
            text = L["front"] if not k else k.lstrip("~") if k.startswith("~") else (
                L["annex"].format(a=k) if re.fullmatch(r"[A-Z]", k) else k)
            href = "#top" if not k else "" if k.startswith("~") else f"#{k}"
            attrs = f' title="{html.escape(title)}"' if title else ""
            refs.append(f'<a href="{href}"{attrs}>{html.escape(text)}</a>' if href else
                        f"<span{attrs}>{html.escape(text)}</span>")
        if refs:
            out.append(f'<p class="compare-list"><b>{L["list_" + what]}</b>{" ".join(refs)}</p>')
    return "".join(out)


def build_compare(old: dict, new: dict, lang: str, v: str, w: str, revs: dict, dest: Path, other_href: str,
                  site_root: str, links: str, work: Path) -> dict:
    """The page that compares version v with the earlier version w, from both as pandoc reads them; returns
    the sections that changed, were added and were removed. revs holds the revisions of both."""
    merged, summary = specdiff.compare(old, new)
    problems = specdiff.check(old, new, merged)
    if problems:
        raise Failed(f"comparing {v} with {w} ({lang}): " + "; ".join(problems))
    src = work / f"{v}-{w}-{lang}.json"
    src.write_text(json.dumps(merged, ensure_ascii=False), encoding="utf-8", newline="\n")
    L = LANGS[lang]
    meta = {**page_meta(lang, f"{L['compare_title'].format(v=v, w=w)} · {L['kicker']}", other_href, site_root),
            "changes-label": L["changes"], "prev-label": L["prev"], "next-label": L["next"], "only-label": L["only"]}
    args = [a for k, val in meta.items() for a in ("-M", f"{k}={val}")]
    args += ["-V", f"compare-html={compare_head(lang, w, revs[w], summary)}"]
    if links:
        args += ["-V", f"versions-html={links}"]
    run(["pandoc", str(src), *common_args(lang, v, revs[v], ("-f", "json")), "-t", "html5", "--standalone",
         "--template", str(HERE / "page.html"), "--toc", "--toc-depth=2", "--section-divs", *args,
         "-o", str(dest)], f"pandoc html ({v} compared with {w}, {lang})")
    return summary


def build_pdf(src: Path, lang: str, v: str, rev: dict, out: Path, work: Path, fonts: dict) -> None:
    typ = work / f"{v}-{lang}.typ"
    font_args = [a for k, fams in fonts.items() for f in fams for a in ("-M", f"{k}={f}")]
    run(["pandoc", str(src), *common_args(lang, v, rev), "-t", "typst", "--standalone",
         "--template", str(HERE / "print.typ"), "-M", f"short-title=RNL {v}", *font_args,
         "-o", str(typ)], f"pandoc typst ({v}, {lang})")
    run(["typst", "compile", "--root", str(work), str(typ), str(out / name(v, lang, ".pdf"))], f"typst ({v}, {lang})")


def build_zip(spec: Path, lang: str, v: str, rev: dict, out: Path) -> None:
    """The chapters as files, with a README that lists them and a MANIFEST of hashes; reproducible."""
    L = LANGS[lang]
    src = spec / v / lang
    files = {p: read(src / p) for p in parts(spec, v)}
    rows = []
    for p, text in files.items():
        m = re.search(r"^## (.+)$", text, re.M)
        rows.append(f"| [{p}]({p}) | {m.group(1) if m else ''} |")
    for f in (f"rnl-words-{v}.json", "rnl-words.schema.json"):
        files[f] = read(spec / f)
        rows.append(f"| [{f}]({f}) | |")
    label = revision_label(lang, v, rev)
    readme = (STAMP.sub("", read(src / "front.md")).strip() + f"\n\n{label}\n\n## {L['files']}\n\n"
              f"| {L['file']} | {L['content']} |\n|---|---|\n" + "\n".join(rows) + "\n")
    files = {"README.md": readme, **files}
    manifest = [f"RNL {v} ({lang}), {label}", ""]
    manifest += [f"{hashlib.sha256(t.encode('utf-8')).hexdigest()}  {f}" for f, t in files.items()]
    files["MANIFEST.txt"] = "\n".join(manifest) + "\n"
    stamp = time.gmtime(rev["epoch"])[:6] if rev["epoch"] else (1980, 1, 1, 0, 0, 0)
    with zipfile.ZipFile(out / name(v, lang, "-chapters.zip"), "w", zipfile.ZIP_DEFLATED) as z:
        for f, t in files.items():
            info = zipfile.ZipInfo(f"{name(v, lang, '')}/{f}", date_time=stamp)
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, t.encode("utf-8"))


def write_sums(d: Path) -> None:
    lines = [f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}"
             for p in sorted(d.iterdir()) if p.is_file() and p.name != "SHA256SUMS"]
    (d / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")


def redirect(path: Path, lang: str, v: str, target: str) -> None:
    """A page of version v at path that leads to target, relative to it, and keeps the query and the section
    of the address."""
    L = LANGS[lang]
    html = (f'<!doctype html>\n<html lang="{L["html_lang"]}">\n<head>\n<meta charset="utf-8">\n'
            f'<title>RNL {v} · {L["kicker"]}</title>\n<link rel="canonical" href="{target}">\n'
            f'<meta http-equiv="refresh" content="0; url={target}">\n'
            f'<script>location.replace("{target}" + location.search + location.hash)</script>\n</head>\n'
            f'<body><p><a href="{target}">RNL {v}</a></p></body>\n</html>\n')
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(html, encoding="utf-8", newline="\n")


def write_redirect(d: Path, lang: str, newest: str, base: str = "") -> None:
    """d/index.html, which leads to the page of the newest version, under base, and keeps the section of
    the address."""
    redirect(d / "index.html", lang, newest, f"{base}{newest}/")


def write_moved(site: Path, v: str) -> None:
    """The old addresses of the Chinese pages of version v, site/spec/v/: each page leads to the same page
    under zh/ and keeps the section of the address, so that spec/0.2/#1.3 opens zh/spec/0.2/#1.3; the
    downloads, which no page can stand in for, are copied."""
    new = LANGS["zh"]["site_dir"].format(v=v)
    old = site / "spec" / v
    old.mkdir(parents=True, exist_ok=True)
    for f in sorted((site / new).iterdir()):
        if f.suffix == ".html":
            redirect(old / f.name, "zh", v, f"../../{new}/" + ("" if f.name == "index.html" else f.name))
        else:
            shutil.copyfile(f, old / f.name)


# --- main ----------------------------------------------------------------------------------------

def stamp(spec: Path, v: str) -> None:
    for part in translated(spec, v):
        p = spec / v / "zh" / part
        text = STAMP.sub("", read(p))
        line = f"<!-- translated from en/{part} sha256:{en_hash(read(spec / v / 'en' / part))} -->\n"
        p.write_text(line + text, encoding="utf-8", newline="\n")
    for lang in LANGS:  # the combined text holds the stamps too
        (spec / name(v, lang, ".md")).write_text(combined(spec, v, lang), encoding="utf-8", newline="\n")
    print(f"{v}: stamped {len(translated(spec, v))} Chinese parts and rewrote the combined texts")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--spec", type=Path, default=HERE.parents[1] / "spec", help="the spec/ folder (default: the repository's)")
    ap.add_argument("--version", help="only this version, such as 0.2 (default: every version in spec/)")
    ap.add_argument("--out", type=Path, default=HERE / "dist", help="where the downloads go (default: scripts/spec/dist/)")
    ap.add_argument("--site", type=Path, help="a built site to add the web pages and their downloads to")
    ap.add_argument("--check", action="store_true", help="run the checks only")
    ap.add_argument("--no-pdf", action="store_true", help="skip the PDFs")
    ap.add_argument("--stamp", action="store_true", help="stamp the Chinese parts with the hashes of the English ones")
    args = ap.parse_args(argv)
    spec = args.spec.resolve()
    all_versions = versions(spec)
    if not all_versions:
        print(f"{spec}: no version folders, such as 0.2/ with en/ and zh/")
        return 1
    if args.version and args.version not in all_versions:
        print(f"{spec}: no version {args.version}; there are {', '.join(all_versions)}")
        return 1
    chosen = [args.version] if args.version else all_versions
    if args.stamp:
        for v in chosen:
            stamp(spec, v)
        return 0

    failures = []
    for what, check in CHECKS:
        msgs = [m for v in chosen for m in check(spec, v)]
        print(f"check {what}: {'ok' if not msgs else f'{len(msgs)} problem(s)'}")
        failures += msgs
    if failures:
        print("\n".join(failures))
        return 1
    if args.check:
        return 0

    out, work = args.out, args.out / "work"
    work.mkdir(parents=True, exist_ok=True)
    fonts = typst_fonts()
    revs, docs = {}, {}  # the revisions of the versions, and the versions as pandoc reads them, once each

    def doc(x: str, lang: str) -> dict:
        if (x, lang) not in docs:
            docs[x, lang] = pandoc_json(spec, x, lang, work)
        return docs[x, lang]

    try:
        for v in chosen:
            for f in own_files(out, v):
                f.unlink()
            rev = revs[v] = revision(spec, v)
            for lang in LANGS:
                L = LANGS[lang]
                shutil.copyfile(spec / name(v, lang, ".md"), out / name(v, lang, ".md"))
                src = pandoc_input(spec, v, lang, work)
                build_zip(spec, lang, v, rev, out)
                if not args.no_pdf:
                    build_pdf(src, lang, v, rev, out, work, fonts)
                on_site = SITE_URL + L["site_dir"].format(v=v) + "/"
                published = version_links(lang, v, all_versions, lambda x: SITE_URL + L["site_dir"].format(v=x) + "/",
                                          lambda w: on_site + f"compare-{w}.html")
                build_html(src, lang, v, rev, out / name(v, lang, ".html"), name(v, L["other"], ".html"),
                           SITE_URL + L["site_home"], published, out, work)
                if args.site:
                    d = args.site / L["site_dir"].format(v=v)
                    d.mkdir(parents=True, exist_ok=True)
                    for ext in (".md", "-chapters.zip") + (() if args.no_pdf else (".pdf",)):
                        shutil.copyfile(out / name(v, lang, ext), d / name(v, lang, ext))
                    build_html(html_input(spec, v, lang, work, "../../"), lang, v, rev, d / "index.html",
                               L["site_other"].format(v=v), "../../",
                               version_links(lang, v, all_versions, lambda x: f"../{x}/",
                                             lambda w: f"compare-{w}.html"), out, work)
                    for w in earlier(all_versions, v):
                        revs.setdefault(w, revision(spec, w))
                        page = f"compare-{w}.html"
                        links = version_links(lang, v, all_versions, lambda x: "./" if x == v else f"../{x}/",
                                              lambda x: f"compare-{x}.html", comparing=w)
                        found = build_compare(doc(w, lang), doc(v, lang), lang, v, w, revs, d / page,
                                              L["site_other"].format(v=v) + page, "../../", links, work)
                        print(f"built {v} compared with {w}, {lang}: " +
                              ", ".join(f"{len(found[k])} sections {k}" for k in ("changed", "added", "removed")))
                    write_sums(d)
                    if lang == "zh" and v in MOVED_ZH:
                        write_moved(args.site, v)
                print(f"built {v} {lang}: {revision_label(lang, v, rev)}")
        if args.site:
            for lang in LANGS:
                write_redirect(args.site / Path(LANGS[lang]["site_dir"].format(v="")), lang, all_versions[-1])
            # spec/, which led to the newest Chinese page before it moved
            write_redirect(args.site / "spec", "zh", all_versions[-1], "../" + LANGS["zh"]["site_dir"].format(v=""))
    except Failed as e:
        print(e)
        return 1
    write_sums(out)
    for v in chosen:
        for p in own_files(out, v):
            print(f"  {p.name:28} {p.stat().st_size // 1024:6} KB")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
